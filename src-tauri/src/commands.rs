// src/commands.rs
use crate::database::search::{perform_file_search, SearchFilters};
use crate::database::SearchResult;
use crate::file_scanner;
use crate::search_window::toggle_search_window_impl;
use crate::services::user_service::{User, UserService};
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Manager;
use tauri::State;

use crate::file_ops::{open_file_impl, open_file_with_impl, show_file_in_explorer_impl};

#[derive(Clone, serde::Serialize)]
pub struct ScanProgress {
    pub current: usize,
    pub total: usize,
    pub current_file: String,
    pub stage: String, // "scanning", "reading", "embedding", "storing"
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ScanSettings {
    pub scan_paths: Vec<String>,
    pub ignored_folders: Vec<String>,
}

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
pub async fn toggle_search_window(app: AppHandle) -> Result<(), String> {
    toggle_search_window_impl(app).await
}

#[tauri::command]
pub async fn hide_search_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(search_window) = app.get_webview_window("search") {
        search_window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn scan_text_files(
    path: String,
    ignored_folders: Option<Vec<String>>,
) -> Result<Vec<String>, String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        file_scanner::find_text_files(&db, &path, Some(50_000_000)).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn read_text_files(
    paths: Vec<String>,
    max_chars: Option<usize>,
) -> Result<Vec<file_scanner::FileContent>, String> {
    tokio::task::spawn_blocking(move || Ok(file_scanner::read_files_content(&paths, max_chars)))
        .await
        .map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
// This command now uses the synchronous file_scanner::read_files_content
// wrapped in spawn_blocking.
pub async fn get_file_content(
    path: String,
    max_chars: Option<usize>,
) -> Result<Option<file_scanner::FileContent>, String> {
    tokio::task::spawn_blocking(move || {
        let results = file_scanner::read_files_content(&[path], max_chars);
        Ok(results.into_iter().next())
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn scan_and_store_files(path: String, app: tauri::AppHandle) -> Result<usize, String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        file_scanner::scan_and_store_files(&db, &path, None, Some(50_000_000), app)
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn load_scan_settings() -> Result<ScanSettings, String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();

        // Get excluded paths from database
        let mut stmt = db
            .prepare("SELECT path FROM path_rules WHERE rule_type = 'exclude'")
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
pub async fn search_files(
    query: String,
    top_k: Option<usize>,
    filters: Option<SearchFilters>,
) -> Result<Vec<SearchResult>, String> {
    perform_file_search(query, top_k, filters).await
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
pub fn open_file(file_path: String) -> Result<(), String> {
    open_file_impl(file_path)
}

#[tauri::command]
pub fn open_file_with(file_path: String, application: String) -> Result<(), String> {
    open_file_with_impl(file_path, application)
}

#[tauri::command]
pub fn show_file_in_explorer(file_path: String) -> Result<(), String> {
    show_file_in_explorer_impl(file_path)
}

#[tauri::command]
pub async fn select_folder() -> Result<String, String> {
    // For now, return a placeholder. You'll need to implement folder selection
    // This would typically use a file dialog
    Err("Folder selection not implemented yet".to_string())
}

#[tauri::command]
pub async fn save_scan_settings(settings: ScanSettings) -> Result<(), String> {
    // For now, just log the settings. You can implement actual storage later
    println!("Saving scan settings: {:?}", settings);
    Ok(())
}
