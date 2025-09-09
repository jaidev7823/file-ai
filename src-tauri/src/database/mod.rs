// src-tauri/src/mod.rs
// No changes related to the error, just ensuring imports are correct.
// Removed `use crate::embed_and_store::get_embedding;`
use once_cell::sync::Lazy;
use rusqlite::ffi::sqlite3_auto_extension;
use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;
// use crate::test::{debug_print_available_functions, debug_print_file_vec_schema};
use rusqlite::params;

pub mod rules;
pub mod schema;
pub mod search;
pub mod seeder;
pub mod lancedb_ops;
use crate::database::lancedb_ops::get_app_data_dir;

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

    // debug_print_file_vec_schema(&conn);
    // debug_print_available_functions(&conn);

    conn.execute_batch(&schema::create_all_sql())?;
    println!("Migrations executed successfully");

    // Seed data
    seeder::seed_initial_data(&conn)?;
    println!("Database seeded successfully");

    Ok(())
}

/// Calculates and updates the score for each folder based on the average score of its files.
pub fn update_folder_scores(conn: &Connection) -> Result<()> {
    println!("Starting to update folder scores...");

    // Get all folder IDs and paths
    let mut stmt = conn.prepare("SELECT id, path FROM folders")?;
    let folder_iter = stmt.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut folders_to_update = Vec::new();
    for folder_result in folder_iter {
        let (folder_id, folder_path) = folder_result?;
        let like_pattern = format!("{}%", folder_path);

        // Calculate the average score of files within that folder path
        // Calculate the average score of files within that folder path
        let avg_score: f64 = conn.query_row(
            "SELECT AVG(score) FROM files WHERE path LIKE ?1",
            [like_pattern],
            |row| row.get(0).or(Ok(0.0)), // If no files, avg is NULL, so default to 0.0
        )?;

        // Clamp between 0.0 and 10.0 and round to 1 decimal place
        let clamped = avg_score.max(0.0f64).min(10.0f64);
        let rounded = (clamped * 10.0).round() / 10.0;

        folders_to_update.push((folder_id, rounded));
    }

    // Update the scores in a single transaction
    let tx = conn.unchecked_transaction()?;
    for (folder_id, score) in folders_to_update {
        tx.execute(
            "UPDATE folders SET score = ?1 WHERE id = ?2",
            params![score, folder_id],
        )?;
    }
    tx.commit()?;

    println!("Successfully updated scores for all folders.");
    Ok(())
}

pub use search::{perform_file_search, SearchResult};
