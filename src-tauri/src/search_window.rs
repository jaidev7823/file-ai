use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder, Manager};

pub async fn toggle_search_window_impl(app: AppHandle) -> Result<(), String> {
    println!("Toggle search window called");

    if let Some(search_window) = app.get_webview_window("search") {
        println!("Search window found");
        let is_visible = search_window.is_visible().map_err(|e| e.to_string())?;
        println!("Search window visible: {}", is_visible);

        if is_visible {
            println!("Hiding search window");
            search_window.hide().map_err(|e| e.to_string())?;
        } else {
            println!("Showing search window");
            search_window.show().map_err(|e| e.to_string())?;
            search_window.set_focus().map_err(|e| e.to_string())?;
        }
    } else {
        println!("Search window not found! Creating it...");

        let search_window = WebviewWindowBuilder::new(
            &app,
            "search",
            WebviewUrl::App("index.html".into()),
        )
        .title("Search")
        .inner_size(600.0, 400.0)
        .resizable(false)
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .center()
        .focused(true)
        .build()
        .map_err(|e| e.to_string())?;

        println!("Search window created successfully");
        search_window.show().map_err(|e| e.to_string())?;
        search_window.set_focus().map_err(|e| e.to_string())?;
    }

    Ok(())
}
