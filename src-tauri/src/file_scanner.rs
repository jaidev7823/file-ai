use crate::embed_and_store;
use crate::embed_and_store::normalize;
use anyhow;
use bytemuck::cast_slice;
use chrono::{DateTime, Utc};
use rusqlite::OptionalExtension;
use rusqlite::{params, Connection, Result, Transaction};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::error::Error;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{fs, path::Path};
use tauri::{AppHandle, Emitter, Manager};
use tokio::runtime::Runtime;
use walkdir::{DirEntry, WalkDir};

// --- GLOBAL STATE ---
static IS_SCANNING: AtomicBool = AtomicBool::new(false);
const BATCH_SIZE: usize = 1000;

// --- PUBLIC API ---

/// Scans files based on Phase 1 rules (included paths, extensions).
pub fn scan_and_store_files(
    db: &Connection,
    _dir: &str,
    max_chars: Option<usize>,
    _max_file_size: Option<u64>,
    app: tauri::AppHandle,
) -> Result<usize, String> {
    scan_and_store_files_with_mode(db, _dir, max_chars, _max_file_size, app, false)
}

/// Scans all drives for metadata (Phase 2).
pub fn scan_drives_metadata_only(db: &Connection, app: &AppHandle) -> Result<usize, String> {
    scan_and_store_files_with_mode(db, "", None, None, app.clone(), true)
}

// --- TYPES ---

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

#[derive(Serialize, Deserialize, Debug)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    pub embedding: Vec<f32>,
    pub category: FileCategory,
    pub content_processed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
            "rs" | "js" | "ts" | "jsx" | "tsx" | "py" | "java" | "c" | "cpp" | "h" | "hpp"
            | "cs" | "php" | "rb" | "go" | "swift" | "kt" | "scala" | "clj" | "hs" | "ml"
            | "fs" | "elm" | "dart" | "r" | "m" | "mm" | "pl" | "sh" | "bash" | "zsh" | "fish" => {
                Self::Code
            }
            "md" | "txt" | "pdf" | "doc" | "docx" | "rtf" | "odt" | "tex" | "rst" | "adoc" => {
                Self::Document
            }
            "csv" | "tsv" | "xls" | "xlsx" | "ods" => Self::Spreadsheet,
            "db" | "sqlite" | "sqlite3" | "sql" | "mdb" | "accdb" => Self::Database,
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" | "ico" | "tiff" | "tif"
            | "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a" | "mp4" | "avi" | "mkv"
            | "mov" | "wmv" | "flv" | "webm" | "m4v" => Self::Media,
            "json" | "yaml" | "yml" | "toml" | "ini" | "cfg" | "conf" | "config" | "xml"
            | "plist" | "properties" | "env" => Self::Config,
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "dmg" | "iso" => Self::Archive,
            "exe" | "dll" | "so" | "dylib" | "bin" | "app" | "deb" | "rpm" | "msi" => Self::Binary,
            _ => Self::Unknown,
        }
    }
}

// --- SCAN ORCHESTRATION ---

