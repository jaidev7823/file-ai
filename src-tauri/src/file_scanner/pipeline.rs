// The core scanning pipeline stages
use super::content::{chunk_text, create_metadata_string, read_file_content_with_category};
use super::db::{file_exists, insert_file_embedding, insert_file_metadata};
use super::lancedb::{insert_file_metadata_lancedb, insert_file_embedding_lancedb};
use super::scoring::{calculate_file_score, check_phase1_rules};
use super::types::{FileCategory, FileContent};
use super::utils::emit_scan_progress;
use rusqlite::Connection;
use std::fs;
use std::path::Path;
use tauri::AppHandle;
use tokio::runtime::Runtime;

const BATCH_SIZE: usize = 1000;

/// Stage 1: Filters a list of paths for new files and reads their content based on scan rules.
pub fn prepare_files_for_processing(
    db: &Connection,
    paths: &[String],
    included_paths: &[String],
    is_phase2: bool,
    max_chars: Option<usize>,
    rt: &Runtime,
    app: &AppHandle,
) -> Result<Vec<FileContent>, String> {
    let mut new_files_to_process = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        emit_scan_progress(
            app,
            (i + 1) as u64,
            paths.len() as u64,
            path.clone(),
            "reading metadata",
        );

        if file_exists(db, path).map_err(|e| e.to_string())? {
            continue;
        }

        let score = match fs::metadata(path) {
            Ok(metadata) => calculate_file_score(path, &metadata, included_paths),
            Err(_) => 0.0,
        };

        let (should_crawl_content, category, content) = if is_phase2 {
            let path_obj = Path::new(path);
            let extension = path_obj.extension().and_then(|e| e.to_str()).unwrap_or("");
            let category = FileCategory::from_extension(extension);
            (false, category, String::new())
        } else {
            let (should_crawl, _) = check_phase1_rules(db, path)?;
            let (content, category) = match rt.block_on(read_file_content_with_category(
                path,
                max_chars,
                should_crawl,
            )) {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("Failed to read file content {}: {}", path, e);
                    (String::new(), FileCategory::Unknown)
                }
            };
            (should_crawl, category, content)
        };

        new_files_to_process.push(FileContent {
            path: path.clone(),
            content,
            embedding: Vec::new(),
            category,
            content_processed: should_crawl_content,
            score,
        });
    }
    Ok(new_files_to_process)
}

/// Stage 2: Generates metadata and content chunks for a list of files.
pub fn build_embedding_chunks(files: &[FileContent]) -> (Vec<String>, Vec<(String, Vec<usize>)>) {
    let mut all_chunks = Vec::new();
    let mut file_chunk_map = Vec::new();

    for file in files {
        let mut current_file_chunk_indices = Vec::new();
        let path_obj = Path::new(&file.path);

        let metadata_string = create_metadata_string(path_obj);
        if !metadata_string.trim().is_empty() {
            current_file_chunk_indices.push(all_chunks.len());
            all_chunks.push(metadata_string);
        }

        if !file.content.trim().is_empty() {
            let content_chunks = chunk_text(&file.content, 200);
            for chunk in content_chunks {
                if !chunk.trim().is_empty() {
                    current_file_chunk_indices.push(all_chunks.len());
                    all_chunks.push(chunk);
                }
            }
        }
        file_chunk_map.push((file.path.clone(), current_file_chunk_indices));
    }
    (all_chunks, file_chunk_map)
}

/// Stage 3: Stores file metadata and embeddings in the database in batches.
use super::lancedb::{get_lancedb_tables, insert_file_embedding_batch, insert_file_metadata_batch};

/// Stage 3: Stores file metadata and embeddings in the database in batches.
pub async fn store_results(
    db: &Connection,
    files: &[FileContent],
    all_chunks: &[String], // This is required to get the text for embeddings
    embeddings: &[Vec<f32>],
    file_chunk_map: &[(String, Vec<usize>)],
    app: &AppHandle,
) -> Result<usize, String> {
    // 1. Open LanceDB tables ONCE.
    let (files_table, file_emb_table) = get_lancedb_tables()
        .await
        .map_err(|e| format!("Failed to open LanceDB tables: {}", e))?;

    // --- Batch insert file metadata ---
    emit_scan_progress(
        app,
        1,
        3,
        "".to_string(),
        "storing file metadata",
    );

    // Associate the correct top-level embedding (usually for metadata) with each file.
    let file_vectors: Vec<Option<Vec<f32>>> = files
        .iter()
        .map(|file| {
            file_chunk_map
                .iter()
                .find(|(path, _)| path == &file.path)
                .and_then(|(_, indices)| indices.get(0)) // Get first chunk index (metadata)
                .and_then(|&idx| embeddings.get(idx).cloned())
        })
        .collect();

    // Use the batch metadata insertion function
    let inserted_file_ids = insert_file_metadata_batch(&files_table, files, file_vectors)
        .await
        .map_err(|e| format!("Batch metadata insert failed: {}", e))?;

    // Create a map from path to new LanceDB file ID for associating embeddings
    let path_to_new_id: std::collections::HashMap<_, _> = files
        .iter()
        .map(|f| f.path.clone())
        .zip(inserted_file_ids)
        .collect();

    // --- Batch insert file embeddings ---
    emit_scan_progress(
        app,
        2,
        3,
        "".to_string(),
        "storing content embeddings",
    );

    let mut embedding_data_batch = Vec::new();
    for (path, chunk_indices) in file_chunk_map {
        if let Some(file_id) = path_to_new_id.get(path) {
            for &chunk_idx in chunk_indices {
                if let (Some(chunk_text), Some(vector)) = (all_chunks.get(chunk_idx), embeddings.get(chunk_idx)) {
                    embedding_data_batch.push((*file_id, chunk_text.clone(), vector.clone()));
                }
            }
        }
    }

    if !embedding_data_batch.is_empty() {
        insert_file_embedding_batch(&file_emb_table, &embedding_data_batch)
            .await
            .map_err(|e| format!("Batch embedding insert failed: {}", e))?;
    }

    emit_scan_progress(app, 3, 3, "".to_string(), "storage complete");

    Ok(files.len())
}
