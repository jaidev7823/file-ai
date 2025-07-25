use crate::file_scanner::get_embedding;
use bytemuck::cast_slice;
use once_cell::sync::Lazy;
use rusqlite::ffi::sqlite3_auto_extension;
use rusqlite::params;
use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;

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
    match conn.execute("CREATE TEMP TABLE test_vec USING vec0(embedding FLOAT[3])", []) {
        Ok(_) => {
            println!("vec0 extension is working");
            // Clean up the test table
            let _ = conn.execute("DROP TABLE test_vec", []);
        },
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

pub fn search_similar_files(
    db: &Connection,
    query: &str,
    top_k: usize,
) -> Result<Vec<(String, f32)>, String> {
    let embedding = get_embedding(query).map_err(|e| e.to_string())?;

    if embedding.is_empty() {
        return Err("Query embedding is empty".into());
    }

    let vector_bytes: &[u8] = cast_slice(&embedding);

    // Try different function names that sqlite-vec might use
    let queries_to_try = [
        // Using vec_distance_cosine (lower is more similar, so we use negative for DESC order)
        "SELECT f.path, -vec_distance_cosine(fv.content_vec, ?) AS score
         FROM file_vec AS fv
         JOIN file_vec_map AS m ON m.vec_rowid = fv.rowid
         JOIN files AS f ON f.id = m.file_id
         ORDER BY score DESC
         LIMIT ?;",
        
        // Using vec_distance_l2 (lower is more similar, so we use negative for DESC order)
        "SELECT f.path, -vec_distance_l2(fv.content_vec, ?) AS score
         FROM file_vec AS fv
         JOIN file_vec_map AS m ON m.vec_rowid = fv.rowid
         JOIN files AS f ON f.id = m.file_id
         ORDER BY score DESC
         LIMIT ?;",
        
        // Using vec0_distance_cosine (alternative naming)
        "SELECT f.path, -vec0_distance_cosine(fv.content_vec, ?) AS score
         FROM file_vec AS fv
         JOIN file_vec_map AS m ON m.vec_rowid = fv.rowid
         JOIN files AS f ON f.id = m.file_id
         ORDER BY score DESC
         LIMIT ?;",
        
        // If sqlite-vec supports DOT_PRODUCT
        "SELECT f.path, DOT_PRODUCT(fv.content_vec, ?) AS score
         FROM file_vec AS fv
         JOIN file_vec_map AS m ON m.vec_rowid = fv.rowid
         JOIN files AS f ON f.id = m.file_id
         ORDER BY score DESC
         LIMIT ?;",
    ];

    for (i, query_sql) in queries_to_try.iter().enumerate() {
        match db.prepare(query_sql) {
            Ok(mut stmt) => {
                println!("Using query variant {}: Success", i + 1);
                let results = stmt
                    .query_map(params![vector_bytes, top_k as i64], |row| {
                        let path: String = row.get(0)?;
                        let score: f32 = row.get(1)?;
                        Ok((path, score))
                    })
                    .map_err(|e| e.to_string())?
                    .filter_map(Result::ok)
                    .collect();

                return Ok(results);
            }
            Err(e) => {
                println!("Query variant {} failed: {}", i + 1, e);
                continue;
            }
        }
    }

    Err("No working vector similarity function found. Make sure sqlite-vec extension is properly loaded.".into())
}

pub fn debug_print_file_vec_schema(db: &Connection) {
    let mut stmt = db
        .prepare("PRAGMA table_info(file_vec);")
        .expect("Failed to prepare PRAGMA query");

    let columns = stmt
        .query_map([], |row| {
            let cid: i32 = row.get(0)?;
            let name: String = row.get(1)?;
            let dtype: String = row.get(2)?;
            Ok((cid, name, dtype))
        })
        .expect("Could not query map");

    println!("Schema for file_vec:");
    for column in columns {
        println!("{:?}", column.expect("Error retrieving column"));
    }
}

pub fn debug_print_available_functions(db: &Connection) {
    // Try to list available functions
    println!("Testing vector functions:");
    
    let test_functions = [
        "vec_distance_cosine",
        "vec_distance_l2", 
        "vec0_distance_cosine",
        "vec0_distance_l2",
        "DOT_PRODUCT",
        "vec_dot",
        "vec0_dot",
    ];
    
    for func_name in &test_functions {
        let test_query = format!("SELECT {}(?, ?)", func_name);
        match db.prepare(&test_query) {
            Ok(_) => println!("✓ {} is available", func_name),
            Err(_) => println!("✗ {} is not available", func_name),
        }
    }
    
    // Try to get sqlite-vec version if possible
    match db.prepare("SELECT vec_version()") {
        Ok(mut stmt) => {
            if let Ok(version) = stmt.query_row([], |row| row.get::<_, String>(0)) {
                println!("sqlite-vec version: {}", version);
            }
        }
        Err(_) => {
            match db.prepare("SELECT vec0_version()") {
                Ok(mut stmt) => {
                    if let Ok(version) = stmt.query_row([], |row| row.get::<_, String>(0)) {
                        println!("sqlite-vec version: {}", version);
                    }
                }
                Err(_) => println!("Could not determine sqlite-vec version"),
            }
        }
    }
}