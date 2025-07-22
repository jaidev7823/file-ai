use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::error::Error;
use reqwest::blocking::Client;

#[derive(Debug, Serialize)]
struct EmbeddingRequest<'a> {
    model: &'a str,
    prompt: &'a str,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
}

/// Get embeddings from Ollama
fn get_embedding(text: &str) -> Result<Vec<f32>, Box<dyn Error>> {
    let client = Client::new();
    let req_body = EmbeddingRequest {
        model: "nomic-embed-text", // ✅ best for general-purpose text
        prompt: text,
    };
    let res: EmbeddingResponse = client
        .post("http://localhost:11434/api/embeddings")
        .json(&req_body)
        .send()?
        .json()?;

    Ok(res.embedding)
}

/// Embed a file's content and store it in file_vec + file_vec_map
pub fn embed_file_content(
    conn: &mut Connection,
    file_id: i32,
) -> Result<(), Box<dyn Error>> {
    // 1. Get file content
    let mut stmt = conn.prepare("SELECT content FROM files WHERE id = ?1")?;
    let content: String = stmt.query_row(params![file_id], |row| row.get(0))?;

    // 2. Call Ollama for embedding
    let vector = get_embedding(&content)?;

    // 3. Insert into file_vec (sqlite_vec)
    conn.execute("INSERT INTO file_vec(content_vec) VALUES(?1)", params![vector])?;

    // 4. Get last inserted vec_rowid
    let vec_rowid = conn.last_insert_rowid();

    // 5. Insert mapping
    conn.execute(
        "INSERT INTO file_vec_map(vec_rowid, file_id) VALUES(?1, ?2)",
        params![vec_rowid, file_id],
    )?;

    println!("✅ Embedded and stored vector for file_id {}", file_id);

    Ok(())
}
