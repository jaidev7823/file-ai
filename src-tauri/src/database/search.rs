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
    // Removed fields that don't exist in your database schema
    // pub author: Option<String>,
    // pub folder: Option<String>, 
    // pub last_accessed_from: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum SearchIntent {
    RecentActivity,
    FileType,
    AuthorBased,
    FolderBased,
    DateBased,
    KeywordBased,
    NaturalLanguage,
}

pub fn classify_intent(query: &str) -> SearchIntent {
    let query_lower = query.to_lowercase();
    if query_lower.contains("recent") || query_lower.contains("last accessed") {
        SearchIntent::RecentActivity
    } else if query_lower.contains("pdf") || query_lower.contains("excel") {
        SearchIntent::FileType
    } else if query_lower.contains("created by") || query_lower.contains("authored by") {
        SearchIntent::AuthorBased
    } else if query_lower.contains("in the folder") || query_lower.contains("directory") {
        SearchIntent::FolderBased
    } else if query_lower.contains("created on") || query_lower.contains("updated on") {
        SearchIntent::DateBased
    } else if query.contains('"') || query.contains("AND") || query.contains("OR") {
        SearchIntent::KeywordBased
    } else {
        SearchIntent::NaturalLanguage
    }
}

fn parse_query(query: &str) -> (String, SearchFilters) {
    let mut filters = SearchFilters::default();
    let mut search_term = query.to_string();

    // Convert query to lowercase for case-insensitive matching
    let query_lower = query.to_lowercase();

    // Extract file extensions (e.g., "pdf", "txt", "doc")
    let extension_keywords = vec!["pdf", "txt", "doc", "docx", "xlsx", "jpg", "png"];
    let mut extensions = Vec::new();
    for ext in extension_keywords {
        if query_lower.contains(ext) {
            extensions.push(ext.to_string());
            search_term = search_term.replace(ext, "").trim().to_string();
        }
    }
    if !extensions.is_empty() {
        filters.extensions = Some(extensions);
    }

    // Extract date ranges (e.g., "from 2023-01-01", "to 2023-12-31")
    let date_from_regex = regex::Regex::new(r"from\s+(\d{4}-\d{2}-\d{2})").unwrap();
    let date_to_regex = regex::Regex::new(r"to\s+(\d{4}-\d{2}-\d{2})").unwrap();
    if let Some(captures) = date_from_regex.captures(&query_lower) {
        if let Some(date) = captures.get(1) {
            filters.date_from = Some(date.as_str().to_string());
            search_term = date_from_regex.replace(&search_term, "").trim().to_string();
        }
    }
    if let Some(captures) = date_to_regex.captures(&query_lower) {
        if let Some(date) = captures.get(1) {
            filters.date_to = Some(date.as_str().to_string());
            search_term = date_to_regex.replace(&search_term, "").trim().to_string();
        }
    }

    // Clean up multiple spaces
    search_term = search_term.split_whitespace().collect::<Vec<&str>>().join(" ");

    (search_term, filters)
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
    println!("DEBUG: Starting vector search with {} dimensions", normalized_query.len());
    
    let vector_bytes: &[u8] = cast_slice(normalized_query);

    let sql = r#"
        SELECT f.id, f.name, f.extension, f.path, f.content, f.created_at, f.updated_at, distance
        FROM file_vec fv
        JOIN file_vec_map fvm ON fv.rowid = fvm.vec_rowid
        JOIN files f ON fvm.file_id = f.id
        WHERE fv.content_vec MATCH ? AND k = ?
        ORDER BY distance
    "#;

    let mut stmt = db.prepare(sql).map_err(|e| {
        println!("DEBUG: Vector search prepare error: {}", e);
        e.to_string()
    })?;
    
    let rows = stmt
        .query_map(params![vector_bytes, limit], |row| {
            let id: i32 = row.get(0)?;
            let distance: f32 = row.get(7)?;
            let relevance = 1.0 - distance;
            
            println!("DEBUG: Vector result - ID: {}, Distance: {}, Relevance: {}", id, distance, relevance);
            
            Ok(SearchResult {
                id: id.to_string(),
                result_type: "file".to_string(),
                title: row.get(1)?,
                path: row.get(3)?,
                relevance_score: relevance,
                match_type: SearchMatchType::Vector(relevance),
                snippet: None,
            })
        })
        .map_err(|e| {
            println!("DEBUG: Vector search query error: {}", e);
            e.to_string()
        })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| e.to_string())?);
    }
    
    println!("DEBUG: Vector search returned {} results", results.len());
    Ok(results)
}

