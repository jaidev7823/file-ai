pub mod commands;
mod database;
mod file_scanner;
pub mod services;

use services::user_service::UserService;
use std::sync::Arc;
use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
mod embed_and_store;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn toggle_search_window(app: tauri::AppHandle) -> Result<(), String> {
    println!("Toggle search window called");

    if let Some(search_window) = app.get_webview_window("search") {
        println!("Search window found");
        let is_visible = search_window.is_visible().map_err(|e| e.to_string())?;
        println!("Search window visible: {}", is_visible);

        if is_visible {
            println!("Hiding search window");
            search_window.hide().map_err(|e| e.to_string())?;
        } else {
            println!("Showing search window");
            search_window.show().map_err(|e| e.to_string())?;
            search_window.set_focus().map_err(|e| e.to_string())?;
        }
    } else {
        println!("Search window not found! Creating it...");

        // Create the search window if it doesn't exist
        let search_window = tauri::WebviewWindowBuilder::new(
            &app,
            "search",
            tauri::WebviewUrl::App("index.html".into()),
        )
        .title("Search")
        .inner_size(600.0, 400.0)
        .resizable(false)
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .center()
        .focused(true)
        .build()
        .map_err(|e| e.to_string())?;

        println!("Search window created successfully");
        search_window.show().map_err(|e| e.to_string())?;
        search_window.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn hide_search_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(search_window) = app.get_webview_window("search") {
        search_window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn scan_text_files(path: String) -> Vec<String> {
    file_scanner::find_text_files(path)
}

#[tauri::command]
// This command now uses the synchronous file_scanner::read_files_content_sync
// wrapped in spawn_blocking.
async fn read_text_files(
    paths: Vec<String>,
    max_chars: Option<usize>,
) -> Result<Vec<file_scanner::FileContent>, String> {
    tokio::task::spawn_blocking(move || {
        Ok(file_scanner::read_files_content_sync(&paths, max_chars))
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
// This command now uses the synchronous file_scanner::read_files_content_sync
// wrapped in spawn_blocking.
async fn get_file_content(
    path: String,
    max_chars: Option<usize>,
) -> Result<Option<file_scanner::FileContent>, String> {
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

            #[cfg(desktop)]
            {
                // Create the shortcut we want (Ctrl+Shift+P)
                let ctrl_shift_p_shortcut =
                    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyP);

                // Clone app_handle for the handler
                let app_handle_for_handler = app_handle.clone();

                // Register the plugin with handler using the documentation pattern
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |_app, shortcut, event| {
                            println!(
                                "Shortcut triggered: {:?}, State: {:?}",
                                shortcut,
                                event.state()
                            );

                            if shortcut == &ctrl_shift_p_shortcut {
                                match event.state() {
                                    ShortcutState::Pressed => {
                                        println!("Ctrl+Shift+P Pressed! Toggling search window...");
                                        let app_handle = app_handle_for_handler.clone();
                                        tauri::async_runtime::spawn(async move {
                                            if let Err(e) = toggle_search_window(app_handle).await {
                                                eprintln!("Failed to toggle search window: {}", e);
                                            }
                                        });
                                    }
                                    ShortcutState::Released => {
                                        println!("Ctrl+Shift+P Released!");
                                    }
                                }
                            }
                        })
                        .build(),
                )?;

                // Register the shortcut
                match app.global_shortcut().register(ctrl_shift_p_shortcut) {
                    Ok(_) => println!("Successfully registered Ctrl+Shift+P global shortcut"),
                    Err(e) => eprintln!("Failed to register Ctrl+Shift+P: {}", e),
                }
            }

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

            println!("Setup completed");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            toggle_search_window,
            hide_search_window,
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
            commands::test_embedding,    // Added for testing
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}
