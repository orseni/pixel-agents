use std::path::Path;

/// Reconstruct the original directory path from a Claude Code project hash.
///
/// Claude hashes project paths by replacing `:`, `\`, and `/` with `-`.
/// For example: `/Users/orseni/Dev/pixel-agents` → `-Users-orseni-Dev-pixel-agents`
///
/// Uses a greedy algorithm: tests filesystem existence segment by segment,
/// joining with `-` when a simple `/` doesn't resolve.
/// Returns the basename of the deepest resolved path, or reconstructs
/// a best-guess name from unresolved trailing segments.
pub fn extract_project_name(hash: &str) -> String {
    // Strip leading dash (represents root `/`)
    let stripped = hash.strip_prefix('-').unwrap_or(hash);
    let segments: Vec<&str> = stripped.split('-').collect();

    if segments.is_empty() {
        return hash.to_string();
    }

    // Greedy path reconstruction
    let mut current_path = String::from("/");
    let mut i = 0;
    let mut last_resolved_end = 0; // track how far we resolved

    while i < segments.len() {
        let mut best_end = i;
        let mut found = false;

        // Try joining segments[i..=j] with dashes
        for j in i..segments.len() {
            let candidate_segment = segments[i..=j].join("-");
            let candidate_path = if current_path == "/" {
                format!("/{}", candidate_segment)
            } else {
                format!("{}/{}", current_path, candidate_segment)
            };

            if Path::new(&candidate_path).exists() {
                best_end = j;
                found = true;
            }
        }

        if found {
            let matched_segment = segments[i..=best_end].join("-");
            if current_path == "/" {
                current_path = format!("/{}", matched_segment);
            } else {
                current_path = format!("{}/{}", current_path, matched_segment);
            }
            last_resolved_end = best_end + 1;
            i = best_end + 1;
        } else {
            // No match found — skip this segment
            i += 1;
        }
    }

    // If we resolved the entire hash, return basename of the full path
    if last_resolved_end >= segments.len() && current_path != "/" {
        return Path::new(&current_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(hash)
            .to_string();
    }

    // We couldn't resolve the trailing segments — they form the project name.
    // Join all unresolved segments with dashes to reconstruct the name.
    if last_resolved_end < segments.len() {
        return segments[last_resolved_end..].join("-");
    }

    // Fallback
    segments.last().unwrap_or(&hash).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_returns_last_segment() {
        let result = extract_project_name("-nonexistent-path-foo-bar");
        assert!(!result.is_empty());
    }

    #[test]
    fn test_real_pixel_agents_hash() {
        let result = extract_project_name("-Users-orseni-Desenvolvimento-pixel-agents");
        eprintln!("Result for pixel-agents: {}", result);
        assert_eq!(result, "pixel-agents");
    }

    #[test]
    fn test_deleted_project_uses_trailing_segments() {
        // If the project directory was deleted, use the unresolved trailing segments
        let result = extract_project_name("-Users-orseni-Desenvolvimento-recria-ai-agentic-db-colector");
        eprintln!("Result for recria: {}", result);
        // Should NOT return "Desenvolvimento" — should return trailing unresolved parts
        assert_ne!(result, "Desenvolvimento");
    }
}
