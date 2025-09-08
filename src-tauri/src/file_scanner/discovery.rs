// File finding logic (find_files, discover_drives)
use super::db::insert_folder_metadata;
use super::scoring::check_phase1_rules;
use super::types::ScannedFile;
use super::utils::emit_scan_progress;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use walkdir::{DirEntry, WalkDir};

pub struct ScanConfig<'a> {
    pub base_paths: Vec<String>,
    pub include_exts: Vec<String>,
    pub exclude_folders: &'a [String],
    pub exclude_paths: &'a [String],
}

/// Generic file discovery function.
pub fn find_files(config: ScanConfig, app: &AppHandle, progress_stage: &str) -> Vec<String> {
    let mut found_files = Vec::new();
    let mut scanned_count = 0;

    for base_path in &config.base_paths {
        let path_buf = PathBuf::from(base_path);
        if !path_buf.exists() {
            continue;
        }

        let walker = WalkDir::new(path_buf).into_iter();
        let filtered_walker = walker.filter_entry(|e| !is_excluded_dir(e, &config));

        for entry in filtered_walker.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            if should_exclude_path(
                path,
                config.exclude_folders,
                config.exclude_paths,
                Some(&config.include_exts),
            ) {
                continue;
            }

            let path_str = path.to_string_lossy().to_string();
            found_files.push(path_str.clone());
            scanned_count += 1;
            if scanned_count % 1000 == 0 {
                emit_scan_progress(app, scanned_count, 0, &path_str, progress_stage);
            }
        }
    }
    found_files
}

fn is_excluded_dir(entry: &DirEntry, config: &ScanConfig) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }

    should_exclude_path(
        entry.path(),
        config.exclude_folders,
        config.exclude_paths,
        None, // Don't check extensions for directories
    )
}

pub fn should_exclude_path(
    path: &Path,
    exclude_folders: &[String],
    exclude_paths: &[String],
    include_exts: Option<&[String]>,
) -> bool {
    let path_str = path.to_string_lossy();

    if exclude_paths.iter().any(|p| {
        let exclude_path = p.trim();
        if exclude_path.is_empty() {
            false
        } else {
            path_str.starts_with(exclude_path)
        }
    }) {
        return true;
    }

    if path.ancestors().any(|ancestor| {
        ancestor
            .file_name()
            .and_then(|n| n.to_str())
            .map_or(false, |name| {
                let lname = name.to_lowercase();
                if lname.starts_with('.') {
                    return true;
                }
                exclude_folders.iter().any(|ex| {
                    let exclude_folder = ex.trim();
                    !exclude_folder.is_empty() && ex.eq_ignore_ascii_case(name)
                })
            })
    }) {
        return true;
    }

    if let Some(include_exts) = include_exts {
        if !include_exts.is_empty() {
            let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if !include_exts
                .iter()
                .any(|ext| ext.eq_ignore_ascii_case(extension))
            {
                return true;
            }
        }
    }

    false
}

/// Phase 1: Find text files based on user-defined rules.
pub fn find_text_files(conn: &Connection, app: &AppHandle) -> Result<Vec<ScannedFile>, String> {
    println!("--- RUNNING find_text_files ---");
    emit_scan_progress(app, 0, 0, "", "scanning");
    let include_exts: Vec<String> = crate::database::rules::get_included_extensions_sync(conn)
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();
    let exclude_folders: Vec<String> = crate::database::rules::get_excluded_folder_sync(conn)
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();
    let base_paths: Vec<String> = crate::database::rules::get_included_paths_sync(conn)
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();
    let config = ScanConfig {
        base_paths,
        include_exts,
        exclude_folders: &exclude_folders,
        exclude_paths: &[],
    };
    let file_paths = find_files(config, app, "scanning");

    let mut scanned_files = Vec::new();
    for path in file_paths {
        let (should_crawl, _) = check_phase1_rules(conn, &path)?;
        scanned_files.push(ScannedFile {
            path,
            content_processed: should_crawl,
        });
    }

    emit_scan_progress(
        app,
        scanned_files.len() as u64,
        scanned_files.len() as u64,
        "",
        "complete",
    );
    Ok(scanned_files)
}

/// Phase 2: Find all files across all drives (metadata only).
pub fn find_all_drive_files(
    conn: &Connection,
    app: &AppHandle,
) -> Result<Vec<ScannedFile>, String> {
    let files = find_all_drive_files_internal(conn, app)?;
    Ok(files
        .into_iter()
        .map(|path| ScannedFile {
            path,
            content_processed: false,
        })
        .collect())
}

fn find_all_drive_files_internal(
    conn: &Connection,
    app: &AppHandle,
) -> Result<Vec<String>, String> {
    emit_scan_progress(app, 0, 0, "", "phase2_discovery");

    let exclude_folders: Vec<String> = crate::database::rules::get_excluded_folder_sync(conn)
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();

    let exclude_paths: Vec<String> = crate::database::rules::get_excluded_paths_sync(conn)
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();

    let config = ScanConfig {
        base_paths: discover_drives(),
        include_exts: Vec::new(),
        exclude_folders: &exclude_folders,
        exclude_paths: &exclude_paths,
    };

    let mut found_files = Vec::new();
    let mut scanned_count = 0;

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    for base_path in &config.base_paths {
        let path_buf = PathBuf::from(base_path);
        if !path_buf.exists() {
            continue;
        }

        let mut walker = WalkDir::new(path_buf).into_iter();
        'walker_loop: while let Some(entry_result) = walker.next() {
            let entry = match entry_result {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            let path = entry.path();

            if path.is_dir() {
                if let Err(e) = insert_folder_metadata(&tx, path) {
                    eprintln!("Failed to store folder {}: {}", path.display(), e);
                }

                if should_exclude_path(path, config.exclude_folders, config.exclude_paths, None) {
                    walker.skip_current_dir();
                    continue 'walker_loop;
                }
                continue 'walker_loop;
            }

            if path.is_file() {
                if should_exclude_path(path, config.exclude_folders, config.exclude_paths, None) {
                    continue 'walker_loop;
                }

                let path_str = path.to_string_lossy().to_string();
                found_files.push(path_str.clone());
                scanned_count += 1;
                if scanned_count % 1000 == 0 {
                    emit_scan_progress(app, scanned_count, 0, &path_str, "phase2_scanning");
                }
            }
        }
    }

    tx.commit().map_err(|e| e.to_string())?;

    emit_scan_progress(
        app,
        found_files.len() as u64,
        found_files.len() as u64,
        "",
        "phase2_scan_complete",
    );

    Ok(found_files)
}

#[cfg(target_os = "windows")]
pub fn discover_drives() -> Vec<String> {
    // Original implementation (commented for reference):

    (b'A'..=b'Z')
        .filter_map(|drive_letter| {
            let path_str = format!("{}:\\", drive_letter as char);
            Path::new(&path_str).exists().then_some(path_str)
        })
        .collect()

    // Test path instead of discovering drives
    // vec![r"C:\Users\Jai Mishra\Downloads\drive-test".to_string()]
}
