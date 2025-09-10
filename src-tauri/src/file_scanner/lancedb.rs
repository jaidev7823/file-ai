use anyhow::anyhow;
use arrow_array::{
    types::Float32Type, FixedSizeListArray, Int32Array, RecordBatch, RecordBatchIterator,
    StringArray,
};
use lancedb::connect;
use lancedb::table::Table;
use std::path::Path;
use std::sync::Arc;
// use arrow_schema::{Field, DataType, Schema};
use super::types::FileContent;
use crate::database::lancedb_ops::get_app_data_dir;
use crate::embed_and_store::normalize;

// Assumed imports from your project
// use crate::database::lancedb_ops::get_app_data_dir;
// use crate::embed_and_store::normalize;
// use super::types::FileContent;

// Helper function to get a handle to the LanceDB tables
async fn get_lancedb_tables() -> anyhow::Result<(Table, Table)> {
    let database_path = get_app_data_dir()
        .ok_or_else(|| anyhow!("Could not get app data directory"))?
        .join("my-lancedb");

    let db = connect(database_path.to_str().unwrap()).execute().await?;

    let files_table = db.open_table("files").execute().await?;

    let file_emb_table = db.open_table("file_embeddings").execute().await?;

    Ok((files_table, file_emb_table))
}

// Corrected function to save to LanceDB's `files` table
pub async fn insert_file_metadata_lancedb(
    file: &FileContent,
    vector: Option<Vec<f32>>,
) -> anyhow::Result<i32> {
    const VECTOR_DIM: i32 = 384;
    let (files_table, _) = get_lancedb_tables().await?;

    let path_obj = Path::new(&file.path);
    let file_name = path_obj
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let extension = path_obj
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();

    // Generate the unique ID for the new record
    let id: i32 = rand::random();

    let vector_array: Arc<FixedSizeListArray> = if let Some(vec) = vector {
        let normalized_vec = normalize(vec);
        let optional_vec: Vec<Option<f32>> = normalized_vec.into_iter().map(Some).collect();
        Arc::new(
            FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                vec![Some(optional_vec)],
                VECTOR_DIM,
            ),
        )
    } else {
        let null_vec: Vec<Option<f32>> = vec![None; VECTOR_DIM as usize];
        Arc::new(
            FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                vec![Some(null_vec)],
                VECTOR_DIM,
            ),
        )
    };

    let content_string_array = Arc::new(StringArray::from_iter_values(vec![file.content.clone()]));

    let batch = RecordBatch::try_new(
        files_table.schema().await?.clone(),
        vec![
            Arc::new(Int32Array::from_iter_values(vec![id])),
            Arc::new(StringArray::from_iter_values(vec![file_name])),
            Arc::new(StringArray::from_iter_values(vec![extension])),
            Arc::new(StringArray::from_iter_values(vec![file.path.clone()])),
            content_string_array,
            vector_array,
        ],
    )?;

    let batches_to_add = RecordBatchIterator::new(vec![Ok(batch)], files_table.schema().await?.clone());
    files_table.add(batches_to_add).execute().await?;

    // Return the generated ID after successful insertion
    Ok(id)
}

// Corrected function to save to LanceDB's `file_embeddings` table
pub async fn insert_file_embedding_lancedb(
    file_id: i32,
    chunk_text: &str,
    vector: Vec<f32>,
) -> anyhow::Result<()> {
    const VECTOR_DIM: i32 = 384;
    let (_, file_emb_table) = get_lancedb_tables().await?;

    if vector.is_empty() {
        return Ok(());
    }

    let normalized_vec = normalize(vector);
    let optional_vec: Vec<Option<f32>> = normalized_vec.into_iter().map(Some).collect();

    let batch = RecordBatch::try_new(
        file_emb_table.schema().await?.clone(),
        vec![
            Arc::new(Int32Array::from_iter_values(vec![rand::random::<i32>()])),
            Arc::new(Int32Array::from_iter_values(vec![file_id])),
            Arc::new(Int32Array::from_iter_values(vec![0])),
            Arc::new(StringArray::from_iter_values(vec![chunk_text])),
            Arc::new(
                FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                    vec![Some(optional_vec)],
                    VECTOR_DIM,
                ),
            ),
        ],
    )?;

    // Use RecordBatchIterator to provide a valid reader to the add method
    let batches_to_add =
        RecordBatchIterator::new(vec![Ok(batch)], file_emb_table.schema().await?.clone());
    file_emb_table.add(batches_to_add).execute().await?;

    Ok(())
}
