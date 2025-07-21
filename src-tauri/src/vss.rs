use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, Statement};

pub async fn insert_embedding(
    db: &DatabaseConnection,
    file_id: i32,
    embedding: &[f32],
) -> Result<(), DbErr> {
    // Convert embedding to blob format for sqlite-vec
    let embedding_blob = bytemuck::cast_slice::<f32, u8>(embedding);
    
    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO file_embeddings (file_id, embedding) VALUES (?, ?)",
        [file_id.into(), embedding_blob.into()],
    );

    db.execute(stmt).await?;
        
    Ok(())
}

pub async fn search_similar_files(
    db: &DatabaseConnection,
    query_embedding: &[f32],
    limit: u32,
) -> Result<Vec<i64>, DbErr> {
    let query_embedding_blob = bytemuck::cast_slice::<f32, u8>(query_embedding);
    
    let results = db.query_all(
        Statement::from_sql_and_values(
            db.get_database_backend(),
            r#"
            SELECT file_id, vec_distance_cosine(embedding, ?) as distance
            FROM file_embeddings
            ORDER BY distance
            LIMIT ?
            "#,
            [query_embedding_blob.into(), limit.into()],
        )
    ).await?;
    
    Ok(results.into_iter().map(|row| row.try_get("", "file_id").unwrap()).collect())
}