pub fn search_files_fts(
    db: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    println!("DEBUG: Starting FTS search for: {}", query);
    
    let sql = r#"
        SELECT f.id, f.name, f.extension, f.path, f.content, f.created_at, f.updated_at,
               rank
        FROM files_fts 
        JOIN files f ON files_fts.rowid = f.id
        WHERE files_fts MATCH ?1
        ORDER BY rank
        LIMIT ?2
    "#;

    let mut stmt = db.prepare(sql).map_err(|e| {
        println!("DEBUG: FTS search prepare error: {}", e);
        e.to_string()
    })?;
    
    let rows = stmt
        .query_map(params![query, limit], |row| {
            let id: i32 = row.get(0)?;
            let rank: f64 = row.get(7)?;
            
            println!("DEBUG: FTS result - ID: {}, Rank: {}", id, rank);
            
            Ok(SearchResult {
                id: id.to_string(),
                result_type: "file".to_string(),
                title: row.get(1)?,
                path: row.get(3)?,
                relevance_score: rank as f32,
                match_type: SearchMatchType::Text(rank as f32),
                snippet: None,
            })
        })
        .map_err(|e| {
            println!("DEBUG: FTS search query error: {}", e);
            e.to_string()
        })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| e.to_string())?);
    }
    
    println!("DEBUG: FTS search returned {} results", results.len());
    Ok(results)
}

pub fn combine_search_results(
    vector_results: Vec<SearchResult>,
    fts_results: Vec<SearchResult>,
    vector_weight: f32,
    text_weight: f32,
) -> Vec<SearchResult> {
    println!("DEBUG: Combining {} vector results with {} FTS results", 
             vector_results.len(), fts_results.len());
    
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
                
                println!("DEBUG: Hybrid result - ID: {}, Vector: {}, Text: {}, Combined: {}", 
                         existing.id, vector_score, text_score, existing.relevance_score);
            }
            None => {
                combined.insert(fts_result.id.clone(), fts_result);
            }
        }
    }

    let mut results: Vec<SearchResult> = combined.into_values().collect();
    results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
    
    println!("DEBUG: Final combined results: {}", results.len());
    results
}

pub fn hybrid_search_with_embedding(
    db: &Connection,
    normalized_embedding: &[f32],
    query: &str,
    filters: SearchFilters,
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    println!("DEBUG: Starting hybrid search for: {}", query);
    
    let intent = classify_intent(query);
    println!("DEBUG: Classified intent: {:?}", intent);
    
    // Adjust weights based on intent, giving more weight to folders for FolderBased intent
    let (vector_weight, text_weight, folder_weight) = match intent {
        SearchIntent::KeywordBased => (0.3, 0.7, 0.2),
        SearchIntent::NaturalLanguage => (0.7, 0.3, 0.2),
        SearchIntent::FolderBased => (0.4, 0.4, 0.2), // Emphasize folder results
        _ => (0.5, 0.5, 0.2),
    };
    
    println!(
        "DEBUG: Using weights - Vector: {}, Text: {}, Folder: {}",
        vector_weight, text_weight, folder_weight
    );

    // Perform vector and FTS searches
    let vector_results = search_similar_files(db, normalized_embedding, limit)?;
    let metadata_results = advanced_search(db, Some(query.to_string()), filters, limit)?;
    
    // Combine file results
    let mut combined_results = combine_search_results(
        vector_results,
        metadata_results,
        vector_weight,
        text_weight,
    );

    // Generate folder results
    let folder_results = extract_folder_results(&combined_results);
    println!("DEBUG: Generated {} folder results", folder_results.len());

    // Combine folder results with file results, applying folder weight
    for mut folder_result in folder_results {
        folder_result.relevance_score *= folder_weight;
        println!(
            "DEBUG: Folder result - ID: {}, Path: {}, Adjusted Score: {}",
            folder_result.id, folder_result.path, folder_result.relevance_score
        );
        combined_results.push(folder_result);
    }

    // Sort all results by relevance score
    combined_results.sort_by(|a, b| {
        b.relevance_score
            .partial_cmp(&a.relevance_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Limit the total number of results
    combined_results.truncate(limit);
    println!("DEBUG: Final hybrid search returned {} results", combined_results.len());
    
    Ok(combined_results)
}

// FIXED: Updated to match your actual database schema
pub fn advanced_search(
    db: &Connection,
    name_query: Option<String>,
    filters: SearchFilters,
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    println!("DEBUG: Starting advanced search");
    
    // FIXED: Only select columns that exist in your database
    let mut sql = String::from("SELECT id, name, extension, path, content, created_at, updated_at FROM files WHERE 1 = 1");
    let mut params: Vec<rusqlite::types::Value> = Vec::new();
    let mut idx = 1;

    if let Some(query) = name_query {
        sql.push_str(&format!(" AND name LIKE ?{}", idx));
        params.push(format!("%{}%", query).into());
        idx += 1;
        println!("DEBUG: Added name filter: {}", query);
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
            println!("DEBUG: Added extension filter: {:?}", extensions);
        }
    }

    if let Some(date_from) = &filters.date_from {
        sql.push_str(&format!(" AND created_at >= ?{}", idx));
        params.push(date_from.clone().into());
        idx += 1;
        println!("DEBUG: Added date_from filter: {}", date_from);
    }

    if let Some(date_to) = &filters.date_to {
        sql.push_str(&format!(" AND created_at <= ?{}", idx));
        params.push(date_to.clone().into());
        idx += 1;
        println!("DEBUG: Added date_to filter: {}", date_to);
    }

    // REMOVED: These filters since the columns don't exist
    // - author filter
    // - folder filter  
    // - min_size/max_size filters

    sql.push_str(&format!(" ORDER BY created_at DESC LIMIT ?{}", idx));
    params.push((limit as i64).into());

    println!("DEBUG: Final SQL: {}", sql);
    println!("DEBUG: Params count: {}", params.len());

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|v| v as _).collect();

    let mut stmt = db.prepare(&sql).map_err(|e| {
        println!("DEBUG: SQL prepare error: {}", e);
        e.to_string()
    })?;
    
    let rows = stmt
        .query_map(&*param_refs, |row| {
            let id: i32 = row.get(0)?;
            let name: String = row.get(1)?;
            let path: String = row.get(3)?;
            
            println!("DEBUG: Found file - ID: {}, Name: {}, Path: {}", id, name, path);
            
            Ok(SearchResult {
                id: id.to_string(),
                result_type: "file".to_string(),
                title: name,
                path,
                relevance_score: 1.0,
                match_type: SearchMatchType::Text(1.0),
                snippet: None,
            })
        })
        .map_err(|e| {
            println!("DEBUG: Query execution error: {}", e);
            e.to_string()
        })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| e.to_string())?);
    }

    println!("DEBUG: Advanced search returned {} results", results.len());
    Ok(results)
}

