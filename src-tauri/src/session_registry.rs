use serde::Deserialize;
use std::path::PathBuf;

/// Session info from ~/.claude/sessions/{PID}.json
#[derive(Debug, Deserialize)]
pub struct SessionInfo {
    pub pid: u32,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub cwd: String,
    #[serde(rename = "startedAt")]
    #[allow(dead_code)]
    pub started_at: u64,
}

/// Get the Claude sessions directory: ~/.claude/sessions/
fn get_sessions_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("sessions"))
}

/// Read all session files and return those with alive PIDs.
pub fn read_active_sessions() -> Vec<SessionInfo> {
    let sessions_dir = match get_sessions_dir() {
        Some(d) if d.exists() => d,
        _ => return Vec::new(),
    };

    let Ok(entries) = std::fs::read_dir(&sessions_dir) else {
        return Vec::new();
    };

    let mut sessions = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let Ok(contents) = std::fs::read_to_string(&path) else {
            continue;
        };

        let Ok(session) = serde_json::from_str::<SessionInfo>(&contents) else {
            continue;
        };

        if is_pid_alive(session.pid) {
            sessions.push(session);
        }
    }

    sessions
}

/// Check if a process with the given PID is still alive using kill -0.
/// Returns true if the process exists (even if we lack permission to signal it).
#[cfg(unix)]
pub fn is_pid_alive(pid: u32) -> bool {
    let ret = unsafe { libc::kill(pid as i32, 0) };
    if ret == 0 {
        return true;
    }
    // EPERM means process exists but we can't signal it — still alive
    std::io::Error::last_os_error()
        .raw_os_error()
        .map_or(false, |e| e == libc::EPERM)
}

#[cfg(not(unix))]
pub fn is_pid_alive(_pid: u32) -> bool {
    // On non-Unix platforms, assume alive (Claude Code is Unix-only)
    true
}

/// Convert a working directory path to the Claude project hash.
/// Claude hashes project paths by replacing `:`, `\`, and `/` with `-`.
pub fn cwd_to_project_hash(cwd: &str) -> String {
    cwd.replace(['/', '\\', ':'], "-")
}

/// Get the JSONL file path for a session, if it exists on disk.
pub fn session_jsonl_path(session: &SessionInfo) -> Option<PathBuf> {
    let projects_dir = dirs::home_dir()?.join(".claude").join("projects");
    let hash = cwd_to_project_hash(&session.cwd);
    let jsonl = projects_dir
        .join(&hash)
        .join(format!("{}.jsonl", session.session_id));
    if jsonl.exists() {
        Some(jsonl)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cwd_to_project_hash() {
        assert_eq!(
            cwd_to_project_hash("/Users/orseni/Desenvolvimento/pixel-agents"),
            "-Users-orseni-Desenvolvimento-pixel-agents"
        );
    }

    #[test]
    fn test_cwd_to_project_hash_windows() {
        assert_eq!(
            cwd_to_project_hash("C:\\Users\\user\\project"),
            "C--Users-user-project"
        );
    }
}
