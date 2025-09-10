use crate::database;
use crate::embed_and_store;
use bytemuck::cast_slice;
use rusqlite::{params, Connection, Error, Result, Row};
use std::collections::HashMap;

// --- DATA STRUCTURES ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub id: String, // Unique ID: "file-123" or "folder-456"
    pub result_type: String, // "file" or "folder"
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

#[derive(Debug, PartialEq)]
pub enum SearchIntent {
    KeywordBased,
    NaturalLanguage,
    // Simplified for now, can be expanded later
}

// --- MAIN SEARCH ORCHESTRATION ---

pub fn perform_file_search(
    query: String,
    top_k: Option<usize>,
    filters: Option<SearchFilters>,
) -> Result<Vec<SearchResult>, String> {
    println!("DEBUG: perform_file_search called with query: '{}'", query);
    
    let limit = top_k.unwrap_or(10);
    let (search_term, parsed_filters) = parse_query(&query);
    let final_filters = filters.unwrap_or(parsed_filters);

    println!("DEBUG: Getting embedding for query");
    let query_embedding = embed_and_store::get_embedding(&search_term)
        .map_err(|e| format!("Embedding error: {}", e))?;
    let normalized_embedding = embed_and_store::normalize(query_embedding);

    println!("DEBUG: Getting database connection");
    let db = database::get_connection();
    
    println!("DEBUG: Starting hybrid search");
    let results = perform_hybrid_search(&db, &normalized_embedding, &search_term, final_filters, limit)?;
    
    println!("DEBUG: Search completed with {} results", results.len());
    Ok(results)
}

// --- HYBRID SEARCH LOGIC ---

pub fn perform_hybrid_search(
    db: &Connection,
    normalized_embedding: &[f32],
    query: &str,
    filters: SearchFilters,
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    let intent = classify_intent(query);
    println!("DEBUG: Classified intent: {:?}", intent);

    // --- Stage 2: Parallel Search Execution ---
    // (Executed sequentially here, but conceptually parallel)
    let vector_results = search_similar_files(db, normalized_embedding, limit * 2)?;
    let fts_results = search_files_fts(db, query, limit * 2)?;
    let folder_results = search_folders_by_name(db, query, limit)?;
    let metadata_results = advanced_search(db, Some(query.to_string()), filters, limit)?;

    // --- Stage 3 & 4: Combine, Rank, and Finalize ---
    let mut combined_results = combine_and_rank_results(
        intent,
        vector_results,
        fts_results,
        folder_results,
        metadata_results,
    );

    combined_results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));
    combined_results.truncate(limit);

    println!("DEBUG: Final hybrid search returned {} results", combined_results.len());
    Ok(combined_results)
}

// --- STAGE 1: QUERY ANALYSIS ---

pub fn classify_intent(query: &str) -> SearchIntent {
    // Simple intent classification
    if query.contains('"') || query.split_whitespace().count() < 4 {
        SearchIntent::KeywordBased
    } else {
        SearchIntent::NaturalLanguage
    }
}

pub fn parse_query(query: &str) -> (String, SearchFilters) {
    // Basic parsing, can be expanded
    let mut filters = SearchFilters::default();
    let search_term = query.to_string();
    // Placeholder for more advanced parsing logic if needed
    (search_term, filters)
}

// --- STAGE 2: SEARCH PRONGS ---

