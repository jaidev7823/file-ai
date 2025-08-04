// src-tauri/src/file_scanner.rs
// Crate is compilation unit each crate can have diffrent part of code here we are importing embed_and_store file with crate 
use crate::embed_and_store;
// bytemuck can convert datatype like parseint and cast_slice can change data type of subbarray
use bytemuck::cast_slice;
// chrono is library for handling datetime and coordinate of universal time (utc)
use chrono::{DateTime, Utc};
// russqlite is lib we use to talk to our sqlite database connection for connecting to sqlite database result for chaking success failure params for safety param
use rusqlite::{params, Connection, Result};
// serde is library for deserialize unpacking the data and serialize for packing the data
use serde::{Deserialize, Serialize};
// is one of the imp libray from rust contain many imp things like error management
use std::error::Error;
// hashset is where we can store only unique type of things, fs file system helps us to work with our files, path to represetn path
use std::{collections::HashSet, fs, path::Path};
// as in our progress from frontend we were using listen and in backend we use emitter to constatnly send data to frontend
use tauri::Emitter;
// this is the librabry we use to go through file and directories
use walkdir::WalkDir;

// importing function from embed_andStore name normalize 
use crate::embed_and_store::normalize;

// here we are declaring struct is kind of like ts but it can encapsulate data and can also give data a methods 
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]

// struct is custom data type we create who hold multiple values of diffrent data type
pub struct File {
    pub id: i32, // i32 mean this is signed integer who can have negative to postive value from range to -2,147,483,648 to 2,147,483,647
    pub name: String,
    pub extension: String,
    pub path: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Files to explicitly skip (system files, etc.)
const SKIP_FILES: &[&str] = &[
    "desktop.ini",
    "thumbs.db",
    ".ds_store",
    "autorun.inf",
    "folder.htt",
];

// serialize mean collecting data and desseralize opening the data 
#[derive(Serialize, Deserialize)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    pub embedding: Vec<f32>, //vec 32 this is from sqlite vec and f mean float 0.25 and other you know
}

// Helper to fetch excluded paths from the database HashSet is where we only have unique kind of values adn we using this to say this weill return hasset of strings 
fn get_excluded_paths(db: &Connection) -> Result<HashSet<String>> {
    // this mut mean stmt value can change 
    let mut stmt = db.prepare("SELECT path FROM path_rules WHERE rule_type = 'exclude'")?;
    // this part of code is used for query execution who use rusqlite query_map function who ask for two argument first para second closure (annonymous function) we have closure who is a closure that extracts the first column (index 0) from each row
    let paths = stmt
        .query_map([], |row| row.get(0))?
        // collect job is to transform iterator into a collection 
        .collect::<Result<Vec<String>>>()?;
    // saying succesfull 
    Ok(paths.into_iter().collect())
}

// Helper to fetch included extensions from the database
fn get_included_extensions(db: &Connection) -> Result<HashSet<String>> {
    // mut 
    let mut stmt =
        db.prepare("SELECT extension FROM extension_rules WHERE rule_type = 'include'")?;
    let extensions = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<String>>>()?;
    Ok(extensions.into_iter().collect())
}

