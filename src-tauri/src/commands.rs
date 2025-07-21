// src/commands.rs
use tauri::State;
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use crate::services::user_service::UserService;
use crate::entities::user::Model as UserModel;
use crate::file_scanner;

#[tauri::command]
pub async fn create_user(
    name: String,
    email: String,
    user_service: State<'_, Arc<UserService>>,
) -> Result<UserModel, String> {
    user_service
        .create_user(name, email)
        .await
        .map_err(|e| format!("Database error: {}", e))
}

#[tauri::command]
pub async fn get_all_users(
    user_service: State<'_, Arc<UserService>>,
) -> Result<Vec<UserModel>, String> {
    user_service
        .get_all_users()
        .await
        .map_err(|e| format!("Database error: {}", e))
}

#[tauri::command]
pub async fn get_user_by_id(
    id: i32,
    user_service: State<'_, Arc<UserService>>,
) -> Result<Option<UserModel>, String> {
    user_service
        .get_user_by_id(id)
        .await
        .map_err(|e| format!("Database error: {}", e))
}

#[tauri::command]
pub async fn update_user(
    id: i32,
    name: Option<String>,
    email: Option<String>,
    user_service: State<'_, Arc<UserService>>,
) -> Result<UserModel, String> {
    user_service
        .update_user(id, name, email)
        .await
        .map_err(|e| format!("Database error: {}", e))
}

#[tauri::command]
pub async fn delete_user(
    id: i32,
    user_service: State<'_, Arc<UserService>>,
) -> Result<(), String> {
    user_service
        .delete_user(id)
        .await
        .map_err(|e| format!("Database error: {}", e))
}

#[tauri::command]
pub async fn scan_and_store_files(
    path: String,
    db: State<'_, DatabaseConnection>,  // Raw connection now
) -> Result<usize, String> {
    file_scanner::scan_and_store_files(&db, &path, None).await
}