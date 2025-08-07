use rusqlite::Connection;
use std::collections::HashSet;
use std::error::Error;

pub fn get_excluded_paths_sync(db: &Connection) -> Result<HashSet<String>, Box<dyn Error>> {
    let mut stmt = db.prepare("SELECT path FROM path_rules WHERE rule_type = 'exclude'")?;
    let paths = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    Ok(paths.into_iter().collect())
}

pub fn get_included_extensions_sync(db: &Connection) -> Result<HashSet<String>, Box<dyn Error>> {
    let mut stmt = db.prepare("SELECT extension FROM extension_rules WHERE rule_type = 'include'")?;
    let extensions = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    Ok(extensions.into_iter().collect())
}

#[tauri::command]
pub async fn remove_excluded_path(path: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        db.execute(
            "DELETE FROM path_rules WHERE path = ?1 AND rule_type = 'exclude'",
            rusqlite::params![path],
        )
        .map_err(|e| format!("Database error: {}", e))?;
        Ok(())
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn add_included_extension(extension: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        let now = chrono::Utc::now().to_rfc3339();
        db.execute(
            "INSERT INTO extension_rules (extension, rule_type, created_at) VALUES (?1, 'include', ?2)",
            rusqlite::params![extension, now],
        )
        .map_err(|e| format!("Database error: {}", e))?;
        Ok(())
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn remove_included_extension(extension: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        db.execute(
            "DELETE FROM extension_rules WHERE extension = ?1 AND rule_type = 'include'",
            rusqlite::params![extension],
        )
        .map_err(|e| format!("Database error: {}", e))?;
        Ok(())
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn add_excluded_path(path: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        let now = chrono::Utc::now().to_rfc3339();
        db.execute(
            "INSERT INTO path_rules (path, rule_type, is_recursive, created_at) VALUES (?1, 'exclude', true, ?2)",
            rusqlite::params![path, now],
        )
        .map_err(|e| format!("Database error: {}", e))?;
        Ok(())
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}
