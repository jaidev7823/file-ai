// src-tauri/src/search.rs
use bytemuck::cast_slice;
use rusqlite::{params, Connection, Error, Result, Row};
use crate::file_scanner::File;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

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

#[derive(Debug, Default)]
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
        .map_err(|_| Error::InvalidColumnType(index, "DateTime".to_string(), rusqlite::types::Type::Text))
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
    let rows = stmt.query_map(params![vector_bytes, limit], |row| {
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
    }).map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| e.to_string())?);
    }
    Ok(results)
}

// Text search using FTS (no change needed here)
pub fn search_files_fts(
    db: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>> {
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
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    let vector_results = search_similar_files(db, normalized_embedding, limit)?;

    let fts_results = search_files_fts(db, query, limit)
        .map_err(|e| e.to_string())?;

    Ok(combine_search_results(vector_results, fts_results, 0.6, 0.4))
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
                
                existing.relevance_score = (vector_score * vector_weight) + (text_score * text_weight);
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
    query: &str,
    filters: SearchFilters,
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    let mut sql = r#"
        SELECT f.id, f.name, f.extension, f.path, f.content, f.created_at, f.updated_at,
               rank
        FROM files_fts 
        JOIN files f ON files_fts.rowid = f.id
        WHERE files_fts MATCH ?1
    "#.to_string();
    
    // Create a vector to hold *owned* `rusqlite::types::Value` instances.
    let mut owned_params: Vec<rusqlite::types::Value> = Vec::new();
    // FIX: Convert `&str` to `String` when creating `Value`
    owned_params.push(rusqlite::types::Value::from(query.to_string()));

    let mut current_param_idx = 2; // For dynamic placeholders ?2, ?3, etc.
    
    // Add extension filter
    if let Some(extensions) = &filters.extensions {
        if !extensions.is_empty() {
            let placeholders: Vec<String> = (0..extensions.len())
                .map(|i| format!("?{}", current_param_idx + i))
                .collect();
            sql.push_str(&format!(" AND f.extension IN ({})", placeholders.join(",")));
            
            for ext in extensions {
                // `ext` is &String, so .clone() creates an owned String which `Value::from` accepts.
                owned_params.push(rusqlite::types::Value::from(ext.clone()));
            }
            current_param_idx += extensions.len();
        }
    }
    
    // Add date filters
    if let Some(date_from) = &filters.date_from {
        sql.push_str(&format!(" AND f.created_at >= ?{}", current_param_idx));
        owned_params.push(rusqlite::types::Value::from(date_from.clone()));
        current_param_idx += 1;
    }
    
    if let Some(date_to) = &filters.date_to {
        sql.push_str(&format!(" AND f.created_at <= ?{}", current_param_idx));
        owned_params.push(rusqlite::types::Value::from(date_to.clone()));
        current_param_idx += 1;
    }
    
    sql.push_str(&format!(" ORDER BY rank LIMIT ?{}", current_param_idx));
    owned_params.push(rusqlite::types::Value::from(limit as i64));

    let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;
    
    // Create a slice of references (&dyn rusqlite::ToSql) from the `owned_params`
    // FIX: Correct variable name from `ms_for_query_map` to `params_for_query_map`
    let params_for_query_map: Vec<&dyn rusqlite::ToSql> = owned_params.iter()
        .map(|v| v as &dyn rusqlite::ToSql)
        .collect();

        let rows = stmt.query_map(params_for_query_map[..].as_ref(), |row| {
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
    }).map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| e.to_string())?);
    }
    Ok(results)
}

// Debug functions (no change needed here)
pub fn debug_print_available_functions(db: &Connection) {
    match db.prepare("SELECT name FROM pragma_function_list() WHERE name LIKE '%vec%'") {
        Ok(mut stmt) => {
            let function_iter = stmt.query_map([], |row| {
                Ok(row.get::<_, String>(0)?)
            });

            println!("Available vector functions:");
            if let Ok(functions) = function_iter {
                for function_result in functions {
                    if let Ok(function_name) = function_result {
                        println!("  - {}", function_name);
                    }
                }
            }
        }
        Err(e) => println!("Error querying functions: {}", e),
    }
}

pub fn debug_print_file_vec_schema(db: &Connection) {
    match db.prepare("PRAGMA table_info(file_vec)") {
        Ok(mut stmt) => {
            let column_iter = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(1)?, // column name
                    row.get::<_, String>(2)?, // data type
                ))
            });

            println!("file_vec table schema:");
            if let Ok(columns) = column_iter {
                for column_result in columns {
                    if let Ok((name, data_type)) = column_result {
                        println!("  - {}: {}", name, data_type);
                    }
                }
            }
        }
        Err(e) => println!("Error querying file_vec schema: {}", e),
    }
}