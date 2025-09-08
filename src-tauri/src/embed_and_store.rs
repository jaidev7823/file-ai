use serde::Deserialize;
use rayon::prelude::*;
use std::sync::{Arc};
use std::sync::atomic::{AtomicUsize, Ordering};
use rayon::ThreadPoolBuilder;

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
}

pub fn normalize(v: Vec<f32>) -> Vec<f32> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 {
        v
    } else {
        v.iter().map(|x| x / norm).collect()
    }
}

// Synchronous embedding
pub fn get_embedding(text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    let res: EmbeddingResponse = client
        .post("http://localhost:11434/api/embeddings")
        .json(&serde_json::json!({
            "model": "nomic-embed-text",
            "prompt": text
        }))
        .send()?
        .json()?;
    Ok(res.embedding)
}

// Synchronous batch embeddings
pub fn get_batch_embeddings(texts: &[String]) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    let batch_size = 10;
    let mut all_embeddings = Vec::new();

    for batch in texts.chunks(batch_size) {
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

pub fn get_batch_embeddings_with_progress<F>(
    texts: &[String],
    progress_callback: F,
) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error + Send + Sync>>
where
    F: Fn(usize, usize) + Send + Sync + 'static,
{
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let total = texts.len();
    let counter = Arc::new(AtomicUsize::new(0));
    let callback = Arc::new(progress_callback);

    let pool = ThreadPoolBuilder::new()
        .num_threads(4) // 👈 limit concurrency (tune this number!)
        .build()?;

    let results = pool.install(|| {
        texts.par_iter()
            .map(|text| {
                let res: EmbeddingResponse = client
                    .post("http://localhost:11434/api/embeddings")
                    .json(&serde_json::json!({
                        "model": "nomic-embed-text",
                        "prompt": text
                    }))
                    .send()?
                    .json()?;

                let num_done = counter.fetch_add(1, Ordering::SeqCst) + 1;
                callback(num_done, total);

                Ok(res.embedding)
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error + Send + Sync>>>()
    });

    results
}