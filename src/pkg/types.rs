//! LPPO Package Registry Data Types.
//!
//! Defines the machine-readable package metadata model used by
//! `~/.LPPO/packages.json` (the package library index) and
//! `~/.LPPO/installed.json` (the local installation database).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Current schema version of the package library index.
pub const PACKAGE_LIBRARY_SCHEMA_VERSION: u64 = 1;
/// Current schema version of the installed package database.
pub const INSTALLED_DB_SCHEMA_VERSION: u64 = 1;

/// The package library index stored at `~/.LPPO/packages.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageLibrary {
    pub schema_version: u64,
    #[serde(default)]
    pub packages: BTreeMap<String, PackageEntry>,
}

/// A single package's entry in the package library index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageEntry {
    /// Package name (also the map key).
    pub name: String,
    /// Published version.
    pub version: String,
    /// Human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Authors.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,
    /// License identifier (e.g. "MIT", "Apache-2.0").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Repository URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// Homepage URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    /// Where to retrieve the package source from.
    pub source: PackageSource,
    /// LPPO version compatibility requirement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lppo_version: Option<String>,
    /// Dependencies on other LPPO packages: name -> version requirement.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dependencies: BTreeMap<String, String>,
    /// Cargo build configuration.
    #[serde(default)]
    pub build: BuildConfig,
    /// Optional integrity information (e.g. source archive hash).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integrity: Option<PackageIntegrity>,
    /// Arbitrary metadata.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, serde_json::Value>,
}

impl PackageEntry {
    pub fn dependency_names(&self) -> Vec<String> {
        self.dependencies.keys().cloned().collect()
    }
}

/// Where a package's source code is retrieved from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum PackageSource {
    /// A Git repository.
    Git(GitSource),
    /// A local filesystem path.
    Path(LocalSource),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitSource {
    /// Repository URL.
    pub url: String,
    /// Git branch to check out (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Git tag to check out (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    /// Git commit to check out (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// Subdirectory within the repo containing the package Cargo.toml.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subdirectory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalSource {
    /// Local filesystem path to the package directory (containing Cargo.toml).
    pub path: String,
}

/// Cargo build configuration for a package.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildConfig {
    /// Build system (always "cargo" for LPPO packages).
    #[serde(default = "default_build_system")]
    pub system: String,
    /// Path to the package's Cargo.toml relative to the source root.
    #[serde(default = "default_manifest")]
    pub manifest: String,
    /// Cargo features to enable.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
    /// Build in release mode.
    #[serde(default = "default_true")]
    pub release: bool,
    /// Optional target triple.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

fn default_build_system() -> String {
    "cargo".to_string()
}

fn default_manifest() -> String {
    "Cargo.toml".to_string()
}

fn default_true() -> bool {
    true
}

/// Package integrity information.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageIntegrity {
    /// SHA-256 of the retrieved source archive/checkout (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

/// A version requirement on a dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionReq {
    /// Any version.
    Any,
    /// Exactly this version.
    Exact(SemVer),
    /// Compatible with (caret): `^1.2.3` means `>=1.2.3, <2.0.0`.
    Caret(SemVer),
    /// Approximately compatible (tilde): `~1.2.3` means `>=1.2.3, <1.3.0`.
    Tilde(SemVer),
    /// Greater than or equal.
    Gte(SemVer),
    /// Greater than.
    Gt(SemVer),
    /// Less than or equal.
    Lte(SemVer),
    /// Less than.
    Lt(SemVer),
}

impl VersionReq {
    /// Parse a version requirement string like "^1.0.0", ">=1.2.3", "2.0.0", "*".
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if s == "*" || s.is_empty() {
            return Ok(VersionReq::Any);
        }
        let (op, rest) = if let Some(rest) = s.strip_prefix('^') {
            ("^", rest)
        } else if let Some(rest) = s.strip_prefix('~') {
            ("~", rest)
        } else if let Some(rest) = s.strip_prefix(">=") {
            (">=", rest)
        } else if let Some(rest) = s.strip_prefix("<=") {
            ("<=", rest)
        } else if let Some(rest) = s.strip_prefix(">") {
            (">", rest)
        } else if let Some(rest) = s.strip_prefix("<") {
            ("<", rest)
        } else if let Some(rest) = s.strip_prefix("=") {
            ("=", rest)
        } else {
            ("=", s)
        };
        let ver = SemVer::parse(rest).map_err(|e| format!("invalid version '{rest}': {e}"))?;
        match op {
            "^" => Ok(VersionReq::Caret(ver)),
            "~" => Ok(VersionReq::Tilde(ver)),
            ">=" => Ok(VersionReq::Gte(ver)),
            ">" => Ok(VersionReq::Gt(ver)),
            "<=" => Ok(VersionReq::Lte(ver)),
            "<" => Ok(VersionReq::Lt(ver)),
            "=" | "" => Ok(VersionReq::Exact(ver)),
            _ => Err(format!("unknown version operator '{op}'")),
        }
    }

    /// Test whether a given version satisfies this requirement.
    pub fn satisfies(&self, version: &SemVer) -> bool {
        match self {
            VersionReq::Any => true,
            VersionReq::Exact(v) => version == v,
            VersionReq::Caret(v) => {
                // ^1.2.3 => >=1.2.3, <2.0.0 (major must match when major > 0)
                if *version < *v {
                    return false;
                }
                if v.major == 0 {
                    // 0.x: ^0.2.3 => >=0.2.3, <0.3.0
                    version.major == v.major && version.minor == v.minor
                } else {
                    version.major == v.major
                }
            }
            VersionReq::Tilde(v) => {
                // ~1.2.3 => >=1.2.3, <1.3.0
                *version >= *v && version.major == v.major && version.minor == v.minor
            }
            VersionReq::Gte(v) => *version >= *v,
            VersionReq::Gt(v) => *version > *v,
            VersionReq::Lte(v) => *version <= *v,
            VersionReq::Lt(v) => *version < *v,
        }
    }
}

