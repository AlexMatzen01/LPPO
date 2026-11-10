use clap::Parser;

mod docs;
mod git;
mod state;
mod tui;

use crate::state::{Channel, VersionState};

#[derive(Parser, Debug)]
#[command(author, version, about = "LPPO version tracking CLI/TUI")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Show the current version state.
    Show,
    /// Launch the interactive TUI.
    Tui,
    /// Set the version manually (major minor patch).
    Set {
        major: u64,
        minor: u64,
        patch: u64,
    },
    /// Bump the build number automatically.
    #[command(alias = "b")]
    BumpBuild,
    /// Set the release channel (alpha|beta|release).
    Channel {
        channel: String,
    },
    /// Reset the build number to 0.
    ResetBuild,
    /// Save state and print the full version string.
    Version,
    /// Sync version state from the latest GitHub release tag.
    Sync,
    /// Create a GitHub release for the current version.
    Release {
        #[arg(long, default_value = "false")]
        prerelease: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        None => crate::tui::run_tui().map_err(|e| e.to_string())?,
        Some(Command::Show) => {
            let state = VersionState::load()?;
            println!("{}", state);
        }
        Some(Command::Tui) => crate::tui::run_tui().map_err(|e| e.to_string())?,
        Some(Command::Set {
            major,
            minor,
            patch,
        }) => {
            let mut state = VersionState::load()?;
            state.major = major;
            state.minor = minor;
            state.patch = patch;
            state.build = 0;
            state.save()?;
            println!("Version set to {}", state.version_string());
        }
        Some(Command::BumpBuild) => {
            let mut state = VersionState::load()?;
            state.bump_build();
            state.save()?;
            println!("Build bumped to {}", state.build);
        }
        Some(Command::Channel { channel }) => {
            let mut state = VersionState::load()?;
            let ch = match channel.to_lowercase().as_str() {
                "alpha" => Channel::Alpha,
                "beta" => Channel::Beta,
                "release" => Channel::Release,
                other => return Err(format!("unknown channel: {}", other).into()),
            };
            state.set_channel(ch);
            state.save()?;
            println!("Channel set to {}", ch.as_str());
        }
        Some(Command::ResetBuild) => {
            let mut state = VersionState::load()?;
            state.build = 0;
            state.save()?;
            println!("Build reset to 0");
        }
        Some(Command::Version) => {
            let state = VersionState::load()?;
            println!("{}", state.version_string());
        }
        Some(Command::Sync) => {
            let mut state = VersionState::load()?;
            let tags = crate::git::git(&["ls-remote", "--tags", "origin"])?;
            // Find the latest tag matching v*
            let mut latest: Option<(String, u64, u64, u64, u64)> = None;
            for line in tags.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 2 {
                    continue;
                }
                let tag = parts[1];
                if let Some(tag_name) = tag.strip_prefix("refs/tags/v") {
                    if let Some(version_str) = tag_name.strip_suffix("^{}") {
                        // Parse version like 1.0.1-beta.5 or 1.0.1+5
                        let parsed = parse_version_tag(version_str);
                        if let Some((maj, min, pat, bld)) = parsed {
                            match &latest {
                                None => latest = Some((version_str.to_string(), maj, min, pat, bld)),
                                Some((_, cm, ci, cp, cb)) => {
                                    if (maj, min, pat, bld) > (*cm, *ci, *cp, *cb) {
                                        latest = Some((version_str.to_string(), maj, min, pat, bld));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Some((tag_str, maj, min, pat, bld)) = latest {
                state.major = maj;
                state.minor = min;
                state.patch = pat;
                state.build = bld;
                // Determine channel from tag string
                if tag_str.contains("-alpha") {
                    state.channel = Channel::Alpha;
                } else if tag_str.contains("-beta") {
                    state.channel = Channel::Beta;
                } else {
                    state.channel = Channel::Release;
                }
                state.save()?;
                println!("Synced version from GitHub tag v{}", tag_str);
                println!("New version: {}", state.version_string());
            } else {
                println!("No version tags found on GitHub.");
            }
        }
        Some(Command::Release { prerelease }) => {
            let state = VersionState::load()?;
            let tag = state.tag_name();
            let title = format!("LPPO {}", state.version_string());
            let body = format!("LPPO version {}\n\nBuilt via version-cli.", state.version_string());
            let is_prerelease = prerelease || state.channel != Channel::Release;
            crate::git::git(&["add", ".version.json"])?;
            crate::git::git(&["commit", "-m", &format!("chore: release v{}", state.version_string())])?;
            crate::git::tag(&tag, &format!("Release v{}", state.version_string()))?;
            crate::git::push()?;
            crate::git::gh_create_release(&tag, &title, &body, is_prerelease)?;
            println!("Released v{} to GitHub.", state.version_string());
        }
    }

    Ok(())
}

/// Parse a version tag string like "1.0.1-beta.5" or "1.0.1+5" into
/// (major, minor, patch, build).
fn parse_version_tag(tag: &str) -> Option<(u64, u64, u64, u64)> {
    // Strip any leading 'v'
    let tag = tag.strip_prefix('v').unwrap_or(tag);
    // Split into base and pre/build metadata
    let (base, rest) = if let Some(idx) = tag.find(['-', '+']) {
        (&tag[..idx], &tag[idx..])
    } else {
        (tag, "")
    };
    let parts: Vec<&str> = base.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let major = parts[0].parse().ok()?;
    let minor = parts[1].parse().ok()?;
    let patch = parts[2].parse().ok()?;
    let build = if rest.starts_with('-') {
        // Pre-release: e.g. -beta.5 -> build = 5
        rest.split('.')
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    } else if rest.starts_with('+') {
        // Build metadata: e.g. +5 -> build = 5
        rest[1..].parse().ok().unwrap_or(0)
    } else {
        0
    };
    Some((major, minor, patch, build))
}

impl std::fmt::Display for VersionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Version: {}", self.version_string())?;
        writeln!(f, "Core:    {}", self.core_string())?;
        writeln!(f, "Channel: {}", self.channel.as_str())?;
        writeln!(f, "Build:   {}", self.build)?;
        writeln!(f, "Tag:     {}", self.tag_name())?;
        Ok(())
    }
}
