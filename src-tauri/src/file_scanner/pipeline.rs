// The core scanning pipeline stages
use super::content::{chunk_text, create_metadata_string, read_file_content_with_category};
use super::db::{file_exists, insert_file_embedding, insert_file_metadata};
use super::lancedb::{insert_file_metadata_lancedb};
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
pub async fn store_results(
    db: &Connection,
    files: &[FileContent],
    embeddings: &[Vec<f32>],
    file_chunk_map: &[(String, Vec<usize>)],
    app: &AppHandle,
) -> Result<usize, String> {
    let mut total_inserted_count = 0;

    for chunk in files.chunks(BATCH_SIZE) {
        let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;

        for (i, file) in chunk.iter().enumerate() {
            emit_scan_progress(
                app,
                (total_inserted_count + i + 1) as u64,
                files.len() as u64,
                file.path.clone(),
                "storing",
            );
            let file_vector = file_chunk_map
                .iter()
                .find(|(path, _)| path == &file.path)
                .and_then(|(_, indices)| embeddings.get(indices[0]).map(|v| v.clone()));

            let file_id = insert_file_metadata(&tx, file).map_err(|e| e.to_string())?;
            
            insert_file_metadata_lancedb(file, file_vector)
                .await
                .map_err(|e| e.to_string())?;

            let chunk_indices = file_chunk_map
                .iter()
                .find(|(path, _)| path == &file.path)
                .map(|(_, indices)| indices);

            if let Some(indices) = chunk_indices {
                for &chunk_idx in indices {
                    if let Some(vector) = embeddings.get(chunk_idx) {
                        match insert_file_embedding(&tx, file_id, vector.clone()) {
                            Ok(_) => {
                                println!("Successfully saved embedding for file_id: {}", file_id)
                            }
                            Err(e) => {
                                eprintln!("Failed to save embedding for file_id {}: {}", file_id, e)
                            }
                        }
                    }
                }
            }
        }

        tx.commit().map_err(|e| e.to_string())?;
        total_inserted_count += chunk.len();
    }

    Ok(total_inserted_count)
}
