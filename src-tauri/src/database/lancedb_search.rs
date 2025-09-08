// src-tauri/src/lancedb_search.rs
use lancedb::connection::{ConnectOptions, Connection};
use lancedb::table::{Table, NewTableBuilder};
use lancedb::error::Result;
use arrow_schema::{Schema, Field, DataType};
use std::sync::Arc;
use super::get_app_data_dir; // Reuse the function from mod.rs

pub async fn initialize_lancedb() -> Result<Connection> {
    // Get the app data directory (same as used for database.db)
    let app_data_dir = get_app_data_dir()
        .expect("Could not get app data directory");
    
    // Set the LanceDB database path (e.g., FileAI/vector.lancedb)
    let db_path = app_data_dir.join("vector.lancedb");
    
    // Connect to LanceDB with async options
    let connect_options = ConnectOptions::new().with_create_dir(true); // Create directory if it doesn't exist
    let db = lancedb::connect(db_path.to_str().expect("Invalid DB path"))
        .with_options(connect_options)
        .execute()
        .await?;

    // Define the schema for the vector table
    let schema = Arc::new(Schema::new(vec![
        Field::new("file_id", DataType::Int64, false), // Matches SQLite file_id
        Field::new("path", DataType::Utf8, false),     // File path
        Field::new("embedding", DataType::FixedSizeList(
            Arc::new(Field::new("item", DataType::Float32, false)),
            384 // Adjust dimension based on your embedding model (e.g., 384 for common models like all-MiniLM-L6-v2)
        ), false),
        Field::new("score", DataType::Float64, true),  // Optional score, nullable
    ]));

    // Create or open the table
    let table_name = "file_vectors";
    let table = NewTableBuilder::new(table_name)
        .schema(schema)
        .create_if_not_exists(true)
        .execute(&db)
        .await?;

    println!("LanceDB initialized with table '{}' at: {}", table_name, db_path.display());
    Ok(db)
}