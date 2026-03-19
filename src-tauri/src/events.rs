use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Emit a typed event to the webview frontend
pub fn emit_to_webview<S: Serialize + Clone>(app: &AppHandle, event: &str, payload: S) {
    if let Err(e) = app.emit(event, payload) {
        tracing::warn!("Failed to emit event '{}': {}", event, e);
    }
}

// ── Event payloads ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCreatedPayload {
    pub r#type: String,
    pub id: i64,
    pub project_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentClosedPayload {
    pub r#type: String,
    pub id: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolStartPayload {
    pub r#type: String,
    pub id: i64,
    pub tool_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolDonePayload {
    pub r#type: String,
    pub id: i64,
    pub tool_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolsClearPayload {
    pub r#type: String,
    pub id: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatusPayload {
    pub r#type: String,
    pub id: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolPermissionPayload {
    pub r#type: String,
    pub id: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolPermissionClearPayload {
    pub r#type: String,
    pub id: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentToolStartPayload {
    pub r#type: String,
    pub id: i64,
    pub parent_tool_id: String,
    pub tool_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentToolDonePayload {
    pub r#type: String,
    pub id: i64,
    pub parent_tool_id: String,
    pub tool_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentClearPayload {
    pub r#type: String,
    pub id: i64,
    pub parent_tool_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentToolPermissionPayload {
    pub r#type: String,
    pub id: i64,
    pub parent_tool_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExistingAgentsPayload {
    pub r#type: String,
    pub agents: Vec<i64>,
    pub agent_meta: serde_json::Value,
    pub project_names: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutLoadedPayload {
    pub r#type: String,
    pub layout: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub was_reset: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsLoadedPayload {
    pub r#type: String,
    pub sound_enabled: bool,
}
