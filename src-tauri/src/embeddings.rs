use ollama_rs::{Ollama, generation::embeddings::request::EmbeddingsRequest};

#[command]
async fn embed_text(text: String) -> Result<Vec<f64>, String> {
    let ollama = Ollama::default();
    
    let request = EmbeddingsRequest::new(
        "nomic-embed-text".to_string(), 
        text
    );
    
    match ollama.generate_embeddings(request).await {
        Ok(response) => Ok(response.embeddings[0].clone()),
        Err(e) => Err(format!("Embedding error: {}", e))
    }
}

#[command]
async fn embed_file_content(file_path: String, content: String) -> Result<Vec<f64>, String> {
    // Combine file path + content for better context
    let combined_text = format!("{} {}", file_path, content);
    embed_text(combined_text).await
}