/// Legacy function for backward compatibility - uses hardcoded ignored folders
pub fn find_text_files_with_ignored<P: AsRef<Path>>(
    dir: P,
    ignored_folders: Vec<String>,
) -> Vec<String> {
    let mut results = Vec::new();
    let ignored_set: HashSet<String> = ignored_folders.into_iter().collect();
    
    // Default text extensions for legacy function
    let text_extensions = [
        "txt", "md", "csv", "json", "xml", "log", "cfg", "yaml", "yml", "toml", "rs", "py",
        "js", "ts", "tsx", "jsx", "html", "css", "scss", "less", "bat", "sh", "c", "cpp",
        "h", "hpp", "java", "cs", "go", "php", "rb", "pl", "swift", "kt", "dart", "sql",
        "r", "m", "vb", "ps1", "lua", "tex", "scala", "erl", "ex", "exs", "clj", "cljs",
        "groovy", "asm", "s", "v", "sv", "makefile", "dockerfile", "gitignore",
        "gitattributes", "pdf",
    ];
    let ext_set: HashSet<&str> = text_extensions.iter().cloned().collect();

    let walker = WalkDir::new(dir)
        .max_depth(10)
        .into_iter()
        .filter_entry(|entry| {
            if entry.file_type().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    !ignored_set.contains(name)
                } else {
                    true
                }
            } else {
                true
            }
        });

    for entry in walker.filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let path = entry.path();

            // Skip system files
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
                if ext_set.contains(&ext.to_lowercase().as_str()) {
                    results.push(path.to_string_lossy().to_string());
                }
            } else {
                // Handle files without extensions
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if ext_set.contains(&file_name.to_lowercase().as_str()) {
                        results.push(path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    
    results
}

/// Enhanced file finder that uses database rules for filtering.
pub fn find_text_files_optimized<P: AsRef<Path>>(
    db: &Connection,
    dir: P,
    max_file_size: Option<u64>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut results = Vec::new();
    let excluded_paths = get_excluded_paths(db)?;
    let included_extensions = get_included_extensions(db)?;

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

            // For directories, skip if the name matches any in the excluded_paths set
            if let Some(name) = entry.file_name().to_str() {
                !excluded_paths.contains(name)
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

            // Check extension against the included_extensions set
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if included_extensions.contains(&ext.to_lowercase()) {
                    results.push(path.to_string_lossy().to_string());
                }
            } else {
                // Handle files without extensions (e.g., "Dockerfile")
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if included_extensions.contains(&file_name.to_lowercase()) {
                        results.push(path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    Ok(results)
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
pub fn read_files_content_sync(paths: &[String], max_chars: Option<usize>) -> Vec<FileContent> {
    let mut results = Vec::new();
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

/// Optimized scan and store without progress reporting
pub fn scan_and_store_files_optimized(
    db: &Connection,
    dir: &str,
    max_chars: Option<usize>,
    max_file_size: Option<u64>,
) -> Result<usize, String> {
    println!("Starting optimized file scan and store for: {}", dir);

    // Stage 1: Find files using database rules
    let paths = find_text_files_optimized(db, dir, max_file_size).map_err(|e| e.to_string())?;
    println!("Found {} files to process", paths.len());

    if paths.is_empty() {
        return Ok(0);
    }

    // Stage 2: Read file contents
    let mut contents = Vec::new();
    for path in paths.iter() {
        match read_file_content_optimized(path, max_chars) {
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
        println!("No new files to process");
        return Ok(0);
    }

    // Stage 3: Prepare chunks for embeddings
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

    // Stage 4: Generate embeddings
    let embeddings = if all_chunks.is_empty() {
        Vec::new()
    } else {
        embed_and_store::get_batch_embeddings(&all_chunks).map_err(|e| e.to_string())?
    };

    // Stage 5: Store in database
    let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;

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

    println!("Successfully inserted {} files", inserted_count);
    Ok(inserted_count)
}

/// Enhanced scan and store with progress reporting, using database rules.
pub fn scan_and_store_files_with_progress(
    db: &Connection,
    dir: &str,
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

    let paths = find_text_files_optimized(db, dir, max_file_size).map_err(|e| e.to_string())?;
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
                current: i + 1,
                total: paths.len(),
                current_file: path.clone(),
                stage: "reading".to_string(),
            },
        );

        match read_file_content_optimized(path, max_chars) {
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
                current: paths.len(),
                total: paths.len(),
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
            total: new_contents.len(),
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
                total: all_chunks.len(),
                current_file: "Generating embeddings...".to_string(),
                stage: "embedding".to_string(),
            },
        );

        let app_clone = app.clone();
        embed_and_store::get_batch_embeddings_with_progress(&all_chunks, move |current, total| {
            let _ = app_clone.emit(
                "scan_progress",
                crate::commands::ScanProgress {
                    current,
                    total,
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
            total: new_contents.len(),
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
                current: file_idx + 1,
                total: new_contents.len(),
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
            current: inserted_count,
            total: inserted_count,
            current_file: format!("Completed! Processed {} files", inserted_count),
            stage: "complete".to_string(),
        },
    );

    println!("Successfully inserted {} files", inserted_count);
    Ok(inserted_count)
}
