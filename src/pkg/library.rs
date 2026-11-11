//! Package library index backed by `~/.LPPO/repositories.json` (global) or
//! `<project>/.lppo/repositories.json` (project-local).
//!
//! Package sources are declared in repositories.json. Each package carries its
//! own `package.json` manifest. Installed package sources live in
//! `~/.LPPO/lppo_modules/<identifier>/` (global) or
//! `<project>/.lppo/lppo_modules/<identifier>/` (project-local).
//! A generated `~/.LPPO/packages.json` records the on-disk path of each
//! installed package.

use crate::error::{LppoError, LppoResult};
use crate::pkg::types::{
    InstalledDb, InstalledPackage, InstalledPackageEntry, PackageBuild, PackageInterface,
    PackageManifest, PackageManifestData, PackagesIndex, Repository, Repositories,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

///////////////////////////////////////////////////////////////////////////////
// Package context
///////////////////////////////////////////////////////////////////////////////

/// Where package metadata lives: globally at `~/.LPPO/` or project-local at
/// `<project>/.lppo/`.
#[derive(Debug, Clone)]
pub enum PackageContext {
    /// Global package storage at `~/.LPPO/`.
    Global,
    /// Project-local storage at `<project>/.lppo/`.
    Project(PathBuf),
}

impl PackageContext {
    /// Discover a context by walking up from `start_dir` looking for
    /// `lppo.toml`. Returns `Project(<root>/.lppo)` if found, else `Global`.
    pub fn discover(start_dir: &Path) -> LppoResult<Self> {
        let mut current = start_dir.to_path_buf();
        loop {
            if current.join("lppo.toml").is_file() {
                return Ok(PackageContext::Project(current.join(".lppo")));
            }
            if !current.pop() {
                break;
            }
        }
        Ok(PackageContext::Global)
    }

    /// Returns `true` for the project-local context.
    pub fn is_project(&self) -> bool {
        matches!(self, PackageContext::Project(_))
    }
}

//////////////////////////////////////////////////////////////////////////////
// Path helpers (global)
//////////////////////////////////////////////////////////////////////////////

/// Root directory for all LPPO package data: `~/.LPPO`.
pub fn global_lppo_dir() -> LppoResult<PathBuf> {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("USERPROFILE").map(PathBuf::from))
        .map_err(|_| {
            LppoError::io_error("could not determine home directory (HOME unset).".to_string())
        })?;
    Ok(home.join(".LPPO"))
}

/// Path to the repositories file: `~/.LPPO/repositories.json`.
pub fn repositories_json_path() -> LppoResult<PathBuf> {
    Ok(global_lppo_dir()?.join("repositories.json"))
}

/// Path to the generated packages index: `~/.LPPO/packages.json`.
pub fn packages_json_path() -> LppoResult<PathBuf> {
    Ok(global_lppo_dir()?.join("packages.json"))
}

/// Path to the installed database: `~/.LPPO/installed.json`.
pub fn installed_json_path() -> LppoResult<PathBuf> {
    Ok(global_lppo_dir()?.join("installed.json"))
}

/// Path to the config file: `~/.LPPO/config.json`.
pub fn config_json_path() -> LppoResult<PathBuf> {
    Ok(global_lppo_dir()?.join("config.json"))
}

/// Cache subdirectories for the global context.
pub fn global_cache_dir() -> LppoResult<PathBuf> {
    Ok(global_lppo_dir()?.join("cache"))
}
pub fn global_downloads_dir() -> LppoResult<PathBuf> {
    Ok(global_cache_dir()?.join("downloads"))
}
pub fn global_git_dir() -> LppoResult<PathBuf> {
    Ok(global_cache_dir()?.join("git"))
}
pub fn global_builds_dir() -> LppoResult<PathBuf> {
    Ok(global_cache_dir()?.join("builds"))
}

/// Directory where installed package source code lives: `~/.LPPO/lppo_modules`.
pub fn global_lppo_modules_dir() -> LppoResult<PathBuf> {
    Ok(global_lppo_dir()?.join("lppo_modules"))
}

/// Retrieved package source checkouts for the global context.
pub fn global_packages_dir() -> LppoResult<PathBuf> {
    Ok(global_lppo_dir()?.join("packages"))
}

//////////////////////////////////////////////////////////////////////////////
// Path helpers (context-aware)
//////////////////////////////////////////////////////////////////////////////

