// src-tauri/src/file_scanner.rs
// use crate::database::rules::{get_excluded_folder_sync, get_included_extensions_sync};
use crate::embed_and_store;
use bytemuck::cast_slice;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::{collections::HashSet, fs, path::Path};
use tauri::Emitter;
use tauri::Manager;
use walkdir::WalkDir;
use tauri::AppHandle;
use std::path::PathBuf;

use crate::embed_and_store::normalize;

fn emit_scan_progress(app: &AppHandle, current: u64, total: u64, current_file: impl Into<String>, stage: &str) {
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct File {
    pub id: i32,
    pub name: String,
    pub extension: String,
    pub path: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}


#[cfg(target_os = "windows")]
fn detect_windows_drives() -> HashSet<String> {
    let mut drives = HashSet::new();
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        if Path::new(&drive).exists() {
            drives.insert(drive);
        }
    }
    drives
}

#[derive(Serialize, Deserialize)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    pub embedding: Vec<f32>,
}

#[derive(Clone, serde::Serialize)]
pub struct ScanProgress {
    pub current: u64,
    pub total: u64,
    pub current_file: String,
    pub stage: String,
}

pub fn find_text_files(
    conn: &Connection,
    max_file_size: Option<u64>,
    app: &AppHandle,
) -> Result<Vec<String>, String> {
    // Load include & exclude rules from DB
    let include_paths: Vec<String> = load_paths(conn, "include")?;
    let exclude_paths: Vec<String> = load_paths(conn, "exclude")?;
    let include_exts: Vec<String> = load_extensions(conn)?;

    // Fallback include paths if DB has none
    let mut search_paths: Vec<String> = include_paths.clone();
    if search_paths.is_empty() {
        #[cfg(target_os = "windows")]
        {
            let drives = detect_windows_drives();
            if !drives.is_empty() {
                search_paths.extend(drives.into_iter());
            }
        }

        if search_paths.is_empty() {
            if let Some(home) = dirs::home_dir() {
                search_paths.push(home.to_string_lossy().to_string());
            }
        }
    }

    // Count total upfront
    let mut estimated_total: u64 = 0;
    for base_path in &search_paths {
        let base = PathBuf::from(base_path);
        if !base.exists() {
            continue;
        }
        estimated_total += WalkDir::new(base)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                let path_str = e.path().to_string_lossy();
                !exclude_paths.iter().any(|ex| path_str.starts_with(ex))
            })
            .count() as u64;
    }

    // Emit initial progress
    emit_scan_progress(app, 0, estimated_total, "", "scanning");

    // Actual scanning
    let mut scanned_count: u64 = 0;
    let mut found_files = Vec::new();

    for base_path in search_paths {
        let base = PathBuf::from(base_path);
        if !base.exists() {
            continue;
        }

        for entry in WalkDir::new(base).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();

            // Skip excluded paths early
            let path_str = path.to_string_lossy();
            if exclude_paths.iter().any(|ex| path_str.starts_with(ex)) {
                continue;
            }

            // Only files
            if !path.is_file() {
                continue;
            }

            // Extension filter
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if !include_exts
                    .iter()
                    .any(|inc| inc.eq_ignore_ascii_case(ext))
                {
                    continue;
                }
            } else {
                continue;
            }

            // File size limit
            if let Some(limit) = max_file_size {
                if let Ok(meta) = path.metadata() {
                    if meta.len() > limit {
                        continue;
                    }
                }
            }

            // If parseable, keep it
            found_files.push(path_str.to_string());

            // Emit progress
            scanned_count += 1;
            emit_scan_progress(app, scanned_count, estimated_total, path_str.to_string(), "scanning");
        }
    }

    // Emit complete stage
    emit_scan_progress(app, scanned_count, estimated_total, "", "complete");

    Ok(found_files)
}


// Helpers to load rules from DB
fn load_paths(conn: &Connection, rule_type: &str) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT path FROM path_rules WHERE rule_type = ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([rule_type], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let mut paths = Vec::new();
    for r in rows {
        if let Ok(p) = r {
            paths.push(p);
        }
    }
    Ok(paths)
}

