// src-tauri/src/database/seeder.rs
use chrono::Utc;
use rusqlite::{params, Connection, Result};

pub fn seed_initial_data(conn: &Connection) -> Result<()> {
    if is_table_empty(conn, "folder_rules")? {
        seed_folder_rules(conn)?;
    }
    if is_table_empty(conn, "extension_rules")? {
        seed_extension_rules(conn)?;
    }
    if is_table_empty(conn, "path_rules")? {
        seed_path_rules(conn)?;
    }
    if is_table_empty(conn, "settings")? {
        seed_settings(conn)?;
    }
    if is_table_empty(conn, "filename_rules")? {
        seed_file_rules(conn)?;
    }
    Ok(())
}

fn is_table_empty(conn: &Connection, table_name: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("SELECT COUNT(*) FROM {}", table_name))?;
    let count: i64 = stmt.query_row([], |row| row.get(0))?;
    Ok(count == 0)
}

/// Some folders excluded, some included
fn seed_folder_rules(conn: &Connection) -> Result<()> {
    let folder_rules = [
        ("node_modules", "exclude"),
        (".venv", "exclude"),
        ("src", "include"),
        ("docs", "include"),
    ];

    let now = Utc::now().to_rfc3339();
    let mut stmt = conn.prepare(
        "INSERT INTO folder_rules (folder_name, rule_type, is_recursive, created_at) VALUES (?1, ?2, true, ?3)",
    )?;

    for (folder, rule_type) in &folder_rules {
        stmt.execute(params![folder, rule_type, &now])?;
    }
    Ok(())
}

/// Some extensions excluded, some included
fn seed_extension_rules(conn: &Connection) -> Result<()> {
    let extension_rules = [
        ("txt", "include"),
        ("md", "include"),
        ("pdf", "include"),
        ("log", "exclude"),
        ("tmp", "exclude"),
    ];

    let now = Utc::now().to_rfc3339();
    let mut stmt = conn.prepare(
        "INSERT INTO extension_rules (extension, rule_type, created_at) VALUES (?1, ?2, ?3)",
    )?;

    for (ext, rule_type) in &extension_rules {
        stmt.execute(params![ext, rule_type, &now])?;
    }
    Ok(())
}

/// Example path rules with mixed include/exclude
fn seed_path_rules(conn: &Connection) -> Result<()> {
    let path_rules = [
        ("/home/user/projects/important", "include"),
        ("/home/user/temp", "exclude"),
    ];

    let now = Utc::now().to_rfc3339();
    let mut stmt = conn.prepare(
        "INSERT INTO path_rules (path, rule_type, is_recursive, created_at) VALUES (?1, ?2, true, ?3)",
    )?;

    for (path, rule_type) in &path_rules {
        stmt.execute(params![path, rule_type, &now])?;
    }
    Ok(())
}

/// Settings defaults
fn seed_settings(conn: &Connection) -> Result<()> {
    let settings = [
        ("max_file_size_mb", "5"),        // Max file size in MB
        ("max_pdf_pages", "25"),          // Max pages to index in a PDF
    ];

    let now = Utc::now().to_rfc3339();
    let mut stmt = conn.prepare(
        "INSERT INTO settings (key, value, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
    )?;

    for (key, value) in &settings {
        stmt.execute(params![key, value, &now])?;
    }
    Ok(())
}

fn seed_file_rules(conn: &Connection) -> Result<()> {
    let file_rules = [
        ("README.md", "include"),
        ("secret.txt", "exclude"),
        ("draft.docx", "exclude"),
    ];

    let now = Utc::now().to_rfc3339();
    let mut stmt = conn.prepare(
        "INSERT INTO filename_rules (filename, rule_type, created_at) VALUES (?1, ?2, ?3)",
    )?;

    for (filename, rule_type) in &file_rules {
        stmt.execute(params![filename, rule_type, &now])?;
    }
    Ok(())
}
