pub const CREATE_USERS_TABLE: &str = "
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
";

pub const CREATE_FILES_TABLE: &str = "
CREATE TABLE IF NOT EXISTS files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    extension TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    content TEXT NOT NULL,
    author TEXT,          -- New: Author of the file
    file_size INTEGER,    -- New: File size in bytes
    category TEXT,        -- New: File category (Code, Document, etc.)
    content_processed BOOLEAN DEFAULT 1, -- New: Whether content was processed or just metadata
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_accessed TEXT    -- New: Last accessed timestamp
);

";

pub const CREATE_FILE_VEC_TABLE: &str = r#"
CREATE VIRTUAL TABLE IF NOT EXISTS file_vec USING vec0(
    content_vec float[768]
);
"#;

pub const CREATE_FILE_VEC_MAP_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS file_vec_map (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    vec_rowid INTEGER NOT NULL,
    file_id INTEGER NOT NULL,
    FOREIGN KEY(file_id) REFERENCES files(id)
);
"#;



pub const CREATE_PATH_RULES_TABLE: &str = "
CREATE TABLE IF NOT EXISTS path_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL,
    rule_type TEXT NOT NULL,
    is_recursive BOOLEAN NOT NULL,
    created_at TEXT NOT NULL
);
";

pub const CREATE_EXTENSION_RULES_TABLE: &str = "
CREATE TABLE IF NOT EXISTS extension_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    extension TEXT NOT NULL,
    rule_type TEXT NOT NULL,
    created_at TEXT NOT NULL
);
";

pub const CREATE_FOLDER_RULES_TABLE: &str = "
CREATE TABLE IF NOT EXISTS folder_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    folder_name TEXT NOT NULL,
    rule_type TEXT NOT NULL, -- 'include' or 'exclude'
    is_recursive BOOLEAN NOT NULL,
    created_at TEXT NOT NULL
);
";

pub const CREATE_FILENAME_RULES_TABLE: &str = "
CREATE TABLE IF NOT EXISTS filename_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    filename TEXT NOT NULL,
    rule_type TEXT NOT NULL, -- 'exclude'
    created_at TEXT NOT NULL
);
";

pub const CREATE_SETTINGS_TABLE: &str = "
CREATE TABLE IF NOT EXISTS settings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key TEXT NOT NULL UNIQUE,
    value TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
";

pub const CREATE_FILES_FTS_TABLE: &str = r#"
CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
    name,
    content,
    content='files',
    content_rowid='id'
);
"#;

pub const CREATE_FOLDERS_TABLE: &str = "
CREATE TABLE IF NOT EXISTS folders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    parent_folder_id INTEGER,
    file_count INTEGER DEFAULT 0,
    folder_size INTEGER DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_accessed TEXT,
    FOREIGN KEY(parent_folder_id) REFERENCES folders(id)
);
";

pub fn create_all_sql() -> String {
    format!(
        "{}{}{}{}{}{}{}{}{}{}{}",
        CREATE_USERS_TABLE,
        CREATE_FILES_TABLE,
        CREATE_FILE_VEC_TABLE,
        CREATE_FILE_VEC_MAP_TABLE,
        CREATE_FILES_FTS_TABLE,
        CREATE_FOLDERS_TABLE,
        
        CREATE_PATH_RULES_TABLE,
        CREATE_FOLDER_RULES_TABLE,
        CREATE_EXTENSION_RULES_TABLE,
        CREATE_FILENAME_RULES_TABLE,
        CREATE_SETTINGS_TABLE
    )
}
