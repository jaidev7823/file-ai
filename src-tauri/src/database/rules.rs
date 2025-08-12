use rusqlite::Connection;
use std::collections::HashSet;
use std::error::Error;
use chrono::Utc;
use rusqlite::params;

// Unified rule type enum
#[derive(Debug)]
pub enum RuleType {
    Include,
    Exclude,
}

impl RuleType {
    fn as_str(&self) -> &'static str {
        match self {
            RuleType::Include => "include",
            RuleType::Exclude => "exclude",
        }
    }
}

// Rule category enum
#[derive(Debug)]
pub enum RuleCategory {
    Path,
    Folder,
    Extension,
    Filename,
}

impl RuleCategory {
    fn table_name(&self) -> &'static str {
        match self {
            RuleCategory::Path => "path_rules",
            RuleCategory::Folder => "folder_rules", 
            RuleCategory::Extension => "extension_rules",
            RuleCategory::Filename => "filename_rules",
        }
    }

    fn column_name(&self) -> &'static str {
        match self {
            RuleCategory::Path => "path",
            RuleCategory::Folder => "folder_name",
            RuleCategory::Extension => "extension", 
            RuleCategory::Filename => "filename",
        }
    }
}

// Generic function to get rules
pub fn get_rules_sync(
    db: &Connection,
    category: RuleCategory,
    rule_type: RuleType,
) -> Result<HashSet<String>, Box<dyn Error>> {
    let query = format!(
        "SELECT {} FROM {} WHERE rule_type = ?1",
        category.column_name(),
        category.table_name()
    );
    
    let mut stmt = db.prepare(&query)?;
    let rules = stmt.query_map([rule_type.as_str()], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    
    println!("{:?} {:?}: {:?}", rule_type, category, rules);
    Ok(rules.into_iter().collect())
}

// Simplified public functions using the generic function
pub fn get_included_paths_sync(db: &Connection) -> Result<HashSet<String>, Box<dyn Error>> {
    get_rules_sync(db, RuleCategory::Path, RuleType::Include)
}

pub fn get_included_folders_sync(db: &Connection) -> Result<HashSet<String>, Box<dyn Error>> {
    get_rules_sync(db, RuleCategory::Folder, RuleType::Include)
}

pub fn get_included_extensions_sync(db: &Connection) -> Result<HashSet<String>, Box<dyn Error>> {
    get_rules_sync(db, RuleCategory::Extension, RuleType::Include)
}

pub fn get_excluded_extensions_sync(db: &Connection) -> Result<HashSet<String>, Box<dyn Error>> {
    get_rules_sync(db, RuleCategory::Extension, RuleType::Exclude)
}

pub fn get_excluded_filenames_sync(db: &Connection) -> Result<HashSet<String>, Box<dyn Error>> {
    get_rules_sync(db, RuleCategory::Filename, RuleType::Exclude)
}

pub fn get_excluded_folder_sync(db: &Connection) -> Result<HashSet<String>, Box<dyn Error>> {
    get_rules_sync(db, RuleCategory::Path, RuleType::Exclude)
}

pub fn get_excluded_paths_sync(db: &Connection) -> Result<HashSet<String>, Box<dyn Error>> {
    get_rules_sync(db, RuleCategory::Path, RuleType::Exclude)
}

// Generic function for adding rules
async fn add_rule(
    category: RuleCategory,
    rule_type: RuleType,
    value: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        let now = Utc::now().to_rfc3339();
        
        match category {
            RuleCategory::Path | RuleCategory::Folder => {
                let query = format!(
                    "INSERT INTO {} ({}, rule_type, is_recursive, created_at) VALUES (?1, ?2, true, ?3)",
                    category.table_name(),
                    category.column_name()
                );
                db.execute(&query, params![value, rule_type.as_str(), now])
            },
            RuleCategory::Extension | RuleCategory::Filename => {
                let query = format!(
                    "INSERT INTO {} ({}, rule_type, created_at) VALUES (?1, ?2, ?3)",
                    category.table_name(),
                    category.column_name()
                );
                db.execute(&query, params![value, rule_type.as_str(), now])
            }
        }.map_err(|e| format!("Database error: {}", e))?;
        
        Ok(())
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

// Generic function for removing rules
async fn remove_rule(
    category: RuleCategory,
    rule_type: RuleType,
    value: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let db = crate::database::get_connection();
        let query = format!(
            "DELETE FROM {} WHERE {} = ?1 AND rule_type = ?2",
            category.table_name(),
            category.column_name()
        );
        
        db.execute(&query, params![value, rule_type.as_str()])
            .map_err(|e| format!("Database error: {}", e))?;
        Ok(())
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))?
}

// Tauri command functions using the generic functions
#[tauri::command]
pub async fn add_included_extension(extension: String) -> Result<(), String> {
    add_rule(RuleCategory::Extension, RuleType::Include, extension).await
}

#[tauri::command]
pub async fn add_excluded_folder(path: String) -> Result<(), String> {
    add_rule(RuleCategory::Path, RuleType::Exclude, path).await
}

#[tauri::command]
pub async fn remove_excluded_folder(path: String) -> Result<(), String> {
    remove_rule(RuleCategory::Path, RuleType::Exclude, path).await
}

#[tauri::command]
pub async fn remove_included_extension(extension: String) -> Result<(), String> {
    remove_rule(RuleCategory::Extension, RuleType::Include, extension).await
}

#[tauri::command]
pub async fn add_included_path(path: String) -> Result<(), String> {
    add_rule(RuleCategory::Path, RuleType::Include, path).await
}

#[tauri::command]
pub async fn remove_included_path(path: String) -> Result<(), String> {
    remove_rule(RuleCategory::Path, RuleType::Include, path).await
}

#[tauri::command]
pub async fn add_excluded_path(path: String) -> Result<(), String> {
    add_rule(RuleCategory::Path, RuleType::Exclude, path).await
}

#[tauri::command]
pub async fn remove_excluded_path(path: String) -> Result<(), String> {
    remove_rule(RuleCategory::Path, RuleType::Exclude, path).await
}

#[tauri::command]
pub async fn add_included_folder(folder_name: String) -> Result<(), String> {
    add_rule(RuleCategory::Folder, RuleType::Include, folder_name).await
}

#[tauri::command]
pub async fn remove_included_folder(folder_name: String) -> Result<(), String> {
    remove_rule(RuleCategory::Folder, RuleType::Include, folder_name).await
}

#[tauri::command]
pub async fn add_excluded_extension(extension: String) -> Result<(), String> {
    add_rule(RuleCategory::Extension, RuleType::Exclude, extension).await
}

#[tauri::command]
pub async fn remove_excluded_extension(extension: String) -> Result<(), String> {
    remove_rule(RuleCategory::Extension, RuleType::Exclude, extension).await
}

#[tauri::command]
pub async fn add_excluded_filename(filename: String) -> Result<(), String> {
    add_rule(RuleCategory::Filename, RuleType::Exclude, filename).await
}

#[tauri::command]
pub async fn remove_excluded_filename(filename: String) -> Result<(), String> {
    remove_rule(RuleCategory::Filename, RuleType::Exclude, filename).await
}