/// Main orchestrator for scanning files.
pub fn scan_and_store_files_with_mode(
    db: &Connection,
    _dir: &str,
    max_chars: Option<usize>,
    _max_file_size: Option<u64>,
    app: tauri::AppHandle,
    is_phase2: bool,
) -> Result<usize, String> {
    if IS_SCANNING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        emit_scan_progress(&app, 0, 0, "A scan is already in progress.", "error");
        return Err("A scan is already in progress.".to_string());
    }
    let _guard = ScanGuard;

    let rt = Runtime::new().map_err(|e| e.to_string())?;
    emit_scan_progress(&app, 0, 0, "Scanning for files and folders...", "scanning");
    println!("scanning for files and folders");

    // Step 1: Discover files and folders, and store folders
    let paths: Vec<String> = if is_phase2 {
        // Phase 2 still uses the old method for broad drive scanning
        let scanned_files = find_all_drive_files(db, &app)?;
        scanned_files.iter().map(|sf| sf.path.clone()).collect()
    } else {
        // Phase 1 now scans and stores folders in the same pass
        let base_paths: Vec<String> = crate::database::rules::get_included_paths_sync(db)
            .map_err(|e| e.to_string())?
            .into_iter()
            .collect();
        let exclude_folders: Vec<String> = crate::database::rules::get_excluded_folder_sync(db)
            .map_err(|e| e.to_string())?
            .into_iter()
            .collect();

        let mut found_files = Vec::new();
        let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;

        for base_path in &base_paths {
            let path_buf = PathBuf::from(base_path);
            if !path_buf.exists() {
                continue;
            }

            let mut walker = WalkDir::new(path_buf).into_iter();
            'walker_loop: while let Some(entry_result) = walker.next() {
                let entry = match entry_result {
                    Ok(entry) => entry,
                    Err(_) => continue, // Skip files we can't access
                };

                if entry.file_type().is_dir() {
                    // Always store the folder metadata itself
                    if let Err(e) = insert_folder_metadata(&tx, entry.path()) {
                        eprintln!("Failed to store folder {}: {}", entry.path().display(), e);
                    }

                    // But if it's an excluded folder, skip everything inside it
                    if entry.file_name().to_str().map_or(false, |name| {
                        let lname = name.to_lowercase();
                        if lname.starts_with('.') { return true; } // Ignore hidden folders like .git
                        exclude_folders.iter().any(|ex| ex.eq_ignore_ascii_case(name))
                    }) {
                        walker.skip_current_dir();
                        continue 'walker_loop;
                    }
                } else if entry.file_type().is_file() {
                    found_files.push(entry.path().to_string_lossy().into_owned());
                }
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        found_files
    };

    println!("Found {} files to process", paths.len());
    if paths.is_empty() {
        emit_scan_progress(&app, 0, 0, "No files found to process", "complete");
        return Ok(0);
    }

    // Step 2: Filter for new files and read their content based on scan rules.
    let new_files = prepare_files_for_processing(db, &paths, is_phase2, max_chars, &rt, &app)?;
    println!("Identified {} new files for processing", new_files.len());
    if new_files.is_empty() {
        emit_scan_progress(
            &app,
            paths.len() as u64,
            paths.len() as u64,
            "No new files to process",
            "complete",
        );
        return Ok(0);
    }

    // Step 3: Create text chunks for embedding
    let (chunks, file_chunk_map) = build_embedding_chunks(&new_files);
    println!("Total {} text units for embedding", chunks.len());

    // Step 4: Generate embeddings
    let embeddings = if chunks.is_empty() {
        Vec::new()
    } else {
        let app_clone = app.clone();
        emit_scan_progress(
            &app,
            0,
            chunks.len() as u64,
            "Generating embeddings...",
            "embedding",
        );
        println!("Generating embeddings...");
        embed_and_store::get_batch_embeddings_with_progress(&chunks, move |current, total| {
            app_clone
                .emit(
                    "scan_progress",
                    serde_json::json!({
                        "current": current, "total": total, "stage": "embedding",
                        "current_file": format!("Processing embedding {} of {}", current, total)
                    }),
                )
                .ok();
        })
        .map_err(|e| e.to_string())?
    };

    // Step 5: Store results in the database
    let inserted_count = store_results(db, &new_files, &embeddings, &file_chunk_map, &app)?;

    emit_scan_progress(
        &app,
        inserted_count as u64,
        inserted_count as u64,
        format!("Completed! Processed {} new files", inserted_count),
        "complete",
    );

    Ok(inserted_count)
}

/// Handles the atomic state of the scan.
struct ScanGuard;
impl Drop for ScanGuard {
    fn drop(&mut self) {
        IS_SCANNING.store(false, Ordering::SeqCst);
    }
}

// --- SCAN PIPELINE STAGES ---

/// Stage 1: Filters a list of paths for new files and reads their content based on scan rules.
fn prepare_files_for_processing(
    db: &Connection,
    paths: &[String],
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
            embedding: Vec::new(), // Will be populated later
            category,
            content_processed: should_crawl_content,
        });
    }
    Ok(new_files_to_process)
}

