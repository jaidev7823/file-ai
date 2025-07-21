use sea_orm::*;
use sea_orm_migration::MigratorTrait;
use std::path::PathBuf;

use crate::migration::Migrator;

pub async fn init_database() -> Result<DatabaseConnection, DbErr> {
    // Get the app data directory
    let app_data_dir = get_app_data_dir().expect("Failed to get app data directory");
    
    // Create the directory if it doesn't exist
    std::fs::create_dir_all(&app_data_dir).expect("Failed to create app data directory");
    
    // Create database file path
    let database_path = app_data_dir.join("database.db");
    let database_url = format!("sqlite://{}?mode=rwc", database_path.display());
    
    println!("Connecting to database at: {}", database_url);
    
    // Connect to database with options
    let mut options = ConnectOptions::new(database_url);
    options.sqlx_logging(true); // Enable SQL logging if needed
    
    let db = Database::connect(options).await?;
    
    // Load sqlite-vec extension after connection
    println!("Loading sqlite-vec extension...");
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "SELECT load_extension('vec0');".to_string(),
    )).await?;
    
    // Run migrations
    println!("Running migrations...");
    Migrator::up(&db, None).await?;
    println!("Migrations completed successfully");    
    println!("Database initialized successfully at: {:?}", database_path);
    
    Ok(db)
}

fn get_app_data_dir() -> Option<PathBuf> {
    // This gets the appropriate directory for each OS:
    // Windows: C:\Users\{username}\AppData\Local\{app_name}
    // macOS: /Users/{username}/Library/Application Support/{app_name}
    // Linux: /home/{username}/.local/share/{app_name}
    
    if let Some(mut dir) = dirs::data_local_dir() {
        dir.push("FileAI"); // Use your actual app name
        Some(dir)
    } else {
        // Fallback to current directory
        std::env::current_dir().ok()
    }
}