// src-tauri/src/embed_and_store.rs
// Removed reqwest::Error as ReqwestError, as we use blocking client.
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
}

// Removed BatchEmbeddingResponse as it's not used with the new synchronous batch approach.

pub fn normalize(v: Vec<f32>) -> Vec<f32> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 {
        v
    } else {
        v.iter().map(|x| x / norm).collect()
    }
}

// This is now the synchronous get_embedding function
pub fn get_embedding(text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new(); // Use blocking client
    let res: EmbeddingResponse = client
        .post("http://localhost:11434/api/embeddings")
        .json(&serde_json::json!({
            "model": "nomic-embed-text",
            "prompt": text
        }))
        .send()? // No .await needed
        .json()?; // No .await needed
    Ok(res.embedding)
}

// Synchronous version for batch embeddings
pub fn get_batch_embeddings_sync(
    texts: &[String],
) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    let batch_size = 10;
    let mut all_embeddings = Vec::new();

    for batch in texts.chunks(batch_size) {
        // Iterate sequentially within the batch.
        // For true blocking parallelism, you'd use `rayon` or `std::thread::spawn` here.
        // Given this runs within `spawn_blocking`, blocking is acceptable.
        for text in batch {
            let res: EmbeddingResponse = client
                .post("http://localhost:11434/api/embeddings")
                .json(&serde_json::json!({
                    "model": "nomic-embed-text",
                    "prompt": text
                }))
                .send()?
                .json()?;
            all_embeddings.push(res.embedding);
        }
    }

    Ok(all_embeddings)
}

// Synchronous version for batch embeddings with progress callback
// here this where is generic parameter - which is type of progress callback 
pub fn get_batch_embeddings_with_progress<F>(
    texts: &[String],
    mut progress_callback: F,
) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>>
where
    F: FnMut(usize, usize),
{
    // this is for sending requiest to url
    let client = reqwest::blocking::Client::new();
    // batch size of emebedding 
    let batch_size = 10;
    // new variable for saving embedding
    let mut all_embeddings = Vec::new();
    // checking length of text we got from scanner fn
    let total = texts.len();

    // running a loop on text chunk with 10 batch size and enumerating it giving each of them id
    for (batch_idx, batch) in texts.chunks(batch_size).enumerate() {
        // runing one more loop on batch making them iterable and again making in enumerate
        for (item_idx, text) in batch.iter().enumerate() {
            // creating current variable multiplying batch_idx with batch_size adding item_idz and adding 1
            let current = batch_idx * batch_size + item_idx + 1;
            // calling progress callback for sending emit a data
            progress_callback(current, total);

            // creating response variable EmbeddingResponse is struct who except vector 
            let res: EmbeddingResponse = client
                .post("http://localhost:11434/api/embeddings")
                .json(&serde_json::json!({
                    "model": "nomic-embed-text",
                    "prompt": text
                }))
                .send()?
                .json()?;
            all_embeddings.push(res.embedding);
        }
    }

    Ok(all_embeddings)
}

// Wrapper function for backward compatibility
pub fn get_batch_embeddings(texts: &[String]) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
    get_batch_embeddings_sync(texts)
}
