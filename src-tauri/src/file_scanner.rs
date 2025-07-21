use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use walkdir::WalkDir;
use sea_orm::{
    entity::prelude::*,
    ActiveModelTrait, ColumnTrait, DatabaseConnection, 
    EntityTrait, QueryFilter, Set
};
use crate::entities::file; // Assuming you have a SeaORM entity generated
use crate::vss::insert_embedding;

use ollama_rs::{
    Ollama,
    generation::embeddings::{
        request::GenerateEmbeddingsRequest
    }
};

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
    pub embedding: Vec<f32>, 
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
                    embedding: Vec::new(),
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


/// Checks if file exists in database
async fn file_exists(db: &DatabaseConnection, path: &str) -> Result<bool, DbErr> {
    file::Entity::find()
        .filter(file::Column::Path.eq(path))
        .count(db)
        .await
        .map(|count| count > 0)
}


/// Inserts files only if they don't exist
pub async fn scan_and_store_files(
    db: &DatabaseConnection,
    dir: &str,
    max_chars: Option<usize>,
) -> Result<usize, String> {
    let paths = find_text_files(dir);
    let mut inserted_count = 0;

    for path in paths {
        if !file_exists(db, &path).await.map_err(|e| e.to_string())? {
            // Read file content
            let mut content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => {
                    // Skip unreadable files
                    continue;
                }
            };

            // Truncate content if max_chars is set
            if let Some(max) = max_chars {
                if content.len() > max {
                    content.truncate(max);
                }
            }

            // Generate embedding for the content
            let embedding = match generate_embedding(&content).await {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("Error generating embedding for file {}: {}", &path, e);
                    continue; // Skip file if embedding fails
                }
            };

            let new_file = file::ActiveModel {
                name: Set(Path::new(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string()),
                extension: Set(Path::new(&path)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_string()),
                path: Set(path.clone()),
                content: Set(content),
                ..Default::default()
            };

            let file_model = new_file.insert(db).await.map_err(|e| e.to_string())?;
            
            insert_embedding(db, file_model.id, &embedding).await.map_err(|e| e.to_string())?;

            inserted_count += 1;
        }
    }

    Ok(inserted_count)
}

pub async fn generate_embedding(text: &str) -> Result<Vec<f32>, String> {
    let ollama = Ollama::default(); // Connects to localhost:11434 by default
    let model_name = "nomic-embed-text".to_string(); // Or "mxbai-embed-large" or your chosen embedding model

    let request = GenerateEmbeddingsRequest::new(model_name, vec![text.to_string()].into());

    match ollama.generate_embeddings(request).await {
        Ok(response) => {
            if let Some(embedding_vec) = response.embeddings.into_iter().next() {
                Ok(embedding_vec)
            } else {
                Err("No embedding found in the response.".to_string())
            }
        },
        Err(e) => Err(format!("Failed to generate embedding: {}", e)),
    }
}