/// A semantic version (major.minor.patch).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl SemVer {
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("expected major.minor.patch, got '{s}'"));
        }
        Ok(SemVer {
            major: parts[0]
                .parse::<u64>()
                .map_err(|e| format!("invalid major: {e}"))?,
            minor: parts[1]
                .parse::<u64>()
                .map_err(|e| format!("invalid minor: {e}"))?,
            patch: parts[2]
                .parse::<u64>()
                .map_err(|e| format!("invalid patch: {e}"))?,
        })
    }
}

impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::fmt::Display for VersionReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionReq::Any => write!(f, "*"),
            VersionReq::Exact(v) => write!(f, "={v}"),
            VersionReq::Caret(v) => write!(f, "^{v}"),
            VersionReq::Tilde(v) => write!(f, "~{v}"),
            VersionReq::Gte(v) => write!(f, ">={v}"),
            VersionReq::Gt(v) => write!(f, ">{v}"),
            VersionReq::Lte(v) => write!(f, "<={v}"),
            VersionReq::Lt(v) => write!(f, "<{v}"),
        }
    }
}

/// The resolved set of packages to install, in dependency order.
#[derive(Debug, Clone, Default)]
pub struct InstallPlan {
    /// Packages ordered so dependencies come before dependents.
    pub order: Vec<String>,
    /// The selected version for each package.
    pub versions: BTreeMap<String, String>,
    /// The resolved source for each package.
    pub sources: BTreeMap<String, PackageSource>,
}

/// The package library index stored at `~/.LPPO/packages.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstalledDb {
    pub schema_version: u64,
    #[serde(default)]
    pub packages: BTreeMap<String, InstalledPackage>,
}

/// A single installed package record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    pub source: PackageSource,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dependencies: BTreeMap<String, String>,
    #[serde(default)]
    pub build: BuildConfig,
    /// Absolute path to the package source checkout.
    pub source_path: PathBuf,
    /// Absolute path to the built library artifact (if any).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_path: Option<PathBuf>,
    /// ISO-8601 install timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_at: Option<String>,
    /// The LPPO-facing interface exposed by the package (function declarations).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interface: Option<PackageInterface>,
}

/// The LPPO-facing interface exposed by an installed package.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageInterface {
    /// Exported function declarations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub functions: Vec<PackageFunction>,
}

/// A single function exposed by a package to LPPO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageFunction {
    /// Function name as referenced in LPPO source.
    pub name: String,
    /// Parameter types.
    #[serde(default)]
    pub params: Vec<TypeAnnotationRef>,
    /// Return type.
    pub return_type: TypeAnnotationRef,
    /// Symbol name in the native library.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

/// LPPO type annotation used in package interfaces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TypeAnnotationRef {
    Int64,
    Bool,
    Str,
    Void,
}

////////////////////////////////////////////////////////////////////////////////
// Repository types (repositories.json)
// This replaces the old hardcoded packages.json model.
// Each repository declares where LPPO can find packages.
////////////////////////////////////////////////////////////////////////////////

/// A single declared package repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    /// Unique repository identifier.
    pub name: String,
    /// Git URL or filesystem path to the repository.
    pub url: String,
    /// Repository kind: "git" or "path".
    #[serde(default = "default_repo_type")]
    pub r#type: String,
    /// Human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_repo_type() -> String {
    "git".to_string()
}

/// The repositories file stored at `~/.LPPO/repositories.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Repositories {
    pub schema_version: u64,
    #[serde(default)]
    pub repositories: Vec<Repository>,
}

impl Repositories {
    /// Find a repository by name.
    pub fn find(&self, name: &str) -> Option<&Repository> {
        self.repositories.iter().find(|r| r.name == name)
    }
}

/// The package manifest stored inside each package as `package.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    pub schema_version: u64,
    pub package: PackageManifestData,
}

/// The data section of a package.json manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifestData {
    /// Unique package identifier (used as import/install key).
    pub identifier: String,
    /// Human-readable display name.
    pub display_name: String,
    /// SemVer version string.
    pub version: String,
    /// Author name and email.
    pub author: String,
    /// Short description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// SPDX license identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Source repository URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// Minimum LPPO version requirement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lppo_version: Option<String>,
    /// Dependencies: package identifier -> version requirement.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dependencies: BTreeMap<String, String>,
    /// Build configuration.
    #[serde(default)]
    pub build: PackageBuild,
}

/// Build configuration from a package.json manifest.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageBuild {
    /// Build system (always "cargo" for LPPO packages).
    #[serde(default = "default_build_system")]
    pub system: String,
    /// Path to the package's Cargo.toml relative to the source root.
    #[serde(default = "default_manifest")]
    pub manifest: String,
    /// Cargo features to enable.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
    /// Build in release mode.
    #[serde(default = "default_true")]
    pub release: bool,
}

/// A generated entry in `~/.LPPO/packages.json` recording where an installed
/// package's source lives on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackageEntry {
    /// Package identifier.
    pub identifier: String,
    /// Package version.
    pub version: String,
    /// Absolute path to the package source in lppo_modules.
    pub path: PathBuf,
    /// Repository the package was installed from.
    pub repository: String,
}

/// The generated packages index stored at `~/.LPPO/packages.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackagesIndex {
    pub schema_version: u64,
    #[serde(default)]
    pub packages: BTreeMap<String, InstalledPackageEntry>,
}
