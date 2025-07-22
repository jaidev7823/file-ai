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
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
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


// ✅ This is a function, not a constant
pub fn create_all_sql() -> String {
    format!(
        "{}{}{}{}",
        CREATE_USERS_TABLE,
        CREATE_FILES_TABLE,
        CREATE_FILE_VEC_TABLE,
        CREATE_FILE_VEC_MAP_TABLE
    )
}
