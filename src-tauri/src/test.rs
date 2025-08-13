use crate::file_scanner;
use crate::database;
use crate::embed_and_store;
use rusqlite::{ Connection, Result};
use tauri::AppHandle;

#[tauri::command]
pub async fn test_embedding(query: String) -> Result<String, String> {
    println!("Testing embedding for: {}", query);

    // Clone query for the blocking task
    let query_for_embedding = query.clone();

    let embedding_task_result = tokio::task::spawn_blocking(move || {
        embed_and_store::get_embedding(&query_for_embedding) // Use the cloned query
            .map_err(|e| format!("Embedding error: {}", e))
    })
    .await;

    let embedding = match embedding_task_result {
        Ok(inner_result) => inner_result?,
        Err(join_err) => return Err(format!("Task spawn error: {}", join_err)),
    };

    // Original `query` is still available here for the final format string
    Ok(format!(
        "Got embedding with {} dimensions for query: {}",
        embedding.len(),
        query
    ))
}

#[tauri::command]
pub async fn test_file_filtering(app: AppHandle) -> Result<Vec<String>, String> {
    tokio::task::spawn_blocking(move || {
        let db = database::get_connection();
        match file_scanner::find_text_files(&db, Some(50_000_000), &app) {
            Ok(files) => {
                println!("Found {} files after filtering", files.len());
                Ok(files)
            }
            Err(e) => Err(format!("Error during file filtering: {}", e)),
        }
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

#[tauri::command]
pub async fn debug_database_rules() -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let db = database::get_connection();

        let mut debug_info = String::new();

        // Path Rules
        let mut stmt = db.prepare("SELECT path, rule_type FROM path_rules")
            .map_err(|e| format!("Database error: {}", e))?;
        let path_rules: Result<Vec<(String, String)>, _> = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| format!("Database error: {}", e))?
            .collect();

        debug_info.push_str("Path Rules:\n");
        for (path, rule_type) in path_rules.map_err(|e| format!("Database error: {}", e))? {
            debug_info.push_str(&format!("  {} - {}\n", path, rule_type));
        }

        // Extension Rules
        let mut stmt = db.prepare("SELECT extension, rule_type FROM extension_rules")
            .map_err(|e| format!("Database error: {}", e))?;
        let ext_rules: Result<Vec<(String, String)>, _> = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| format!("Database error: {}", e))?
            .collect();

        debug_info.push_str("\nExtension Rules:\n");
        for (ext, rule_type) in ext_rules.map_err(|e| format!("Database error: {}", e))? {
            debug_info.push_str(&format!("  {} - {}\n", ext, rule_type));
        }

        Ok(debug_info)
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

// Debug functions (no change needed here)
pub fn debug_print_available_functions(db: &Connection) {
    match db.prepare("SELECT name FROM pragma_function_list() WHERE name LIKE '%vec%'") {
        Ok(mut stmt) => {
            let function_iter = stmt.query_map([], |row| Ok(row.get::<_, String>(0)?));

            println!("Available vector functions:");
            if let Ok(functions) = function_iter {
                for function_result in functions {
                    if let Ok(function_name) = function_result {
                        println!("  - {}", function_name);
                    }
                }
            }
        }
        Err(e) => println!("Error querying functions: {}", e),
    }
}

pub fn debug_print_file_vec_schema(db: &Connection) {
    match db.prepare("PRAGMA table_info(file_vec)") {
        Ok(mut stmt) => {
            let column_iter = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(1)?, // column name
                    row.get::<_, String>(2)?, // data type
                ))
            });

            println!("file_vec table schema:");
            if let Ok(columns) = column_iter {
                for column_result in columns {
                    if let Ok((name, data_type)) = column_result {
                        println!("  - {}: {}", name, data_type);
                    }
                }
            }
        }
        Err(e) => println!("Error querying file_vec schema: {}", e),
    }
}
