use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::str::FromStr;

pub async fn enable_vss_extension(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Load VSS extension (you'll need to provide the path to your compiled VSS extension)
    sqlx::query("SELECT load_extension('vector0')")
        .execute(pool)
        .await?;
    
    sqlx::query("SELECT load_extension('vss0')")
        .execute(pool)
        .await?;
    
    Ok(())
}

pub async fn create_vss_tables(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Create virtual table for vector search
    sqlx::query(
        r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS vss_files USING vss0(
            embedding(1536),  // Adjust dimension based on your embedding model
            file_id INTEGER
        );
        "#
    )
    .execute(pool)
    .await?;
    
    Ok(())
}