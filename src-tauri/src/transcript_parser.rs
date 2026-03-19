use serde_json::Value;
use tauri::{AppHandle, Manager};

use crate::constants::{
    BASH_COMMAND_DISPLAY_MAX_LENGTH, PERMISSION_EXEMPT_TOOLS,
    TASK_DESCRIPTION_DISPLAY_MAX_LENGTH, TEXT_IDLE_DELAY_MS, TOOL_DONE_DELAY_MS,
};
use crate::events::{self, *};
use crate::state::AppState;
use crate::timer_manager;

fn is_permission_exempt(tool_name: &str) -> bool {
    PERMISSION_EXEMPT_TOOLS.contains(&tool_name)
}

pub fn format_tool_status(tool_name: &str, input: &Value) -> String {
    let base = |key: &str| -> String {
        input
            .get(key)
            .and_then(|v| v.as_str())
            .map(|p| {
                std::path::Path::new(p)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(p)
                    .to_string()
            })
            .unwrap_or_default()
    };

    match tool_name {
        "Read" => format!("Reading {}", base("file_path")),
        "Edit" => format!("Editing {}", base("file_path")),
        "Write" => format!("Writing {}", base("file_path")),
        "Bash" => {
            let cmd = input
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if cmd.len() > BASH_COMMAND_DISPLAY_MAX_LENGTH {
                format!("Running: {}\u{2026}", &cmd[..BASH_COMMAND_DISPLAY_MAX_LENGTH])
            } else {
                format!("Running: {}", cmd)
            }
        }
        "Glob" => "Searching files".to_string(),
        "Grep" => "Searching code".to_string(),
        "WebFetch" => "Fetching web content".to_string(),
        "WebSearch" => "Searching the web".to_string(),
        "Task" | "Agent" => {
            let desc = input
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !desc.is_empty() {
                if desc.len() > TASK_DESCRIPTION_DISPLAY_MAX_LENGTH {
                    format!(
                        "Subtask: {}\u{2026}",
                        &desc[..TASK_DESCRIPTION_DISPLAY_MAX_LENGTH]
                    )
                } else {
                    format!("Subtask: {}", desc)
                }
            } else {
                "Running subtask".to_string()
            }
        }
        "AskUserQuestion" => "Waiting for your answer".to_string(),
        "EnterPlanMode" => "Planning".to_string(),
        "NotebookEdit" => "Editing notebook".to_string(),
        _ => format!("Using {}", tool_name),
    }
}

pub async fn process_transcript_line(app: &AppHandle, agent_id: i64, line: &str) {
    let Ok(record) = serde_json::from_str::<Value>(line) else {
        return;
    };
    let state = app.state::<AppState>();

    let record_type = record.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match record_type {
        "assistant" => process_assistant(app, &state, agent_id, &record).await,
        "user" => process_user(app, &state, agent_id, &record).await,
        "system" => process_system(app, &state, agent_id, &record).await,
        "progress" => process_progress(app, &state, agent_id, &record).await,
        _ => {}
    }
}