/// Stage 2: Generates metadata and content chunks for a list of files.
fn build_embedding_chunks(files: &[FileContent]) -> (Vec<String>, Vec<(String, Vec<usize>)>) {
    let mut all_chunks = Vec::new();
    let mut file_chunk_map = Vec::new();

    for file in files {
        let mut current_file_chunk_indices = Vec::new();
        let path_obj = Path::new(&file.path);

        // Create and add metadata chunk
        let metadata_string = create_metadata_string(path_obj);
        if !metadata_string.trim().is_empty() {
            current_file_chunk_indices.push(all_chunks.len());
            all_chunks.push(metadata_string);
        }

        // Create and add content chunks
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
fn store_results(
    db: &Connection,
    files: &[FileContent],
    embeddings: &[Vec<f32>],
    file_chunk_map: &[(String, Vec<usize>)],
    app: &AppHandle,
) -> Result<usize, String> {
    let mut total_inserted_count = 0;

    // Process in batches
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

            let file_id = insert_file_metadata(&tx, file).map_err(|e| e.to_string())?;

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

        // Commit each batch
        tx.commit().map_err(|e| e.to_string())?;
        total_inserted_count += chunk.len();
    }

    Ok(total_inserted_count)
}

// --- FILE DISCOVERY ---

#[derive(serde::Serialize, Clone)]
pub struct ScannedFile {
    path: String,
    content_processed: bool,
}

struct ScanConfig<'a> {
    base_paths: Vec<String>,
    include_exts: Vec<String>,
    exclude_folders: &'a [String],
    exclude_paths: &'a [String],
    system_folder_names: &'a [&'a str],
}

/// Generic file discovery function.
fn find_files(config: ScanConfig, app: &AppHandle, progress_stage: &str) -> Vec<String> {
    let mut found_files = Vec::new();
    let mut scanned_count = 0;

    for base_path in &config.base_paths {
        let path_buf = PathBuf::from(base_path);
        if !path_buf.exists() {
            continue;
        }

        let walker = WalkDir::new(path_buf).into_iter();
        let filtered_walker = walker.filter_entry(|e| !is_excluded_dir(e, &config));

        for entry in filtered_walker.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let path_str = path.to_string_lossy().to_string();
            if config.exclude_paths.iter().any(|p| path_str.starts_with(p)) {
                continue;
            }

            found_files.push(path_str.clone());
            scanned_count += 1;
            if scanned_count % 1000 == 0 {
                emit_scan_progress(app, scanned_count, 0, &path_str, progress_stage);
            }
        }
    }
    found_files
}

fn is_excluded_dir(entry: &DirEntry, config: &ScanConfig) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }

    if let Some(name) = entry.file_name().to_str() {
        let lname = name.to_lowercase();
        if lname.starts_with('.') {
            return true;
        }
        if config.system_folder_names.iter().any(|&sys| sys == lname) {
            return true;
        }
    }
    false
}

/// Phase 1: Find text files based on user-defined rules.
pub fn find_text_files(conn: &Connection, app: &AppHandle) -> Result<Vec<ScannedFile>, String> {
    println!("--- RUNNING find_text_files ---"); // Temporary debug line
    emit_scan_progress(app, 0, 0, "", "scanning");
    let include_exts: Vec<String> = crate::database::rules::get_included_extensions_sync(conn)
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();
    let exclude_folders: Vec<String> = crate::database::rules::get_excluded_folder_sync(conn)
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();
    let base_paths: Vec<String> = crate::database::rules::get_included_paths_sync(conn)
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();
    let config = ScanConfig {
        base_paths,
        include_exts,
        exclude_folders: &exclude_folders,
        exclude_paths: &[],
        system_folder_names: &[],
    };
    let file_paths = find_files(config, app, "scanning");

    let mut scanned_files = Vec::new();
    for path in file_paths {
        let (should_crawl, _) = check_phase1_rules(conn, &path)?;
        scanned_files.push(ScannedFile {
            path,
            content_processed: should_crawl,
        });
    }

    emit_scan_progress(
        app,
        scanned_files.len() as u64,
        scanned_files.len() as u64,
        "",
        "complete",
    );
    Ok(scanned_files)
}

/// Phase 2: Find all files across all drives (metadata only).
pub fn find_all_drive_files(
    conn: &Connection,
    app: &AppHandle,
) -> Result<Vec<ScannedFile>, String> {
    let files = find_all_drive_files_internal(conn, app)?;
    // Convert Vec<String> to Vec<ScannedFile>
    Ok(files
        .into_iter()
        .map(|path| ScannedFile {
            path,
            content_processed: false, // Phase 2 doesn't process content
        })
        .collect())
}

