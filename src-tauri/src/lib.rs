mod database;
mod file_scanner;
pub mod services;
pub mod commands;

use services::user_service::UserService;
use std::sync::Arc;
use tauri::Manager;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn scan_text_files(path: String) -> Vec<String> {
    file_scanner::find_text_files(path)
}

#[tauri::command]
fn read_text_files(paths: Vec<String>, max_chars: Option<usize>) -> Vec<file_scanner::FileContent> {
    file_scanner::read_files_content(&paths, max_chars)
}

#[tauri::command]
fn get_file_content(path: String, max_chars: Option<usize>) -> Option<file_scanner::FileContent> {
    let results = file_scanner::read_files_content(&[path], max_chars);
    results.into_iter().next()
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
                    let user_service = UserService::new(db_mutex.clone());
                    
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
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}