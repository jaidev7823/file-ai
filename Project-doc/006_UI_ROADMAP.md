Phase 1: UI Foundations
Task 1.1 - Create React UI (Tauri)

 Setup project with TailwindCSS + React
 Add global shortcut handler (Ctrl+Shift+P)
 Build a modal/command-palette UI like Raycast/Spotlight

Task 1.2 - Implement Search Box

 Input field with debounce (for semantic search)
 Call backend API with vector query
 Display results in a horizontal scroll list
 On hover, show buttons:
 Open
 Rename
 Open with…
 Delete
 Copy Path

Phase 2: Settings Interface
Task 2.1 - Build Settings Page

 UI with folder-picker
 List of drives/folders to exclude
 Save preferences in config file (e.g., ~/.file-ai/config.json)
 Validate access rights and disk I/O warnings

Task 2.2 - Config Sync

 Sync with crawler to respect settings
 Exclude common folders by default:

node_modules

.venv / venv

C:/Program Files, etc.

Phase 3: Usability Polish
Task 3.1 - Preview Features

 File icons based on type (PDF, TXT, DOCX, etc.)
 Tooltip preview (first few lines)
 Highlight match in filename / content

Task 3.2 - Hotkeys + Shortcuts

 ESC to close
 ↑ ↓ navigation
 Enter to open
 Ctrl+Enter to open in other app

Phase 4: Advanced Features (Optional)
Task 4.1 - Quick Actions / Commands

 Run commands on files: convert, summarize, send to email, etc.

Task 4.2 - Plugin API

 Let developers build custom actions (with security model)

Task 4.3 - Caching & Performance

 Smart caching of frequent queries
 Background sync
 Progress bar for indexing

