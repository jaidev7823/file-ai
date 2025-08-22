use chrono::Utc;
use rusqlite::{params, Connection, Result};
use std::env;
use sysinfo::Disks;
use std::path::PathBuf;

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

fn seed_folder_rules(conn: &Connection) -> Result<()> {
    let os = env::consts::OS;
    let now = Utc::now().to_rfc3339();

    let folder_rules: Vec<(&str, &str)> = match os {
        "windows" => vec![
            ("node_modules", "exclude"),
            (".venv", "exclude"),
            ("venv", "exclude"),
            ("dist", "exclude"),
            ("build", "exclude"),
            (".git", "exclude"),
            (".idea", "exclude"),
            (".vscode", "exclude"),
            (".cache", "exclude"),
            ("__pycache__", "exclude"),
            ("Thumbs.db", "exclude"),
            ("desktop.ini", "exclude"),
            ("target", "exclude"),
            ("coverage", "exclude"),
            ("logs", "exclude"),
            ("Recycle.Bin", "exclude"),
            ("Windows", "exclude"),
            ("Program Files", "exclude"),
            ("Program Files (x86)", "exclude"),
            ("ProgramData", "exclude"),
            ("AppData", "exclude"),
            ("src", "include"),
            ("docs", "include"),
            ("public", "include"),
            ("assets", "include"),
        ],
        "macos" => vec![
            ("node_modules", "exclude"),
            (".venv", "exclude"),
            ("venv", "exclude"),
            ("dist", "exclude"),
            ("build", "exclude"),
            (".git", "exclude"),
            (".idea", "exclude"),
            (".vscode", "exclude"),
            (".cache", "exclude"),
            ("__pycache__", "exclude"),
            (".DS_Store", "exclude"),
            ("target", "exclude"),
            ("coverage", "exclude"),
            ("logs", "exclude"),
            ("System", "exclude"),
            ("Library", "exclude"),
            ("Applications", "exclude"),
            ("src", "include"),
            ("docs", "include"),
            ("public", "include"),
            ("assets", "include"),
        ],
        _ => vec![
            // Linux default
            ("node_modules", "exclude"),
            (".venv", "exclude"),
            ("venv", "exclude"),
            ("dist", "exclude"),
            ("build", "exclude"),
            (".git", "exclude"),
            (".idea", "exclude"),
            (".vscode", "exclude"),
            (".cache", "exclude"),
            ("__pycache__", "exclude"),
            ("target", "exclude"),
            ("coverage", "exclude"),
            ("logs", "exclude"),
            ("/proc", "exclude"),
            ("/sys", "exclude"),
            ("/dev", "exclude"),
            ("/run", "exclude"),
            ("/var/cache", "exclude"),
            ("/var/log", "exclude"),
            ("/tmp", "exclude"),
            ("src", "include"),
            ("docs", "include"),
            ("public", "include"),
            ("assets", "include"),
        ],
    };

    let now = Utc::now().to_rfc3339();
    let mut stmt = conn.prepare(
        "INSERT INTO folder_rules (folder_name, rule_type, is_recursive, created_at) VALUES (?1, ?2, true, ?3)",
    )?;
    for (folder, rule_type) in &folder_rules {
        stmt.execute(params![folder, rule_type, &now])?;
    }
    Ok(())
}

