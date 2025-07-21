// src/entities/mod.rs
pub mod user;
pub mod file;

// Re-export for convenience
pub use user::{Entity as User, Model as UserModel};