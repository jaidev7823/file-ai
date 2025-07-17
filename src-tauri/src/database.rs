use sqlx::{SqlitePool, Row};

pub async fn get_db() -> Result<SqlitePool, String> {
    SqlitePool::connect("sqlite:db/database.sqlite")
        .await
        .map_err(|e| e.to_string())
}
// Add a toy command
#[tauri::command]
pub async fn add_toy(name: String, color: String) -> Result<(), String> {
    let db = get_db().await?;
    sqlx::query("INSERT INTO toys (name, color) VALUES (?, ?)")
        .bind(name)
        .bind(color)
        .execute(&db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// Get all toys command
#[tauri::command]
pub async fn get_toys() -> Result<Vec<(i64, String, String)>, String> {
    let db = get_db().await?;
    let rows = sqlx::query("SELECT id, name, color FROM toys")
        .fetch_all(&db)
        .await
        .map_err(|e| e.to_string())?;
    
    let toys = rows.iter().map(|r| {
        (
            r.get::<i64, _>("id"),
            r.get::<String, _>("name"),
            r.get::<String, _>("color")
        )
    }).collect();
    
    Ok(toys)
}