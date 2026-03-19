use std::collections::{HashMap, HashSet};
use tauri::{AppHandle, Manager};
use tokio::time::{interval, Duration};

use crate::constants::DISCOVERY_SCAN_INTERVAL_MS;
use crate::events::{self, AgentClosedPayload, AgentCreatedPayload};
use crate::file_watcher;
use crate::project_name::extract_project_name;
use crate::session_registry;
use crate::state::AppState;

/// Info about a tracked session
struct TrackedSession {
    agent_id: i64,
    pid: u32,
    file_key: String,
}

/// Main discovery loop — runs on a background tokio task.
/// Uses ~/.claude/sessions/ PID registry for reliable session detection.
/// Sessions stay alive as long as the Claude Code process is running,
/// regardless of JSONL file activity.
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

    tracing::info!("Discovery loop started (PID-based session detection)");

    let mut tick = interval(Duration::from_millis(DISCOVERY_SCAN_INTERVAL_MS));
    // Track sessions: session_id → TrackedSession
    let mut tracked: HashMap<String, TrackedSession> = HashMap::new();

    loop {
        tick.tick().await;

        // Read active sessions from PID registry (~/.claude/sessions/*.json)
        let active_sessions = session_registry::read_active_sessions();

        // Collect active session IDs for dead-check later
        let active_session_ids: HashSet<String> = active_sessions
            .iter()
            .map(|s| s.session_id.clone())
            .collect();

        // Check for new sessions
        for session in &active_sessions {
            if tracked.contains_key(&session.session_id) {
                continue;
            }

            // Find the JSONL file for this session
            let Some(jsonl_path) = session_registry::session_jsonl_path(session) else {
                continue; // JSONL not created yet, retry next scan
            };

            let file_key = jsonl_path.to_string_lossy().to_string();

            // Check if already watched by another tracking entry
            {
                let watched = state.watched_files.read().await;
                if watched.contains_key(&file_key) {
                    continue;
                }
            }

            // Extract project name from hash
            let project_hash = session_registry::cwd_to_project_hash(&session.cwd);
            let project_name = extract_project_name(&project_hash);
            let project_path = jsonl_path.parent().unwrap().to_path_buf();
            let agent_id = state.allocate_agent_id().await;

            tracing::info!(
                "Discovered active session: {} (project: {}, pid: {})",
                jsonl_path.display(),
                project_name,
                session.pid,
            );

            // Create agent state
            {
                let mut agents = state.agents.write().await;
                let agent = crate::models::AgentState::new(
                    agent_id,
                    project_path,
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

            tracked.insert(
                session.session_id.clone(),
                TrackedSession {
                    agent_id,
                    pid: session.pid,
                    file_key,
                },
            );

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

        // Check for dead sessions (PID no longer alive)
        let dead: Vec<String> = tracked
            .keys()
            .filter(|session_id| !active_session_ids.contains(*session_id))
            .cloned()
            .collect();

        for session_id in dead {
            let Some(session) = tracked.remove(&session_id) else {
                continue;
            };

            tracing::info!(
                "Session closed (pid {} exited): agent {}",
                session.pid,
                session.agent_id,
            );

            // Remove agent state
            {
                let mut agents = state.agents.write().await;
                agents.remove(&session.agent_id);
            }

            // Remove from watched files
            {
                let mut watched = state.watched_files.write().await;
                watched.remove(&session.file_key);
            }

            // Cancel timers
            {
                let mut wt = state.waiting_timers.write().await;
                if let Some(handle) = wt.remove(&session.agent_id) {
                    handle.abort();
                }
            }
            {
                let mut pt = state.permission_timers.write().await;
                if let Some(handle) = pt.remove(&session.agent_id) {
                    handle.abort();
                }
            }

            // Emit agent-closed event
            events::emit_to_webview(
                &app,
                "agent-closed",
                AgentClosedPayload {
                    r#type: "agentClosed".to_string(),
                    id: session.agent_id,
                },
            );
        }
    }
}
