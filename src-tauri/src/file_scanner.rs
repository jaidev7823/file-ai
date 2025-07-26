// src-tauri/src/file_scanner.rs
use crate::embed_and_store;
use bytemuck::cast_slice;
use chrono::{DateTime, Utc};
// Removed ollama_rs imports as we are using direct reqwest calls for embeddings.
// Removed reqwest::blocking::Client as we're now passing it.
use rusqlite::{params, Connection, Result, Transaction};
use serde::{Deserialize, Serialize};
use std::error::Error;
// Removed tokio::sync::Semaphore;
use std::{fs, path::Path};
use walkdir::WalkDir;
use crate::embed_and_store::normalize;

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

/// List of common human-readable/editable text file extensions
const TEXT_EXTENSIONS: &[&str] = &[
    "txt",
    "md",
    "csv",
    "json",
    "xml",
    "log",
    "cfg",
    "yaml",
    "yml",
    "toml",
    "rs",
    "py",
    "js",
    "ts",
    "tsx",
    "jsx",
    "html",
    "css",
    "scss",
    "less",
    "bat",
    "sh",
    "c",
    "cpp",
    "h",
    "hpp",
    "java",
    "cs",
    "go",
    "php",
    "rb",
    "pl",
    "swift",
    "kt",
    "dart",
    "sql",
    "r",
    "m",
    "vb",
    "ps1",
    "lua",
    "tex",
    "scala",
    "erl",
    "ex",
    "exs",
    "clj",
    "cljs",
    "groovy",
    "asm",
    "s",
    "v",
    "sv",
    "makefile",
    "dockerfile",
    "gitignore",
    "gitattributes",
    "pdf",
];

/// Files to explicitly skip (system files, etc.)
const SKIP_FILES: &[&str] = &[
    "desktop.ini",
    "thumbs.db",
    ".ds_store",
    "autorun.inf",
    "folder.htt",
];

#[derive(Serialize, Deserialize)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    pub embedding: Vec<f32>,
}

/// Enhanced file finder with early filtering and size checking
pub fn find_text_files_optimized<P: AsRef<Path>>(
    dir: P,
    max_file_size: Option<u64>,
) -> Vec<String> {
    let mut results = Vec::new();
    let skip_dirs = [
        "node_modules",
        ".venv",
        "ComfyUI",
        "Adobe",
        ".git",
        "target",
        "build",
        "dist",
    ];

    let walker = WalkDir::new(dir)
        .max_depth(10) // Limit recursion depth
        .into_iter()
        .filter_entry(|entry| {
            if entry.file_type().is_file() {
                // Early size check for files
                if let Some(max_size) = max_file_size {
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.len() > max_size {
                            return false;
                        }
                    }
                }
                return true;
            }

            // For directories, skip if the name matches any in skip_dirs
            if let Some(name) = entry.file_name().to_str() {
                !skip_dirs
                    .iter()
                    .any(|&skip| skip.eq_ignore_ascii_case(name))
            } else {
                true
            }
        });

    for entry in walker.filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let path = entry.path();

            // Skip system files by name
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if SKIP_FILES
                    .iter()
                    .any(|&skip| skip.eq_ignore_ascii_case(file_name))
                {
                    continue;
                }
            }

            // Check extension
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if TEXT_EXTENSIONS
                    .iter()
                    .any(|&allowed| allowed.eq_ignore_ascii_case(ext))
                {
                    results.push(path.to_string_lossy().to_string());
                }
            } else {
                // Handle files without extensions
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    let file_name_lower = file_name.to_lowercase();
                    if TEXT_EXTENSIONS
                        .iter()
                        .any(|&allowed| file_name_lower == allowed)
                    {
                        results.push(path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    results
}

/// Optimized PDF text extraction with better error handling
fn extract_pdf_text_optimized(path: &str) -> Result<String, Box<dyn Error>> {
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
fn read_file_content_optimized(
    path: &str,
    max_chars: Option<usize>,
) -> Result<String, Box<dyn Error>> {
    let path_obj = Path::new(path);
    let extension = path_obj
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut content = match extension.as_str() {
        "pdf" => extract_pdf_text_optimized(path)?,
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
pub fn read_files_content_sync(
    paths: &[String],
    max_chars: Option<usize>,
) -> Vec<FileContent> {
    let mut results = Vec::new();
    // This is now purely synchronous, running within a `spawn_blocking` thread.
    // For parallelism within this blocking context, consider `rayon` or `std::thread::spawn` directly.
    // For now, sequential processing is simplest and avoids nested Tokio runtimes.
    for path in paths {
        match read_file_content_optimized(path, max_chars) {
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

/// Optimized database operations with transactions and prepared statements
/// This function is now fully synchronous.
pub fn scan_and_store_files_optimized(
    db: &Connection,
    dir: &str,
    max_chars: Option<usize>,
    max_file_size: Option<u64>,
) -> Result<usize, String> {
    // Find files with size filtering
    let paths = find_text_files_optimized(dir, max_file_size);
    println!("Found {} files to process", paths.len());

    // Read files using the synchronous version
    let contents = read_files_content_sync(&paths, max_chars);
    println!("Successfully read {} files", contents.len());

    // Filter out files that already exist in the database
    let mut new_contents = Vec::new();
    for file_content in contents {
        if !file_exists(db, &file_content.path).map_err(|e| e.to_string())? {
            new_contents.push(file_content);
        }
    }

    if new_contents.is_empty() {
        println!("No new files to process");
        return Ok(0);
    }

    // Prepare all chunks and texts for batch processing
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

    // Generate embeddings in batches using the synchronous version
    let embeddings = embed_and_store::get_batch_embeddings_sync(&all_chunks)
        .map_err(|e| e.to_string())?;

    // Begin database transaction for batch inserts
    let mut tx = db.unchecked_transaction().map_err(|e| e.to_string())?;

    let mut inserted_count = 0;
    let mut current_chunk_idx = 0;

    for file_content in new_contents.iter() {
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
        ).map_err(|e| e.to_string())?;

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

    println!("Successfully inserted {} files", inserted_count);
    Ok(inserted_count)
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

/// Checks if file exists in database (for backward compatibility)
fn file_exists(db: &Connection, path: &str) -> Result<bool> {
    let mut stmt = db.prepare("SELECT COUNT(*) FROM files WHERE path = ?1")?;
    let count: i64 = stmt.query_row(params![path], |row| row.get(0))?;
    Ok(count > 0)
}

pub fn find_text_files<P: AsRef<Path>>(dir: P) -> Vec<String> {
    find_text_files_optimized(dir, Some(50_000_000)) // 50MB default limit
}