/// Root package directory for the given context.
pub fn lppo_dir(context: &PackageContext) -> LppoResult<PathBuf> {
    match context {
        PackageContext::Global => global_lppo_dir(),
        PackageContext::Project(dir) => Ok(dir.clone()),
    }
}

/// Path to the repositories file for the context.
pub fn repositories_json_path_with(context: &PackageContext) -> LppoResult<PathBuf> {
    match context {
        PackageContext::Global => repositories_json_path(),
        PackageContext::Project(dir) => Ok(dir.join("repositories.json")),
    }
}

/// Path to the generated packages index for the context.
pub fn packages_json_path_with(context: &PackageContext) -> LppoResult<PathBuf> {
    match context {
        PackageContext::Global => packages_json_path(),
        PackageContext::Project(dir) => Ok(dir.join("packages.json")),
    }
}

/// Path to the installed database for the context.
pub fn installed_json_path_with(context: &PackageContext) -> LppoResult<PathBuf> {
    match context {
        PackageContext::Global => installed_json_path(),
        PackageContext::Project(dir) => Ok(dir.join("installed.json")),
    }
}

/// Path to the config file for the context.
pub fn config_json_path_with(context: &PackageContext) -> LppoResult<PathBuf> {
    match context {
        PackageContext::Global => config_json_path(),
        PackageContext::Project(dir) => Ok(dir.join("config.json")),
    }
}

/// Cache subdirectories for the context.
pub fn cache_dir(context: &PackageContext) -> LppoResult<PathBuf> {
    match context {
        PackageContext::Global => global_cache_dir(),
        PackageContext::Project(dir) => Ok(dir.join("cache")),
    }
}
pub fn downloads_dir(context: &PackageContext) -> LppoResult<PathBuf> {
    Ok(cache_dir(context)?.join("downloads"))
}
pub fn git_dir(context: &PackageContext) -> LppoResult<PathBuf> {
    Ok(cache_dir(context)?.join("git"))
}
pub fn builds_dir(context: &PackageContext) -> LppoResult<PathBuf> {
    Ok(cache_dir(context)?.join("builds"))
}

/// Directory where installed package source code lives for the context.
pub fn lppo_modules_dir(context: &PackageContext) -> LppoResult<PathBuf> {
    match context {
        PackageContext::Global => global_lppo_modules_dir(),
        PackageContext::Project(dir) => Ok(dir.join("lppo_modules")),
    }
}

/// Path to a specific installed package's module directory.
pub fn package_module_dir(context: &PackageContext, identifier: &str) -> LppoResult<PathBuf> {
    Ok(lppo_modules_dir(context)?.join(identifier))
}

/// Retrieved package source checkouts for the context.
pub fn packages_dir(context: &PackageContext) -> LppoResult<PathBuf> {
    match context {
        PackageContext::Global => global_packages_dir(),
        PackageContext::Project(dir) => Ok(dir.join("packages")),
    }
}

/// Load a package's LPPO-facing interface from its source directory.
///
/// Looks for `<source_path>/lppo_interface.json`. Returns `None` if absent.
pub fn load_package_interface(source_path: &Path) -> LppoResult<Option<PackageInterface>> {
    let interface_path = source_path.join("lppo_interface.json");
    if !interface_path.is_file() {
        return Ok(None);
    }
    let contents = fs::read_to_string(&interface_path).map_err(|e| {
        LppoError::io_error(format!(
            "failed to read package interface '{}': {}",
            interface_path.display(),
            e
        ))
    })?;
    let iface: PackageInterface =
        serde_json::from_str(&contents).map_err(|e| {
            LppoError::serialization_error(format!(
                "failed to parse package interface '{}': {}",
                interface_path.display(),
                e
            ))
        })?;
    Ok(Some(iface))
}

/// Ensure the context's directory tree exists.
pub fn ensure_dirs(context: &PackageContext) -> LppoResult<()> {
    let root = lppo_dir(context)?;
    for dir in [
        root.clone(),
        cache_dir(context)?,
        downloads_dir(context)?,
        git_dir(context)?,
        builds_dir(context)?,
        lppo_modules_dir(context)?,
    ] {
        fs::create_dir_all(&dir).map_err(|e| {
            LppoError::io_error(format!("failed to create directory '{}': {}", dir.display(), e))
        })?;
    }
    Ok(())
}

//////////////////////////////////////////////////////////////////////////////
// JSON helpers
//////////////////////////////////////////////////////////////////////////////

