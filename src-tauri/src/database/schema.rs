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

pub const CREATE_FILES_FTS_TABLE: &str = r#"
CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
    name, 
    content, 
    extension,
    content='files',
    content_rowid='id'
);
"#;

pub const CREATE_FTS_TRIGGERS: &str = r#"
CREATE TRIGGER IF NOT EXISTS files_fts_insert AFTER INSERT ON files BEGIN
    INSERT INTO files_fts(rowid, name, content, extension) 
    VALUES (new.id, new.name, new.content, new.extension);
END;

CREATE TRIGGER IF NOT EXISTS files_fts_update AFTER UPDATE ON files BEGIN
    UPDATE files_fts SET name = new.name, content = new.content, extension = new.extension 
    WHERE rowid = new.id;
END;

CREATE TRIGGER IF NOT EXISTS files_fts_delete AFTER DELETE ON files BEGIN
    DELETE FROM files_fts WHERE rowid = old.id;
END;
"#;

pub fn create_all_sql() -> String {
    format!(
        "{}{}{}{}{}{}",
        CREATE_USERS_TABLE,
        CREATE_FILES_TABLE,
        CREATE_FILE_VEC_TABLE,
        CREATE_FILE_VEC_MAP_TABLE,
        CREATE_FILES_FTS_TABLE,
        CREATE_FTS_TRIGGERS
    )
}
