use anyhow::anyhow;
use arrow_array::{
    Array, Float32Array, FixedSizeListArray, Int32Array, RecordBatch, RecordBatchIterator,
    StringArray,
};
use arrow::array::ArrayData;
use arrow_schema::{Field, Schema, DataType};
use lancedb::connect;
use lancedb::table::Table;
use std::path::Path;
use std::sync::Arc;

use super::types::FileContent;
use crate::database::lancedb_ops::get_app_data_dir;
use crate::embed_and_store::normalize;


/// Opens a connection to LanceDB and the required tables.
/// This should be called ONCE, outside of any loops.
pub async fn get_lancedb_tables() -> anyhow::Result<(Table, Table)> {
    let database_path = get_app_data_dir()
        .ok_or_else(|| anyhow!("Could not get app data directory"))?
        .join("my-lancedb");

    let db = connect(database_path.to_str().unwrap()).execute().await?;
    let files_table = db.open_table("files").execute().await?;
    let file_emb_table = db.open_table("file_embeddings").execute().await?;

    Ok((files_table, file_emb_table))
}

/// Inserts a batch of file metadata records into the `files` table.
pub async fn insert_file_metadata_batch(
    files_table: &Table,
    files: &[FileContent],
    vectors: Vec<Option<Vec<f32>>>,
) -> anyhow::Result<Vec<i32>> {
    const VECTOR_DIM: i32 = 768;
    if files.is_empty() {
        return Ok(Vec::new());
    }

    let len = files.len();
    let mut ids = Vec::with_capacity(len);
    let mut names = Vec::with_capacity(len);
    let mut exts = Vec::with_capacity(len);
    let mut paths = Vec::with_capacity(len);
    let mut contents = Vec::with_capacity(len);
    let mut all_vectors_flat = Vec::with_capacity(len * VECTOR_DIM as usize);

    for (i, file) in files.iter().enumerate() {
        let path_obj = Path::new(&file.path);
        ids.push(rand::random::<i32>());
        names.push(path_obj.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string());
        exts.push(path_obj.extension().and_then(|e| e.to_str()).unwrap_or("").to_string());
        paths.push(file.path.clone());
        contents.push(file.content.clone());

        let vector = vectors.get(i).and_then(|v| v.as_ref());
        if let Some(vec) = vector {
            all_vectors_flat.extend(normalize(vec.clone()));
        } else {
            all_vectors_flat.extend(vec![0.0; VECTOR_DIM as usize]);
        }
    }

    let schema = files_table.schema().await?;

    // Manually construct FixedSizeListArray for compatibility
    let list_data_type = DataType::FixedSizeList(
        Arc::new(Field::new("item", DataType::Float32, true)),
        VECTOR_DIM,
    );
    let values_array_data = Float32Array::from(all_vectors_flat).into_data();
    let list_array_data = ArrayData::builder(list_data_type)
        .len(len) // Number of lists/vectors
        .add_child_data(values_array_data)
        .build()?;
    let vector_array = Arc::new(FixedSizeListArray::from(list_array_data));

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(Int32Array::from(ids.clone())),
            Arc::new(StringArray::from(names)),
            Arc::new(StringArray::from(exts)),
            Arc::new(StringArray::from(paths)),
            Arc::new(StringArray::from(contents)),
            vector_array,
        ],
    )?;

    let batches_to_add = RecordBatchIterator::new(vec![Ok(batch)], schema);
    files_table.add(batches_to_add).execute().await?;
    Ok(ids)
}

