# Global Spotlight Search Feature

## Overview
A **system-wide** Mac Spotlight-style search interface that opens with `Ctrl+Shift+P` keyboard shortcut. The search appears as a global overlay that works even when the main app is minimized or in the background.

## 🌟 Key Features
- **🌍 Global System-Wide Access**: Works from anywhere on your system, not just inside the app
- **⚡ Instant Global Shortcut**: Press `Ctrl+Shift+P` from any application
- **🎯 Always-On-Top Overlay**: Search window appears centered on screen above all other windows
- **🔍 Real-time Search**: Debounced search with 300ms delay for smooth performance
- **🎨 Modern UI**: Clean design with backdrop blur and transparency
- **⌨️ Keyboard Navigation**: ESC to close, auto-focus on open
- **📁 Multiple Result Types**: Files, folders, and content with appropriate icons
- **🔄 Loading States**: Spinner, error handling, and empty states

## 🏗️ Architecture

### Multi-Window Setup
- **Main Window**: Your primary app interface (can run in background)
- **Search Window**: Dedicated overlay window for search functionality
  - Size: 600x400px
  - Always on top, centered, transparent background
  - No decorations (frameless)
  - Hidden by default, shown on shortcut

### Global Shortcut Registration
Implemented in Rust backend using `tauri-plugin-global-shortcut`:
```rust
app.global_shortcut().register("Ctrl+Shift+P", move || {
    // Toggle search window visibility
});
```

## 📁 File Structure

### Frontend Components
- `src/pages/SearchPage.tsx` - Dedicated search interface for search window
- `src/WindowRouter.tsx` - Routes different windows to appropriate components
- `src/hooks/useSearch.ts` - Search logic and state management
- `src/components/ui/input.tsx` - Input component for search

### Backend (Rust)
- `src-tauri/src/lib.rs` - Global shortcut registration and window management
- `src-tauri/tauri.conf.json` - Multi-window configuration

## ⚙️ Configuration

### Tauri Window Config
```json
{
  "label": "search",
  "width": 600,
  "height": 400,
  "resizable": false,
  "transparent": true,
  "decorations": false,
  "visible": false,
  "alwaysOnTop": true,
  "skipTaskbar": true,
  "center": true,
  "focus": true
}
```

### Global Shortcut Commands
- `toggle_search_window()` - Shows/hides search window
- `hide_search_window()` - Hides search window

## 🚀 Implementation Status
✅ **Multi-window architecture**  
✅ **Global system shortcut (Ctrl+Shift+P)**  
✅ **Always-on-top search overlay**  
✅ **Window routing system**  
✅ **Search UI with transparency**  
✅ **Keyboard navigation (ESC to close)**  
🔄 **TODO**: Connect to actual search backend  
🔄 **TODO**: File opening functionality  

## 🎯 How It Works

1. **Background Operation**: Main app can run minimized
2. **Global Trigger**: User presses `Ctrl+Shift+P` from anywhere
3. **Instant Overlay**: Search window appears centered on screen
4. **System-Wide Search**: Search through files regardless of current app focus
5. **Quick Dismiss**: ESC or click X to hide, app continues running in background

## 🔧 Next Steps

1. **Backend Integration**: 
   ```typescript
   // Replace mock search in useSearch.ts
   const searchResults = await invoke('search_files', { query });
   ```

2. **File Opening**: Add file opening logic when results are clicked

3. **Advanced Features**:
   - Arrow key navigation through results
   - Recent searches
   - Search filters and scopes
   - Keyboard shortcuts for actions

## 💡 Usage Benefits

- **Always Available**: Search from any app, any time
- **Non-Intrusive**: Main app can stay hidden/minimized
- **Fast Access**: No need to switch windows or find the app
- **System Integration**: Feels like a native OS feature
- **Productivity Boost**: Instant access to your files from anywhere

This implementation transforms your app into a true system utility, similar to macOS Spotlight or Windows PowerToys Run!