/// Write JSON atomically: write to a temp file then rename.
fn write_json_atomic<T: Serialize + ?Sized>(path: &Path, value: &T) -> LppoResult<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(dir).map_err(|e| {
        LppoError::io_error(format!("failed to create directory '{}': {}", dir.display(), e))
    })?;
    let temp = path.with_extension("json.tmp");
    let serialized = serde_json::to_string_pretty(value).map_err(|e| {
        LppoError::serialization_error(format!("failed to serialize JSON: {e}"))
    })?;
    fs::write(&temp, serialized).map_err(|e| {
        LppoError::io_error(format!("failed to write temp file '{}': {}", temp.display(), e))
    })?;
    fs::rename(&temp, path).map_err(|e| {
        LppoError::io_error(format!("failed to rename temp file to '{}': {}", path.display(), e))
    })?;
    Ok(())
}

/// Read and parse a JSON file, returning a default if the file is missing.
fn read_json_or_default<T: DeserializeOwned + Default>(path: &Path) -> LppoResult<T> {
    if !path.is_file() {
        return Ok(T::default());
    }
    let contents = fs::read_to_string(path).map_err(|e| {
        LppoError::io_error(format!("failed to read '{}': {}", path.display(), e))
    })?;
    if contents.trim().is_empty() {
        return Ok(T::default());
    }
    serde_json::from_str(&contents).map_err(|e| {
        LppoError::serialization_error(format!(
            "failed to parse JSON from '{}': {}",
            path.display(),
            e
        ))
    })
}

/// Read and parse a JSON file, erroring if missing or invalid.
fn read_json_required<T: DeserializeOwned>(path: &Path) -> LppoResult<T> {
    if !path.is_file() {
        return Err(LppoError::io_error(format!(
            "package index file not found: {}",
            path.display()
        )));
    }
    let contents = fs::read_to_string(path).map_err(|e| {
        LppoError::io_error(format!("failed to read '{}': {}", path.display(), e))
    })?;
    if contents.trim().is_empty() {
        return Err(LppoError::serialization_error(format!(
            "package index file is empty: {}",
            path.display()
        )));
    }
    serde_json::from_str(&contents).map_err(|e| {
        LppoError::serialization_error(format!(
            "failed to parse JSON from '{}': {}",
            path.display(),
            e
        ))
    })
}

//////////////////////////////////////////////////////////////////////////////
// Embedded default repositories
//////////////////////////////////////////////////////////////////////////////

/// Default repositories file embedded at compile time from the repo's
/// `repositories.json`. Written to `~/.LPPO/repositories.json` on first load
/// when absent.
const DEFAULT_REPOSITORIES_JSON: &str = include_str!("../../repositories.json");

//////////////////////////////////////////////////////////////////////////////
// Repositories
//////////////////////////////////////////////////////////////////////////////

impl Repositories {
    /// Load the repositories file for the given context, seeding the global
    /// index from the embedded default if absent.
    pub fn load_with(context: &PackageContext) -> LppoResult<Self> {
        ensure_dirs(context)?;
        let path = repositories_json_path_with(context)?;
        if !path.is_file() {
            if matches!(context, PackageContext::Global) {
                let repos: Repositories = serde_json::from_str(DEFAULT_REPOSITORIES_JSON)
                    .map_err(|e| {
                        LppoError::serialization_error(format!(
                            "failed to parse embedded default repositories.json: {e}"
                        ))
                    })?;
                repos.save_with(context)?;
            } else {
                let empty = Repositories::default();
                empty.save_with(context)?;
            }
        }
        let repos = read_json_or_default(&path)?;
        Ok(repos)
    }

    /// Load the global repositories file (`~/.LPPO/repositories.json`).
    pub fn load() -> LppoResult<Self> {
        Self::load_with(&PackageContext::Global)
    }

    /// Save the repositories file for the given context atomically.
    pub fn save_with(&self, context: &PackageContext) -> LppoResult<()> {
        ensure_dirs(context)?;
        let path = repositories_json_path_with(context)?;
        write_json_atomic(&path, self)
    }

    /// Save the global repositories file atomically.
    pub fn save(&self) -> LppoResult<()> {
        self.save_with(&PackageContext::Global)
    }

    /// Add a repository.
    pub fn add(&mut self, repo: Repository) -> bool {
        if self.repositories.iter().any(|r| r.name == repo.name) {
            false
        } else {
            self.repositories.push(repo);
            true
        }
    }

    /// Remove a repository by name. Returns true if removed.
    pub fn remove(&mut self, name: &str) -> bool {
        self.repositories.retain(|r| r.name != name);
        true
    }