/// Inserts a batch of file embedding chunks into the `file_embeddings` table.
pub async fn insert_file_embedding_batch(
    file_emb_table: &Table,
    embedding_data: &[(i32, String, Vec<f32>)], // (file_id, chunk_text, vector)
) -> anyhow::Result<()> {
    const VECTOR_DIM: i32 = 768;
    let clean_data: Vec<_> = embedding_data.iter().filter(|d| !d.2.is_empty()).collect();

    if clean_data.is_empty() {
        return Ok(());
    }

    let len = clean_data.len();
    let mut ids = Vec::with_capacity(len);
    let mut file_ids = Vec::with_capacity(len);
    let mut chunk_nos = Vec::with_capacity(len);
    let mut chunk_texts = Vec::with_capacity(len);
    let mut all_vectors_flat = Vec::with_capacity(len * VECTOR_DIM as usize);

    for (file_id, chunk_text, vector) in clean_data {
        ids.push(rand::random::<i32>());
        file_ids.push(*file_id);
        chunk_nos.push(0); // Placeholder for chunk number
        chunk_texts.push(chunk_text.clone());
        all_vectors_flat.extend(normalize(vector.clone()));
    }

    let schema = file_emb_table.schema().await?;

    // Manually construct FixedSizeListArray
    let list_data_type = DataType::FixedSizeList(
        Arc::new(Field::new("item", DataType::Float32, true)),
        VECTOR_DIM,
    );
    let values_array_data = Float32Array::from(all_vectors_flat).into_data();
    let list_array_data = ArrayData::builder(list_data_type)
        .len(len)
        .add_child_data(values_array_data)
        .build()?;
    let vector_array = Arc::new(FixedSizeListArray::from(list_array_data));

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(Int32Array::from(ids)),
            Arc::new(Int32Array::from(file_ids)),
            Arc::new(Int32Array::from(chunk_nos)),
            Arc::new(StringArray::from(chunk_texts)),
            vector_array,
        ],
    )?;

    let batches_to_add = RecordBatchIterator::new(vec![Ok(batch)], schema);
    file_emb_table.add(batches_to_add).execute().await?;
    Ok(())
}


// --- DEPRECATED ---
// The following functions are inefficient and should be replaced with their `_batch` counterparts.
// They are kept for reference and to avoid breaking the build immediately.

// Corrected function to save to LanceDB's `files` table
pub async fn insert_file_metadata_lancedb(
    files_table: &Table,
    file: &FileContent,
    vector: Option<Vec<f32>>,
) -> anyhow::Result<i32> {
    const VECTOR_DIM: i32 = 768;

    let path_obj = Path::new(&file.path);
    let file_name = path_obj.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
    let extension = path_obj.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
    let id: i32 = rand::random();

    let vector_values = if let Some(vec) = vector {
        normalize(vec)
    } else {
        vec![0.0; VECTOR_DIM as usize]
    };

    let schema = files_table.schema().await?;

    // Manually construct FixedSizeListArray
    let list_data_type = DataType::FixedSizeList(
        Arc::new(Field::new("item", DataType::Float32, true)),
        VECTOR_DIM,
    );
    let values_array_data = Float32Array::from(vector_values).into_data();
    let list_array_data = ArrayData::builder(list_data_type)
        .len(1) // Only one list
        .add_child_data(values_array_data)
        .build()?;
    let vector_array = Arc::new(FixedSizeListArray::from(list_array_data));

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(Int32Array::from(vec![id])),
            Arc::new(StringArray::from(vec![file_name])),
            Arc::new(StringArray::from(vec![extension])),
            Arc::new(StringArray::from(vec![file.path.clone()])),
            Arc::new(StringArray::from(vec![file.content.clone()])),
            vector_array,
        ],
    )?;

    let batches_to_add = RecordBatchIterator::new(vec![Ok(batch)], schema);
    files_table.add(batches_to_add).execute().await?;
    Ok(id)
}

// Corrected function to save to LanceDB's `file_embeddings` table
pub async fn insert_file_embedding_lancedb(
    file_emb_table: &Table,
    file_id: i32,
    chunk_text: &str,
    vector: Vec<f32>,
) -> anyhow::Result<()> {
    const VECTOR_DIM: i32 = 768;

    if vector.is_empty() {
        return Ok(());
    }

    let normalized_vec = normalize(vector);
    
    let schema = file_emb_table.schema().await?;

    // Manually construct FixedSizeListArray
    let list_data_type = DataType::FixedSizeList(
        Arc::new(Field::new("item", DataType::Float32, true)),
        VECTOR_DIM,
    );
    let values_array_data = Float32Array::from(normalized_vec).into_data();
    let list_array_data = ArrayData::builder(list_data_type)
        .len(1) // Only one list
        .add_child_data(values_array_data)
        .build()?;
    let vector_array = Arc::new(FixedSizeListArray::from(list_array_data));

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(Int32Array::from(vec![rand::random::<i32>()])),
            Arc::new(Int32Array::from(vec![file_id])),
            Arc::new(Int32Array::from(vec![0])), // Chunk number
            Arc::new(StringArray::from(vec![chunk_text.to_string()])),
            vector_array,
        ],
    )?;

    let batches_to_add = RecordBatchIterator::new(vec![Ok(batch)], schema);
    file_emb_table.add(batches_to_add).execute().await?;
    Ok(())
}