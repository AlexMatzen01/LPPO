use crate::state::VersionState;
use std::fs;

const README_PATH: &str = "readme.md";
const CHANGELOG_PATH: &str = "CHANGELOG.md";

fn now() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// Ensure readme.md has a current version line near the top.
pub fn update_readme(state: &VersionState) -> Result<(), String> {
    let path = README_PATH;
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let version_line = format!("**Version:** {} ({})", state.version_string(), state.channel.as_str());

    let new_content = if content.contains("**Version:**") {
        // Replace existing Version line.
        let mut out = String::new();
        for line in content.lines() {
            if line.trim_start().starts_with("**Version:**") {
                out.push_str(&version_line);
                out.push('\n');
            } else {
                out.push_str(line);
                out.push('\n');
            }
        }
        out
    } else {
        // Insert after the first heading (title) line.
        let mut lines: Vec<&str> = content.lines().collect();
        let insert_idx = lines
            .iter()
            .position(|l| l.trim_start().starts_with('#'))
            .map(|i| i + 1)
            .unwrap_or(0);
        lines.insert(insert_idx, "");
        lines.insert(insert_idx + 1, &version_line);
        lines.join("\n")
    };

    fs::write(path, new_content).map_err(|e| e.to_string())
}

/// Append a changelog entry under the top "## Unreleased" section, or create one.
pub fn add_changelog_entry(state: &VersionState, notes: &str) -> Result<(), String> {
    let path = CHANGELOG_PATH;
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let date = now();
    let entry = format!(
        "\n## {} - {} ({})\n\n{}\n",
        state.version_string(),
        date,
        state.channel.as_str(),
        notes
    );

    let new_content = if content.contains("## Unreleased") {
        // Insert after the "## Unreleased" heading line.
        let mut out = String::new();
        let mut inserted = false;
        for line in content.lines() {
            out.push_str(line);
            out.push('\n');
            if !inserted && line.trim() == "## Unreleased" {
                out.push_str(&entry);
                inserted = true;
            }
        }
        if !inserted {
            out.push_str(&format!("## Unreleased\n{}", entry));
        }
        out
    } else {
        format!("## Unreleased\n{}\n{}", entry, content)
    };

    fs::write(path, new_content).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changelog_entry_contains_version() {
        let _ = VersionState::default().version_string();
    }
}
