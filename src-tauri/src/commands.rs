// src/commands.rs
use tauri::State;
use std::sync::Arc;
use rusqlite::Connection;
use crate::services::user_service::{UserService, User};
use crate::file_scanner;

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
pub fn get_all_users(
    user_service: State<'_, Arc<UserService>>,
) -> Result<Vec<User>, String> {
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
pub fn delete_user(
    id: i32,
    user_service: State<'_, Arc<UserService>>,
) -> Result<(), String> {
    user_service
        .delete_user(id)
        .map_err(|e| format!("Database error: {}", e))
}

#[tauri::command]
pub async fn scan_and_store_files(
    path: String,
    db: State<'_, Arc<std::sync::Mutex<Connection>>>,
) -> Result<usize, String> {
    let db_clone = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let db_conn = db_clone.lock().unwrap();
        file_scanner::scan_and_store_files(&db_conn, &path, None)
            .map_err(|e| format!("Database error: {}", e))
    }).await.unwrap()
}

