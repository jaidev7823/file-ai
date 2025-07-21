# Development Log

This file is a chronological log of the development process, tracking tasks, challenges, and solutions.

---

### **Date:** 2025-07-21 (Continued)

**Goal:** Resolve "not authorized" database error during extension loading.

**Summary of Work:**
- Faced an error (`code: 1) not authorized`) when the application tried to load SQLite extensions (`vector0`, `vss0`). This indicated that the database connection was not configured to allow extension loading.
- **Initial Incorrect Attempt 1:** Tried to use `options.sqlx_sqlite_options().load_extension("vss0")`. This failed because `sea_orm::ConnectOptions` (version 0.12) does not expose `sqlx`-specific methods directly.
- **Initial Incorrect Attempt 2:** Tried to use `options.after_connect(...)`. This also failed because the `after_connect` method is not available in `sea-orm` version 0.12.
- **Root Cause:** My understanding of the `sea-orm` API for version 0.12 was incorrect. The `ConnectOptions` in this version do not provide direct methods for loading extensions or `after_connect` hooks.

**Key Changes & Decisions:**
1.  **Correct Extension Loading:** The correct approach for `sea_orm` 0.12 is to:
    -   Establish the `DatabaseConnection` first.
    -   Retrieve the underlying `sqlx::SqlitePool` from the `DatabaseConnection` using `db.get_sqlite_connection_pool()`.
    -   Execute the `SELECT load_extension(...)` SQL commands directly on this `sqlx::SqlitePool`.
    -   This ensures the extensions are loaded *before* migrations run, resolving the "not authorized" error.
    -   **Clarification:** This direct `sqlx` usage is *only* for loading extensions during connection initialization. All other database interactions (migrations, CRUD operations) will continue to use SeaORM's high-level API, maintaining our ORM-centric approach.

---

### **Date:** 2025-07-21

**Goal:**
- Fix initial Rust compilation errors and establish a stable baseline.
- Set up the project documentation structure.

**Summary of Work:**
- Successfully resolved a series of Rust compilation errors that were preventing the project from building.
- The initial errors were caused by a misunderstanding of how to handle database types with SeaORM and how to structure the vector storage.
- Refactored the database logic to correctly use a separate virtual table (`vss_files`) for embeddings, as intended with `sqlite-vec`.

**Key Changes & Decisions:**
1.  **Database Schema:**
    -   **Removed** the `embedding` column from the main `file` entity in `src/entities/file.rs`.
    -   **Confirmed** that embeddings will be stored in the `vss_files` virtual table, linked by `file_id`. This is a much cleaner and more scalable approach.
2.  **File Scanning Logic (`file_scanner.rs`):
    -   Updated the `scan_and_store_files` function. It now first inserts the file metadata into the `file` table, gets the new file's ID, and then calls a separate function to insert the embedding into `vss_files`.
3.  **Vector Storage Logic (`vss.rs`):
    -   Rewrote the file to use SeaORM's `DatabaseConnection` and `Statement` builder, removing the old `sqlx` code. This ensures we use a single, consistent database connection throughout the app.
4.  **Project Structure:**
    -   Made the `vss` module public by adding `pub mod vss;` to `src/lib.rs`, resolving the final compilation error.
    -   Created the `project-docs` folder to maintain a shared understanding of the project.