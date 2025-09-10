use arrow_array::{
    types::Float32Type,
    FixedSizeListArray,
    Int32Array,
    RecordBatch,
    RecordBatchIterator,
    StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use lancedb::{
    connect,  // Simplified connect import
    index::{Index, scalar::FtsIndexBuilder},  // Correct FTS builder
};
use std::sync::Arc;
use tauri::command;

// Helper function (unchanged)
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

    let db = connect(
        database_path
            .to_str()
            .ok_or("Failed to convert path to string")?,
    )
    .execute()
    .await
    .map_err(|e| format!("LanceDB connection error: {}", e))?;

    const VECTOR_DIM: i32 = 384;

    // Helper closure: only create table if it doesn't already exist
    async fn ensure_table<F>(
        db: &lancedb::Connection,
        table_name: &str,
        schema: Arc<Schema>,
        batch: RecordBatch,
        build_indexes: F,
    ) -> Result<(), String>
    where
        F: FnOnce(&lancedb::Table) -> futures::future::BoxFuture<'_, Result<(), String>>,
    {
        match db.open_table(table_name).execute().await {
            Ok(table) => {
                // Table exists → maybe ensure indexes
                build_indexes(&table).await?;
            }
            Err(_) => {
                // Create table since it does not exist
                let batches = RecordBatchIterator::new(vec![Ok(batch)], schema.clone());
                db.create_table(table_name, Box::new(batches))
                    .execute()
                    .await
                    .map_err(|e| format!("Failed to create {} table: {}", table_name, e))?;

                let table = db.open_table(table_name).execute().await
                    .map_err(|e| format!("Failed to open {} table after creation: {}", table_name, e))?;

                build_indexes(&table).await?;
            }
        }
        Ok(())
    }

    // -------- files table --------
    let files_schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("extension", DataType::Utf8, false),
        Field::new("path", DataType::Utf8, false),
        Field::new("content", DataType::Utf8, true),
        Field::new(
            "vector",
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), VECTOR_DIM),
            true,
        ),
    ]));

    let files_batch = RecordBatch::try_new(
        files_schema.clone(),
        vec![
            Arc::new(Int32Array::from_iter_values(0..256)),
            Arc::new(StringArray::from_iter_values((0..256).map(|i| format!("file_{}.txt", i)))),
            Arc::new(StringArray::from_iter_values((0..256).map(|_| "txt".to_string()))),
            Arc::new(StringArray::from_iter_values((0..256).map(|i| format!("/path/to/file_{}", i)))),
            Arc::new(StringArray::from_iter((0..256).map(|_| Some("Sample content".to_string())))),
            Arc::new(FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                (0..256).map(|_| Some(vec![Some(1.0); VECTOR_DIM as usize])),
                VECTOR_DIM,
            )),
        ],
    ).map_err(|e| format!("Failed to create files batch: {}", e))?;

    ensure_table(&db, "files", files_schema.clone(), files_batch, |table| {
        Box::pin(async move {
            // Indexes (idempotent attempt – will skip if already built)
            table.create_index(&["name"], Index::FTS(FtsIndexBuilder::default()))
                .execute().await.ok();
            table.create_index(&["content"], Index::FTS(FtsIndexBuilder::default()))
                .execute().await.ok();
            Ok(())
        })
    }).await?;

    // -------- file_embeddings table --------
    let file_emb_schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("file_id", DataType::Int32, false),
        Field::new("chunk_index", DataType::Int32, false),
        Field::new("chunk_text", DataType::Utf8, true),
        Field::new(
            "content_vec",
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), VECTOR_DIM),
            false,
        ),
    ]));

    let file_emb_batch = RecordBatch::try_new(
        file_emb_schema.clone(),
        vec![
            Arc::new(Int32Array::from_iter_values(0..256)),
            Arc::new(Int32Array::from_iter_values(0..256)),
            Arc::new(Int32Array::from_iter_values((0..256).map(|_| 0))),
            Arc::new(StringArray::from_iter((0..256).map(|_| Some("Chunked content".to_string())))),
            Arc::new(FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                (0..256).map(|_| Some(vec![Some(0.5); VECTOR_DIM as usize])),
                VECTOR_DIM,
            )),
        ],
    ).map_err(|e| format!("Failed to create file_embeddings batch: {}", e))?;

    ensure_table(&db, "file_embeddings", file_emb_schema, file_emb_batch, |_| {
        Box::pin(async { Ok(()) }) // no indexes for now
    }).await?;

    // -------- folder table --------
    let folder_schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("folder_name", DataType::Utf8, false),
        Field::new("created_date", DataType::Utf8, false),
        Field::new(
            "folder_metadata_embed",
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), VECTOR_DIM),
            false,
        ),
    ]));

    let folder_batch = RecordBatch::try_new(
        folder_schema.clone(),
        vec![
            Arc::new(Int32Array::from_iter_values(0..1)),
            Arc::new(StringArray::from(vec!["ExampleFolder"])),
            Arc::new(StringArray::from(vec!["2025-09-09"])),
            Arc::new(FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                vec![Some(vec![Some(0.0); VECTOR_DIM as usize])],
                VECTOR_DIM,
            )),
        ],
    ).map_err(|e| format!("Failed to create folder batch: {}", e))?;

    ensure_table(&db, "folder", folder_schema, folder_batch, |_| {
        Box::pin(async { Ok(()) })
    }).await?;

    Ok(())
}