async fn process_assistant(
    app: &AppHandle,
    state: &AppState,
    agent_id: i64,
    record: &Value,
) {
    let Some(content) = record
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
    else {
        return;
    };

    let has_tool_use = content.iter().any(|b| b.get("type").and_then(|t| t.as_str()) == Some("tool_use"));

    if has_tool_use {
        timer_manager::cancel_waiting_timer(app, agent_id).await;

        {
            let mut agents = state.agents.write().await;
            if let Some(agent) = agents.get_mut(&agent_id) {
                agent.is_waiting = false;
                agent.had_tools_in_turn = true;
            }
        }

        events::emit_to_webview(
            app,
            "agent-status",
            AgentStatusPayload {
                r#type: "agentStatus".to_string(),
                id: agent_id,
                status: "active".to_string(),
            },
        );

        let mut has_non_exempt_tool = false;

        for block in content {
            if block.get("type").and_then(|t| t.as_str()) != Some("tool_use") {
                continue;
            }
            let Some(tool_id) = block.get("id").and_then(|v| v.as_str()) else {
                continue;
            };
            let tool_name = block.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let input = block.get("input").cloned().unwrap_or(Value::Object(Default::default()));
            let status = format_tool_status(tool_name, &input);

            tracing::info!("Agent {} tool start: {} {}", agent_id, tool_id, status);

            {
                let mut agents = state.agents.write().await;
                if let Some(agent) = agents.get_mut(&agent_id) {
                    agent.active_tool_ids.insert(tool_id.to_string());
                    agent.active_tool_statuses.insert(tool_id.to_string(), status.clone());
                    agent.active_tool_names.insert(tool_id.to_string(), tool_name.to_string());
                }
            }

            if !is_permission_exempt(tool_name) {
                has_non_exempt_tool = true;
            }

            events::emit_to_webview(
                app,
                "agent-tool-start",
                AgentToolStartPayload {
                    r#type: "agentToolStart".to_string(),
                    id: agent_id,
                    tool_id: tool_id.to_string(),
                    status,
                },
            );
        }

        if has_non_exempt_tool {
            timer_manager::start_permission_timer(app, agent_id).await;
        }
    } else {
        // Check for text-only response in a turn without tools
        let has_text = content.iter().any(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"));
        let had_tools = {
            let agents = state.agents.read().await;
            agents.get(&agent_id).map(|a| a.had_tools_in_turn).unwrap_or(false)
        };

        if has_text && !had_tools {
            timer_manager::start_waiting_timer(app, agent_id, TEXT_IDLE_DELAY_MS).await;
        }
    }
}

async fn process_user(
    app: &AppHandle,
    state: &AppState,
    agent_id: i64,
    record: &Value,
) {
    let Some(content) = record.get("message").and_then(|m| m.get("content")) else {
        return;
    };

    if let Some(blocks) = content.as_array() {
        let has_tool_result = blocks
            .iter()
            .any(|b| b.get("type").and_then(|t| t.as_str()) == Some("tool_result"));

        if has_tool_result {
            for block in blocks {
                if block.get("type").and_then(|t| t.as_str()) != Some("tool_result") {
                    continue;
                }
                let Some(tool_use_id) = block.get("tool_use_id").and_then(|v| v.as_str()) else {
                    continue;
                };

                tracing::info!("Agent {} tool done: {}", agent_id, tool_use_id);

                let completed_tool_name = {
                    let mut agents = state.agents.write().await;
                    if let Some(agent) = agents.get_mut(&agent_id) {
                        let name = agent.active_tool_names.get(tool_use_id).cloned();

                        // If completed tool was Task/Agent, clear subagent tools
                        if matches!(name.as_deref(), Some("Task") | Some("Agent")) {
                            agent.active_subagent_tool_ids.remove(tool_use_id);
                            agent.active_subagent_tool_names.remove(tool_use_id);

                            events::emit_to_webview(
                                app,
                                "subagent-clear",
                                SubagentClearPayload {
                                    r#type: "subagentClear".to_string(),
                                    id: agent_id,
                                    parent_tool_id: tool_use_id.to_string(),
                                },
                            );
                        }

                        agent.active_tool_ids.remove(tool_use_id);
                        agent.active_tool_statuses.remove(tool_use_id);
                        agent.active_tool_names.remove(tool_use_id);
                        name
                    } else {
                        None
                    }
                };

                // Emit tool done with delay (anti-flicker)
                let app_clone = app.clone();
                let tid = tool_use_id.to_string();
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(TOOL_DONE_DELAY_MS)).await;
                    events::emit_to_webview(
                        &app_clone,
                        "agent-tool-done",
                        AgentToolDonePayload {
                            r#type: "agentToolDone".to_string(),
                            id: agent_id,
                            tool_id: tid,
                        },
                    );
                });

                let _ = completed_tool_name;
            }

            // All tools completed — allow text-idle timer as fallback
            {
                let mut agents = state.agents.write().await;
                if let Some(agent) = agents.get_mut(&agent_id) {
                    if agent.active_tool_ids.is_empty() {
                        agent.had_tools_in_turn = false;
                    }
                }
            }
        } else {
            // New user text prompt — new turn starting
            timer_manager::cancel_waiting_timer(app, agent_id).await;
            clear_agent_activity(app, state, agent_id).await;
        }
    } else if content.is_string() {
        let text = content.as_str().unwrap_or("");
        if !text.trim().is_empty() {
            // New user text prompt
            timer_manager::cancel_waiting_timer(app, agent_id).await;
            clear_agent_activity(app, state, agent_id).await;
        }
    }
}

