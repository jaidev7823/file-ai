// src/commands.rs
use crate::database::search::SearchFilters;
use crate::database::SearchResult;
use crate::embed_and_store;
use crate::file_scanner;
use crate::services::user_service::{User, UserService};
use std::process::Command;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn create_user(
    name: String,
    email: String,
    user_service: State<'_, Arc<UserService>>,
) -> Result<User, String> {
    user_service
        .create_user(name, email)
        .map_err(|e| format!("Database error: {}", e))
}

#[tauri::command]
pub fn get_all_users(user_service: State<'_, Arc<UserService>>) -> Result<Vec<User>, String> {
    user_service
        .get_all_users()
        .map_err(|e| format!("Database error: {}", e))
}

#[tauri::command]
pub fn get_user_by_id(
    id: i32,
    user_service: State<'_, Arc<UserService>>,
) -> Result<Option<User>, String> {
    user_service
        .get_user_by_id(id)
        .map_err(|e| format!("Database error: {}", e))
}

#[tauri::command]
pub fn update_user(
    id: i32,
    name: Option<String>,
    email: Option<String>,
    user_service: State<'_, Arc<UserService>>,
) -> Result<User, String> {
    user_service
        .update_user(id, name, email)
        .map_err(|e| format!("Database error: {}", e))
}

#[tauri::command]
pub fn delete_user(id: i32, user_service: State<'_, Arc<UserService>>) -> Result<(), String> {
    user_service
        .delete_user(id)
        .map_err(|e| format!("Database error: {}", e))
}

#[tauri::command]
pub async fn scan_and_store_files(path: String) -> Result<usize, String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        file_scanner::scan_and_store_files_optimized(&db, &path, None, Some(50_000_000))
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

#[derive(Clone, serde::Serialize)]
pub struct ScanProgress {
    pub current: usize,
    pub total: usize,
    pub current_file: String,
    pub stage: String, // "scanning", "reading", "embedding", "storing"
}

