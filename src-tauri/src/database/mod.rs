// src-tauri/src/mod.rs
// No changes related to the error, just ensuring imports are correct.
// Removed `use crate::embed_and_store::get_embedding;`
use once_cell::sync::Lazy;
use rusqlite::ffi::sqlite3_auto_extension;
use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;
use crate::test::{debug_print_available_functions, debug_print_file_vec_schema};

pub mod schema;
pub mod search;
pub mod seeder;
pub mod rules;

fn get_app_data_dir() -> Option<PathBuf> {
    if let Some(mut dir) = dirs::data_local_dir() {
        dir.push("FileAI"); // your app name
        Some(dir)
    } else {
        std::env::current_dir().ok()
    }
}

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

    let conn = Connection::open(&database_path).expect("Failed to open DB");

    println!("Connected to database at: {}", database_path.display());

    Mutex::new(conn)
});

pub fn init_database() -> Result<Connection> {
    // Register the vec0 module BEFORE doing anything
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

    let conn = Connection::open(&database_path)?;

    // The extension should already be loaded via sqlite3_auto_extension
    // Let's test if it's working by trying to create a simple vec0 table
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


pub fn initialize() -> Result<()> {
    let conn = get_connection();

    // vec0 extension test
    match conn.execute(
        "CREATE TEMP TABLE test_vec USING vec0(embedding FLOAT[3])",
        [],
    ) {
        Ok(_) => {
            println!("vec0 extension is working");
            let _ = conn.execute("DROP TABLE test_vec", []);
        }
        Err(e) => {
            println!("Warning: vec0 extension may not be loaded properly: {}", e);
        }
    }

    debug_print_file_vec_schema(&conn);
    debug_print_available_functions(&conn);

    conn.execute_batch(&schema::create_all_sql())?;
    println!("Migrations executed successfully");

    // Seed data
    seeder::seed_initial_data(&conn)?;
    println!("Database seeded successfully");

    Ok(())
}

pub use search::{
    hybrid_search_with_embedding,
    search_files_fts, SearchResult,
};