fn load_extensions(conn: &Connection) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT extension FROM extension_rules") // no WHERE is_allowed
        .map_err(|e| e.to_string())?;

    let exts = stmt
        .query_map([], |row| {
            let ext: String = row.get(0)?;
            Ok(ext)
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();

    Ok(exts)
}


/// Optimized PDF text extraction with better error handling
fn extract_pdf_text(path: &str) -> Result<String, Box<dyn Error>> {
    use lopdf::Document;

    let doc = Document::load(path)?;
    let mut text = String::new();
    let pages = doc.get_pages();

    // Process only first N pages for very large PDFs
    let max_pages = 50;
    let page_ids: Vec<u32> = pages.keys().copied().take(max_pages).collect();

    for page_id in page_ids {
        if let Ok(page_text) = doc.extract_text(&[page_id]) {
            text.push_str(&page_text);
            text.push('\n');
        }
    }

    Ok(text)
}

/// Optimized file content reading with better memory management
pub fn read_file_content(path: &str, max_chars: Option<usize>) -> Result<String, Box<dyn Error>> {
    let path_obj = Path::new(path);
    let extension = path_obj
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut content = match extension.as_str() {
        "pdf" => extract_pdf_text(path)?,
        _ => {
            // Use memory-mapped files for large files
            if let Ok(metadata) = fs::metadata(path) {
                if metadata.len() > 10_000_000 {
                    // 10MB threshold
                    // For very large files, read in chunks or skip
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
    };

    // Apply character limit early to save memory
    if let Some(max) = max_chars {
        if content.len() > max {
            content.truncate(max);
        }
    }

    Ok(content)
}

/// Synchronous file content reading
pub fn read_files_content(paths: &[String], max_chars: Option<usize>) -> Vec<FileContent> {
    let mut results = Vec::new();
    for path in paths {
        match read_file_content(path, max_chars) {
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
}

/// Checks if file exists in database
fn file_exists(db: &Connection, path: &str) -> Result<bool> {
    let mut stmt = db.prepare("SELECT COUNT(*) FROM files WHERE path = ?1")?;
    let count: i64 = stmt.query_row(params![path], |row| row.get(0))?;
    Ok(count > 0)
}

/// Optimized text chunking with better word boundary handling
fn chunk_text(text: &str, max_words: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut chunks = Vec::new();

    for chunk in words.chunks(max_words) {
        let chunk_text = chunk.join(" ");
        if chunk_text.len() > 50 {
            // Only include meaningful chunks
            chunks.push(chunk_text);
        }
    }

    chunks
}

/// Enhanced scan and store with progress reporting, using database rules.
pub fn scan_and_store_files(
    db: &Connection,
    _dir: &str,
    max_chars: Option<usize>,
    max_file_size: Option<u64>,
    app: tauri::AppHandle,
) -> Result<usize, String> {
    // Stage 1: Scanning for files
    let _ = app.emit(
        "scan_progress",
        crate::commands::ScanProgress {
            current: 0,
            total: 0,
            current_file: "Scanning for files...".to_string(),
            stage: "scanning".to_string(),
        },
    );

    let paths = find_text_files(db, max_file_size, &app).map_err(|e| e.to_string())?;
    println!("Found {} files to process", paths.len());

    if paths.is_empty() {
        return Ok(0);
    }

    // Stage 2: Reading files
    let mut contents = Vec::new();
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

        match read_file_content(path, max_chars) {
            Ok(content) => contents.push(FileContent {
                path: path.clone(),
                content,
                embedding: Vec::new(),
            }),
            Err(e) => {
                eprintln!("Failed to read file {}: {}", path, e);
            }
        }
    }

    println!("Successfully read {} files", contents.len());

    // Filter out files that already exist in the database
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

    // Stage 3: Preparing chunks for embeddings
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

    // Stage 4: Generate embeddings with progress
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

    // Stage 5: Storing in database
    let _ = app.emit(
        "scan_progress",
        crate::commands::ScanProgress {
            current: 0,
            total: new_contents.len() as u64,
            current_file: "Storing in database...".to_string(),
            stage: "storing".to_string(),
        },
    );

    // Begin database transaction for batch inserts
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

        // Insert file record
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

        // Process chunks for this file
        let file_chunks = chunk_text(&file_content.content, 200);
        for chunk in file_chunks {
            if chunk.trim().is_empty() {
                continue;
            }

            // Find corresponding embedding
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

    // Commit transaction
    tx.commit().map_err(|e| e.to_string())?;

    // Final progress update
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
