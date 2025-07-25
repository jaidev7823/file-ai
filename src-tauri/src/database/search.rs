use crate::embed_and_store::get_embedding;
use bytemuck::cast_slice;
use rusqlite::{params, Connection, Result};

pub fn search_similar_files(
    db: &Connection,
    query: &str,
    top_k: usize,
) -> Result<Vec<(String, f32)>, String> {
    let embedding = get_embedding(query).map_err(|e| e.to_string())?;

    if embedding.is_empty() {
        return Err("Query embedding is empty".into());
    }

    let vector_bytes: &[u8] = cast_slice(&embedding);

    let queries_to_try = [
        "SELECT f.path, -vec_distance_cosine(fv.content_vec, ?) AS score
         FROM file_vec AS fv
         JOIN file_vec_map AS m ON m.vec_rowid = fv.rowid
         JOIN files AS f ON f.id = m.file_id
         ORDER BY score DESC
         LIMIT ?;",
        "SELECT f.path, -vec_distance_l2(fv.content_vec, ?) AS score
         FROM file_vec AS fv
         JOIN file_vec_map AS m ON m.vec_rowid = fv.rowid
         JOIN files AS f ON f.id = m.file_id
         ORDER BY score DESC
         LIMIT ?;",
        "SELECT f.path, -vec0_distance_cosine(fv.content_vec, ?) AS score
         FROM file_vec AS fv
         JOIN file_vec_map AS m ON m.vec_rowid = fv.rowid
         JOIN files AS f ON f.id = m.file_id
         ORDER BY score DESC
         LIMIT ?;",
        "SELECT f.path, DOT_PRODUCT(fv.content_vec, ?) AS score
         FROM file_vec AS fv
         JOIN file_vec_map AS m ON m.vec_rowid = fv.rowid
         JOIN files AS f ON f.id = m.file_id
         ORDER BY score DESC
         LIMIT ?;",
    ];

    for (i, query_sql) in queries_to_try.iter().enumerate() {
        match db.prepare(query_sql) {
            Ok(mut stmt) => {
                println!("Using query variant {}: Success", i + 1);
                let results = stmt
                    .query_map(params![vector_bytes, top_k as i64], |row| {
                        let path: String = row.get(0)?;
                        let score: f32 = row.get(1)?;
                        Ok((path, score))
                    })
                    .map_err(|e| e.to_string())?
                    .filter_map(Result::ok)
                    .collect();

                return Ok(results);
            }
            Err(e) => {
                println!("Query variant {} failed: {}", i + 1, e);
                continue;
            }
        }
    }

    Err("No working vector similarity function found. Make sure sqlite-vec extension is properly loaded.".into())
}

pub fn debug_print_file_vec_schema(db: &Connection) {
    let mut stmt = db
        .prepare("PRAGMA table_info(file_vec);")
        .expect("Failed to prepare PRAGMA query");

    let columns = stmt
        .query_map([], |row| {
            let cid: i32 = row.get(0)?;
            let name: String = row.get(1)?;
            let dtype: String = row.get(2)?;
            Ok((cid, name, dtype))
        })
        .expect("Could not query map");

    println!("Schema for file_vec:");
    for column in columns {
        println!("{:?}", column.expect("Error retrieving column"));
    }
}

pub fn debug_print_available_functions(db: &Connection) {
    println!("Testing vector functions:");

    let test_functions = [
        "vec_distance_cosine",
        "vec_distance_l2",
        "vec0_distance_cosine",
        "vec0_distance_l2",
        "DOT_PRODUCT",
        "vec_dot",
        "vec0_dot",
    ];

    for func_name in &test_functions {
        let test_query = format!("SELECT {}(?, ?)", func_name);
        match db.prepare(&test_query) {
            Ok(_) => println!("✓ {} is available", func_name),
            Err(_) => println!("✗ {} is not available", func_name),
        }
    }

    match db.prepare("SELECT vec_version()") {
        Ok(mut stmt) => {
            if let Ok(version) = stmt.query_row([], |row| row.get::<_, String>(0)) {
                println!("sqlite-vec version: {}", version);
            }
        }
        Err(_) => {
            match db.prepare("SELECT vec0_version()") {
                Ok(mut stmt) => {
                    if let Ok(version) = stmt.query_row([], |row| row.get::<_, String>(0)) {
                        println!("sqlite-vec version: {}", version);
                    }
                }
                Err(_) => println!("Could not determine sqlite-vec version"),
            }
        }
    }
}
