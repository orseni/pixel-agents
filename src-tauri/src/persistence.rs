use std::fs;
use std::path::PathBuf;

use crate::constants::{LAYOUT_FILE_DIR, LAYOUT_FILE_NAME, LAYOUT_REVISION_KEY, SETTINGS_FILE_NAME};
use crate::models::AppSettings;

fn get_pixel_agents_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(LAYOUT_FILE_DIR))
}

pub fn get_layout_file_path() -> Option<PathBuf> {
    get_pixel_agents_dir().map(|d| d.join(LAYOUT_FILE_NAME))
}

fn get_settings_file_path() -> Option<PathBuf> {
    get_pixel_agents_dir().map(|d| d.join(SETTINGS_FILE_NAME))
}

/// Read layout from ~/.pixel-agents/layout.json
pub fn read_layout_from_file() -> Option<serde_json::Value> {
    let path = get_layout_file_path()?;
    let raw = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Atomic write: write to .tmp then rename
pub fn write_layout_to_file(layout: &serde_json::Value) -> Result<(), String> {
    let path = get_layout_file_path().ok_or("Could not determine layout file path")?;
    let dir = path.parent().ok_or("No parent directory")?;

    fs::create_dir_all(dir).map_err(|e| format!("Failed to create directory: {}", e))?;

    let json =
        serde_json::to_string_pretty(layout).map_err(|e| format!("Serialization error: {}", e))?;

    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, &json).map_err(|e| format!("Failed to write tmp file: {}", e))?;
    fs::rename(&tmp_path, &path).map_err(|e| format!("Failed to rename tmp file: {}", e))?;

    Ok(())
}

/// Load layout, applying revision check against bundled default
pub fn load_layout(
    default_layout: Option<&serde_json::Value>,
) -> Option<(serde_json::Value, bool)> {
    let from_file = read_layout_from_file();

    if let Some(existing) = from_file {
        let file_revision = existing
            .get(LAYOUT_REVISION_KEY)
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let default_revision = default_layout
            .and_then(|d| d.get(LAYOUT_REVISION_KEY))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        if default_revision > file_revision {
            if let Some(def) = default_layout {
                tracing::info!(
                    "Layout revision outdated ({} < {}), resetting to default",
                    file_revision,
                    default_revision
                );
                let _ = write_layout_to_file(def);
                return Some((def.clone(), true));
            }
        }

        tracing::info!("Layout loaded from file");
        return Some((existing, false));
    }

    // Use bundled default
    if let Some(def) = default_layout {
        tracing::info!("Writing default layout to file");
        let _ = write_layout_to_file(def);
        return Some((def.clone(), false));
    }

    None
}

/// Read settings from ~/.pixel-agents/settings.json
pub fn read_settings() -> AppSettings {
    let Some(path) = get_settings_file_path() else {
        return AppSettings::default();
    };
    let Ok(raw) = fs::read_to_string(&path) else {
        return AppSettings::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

/// Write settings atomically
pub fn write_settings(settings: &AppSettings) -> Result<(), String> {
    let path = get_settings_file_path().ok_or("Could not determine settings file path")?;
    let dir = path.parent().ok_or("No parent directory")?;

    fs::create_dir_all(dir).map_err(|e| format!("Failed to create directory: {}", e))?;

    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Serialization error: {}", e))?;

    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, &json).map_err(|e| format!("Failed to write tmp file: {}", e))?;
    fs::rename(&tmp_path, &path).map_err(|e| format!("Failed to rename tmp file: {}", e))?;

    Ok(())
}
