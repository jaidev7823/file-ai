// Helper utilities (emit_scan_progress)
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager};

pub fn emit_scan_progress(
    app: &AppHandle,
    current: u64,
    total: u64,
    current_file: impl Into<String>,
    stage: &str,
) {
    let payload = serde_json::json!({
        "current": current, "total": total, "stage": stage, "current_file": current_file.into()
    });
    app.emit("scan_progress", &payload).ok();
    if let Some(win) = app.get_webview_window("main") {
        win.emit("scan_progress", &payload).ok();
    }
}

pub fn extract_drive(file_path: &str) -> String {
    Path::new(file_path)
        .components()
        .next()
        .and_then(|comp| match comp {
            std::path::Component::Prefix(prefix) => prefix.as_os_str().to_str().map(String::from),
            _ => None,
        })
        .unwrap_or_else(|| "unknown".to_string())
}
