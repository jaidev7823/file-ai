// src-tauri/src/services/file_vector_service.rs
use crate::database::lancedb_ops::{FileVector, insert_file_vectors, vector_search, delete_file_vectors, batch_upsert_vectors};
use crate::database::get_connection;
use rusqlite::{params, Result as SqlResult};
use sha2::{Sha256, Digest};

pub struct FileVectorService;

impl FileVectorService {
    /// Store or update file with its vector embedding
    pub async fn store_file_with_vector(
        file_id: i64,
        file_path: &str,
        file_name: &str,
        content: &str,
        embedding: Vec<f32>,
        chunk_index: Option<i32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Calculate content hash
        let content_hash = Self::calculate_content_hash(content);
        
        // Update SQLite metadata
        {
            let conn = get_connection();
            conn.execute(
                "UPDATE files SET content_hash = ?1, vector_indexed = 1, updated_at = ?2 WHERE id = ?3",
                params![content_hash, chrono::Utc::now().to_rfc3339(), file_id],
            )?;
        }
        
        // Store vector in LanceDB
        let file_vector = FileVector {
            file_id,
            file_path: file_path.to_string(),
            file_name: file_name.to_string(),
            content_hash: content_hash.clone(),
            embedding,
            chunk_index,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        
        insert_file_vectors(vec![file_vector]).await?;
        
        Ok(())
    }
    
    /// Batch process multiple files for better performance
    pub async fn batch_store_files_with_vectors(
        files_data: Vec<(i64, String, String, String, Vec<f32>, Option<i32>)>, // (id, path, name, content, embedding, chunk_index)
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut file_vectors = Vec::new();
        let mut sqlite_updates = Vec::new();
        
        for (file_id, file_path, file_name, content, embedding, chunk_index) in files_data {
            let content_hash = Self::calculate_content_hash(&content);
            
            file_vectors.push(FileVector {
                file_id,
                file_path,
                file_name,
                content_hash: content_hash.clone(),
                embedding,
                chunk_index,
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            });
            
            sqlite_updates.push((content_hash, file_id));
        }
        
        // Batch update SQLite
        {
            let conn = get_connection();
            let tx = conn.unchecked_transaction()?;
            
            for (content_hash, file_id) in sqlite_updates {
                tx.execute(
                    "UPDATE files SET content_hash = ?1, vector_indexed = 1, updated_at = ?2 WHERE id = ?3",
                    params![content_hash, chrono::Utc::now().to_rfc3339(), file_id],
                )?;
            }
            
            tx.commit()?;
        }
        
        // Batch insert vectors
        batch_upsert_vectors(file_vectors).await?;
        
        Ok(())
    }
    
    /// Semantic search combining SQLite metadata with LanceDB vectors
    pub async fn semantic_search(
        query_embedding: Vec<f32>,
        limit: usize,
        file_path_filter: Option<&str>,
        extension_filter: Option<&str>,
        min_score: Option<f64>,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        // Search vectors in LanceDB
        let lance_results = vector_search(query_embedding, limit * 2, file_path_filter).await?;
        
        if lance_results.is_empty() {
            return Ok(vec![]);
        }
        
        // Get file IDs from LanceDB results
        let file_ids: Vec<i64> = lance_results.iter().map(|r| r.file_id).collect();
        
        // Get metadata from SQLite
        let mut results = Vec::new();
        {
            let conn = get_connection();
            let id_list = file_ids.iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",");
            
            let mut query = format!(
                "SELECT id, name, path, extension, score, file_size, category, updated_at 
                 FROM files WHERE id IN ({})",
                id_list
            );
            
            // Add extension filter
            if let Some(ext) = extension_filter {
                query.push_str(&format!(" AND extension = '{}'", ext));
            }
            
            // Add score filter
            if let Some(min_score_val) = min_score {
                query.push_str(&format!(" AND score >= {}", min_score_val));
            }
            
            let mut stmt = conn.prepare(&query)?;
            let rows = stmt.query_map([], |row| {
                let file_id: i64 = row.get(0)?;
                let similarity = lance_results.iter()
                    .find(|r| r.file_id == file_id)
                    .map(|_| 0.95) // LanceDB doesn't return similarity score directly
                    .unwrap_or(0.0);
                
                Ok(SearchResult {
                    file_id,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    extension: row.get(3)?,
                    score: row.get(4)?,
                    file_size: row.get(5)?,
                    category: row.get(6)?,
                    updated_at: row.get(7)?,
                    similarity_score: similarity,
                })
            })?;
            
            for row in rows {
                results.push(row?);
            }
        }
        
        // Sort by similarity and limit
        results.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());
        results.truncate(limit);
        
        Ok(results)
    }
    
    /// Delete vectors for specific files
    pub async fn delete_file_vectors_by_ids(file_ids: &[i64]) -> Result<(), Box<dyn std::error::Error>> {
        // Update SQLite
        {
            let conn = get_connection();
            let tx = conn.unchecked_transaction()?;
            
            for file_id in file_ids {
                tx.execute(
                    "UPDATE files SET vector_indexed = 0, content_hash = NULL WHERE id = ?1",
                    params![file_id],
                )?;
            }
            
            tx.commit()?;
        }
        
        // Delete from LanceDB
        delete_file_vectors(file_ids).await?;
        
        Ok(())
    }
    
    /// Check which files need re-indexing (content changed)
    pub async fn find_files_needing_reindex() -> SqlResult<Vec<(i64, String, String)>> {
        let conn = get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, path, content FROM files 
             WHERE vector_indexed = 0 OR content_hash IS NULL 
             OR content_hash != ?1"
        )?;
        
        let mut results = Vec::new();
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let path: String = row.get(1)?;
            let content: String = row.get(2)?;
            let current_hash = Self::calculate_content_hash(&content);
            
            // Only include if hash doesn't match or is null
            Ok((id, path, current_hash))
        })?;
        
        for row in rows {
            let (id, path, hash) = row?;
            
            // Check if stored hash matches current
            let stored_hash: Option<String> = conn.query_row(
                "SELECT content_hash FROM files WHERE id = ?1",
                params![id],
                |row| row.get(0),
            ).unwrap_or(None);
            
            if stored_hash.is_none() || stored_hash.unwrap() != hash {
                results.push((id, path, hash));
            }
        }
        
        Ok(results)
    }
    
    fn calculate_content_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[derive(Debug)]
pub struct SearchResult {
    pub file_id: i64,
    pub name: String,
    pub path: String,
    pub extension: String,
    pub score: f64,
    pub file_size: Option<i64>,
    pub category: Option<String>,
    pub updated_at: String,
    pub similarity_score: f64,
}