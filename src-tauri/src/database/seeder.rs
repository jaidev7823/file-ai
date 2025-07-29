// src-tauri/src/database/seeder.rs
use chrono::Utc;
use rusqlite::{params, Connection, Result};

pub fn seed_initial_data(conn: &Connection) -> Result<()> {
    if is_table_empty(conn, "path_rules")? {
        seed_path_rules(conn)?;
    }
    if is_table_empty(conn, "extension_rules")? {
        seed_extension_rules(conn)?;
    }
    Ok(())
}

fn is_table_empty(conn: &Connection, table_name: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("SELECT COUNT(*) FROM {}", table_name))?;
    let count: i64 = stmt.query_row([], |row| row.get(0))?;
    Ok(count == 0)
}

fn seed_path_rules(conn: &Connection) -> Result<()> {
    let default_excluded_paths = [
        "node_modules",
        ".venv",
        "ComfyUI",
        "Adobe",
        ".git",
        "target",
        "build",
        "dist",
    ];

    let now = Utc::now().to_rfc3339();
    let mut stmt = conn.prepare(
        "INSERT INTO path_rules (path, rule_type, is_recursive, created_at) VALUES (?1, 'exclude', true, ?2)",
    )?;

    for path in &default_excluded_paths {
        stmt.execute(params![path, &now])?;
    }
    Ok(())
}

fn seed_extension_rules(conn: &Connection) -> Result<()> {
    let text_extensions = [
        "txt", "md", "csv", "json", "xml", "log", "cfg", "yaml", "yml", "toml", "rs", "py",
        "js", "ts", "tsx", "jsx", "html", "css", "scss", "less", "bat", "sh", "c", "cpp",
        "h", "hpp", "java", "cs", "go", "php", "rb", "pl", "swift", "kt", "dart", "sql",
        "r", "m", "vb", "ps1", "lua", "tex", "scala", "erl", "ex", "exs", "clj", "cljs",
        "groovy", "asm", "s", "v", "sv", "makefile", "dockerfile", "gitignore",
        "gitattributes", "pdf",
    ];

    let now = Utc::now().to_rfc3339();
    let mut stmt = conn.prepare(
        "INSERT INTO extension_rules (extension, rule_type, created_at) VALUES (?1, 'include', ?2)",
    )?;

    for ext in &text_extensions {
        stmt.execute(params![ext, &now])?;
    }
    Ok(())
}