fn seed_extension_rules(conn: &Connection) -> Result<()> {
    let extension_rules = [
        // Exclude binaries & junk
        ("exe", "exclude"),
        ("dll", "exclude"),
        ("bin", "exclude"),
        ("obj", "exclude"),
        ("o", "exclude"),
        ("class", "exclude"),
        ("so", "exclude"),
        ("dylib", "exclude"),
        ("lib", "exclude"),
        ("log", "exclude"),
        ("tmp", "exclude"),
        ("bak", "exclude"),
        ("swp", "exclude"),
        ("old", "exclude"),
        ("iso", "exclude"),
        ("dmg", "exclude"),
        ("vhd", "exclude"),
        ("vhdx", "exclude"),
        ("tar", "exclude"),
        ("gz", "exclude"),
        ("zip", "exclude"),
        ("rar", "exclude"),
        ("7z", "exclude"),
        ("xz", "exclude"),
        ("mp4", "exclude"),
        ("mp3", "exclude"),
        ("wav", "exclude"),
        ("mov", "exclude"),
        ("avi", "exclude"),
        ("mkv", "exclude"),
        ("flac", "exclude"),
        ("webm", "exclude"),
        ("flv", "exclude"),
        // Include text/code/docs
        ("txt", "include"),
        ("md", "include"),
        ("pdf", "include"),
        ("json", "include"),
        ("csv", "include"),
        ("xml", "include"),
        ("html", "include"),
        ("js", "exclude"),
        ("ts", "exclude"),
        ("rs", "exclude"),
        ("py", "exclude"),
        ("java", "exclude"),
        ("c", "exclude"),
        ("cpp", "exclude"),
        ("h", "exclude"),
        ("css", "exclude"),
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

fn seed_path_rules(conn: &Connection) -> Result<()> {
    let os = env::consts::OS;

    // Detect home/user directory
    let home_dir: Option<PathBuf> = dirs::home_dir();

    let system_paths_to_exclude = match os {
        "windows" => vec![
            "C:\\Windows",
            "C:\\Program Files",
            "C:\\Program Files (x86)",
            "C:\\ProgramData",
        ],
        "macos" => vec!["/System", "/Library", "/Applications"],
        _ => vec![
            "/proc",
            "/sys",
            "/dev",
            "/run",
            "/var/cache",
            "/var/log",
            "/tmp",
        ],
    };

    let now = Utc::now().to_rfc3339();
    let mut stmt = conn.prepare(
        "INSERT INTO path_rules (path, rule_type, is_recursive, created_at) 
         SELECT ?1, ?2, true, ?3
         WHERE NOT EXISTS (SELECT 1 FROM path_rules WHERE path = ?1)"
    )?;

    // 1️⃣ Always include home directory if found
    if let Some(home) = home_dir {
        let home_str = home.to_string_lossy().to_string();
        if !system_paths_to_exclude.iter().any(|p| home_str.starts_with(p)) {
            stmt.execute(params![home_str, "include", &now])?;
        }
    }

    // 2️⃣ Include all non-system drives
    let drives = get_all_drives();
    for drive in drives {
        if !system_paths_to_exclude.iter().any(|p| drive.starts_with(p)) {
            stmt.execute(params![drive, "include", &now])?;
        }
    }

    Ok(())
}

fn get_all_drives() -> Vec<String> {
    let disks = Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .map(|d| d.mount_point().to_string_lossy().to_string())
        .collect()
}

fn seed_settings(conn: &Connection) -> Result<()> {
    let settings = [
        ("max_file_size_mb", "5"),
        ("max_pdf_pages", "25"),
        ("max_index_depth", "10"),
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
        // Exclude common junk/lock/system files
        ("package-lock.json", "exclude"),
        ("yarn.lock", "exclude"),
        ("pnpm-lock.yaml", "exclude"),
        ("Pipfile.lock", "exclude"),
        ("poetry.lock", "exclude"),
        ("Cargo.lock", "exclude"),
        ("composer.lock", "exclude"),
        (".env", "exclude"),
        (".env.local", "exclude"),
        (".env.development", "exclude"),
        (".env.production", "exclude"),
        (".npmrc", "exclude"),
        (".yarnrc", "exclude"),
        (".DS_Store", "exclude"),
        ("Thumbs.db", "exclude"),
        ("desktop.ini", "exclude"),
        (".gitignore", "exclude"),
        (".gitattributes", "exclude"),
        (".gitmodules", "exclude"),
        ("manifest.json", "exclude"),
        ("robots.txt", "exclude"),
        ("service-worker.js", "exclude"),
        ("asset-manifest.json", "exclude"),
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