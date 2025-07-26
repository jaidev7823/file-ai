mod database;
mod file_scanner;
pub mod services;
pub mod commands;

use services::user_service::UserService;
use std::sync::Arc;
use tauri::Manager;
mod embed_and_store;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn scan_text_files(path: String) -> Vec<String> {
    file_scanner::find_text_files(path)
}

#[tauri::command]
// This command now uses the synchronous file_scanner::read_files_content_sync
// wrapped in spawn_blocking.
async fn read_text_files(paths: Vec<String>, max_chars: Option<usize>) -> Result<Vec<file_scanner::FileContent>, String> {
    tokio::task::spawn_blocking(move || {
        Ok(file_scanner::read_files_content_sync(&paths, max_chars))
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
// This command now uses the synchronous file_scanner::read_files_content_sync
// wrapped in spawn_blocking.
async fn get_file_content(path: String, max_chars: Option<usize>) -> Result<Option<file_scanner::FileContent>, String> {
    tokio::task::spawn_blocking(move || {
        let results = file_scanner::read_files_content_sync(&[path], max_chars);
        Ok(results.into_iter().next())
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(move |app| {
            let app_handle = app.handle().clone();
            
            match database::init_database() {
                Ok(db) => {
                    let db_mutex = Arc::new(std::sync::Mutex::new(db));
                    // Create user service with the connection
                    let user_service = UserService::new();
                    
                    // Manage both the raw connection and service
                    app_handle.manage(db_mutex);
                    app_handle.manage(Arc::new(user_service));
                    
                    println!("Database initialized");
                }
                Err(e) => {
                    eprintln!("Database error: {}", e);
                    std::process::exit(1);
                }
            }
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            scan_text_files,
            read_text_files,
            get_file_content,
            commands::create_user,
            commands::get_all_users,
            commands::get_user_by_id,
            commands::update_user,
            commands::delete_user,
            commands::scan_and_store_files,
            commands::search_files,
            commands::search_files_test, // Added for testing
            commands::test_embedding, // Added for testing
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}