// Timing (ms)
pub const JSONL_POLL_INTERVAL_MS: u64 = 1000;
pub const FILE_WATCHER_POLL_INTERVAL_MS: u64 = 1000;
pub const DISCOVERY_SCAN_INTERVAL_MS: u64 = 5000;
pub const TOOL_DONE_DELAY_MS: u64 = 300;
pub const PERMISSION_TIMER_DELAY_MS: u64 = 7000;
pub const TEXT_IDLE_DELAY_MS: u64 = 5000;
// Note: Session liveness is now determined by PID checks in session_registry.rs,
// not by JSONL file mtime thresholds.

// Display truncation
pub const BASH_COMMAND_DISPLAY_MAX_LENGTH: usize = 30;
pub const TASK_DESCRIPTION_DISPLAY_MAX_LENGTH: usize = 40;

// Layout persistence
pub const LAYOUT_FILE_DIR: &str = ".pixel-agents";
pub const LAYOUT_FILE_NAME: &str = "layout.json";
pub const SETTINGS_FILE_NAME: &str = "settings.json";
pub const LAYOUT_FILE_POLL_INTERVAL_MS: u64 = 2000;
pub const LAYOUT_REVISION_KEY: &str = "layoutRevision";

// Permission-exempt tools (delegating tools that don't need user approval)
pub const PERMISSION_EXEMPT_TOOLS: &[&str] = &["Task", "Agent", "AskUserQuestion"];
