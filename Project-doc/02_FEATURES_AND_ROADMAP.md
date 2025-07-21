# Features and Roadmap

This document tracks the features of File-AI, from the initial MVP to future goals.

## ✅ MVP (Minimum Viable Product)

This is the core feature set required to achieve the vision outlined in `00_OVERVIEW.md` and to have a compelling product for Y Combinator or the Thiel Fellowship.

-   [ ] **Global Shortcut:** User can press a key combination (e.g., `Cmd+Shift+P`) anywhere in their OS to bring up the File-AI search window.
-   [x] **File Scanning:** The app scans the user's home directory for text-based files on startup.
-   [x] **Embedding Generation:** File content is converted into vector embeddings using a local Ollama model (`nomic-embed-text`).
-   [x] **Local Storage:** File metadata (name, path) and its embedding are stored in a local SQLite database.
-   [ ] **Search UI:** A clean, centered search box appears when the global shortcut is pressed.
-   [ ] **Semantic Search:** User can type a natural language query (e.g., "notes from the marketing meeting"). The app converts this query to an embedding and finds the most similar files in the database.
-   [ ] **Display Results:** The UI displays a list of the most relevant files based on the search.
-   [ ] **Open Files:** User can click on a search result to open the file in their default application.

---

## 🚀 Future Ideas (Post-MVP)

Once the MVP is solid, we can explore these more advanced features to build a full-fledged AI-OS.

### Tier 1: Core Improvements
-   **Real-time Indexing:** Use a file system watcher to automatically update the database when files are created, modified, or deleted.
-   **Content Preview:** Show a preview of the file's content directly in the search results.
-   **Advanced Filters:** Allow users to filter search results by file type, date range, or folder.
-   **Settings Panel:** Create a UI where users can:
    -   Configure which folders to include or exclude from scanning.
    -   Choose a different Ollama embedding model.
    -   Manage the database (e.g., re-scan all files).

### Tier 2: Deeper AI Integration
-   **Conversational Search:** Use a local LLM (e.g., Llama 3, Phi-3) to have a conversation about the files.
    -   *User:* "Summarize my notes about the Q3 budget."
    -   *File-AI:* "Your notes mention a 15% increase in the marketing budget and a new allocation for R&D..."
-   **Action Execution:** Allow the AI to perform actions based on file content.
    -   *User:* "Create a new to-do list from the action items in my last project update."
    -   *File-AI:* Creates a new file `todo.md` with the extracted items.

### Tier 3: The AI-OS Vision
-   **Cross-Application Integration:** Connect with other applications (e.g., calendar, email, browser history) to provide a truly unified search experience.
-   **Proactive Assistance:** The AI could proactively surface relevant files based on the user's current context (e.g., upcoming meetings, current active application).
