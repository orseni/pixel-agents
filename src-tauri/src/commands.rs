use serde_json::Value;
use tauri::AppHandle;

use crate::events::{self, LayoutLoadedPayload, SettingsLoadedPayload};
use crate::persistence;
use crate::state::AppState;

// Preset layouts embedded at compile time
const PRESET_CORPORATE: &str =
    include_str!("../../webview-ui/public/assets/layout-corporate.json");
const PRESET_STARTUP: &str =
    include_str!("../../webview-ui/public/assets/layout-startup.json");
const PRESET_LIBRARY: &str =
    include_str!("../../webview-ui/public/assets/layout-cozy-library.json");
const PRESET_GARDEN: &str =
    include_str!("../../webview-ui/public/assets/layout-garden-office.json");

// Default layout for new users
const DEFAULT_LAYOUT_JSON: &str = PRESET_STARTUP;

fn get_preset(name: &str) -> Option<&'static str> {
    match name {
        "corporate" => Some(PRESET_CORPORATE),
        "startup" => Some(PRESET_STARTUP),
        "cozy-library" => Some(PRESET_LIBRARY),
        "garden-office" => Some(PRESET_GARDEN),
        _ => None,
    }
}

#[tauri::command]
pub async fn webview_ready(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    tracing::info!("Webview ready signal received");

    // Load settings
    let settings = persistence::read_settings();
    {
        let mut s = state.settings.write().await;
        *s = settings.clone();
    }

    // Emit settings
    events::emit_to_webview(
        &app,
        "settings-loaded",
        SettingsLoadedPayload {
            r#type: "settingsLoaded".to_string(),
            sound_enabled: settings.sound_enabled,
        },
    );

    // Load and emit layout (with bundled default as fallback)
    let default_layout: Option<Value> = serde_json::from_str(DEFAULT_LAYOUT_JSON).ok();
    let layout_result = persistence::load_layout(default_layout.as_ref());
    if let Some((layout, was_reset)) = layout_result {
        events::emit_to_webview(
            &app,
            "layout-loaded",
            LayoutLoadedPayload {
                r#type: "layoutLoaded".to_string(),
                layout,
                was_reset: if was_reset { Some(true) } else { None },
            },
        );
    }

    // Signal discovery loop to start
    {
        let mut ready = state.webview_ready.write().await;
        *ready = true;
    }

    Ok(())
}

#[tauri::command]
pub async fn save_layout(layout: Value) -> Result<(), String> {
    persistence::write_layout_to_file(&layout)
}

#[tauri::command]
pub async fn save_agent_seats(
    seats: Value,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut settings = state.settings.write().await;

    // Merge seats into settings
    if let Some(seats_obj) = seats.as_object() {
        for (key, val) in seats_obj {
            if let Ok(seat_info) = serde_json::from_value(val.clone()) {
                settings.agent_seats.insert(key.clone(), seat_info);
            }
        }
    }

    persistence::write_settings(&settings)
}

#[tauri::command]
pub async fn set_sound_enabled(
    enabled: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut settings = state.settings.write().await;
    settings.sound_enabled = enabled;
    persistence::write_settings(&settings)
}

#[tauri::command]
pub async fn export_layout(path: String) -> Result<(), String> {
    let layout = persistence::read_layout_from_file().ok_or("No layout to export")?;
    let json = serde_json::to_string_pretty(&layout)
        .map_err(|e| format!("Serialization error: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write file: {}", e))
}

#[tauri::command]
pub async fn import_layout(path: String, app: AppHandle) -> Result<(), String> {
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))?;
    let layout: Value =
        serde_json::from_str(&raw).map_err(|e| format!("Invalid JSON: {}", e))?;

    // Validate
    let version = layout.get("version").and_then(|v| v.as_i64());
    if version != Some(1) {
        return Err("Invalid layout: version must be 1".to_string());
    }
    if layout.get("tiles").and_then(|t| t.as_array()).is_none() {
        return Err("Invalid layout: missing tiles array".to_string());
    }

    // Write and emit
    persistence::write_layout_to_file(&layout)?;

    events::emit_to_webview(
        &app,
        "layout-loaded",
        LayoutLoadedPayload {
            r#type: "layoutLoaded".to_string(),
            layout,
            was_reset: None,
        },
    );

    Ok(())
}

#[tauri::command]
pub async fn apply_preset_layout(name: String, app: AppHandle) -> Result<(), String> {
    let json_str = get_preset(&name).ok_or_else(|| format!("Unknown preset: {}", name))?;
    let layout: Value =
        serde_json::from_str(json_str).map_err(|e| format!("Parse error: {}", e))?;

    persistence::write_layout_to_file(&layout)?;

    events::emit_to_webview(
        &app,
        "layout-loaded",
        LayoutLoadedPayload {
            r#type: "layoutLoaded".to_string(),
            layout,
            was_reset: None,
        },
    );

    Ok(())
}

#[tauri::command]
pub async fn open_sessions_folder() -> Result<(), String> {
    let dir = dirs::home_dir()
        .map(|h| h.join(".claude").join("projects"))
        .ok_or("Could not determine home directory")?;

    if !dir.exists() {
        return Err("Sessions directory does not exist".to_string());
    }

    open::that(&dir).map_err(|e| format!("Failed to open directory: {}", e))
}

#[tauri::command]
pub async fn close_agent(
    id: i64,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    // Remove agent
    {
        let mut agents = state.agents.write().await;
        agents.remove(&id);
    }

    // Remove from watched files
    {
        let mut watched = state.watched_files.write().await;
        watched.retain(|_, &mut agent_id| agent_id != id);
    }

    // Cancel timers
    {
        let mut wt = state.waiting_timers.write().await;
        if let Some(handle) = wt.remove(&id) {
            handle.abort();
        }
    }
    {
        let mut pt = state.permission_timers.write().await;
        if let Some(handle) = pt.remove(&id) {
            handle.abort();
        }
    }

    events::emit_to_webview(
        &app,
        "agent-closed",
        crate::events::AgentClosedPayload {
            r#type: "agentClosed".to_string(),
            id,
        },
    );

    Ok(())
}
