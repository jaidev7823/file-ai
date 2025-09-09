use std::path::PathBuf;

pub fn get_app_data_dir() -> Option<PathBuf> {
    if let Some(mut dir) = dirs::data_local_dir() {
        dir.push("FileAI"); // your app name
        Some(dir)
    } else {
        std::env::current_dir().ok()
    }
}

#[tauri::command]
pub async fn create_local_lancedb() -> Result<(), String> {
    let database_path = get_app_data_dir()
        .expect("Could not get app data directory")
        .join("my-lancedb");
    // let uri = "data/my-lancedb";
    let db = lancedb::connect(
        database_path
            .to_str()
            .ok_or("Failed to convert path to string")?,
    )
    .execute()
    .await
    .map_err(|e| format!("LanceDB Error: {}", e))?;

    Ok(())
}
