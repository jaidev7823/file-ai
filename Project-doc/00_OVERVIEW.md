# 00_OVERVIEW.md

* what it is
We want to build ai-os but first we are creating a file ai for mvp goal is to get in y combinator or thiel fellow as a entreprenaur

* our vision
so what is file-ai
the desired goal of file ai for user to not stuck in finding file
user will do cmd+shift+p and a search box appear in center user destop user will search
"Where is my file about last meeting on qr?"
FILE-ai
here is your file
"where is my file which i saved last month about economics"
file-ai
here is your file

* how we plan to do it?
We will crawl all the docs file from user device except .venv and node module like unnecessary file
embedd all content in them by using ollma nomic embed
and save in user device with the help of sqlite-vec
and show them there file by implemnt vector semmantic and other

* what goes inside?
- **Core Framework:** Tauri (Rust backend, web frontend)
- **Backend:** Rust
- **Frontend:** React with TypeScript
- **Database:** SQLite with SeaORM for queries.
- **Vector Search:** `sqlite-vec` for efficient semantic search.
- **Embeddings:** A local Ollama model (e.g., `nomic-embed-text`) to create vector embeddings.
