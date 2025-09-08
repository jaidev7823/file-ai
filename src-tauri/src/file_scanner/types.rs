// Data structures (FileContent, FileCategory)
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    pub score: f64, // The calculated score of the file
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

#[derive(serde::Serialize, Clone)]
pub struct ScannedFile {
    pub path: String,
    pub content_processed: bool,
}
