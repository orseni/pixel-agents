use std::io::{Read, Seek, SeekFrom};
use tauri::{AppHandle, Manager};

use crate::events::{self, AgentToolPermissionClearPayload};
use crate::state::AppState;
use crate::timer_manager;
use crate::transcript_parser;

/// Read new lines from a JSONL file incrementally.
/// Maintains partial line buffering for mid-write reads.
pub async fn read_new_lines(app: &AppHandle, agent_id: i64) {
    let state = app.state::<AppState>();

    // Get file path and current offset
    let (jsonl_path, file_offset, line_buffer) = {
        let agents = state.agents.read().await;
        let Some(agent) = agents.get(&agent_id) else {
            return;
        };
        (
            agent.jsonl_file.clone(),
            agent.file_offset,
            agent.line_buffer.clone(),
        )
    };

    // Read new data from file
    let Ok(metadata) = std::fs::metadata(&jsonl_path) else {
        return;
    };
    let file_size = metadata.len();
    if file_size <= file_offset {
        return;
    }

    let bytes_to_read = (file_size - file_offset) as usize;
    let mut buf = vec![0u8; bytes_to_read];

    let Ok(mut file) = std::fs::File::open(&jsonl_path) else {
        return;
    };
    if file.seek(SeekFrom::Start(file_offset)).is_err() {
        return;
    }
    if file.read_exact(&mut buf).is_err() {
        return;
    }

    // Update offset
    {
        let mut agents = state.agents.write().await;
        if let Some(agent) = agents.get_mut(&agent_id) {
            agent.file_offset = file_size;
        }
    }

    // Parse lines with partial line buffering
    let text = format!("{}{}", line_buffer, String::from_utf8_lossy(&buf));
    let mut lines: Vec<&str> = text.split('\n').collect();
    let remainder = lines.pop().unwrap_or("").to_string();

    // Store remainder as line buffer
    {
        let mut agents = state.agents.write().await;
        if let Some(agent) = agents.get_mut(&agent_id) {
            agent.line_buffer = remainder;
        }
    }

    let has_lines = lines.iter().any(|l| !l.trim().is_empty());
    if has_lines {
        // New data arriving — cancel timers (data flowing means agent is active)
        timer_manager::cancel_waiting_timer(app, agent_id).await;
        timer_manager::cancel_permission_timer(app, agent_id).await;

        // Clear permission state if it was sent
        let should_clear = {
            let mut agents = state.agents.write().await;
            if let Some(agent) = agents.get_mut(&agent_id) {
                if agent.permission_sent {
                    agent.permission_sent = false;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };

        if should_clear {
            events::emit_to_webview(
                app,
                "agent-tool-permission-clear",
                AgentToolPermissionClearPayload {
                    r#type: "agentToolPermissionClear".to_string(),
                    id: agent_id,
                },
            );
        }
    }

    // Process each complete line
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        transcript_parser::process_transcript_line(app, agent_id, trimmed).await;
    }
}
