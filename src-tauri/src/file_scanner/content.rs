// Content extraction (read_file_content_with_category)
use super::types::{FileCategory, FileContent};
use super::utils::extract_drive;
use std::error::Error;
use std::fs;
use std::path::Path;
use tokio::runtime::Runtime;

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
                path_str,
                e
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
        FileCategory::Media => String::new(),
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

pub fn chunk_text(text: &str, max_words: usize) -> Vec<String> {
    text.split_whitespace()
        .collect::<Vec<&str>>()
        .chunks(max_words)
        .map(|chunk| chunk.join(" "))
        .filter(|s| s.len() > 50)
        .collect()
}

pub fn create_metadata_string(path_obj: &Path) -> String {
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
                    score: 0.0,
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