fn find_all_drive_files_internal(
    conn: &Connection,
    app: &AppHandle,
) -> Result<Vec<String>, String> {
    emit_scan_progress(app, 0, 0, "", "phase2_discovery");
    let exclude_folders: Vec<String> = crate::database::rules::get_excluded_folder_sync(conn)
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();
    let exclude_paths: Vec<String> = crate::database::rules::get_excluded_paths_sync(conn)
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();
    let config = ScanConfig {
        base_paths: discover_drives(),
        include_exts: Vec::new(), // All extensions
        exclude_folders: &exclude_folders,
        exclude_paths: &exclude_paths,
        system_folder_names: &[
            "system volume information",
            "$recycle.bin",
            "windows",
            "program files",
            "program files (x86)",
        ],
    };
    let files = find_files(config, app, "phase2_scanning");
    emit_scan_progress(
        app,
        files.len() as u64,
        files.len() as u64,
        "",
        "phase2_scan_complete",
    );
    Ok(files)
}

// --- CONTENT EXTRACTION & PREPARATION ---

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
        .to_string();

    let category = FileCategory::from_extension(&extension);

    if !process_content {
        return Ok((String::new(), category));
    }

    let mut content = match extension.as_str() {
        "pdf" => extract_pdf_text(path).await?,
        "csv" | "tsv" => {
            let path_str = path.to_string();
            let ext = extension.clone();
            tokio::task::spawn_blocking(move || read_csv_to_string(&path_str, &ext)).await??
        }
        _ => {
            if let Ok(metadata) = fs::metadata(path) {
                if metadata.len() > 10_000_000 {
                    // 10MB limit
                    return Err("File too large for processing".into());
                }
            }
            fs::read_to_string(path).or_else(|_| {
                fs::read(path).map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            })?
        }
    };

    content = extract_category_content(&content, &category, &extension, path);

    if let Some(max) = max_chars {
        if content.len() > max {
            content.truncate(max);
        }
    }

    Ok((content, category))
}

fn read_csv_to_string(path: &str, ext: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let delimiter = if ext == "tsv" { b'\t' } else { b',' };
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true)
        .from_path(path)?;
    let headers = rdr.headers().map(|h| h.clone()).unwrap_or_default();
    let max_rows = 1000;

    let mut rows: Vec<String> = rdr
        .records()
        .enumerate()
        .map_while(|(i, result)| {
            if i >= max_rows {
                return None;
            }
            result.ok().map(|record| {
                if !headers.is_empty() && headers.len() == record.len() {
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
                }
            })
        })
        .collect();

    if rows.len() >= max_rows {
        rows.push(format!("[truncated after {} rows]", max_rows));
    }

    Ok(rows.join("\n"))
}

pub async fn extract_pdf_text(path: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let path_str = path.to_string();
    tokio::task::spawn_blocking(move || {
        use lopdf::Document;
        use rayon::prelude::*;
        const MAX_PAGES: usize = 25;

        let doc = Document::load(&path_str).map_err(|e| {
            Box::<dyn Error + Send + Sync>::from(format!(
                "Failed to load PDF '{}': {}",
                path_str, e
            ))
        })?;
        let page_ids: Vec<u32> = doc.get_pages().keys().copied().take(MAX_PAGES).collect();
        if page_ids.is_empty() {
            return Err(Box::<dyn Error + Send + Sync>::from(
                "PDF appears to be empty",
            ));
        }

        let texts: Vec<String> = page_ids
            .par_iter()
            .map(|&id| doc.extract_text(&[id]).unwrap_or_default())
            .collect();

        let combined = texts.join("\n");
        if combined.trim().is_empty() {
            Err(Box::<dyn Error + Send + Sync>::from(
                "No text content found in PDF",
            ))
        } else {
            Ok(combined)
        }
    })
    .await?
}

pub fn extract_category_content(
    content: &str,
    category: &FileCategory,
    extension: &str,
    file_path: &str,
) -> String {
    match category {
        FileCategory::Code => extract_code_metadata(file_path, extension),
        FileCategory::Document | FileCategory::Spreadsheet | FileCategory::Config => {
            content.to_string()
        }
        FileCategory::Media => String::new(), // No content for media
        _ => content.to_string(),
    }
}

