use reqwest::{blocking::Client, Error as ReqwestError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
}

#[derive(Debug, Deserialize)]
struct BatchEmbeddingResponse {
    embeddings: Vec<Vec<f32>>,
}

// Synchronous embedding for one string
pub fn get_embedding(text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let client = Client::new();
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

// Async version: multiple texts at once
pub async fn get_batch_embeddings(texts: &[String]) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let batch_size = 10;
    let mut all_embeddings = Vec::new();

    for batch in texts.chunks(batch_size) {
        let responses = futures::future::join_all(batch.iter().map(|text| {
            let client = client.clone();
            let text = text.clone();
            async move {
                let res: EmbeddingResponse = client
                    .post("http://localhost:11434/api/embeddings")
                    .json(&serde_json::json!({
                        "model": "nomic-embed-text",
                        "prompt": text
                    }))
                    .send()
                    .await?
                    .json()
                    .await?;
                Ok::<Vec<f32>, reqwest::Error>(res.embedding)
            }
        }))
        .await;

        for response in responses {
            match response {
                Ok(embedding) => all_embeddings.push(embedding),
                Err(e) => return Err(Box::new(e)),
            }
        }
    }

    Ok(all_embeddings)
}
