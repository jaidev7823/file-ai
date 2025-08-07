// src/shortcuts.rs
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::commands;

pub fn register_global_shortcuts(app: &AppHandle) -> Result<(), String> {
    let ctrl_shift_p = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyP);
    let app_handle = app.clone();

    app.plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |_app, shortcut, event| {
                if shortcut == &ctrl_shift_p {
                    match event.state() {
                        ShortcutState::Pressed => {
                            let app_handle = app_handle.clone();
                            println!("Ctrl+Shift+P Pressed!");
                            tauri::async_runtime::spawn(async move {
                                if let Err(e) = commands::toggle_search_window(app_handle).await {
                                    eprintln!("Failed to toggle search window: {}", e);
                                }
                            });
                        }
                        ShortcutState::Released => {
                            println!("Ctrl+Shift+P Released!");
                        }
                    }
                }
            })
            .build(),
    )
    .map_err(|e| e.to_string())?;

    app.global_shortcut()
        .register(ctrl_shift_p)
        .map_err(|e| format!("Failed to register shortcut: {}", e))?;

    Ok(())
}
