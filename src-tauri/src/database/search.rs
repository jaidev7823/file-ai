use crate::database;
use crate::embed_and_store;
use crate::file_scanner::File;
use bytemuck::cast_slice;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Error, Result, Row};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub result_type: String,
    pub title: String,
    pub path: String,
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
            let id: i32 = row.get(0)?;
            Ok(SearchResult {
                id: id.to_string(),
                result_type: "file".to_string(),
                title: row.get(1)?,
                path: row.get(3)?,
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

pub fn search_files_fts(db: &Connection, query: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
    let sql = r#"
        SELECT f.id, f.name, f.extension, f.path, f.content, f.created_at, f.updated_at,
               rank
        FROM files_fts 
        JOIN files f ON files_fts.rowid = f.id
        WHERE files_fts MATCH ?1
        ORDER BY rank
        LIMIT ?2
    "#;

    let mut stmt = db.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![query, limit], |row| {
        let id: i32 = row.get(0)?;
        Ok(SearchResult {
            id: id.to_string(),
            result_type: "file".to_string(),
            title: row.get(1)?,
            path: row.get(3)?,
            relevance_score: row.get::<_, f64>(7)? as f32,
            match_type: SearchMatchType::Text(row.get::<_, f64>(7)? as f32),
            snippet: None,
        })
    }).map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| e.to_string())?);
    }
    Ok(results)
}

pub fn combine_search_results(
    vector_results: Vec<SearchResult>,
    fts_results: Vec<SearchResult>,
    vector_weight: f32,
    text_weight: f32,
) -> Vec<SearchResult> {
    let mut combined: HashMap<String, SearchResult> = HashMap::new();

    // Add vector results
    for result in vector_results {
        combined.insert(result.id.clone(), result);
    }

    // Merge FTS results
    for fts_result in fts_results {
        match combined.get_mut(&fts_result.id) {
            Some(existing) => {
                let vector_score = match &existing.match_type {
                    SearchMatchType::Vector(score) => *score,
                    _ => 0.0,
                };
                let text_score = match &fts_result.match_type {
                    SearchMatchType::Text(score) => *score,
                    _ => 0.0,
                };

                existing.relevance_score = (vector_score * vector_weight) + (text_score * text_weight);
                existing.match_type = SearchMatchType::Hybrid(vector_score, text_score);
            }
            None => {
                combined.insert(fts_result.id.clone(), fts_result);
            }
        }
    }

    let mut results: Vec<SearchResult> = combined.into_values().collect();
    results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
    results
}

pub fn hybrid_search_with_embedding(
    db: &Connection,
    normalized_embedding: &[f32],
    query: &str,
    filters: SearchFilters,
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    let vector_results = search_similar_files(db, normalized_embedding, limit)?;
    let metadata_results = advanced_search(db, Some(query.to_string()), filters, limit)?;

    let mut results = combine_search_results(vector_results, metadata_results, 0.6, 0.4);
    let folder_results = extract_folder_results(&results);
    results.extend(folder_results);

    results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
    Ok(results)
}

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
            let id: i32 = row.get(0)?;
            Ok(SearchResult {
                id: id.to_string(),
                result_type: "file".to_string(),
                title: row.get(1)?,
                path: row.get(3)?,
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

// FIXED: Removed async and spawn_blocking to prevent runtime issues
pub fn perform_file_search(
    query: String,
    top_k: Option<usize>,
    filters: Option<SearchFilters>,
) -> Result<Vec<SearchResult>, String> {
    let limit = top_k.unwrap_or(5);
    let filters = filters.unwrap_or_default();

    // Get embedding synchronously - make sure embed_and_store::get_embedding is NOT async
    let query_embedding = embed_and_store::get_embedding(&query)
        .map_err(|e| format!("Embedding error: {}", e))?;
    
    let normalized = embed_and_store::normalize(query_embedding);

    // Get database connection and perform search synchronously
    let db = database::get_connection();
    hybrid_search_with_embedding(&db, &normalized, &query, filters, limit)
}

fn extract_folder_results(file_results: &[SearchResult]) -> Vec<SearchResult> {
    let mut folder_map: HashMap<String, usize> = HashMap::new();

    for result in file_results {
        if result.result_type == "file" {
            if let Some(parent) = Path::new(&result.path).parent() {
                let folder_path = parent.to_string_lossy().to_string();
                *folder_map.entry(folder_path).or_insert(0) += 1;
            }
        }
    }

    folder_map
        .into_iter()
        .filter(|(_, count)| *count >= 2) // Only include folders with 2+ matching files
        .map(|(path, count)| {
            let folder_name = Path::new(&path)
                .file_name()
                .unwrap_or_else(|| Path::new(&path).as_os_str())
                .to_string_lossy()
                .to_string();
            
            SearchResult {
                id: format!("folder_{}", path.replace('/', "_").replace('\\', "_")),
                result_type: "folder".to_string(),
                title: folder_name,
                path,
                relevance_score: (count as f32) * 0.1, // Score based on number of matching files
                match_type: SearchMatchType::Text((count as f32) * 0.1),
                snippet: Some(format!("{} matching files", count)),
            }
        })
        .collect()
}