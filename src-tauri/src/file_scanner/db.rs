// All database interactions
use super::types::FileContent;
use crate::embed_and_store::normalize;
use anyhow;
use bytemuck::cast_slice;
use chrono::{DateTime, Utc};
use rusqlite::OptionalExtension;
use rusqlite::{params, Connection, Result, Transaction};
use std::fs;
use std::path::Path;

pub fn insert_folder_metadata(tx: &Transaction, path: &Path) -> anyhow::Result<i64> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let path_str = path.to_string_lossy().to_string();

    let mut stmt_exists = tx.prepare("SELECT id FROM folders WHERE path = ?1")?;
    if let Ok(id) = stmt_exists.query_row(params![&path_str], |row| row.get::<_, i64>(0)) {
        return Ok(id);
    }

    let metadata = fs::metadata(path).map_err(|e| anyhow::anyhow!("IO Error: {}", e))?;
    let created = Into::<DateTime<Utc>>::into(
        metadata
            .created()
            .map_err(|e| anyhow::anyhow!("IO Error: {}", e))?,
    )
    .to_rfc3339();
    let updated = Into::<DateTime<Utc>>::into(
        metadata
            .modified()
            .map_err(|e| anyhow::anyhow!("IO Error: {}", e))?,
    )
    .to_rfc3339();
    let accessed = Into::<DateTime<Utc>>::into(
        metadata
            .accessed()
            .map_err(|e| anyhow::anyhow!("IO Error: {}", e))?,
    )
    .to_rfc3339();

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

pub fn file_exists(db: &Connection, path: &str) -> Result<bool> {
    let mut stmt = db.prepare("SELECT 1 FROM files WHERE path = ?1 COLLATE NOCASE")?;
    stmt.exists(params![path])
}

pub fn insert_file_metadata(tx: &Transaction, file: &FileContent) -> anyhow::Result<i64> {
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
    let metadata = fs::metadata(&file.path).map_err(|_| rusqlite::Error::InvalidQuery)?;

    let created = Utc::now().to_rfc3339();
    let updated = Utc::now().to_rfc3339();
    let accessed = Into::<DateTime<Utc>>::into(metadata.accessed()?).to_rfc3339();

    let mut stmt = tx.prepare(
        "INSERT INTO files (name, extension, path, content, author, file_size, category, score, content_processed, created_at, updated_at, last_accessed)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"
    )?;
    stmt.execute(params![
        file_name,
        extension,
        file.path,
        file.content,
        None::<String>,
        metadata.len(),
        format!("{:?}", file.category),
        file.score,
        file.content_processed,
        created,
        updated,
        accessed,
    ])?;
    Ok(tx.last_insert_rowid())
}

pub fn insert_file_embedding(tx: &Transaction, file_id: i64, vector: Vec<f32>) -> Result<()> {
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

pub fn verify_embeddings(db: &Connection, file_id: i64) -> Result<bool, rusqlite::Error> {
    let count: i64 = db.query_row(
        "SELECT COUNT(*) FROM file_vec_map WHERE file_id = ?1",
        params![file_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}
