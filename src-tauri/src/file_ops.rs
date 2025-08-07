// src/file.rs

use std::process::Command;

pub fn open_file_impl(file_path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", &file_path])
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&file_path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&file_path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }

    Ok(())
}

pub fn open_file_with_impl(file_path: String, application: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new(&application)
            .arg(&file_path)
            .spawn()
            .map_err(|e| format!("Failed to open file with {}: {}", application, e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", &application, &file_path])
            .spawn()
            .map_err(|e| format!("Failed to open file with {}: {}", application, e))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new(&application)
            .arg(&file_path)
            .spawn()
            .map_err(|e| format!("Failed to open file with {}: {}", application, e))?;
    }

    Ok(())
}

pub fn show_file_in_explorer_impl(file_path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .args(["/select,", &file_path])
            .spawn()
            .map_err(|e| format!("Failed to show file in explorer: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-R", &file_path])
            .spawn()
            .map_err(|e| format!("Failed to show file in finder: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(parent) = std::path::Path::new(&file_path).parent() {
            Command::new("xdg-open")
                .arg(parent)
                .spawn()
                .map_err(|e| format!("Failed to show file in file manager: {}", e))?;
        } else {
            return Err("Could not determine parent directory".to_string());
        }
    }

    Ok(())
}
