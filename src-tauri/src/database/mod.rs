// src-tauri/src/mod.rs
// No changes related to the error, just ensuring imports are correct.
// Removed `use crate::embed_and_store::get_embedding;`
use once_cell::sync::Lazy;
use rusqlite::ffi::sqlite3_auto_extension;
use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;

pub mod schema;
pub mod search;
pub mod seeder;

// getting app data dir for saving database file 
fn get_app_data_dir() -> Option<PathBuf> {
    if let Some(mut dir) = dirs::data_local_dir() {
        dir.push("FileAI"); // your app name
        Some(dir)
    } else {
        std::env::current_dir().ok()
    }
}

// setting db_commention with database
static DB_CONNECTION: Lazy<Mutex<Connection>> = Lazy::new(|| {
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute(
            sqlite_vec::sqlite3_vec_init as *const (),
        )));
    }

    let database_path = get_app_data_dir()
        .expect("Could not get app data directory")
        .join("database.db");

    std::fs::create_dir_all(database_path.parent().unwrap())
        .expect("Could not create app data directory");

    let conn = Connection::open(database_path).expect("Failed to open DB");
    Mutex::new(conn)
});

// initializing database creating datbaase file and adding sqlite-vec extension
pub fn init_database() -> Result<Connection> {
    // Register the vec0 module BEFORE doing anything
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute(
            sqlite_vec::sqlite3_vec_init as *const (),
        )));
    }

    // using get_app_data_dir for getting local folder and saving database.db file
    let database_path = get_app_data_dir()
        .expect("Could not get app data directory")
        .join("database.db");

    // creating database file with fs 
    std::fs::create_dir_all(database_path.parent().unwrap())
        .expect("Could not create app data directory");

    // opening the connection to database
    let conn = Connection::open(&database_path)?;

    // The extension should already be loaded via sqlite3_auto_extension
    // Testing sqlite-vec table if it is creating properlly or not
    match conn.execute(
        "CREATE TEMP TABLE test_vec USING vec0(embedding FLOAT[3])",
        [],
    ) {
        Ok(_) => {
            println!("vec0 extension is working");
            // Clean up the test table
            let _ = conn.execute("DROP TABLE test_vec", []);
        }
        Err(e) => {
            println!("Warning: vec0 extension may not be loaded properly: {}", e);
        }
    }

    // debuging avaiable file fun for sqlite vec for future operation 
    debug_print_file_vec_schema(&conn);
    debug_print_available_functions(&conn);

    println!("Connected to database at: {}", database_path.display());

    // Now run the migrations
    conn.execute_batch(&schema::create_all_sql())?;

    println!("Migrations executed successfully");

    Ok(conn)
}

pub fn get_connection() -> std::sync::MutexGuard<'static, Connection> {
    DB_CONNECTION.lock().expect("Failed to lock DB")
}
pub use search::{
    debug_print_available_functions, debug_print_file_vec_schema, hybrid_search_with_embedding,
    search_files_fts, SearchResult,
};
