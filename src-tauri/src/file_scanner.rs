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
        return Ok(0);
    }

    let mut contents = rt.block_on(async {
        let mut results = Vec::new();
        for (i, path) in paths.iter().enumerate() {
            let _ = app.emit(
                "scan_progress",
                crate::commands::ScanProgress {
                    current: (i + 1) as u64,
                    total: paths.len() as u64,
                    current_file: path.clone(),
                    stage: "reading".to_string(),
                },
            );

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
    });

    println!("Successfully read {} files", contents.len());

    let mut new_contents = Vec::new();
    for file_content in contents {
        if !file_exists(db, &file_content.path).map_err(|e| e.to_string())? {
            new_contents.push(file_content);
        }
    }

    if new_contents.is_empty() {
        let _ = app.emit(
            "scan_progress",
            crate::commands::ScanProgress {
                current: paths.len() as u64,
                total: paths.len() as u64,
                current_file: "No new files to process".to_string(),
                stage: "complete".to_string(),
            },
        );
        println!("No new files to process");
        return Ok(0);
    }

    let _ = app.emit(
        "scan_progress",
        crate::commands::ScanProgress {
            current: 0,
            total: new_contents.len() as u64,
            current_file: "Preparing text chunks...".to_string(),
            stage: "embedding".to_string(),
        },
    );

    let mut all_chunks = Vec::new();
    for file_content in new_contents.iter() {
        if !file_content.content.trim().is_empty() {
            let chunks = chunk_text(&file_content.content, 200);
            for chunk in chunks {
                if !chunk.trim().is_empty() {
                    all_chunks.push(chunk);
                }
            }
        }
    }

    println!("Processing {} chunks for embeddings", all_chunks.len());

    let embeddings = if all_chunks.is_empty() {
        let _ = app.emit(
            "scan_progress",
            crate::commands::ScanProgress {
                current: 0,
                total: 0,
                current_file: "No text chunks to process".to_string(),
                stage: "embedding".to_string(),
            },
        );
        Vec::new()
    } else {
        let _ = app.emit(
            "scan_progress",
            crate::commands::ScanProgress {
                current: 0,
                total: all_chunks.len() as u64,
                current_file: "Generating embeddings...".to_string(),
                stage: "embedding".to_string(),
            },
        );

        let app_clone = app.clone();
        embed_and_store::get_batch_embeddings_with_progress(&all_chunks, move |current, total| {
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

    let _ = app.emit(
        "scan_progress",
        crate::commands::ScanProgress {
            current: 0,
            total: new_contents.len() as u64,
            current_file: "Storing in database...".to_string(),
            stage: "storing".to_string(),
        },
    );

    let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;

    let mut inserted_count = 0;
    let mut current_chunk_idx = 0;

    for (file_idx, file_content) in new_contents.iter().enumerate() {
        let _ = app.emit(
            "scan_progress",
            crate::commands::ScanProgress {
                current: (file_idx + 1) as u64,
                total: new_contents.len() as u64,
                current_file: file_content.path.clone(),
                stage: "storing".to_string(),
            },
        );

        let now = Utc::now();
        let file_name = Path::new(&file_content.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let extension = Path::new(&file_content.path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();

        tx.execute(
            "INSERT INTO files (name, extension, path, content, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                file_name,
                extension,
                file_content.path,
                file_content.content,
                now.to_rfc3339(),
                now.to_rfc3339(),
            ],
        )
        .map_err(|e| e.to_string())?;

        let file_id = tx.last_insert_rowid();

        let file_chunks = chunk_text(&file_content.content, 200);
        for chunk in file_chunks {
            if chunk.trim().is_empty() {
                continue;
            }

            if current_chunk_idx < embeddings.len() {
                let vector = normalize(embeddings[current_chunk_idx].clone());
                current_chunk_idx += 1;

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

        println!("Processed: {}", file_content.path);
        inserted_count += 1;
    }

    tx.commit().map_err(|e| e.to_string())?;

    let _ = app.emit(
        "scan_progress",
        crate::commands::ScanProgress {
            current: inserted_count as u64,
            total: inserted_count as u64,
            current_file: format!("Completed! Processed {} files", inserted_count),
            stage: "complete".to_string(),
        },
    );

    println!("Successfully inserted {} files", inserted_count);
    Ok(inserted_count)
}
