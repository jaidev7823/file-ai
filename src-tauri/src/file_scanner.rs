use crate::embed_and_store;
use crate::embed_and_store::normalize;
use bytemuck::cast_slice;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::PathBuf;
use std::{fs, path::Path};
use tauri::{AppHandle, Emitter, Manager};
use tokio::process::Command as AsyncCommand;
use tokio::runtime::Runtime;
use walkdir::WalkDir;

fn emit_scan_progress(
    app: &AppHandle,
    current: u64,
    total: u64,
    current_file: impl Into<String>,
    stage: &str,
) {
    let payload = serde_json::json!({
        "current": current,
        "total": total,
        "current_file": current_file.into(),
        "stage": stage,
    });

    let _ = app.emit("scan_progress", &payload);
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.emit("scan_progress", &payload);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub id: i32,
    pub name: String,
    pub extension: String,
    pub path: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    pub embedding: Vec<f32>,
}

pub fn find_text_files(conn: &Connection, app: &AppHandle) -> Result<Vec<String>, String> {
    let include_paths =
        crate::database::rules::get_included_paths_sync(conn).map_err(|e| e.to_string())?;
    let include_exts =
        crate::database::rules::get_included_extensions_sync(conn).map_err(|e| e.to_string())?;
    let exclude_folders =
        crate::database::rules::get_excluded_folder_sync(conn).map_err(|e| e.to_string())?;

    let mut scanned_count = 0;
    let mut found_files = Vec::new();

    emit_scan_progress(app, 0, 0, "", "scanning");

    for base_path in include_paths {
        let base = PathBuf::from(&base_path);
        if !base.exists() {
            continue;
        }

        for entry in WalkDir::new(base)
            .into_iter()
            .filter_entry(|e| {
                if e.file_type().is_dir() {
                    if let Some(name) = e.file_name().to_str() {
                        let lname = name.to_lowercase();
                        if lname.starts_with('.') {
                            return false;
                        }
                        if exclude_folders
                            .iter()
                            .any(|ex| ex.eq_ignore_ascii_case(&lname))
                        {
                            return false;
                        }
                    }
                }
                true
            })
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if !include_exts.iter().any(|inc| inc.eq_ignore_ascii_case(ext)) {
                    continue;
                }
            } else {
                continue;
            }

            let path_str = path.to_string_lossy().to_string();
            found_files.push(path_str.clone());
            scanned_count += 1;

            if scanned_count % 50000 == 0 {
                emit_scan_progress(app, scanned_count, 0, &path_str, "scanning");
            }
        }
    }

    emit_scan_progress(app, scanned_count, scanned_count, "", "complete");
    Ok(found_files)
}

pub async fn extract_pdf_text(path: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let max_pages = 25;

    tokio::task::spawn_blocking({
        let path = path.to_string();
        move || -> Result<String, Box<dyn Error + Send + Sync>> {
            use lopdf::Document;
            use rayon::prelude::*;

            // Add specific error handling for PDF loading
            let doc = match Document::load(&path) {
                Ok(doc) => doc,
                Err(e) => return Err(format!("Failed to load PDF '{}': {}", path, e).into()),
            };

            let pages = doc.get_pages();
            let page_ids: Vec<u32> = pages.keys().copied().take(max_pages as usize).collect();

            if page_ids.is_empty() {
                return Err("PDF appears to be empty".into());
            }

            let results: Result<Vec<String>, _> = page_ids
                .par_iter()
                .map(|&page_id| {
                    doc.extract_text(&[page_id])
                        .map_err(|e| format!("Failed to extract text from page {}: {}", page_id, e))
                })
                .collect();

            match results {
                Ok(texts) => {
                    let combined = texts.join("\n");
                    if combined.trim().is_empty() {
                        Err("No text content found in PDF".into())
                    } else {
                        Ok(combined)
                    }
                }
                Err(e) => Err(e.into()),
            }
        }
    })
    .await?
}

