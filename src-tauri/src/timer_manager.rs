use tauri::{AppHandle, Manager};
use tokio::time::Duration;

use crate::constants::{PERMISSION_EXEMPT_TOOLS, PERMISSION_TIMER_DELAY_MS};
use crate::events::{self, *};
use crate::state::AppState;

pub async fn cancel_waiting_timer(app: &AppHandle, agent_id: i64) {
    let state = app.state::<AppState>();
    let mut timers = state.waiting_timers.write().await;
    if let Some(handle) = timers.remove(&agent_id) {
        handle.abort();
    }
}

pub async fn start_waiting_timer(app: &AppHandle, agent_id: i64, delay_ms: u64) {
    cancel_waiting_timer(app, agent_id).await;

    let state = app.state::<AppState>();
    let app_clone = app.clone();
    let agents_arc = state.agents.clone();

    let handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;

        {
            let mut agents = agents_arc.write().await;
            if let Some(agent) = agents.get_mut(&agent_id) {
                agent.is_waiting = true;
            }
        }

        events::emit_to_webview(
            &app_clone,
            "agent-status",
            AgentStatusPayload {
                r#type: "agentStatus".to_string(),
                id: agent_id,
                status: "waiting".to_string(),
            },
        );
    });

    let mut timers = state.waiting_timers.write().await;
    timers.insert(agent_id, handle);
}

pub async fn cancel_permission_timer(app: &AppHandle, agent_id: i64) {
    let state = app.state::<AppState>();
    let mut timers = state.permission_timers.write().await;
    if let Some(handle) = timers.remove(&agent_id) {
        handle.abort();
    }
}

pub async fn start_permission_timer(app: &AppHandle, agent_id: i64) {
    cancel_permission_timer(app, agent_id).await;

    let state = app.state::<AppState>();
    let app_clone = app.clone();
    let agents_arc = state.agents.clone();

    let handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(PERMISSION_TIMER_DELAY_MS)).await;

        let mut agents = agents_arc.write().await;
        let Some(agent) = agents.get_mut(&agent_id) else {
            return;
        };

        // Check if there are still active non-exempt tools
        let mut has_non_exempt = false;
        for tool_id in &agent.active_tool_ids {
            if let Some(name) = agent.active_tool_names.get(tool_id) {
                if !PERMISSION_EXEMPT_TOOLS.contains(&name.as_str()) {
                    has_non_exempt = true;
                    break;
                }
            }
        }

        // Check sub-agent tools
        let mut stuck_parent_tool_ids: Vec<String> = Vec::new();
        for (parent_tool_id, sub_names) in &agent.active_subagent_tool_names {
            for name in sub_names.values() {
                if !PERMISSION_EXEMPT_TOOLS.contains(&name.as_str()) {
                    stuck_parent_tool_ids.push(parent_tool_id.clone());
                    has_non_exempt = true;
                    break;
                }
            }
        }

        if has_non_exempt {
            agent.permission_sent = true;
            tracing::info!("Agent {}: possible permission wait detected", agent_id);

            events::emit_to_webview(
                &app_clone,
                "agent-tool-permission",
                AgentToolPermissionPayload {
                    r#type: "agentToolPermission".to_string(),
                    id: agent_id,
                },
            );

            // Also notify stuck sub-agents
            for parent_tool_id in stuck_parent_tool_ids {
                events::emit_to_webview(
                    &app_clone,
                    "subagent-tool-permission",
                    SubagentToolPermissionPayload {
                        r#type: "subagentToolPermission".to_string(),
                        id: agent_id,
                        parent_tool_id,
                    },
                );
            }
        }
    });

    let mut timers = state.permission_timers.write().await;
    timers.insert(agent_id, handle);
}