pub fn perform_file_search(
    query: String,
    top_k: Option<usize>,
    filters: Option<SearchFilters>,
) -> Result<Vec<SearchResult>, String> {
    println!("DEBUG: perform_file_search called with query: {}", query);
    
    let limit = top_k.unwrap_or(5);
    let filters = filters.unwrap_or_default();

    // Get embedding synchronously
    println!("DEBUG: Getting embedding for query");
    let query_embedding = embed_and_store::get_embedding(&query)
        .map_err(|e| {
            let error_msg = format!("Embedding error: {}", e);
            println!("DEBUG: {}", error_msg);
            error_msg
        })?;

    println!("DEBUG: Got embedding with {} dimensions", query_embedding.len());
    let normalized = embed_and_store::normalize(query_embedding);

    // Get database connection and perform search synchronously
    println!("DEBUG: Getting database connection");
    let db = database::get_connection();
    
    println!("DEBUG: Starting hybrid search");
    let results = hybrid_search_with_embedding(&db, &normalized, &query, filters, limit)?;
    
    println!("DEBUG: Search completed with {} results", results.len());
    Ok(results)
}

fn extract_folder_results(file_results: &[SearchResult]) -> Vec<SearchResult> {
    let mut folder_map: HashMap<String, usize> = HashMap::new();

    for result in file_results {
        if result.result_type == "file" {
            if let Some(parent) = Path::new(&result.path).parent() {
                let folder_path = parent.to_string_lossy().to_string();
                *folder_map.entry(folder_path).or_insert(0) += 1;
            } else {
                println!("DEBUG: No parent folder for path: {}", result.path);
            }
        }
    }

    let folder_results = folder_map
        .into_iter()
        .filter(|(_, count)| *count >= 2) // Only include folders with 2+ matching files
        .map(|(path, count)| {
            let folder_name = Path::new(&path)
                .file_name()
                .unwrap_or_else(|| Path::new(&path).as_os_str())
                .to_string_lossy()
                .to_string();

            let result = SearchResult {
                id: format!("folder_{}", path.replace('/', "_").replace('\\', "_")),
                result_type: "folder".to_string(),
                title: folder_name,
                path,
                relevance_score: (count as f32) * 0.1,
                match_type: SearchMatchType::Text((count as f32) * 0.1),
                snippet: Some(format!("{} matching files", count)),
            };
            
            println!(
                "DEBUG: Folder result - Path: {}, Count: {}, Score: {}",
                result.path, count, result.relevance_score
            );
            
            result
        })
        .collect::<Vec<SearchResult>>();

    println!("DEBUG: Total folder results generated: {}", folder_results.len());
    folder_results
}