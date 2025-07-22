use rusqlite::{Connection, Result};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use std::path::PathBuf;
use sqlite_vec::sqlite3_vec_init;
use rusqlite::ffi::sqlite3_auto_extension;

pub mod schema;

fn get_app_data_dir() -> Option<PathBuf> {
    if let Some(mut dir) = dirs::data_local_dir() {
        dir.push("FileAI"); // your app name
        Some(dir)
    } else {
        std::env::current_dir().ok()
    }
}

static DB_CONNECTION: Lazy<Mutex<Connection>> = Lazy::new(|| {
    let database_path = get_app_data_dir()
        .expect("Could not get app data directory")
        .join("database.db");

    std::fs::create_dir_all(database_path.parent().unwrap())
        .expect("Could not create app data directory");

    let conn = Connection::open(database_path).expect("Failed to open DB");
    Mutex::new(conn)
});

pub fn init_database() -> Result<Connection> {
    // Register the vec0 module BEFORE doing anything
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute(
            sqlite3_vec_init as *const (),
        )));
    }

    let database_path = get_app_data_dir()
        .expect("Could not get app data directory")
        .join("database.db");

    std::fs::create_dir_all(database_path.parent().unwrap())
        .expect("Could not create app data directory");

    let conn = Connection::open(&database_path)?;

    println!("Connected to database at: {}", database_path.display());

    // Now run the migrations
    conn.execute_batch(&schema::create_all_sql())?;

    println!("Migrations executed successfully");

    Ok(conn)
}


pub fn get_connection() -> std::sync::MutexGuard<'static, Connection> {
    DB_CONNECTION.lock().expect("Failed to lock DB")
}
