// Import the get_connection function from the database module
use crate::database::get_connection;

// Import types from chrono for handling timestamps
use chrono::{DateTime, Utc};

// Import rusqlite functions and types
use rusqlite::{params, Result};

// Define a struct to represent a user
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: i32,                      // Unique ID for each user (Primary Key)
    pub name: String,                // User's name
    pub email: String,               // User's email
    pub created_at: DateTime<Utc>,   // Timestamp when user was created
    pub updated_at: DateTime<Utc>,   // Timestamp when user was last updated
}

// Define a struct to hold user-related database operations (empty struct)
pub struct UserService;

// Implementation block for UserService
impl UserService {
    // Constructor method to create a new instance of UserService
    pub fn new() -> Self {
        Self // Return an instance of the empty struct
    }

    // Method to create a new user and insert into the database
    pub fn create_user(&self, name: String, email: String) -> Result<User> {
        let conn = get_connection(); // Get a connection to the database
        let now = Utc::now(); // Current UTC timestamp

        // Insert the new user into the `users` table with current timestamps
        conn.execute(
            "INSERT INTO users (name, email, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![name, email, now.to_rfc3339(), now.to_rfc3339()], // Safe SQL parameters
        )?;

        // Get the ID of the newly inserted row
        let id = conn.last_insert_rowid() as i32;

        // Return the created user object
        Ok(User {
            id,
            name,
            email,
            created_at: now,
            updated_at: now,
        })
    }

    // Method to retrieve all users from the database
    pub fn get_all_users(&self) -> Result<Vec<User>> {
        let conn = get_connection(); // Get DB connection

        // Prepare the SQL query
        let mut stmt = conn.prepare("SELECT id, name, email, created_at, updated_at FROM users")?;

        // Run the query and map each row to a User struct
        let user_iter = stmt.query_map([], |row| {
            Ok(User {
                id: row.get(0)?, // Get ID
                name: row.get(1)?, // Get name
                email: row.get(2)?, // Get email

                // Parse created_at string into DateTime<Utc>
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    ))?
                    .with_timezone(&Utc),

                // Parse updated_at string into DateTime<Utc>
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    ))?
                    .with_timezone(&Utc),
            })
        })?;

        // Collect results into a vector
        let mut users = Vec::new();
        for user in user_iter {
            users.push(user?); // Unwrap or return error
        }

        Ok(users) // Return list of users
    }

    // Method to get a user by their ID
    pub fn get_user_by_id(&self, id: i32) -> Result<Option<User>> {
        let conn = get_connection(); // Get DB connection

        // Prepare and execute the query
        let mut stmt = conn.prepare(
            "SELECT id, name, email, created_at, updated_at FROM users WHERE id = ?1"
        )?;

        // Run the query with ID as parameter
        let mut user_iter = stmt.query_map(params![id], |row| {
            Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    ))?
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    ))?
                    .with_timezone(&Utc),
            })
        })?;

        // Return the first user if it exists, otherwise None
        Ok(user_iter.next().transpose()?)
    }

    // Method to update an existing user’s name and/or email
    pub fn update_user(
        &self,
        id: i32,
        name: Option<String>,
        email: Option<String>,
    ) -> Result<User> {
        let conn = get_connection(); // DB connection
        let now = Utc::now(); // Current time

        let mut updates = Vec::new(); // SQL SET clauses (e.g., name = ?)
        let mut params_vec: Vec<rusqlite::types::Value> = Vec::new(); // Actual values for SQL

        // If name is provided, include it in the update
        if let Some(n) = name {
            updates.push("name = ?"); // Add SQL part
            params_vec.push(rusqlite::types::Value::from(n)); // Add actual value
        }

        // If email is provided, include it in the update
        if let Some(e) = email {
            updates.push("email = ?");
            params_vec.push(rusqlite::types::Value::from(e));
        }

        // Always update the updated_at timestamp
        updates.push("updated_at = ?");
        params_vec.push(rusqlite::types::Value::from(now.to_rfc3339()));

        // Join all updates into one string: e.g., "name = ?, email = ?, updated_at = ?"
        let set_clause = updates.join(", ");

        // Build the final query with a parameter for the user ID at the end
        let query = format!(
            "UPDATE users SET {} WHERE id = ?{}",
            set_clause,
            params_vec.len() + 1 // the ID comes last in the param list
        );

        // Add the user ID as the final parameter
        params_vec.push(rusqlite::types::Value::from(id));

        // Execute the update
        conn.execute(&query, rusqlite::params_from_iter(params_vec))?;

        // After updating, retrieve and return the updated user
        let mut stmt = conn.prepare(
            "SELECT id, name, email, created_at, updated_at FROM users WHERE id = ?1"
        )?;
        let user = stmt.query_row(params![id], |row| {
            Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    ))?
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    ))?
                    .with_timezone(&Utc),
            })
        })?;

        Ok(user)
    }

    // Method to delete a user by their ID
    pub fn delete_user(&self, id: i32) -> Result<()> {
        let conn = get_connection(); // Get connection
        conn.execute("DELETE FROM users WHERE id = ?1", params![id])?; // Execute deletion
        Ok(()) // Return success (no value)
    }
}