    /// List all repository names.
    pub fn names(&self) -> Vec<String> {
        self.repositories.iter().map(|r| r.name.clone()).collect()
    }
}

//////////////////////////////////////////////////////////////////////////////
// PackagesIndex (generated packages.json)
// This is generated by scanning lppo_modules and reading each package's
// package.json manifest.
//////////////////////////////////////////////////////////////////////////////

impl PackagesIndex {
    /// Load the generated packages index for the context.
    pub fn load_with(context: &PackageContext) -> LppoResult<Self> {
        ensure_dirs(context)?;
        let path = packages_json_path_with(context)?;
        Ok(read_json_or_default(&path)?)
    }

    /// Load the global packages index.
    pub fn load() -> LppoResult<Self> {
        Self::load_with(&PackageContext::Global)
    }

    /// Save the packages index for the given context atomically.
    pub fn save_with(&self, context: &PackageContext) -> LppoResult<()> {
        ensure_dirs(context)?;
        let path = packages_json_path_with(context)?;
        write_json_atomic(&path, self)
    }

    /// Save the global packages index atomically.
    pub fn save(&self) -> LppoResult<()> {
        self.save_with(&PackageContext::Global)
    }

    /// Look up an installed package entry by identifier.
    pub fn get(&self, identifier: &str) -> Option<&InstalledPackageEntry> {
        self.packages.get(identifier)
    }

    /// Insert or update a package entry.
    pub fn insert(&mut self, entry: InstalledPackageEntry) {
        self.packages.insert(entry.identifier.clone(), entry);
    }

    /// Remove a package entry by identifier. Returns true if removed.
    pub fn remove(&mut self, identifier: &str) -> bool {
        self.packages.remove(identifier).is_some()
    }

    /// Returns the identifiers of all indexed packages.
    pub fn package_identifiers(&self) -> Vec<String> {
        self.packages.keys().cloned().collect()
    }

    /// Regenerate the packages index by scanning `lppo_modules/` and reading
    /// each package's `package.json` manifest.
    pub fn regenerate_with(context: &PackageContext) -> LppoResult<Self> {
        ensure_dirs(context)?;
        let modules_dir = lppo_modules_dir(context)?;
        let mut index = PackagesIndex::default();
        index.schema_version = 1;

        if modules_dir.is_dir() {
            for entry in fs::read_dir(&modules_dir).map_err(|e| {
                LppoError::io_error(format!(
                    "failed to read lppo_modules directory '{}': {}",
                    modules_dir.display(),
                    e
                ))
            })? {
                let entry = entry.map_err(|e| {
                    LppoError::io_error(format!(
                        "failed to read lppo_modules entry: {e}"
                    ))
                })?;
                let pkg_dir = entry.path();
                if !pkg_dir.is_dir() {
                    continue;
                }
                let manifest_path = pkg_dir.join("package.json");
                let manifest_path = if manifest_path.is_file() {
                    manifest_path
                } else {
                    let alt = pkg_dir.join("lppo_package.json");
                    if alt.is_file() {
                        alt
                    } else {
                        continue;
                    }
                };
                let manifest: PackageManifest = match fs::read_to_string(&manifest_path) {
                    Ok(contents) => match try_parse_manifest(&contents) {
                        Some(m) => m,
                        None => {
                            eprintln!(
                                "warning: failed to parse package manifest in {}",
                                pkg_dir.display()
                            );
                            continue;
                        }
                    },
                    Err(e) => {
                        eprintln!(
                            "warning: failed to read package manifest in {}: {e}",
                            pkg_dir.display()
                        );
                        continue;
                    }
                };
                let data = &manifest.package;
                index.insert(InstalledPackageEntry {
                    identifier: data.identifier.clone(),
                    version: data.version.clone(),
                    path: pkg_dir,
                    repository: data.repository.clone().unwrap_or_default(),
                });
            }
        }

        index.save_with(context)?;
        Ok(index)
    }
}

//////////////////////////////////////////////////////////////////////////////
// InstalledDb
//////////////////////////////////////////////////////////////////////////////

impl InstalledDb {
    /// Load the installed database for the given context.
    pub fn load_with(context: &PackageContext) -> LppoResult<Self> {
        ensure_dirs(context)?;
        let path = installed_json_path_with(context)?;
        Ok(read_json_or_default(&path)?)
    }

    /// Load the global installed database.
    pub fn load() -> LppoResult<Self> {
        Self::load_with(&PackageContext::Global)
    }