pub async fn read_file_content(
    path: &str,
    max_chars: Option<usize>,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let path_obj = Path::new(path);
    let extension = path_obj
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut content = match extension.as_str() {
        "pdf" => extract_pdf_text(path).await?,
        _ => {
            if let Ok(metadata) = fs::metadata(path) {
                if metadata.len() > 10_000_000 {
                    return Err("File too large for processing".into());
                }
            }
            match fs::read_to_string(path) {
                Ok(content) => content,
                Err(_) => {
                    let bytes = fs::read(path)?;
                    String::from_utf8_lossy(&bytes).into_owned()
                }
            }
        }
        "csv" | "tsv" => {
            let path = path.to_string();
            let ext = extension.clone();
            let max_rows = 1000; // 👈 cap rows here

            tokio::task::spawn_blocking(
                move || -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
                    use csv::ReaderBuilder;

                    let delimiter = if ext == "tsv" { b'\t' } else { b',' };

                    let mut rdr = ReaderBuilder::new()
                        .delimiter(delimiter)
                        .flexible(true)
                        .from_path(&path)?;

                    let headers = rdr
                        .headers()
                        .map(|h| h.clone())
                        .unwrap_or(csv::StringRecord::new());

                    let mut rows: Vec<String> = Vec::new();

                    for (i, result) in rdr.records().enumerate() {
                        if i >= max_rows {
                            rows.push(format!("[truncated after {} rows]", max_rows));
                            break;
                        }

                        let record = result?;
                        let row_text = if !headers.is_empty() && headers.len() == record.len() {
                            headers
                                .iter()
                                .zip(record.iter())
                                .map(|(h, v)| format!("{}: {}", h, v))
                                .collect::<Vec<_>>()
                                .join(" | ")
                        } else {
                            record
                                .iter()
                                .enumerate()
                                .map(|(i, v)| format!("col{}: {}", i + 1, v))
                                .collect::<Vec<_>>()
                                .join(" | ")
                        };
                        rows.push(row_text);
                    }

                    Ok(rows.join("\n"))
                },
            )
            .await??
        }
    };

    if let Some(max) = max_chars {
        if content.len() > max {
            content.truncate(max);
        }
    }

    Ok(content)
}

pub fn read_files_content(paths: &[String], max_chars: Option<usize>) -> Vec<FileContent> {
    let rt = Runtime::new().expect("Failed to create runtime");
    rt.block_on(async {
        let mut results = Vec::new();
        for path in paths {
            match read_file_content(path, max_chars).await {
                Ok(content) => results.push(FileContent {
                    path: path.clone(),
                    content,
                    embedding: Vec::new(),
                }),
                Err(e) => {
                    eprintln!("Failed to read file {}: {}", path, e);
                }
            }
        }
        results
    })
}

fn file_exists(db: &Connection, path: &str) -> Result<bool> {
    let mut stmt = db.prepare("SELECT COUNT(*) FROM files WHERE path = ?1")?;
    let count: i64 = stmt.query_row(params![path], |row| row.get(0))?;
    Ok(count > 0)
}

fn chunk_text(text: &str, max_words: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut chunks = Vec::new();

    for chunk in words.chunks(max_words) {
        let chunk_text = chunk.join(" ");
        if chunk_text.len() > 50 {
            chunks.push(chunk_text);
        }
    }

    chunks
}

const BATCH_SIZE: usize = 1000;

