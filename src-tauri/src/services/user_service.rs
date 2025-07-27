// src-tauri/src/services/user_services.rs
// No changes needed here, as it was already using `rusqlite` synchronously via `get_connection`.
use crate::database::get_connection;
use chrono::{DateTime, Utc};
use rusqlite::{params, Result};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct UserService;

impl UserService {
    pub fn new() -> Self {
        Self
    }

    pub fn create_user(&self, name: String, email: String) -> Result<User> {
        let conn = get_connection();
        let now = Utc::now();
        conn.execute(
            "INSERT INTO users (name, email, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![name, email, now.to_rfc3339(), now.to_rfc3339()],
        )?;
        let id = conn.last_insert_rowid() as i32;
        Ok(User {
            id,
            name,
            email,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn get_all_users(&self) -> Result<Vec<User>> {
        let conn = get_connection();
        let mut stmt = conn.prepare("SELECT id, name, email, created_at, updated_at FROM users")?;
        let user_iter = stmt.query_map([], |row| {
            Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            4,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?
                    .with_timezone(&Utc),
            })
        })?;

        let mut users = Vec::new();
        for user in user_iter {
            users.push(user?);
        }
        Ok(users)
    }

    pub fn get_user_by_id(&self, id: i32) -> Result<Option<User>> {
        let conn = get_connection();
        let mut stmt = conn
            .prepare("SELECT id, name, email, created_at, updated_at FROM users WHERE id = ?1")?;
        let mut user_iter = stmt.query_map(params![id], |row| {
            Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            4,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?
                    .with_timezone(&Utc),
            })
        })?;

        Ok(user_iter.next().transpose()?)
    }

    pub fn update_user(
        &self,
        id: i32,
        name: Option<String>,
        email: Option<String>,
    ) -> Result<User> {
        let conn = get_connection();
        let now = Utc::now();
        let mut updates = Vec::new();
        let mut params_vec: Vec<rusqlite::types::Value> = Vec::new();

        if let Some(n) = name {
            updates.push("name = ?");
            params_vec.push(rusqlite::types::Value::from(n));
        }
        if let Some(e) = email {
            updates.push("email = ?");
            params_vec.push(rusqlite::types::Value::from(e));
        }
        updates.push("updated_at = ?");
        params_vec.push(rusqlite::types::Value::from(now.to_rfc3339()));

        let set_clause = updates.join(", ");
        let query = format!(
            "UPDATE users SET {} WHERE id = ?{}",
            set_clause,
            params_vec.len() + 1
        );
        params_vec.push(rusqlite::types::Value::from(id));

        conn.execute(&query, rusqlite::params_from_iter(params_vec))?;

        // Retrieve the updated user
        let mut stmt = conn
            .prepare("SELECT id, name, email, created_at, updated_at FROM users WHERE id = ?1")?;
        let user = stmt.query_row(params![id], |row| {
            Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            4,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?
                    .with_timezone(&Utc),
            })
        })?;
        Ok(user)
    }

    pub fn delete_user(&self, id: i32) -> Result<()> {
        let conn = get_connection();
        conn.execute("DELETE FROM users WHERE id = ?1", params![id])?;
        Ok(())
    }
}
