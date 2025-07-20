mod database;
pub mod entities;
pub mod migration;
mod file_scanner;
pub mod services;
pub mod commands;

use services::user_service::UserService;
use std::sync::Arc;
use tauri::Manager; // Add this import

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
    // Create a runtime for async setup
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(move |app| {  // Added 'move' keyword here
            let app_handle = app.handle().clone();
            
            // Block on database initialization
            rt.block_on(async move {
                match database::init_database().await {
                    Ok(db) => {
                        // Create and manage the UserService with Arc for thread safety
                        let user_service = Arc::new(UserService::new(db));
                        app_handle.manage(user_service);
                        println!("SeaORM Database initialized successfully");
                    }
                    Err(e) => {
                        eprintln!("Failed to initialize SeaORM database: {}", e);
                        std::process::exit(1); // Exit if database fails
                    }
                }
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            scan_text_files,
            read_text_files,
            get_file_content,
            // Database commands
            commands::create_user,
            commands::get_all_users,
            commands::get_user_by_id,
            commands::update_user,
            commands::delete_user,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}