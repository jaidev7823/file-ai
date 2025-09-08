// File scoring and rule checking
use super::types::FileCategory;
use chrono::Duration;
use rusqlite::Connection;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

/// Calculates a file's score based on a set of rules.
/// The score is clamped between 0.0 and 10.0.
pub fn calculate_file_score(
    path_str: &str,
    metadata: &fs::Metadata,
    included_paths: &[String],
) -> f64 {
    let path = Path::new(path_str);
    let now = SystemTime::now();

    // 1. Base Category Score (0-4 points)
    let mut category_score = 0.0;
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let category = FileCategory::from_extension(extension);

    let path_lower = path_str.to_lowercase();
    if path_lower.contains("/log/") || path_lower.contains("/logs/") || path_lower.contains("/tmp/")
    {
        category_score = 0.5;
    } else {
        category_score = match category {
            FileCategory::Document => 4.0,
            FileCategory::Code => 3.0,
            FileCategory::Spreadsheet | FileCategory::Database => 3.0,
            FileCategory::Config => 2.0,
            FileCategory::Media | FileCategory::Archive | FileCategory::Binary => 1.0,
            FileCategory::Unknown => 0.0,
        };
    }

    // 2. Path Importance (0-3 points)
    let mut path_score = 0.0;
    if included_paths.iter().any(|p| path_str.starts_with(p)) {
        path_score = 3.0;
    }

    // 3. Recency Score (0-2 points)
    let recency_score = if let Ok(modified) = metadata.modified() {
        let age = now.duration_since(modified).unwrap_or_default();
        if age < Duration::days(7).to_std().unwrap_or_default() {
            2.0
        } else if age < Duration::days(30).to_std().unwrap_or_default() {
            1.5
        } else if age < Duration::days(182).to_std().unwrap_or_default() {
            1.0
        } else {
            0.0
        }
    } else {
        0.0
    };

    // 4. File Size Penalty (deductions)
    let size_penalty = {
        let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
        if size_mb > 500.0 {
            -1.0
        } else if size_mb > 100.0 {
            -0.5
        } else {
            0.0
        }
    };

    // 5. Special Bonus (max +1 point)
    let bonus_score = if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        let name_lower = file_name.to_lowercase();
        let mut bonus: f64 = 0.0;
        if name_lower.contains("project") {
            bonus += 0.5;
        }
        if name_lower.contains("report") {
            bonus += 0.5;
        }
        if name_lower.contains("final") {
            bonus += 0.5;
        }
        if name_lower.contains("db") {
            bonus += 0.5;
        }
        bonus.min(1.0)
    } else {
        0.0
    };

    let total_score = category_score + path_score + recency_score + size_penalty + bonus_score;
    let clamped = total_score.max(0.0f64).min(10.0f64);
    (clamped * 10.0).round() / 10.0
}

/// Phase 1: Check if file should have content processed based on included/excluded paths.
pub fn check_phase1_rules(db: &Connection, file_path: &str) -> Result<(bool, bool), String> {
    let include_paths =
        crate::database::rules::get_included_paths_sync(db).map_err(|e| e.to_string())?;
    let exclude_folders =
        crate::database::rules::get_excluded_folder_sync(db).map_err(|e| e.to_string())?;
    let include_exts: HashSet<String> = crate::database::rules::get_included_extensions_sync(db)
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();
    let path_obj = Path::new(file_path);

    if !include_paths.iter().any(|p| file_path.starts_with(p)) {
        return Ok((false, false));
    }

    let is_in_excluded_folder = path_obj.ancestors().any(|ancestor| {
        ancestor
            .file_name()
            .and_then(|n| n.to_str())
            .map_or(false, |name| {
                exclude_folders
                    .iter()
                    .any(|ex| ex.eq_ignore_ascii_case(name))
            })
    });

    if is_in_excluded_folder {
        return Ok((false, true));
    }

    let extension = path_obj.extension().and_then(|s| s.to_str()).unwrap_or("");

    if !include_exts.is_empty()
        && !include_exts
            .iter()
            .any(|inc| inc.eq_ignore_ascii_case(extension))
    {
        return Ok((false, false));
    }

    Ok((true, false))
}
