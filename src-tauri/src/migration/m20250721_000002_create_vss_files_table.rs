use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        
        // Load sqlite-vec extension
        db.execute_unprepared("SELECT load_extension('vec0');").await?;

        // Create the table for storing file embeddings using sqlite-vec
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS file_embeddings (
                    file_id INTEGER PRIMARY KEY,
                    embedding BLOB NOT NULL,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY (file_id) REFERENCES files (id) ON DELETE CASCADE
                );
                "#,
            )
            .await?;
            
        // Create a vector index for faster similarity search
        // Note: This syntax may vary depending on sqlite-vec version
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX IF NOT EXISTS idx_file_embeddings_vec 
                ON file_embeddings (embedding);
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS file_embeddings")
            .await?;
        Ok(())
    }
}