pub fn scan_and_store_files(
    db: &Connection,
    dir: &str,
    max_chars: Option<usize>,
    max_file_size: Option<u64>,
    app: tauri::AppHandle,
) -> Result<usize, String> {
    let rt = Runtime::new().map_err(|e| e.to_string())?;

    let _ = app.emit(
        "scan_progress",
        crate::commands::ScanProgress {
            current: 0,
            total: 0,
            current_file: "Scanning for files...".to_string(),
            stage: "scanning".to_string(),
        },
    );

    let paths: Vec<String> = find_text_files(db, &app).map_err(|e| e.to_string())?;
    println!("Found {} files to process", paths.len());

    if paths.is_empty() {
        let _ = app.emit(
            "scan_progress",
            crate::commands::ScanProgress {
                current: 0,
                total: 0,
                current_file: "No files found to process".to_string(),
                stage: "complete".to_string(),
            },
        );
        return Ok(0);
    }

    let mut new_files_to_process = Vec::new();

    for (i, path) in paths.iter().enumerate() {
        let _ = app.emit(
            "scan_progress",
            crate::commands::ScanProgress {
                current: (i + 1) as u64,
                total: paths.len() as u64,
                current_file: path.clone(),
                stage: "reading metadata".to_string(),
            },
        );

        if file_exists(db, path).map_err(|e| e.to_string())? {
            continue;
        }

        // Check if this file should have its content crawled based on path rules
        let should_crawl_content = check_path_rules(path);
        
        let content = if should_crawl_content {
            // Only read content for files that match path rules
            match rt.block_on(read_file_content(path, max_chars)) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("Failed to read file content {}: {}", path, e);
                    String::new()
                }
            }
        } else {
            // For files outside path rules, we still index metadata but no content
            String::new()
        };

        new_files_to_process.push(FileContent {
            path: path.clone(),
            content,
            embedding: Vec::new(),
        });
    }

    println!("Identified {} new files for processing", new_files_to_process.len());

    if new_files_to_process.is_empty() {
        let _ = app.emit(
            "scan_progress",
            crate::commands::ScanProgress {
                current: paths.len() as u64,
                total: paths.len() as u64,
                current_file: "No new files to process".to_string(),
                stage: "complete".to_string(),
            },
        );
        return Ok(0);
    }

    // Enhanced metadata generation with folder hierarchy
    let mut all_chunks_to_embed: Vec<String> = Vec::new();
    let mut file_path_to_chunk_indices: Vec<(String, Vec<usize>)> = Vec::new();

    for file_content in new_files_to_process.iter() {
        let mut current_file_chunk_indices = Vec::new();

        // Extract enhanced metadata including folder structure
        let path_obj = Path::new(&file_content.path);
        let file_name = path_obj
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        
        let file_stem = path_obj
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
            
        let extension = path_obj
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();

        // Extract folder hierarchy
        let parent_folders: Vec<String> = path_obj
            .parent()
            .map(|p| {
                p.components()
                    .filter_map(|comp| {
                        match comp {
                            std::path::Component::Normal(os_str) => os_str.to_str().map(|s| s.to_string()),
                            std::path::Component::RootDir => Some("root".to_string()),
                            std::path::Component::Prefix(prefix) => {
                                // Handle drive letters like C:, D:, etc.
                                Some(format!("drive_{}", prefix.as_os_str().to_str().unwrap_or("unknown")))
                            },
                            _ => None,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let immediate_parent = parent_folders.last().cloned().unwrap_or_default();
        let folder_hierarchy = parent_folders.join(" > ");

        // Create comprehensive metadata string
        let metadata_string = format!(
            "filename: {} stem: {} extension: {} path: {} parent_folder: {} folder_hierarchy: {} drive: {}",
            file_name,
            file_stem,
            extension,
            file_content.path,
            immediate_parent,
            folder_hierarchy,
            extract_drive(&file_content.path)
        );

        // Add metadata chunk
        if !metadata_string.trim().is_empty() {
            current_file_chunk_indices.push(all_chunks_to_embed.len());
            all_chunks_to_embed.push(metadata_string);
        }

        // Add content chunks only if we have content (i.e., file was within path rules)
        if !file_content.content.trim().is_empty() {
            let content_chunks = chunk_text(&file_content.content, 200);
            for chunk in content_chunks {
                if !chunk.trim().is_empty() {
                    current_file_chunk_indices.push(all_chunks_to_embed.len());
                    all_chunks_to_embed.push(chunk);
                }
            }
        }
        
        file_path_to_chunk_indices.push((file_content.path.clone(), current_file_chunk_indices));
    }

    println!("Total {} text units (enhanced metadata + content chunks) for embedding", all_chunks_to_embed.len());

    // Rest of the embedding and storage logic remains the same...
    let all_embeddings = if all_chunks_to_embed.is_empty() {
        Vec::new()
    } else {
        let _ = app.emit(
            "scan_progress",
            crate::commands::ScanProgress {
                current: 0,
                total: all_chunks_to_embed.len() as u64,
                current_file: "Generating embeddings for all text units...".to_string(),
                stage: "embedding".to_string(),
            },
        );

        let app_clone = app.clone();
        embed_and_store::get_batch_embeddings_with_progress(&all_chunks_to_embed, move |current, total| {
            let _ = app_clone.emit(
                "scan_progress",
                crate::commands::ScanProgress {
                    current: current as u64,
                    total: total as u64,
                    current_file: format!("Processing embedding {} of {}", current, total),
                    stage: "embedding".to_string(),
                },
            );
        })
        .map_err(|e| e.to_string())?
    };

    // Storage logic with enhanced metadata - BATCHED
    let mut total_inserted_count = 0;
    let mut tx = db.unchecked_transaction().map_err(|e| e.to_string())?;

    for (file_idx, file_content) in new_files_to_process.iter().enumerate() {
        // Commit transaction and start a new one if the batch size is reached
        if file_idx > 0 && file_idx % BATCH_SIZE == 0 {
            tx.commit().map_err(|e| e.to_string())?;
            tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
            println!("Batch committed. Starting a new batch...");
        }

        let _ = app.emit(
            "scan_progress",
            crate::commands::ScanProgress {
                current: (file_idx + 1) as u64,
                total: new_files_to_process.len() as u64,
                current_file: file_content.path.clone(),
                stage: "storing".to_string(),
            },
        );

        let now = Utc::now();
        let path_obj = Path::new(&file_content.path);
        let file_name = path_obj
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let extension = path_obj
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();

        // Enhanced file table with folder information
        tx.execute(
            "INSERT INTO files (name, extension, path, content, parent_folder, folder_hierarchy, drive, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                file_name,
                extension,
                file_content.path,
                file_content.content,
                extract_immediate_parent(&file_content.path),
                extract_folder_hierarchy(&file_content.path),
                extract_drive(&file_content.path),
                now.to_rfc3339(),
                now.to_rfc3339(),
            ],
        )
        .map_err(|e| e.to_string())?;

        let file_id = tx.last_insert_rowid();

        // Store embeddings logic remains the same...
        let current_file_all_chunk_indices = file_path_to_chunk_indices
            .iter()
            .find(|(path, _)| path == &file_content.path)
            .map(|(_, indices)| indices.clone())
            .unwrap_or_default();

        for chunk_idx_in_all_chunks in current_file_all_chunk_indices {
            if chunk_idx_in_all_chunks < all_embeddings.len() {
                let vector = normalize(all_embeddings[chunk_idx_in_all_chunks].clone());

                if !vector.is_empty() {
                    let vector_bytes: &[u8] = cast_slice(&vector);
                    tx.execute(
                        "INSERT INTO file_vec(content_vec) VALUES(?1)",
                        params![vector_bytes],
                    )
                    .map_err(|e| e.to_string())?;

                    let vec_rowid = tx.last_insert_rowid();

                    tx.execute(
                        "INSERT INTO file_vec_map(vec_rowid, file_id) VALUES(?1, ?2)",
                        params![vec_rowid, file_id],
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
        }
        total_inserted_count += 1;
    }

    // Commit any remaining files in the last batch
    tx.commit().map_err(|e| e.to_string())?;
    
    let _ = app.emit(
        "scan_progress",
        crate::commands::ScanProgress {
            current: total_inserted_count as u64,
            total: total_inserted_count as u64,
            current_file: format!("Completed! Processed {} new files", total_inserted_count),
            stage: "complete".to_string(),
        },
    );

    Ok(total_inserted_count)
}

// Helper functions you'll need to add:

fn check_path_rules(file_path: &str) -> bool {
    // Implement your path rules here
    // Return true if this file's content should be crawled
    // Return false if only metadata should be indexed
    
    // Example implementation:
    let allowed_folders = vec![
        "Documents", "Projects", "Code", "Work"
    ];
    
    allowed_folders.iter().any(|folder| file_path.contains(folder))
}

fn extract_drive(file_path: &str) -> String {
    if let Some(first_component) = Path::new(file_path).components().next() {
        match first_component {
            std::path::Component::Prefix(prefix) => {
                prefix.as_os_str().to_str().unwrap_or("unknown").to_string()
            },
            _ => "unknown".to_string(),
        }
    } else {
        "unknown".to_string()
    }
}

fn extract_immediate_parent(file_path: &str) -> String {
    Path::new(file_path)
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string()
}

fn extract_folder_hierarchy(file_path: &str) -> String {
    let path_obj = Path::new(file_path);
    if let Some(parent) = path_obj.parent() {
        parent.components()
            .filter_map(|comp| {
                match comp {
                    std::path::Component::Normal(os_str) => os_str.to_str(),
                    std::path::Component::Prefix(prefix) => prefix.as_os_str().to_str(),
                    _ => None,
                }
            })
            .collect::<Vec<_>>()
            .join(" > ")
    } else {
        String::new()
    }
}