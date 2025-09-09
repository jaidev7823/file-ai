use arrow_array::{
    types::Float32Type, // Add this import to fix Float32Type error
    FixedSizeListArray,
    Int32Array,
    RecordBatch,
    RecordBatchIterator,
};
use arrow_schema::{DataType, Field, Schema};

use std::sync::Arc;
use tauri::command;

// Helper function to get app data path (same as yours)
pub fn get_app_data_dir() -> Option<std::path::PathBuf> {
    if let Some(mut dir) = dirs::data_local_dir() {
        dir.push("FileAI");
        Some(dir)
    } else {
        std::env::current_dir().ok()
    }
}

#[command]
pub async fn create_local_lancedb() -> Result<(), String> {
    let database_path = get_app_data_dir()
        .ok_or("Could not get app data directory")?
        .join("my-lancedb");

    let db = lancedb::connect(
        database_path
            .to_str()
            .ok_or("Failed to convert path to string")?,
    )
    .execute()
    .await
    .map_err(|e| format!("LanceDB connection error: {}", e))?;
    let vector_dim: usize = 128;

    // --- Create 'files' table ---
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 128),
            true,
        ),
    ]));

    // Create a RecordBatch stream with sample data
    let batches = RecordBatchIterator::new(
        vec![RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int32Array::from_iter_values(0..256)),
                Arc::new(
                    FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        (0..256).map(|_| Some(vec![Some(1.0); 128])),
                        128,
                    ),
                ),
            ],
        )
        .map_err(|e| format!("Failed to create RecordBatch: {}", e))?]
        .into_iter()
        .map(Ok),
        schema.clone(),
    );

    db.create_table("files", Box::new(batches))
        .execute()
        .await
        .map_err(|e| format!("Failed to create table: {}", e))?;
    // --- Create 'folder' table ---

    let folder_schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("folder_name", DataType::Utf8, false),
        Field::new("created_date", DataType::Utf8, false),
        Field::new(
            "folder_metadata_embed",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                vector_dim as i32, // Cast to i32 for schema definition
            ),
            false,
        ),
    ]));

    let folder_batch = RecordBatch::try_new(
        folder_schema.clone(),
        vec![
            Arc::new(Int32Array::from_iter_values(0..1)),
            Arc::new(arrow_array::StringArray::from(vec!["ExampleFolder"])),
            Arc::new(arrow_array::StringArray::from(vec!["2025-09-09"])),
            Arc::new(
                FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                    vec![Some(vec![Some(0.0); vector_dim])], // vector_dim is now usize
                    vector_dim as i32, // Cast to i32 for the list size parameter
                ),
            ),
        ],
    )
    .map_err(|e| format!("Failed to create folder batch: {}", e))?;

    // Create RecordBatchIterator as LanceDB expects RecordBatchReader
    let folder_batches = RecordBatchIterator::new(vec![Ok(folder_batch)], folder_schema.clone());

    // LanceDB expects RecordBatchReader, so Box the iterator
    db.create_table("folder", Box::new(folder_batches))
        .execute()
        .await
        .map_err(|e| format!("Create folder table failed: {}", e))?;
    Ok(())
}