    /// Save the installed database for the given context atomically.
    pub fn save_with(&self, context: &PackageContext) -> LppoResult<()> {
        ensure_dirs(context)?;
        let path = installed_json_path_with(context)?;
        write_json_atomic(&path, self)
    }

    /// Save the global installed database atomically.
    pub fn save(&self) -> LppoResult<()> {
        self.save_with(&PackageContext::Global)
    }

    /// Look up an installed package by name.
    pub fn get(&self, name: &str) -> Option<&InstalledPackage> {
        self.packages.get(name)
    }

    /// Insert or update an installed package record, preserving others.
    pub fn insert(&mut self, entry: InstalledPackage) {
        self.packages.insert(entry.name.clone(), entry);
    }

    /// Remove an installed package record by name. Returns true if removed.
    pub fn remove(&mut self, name: &str) -> bool {
        self.packages.remove(name).is_some()
    }

    /// Returns the names of all installed packages.
    pub fn package_names(&self) -> Vec<String> {
        self.packages.keys().cloned().collect()
    }
}

//////////////////////////////////////////////////////////////////////////////
// PackageManifest loading
//////////////////////////////////////////////////////////////////////////////

/// Load a package's `package.json` manifest from its source directory.
pub fn load_package_manifest(source_path: &Path) -> LppoResult<PackageManifest> {
    // Prefer package.json; fall back to lppo_package.json for older packages.
    let manifest_path = source_path.join("package.json");
    let manifest_path = if manifest_path.is_file() {
        manifest_path
    } else {
        let alt = source_path.join("lppo_package.json");
        if alt.is_file() {
            alt
        } else {
            return Err(LppoError::io_error(format!(
                "package.json or lppo_package.json not found in {}",
                source_path.display()
            )));
        }
    };
    let contents = fs::read_to_string(&manifest_path).map_err(|e| {
        LppoError::io_error(format!(
            "failed to read package manifest '{}': {}",
            manifest_path.display(),
            e
        ))
    })?;
    // Try the new wrapped format { schema_version, package: { ... } } first,
    // then the old flat format { name, version, ... }.
    let mut manifest: PackageManifest =
        try_parse_manifest(&contents).ok_or_else(|| {
            LppoError::serialization_error(format!(
                "failed to parse package manifest '{}': unknown format",
                manifest_path.display()
            ))
        })?;
    // Fallback display_name to identifier if absent.
    if manifest.package.display_name.is_none() {
        manifest.package.display_name = Some(manifest.package.identifier.clone());
    }
    Ok(manifest)
}

/// Try to parse a manifest string in either new wrapped format
/// `{ schema_version, package: { ... } }` or old flat format `{ name, version, ... }`.
pub fn try_parse_manifest(contents: &str) -> Option<PackageManifest> {
    serde_json::from_str::<PackageManifest>(contents).ok().or_else(|| {
        serde_json::from_str::<PackageManifestData>(contents).ok().map(|data| PackageManifest {
            schema_version: 1,
            package: data,
        })
    })
}

/// Current UTC timestamp (unix seconds), used for install records. Avoids a
/// hard dependency on chrono.
pub fn current_timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pkg::types::PackageSource;
    use std::collections::BTreeMap;

    #[test]
    fn context_discover_finds_project_toml() {
        let dir = temp_path("proj_discover");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("lppo.toml"), "[project]\nname = \"x\"\n").unwrap();
        let ctx = PackageContext::discover(&dir.join("src")).unwrap();
        assert!(ctx.is_project());
        let PackageContext::Project(p) = ctx else { panic!("expected project context") };
        assert_eq!(p, dir.join(".lppo"));
    }

    #[test]
    fn context_discover_falls_back_to_global() {
        let dir = temp_path("no_proj");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let ctx = PackageContext::discover(&dir).unwrap();
        assert!(!ctx.is_project());
    }

    #[test]
    fn repositories_add_and_remove() {
        let mut repos = Repositories::default();
        repos.schema_version = 1;
        assert!(repos.add(Repository {
            name: "test".to_string(),
            url: "https://example.com/test".to_string(),
            r#type: "git".to_string(),
            description: None,
        }));
        assert!(!repos.add(Repository {
            name: "test".to_string(),
            url: "https://example.com/test2".to_string(),
            r#type: "git".to_string(),
            description: None,
        }));
        assert_eq!(repos.names(), vec!["test".to_string()]);
        assert!(repos.remove("test"));
        assert!(repos.names().is_empty());
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("lppo_pkg_ctx_test_{}", name))
    }
}
