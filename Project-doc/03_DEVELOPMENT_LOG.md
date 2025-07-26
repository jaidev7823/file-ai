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

  Here's a breakdown of the project structure and how each part fits into the async/sync model.

  src-tauri/src/main.rs & src-tauri/src/lib.rs

   * Nature: [ASYNC]
   * Purpose: This is the entry point of the application. It sets up the Tauri application, including the main window and the Tokio runtime. The main function is async to manage the application's
     lifecycle.
   * Key Functions:
       * main(): The application entry point, running in an async context.
       * run(): Initializes the application, sets up the database, and defines the Tauri commands.

  src-tauri/src/commands.rs

   * Nature: [ASYNC] (Tauri Commands)
   * Purpose: This file defines the functions that can be called from the frontend (JavaScript). These functions act as the bridge between the async frontend and the backend logic.
   * Key Functions:
       * scan_and_store_files(path: String): `async`. This command orchestrates the file scanning process. It uses tokio::task::spawn_blocking to move the synchronous file system and database
         operations to a background thread pool, preventing the UI from freezing.
       * search_files(query: String, top_k: Option<usize>): `async`. This command handles search requests.
           * It calls the embed_and_store::get_embedding function, which is an async network call.
           * It then uses tokio::task::spawn_blocking to run the synchronous database search (hybrid_search_with_embedding) on a background thread.

  src-tauri/src/file_scanner.rs

   * Nature: [SYNC] (Blocking I/O)
   * Purpose: This module is responsible for all file system interactions, such as finding files and reading their content. These are blocking, synchronous operations.
   * Key Functions:
       * find_text_files_optimized(dir: P, max_file_size: Option<u64>): `SYNC`. This function walks the file system directory (walkdir) and collects file paths. This is a blocking operation.
       * read_file_content_optimized(path: &str, max_chars: Option<usize>): `SYNC`. This function reads the content of a file from the disk (fs::read_to_string), which is a blocking operation.
       * scan_and_store_files_optimized(...): `ASYNC` wrapper. This function orchestrates the process of finding files, reading them, generating embeddings, and storing them in the database. It uses
         tokio::spawn for concurrency when processing files.

  src-tauri/src/database/

   * Nature: [SYNC] (Blocking I/O)
   * Purpose: This module handles all interactions with the SQLite database using the rusqlite crate, which is a synchronous library.
   * Key Files & Functions:
       * database/mod.rs:
           * init_database(): `SYNC`. Creates the database file and runs migrations.
           * get_connection(): `SYNC`. Provides a thread-safe connection to the database.
       * database/schema.rs:
           * Contains the CREATE TABLE statements as &'static str constants. These are synchronous definitions.
       * database/search.rs:
           * search_similar_files(...): `SYNC`. Performs a vector search against the database.
           * search_files_fts(...): `SYNC`. Performs a full-text search against the database.
           * hybrid_search_with_embedding(...): `SYNC`. Combines vector and full-text search results.

  src-tauri/src/embed_and_store.rs

   * Nature: [ASYNC] (Network I/O) & [SYNC] (CPU-bound)
   * Purpose: This module handles communication with the external embedding service (like Ollama).
   * Key Functions:
       * get_embedding(text: &str): `ASYNC`. Makes a network request to the Ollama API to get embeddings for a given text. This is an I/O-bound operation, making it a perfect candidate for async.
       * get_batch_embeddings(texts: &[String]): `ASYNC`. Makes multiple network requests concurrently to get embeddings for a batch of texts.
       * normalize(v: Vec<f32>): `SYNC`. A CPU-bound function that normalizes a vector. It's fast and synchronous.

  Workflow Example: search_files

  Here is how the components work together for a search query:

   1. Frontend: The user types a query in the UI and clicks "Search". A JavaScript function calls invoke('search_files', { query: '...' }).

   2. `commands.rs` (`[ASYNC]`):
       * The search_files #[tauri::command] receives the request.
       * It awaits the embed_and_store::get_embedding() function to get the vector for the query. This is an async network call, so the main thread is not blocked.

   3. `embed_and_store.rs` (`[ASYNC]`):
       * The get_embedding() function sends an HTTP request to the Ollama API.
       * It awaits the response from the server.

   4. `commands.rs` (`[ASYNC]` -> `[SYNC]`):
       * Once the embedding is received, the search_files command needs to query the database.
       * Since rusqlite is synchronous, it wraps the database call in tokio::task::spawn_blocking.

   5. `database/search.rs` (`[SYNC]`):
       * Inside the spawn_blocking closure, the hybrid_search_with_embedding function is called.
       * This function executes synchronous rusqlite queries against the database on a background thread, so the UI remains responsive.

   6. `commands.rs` (`[SYNC]` -> `[ASYNC]`):
       * The spawn_blocking task finishes and returns the search results.
       * The search_files command awaits this result.
       * The final list of results is sent back to the frontend.

  This architecture ensures that the UI remains responsive by offloading all blocking I/O (database and file system access) to a dedicated thread pool, while using async/await for non-blocking I/O
  (network requests).