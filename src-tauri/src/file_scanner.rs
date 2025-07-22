use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use rusqlite::{Connection, Result, params};
use chrono::{Utc, DateTime};
use walkdir::WalkDir;
use reqwest::blocking::Client;
use std::error::Error;
use bytemuck::{cast_slice};

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
    "ini",
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
];

/// Recursively scans a directory for human-readable text files and returns their paths.
pub fn find_text_files<P: AsRef<Path>>(dir: P) -> Vec<String> {
    let mut results = Vec::new();
    let skip_dirs = ["node_modules", ".venv"];
    let walker = WalkDir::new(dir).into_iter().filter_entry(|entry| {
        // Always include files
        if entry.file_type().is_file() {
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
            if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                if TEXT_EXTENSIONS
                    .iter()
                    .any(|&allowed| allowed.eq_ignore_ascii_case(ext))
                {
                    results.push(entry.path().to_string_lossy().to_string());
                }
            }
        }
    }
    results
}

#[derive(Serialize, Deserialize)]
pub struct FileContent {
    pub path: String,
    pub content: String,
}

/// Reads the content of each file in the given list of paths.
/// Optionally limits the number of characters read per file.
pub fn read_files_content(paths: &[String], max_chars: Option<usize>) -> Vec<FileContent> {
    let mut results = Vec::new();
    for path in paths {
        match fs::read_to_string(path) {
            Ok(mut content) => {
                if let Some(max) = max_chars {
                    if content.len() > max {
                        content.truncate(max);
                    }
                }
                results.push(FileContent {
                    path: path.clone(),
                    content,
                });
            }
            Err(_) => {
                // Skip unreadable files
                continue;
            }
        }
    }
    results
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
}

fn get_embedding(text: &str) -> Result<Vec<f32>, Box<dyn Error>> {
    let client = Client::new();
    let res: EmbeddingResponse = client
        .post("http://localhost:11434/api/embeddings")
        .json(&serde_json::json!({
            "model": "nomic-embed-text",
            "prompt": text
        }))
        .send()?
        .json()?;

    Ok(res.embedding)
}

/// Checks if file exists in database
fn file_exists(db: &Connection, path: &str) -> Result<bool> {
    let mut stmt = db.prepare("SELECT COUNT(*) FROM files WHERE path = ?1")?;
    let count: i64 = stmt.query_row(params![path], |row| row.get(0))?;
    Ok(count > 0)
}

pub fn scan_and_store_files(
    db: &Connection,
    dir: &str,
    max_chars: Option<usize>,
) -> Result<usize, String> {
    let paths = find_text_files(dir);
    let contents = read_files_content(&paths, max_chars);
    let mut inserted_count = 0;

    for file_content in contents {
        if !file_exists(db, &file_content.path).map_err(|e| e.to_string())? {
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

            // Insert file into `files`
            db.execute(
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

            // Get inserted file_id
            let file_id = db.last_insert_rowid();

            // Get embedding from Ollama
            if file_content.content.trim().is_empty() {
                continue; // Skip empty files
            }           

            let vector = get_embedding(&file_content.content).map_err(|e| e.to_string())?;          

            if vector.is_empty() {
                return Err("Embedding failed: received empty vector".into());
            }           
            println!("Inserted vector with {} dimensions for file: {}", vector.len(), file_content.path);

            // Insert into file_vec
            let vector_bytes: &[u8] = cast_slice(&vector);
            db.execute("INSERT INTO file_vec(content_vec) VALUES(?1)", params![vector_bytes])
                .map_err(|e| e.to_string())?;           

            let vec_rowid = db.last_insert_rowid();

            // Insert into file_vec_map
            db.execute(
                "INSERT INTO file_vec_map(vec_rowid, file_id) VALUES(?1, ?2)",
                params![vec_rowid, file_id],
            ).map_err(|e| e.to_string())?;

            inserted_count += 1;
        }
    }

    Ok(inserted_count)
}