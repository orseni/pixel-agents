use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::models::{AgentState, AppSettings};

/// Shared application state managed by Tauri
pub struct AppState {
    pub agents: Arc<RwLock<HashMap<i64, AgentState>>>,
    pub next_agent_id: Arc<RwLock<i64>>,
    pub waiting_timers: Arc<RwLock<HashMap<i64, JoinHandle<()>>>>,
    pub permission_timers: Arc<RwLock<HashMap<i64, JoinHandle<()>>>>,
    /// Set of JSONL file paths currently being watched
    pub watched_files: Arc<RwLock<HashMap<String, i64>>>,
    pub settings: Arc<RwLock<AppSettings>>,
    /// Whether the webview has signaled it's ready
    pub webview_ready: Arc<RwLock<bool>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            next_agent_id: Arc::new(RwLock::new(1)),
            waiting_timers: Arc::new(RwLock::new(HashMap::new())),
            permission_timers: Arc::new(RwLock::new(HashMap::new())),
            watched_files: Arc::new(RwLock::new(HashMap::new())),
            settings: Arc::new(RwLock::new(AppSettings::default())),
            webview_ready: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn allocate_agent_id(&self) -> i64 {
        let mut id = self.next_agent_id.write().await;
        let current = *id;
        *id += 1;
        current
    }
}