async fn process_system(
    app: &AppHandle,
    state: &AppState,
    agent_id: i64,
    record: &Value,
) {
    let subtype = record.get("subtype").and_then(|v| v.as_str()).unwrap_or("");
    if subtype != "turn_duration" {
        return;
    }

    timer_manager::cancel_waiting_timer(app, agent_id).await;
    timer_manager::cancel_permission_timer(app, agent_id).await;

    // Definitive turn-end: clean up stale tool state
    let had_tools = {
        let mut agents = state.agents.write().await;
        if let Some(agent) = agents.get_mut(&agent_id) {
            let had = !agent.active_tool_ids.is_empty();
            if had {
                agent.active_tool_ids.clear();
                agent.active_tool_statuses.clear();
                agent.active_tool_names.clear();
                agent.active_subagent_tool_ids.clear();
                agent.active_subagent_tool_names.clear();
            }
            agent.is_waiting = true;
            agent.permission_sent = false;
            agent.had_tools_in_turn = false;
            had
        } else {
            false
        }
    };

    if had_tools {
        events::emit_to_webview(
            app,
            "agent-tools-clear",
            AgentToolsClearPayload {
                r#type: "agentToolsClear".to_string(),
                id: agent_id,
            },
        );
    }

    events::emit_to_webview(
        app,
        "agent-status",
        AgentStatusPayload {
            r#type: "agentStatus".to_string(),
            id: agent_id,
            status: "waiting".to_string(),
        },
    );
}

