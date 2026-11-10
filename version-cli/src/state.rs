use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Release channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Alpha,
    Beta,
    Release,
}

impl Default for Channel {
    fn default() -> Self {
        Channel::Beta
    }
}

impl Channel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Channel::Alpha => "alpha",
            Channel::Beta => "beta",
            Channel::Release => "release",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![Channel::Alpha, Channel::Beta, Channel::Release]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionState {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub build: u64,
    pub channel: Channel,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl Default for VersionState {
    fn default() -> Self {
        VersionState {
            major: 1,
            minor: 0,
            patch: 1,
            build: 0,
            channel: Channel::Beta,
            created_at: None,
            updated_at: None,
        }
    }
}

impl VersionState {
    pub fn path() -> PathBuf {
        Path::new(".version.json").to_path_buf()
    }

    pub fn load() -> Result<Self, String> {
        let p = Self::path();
        if !p.exists() {
            let mut s = Self::default();
            let now = chrono::Utc::now().to_rfc3339();
            s.created_at = Some(now.clone());
            s.updated_at = Some(now);
            s.save()?;
            return Ok(s);
        }
        let data = fs::read_to_string(&p).map_err(|e| e.to_string())?;
        let mut s: VersionState = serde_json::from_str(&data).map_err(|e| e.to_string())?;
        // Backfill missing fields.
        if s.created_at.is_none() {
            s.created_at = Some(chrono::Utc::now().to_rfc3339());
        }
        s.updated_at = Some(chrono::Utc::now().to_rfc3339());
        s.save()?;
        Ok(s)
    }

    pub fn save(&self) -> Result<(), String> {
        let data = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(Self::path(), data).map_err(|e| e.to_string())
    }

    /// Full display version.
    /// release: 1.0.1+5
    /// beta:    1.0.1-beta.5
    /// alpha:   1.0.1-alpha.5
    pub fn version_string(&self) -> String {
        let base = format!("{}.{}.{}", self.major, self.minor, self.patch);
        match self.channel {
            Channel::Release => format!("{}+{}", base, self.build),
            Channel::Beta => format!("{}-beta.{}", base, self.build),
            Channel::Alpha => format!("{}-alpha.{}", base, self.build),
        }
    }

    /// Semver core without build metadata.
    pub fn core_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }

    /// Tag name for git.
    pub fn tag_name(&self) -> String {
        format!("v{}", self.version_string())
    }

    pub fn bump_build(&mut self) {
        self.build += 1;
        self.updated_at = Some(chrono::Utc::now().to_rfc3339());
    }

    pub fn set_channel(&mut self, channel: Channel) {
        self.channel = channel;
        self.build = 0; // reset build on channel change
        self.updated_at = Some(chrono::Utc::now().to_rfc3339());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_string_release() {
        let s = VersionState {
            major: 1,
            minor: 0,
            patch: 1,
            build: 5,
            channel: Channel::Release,
            created_at: None,
            updated_at: None,
        };
        assert_eq!(s.version_string(), "1.0.1+5");
    }

    #[test]
    fn version_string_beta() {
        let s = VersionState {
            major: 1,
            minor: 0,
            patch: 1,
            build: 3,
            channel: Channel::Beta,
            created_at: None,
            updated_at: None,
        };
        assert_eq!(s.version_string(), "1.0.1-beta.3");
    }

    #[test]
    fn version_string_alpha() {
        let s = VersionState {
            major: 1,
            minor: 0,
            patch: 1,
            build: 0,
            channel: Channel::Alpha,
            created_at: None,
            updated_at: None,
        };
        assert_eq!(s.version_string(), "1.0.1-alpha.0");
    }

    #[test]
    fn tag_name() {
        let s = VersionState {
            major: 1,
            minor: 0,
            patch: 1,
            build: 2,
            channel: Channel::Beta,
            created_at: None,
            updated_at: None,
        };
        assert_eq!(s.tag_name(), "v1.0.1-beta.2");
    }
}
