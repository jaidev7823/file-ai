# Project Architecture

This document outlines the architecture of File-AI, a Tauri-based desktop application.

## 1. High-Level Overview

File-AI uses [Tauri](https://tauri.app/), which allows us to build a desktop application with a Rust backend and a web-based frontend.

-   **Backend (`src-tauri`):** Written in Rust. It handles all the heavy lifting: file system scanning, database management, embedding generation, and searching. It exposes specific functions (commands) to the frontend.
-   **Frontend (`src`):** Written in React and TypeScript. This is the user interface (UI) that the user sees and interacts with. It calls the Rust backend to get data and perform actions.

```
+--------------------------------+
|       Frontend (React)         |
|  (Search Box, Results List)    |
+--------------------------------+
          |         ^
          | (API)   | (Data)
          v         |
+--------------------------------+
|        Backend (Rust)          |
| (File Scanner, DB, Search)     |
+--------------------------------+
```

## 2. Backend (`src-tauri`)

The backend is the core engine of the application.

-   `main.rs` & `lib.rs`: The entry point of the Rust application. It sets up the Tauri application, initializes the database, and registers the commands that the frontend can call.
-   `commands.rs`: Contains all the `#[tauri::command]` functions. These are the only Rust functions that the frontend can call directly. This is our API layer.
-   `file_scanner.rs`: Responsible for finding all relevant text files on the user's computer. It knows which folders to skip (like `node_modules`).
-   `database.rs`: Manages the connection to the local SQLite database.
-   `entities/`: Defines the structure of our database tables using SeaORM. For example, `file.rs` maps to the `file` table.
-   `migration/`: Contains the instructions for creating and updating our database tables. This is how we manage changes to the database schema over time.
-   `vss.rs`: Implements the vector search functionality using the `sqlite-vec` extension. It handles inserting embeddings and searching for similar ones.
-   `embeddings.rs`: (Assumed Purpose) Will contain the logic for taking text content and converting it into vector embeddings using a local Ollama model.

## 3. Frontend (`src`)

The frontend is the visual part of the application.

-   `main.tsx`: The entry point for the React application.
-   `App.tsx`: The main component that holds the entire UI.
-   `components/`: Contains reusable UI pieces.
    -   `SearchBox.tsx`: The input field where the user types their search query.
    -   `ResultsList.tsx`: The component that displays the list of files found.
    -   `ui/`: Smaller, general-purpose UI components (buttons, cards, etc.) from `shadcn/ui`.
-   `lib/api.ts`: A helper file that makes it easy to call the Rust commands from our TypeScript code. It uses Tauri's `invoke` function.
