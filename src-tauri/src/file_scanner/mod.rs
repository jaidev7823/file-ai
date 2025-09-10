// Public API and main orchestration logic
pub mod content;
pub mod db;
pub mod discovery;
pub mod pipeline;
pub mod scoring;
pub mod types;
pub mod utils;
pub mod lancedb;

use crate::embed_and_store;
use db::insert_folder_metadata;
use discovery::{find_all_drive_files, should_exclude_path};
use pipeline::{build_embedding_chunks, prepare_files_for_processing, store_results};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager};
use tokio::runtime::Runtime;
use types::ScannedFile;
use utils::emit_scan_progress;
use walkdir::WalkDir;

static IS_SCANNING: AtomicBool = AtomicBool::new(false);

/// Scans files based on Phase 1 rules (included paths, extensions).
pub fn scan_and_store_files(
    db: &Connection,
    _dir: &str,
    max_chars: Option<usize>,
    _max_file_size: Option<u64>,
    app: tauri::AppHandle,
) -> Result<usize, String> {
    scan_and_store_files_with_mode(db, _dir, max_chars, _max_file_size, app, false)
}

/// Scans all drives for metadata (Phase 2).
pub fn scan_drives_metadata_only(db: &Connection, app: &AppHandle) -> Result<usize, String> {
    scan_and_store_files_with_mode(db, "", None, None, app.clone(), true)
}

/// Discovers files based on Phase 1 rules without indexing, for UI display.
pub fn discover_files_with_rules(
    db: &Connection,
    app: &AppHandle,
) -> Result<Vec<ScannedFile>, String> {
    let base_paths: Vec<String> = crate::database::rules::get_included_paths_sync(db)
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();
    let exclude_folders: Vec<String> = crate::database::rules::get_excluded_folder_sync(db)
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();

    let mut found_files = Vec::new();
    for base_path in &base_paths {
        let path_buf = PathBuf::from(base_path);
        if !path_buf.exists() {
            continue;
        }

        let walker = WalkDir::new(path_buf).into_iter();
        for entry_result in walker.filter_map(|e| e.ok()) {
            if entry_result.file_type().is_file() {
                if !should_exclude_path(entry_result.path(), &exclude_folders, &[], None) {
                    let path_str = entry_result.path().to_string_lossy().into_owned();
                    let (should_crawl, _) =
                        scoring::check_phase1_rules(db, &path_str)?;
                    found_files.push(ScannedFile {
                        path: path_str,
                        content_processed: should_crawl,
                    });
                }
            }
        }
    }
    Ok(found_files)
}

/// Main orchestrator for scanning files.
pub fn scan_and_store_files_with_mode(
    db: &Connection,
    _dir: &str,
    max_chars: Option<usize>,
    _max_file_size: Option<u64>,
    app: tauri::AppHandle,
    is_phase2: bool,
) -> Result<usize, String> {
    if IS_SCANNING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        emit_scan_progress(&app, 0, 0, "A scan is already in progress.", "error");
        return Err("A scan is already in progress.".to_string());
    }
    let _guard = ScanGuard;

    let rt = Runtime::new().map_err(|e| e.to_string())?;
    emit_scan_progress(&app, 0, 0, "Scanning for files and folders...", "scanning");
    println!("scanning for files and folders");

    let paths: Vec<String> = if is_phase2 {
        let scanned_files = find_all_drive_files(db, &app)?;
        scanned_files.iter().map(|sf| sf.path.clone()).collect()
    } else {
        let base_paths: Vec<String> = crate::database::rules::get_included_paths_sync(db)
            .map_err(|e| e.to_string())?
            .into_iter()
            .collect();
        let exclude_folders: Vec<String> = crate::database::rules::get_excluded_folder_sync(db)
            .map_err(|e| e.to_string())?
            .into_iter()
            .collect();

        let mut found_files = Vec::new();
        let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;

        for base_path in &base_paths {
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

                if entry.file_type().is_dir() {
                    if let Err(e) = insert_folder_metadata(&tx, entry.path()) {
                        eprintln!("Failed to store folder {}: {}", entry.path().display(), e);
                    }

                    if should_exclude_path(
                        entry.path(),
                        &exclude_folders,
                        &[],
                        None,
                    ) {
                        walker.skip_current_dir();
                        continue 'walker_loop;
                    }
                } else if entry.file_type().is_file() {
                    if should_exclude_path(
                        entry.path(),
                        &exclude_folders,
                        &[],
                        None,
                    ) {
                        continue 'walker_loop;
                    }
                    found_files.push(entry.path().to_string_lossy().into_owned());
                }
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        found_files
    };

    println!("Found {} files to process", paths.len());
    if paths.is_empty() {
        emit_scan_progress(&app, 0, 0, "No files found to process", "complete");
        return Ok(0);
    }

    let included_paths: Vec<String> = if is_phase2 {
        Vec::new()
    } else {
        crate::database::rules::get_included_paths_sync(db)
            .map_err(|e| e.to_string())?
            .into_iter()
            .collect()
    };
    let new_files =
        prepare_files_for_processing(db, &paths, &included_paths, is_phase2, max_chars, &rt, &app)?;
    println!("Identified {} new files for processing", new_files.len());
    if new_files.is_empty() {
        emit_scan_progress(
            &app,
            paths.len() as u64,
            paths.len() as u64,
            "No new files to process",
            "complete",
        );
        return Ok(0);
    }

    let (chunks, file_chunk_map) = build_embedding_chunks(&new_files);
    println!("Total {} text units for embedding", chunks.len());

    let embeddings = if chunks.is_empty() {
        Vec::new()
    } else {
        let app_clone = app.clone();
        emit_scan_progress(
            &app,
            0,
            chunks.len() as u64,
            "Generating embeddings...",
            "embedding",
        );
        println!("Generating embeddings...");
        embed_and_store::get_batch_embeddings_with_progress(&chunks, move |current, total| {
            app_clone
                .emit(
                    "scan_progress",
                    serde_json::json!({
                        "current": current, "total": total, "stage": "embedding",
                        "current_file": format!("Processing embedding {} of {}", current, total)
                    }),
                )
                .ok();
        })
        .map_err(|e| e.to_string())?
    };

    let inserted_count = rt.block_on(store_results(db, &new_files, &embeddings, &file_chunk_map, &app)).map_err(|e| e.to_string())?;

    emit_scan_progress(
        &app,
        0,
        0,
        "Calculating folder scores...",
        "scoring_folders",
    );
    crate::database::update_folder_scores(db).map_err(|e| e.to_string())?;

    emit_scan_progress(
        &app,
        inserted_count as u64,
        inserted_count as u64,
        format!("Completed! Processed {} new files", inserted_count),
        "complete",
    );

    Ok(inserted_count)
}

struct ScanGuard;
impl Drop for ScanGuard {
    fn drop(&mut self) {
        IS_SCANNING.store(false, Ordering::SeqCst);
    }
}
