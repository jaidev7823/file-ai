use crate::embed_and_store;
use crate::embed_and_store::normalize;
use bytemuck::cast_slice;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{fs, path::Path};
use tauri::{AppHandle, Emitter, Manager};

use tokio::runtime::Runtime;
use walkdir::WalkDir;

static IS_SCANNING: AtomicBool = AtomicBool::new(false);

struct ScanGuard;

impl Drop for ScanGuard {
    fn drop(&mut self) {
        IS_SCANNING.store(false, Ordering::SeqCst);
    }
}

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
    pub category: FileCategory,
    pub content_processed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileCategory {
    Code,
    Document,
    Spreadsheet,
    Database,
    Media,
    Config,
    Binary,
    Archive,
    Unknown,
}

impl FileCategory {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            // Code files
            "rs" | "js" | "ts" | "jsx" | "tsx" | "py" | "java" | "c" | "cpp" | "h" | "hpp"
            | "cs" | "php" | "rb" | "go" | "swift" | "kt" | "scala" | "clj" | "hs" | "ml"
            | "fs" | "elm" | "dart" | "r" | "m" | "mm" | "pl" | "sh" | "bash" | "zsh" | "fish" => {
                Self::Code
            }

            // Documents
            "md" | "txt" | "pdf" | "doc" | "docx" | "rtf" | "odt" | "tex" | "rst" | "adoc" => {
                Self::Document
            }

            // Spreadsheets
            "csv" | "tsv" | "xls" | "xlsx" | "ods" => Self::Spreadsheet,

            // Database
            "db" | "sqlite" | "sqlite3" | "sql" | "mdb" | "accdb" => Self::Database,

            // Media
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" | "ico" | "tiff" | "tif"
            | "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a" | "mp4" | "avi" | "mkv"
            | "mov" | "wmv" | "flv" | "webm" | "m4v" => Self::Media,

            // Config
            "json" | "yaml" | "yml" | "toml" | "ini" | "cfg" | "conf" | "config" | "xml"
            | "plist" | "properties" | "env" => Self::Config,

            // Archives
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "dmg" | "iso" => Self::Archive,

            // Binary
            "exe" | "dll" | "so" | "dylib" | "bin" | "app" | "deb" | "rpm" | "msi" => Self::Binary,

            _ => Self::Unknown,
        }
    }
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

            if scanned_count % 500 == 0 {
                emit_scan_progress(app, scanned_count, 0, &path_str, "scanning");
            }
        }
    }

    emit_scan_progress(app, scanned_count, scanned_count, "", "complete");
    Ok(found_files)
}

/// Extract metadata-only content for code files (no actual code content)
pub fn extract_code_metadata(file_path: &str, extension: &str) -> String {
    let path_obj = Path::new(file_path);
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

    // Create metadata string for code files
    format!(
        "code_file: {} language: {} filename: {} stem: {}",
        file_name,
        get_language_from_extension(extension),
        file_name,
        file_stem
    )
}

fn get_language_from_extension(ext: &str) -> &str {
    match ext.to_lowercase().as_str() {
        "rs" => "rust",
        "js" => "javascript",
        "ts" => "typescript",
        "jsx" => "react_javascript",
        "tsx" => "react_typescript",
        "py" => "python",
        "java" => "java",
        "c" => "c",
        "cpp" | "cc" | "cxx" => "cpp",
        "h" | "hpp" => "c_header",
        "cs" => "csharp",
        "php" => "php",
        "rb" => "ruby",
        "go" => "go",
        "swift" => "swift",
        "kt" => "kotlin",
        "scala" => "scala",
        "clj" => "clojure",
        "hs" => "haskell",
        "ml" => "ocaml",
        "fs" => "fsharp",
        "elm" => "elm",
        "dart" => "dart",
        "r" => "r",
        "m" | "mm" => "objective_c",
        "pl" => "perl",
        "sh" | "bash" | "zsh" | "fish" => "shell",
        _ => "unknown",
    }
}

