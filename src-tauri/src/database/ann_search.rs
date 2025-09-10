use crate::database;
use crate::embed_and_store;
use arrow_array::{Int32Array, RecordBatch, StringArray};
use database::search::{
    advanced_search, classify_intent, combine_and_rank_results, parse_query, search_files_fts,
    search_folders_by_name, SearchFilters, SearchMatchType, SearchResult,
};
use rusqlite::{Connection, Result};
use lancedb::{Table, DistanceType};
use anyhow::{ anyhow};
use lancedb::query::QueryBase;
use lancedb::query::ExecutableQuery;
use futures::TryStreamExt;

// This function gets a connection to the LanceDB database and opens the 'files' table.
pub async fn get_lancedb_files_table() -> anyhow::Result<Table> {
    let database_path = crate::database::lancedb_ops::get_app_data_dir()
        .ok_or_else(|| anyhow!("Could not get app data directory"))?
        .join("my-lancedb");

    let db = lancedb::connect(database_path.to_str().unwrap())
        .execute()
        .await?;
    let files_table = db.open_table("files").execute().await?;
    Ok(files_table)
}

// --- MAIN SEARCH ORCHESTRATION ---

pub async fn perform_file_search(
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

    println!("DEBUG: Getting database connections");
    let sqlite_db = database::get_connection();
    let lancedb_files_table = get_lancedb_files_table().await.map_err(|e| e.to_string())?;

    println!("DEBUG: Starting hybrid search");
    let results = perform_hybrid_search(
        &sqlite_db,
        &lancedb_files_table,
        &normalized_embedding,
        &search_term,
        final_filters,
        limit,
    )
    .await?;

    println!("DEBUG: Search completed with {} results", results.len());
    Ok(results)
}

pub async fn perform_hybrid_search(
    db: &Connection,             // Your existing SQLite connection
    lancedb_files_table: &Table, // New: LanceDB files table
    normalized_embedding: &[f32],
    query: &str,
    filters: SearchFilters,
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    let intent = classify_intent(query);
    println!("DEBUG: Classified intent: {:?}", intent);

    // --- Stage 2: Execute Search Prongs ---
    let vector_results =
        search_similar_files_lancedb(lancedb_files_table, normalized_embedding, limit * 2).await?;
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

    combined_results.sort_by(|a, b| {
        b.relevance_score
            .partial_cmp(&a.relevance_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    combined_results.truncate(limit);

    println!(
        "DEBUG: Final hybrid search returned {} results",
        combined_results.len()
    );
    Ok(combined_results)
}

// --- CORRECTED LANCEDB SEARCH LOGIC FOR v0.22.0 ---

pub async fn search_similar_files_lancedb(
    lancedb_table: &Table,
    normalized_query: &[f32],
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    println!("DEBUG: Starting LanceDB vector search");

    // Execute vector search with explicit IVF_PQ parameters
    let mut search_result = lancedb_table
        .vector_search(normalized_query.to_vec())
        .map_err(|e| format!("Failed to create vector query: {}", e))?
        .distance_type(DistanceType::Cosine) // Match index training distance type
        .limit((limit as u64).try_into().unwrap()) // Ensure limit is u64
        .nprobes(20) // Search 5-15% of partitions for good recall/latency balance
        .refine_factor(10) // Add refine step for better accuracy
        .execute()
        .await
        .map_err(|e| format!("LanceDB search error: {}", e))?;

    let mut search_results = Vec::new();

    // Process results as a stream
    while let Some(batch) = search_result
        .try_next()
        .await
        .map_err(|e| format!("Error reading batch: {}", e))?
    {
        let batch: RecordBatch = batch;

        if batch.num_rows() == 0 {
            continue;
        }

        let id_array = batch
            .column_by_name("id")
            .ok_or("ID column not found in LanceDB result".to_string())?
            .as_any()
            .downcast_ref::<Int32Array>()
            .ok_or("Failed to downcast 'id' column".to_string())?;

        let name_array = batch
            .column_by_name("name")
            .ok_or("Name column not found in LanceDB result".to_string())?
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or("Failed to downcast 'name' column".to_string())?;

        let path_array = batch
            .column_by_name("path")
            .ok_or("Path column not found in LanceDB result".to_string())?
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or("Failed to downcast 'path' column".to_string())?;

        for i in 0..batch.num_rows() {
            let id = id_array.value(i);
            let name = name_array.value(i).to_string();
            let path = path_array.value(i).to_string();
            let distance = batch
                .column_by_name("_distance")
                .ok_or("Distance column not found".to_string())?
                .as_any()
                .downcast_ref::<arrow_array::Float32Array>()
                .ok_or("Failed to downcast '_distance' column".to_string())?
                .value(i);
            let relevance = 1.0 - distance;

            search_results.push(SearchResult {
                id: format!("file-{}", id),
                result_type: "file".to_string(),
                title: name,
                path,
                relevance_score: relevance,
                match_type: SearchMatchType::Vector(relevance),
                snippet: None,
            });
        }
    }

    Ok(search_results)
}