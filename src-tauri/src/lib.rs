mod file_scanner;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn scan_text_files(path: String) -> Vec<String> {
    file_scanner::find_text_files(path)
}

#[tauri::command]
fn read_text_files(paths: Vec<String>, max_chars: Option<usize>) -> Vec<file_scanner::FileContent> {
    file_scanner::read_files_content(&paths, max_chars)
}

#[tauri::command]
fn get_file_content(path: String, max_chars: Option<usize>) -> Option<file_scanner::FileContent> {
    let results = file_scanner::read_files_content(&[path], max_chars);
    results.into_iter().next()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            scan_text_files,
            read_text_files,
            get_file_content
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