/// Extract content based on file category
pub fn extract_category_content(
    content: &str,
    category: &FileCategory,
    extension: &str,
    file_path: &str,
) -> String {
    match category {
        FileCategory::Code => {
            // For code files, return only metadata, not actual content
            extract_code_metadata(file_path, extension)
        }
        FileCategory::Document => content.to_string(), // Full content for documents
        FileCategory::Spreadsheet => {
            // For CSV/TSV, this will be handled in read_file_content
            content.to_string()
        }
        FileCategory::Media => {
            // For media files, we only want metadata, not content
            String::new()
        }
        FileCategory::Config => {
            // For config files, extract key sections
            if extension == "json" || extension == "yaml" || extension == "yml" {
                // Extract top-level keys and structure
                extract_config_structure(content)
            } else {
                content.to_string()
            }
        }
        _ => content.to_string(),
    }
}

fn extract_config_structure(content: &str) -> String {
    // For config files, just return the full content for now
    // We can enhance this later if needed without regex complexity
    content.to_string()
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
                    let combined = texts.join(
                        "
",
                    );
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

pub async fn read_file_content_with_category(
    path: &str,
    max_chars: Option<usize>,
    process_content: bool,
) -> Result<(String, FileCategory), Box<dyn Error + Send + Sync>> {
    let path_obj = Path::new(path);
    let extension = path_obj
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let category = FileCategory::from_extension(&extension);

    // If content processing is disabled, return empty content but keep category
    if !process_content {
        return Ok((String::new(), category));
    }

    let mut content = match extension.as_str() {
        "pdf" => extract_pdf_text(path).await?,
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

                    Ok(rows.join(
                        "
",
                    ))
                },
            )
            .await??
        }
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
    };

    // Apply category-specific content extraction
    content = extract_category_content(&content, &category, &extension, path);

    if let Some(max) = max_chars {
        if content.len() > max {
            content.truncate(max);
        }
    }

    Ok((content, category))
}

// Backward compatibility wrapper
pub async fn read_file_content(
    path: &str,
    max_chars: Option<usize>,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let (content, _) = read_file_content_with_category(path, max_chars, true).await?;
    Ok(content)
}

pub fn read_files_content_with_processing(
    paths: &[String],
    max_chars: Option<usize>,
    process_content: bool,
) -> Vec<FileContent> {
    let rt = Runtime::new().expect("Failed to create runtime");
    rt.block_on(async {
        let mut results = Vec::new();
        for path in paths {
            match read_file_content_with_category(path, max_chars, process_content).await {
                Ok((content, category)) => results.push(FileContent {
                    path: path.clone(),
                    content,
                    embedding: Vec::new(),
                    category,
                    content_processed: process_content,
                }),
                Err(e) => {
                    eprintln!("Failed to read file {}: {}", path, e);
                }
            }
        }
        results
    })
}

// Backward compatibility wrapper
pub fn read_files_content(paths: &[String], max_chars: Option<usize>) -> Vec<FileContent> {
    read_files_content_with_processing(paths, max_chars, true)
}