pub fn extract_code_metadata(file_path: &str, extension: &str) -> String {
    let path_obj = Path::new(file_path);
    let file_name = path_obj.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let file_stem = path_obj.file_stem().and_then(|n| n.to_str()).unwrap_or("");
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

fn chunk_text(text: &str, max_words: usize) -> Vec<String> {
    text.split_whitespace()
        .collect::<Vec<&str>>()
        .chunks(max_words)
        .map(|chunk| chunk.join(" "))
        .filter(|s| s.len() > 50)
        .collect()
}

fn create_metadata_string(path_obj: &Path) -> String {
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

    let parent_folders: Vec<String> = path_obj
        .parent()
        .map(|p| {
            p.components()
                .filter_map(|comp| match comp {
                    std::path::Component::Normal(os_str) => os_str.to_str().map(String::from),
                    std::path::Component::RootDir => Some("root".to_string()),
                    std::path::Component::Prefix(prefix) => {
                        prefix.as_os_str().to_str().map(|s| format!("drive_{}", s))
                    }
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();

    let immediate_parent = parent_folders.last().cloned().unwrap_or_default();
    let folder_hierarchy = parent_folders.join(" > ");

    format!(
        "filename: {} stem: {} extension: {} path: {} parent_folder: {} folder_hierarchy: {} drive: {}",
        file_name, file_stem, extension, path_obj.to_string_lossy(),
        immediate_parent, folder_hierarchy, extract_drive(&path_obj.to_string_lossy())
    )
}

// --- DATABASE HELPERS ---
fn insert_folder_metadata(tx: &Transaction, path: &Path) -> anyhow::Result<i64> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let path_str = path.to_string_lossy().to_string();

    // Return early if the folder already exists to avoid metadata lookup
    let mut stmt_exists = tx.prepare("SELECT id FROM folders WHERE path = ?1")?;
    if let Ok(id) = stmt_exists.query_row(params![&path_str], |row| row.get::<_, i64>(0)) {
        return Ok(id);
    }

    // Fix the error conversion here
    let metadata = fs::metadata(path).map_err(|e| anyhow::anyhow!("IO Error: {}", e))?;
    let created = Into::<DateTime<Utc>>::into(metadata.created().map_err(|e| anyhow::anyhow!("IO Error: {}", e))?,).to_rfc3339();
    let updated = Into::<DateTime<Utc>>::into(metadata.modified().map_err(|e| anyhow::anyhow!("IO Error: {}", e))?,).to_rfc3339();
    let accessed = Into::<DateTime<Utc>>::into(metadata.accessed().map_err(|e| anyhow::anyhow!("IO Error: {}", e))?,).to_rfc3339();

    let parent_id: Option<i64> = if let Some(parent_path) = path.parent() {
        tx.query_row(
            "SELECT id FROM folders WHERE path = ?1",
            params![parent_path.to_string_lossy().to_string()],
            |row| row.get(0),
        )
        .optional()?
    } else {
        None
    };

    let mut stmt = tx.prepare(
        "INSERT INTO folders (name, path, parent_folder_id, created_at, updated_at, last_accessed)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    stmt.execute(params![
        name, path_str, parent_id, created, updated, accessed,
    ])?;
    Ok(tx.last_insert_rowid())
}

fn file_exists(db: &Connection, path: &str) -> Result<bool> {
    let mut stmt = db.prepare("SELECT 1 FROM files WHERE path = ?1 COLLATE NOCASE")?;
    stmt.exists(params![path])
}

fn insert_file_metadata(tx: &Transaction, file: &FileContent) -> anyhow::Result<i64> {
    let path_obj = Path::new(&file.path);
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
    let metadata = fs::metadata(&file.path).map_err(|e| rusqlite::Error::InvalidQuery)?;

    let created = Utc::now().to_rfc3339();
    let updated = Utc::now().to_rfc3339();
    let accessed = Into::<DateTime<Utc>>::into(metadata.accessed()?).to_rfc3339();

    let mut stmt = tx.prepare(
        "INSERT INTO files (name, extension, path, content, author, file_size, category, content_processed, created_at, updated_at, last_accessed)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
    )?;
    stmt.execute(params![
        file_name,
        extension,
        file.path,
        file.content,
        None::<String>,
        metadata.len(),
        format!("{:?}", file.category),
        file.content_processed,
        created,
        updated,
        accessed,
    ])?;
    Ok(tx.last_insert_rowid())
}

fn insert_file_embedding(tx: &Transaction, file_id: i64, vector: Vec<f32>) -> Result<()> {
    if vector.is_empty() {
        return Ok(());
    }
    let normalized_vec = normalize(vector);
    let vector_bytes: &[u8] = cast_slice(&normalized_vec);

    let mut stmt_vec = tx.prepare("INSERT INTO file_vec(content_vec) VALUES(?1)")?;
    stmt_vec.execute(params![vector_bytes])?;
    let vec_rowid = tx.last_insert_rowid();

    let mut stmt_map = tx.prepare("INSERT INTO file_vec_map(vec_rowid, file_id) VALUES(?1, ?2)")?;
    stmt_map.execute(params![vec_rowid, file_id])?;
    Ok(())
}

/// Phase 1: Check if file should have content processed based on included/excluded paths.
fn check_phase1_rules(db: &Connection, file_path: &str) -> Result<(bool, bool), String> {
    let include_paths =
        crate::database::rules::get_included_paths_sync(db).map_err(|e| e.to_string())?;
    let exclude_folders =
        crate::database::rules::get_excluded_folder_sync(db).map_err(|e| e.to_string())?;
    let include_exts: HashSet<String> = crate::database::rules::get_included_extensions_sync(db)
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();
    let path_obj = Path::new(file_path);

    // Rule 1: Must be in an included path
    if !include_paths.iter().any(|p| file_path.starts_with(p)) {
        return Ok((false, false)); // Not in an included path, metadata only
    }

    // Rule 2: Check if in an excluded folder
    let is_in_excluded_folder = path_obj.ancestors().any(|ancestor| {
        ancestor
            .file_name()
            .and_then(|n| n.to_str())
            .map_or(false, |name| {
                exclude_folders
                    .iter()
                    .any(|ex| ex.eq_ignore_ascii_case(name))
            })
    });

    if is_in_excluded_folder {
        return Ok((false, true)); // In excluded folder, metadata only
    }

    // Rule 3: Check if extension is on the include list
    let extension = path_obj.extension().and_then(|s| s.to_str()).unwrap_or("");

    if !include_exts.is_empty()
        && !include_exts
            .iter()
            .any(|inc| inc.eq_ignore_ascii_case(extension))
    {
        return Ok((false, false)); // Extension not included, metadata only
    }

    // If all checks pass, process content
    Ok((true, false))
}

// --- UTILITIES ---

fn emit_scan_progress(
    app: &AppHandle,
    current: u64,
    total: u64,
    current_file: impl Into<String>,
    stage: &str,
) {
    let payload = serde_json::json!({
        "current": current, "total": total, "stage": stage, "current_file": current_file.into()
    });
    app.emit("scan_progress", &payload).ok();
    if let Some(win) = app.get_webview_window("main") {
        win.emit("scan_progress", &payload).ok();
    }
}

fn extract_drive(file_path: &str) -> String {
    Path::new(file_path)
        .components()
        .next()
        .and_then(|comp| match comp {
            std::path::Component::Prefix(prefix) => prefix.as_os_str().to_str().map(String::from),
            _ => None,
        })
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(target_os = "windows")]
pub fn discover_drives() -> Vec<String> {
    // Original implementation (commented for reference):
    //
    // (b'A'..=b'Z')
    //     .filter_map(|drive_letter| {
    //         let path_str = format!("{}:\\", drive_letter as char);
    //         Path::new(&path_str).exists().then_some(path_str)
    //     })
    //     .collect()

    // Test path instead of discovering drives
    vec![r"C:\Users\Jai Mishra\Downloads\drive-test".to_string()]
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
                Err(e) => eprintln!("Failed to read file {}: {}", path, e),
            }
        }
        results
    })
}

pub fn read_files_content(paths: &[String], max_chars: Option<usize>) -> Vec<FileContent> {
    read_files_content_with_processing(paths, max_chars, true)
}

pub fn verify_embeddings(db: &Connection, file_id: i64) -> Result<bool, rusqlite::Error> {
    let count: i64 = db.query_row(
        "SELECT COUNT(*) FROM file_vec_map WHERE file_id = ?1",
        params![file_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}