pub fn search_similar_files(
    db: &Connection,
    normalized_query: &[f32],
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    println!("DEBUG: Starting vector search");
    let vector_bytes: &[u8] = cast_slice(normalized_query);

    let sql = r#"
        SELECT f.id, f.name, f.path, f.score, distance
        FROM file_vec fv
        JOIN file_vec_map fvm ON fv.rowid = fvm.vec_rowid
        JOIN files f ON fvm.file_id = f.id
        WHERE fv.content_vec MATCH ?1 AND k = ?2
        ORDER BY distance
    "#;

    let mut stmt = db.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![vector_bytes, limit], |row| {
        let id: i64 = row.get(0)?;
        let distance: f32 = row.get(4)?;
        let relevance = 1.0 - distance;
        Ok(SearchResult {
            id: format!("file-{}", id),
            result_type: "file".to_string(),
            title: row.get(1)?,
            path: row.get(2)?,
            relevance_score: relevance,
            match_type: SearchMatchType::Vector(relevance),
            snippet: None,
        })
    }).map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn search_files_fts(
    db: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    println!("DEBUG: Starting FTS search for: {}", query);
    let sql = r#"
        SELECT f.id, f.name, f.path, f.score, rank
        FROM files_fts
        JOIN files f ON files_fts.rowid = f.id
        WHERE files_fts MATCH ?1
        ORDER BY rank
        LIMIT ?2
    "#;

    let mut stmt = db.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![query, limit], |row| {
        let id: i64 = row.get(0)?;
        let rank: f64 = row.get(4)?;
        Ok(SearchResult {
            id: format!("file-{}", id),
            result_type: "file".to_string(),
            title: row.get(1)?,
            path: row.get(2)?,
            relevance_score: rank as f32,
            match_type: SearchMatchType::Text(rank as f32),
            snippet: None,
        })
    }).map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn search_folders_by_name(
    db: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    println!("DEBUG: Starting folder name search for: {}", query);
    let sql = r#"
        SELECT id, name, path, score
        FROM folders
        WHERE name LIKE ?1 OR path LIKE ?1
        ORDER BY score DESC
        LIMIT ?2
    "#;

    let mut stmt = db.prepare(sql).map_err(|e| e.to_string())?;
    let query_param = format!("%{}%", query);
    let rows = stmt.query_map(params![query_param, limit], |row| {
        let id: i64 = row.get(0)?;
        let score: f32 = row.get(3)?;
        Ok(SearchResult {
            id: format!("folder-{}", id),
            result_type: "folder".to_string(),
            title: row.get(1)?,
            path: row.get(2)?,
            relevance_score: score,
            match_type: SearchMatchType::Text(score),
            snippet: None,
        })
    }).map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn advanced_search(
    db: &Connection,
    name_query: Option<String>,
    filters: SearchFilters,
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    println!("DEBUG: Starting advanced search");
    let mut sql = String::from("SELECT id, name, path, score FROM files WHERE 1 = 1");
    let mut params_values: Vec<rusqlite::types::Value> = Vec::new();

    if let Some(query) = name_query {
        sql.push_str(" AND name LIKE ?");
        params_values.push(format!("%{}%", query).into());
    }
    // Add other filters (extensions, dates) here...

    sql.push_str(" ORDER BY score DESC LIMIT ?");
    params_values.push((limit as i64).into());

    let params_refs: Vec<&dyn rusqlite::ToSql> = params_values.iter().map(|v| v as _).collect();
    let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(&*params_refs, |row| {
        let id: i64 = row.get(0)?;
        Ok(SearchResult {
            id: format!("file-{}", id),
            result_type: "file".to_string(),
            title: row.get(1)?,
            path: row.get(2)?,
            relevance_score: row.get(3)?,
            match_type: SearchMatchType::Text(row.get(3)?),
            snippet: None,
        })
    }).map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}


// --- STAGE 3: RERANKING AND SCORING ---

pub fn combine_and_rank_results(
    intent: SearchIntent,
    vector_results: Vec<SearchResult>,
    fts_results: Vec<SearchResult>,
    folder_results: Vec<SearchResult>,
    metadata_results: Vec<SearchResult>,
) -> Vec<SearchResult> {
    let mut combined: HashMap<String, (f32, SearchResult)> = HashMap::new();

    // Define weights based on intent
    let (w_sem, w_key, w_folder, w_meta) = match intent {
        SearchIntent::NaturalLanguage => (0.6, 0.2, 0.3, 0.1),
        SearchIntent::KeywordBased => (0.2, 0.6, 0.4, 0.2),
    };

    let hybrid_boost = 1.5;

    // Process vector results
    for r in vector_results {
        let score = r.relevance_score * w_sem;
        combined.insert(r.id.clone(), (score, r));
    }

    // Process and merge FTS results
    for r in fts_results {
        let score = r.relevance_score * w_key;
        if let Some(existing) = combined.get_mut(&r.id) {
            existing.0 += score * hybrid_boost; // Boost score for hybrid matches
        } else {
            combined.insert(r.id.clone(), (score, r));
        }
    }
    
    // Process and merge metadata results
    for r in metadata_results {
        let score = r.relevance_score * w_meta;
         if let Some(existing) = combined.get_mut(&r.id) {
            existing.0 += score;
        } else {
            combined.insert(r.id.clone(), (score, r));
        }
    }

    // Process and merge folder results
    for r in folder_results {
        let score = r.relevance_score * w_folder;
        if let Some(existing) = combined.get_mut(&r.id) {
            existing.0 += score * hybrid_boost;
        } else {
            combined.insert(r.id.clone(), (score, r));
        }
    }

    // Finalize scores and collect results
    combined.into_values().map(|(score, mut result)| {
        result.relevance_score = score;
        result
    }).collect()
}