fn file_exists(db: &Connection, path: &str) -> Result<bool> {
    let mut stmt = db.prepare("SELECT COUNT(*) FROM files WHERE path = ?1 COLLATE NOCASE")?;
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
    _dir: &str,
    max_chars: Option<usize>,
    _max_file_size: Option<u64>,
    app: tauri::AppHandle,
) -> Result<usize, String> {
    if IS_SCANNING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        let _ = app.emit(
            "scan_progress",
            crate::commands::ScanProgress {
                current: 0,
                total: 0,
                current_file: "A scan is already in progress.".to_string(),
                stage: "error".to_string(),
            },
        );
        return Err("A scan is already in progress.".to_string());
    }
    let _guard = ScanGuard;

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

        // Phase 1: Check if this file is in included paths and should have content processed
        let (should_crawl_content, _is_in_excluded_folder) =
            check_phase1_rules(db, path).map_err(|e| e.to_string())?;

        let (content, category) = match rt.block_on(read_file_content_with_category(
            path,
            max_chars,
            should_crawl_content,
        )) {
            Ok((content, category)) => (content, category),
            Err(e) => {
                eprintln!("Failed to read file content {}: {}", path, e);
                (String::new(), FileCategory::Unknown)
            }
        };

        new_files_to_process.push(FileContent {
            path: path.clone(),
            content,
            embedding: Vec::new(),
            category,
            content_processed: should_crawl_content,
        });
    }

    println!(
        "Identified {} new files for processing",
        new_files_to_process.len()
    );

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
                            std::path::Component::Normal(os_str) => {
                                os_str.to_str().map(|s| s.to_string())
                            }
                            std::path::Component::RootDir => Some("root".to_string()),
                            std::path::Component::Prefix(prefix) => {
                                // Handle drive letters like C:, D:, etc.
                                Some(format!(
                                    "drive_{}",
                                    prefix.as_os_str().to_str().unwrap_or("unknown")
                                ))
                            }
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

    println!(
        "Total {} text units (enhanced metadata + content chunks) for embedding",
        all_chunks_to_embed.len()
    );

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
        embed_and_store::get_batch_embeddings_with_progress(
            &all_chunks_to_embed,
            move |current, total| {
                let _ = app_clone.emit(
                    "scan_progress",
                    crate::commands::ScanProgress {
                        current: current as u64,
                        total: total as u64,
                        current_file: format!("Processing embedding {} of {}", current, total),
                        stage: "embedding".to_string(),
                    },
                );
            },
        )
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

        let metadata = fs::metadata(&file_content.path).map_err(|e| e.to_string())?;
        let file_size = metadata.len();
        let last_accessed: DateTime<Utc> = metadata.accessed().map_err(|e| e.to_string())?.into();

        // Insert with Phase 1 enhancements: category and content processing status
        tx.execute(
            "INSERT INTO files (name, extension, path, content, author, file_size, category, content_processed, created_at, updated_at, last_accessed) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                file_name,
                extension,
                file_content.path,
                file_content.content,
                None::<String>, // Author - Not implemented yet
                file_size,      // File Size in bytes
                format!("{:?}", file_content.category), // Category as string
                file_content.content_processed, // Whether content was processed
                now.to_rfc3339(),
                now.to_rfc3339(),
                last_accessed.to_rfc3339(),
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

/// Phase 1: Check if file should have content processed based on included/excluded paths
fn check_phase1_rules(db: &Connection, file_path: &str) -> Result<(bool, bool), String> {
    // Get included paths (VIP zone)
    let include_paths =
        crate::database::rules::get_included_paths_sync(db).map_err(|e| e.to_string())?;
    let exclude_folders =
        crate::database::rules::get_excluded_folder_sync(db).map_err(|e| e.to_string())?;

    let path_obj = Path::new(file_path);

    // Check if file is within any included path
    let is_in_included_path = include_paths
        .iter()
        .any(|include_path| file_path.starts_with(include_path));

    if !is_in_included_path {
        // File is not in any included path, so no content processing for Phase 1
        return Ok((false, false));
    }

    // File is in included path, now check if it's in an excluded folder within that path
    let is_in_excluded_folder = path_obj.ancestors().any(|ancestor| {
        if let Some(folder_name) = ancestor.file_name().and_then(|n| n.to_str()) {
            exclude_folders
                .iter()
                .any(|excluded| excluded.eq_ignore_ascii_case(folder_name))
        } else {
            false
        }
    });

    if is_in_excluded_folder {
        // File is in excluded folder within included path - save metadata only
        return Ok((false, true));
    }

    // File is in included path and not in excluded folder - process content
    Ok((true, false))
}

fn extract_drive(file_path: &str) -> String {
    if let Some(first_component) = Path::new(file_path).components().next() {
        match first_component {
            std::path::Component::Prefix(prefix) => {
                prefix.as_os_str().to_str().unwrap_or("unknown").to_string()
            }
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
        parent
            .components()
            .filter_map(|comp| match comp {
                std::path::Component::Normal(os_str) => os_str.to_str(),
                std::path::Component::Prefix(prefix) => prefix.as_os_str().to_str(),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" > ")
    } else {
        String::new()
    }
}
