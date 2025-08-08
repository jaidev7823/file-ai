# File-AI

Find your files without searching. Just ask.

File-AI is a desktop application that allows you to find files on your computer using natural language. Instead of digging through folders, simply press `Cmd+Shift+P`, type what you're looking for (e.g., "my presentation on Q3 earnings" or "the document I saved last week about economics"), and File-AI will find it for you.

Our MVP goal is to create a powerful, focused tool with the potential to be used by some users. The long-term vision is to build the foundation for a future AI-OS.

## ✨ Features (MVP)

-   [x] **Global Shortcut:** Press `Cmd+Shift+P` anywhere to open the search window.
-   [x] **File Scanning:** Scans your home directory for text-based files.
-   [x] **Embedding Generation:** Converts file content into vector embeddings using a local Ollama model (`nomic-embed-text`).
-   [x] **Local First:** All your data and embeddings are stored locally in a SQLite database.
-   [x] **Semantic Search:** Understands natural language queries to find the most relevant files.
-   [x] **Clean UI:** A simple, centered search box for a distraction-free experience.
-   [x] **Display Results:** Shows a list of relevant files based on your search.
-   [x] **Open Files:** Click a result to open the file in its default application.
-   [x] **Include Exclude Folder:** Add configuration feature for which file, extension to ignore or allow.
-   [ ] **Meta Data Search:** . Add feature for if user want to get data like file from last month or lil bit understand what is meta data keyword already implemented but update

## 🛠️ Tech Stack & Architecture

File-AI is built with [Tauri](https://tauri.app/), combining a Rust backend with a web-based frontend.

-   **Core Framework:** Tauri
-   **Backend:** Rust
-   **Frontend:** React with TypeScript & shadcn/ui
-   **Database:** SQLite
-   **Vector Search:** `sqlite-vec` for efficient semantic search.
-   **Embeddings:** A local Ollama model (e.g., `nomic-embed-text`).

### Architecture Overview

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

-   **Backend (`src-tauri`):** Handles file system scanning, embedding generation, and vector search. It exposes commands to the frontend.
-   **Frontend (`src`):** The UI the user interacts with, built in React. It calls the Rust backend to perform actions and display data.

## 🗺️ Roadmap (Post-MVP)

### Tier 1: Core Improvements
-   **Real-time Indexing:** Watch the file system for changes.
-   **Content Preview:** Show file previews in the search results.
-   **Advanced Filters:** Filter by file type, date, etc.
-   **Settings Panel:** Configure folders, models, and the database.

### Tier 2: Deeper AI Integration
-   **Conversational Search:** Use a local LLM to "talk" to your files.
-   **Action Execution:** Allow the AI to perform tasks based on file content (e.g., "create a summary of this doc").

### Tier 3: The AI-OS Vision
-   **Cross-Application Integration:** Connect with calendars, email, browsers, etc.
-   **Proactive Assistance:** Surface relevant files based on your current context.

## 🔧 Development Status

The project is currently in the MVP development phase.

**Recent Activity:** I am currently improving on search ranking.

## 🚀 Getting Started

1.  **Install dependencies:**
    ```bash
    npm install
    ```
2.  **Run the application in development mode:**
    ```bash
    npm run tauri dev
    ```

---
*This README was generated based on the project's internal documentation.*