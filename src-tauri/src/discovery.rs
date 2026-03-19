use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use tauri::{AppHandle, Manager};
use tokio::time::{interval, Duration};

use crate::constants::{DISCOVERY_SCAN_INTERVAL_MS, SESSION_INACTIVE_THRESHOLD_SECS};
use crate::events::{self, AgentClosedPayload, AgentCreatedPayload};
use crate::file_watcher;
use crate::project_name::extract_project_name;
use crate::state::AppState;

/// Get the Claude projects directory: ~/.claude/projects/
fn get_claude_projects_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("projects"))
}

/// Check if a JSONL file was recently modified (candidate for new agent).
fn is_recently_active(path: &PathBuf) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = metadata.modified() else {
        return false;
    };
    let Ok(elapsed) = SystemTime::now().duration_since(modified) else {
        return false;
    };
    elapsed.as_secs() < SESSION_INACTIVE_THRESHOLD_SECS
}

/// Check if a session is still alive: file exists and was modified within
/// a generous window (5 minutes). Agents waiting for user input can be
/// idle for a while but the session is still valid.
fn is_session_alive(path: &PathBuf) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false; // file deleted
    };
    let Ok(modified) = metadata.modified() else {
        return false;
    };
    let Ok(elapsed) = SystemTime::now().duration_since(modified) else {
        return false;
    };
    // 5 minutes — generous window for idle sessions
    elapsed.as_secs() < 300
}

/// Main discovery loop — runs on a background tokio task.
/// Scans ~/.claude/projects/*/*.jsonl for active sessions.
pub async fn start_discovery_loop(app: AppHandle) {
    let state = app.state::<AppState>();

    // Wait for webview to be ready before starting discovery
    loop {
        {
            let ready = state.webview_ready.read().await;
            if *ready {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    tracing::info!("Discovery loop started");

    let mut tick = interval(Duration::from_millis(DISCOVERY_SCAN_INTERVAL_MS));
    // Track JSONL files we've created agents for: file_key → agent_id
    let mut tracked: HashMap<String, i64> = HashMap::new();

    loop {
        tick.tick().await;

        let projects_dir = match get_claude_projects_dir() {
            Some(d) if d.exists() => d,
            _ => continue,
        };

        // Scan all project directories
        let Ok(entries) = std::fs::read_dir(&projects_dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let project_path = entry.path();
            if !project_path.is_dir() {
                continue;
            }

            let project_hash = entry
                .file_name()
                .to_string_lossy()
                .to_string();

            let Ok(jsonl_entries) = std::fs::read_dir(&project_path) else {
                continue;
            };

            for jsonl_entry in jsonl_entries.flatten() {
                let jsonl_path = jsonl_entry.path();
                let Some(ext) = jsonl_path.extension() else {
                    continue;
                };
                if ext != "jsonl" {
                    continue;
                }

                let file_key = jsonl_path.to_string_lossy().to_string();

                // Skip if already tracked
                if tracked.contains_key(&file_key) {
                    continue;
                }

                // Only create agents for recently active sessions
                if !is_recently_active(&jsonl_path) {
                    continue;
                }

                // Check if already watched by another path
                {
                    let watched = state.watched_files.read().await;
                    if watched.contains_key(&file_key) {
                        continue;
                    }
                }

                // New active session — create agent
                let project_name = extract_project_name(&project_hash);
                let agent_id = state.allocate_agent_id().await;

                tracing::info!(
                    "Discovered active session: {} (project: {})",
                    jsonl_path.display(),
                    project_name
                );

                // Create agent state
                {
                    let mut agents = state.agents.write().await;
                    let agent = crate::models::AgentState::new(
                        agent_id,
                        project_path.clone(),
                        project_name.clone(),
                        jsonl_path.clone(),
                    );
                    agents.insert(agent_id, agent);
                }

                // Track watched file
                {
                    let mut watched = state.watched_files.write().await;
                    watched.insert(file_key.clone(), agent_id);
                }

                tracked.insert(file_key, agent_id);

                // Emit agent-created event
                events::emit_to_webview(
                    &app,
                    "agent-created",
                    AgentCreatedPayload {
                        r#type: "agentCreated".to_string(),
                        id: agent_id,
                        project_name: project_name.clone(),
                    },
                );

                // Start file watching for this session
                let app_clone = app.clone();
                let jsonl_clone = jsonl_path.clone();
                tokio::spawn(async move {
                    file_watcher::start_watching(app_clone, agent_id, jsonl_clone).await;
                });
            }
        }

        // Check for sessions that are no longer alive (file deleted or idle > 5min)
        let dead: Vec<String> = tracked
            .keys()
            .filter(|file_key| !is_session_alive(&PathBuf::from(file_key.as_str())))
            .cloned()
            .collect();

        for file_key in dead {
            let Some(agent_id) = tracked.remove(&file_key) else {
                continue;
            };

            tracing::info!("Session no longer alive: {} (agent {})", file_key, agent_id);

            // Remove agent state
            {
                let mut agents = state.agents.write().await;
                agents.remove(&agent_id);
            }

            // Remove from watched files
            {
                let mut watched = state.watched_files.write().await;
                watched.remove(&file_key);
            }

            // Cancel timers
            {
                let mut wt = state.waiting_timers.write().await;
                if let Some(handle) = wt.remove(&agent_id) {
                    handle.abort();
                }
            }
            {
                let mut pt = state.permission_timers.write().await;
                if let Some(handle) = pt.remove(&agent_id) {
                    handle.abort();
                }
            }

            // Emit agent-closed event
            events::emit_to_webview(
                &app,
                "agent-closed",
                AgentClosedPayload {
                    r#type: "agentClosed".to_string(),
                    id: agent_id,
                },
            );
        }
    }
}
