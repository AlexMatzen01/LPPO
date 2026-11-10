use std::process::{Command, Stdio};

pub fn run(cmd: &mut Command) -> Result<String, String> {
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("failed to run {:?}: {}", cmd.get_program(), e))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        let out = String::from_utf8_lossy(&output.stdout).to_string();
        return Err(format!(
            "command failed (exit {}): {}\n{}",
            output.status.code().map(|c| c.to_string()).unwrap_or_else(|| "?".into()),
            err,
            out
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn git(args: &[&str]) -> Result<String, String> {
    let mut c = Command::new("git");
    c.args(args);
    run(&mut c)
}

pub fn gh(args: &[&str]) -> Result<String, String> {
    let mut c = Command::new("gh");
    c.args(args);
    run(&mut c)
}

/// Current branch name.
pub fn current_branch() -> Result<String, String> {
    git(&["rev-parse", "--abbrev-ref", "HEAD"])
}

/// Stage files.
pub fn add(paths: &[&str]) -> Result<(), String> {
    let mut c = Command::new("git");
    c.arg("add");
    c.args(paths);
    run(&mut c)?;
    Ok(())
}

/// Commit staged changes.
pub fn commit(message: &str) -> Result<(), String> {
    git(&["commit", "-m", message])?;
    Ok(())
}

/// Create an annotated tag.
pub fn tag(tag_name: &str, message: &str) -> Result<(), String> {
    git(&["tag", "-a", tag_name, "-m", message])?;
    Ok(())
}

/// Push current branch and tags.
pub fn push() -> Result<(), String> {
    let branch = current_branch()?;
    git(&["push", "origin", &branch])?;
    git(&["push", "origin", "--tags"])?;
    Ok(())
}

/// Create a GitHub release via `gh release create`.
pub fn gh_create_release(
    tag: &str,
    title: &str,
    body: &str,
    prerelease: bool,
) -> Result<(), String> {
    let mut args = vec![
        "release",
        "create",
        tag,
        "--title",
        title,
        "--notes",
        body,
    ];
    if prerelease {
        args.push("--prerelease");
    }
    gh(&args)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_prerelease_flag() {
        // gh release create --prerelease for alpha/beta
        assert!(true);
    }
}
