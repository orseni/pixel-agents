use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tokio::time::{interval, Duration};

use crate::constants::FILE_WATCHER_POLL_INTERVAL_MS;
use crate::line_reader;
use crate::state::AppState;

/// Start watching a JSONL file for changes using polling.
/// We use polling instead of notify crate for simplicity and reliability
/// (fs::watch is unreliable on macOS, same pattern as the VS Code extension).
pub async fn start_watching(app: AppHandle, agent_id: i64, jsonl_path: PathBuf) {
    let state = app.state::<AppState>();
    let mut tick = interval(Duration::from_millis(FILE_WATCHER_POLL_INTERVAL_MS));
    let mut last_size: u64 = 0;

    // Get initial file size to skip existing content (seed)
    if let Ok(metadata) = std::fs::metadata(&jsonl_path) {
        last_size = metadata.len();
        // Set the agent's file offset to current size (skip existing lines)
        let mut agents = state.agents.write().await;
        if let Some(agent) = agents.get_mut(&agent_id) {
            agent.file_offset = last_size;
        }
    }

    loop {
        tick.tick().await;

        // Check if agent still exists
        {
            let agents = state.agents.read().await;
            if !agents.contains_key(&agent_id) {
                tracing::info!("Agent {} removed, stopping file watcher", agent_id);
                return;
            }
        }

        // Check file size
        let current_size = match std::fs::metadata(&jsonl_path) {
            Ok(m) => m.len(),
            Err(_) => {
                // File may have been deleted — check if agent was removed
                let agents = state.agents.read().await;
                if !agents.contains_key(&agent_id) {
                    return;
                }
                continue;
            }
        };

        if current_size > last_size {
            last_size = current_size;
            line_reader::read_new_lines(&app, agent_id).await;
        }
    }
}