#[tauri::command]
pub async fn scan_and_store_files_with_progress(
    path: String,
    app: tauri::AppHandle,
) -> Result<usize, String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        file_scanner::scan_and_store_files_with_progress(
            &db,
            &path,
            None,
            Some(50_000_000),
            app,
        )
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn search_files_test(
    query: String,
    top_k: Option<usize>,
) -> Result<Vec<SearchResult>, String> {
    let limit = top_k.unwrap_or(5);

    // query is only used here, so it can be moved directly
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        crate::database::search_files_fts(&db, &query, limit)
            .map_err(|e| format!("Database error: {}", e))
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn search_files(
    query: String,
    top_k: Option<usize>,
    filters: Option<SearchFilters>,
) -> Result<Vec<SearchResult>, String> {
    let limit = top_k.unwrap_or(5);

    // Clone query for the first blocking task (embedding generation)
    let query_for_embedding = query.clone();
    // The original `query` can be moved into the second blocking task (DB search)

    // Step 1: Get embedding synchronously by spawning a blocking task for it
    let query_embedding_task_result = tokio::task::spawn_blocking(move || {
        embed_and_store::get_embedding(&query_for_embedding) // Use the cloned query
            .map_err(|e| format!("Embedding error in blocking task: {}", e))
    })
    .await;

    let query_embedding = match query_embedding_task_result {
        Ok(inner_result) => inner_result?,
        Err(join_err) => {
            return Err(format!(
                "Failed to spawn blocking task for embedding: {}",
                join_err
            ));
        }
    };

    let normalized = embed_and_store::normalize(query_embedding);

    // Step 2: Move to blocking context for database operations
    // `query` (the original) and `normalized` are moved into this closure
    let filters = filters.unwrap_or_default(); // Use default if none passed

    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        crate::database::hybrid_search_with_embedding(&db, &normalized, &query, filters, limit)
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn search_indexed_files(
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, String> {
    let limit = limit.unwrap_or(10);

    // This is essentially the same as search_files_test but with a more specific name
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        crate::database::search_files_fts(&db, &query, limit)
            .map_err(|e| format!("Database error: {}", e))
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn test_embedding(query: String) -> Result<String, String> {
    println!("Testing embedding for: {}", query);

    // Clone query for the blocking task
    let query_for_embedding = query.clone();

    let embedding_task_result = tokio::task::spawn_blocking(move || {
        embed_and_store::get_embedding(&query_for_embedding) // Use the cloned query
            .map_err(|e| format!("Embedding error: {}", e))
    })
    .await;

    let embedding = match embedding_task_result {
        Ok(inner_result) => inner_result?,
        Err(join_err) => return Err(format!("Task spawn error: {}", join_err)),
    };

    // Original `query` is still available here for the final format string
    Ok(format!(
        "Got embedding with {} dimensions for query: {}",
        embedding.len(),
        query
    ))
}

#[tauri::command]
pub fn open_file(file_path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", &file_path])
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&file_path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&file_path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub fn open_file_with(file_path: String, application: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new(&application)
            .arg(&file_path)
            .spawn()
            .map_err(|e| format!("Failed to open file with {}: {}", application, e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", &application, &file_path])
            .spawn()
            .map_err(|e| format!("Failed to open file with {}: {}", application, e))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new(&application)
            .arg(&file_path)
            .spawn()
            .map_err(|e| format!("Failed to open file with {}: {}", application, e))?;
    }

    Ok(())
}

#[tauri::command]
pub fn show_file_in_explorer(file_path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .args(["/select,", &file_path])
            .spawn()
            .map_err(|e| format!("Failed to show file in explorer: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-R", &file_path])
            .spawn()
            .map_err(|e| format!("Failed to show file in finder: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        // Try to get the directory and open it
        if let Some(parent) = std::path::Path::new(&file_path).parent() {
            Command::new("xdg-open")
                .arg(parent)
                .spawn()
                .map_err(|e| format!("Failed to show file in file manager: {}", e))?;
        } else {
            return Err("Could not determine parent directory".to_string());
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn select_folder() -> Result<String, String> {
    // For now, return a placeholder. You'll need to implement folder selection
    // This would typically use a file dialog
    Err("Folder selection not implemented yet".to_string())
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ScanSettings {
    pub scan_paths: Vec<String>,
    pub ignored_folders: Vec<String>,
}

#[tauri::command]
pub async fn save_scan_settings(settings: ScanSettings) -> Result<(), String> {
    // For now, just log the settings. You can implement actual storage later
    println!("Saving scan settings: {:?}", settings);
    Ok(())
}

#[tauri::command]
pub async fn load_scan_settings() -> Result<ScanSettings, String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        
        // Get excluded paths from database
        let mut stmt = db.prepare("SELECT path FROM path_rules WHERE rule_type = 'exclude'")
            .map_err(|e| format!("Database error: {}", e))?;
        let ignored_folders: Result<Vec<String>, _> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| format!("Database error: {}", e))?
            .collect();
        
        Ok(ScanSettings {
            scan_paths: vec!["C://Users/Jai Mishra/OneDrive/Documents".to_string()],
            ignored_folders: ignored_folders.map_err(|e| format!("Database error: {}", e))?,
        })
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn get_excluded_paths() -> Result<Vec<String>, String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        let mut stmt = db.prepare("SELECT path FROM path_rules WHERE rule_type = 'exclude'")
            .map_err(|e| format!("Database error: {}", e))?;
        let paths: Result<Vec<String>, _> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| format!("Database error: {}", e))?
            .collect();
        paths.map_err(|e| format!("Database error: {}", e))
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn get_included_extensions() -> Result<Vec<String>, String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        let mut stmt = db.prepare("SELECT extension FROM extension_rules WHERE rule_type = 'include'")
            .map_err(|e| format!("Database error: {}", e))?;
        let extensions: Result<Vec<String>, _> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| format!("Database error: {}", e))?
            .collect();
        extensions.map_err(|e| format!("Database error: {}", e))
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
pub async fn test_file_filtering(path: String) -> Result<Vec<String>, String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        
        // Test the filtering function directly
        match file_scanner::find_text_files_optimized(&db, &path, Some(50_000_000)) {
            Ok(files) => {
                println!("Found {} files after filtering", files.len());
                Ok(files)
            }
            Err(e) => Err(format!("Error during file filtering: {}", e))
        }
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn debug_database_rules() -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        
        let mut debug_info = String::new();
        
        // Check path rules
        let mut stmt = db.prepare("SELECT path, rule_type FROM path_rules")
            .map_err(|e| format!("Database error: {}", e))?;
        let path_rules: Result<Vec<(String, String)>, _> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| format!("Database error: {}", e))?
            .collect();
        
        debug_info.push_str("Path Rules:\n");
        for (path, rule_type) in path_rules.map_err(|e| format!("Database error: {}", e))? {
            debug_info.push_str(&format!("  {} - {}\n", path, rule_type));
        }
        
        // Check extension rules
        let mut stmt = db.prepare("SELECT extension, rule_type FROM extension_rules")
            .map_err(|e| format!("Database error: {}", e))?;
        let ext_rules: Result<Vec<(String, String)>, _> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| format!("Database error: {}", e))?
            .collect();
        
        debug_info.push_str("\nExtension Rules:\n");
        for (ext, rule_type) in ext_rules.map_err(|e| format!("Database error: {}", e))? {
            debug_info.push_str(&format!("  {} - {}\n", ext, rule_type));
        }
        
        Ok(debug_info)
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}