async fn process_progress(
    app: &AppHandle,
    state: &AppState,
    agent_id: i64,
    record: &Value,
) {
    let Some(parent_tool_id) = record.get("parentToolUseID").and_then(|v| v.as_str()) else {
        return;
    };
    let Some(data) = record.get("data") else {
        return;
    };
    let data_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("");

    // bash_progress / mcp_progress: tool is actively executing
    if data_type == "bash_progress" || data_type == "mcp_progress" {
        let has_tool = {
            let agents = state.agents.read().await;
            agents
                .get(&agent_id)
                .map(|a| a.active_tool_ids.contains(parent_tool_id))
                .unwrap_or(false)
        };
        if has_tool {
            timer_manager::start_permission_timer(app, agent_id).await;
        }
        return;
    }

    // Verify parent is an active Task/Agent tool
    let parent_tool_name = {
        let agents = state.agents.read().await;
        agents
            .get(&agent_id)
            .and_then(|a| a.active_tool_names.get(parent_tool_id).cloned())
    };

    if !matches!(parent_tool_name.as_deref(), Some("Task") | Some("Agent")) {
        return;
    }

    let Some(msg) = data.get("message") else {
        return;
    };
    let msg_type = msg.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let Some(content) = msg.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_array())
    else {
        return;
    };

    if msg_type == "assistant" {
        let mut has_non_exempt = false;

        for block in content {
            if block.get("type").and_then(|t| t.as_str()) != Some("tool_use") {
                continue;
            }
            let Some(tool_id) = block.get("id").and_then(|v| v.as_str()) else {
                continue;
            };
            let tool_name = block.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let input = block.get("input").cloned().unwrap_or(Value::Object(Default::default()));
            let status = format_tool_status(tool_name, &input);

            tracing::info!(
                "Agent {} subagent tool start: {} {} (parent: {})",
                agent_id, tool_id, status, parent_tool_id
            );

            {
                let mut agents = state.agents.write().await;
                if let Some(agent) = agents.get_mut(&agent_id) {
                    agent
                        .active_subagent_tool_ids
                        .entry(parent_tool_id.to_string())
                        .or_default()
                        .insert(tool_id.to_string());

                    agent
                        .active_subagent_tool_names
                        .entry(parent_tool_id.to_string())
                        .or_default()
                        .insert(tool_id.to_string(), tool_name.to_string());
                }
            }

            if !is_permission_exempt(tool_name) {
                has_non_exempt = true;
            }

            events::emit_to_webview(
                app,
                "subagent-tool-start",
                SubagentToolStartPayload {
                    r#type: "subagentToolStart".to_string(),
                    id: agent_id,
                    parent_tool_id: parent_tool_id.to_string(),
                    tool_id: tool_id.to_string(),
                    status,
                },
            );
        }

        if has_non_exempt {
            timer_manager::start_permission_timer(app, agent_id).await;
        }
    } else if msg_type == "user" {
        for block in content {
            if block.get("type").and_then(|t| t.as_str()) != Some("tool_result") {
                continue;
            }
            let Some(tool_use_id) = block.get("tool_use_id").and_then(|v| v.as_str()) else {
                continue;
            };

            tracing::info!(
                "Agent {} subagent tool done: {} (parent: {})",
                agent_id, tool_use_id, parent_tool_id
            );

            {
                let mut agents = state.agents.write().await;
                if let Some(agent) = agents.get_mut(&agent_id) {
                    if let Some(sub_tools) = agent.active_subagent_tool_ids.get_mut(parent_tool_id) {
                        sub_tools.remove(tool_use_id);
                    }
                    if let Some(sub_names) = agent.active_subagent_tool_names.get_mut(parent_tool_id) {
                        sub_names.remove(tool_use_id);
                    }
                }
            }

            let app_clone = app.clone();
            let pid = parent_tool_id.to_string();
            let tid = tool_use_id.to_string();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(TOOL_DONE_DELAY_MS)).await;
                events::emit_to_webview(
                    &app_clone,
                    "subagent-tool-done",
                    SubagentToolDonePayload {
                        r#type: "subagentToolDone".to_string(),
                        id: agent_id,
                        parent_tool_id: pid,
                        tool_id: tid,
                    },
                );
            });
        }

        // Check if still has non-exempt sub-agent tools
        let still_has_non_exempt = {
            let agents = state.agents.read().await;
            if let Some(agent) = agents.get(&agent_id) {
                agent.active_subagent_tool_names.values().any(|sub_names| {
                    sub_names.values().any(|name| !is_permission_exempt(name))
                })
            } else {
                false
            }
        };

        if still_has_non_exempt {
            timer_manager::start_permission_timer(app, agent_id).await;
        }
    }
}

async fn clear_agent_activity(app: &AppHandle, state: &AppState, agent_id: i64) {
    timer_manager::cancel_permission_timer(app, agent_id).await;

    {
        let mut agents = state.agents.write().await;
        if let Some(agent) = agents.get_mut(&agent_id) {
            agent.clear_activity();
            agent.had_tools_in_turn = false;
        }
    }

    events::emit_to_webview(
        app,
        "agent-tools-clear",
        AgentToolsClearPayload {
            r#type: "agentToolsClear".to_string(),
            id: agent_id,
        },
    );

    events::emit_to_webview(
        app,
        "agent-status",
        AgentStatusPayload {
            r#type: "agentStatus".to_string(),
            id: agent_id,
            status: "active".to_string(),
        },
    );
}
