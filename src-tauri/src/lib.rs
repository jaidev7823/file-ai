pub mod commands;
mod database;
pub mod file_ops;
mod file_scanner;
pub mod search_window;
pub mod services;
mod shortcuts;
pub mod test;
use services::user_service::UserService;
use std::sync::Arc;
use tauri::Manager;
mod embed_and_store;
use crate::test::{debug_database_rules, test_embedding, test_file_filtering};
use crate::database::rules::{
    add_excluded_folder,
    remove_excluded_folder,
    add_included_extension,
    remove_included_extension,
    add_included_path,
    add_excluded_path,
    add_included_folder,
    add_excluded_extension,
    add_excluded_filename,
    remove_included_path,
    remove_excluded_path,
    remove_included_folder,
    remove_excluded_extension,
    remove_excluded_filename,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(move |app| {
            let app_handle = app.handle().clone();

            #[cfg(desktop)]
            {
                if let Err(e) = crate::shortcuts::register_global_shortcuts(&app_handle) {
                    eprintln!("Global shortcut setup failed: {}", e);
                }
            }

            // Initialize database and seed initial data
            match database::initialize() {
                Ok(_) => {
                    let user_service = UserService::new();
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
            commands::hide_search_window,
            commands::read_text_files,
            commands::get_file_content,
            commands::create_user,
            commands::get_all_users,
            commands::get_user_by_id,
            commands::update_user,
            commands::delete_user,
            // scan and embed and sve file commands
            commands::scan_text_files,
            commands::scan_and_store_files,
            commands::search_files,
            commands::search_indexed_files, // New search command
            // open file and open file with commands
            commands::open_file,             // New file opening command
            commands::open_file_with,        // New open with command
            commands::show_file_in_explorer, // New show in explorer command
            commands::select_folder,         // Folder selection command

            // other operations to site
            commands::toggle_search_window,

            // test debugs
            test_file_filtering,  // Test file filtering with database rules
            debug_database_rules, // Debug database rules
            test_embedding,       // Added for testing
            // save or load file
            commands::save_scan_settings, // Save settings command
            commands::load_scan_settings, // Load settings command

            // config commands
            commands::get_excluded_folder,
            add_excluded_folder,       // Add excluded path to database
            remove_excluded_folder,    // Remove excluded path from database

            add_included_extension,  // Add included extension to database
            remove_included_extension, // Remove included extension from database
            add_included_path,
            add_excluded_path,
            add_included_folder,
            add_excluded_extension,
            add_excluded_filename,
            remove_included_path,
            remove_excluded_path,
            remove_included_folder,
            remove_excluded_extension,
            remove_excluded_filename,
            commands::get_included_extensions,
            commands::get_included_folders,
            commands::get_excluded_extensions,
            commands::get_included_paths,
            commands::get_excluded_paths,
            commands::get_excluded_filenames
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}
