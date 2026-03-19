use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Represents a single active Claude Code session being monitored
#[derive(Debug)]
pub struct AgentState {
    pub id: i64,
    pub project_dir: PathBuf,
    pub project_name: String,
    pub jsonl_file: PathBuf,
    pub file_offset: u64,
    pub line_buffer: String,
    pub active_tool_ids: HashSet<String>,
    pub active_tool_statuses: HashMap<String, String>,
    pub active_tool_names: HashMap<String, String>,
    /// parentToolId → active sub-tool IDs
    pub active_subagent_tool_ids: HashMap<String, HashSet<String>>,
    /// parentToolId → (subToolId → toolName)
    pub active_subagent_tool_names: HashMap<String, HashMap<String, String>>,
    pub is_waiting: bool,
    pub permission_sent: bool,
    pub had_tools_in_turn: bool,
}

impl AgentState {
    pub fn new(id: i64, project_dir: PathBuf, project_name: String, jsonl_file: PathBuf) -> Self {
        Self {
            id,
            project_dir,
            project_name,
            jsonl_file,
            file_offset: 0,
            line_buffer: String::new(),
            active_tool_ids: HashSet::new(),
            active_tool_statuses: HashMap::new(),
            active_tool_names: HashMap::new(),
            active_subagent_tool_ids: HashMap::new(),
            active_subagent_tool_names: HashMap::new(),
            is_waiting: false,
            permission_sent: false,
            had_tools_in_turn: false,
        }
    }

    pub fn clear_activity(&mut self) {
        self.active_tool_ids.clear();
        self.active_tool_statuses.clear();
        self.active_tool_names.clear();
        self.active_subagent_tool_ids.clear();
        self.active_subagent_tool_names.clear();
        self.is_waiting = false;
        self.permission_sent = false;
    }
}

/// Info about a discovered project directory
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    pub hash_name: String,
    pub display_name: String,
    pub path: Option<String>,
    pub active_sessions: usize,
}

/// Persisted agent seat/appearance info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSeatInfo {
    pub palette: u8,
    pub hue_shift: f64,
    pub seat_id: Option<String>,
}

/// Settings persisted to disk
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default = "default_true")]
    pub sound_enabled: bool,
    #[serde(default)]
    pub agent_seats: HashMap<String, AgentSeatInfo>,
}

fn default_true() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            sound_enabled: true,
            agent_seats: HashMap::new(),
        }
    }
}
