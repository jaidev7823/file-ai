use std::path::Path;
use walkdir::WalkDir;
use std::fs;
use serde::{Serialize, Deserialize}; // Import these traits

/// List of common human-readable/editable text file extensions
const TEXT_EXTENSIONS: &[&str] = &[
    "txt", "md", "csv", "json", "xml", "log", "ini", "cfg", "yaml", "yml", "toml", "rs", "py", "js", "ts", "tsx", "jsx", "html", "css", "scss", "less", "bat", "sh", "c", "cpp", "h", "hpp", "java", "cs", "go", "php", "rb", "pl", "swift", "kt", "dart", "sql", "r", "m", "vb", "ps1", "lua", "tex", "scala", "erl", "ex", "exs", "clj", "cljs", "groovy", "asm", "s", "v", "sv", "makefile", "dockerfile", "gitignore", "gitattributes"
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
            !skip_dirs.iter().any(|&skip| skip.eq_ignore_ascii_case(name))
        } else {
            true
        }
    });
    for entry in walker.filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                if TEXT_EXTENSIONS.iter().any(|&allowed| allowed.eq_ignore_ascii_case(ext)) {
                    results.push(entry.path().to_string_lossy().to_string());
                }
            }
        }
    }
    results
}

#[derive(Serialize, Deserialize)] // Add these derives
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