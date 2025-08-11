use rusqlite::Connection;
use std::collections::HashSet;
use std::error::Error;
use chrono::Utc;
use rusqlite::params;

pub fn get_included_paths_sync(db: &Connection) -> Result<HashSet<String>, Box<dyn Error>> {
    let mut stmt = db.prepare("SELECT path FROM path_rules WHERE rule_type = 'include'")?;
    let paths = stmt.query_map([], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    println!("{:?}", paths);
    Ok(paths.into_iter().collect())
}

pub fn get_included_folders_sync(db: &Connection) -> Result<HashSet<String>, Box<dyn Error>> {
    let mut stmt = db.prepare("SELECT folder_name FROM folder_rules WHERE rule_type = 'include'")?;
    let folders = stmt.query_map([], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    println!("{:?}", folders);
    Ok(folders.into_iter().collect())
}

pub fn get_included_extensions_sync(db: &Connection) -> Result<HashSet<String>, Box<dyn Error>> {
    let mut stmt = db.prepare("SELECT extension FROM extension_rules WHERE rule_type = 'include'")?;
    let extensions = stmt.query_map([], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    println!("{:?}", extensions);
    Ok(extensions.into_iter().collect())
}

pub fn get_excluded_extensions_sync(db: &Connection) -> Result<HashSet<String>, Box<dyn Error>> {
    let mut stmt = db.prepare("SELECT extension FROM extension_rules WHERE rule_type = 'exclude'")?;
    let extensions = stmt.query_map([], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    println!("{:?}", extensions);
    Ok(extensions.into_iter().collect())
}

pub fn get_excluded_filenames_sync(db: &Connection) -> Result<HashSet<String>, Box<dyn Error>> {
    let mut stmt = db.prepare("SELECT filename FROM filename_rules WHERE rule_type = 'exclude'")?;
    let filenames = stmt.query_map([], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    println!("{:?}", filenames);
    Ok(filenames.into_iter().collect())
}

pub fn get_excluded_folder_sync(db: &Connection) -> Result<HashSet<String>, Box<dyn Error>> {
    let mut stmt = db.prepare("SELECT path FROM path_rules WHERE rule_type = 'exclude'")?;
    let paths = stmt.query_map([], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    println!("{:?}", paths);
    Ok(paths.into_iter().collect())
}

pub fn get_excluded_paths_sync(db: &Connection) -> Result<HashSet<String>, Box<dyn Error>> {
    let mut stmt = db.prepare("SELECT path FROM path_rules WHERE rule_type = 'exclude'")?;
    let paths = stmt.query_map([], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    println!("{:?}", paths);
    Ok(paths.into_iter().collect())
}

#[tauri::command]
pub async fn add_included_extension(extension: String) -> Result<(), String> {
    println!("working");
    tokio::task::spawn_blocking(move || {
        println!("tokio working");
        let db = crate::database::get_connection();
        println!("db working");
        let now = chrono::Utc::now().to_rfc3339();
        println!("chrono working");
        db.execute(
            "INSERT INTO extension_rules (extension, rule_type, created_at) VALUES (?1, 'include', ?2)",
            rusqlite::params![extension, now],
        ).map_err(|e| format!("Database error: {}", e))?;
        Ok(())
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn add_excluded_folder(path: String) -> Result<(), String> {
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

#[tauri::command]
pub async fn remove_excluded_folder(pathfolder: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        db.execute(
            "DELETE FROM path_rules WHERE path = ?1 AND rule_type = 'exclude'",
            rusqlite::params![pathfolder],
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
pub async fn add_included_path(path: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        let now = Utc::now().to_rfc3339();
        db.execute(
            "INSERT INTO path_rules (path, rule_type, is_recursive, created_at) VALUES (?1, 'include', true, ?2)",
            params![path, now],
        ).map_err(|e| format!("Database error: {}", e))?;
        Ok(())
    }).await.map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn remove_included_path(path: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        db.execute(
            "DELETE FROM path_rules WHERE path = ?1 AND rule_type = 'include'",
            params![path],
        ).map_err(|e| format!("Database error: {}", e))?;
        Ok(())
    }).await.map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn add_excluded_path(path: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        let now = Utc::now().to_rfc3339();
        db.execute(
            "INSERT INTO path_rules (path, rule_type, is_recursive, created_at) VALUES (?1, 'exclude', true, ?2)",
            params![path, now],
        ).map_err(|e| format!("Database error: {}", e))?;
        Ok(())
    }).await.map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn remove_excluded_path(path: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        db.execute(
            "DELETE FROM path_rules WHERE path = ?1 AND rule_type = 'exclude'",
            params![path],
        ).map_err(|e| format!("Database error: {}", e))?;
        Ok(())
    }).await.map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn add_included_folder(folderName: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        let now = Utc::now().to_rfc3339();
        db.execute(
            "INSERT INTO folder_rules (folder_name, rule_type, is_recursive, created_at) VALUES (?1, 'include', true, ?2)",
            params![folderName, now],
        ).map_err(|e| format!("Database error: {}", e))?;
        Ok(())
    }).await.map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn remove_included_folder(folderName: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        db.execute(
            "DELETE FROM folder_rules WHERE folder_name = ?1 AND rule_type = 'include'",
            params![folderName],
        ).map_err(|e| format!("Database error: {}", e))?;
        Ok(())
    }).await.map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn add_excluded_extension(extension: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        let now = Utc::now().to_rfc3339();
        db.execute(
            "INSERT INTO extension_rules (extension, rule_type, created_at) VALUES (?1, 'exclude', ?2)",
            params![extension, now],
        ).map_err(|e| format!("Database error: {}", e))?;
        Ok(())
    }).await.map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn remove_excluded_extension(extension: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        db.execute(
            "DELETE FROM extension_rules WHERE extension = ?1 AND rule_type = 'exclude'",
            params![extension],
        ).map_err(|e| format!("Database error: {}", e))?;
        Ok(())
    }).await.map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn add_excluded_filename(filename: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        let now = Utc::now().to_rfc3339();
        db.execute(
            "INSERT INTO filename_rules (filename, rule_type, created_at) VALUES (?1, 'exclude', ?2)",
            params![filename, now],
        ).map_err(|e| format!("Database error: {}", e))?;
        Ok(())
    }).await.map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn remove_excluded_filename(filename: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        db.execute(
            "DELETE FROM filename_rules WHERE filename = ?1 AND rule_type = 'exclude'",
            params![filename],
        ).map_err(|e| format!("Database error: {}", e))?;
        Ok(())
    }).await.map_err(|e| format!("Task spawn error: {}", e))?
}