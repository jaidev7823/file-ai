// src-tauri/src/search.rs
use crate::file_scanner::File;
use bytemuck::cast_slice;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Error, Result, Row};
use std::collections::HashMap;
use crate::database;
use crate::embed_and_store;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub file: File,
    pub relevance_score: f32,
    pub match_type: SearchMatchType,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SearchMatchType {
    Vector(f32),
    Text(f32),
    Hybrid(f32, f32), // vector_score, text_score
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchFilters {
    pub extensions: Option<Vec<String>>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub min_size: Option<usize>,
    pub max_size: Option<usize>,
}

// Helper function to parse DateTime from SQLite string
fn parse_datetime_from_row(row: &Row, index: usize) -> Result<DateTime<Utc>, Error> {
    let date_str: String = row.get(index)?;
    DateTime::parse_from_rfc3339(&date_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| {
            Error::InvalidColumnType(index, "DateTime".to_string(), rusqlite::types::Type::Text)
        })
}

// Fixed vector search function with proper vec0 syntax
pub fn search_similar_files(
    db: &Connection,
    normalized_query: &[f32],
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    let vector_bytes: &[u8] = cast_slice(normalized_query);

    let sql = r#"
        SELECT f.id, f.name, f.extension, f.path, f.content, f.created_at, f.updated_at,
               distance
        FROM file_vec fv
        JOIN file_vec_map fvm ON fv.rowid = fvm.vec_rowid
        JOIN files f ON fvm.file_id = f.id
        WHERE fv.content_vec MATCH ? AND k = ?
        ORDER BY distance
    "#;

    let mut stmt = db.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![vector_bytes, limit], |row| {
            Ok(SearchResult {
                file: File {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    extension: row.get(2)?,
                    path: row.get(3)?,
                    content: row.get(4)?,
                    created_at: parse_datetime_from_row(row, 5)?,
                    updated_at: parse_datetime_from_row(row, 6)?,
                },
                relevance_score: 1.0 - row.get::<_, f32>(7)?,
                match_type: SearchMatchType::Vector(1.0 - row.get::<_, f32>(7)?),
                snippet: None,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| e.to_string())?);
    }
    Ok(results)
}

// Text search using FTS (no change needed here)
pub fn search_files_fts(db: &Connection, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
    let sql = r#"
        SELECT f.id, f.name, f.extension, f.path, f.content, f.created_at, f.updated_at,
               rank
        FROM files_fts 
        JOIN files f ON files_fts.rowid = f.id
        WHERE files_fts MATCH ?1
        ORDER BY rank
        LIMIT ?2
    "#;

    let mut stmt = db.prepare(sql)?;
    let rows = stmt.query_map(params![query, limit], |row| {
        Ok(SearchResult {
            file: File {
                id: row.get(0)?,
                name: row.get(1)?,
                extension: row.get(2)?,
                path: row.get(3)?,
                content: row.get(4)?,
                created_at: parse_datetime_from_row(row, 5)?,
                updated_at: parse_datetime_from_row(row, 6)?,
            },
            relevance_score: row.get::<_, f64>(7)? as f32,
            match_type: SearchMatchType::Text(row.get::<_, f64>(7)? as f32),
            snippet: None,
        })
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

// Hybrid search combining vector + FTS (no change needed here)
pub fn hybrid_search_with_embedding(
    db: &Connection,
    normalized_embedding: &[f32],
    query: &str,
    filters: SearchFilters,
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    let vector_results = search_similar_files(db, normalized_embedding, limit)?;

    let metadata_results = advanced_search(db, Some(query.to_string()), filters, limit)?;

    Ok(combine_search_results(
        vector_results,
        metadata_results,
        0.6,
        0.4,
    ))
}

// Helper function to combine search results (no change needed here)
fn combine_search_results(
    vector_results: Vec<SearchResult>,
    fts_results: Vec<SearchResult>,
    vector_weight: f32,
    text_weight: f32,
) -> Vec<SearchResult> {
    let mut combined: HashMap<i32, SearchResult> = HashMap::new();

    // Add vector results
    for result in vector_results {
        combined.insert(result.file.id, result);
    }

    // Merge FTS results
    for fts_result in fts_results {
        match combined.get_mut(&fts_result.file.id) {
            Some(existing) => {
                // File found in both - create hybrid score
                let vector_score = match &existing.match_type {
                    SearchMatchType::Vector(score) => *score,
                    _ => 0.0,
                };
                let text_score = match &fts_result.match_type {
                    SearchMatchType::Text(score) => *score,
                    _ => 0.0,
                };

                existing.relevance_score =
                    (vector_score * vector_weight) + (text_score * text_weight);
                existing.match_type = SearchMatchType::Hybrid(vector_score, text_score);
            }
            None => {
                // New file from FTS only
                combined.insert(fts_result.file.id, fts_result);
            }
        }
    }

    // Sort by relevance score and return
    let mut results: Vec<SearchResult> = combined.into_values().collect();
    results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
    results
}

// Enhanced search with filters - FIX applied here
pub fn advanced_search(
    db: &Connection,
    name_query: Option<String>,
    filters: SearchFilters,
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    let mut sql = String::from(
        "
        SELECT id, name, extension, path, content, created_at, updated_at
        FROM files
        WHERE 1 = 1
    ",
    );

    let mut params: Vec<rusqlite::types::Value> = Vec::new();
    let mut idx = 1;

    if let Some(query) = name_query {
        sql.push_str(&format!(" AND name LIKE ?{}", idx));
        params.push(format!("%{}%", query).into());
        idx += 1;
    }

    if let Some(extensions) = &filters.extensions {
        if !extensions.is_empty() {
            let placeholders: Vec<String> = (0..extensions.len())
                .map(|i| format!("?{}", idx + i))
                .collect();
            sql.push_str(&format!(" AND extension IN ({})", placeholders.join(",")));
            for ext in extensions {
                params.push(ext.clone().into());
            }
            idx += extensions.len();
        }
    }

    if let Some(date_from) = &filters.date_from {
        sql.push_str(&format!(" AND created_at >= ?{}", idx));
        params.push(date_from.clone().into());
        idx += 1;
    }

    if let Some(date_to) = &filters.date_to {
        sql.push_str(&format!(" AND created_at <= ?{}", idx));
        params.push(date_to.clone().into());
        idx += 1;
    }

    sql.push_str(&format!(" ORDER BY created_at DESC LIMIT ?{}", idx));
    params.push((limit as i64).into());

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|v| v as _).collect();

    let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(&*param_refs, |row| {
            Ok(SearchResult {
                file: File {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    extension: row.get(2)?,
                    path: row.get(3)?,
                    content: row.get(4)?,
                    created_at: parse_datetime_from_row(row, 5)?,
                    updated_at: parse_datetime_from_row(row, 6)?,
                },
                relevance_score: 1.0,
                match_type: SearchMatchType::Text(1.0),
                snippet: None,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| e.to_string())?);
    }

    Ok(results)
}

pub async fn perform_file_search(
    query: String,
    top_k: Option<usize>,
    filters: Option<SearchFilters>,
) -> Result<Vec<SearchResult>, String> {
    let limit = top_k.unwrap_or(5);

    let query_for_embedding = query.clone();

    let query_embedding_task_result = tokio::task::spawn_blocking(move || {
        embed_and_store::get_embedding(&query_for_embedding)
            .map_err(|e| format!("Embedding error in blocking task: {}", e))
    })
    .await;

    let query_embedding = match query_embedding_task_result {
        Ok(inner_result) => inner_result?,
        Err(join_err) => {
            return Err(format!(
                "Failed to spawn blocking task for embedding: {}",
                join_err
            ));
        }
    };

    let normalized = embed_and_store::normalize(query_embedding);
    let filters = filters.unwrap_or_default();

    tokio::task::spawn_blocking(move || {
        let db = database::get_connection();
        database::hybrid_search_with_embedding(&db, &normalized, &query, filters, limit)
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}