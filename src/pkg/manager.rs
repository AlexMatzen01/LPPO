//! The LPPO package manager.
//!
//! Coordinates package retrieval from declared repositories, source staging
//! into `lppo_modules/`, Cargo builds, and installation tracking. Works in
//! either the global context (`~/.LPPO/`) or a project-local context
//! (`<project>/.lppo/`).

use crate::error::{LppoError, LppoResult};
use crate::pkg::builder::CargoBuilder;
use crate::pkg::installed::InstalledTracker;
use crate::pkg::library::{
    ensure_dirs, package_module_dir, load_package_manifest, PackageContext, try_parse_manifest,
};
use crate::pkg::source::SourceRetriever;
use crate::pkg::types::{
    BuildConfig, InstalledPackage, PackageManifest, PackageManifestData, PackageSource,
    Repository, Repositories,
};
use serde_json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The package manager.
pub struct PackageManager {
    context: PackageContext,
    tracker: InstalledTracker,
}

impl PackageManager {
    /// Load the package manager, auto-detecting project context from the
    /// current directory (walking up for `lppo.toml`).
    pub fn load() -> LppoResult<Self> {
        let cwd = std::env::current_dir()
            .map_err(|e| LppoError::io_error(format!("failed to get current dir: {e}")))?;
        let context = PackageContext::discover(&cwd)?;
        Self::load_with_context(context)
    }

    /// Load the package manager with an explicit context.
    pub fn load_with_context(context: PackageContext) -> LppoResult<Self> {
        ensure_dirs(&context)?;
        let tracker = InstalledTracker::load_with(context.clone())?;
        Ok(Self { context, tracker })
    }

    /// The package context in use.
    pub fn context(&self) -> &PackageContext {
        &self.context
    }

    /// Reload the installed tracker from disk (picks up external changes).
    pub fn reload_tracker(&mut self) -> LppoResult<()> {
        self.tracker = InstalledTracker::load_with(self.context.clone())?;
        Ok(())
    }

    /// Search installed packages by query.
    pub fn search(&self, query: &str) -> Vec<&InstalledPackage> {
        let q = query.to_lowercase();
        self.tracker
            .all_packages()
            .into_iter()
            .filter(|p| p.name.to_lowercase().contains(&q))
            .collect()
    }

    /// Search available packages across all configured repositories (path repos
    /// only, since git repos require cloning). Returns matches with their repo
    /// name and whether they're installed.
    pub fn search_available(&self, query: &str) -> Vec<(String, String, bool)> {
        let q = query.to_lowercase();
        let mut results: Vec<(String, String, bool)> = Vec::new();

        // Also include installed packages.
        for p in self.tracker.all_packages() {
            if p.name.to_lowercase().contains(&q) {
                results.push((p.name.clone(), format!("v{}", p.version), true));
            }
        }

        // Scan path repositories for available packages.
        if let Ok(repos) = Repositories::load_with(&self.context) {
            for r in &repos.repositories {
                if r.r#type.as_str() == "path" {
                    self.scan_path_repo_for_available(&r, &q, &mut results);
                } else if r.r#type.as_str() == "git" {
                    self.scan_git_repo_for_available(&r, &q, &mut results);
                }
            }
        }

        results
    }

    /// Scan a path repository for available packages matching the query.
    fn scan_path_repo_for_available(
        &self,
        repo: &Repository,
        q: &str,
        results: &mut Vec<(String, String, bool)>,
    ) {
        let repo_dir = Path::new(&repo.url);
        for manifest_name in ["package.json", "lppo_package.json"] {
            let root_manifest = repo_dir.join(manifest_name);
            if root_manifest.is_file() {
                if let Ok(contents) = fs::read_to_string(&root_manifest) {
                    if let Some(manifest) = try_parse_manifest(&contents) {
                        let id = &manifest.package.identifier;
                        if id.to_lowercase().contains(q) {
                            let installed = self.tracker.is_installed(id);
                            if !results.iter().any(|(n, _, _)| n == id) {
                                results.push((
                                    id.clone(),
                                    format!("v{}", manifest.package.version),
                                    installed,
                                ));
                            }
                        }
                    }
                }
            }
        }
        if let Ok(entries) = fs::read_dir(repo_dir) {
            for entry in entries.flatten() {
                let sub_dir = entry.path();
                if sub_dir.is_dir() {
                    for manifest_name in ["package.json", "lppo_package.json"] {
                        let manifest_path = sub_dir.join(manifest_name);
                        if manifest_path.is_file() {
                            if let Ok(contents) = fs::read_to_string(&manifest_path) {
                                if let Some(manifest) = try_parse_manifest(&contents) {
                                    let id = &manifest.package.identifier;
                                    if id.to_lowercase().contains(q) {
                                        let installed = self.tracker.is_installed(id);
                                        if !results.iter().any(|(n, _, _)| n == id) {
                                            results.push((
                                                id.clone(),
                                                format!("v{}", manifest.package.version),
                                                installed,
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Clone a git repository to a temp dir and scan for available packages.
    fn scan_git_repo_for_available(
        &self,
        repo: &Repository,
        q: &str,
        results: &mut Vec<(String, String, bool)>,
    ) {
        use std::process::Command;
        let temp_dir = std::env::temp_dir().join(format!("lppo_repo_scan_{}", repo.name));
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);

        let status = Command::new("git")
            .arg("clone")
            .arg("--depth")
            .arg("1")
            .arg(&repo.url)
            .arg(&temp_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        if status.map_or(false, |s| !s.success()) {
            let _ = fs::remove_dir_all(&temp_dir);
            return;
        }

        self.scan_dir_for_available(&temp_dir, q, results);
        let _ = fs::remove_dir_all(&temp_dir);
    }

    /// Recursively scan a directory for package manifests matching the query.
    fn scan_dir_for_available(
        &self,
        dir: &Path,
        q: &str,
        results: &mut Vec<(String, String, bool)>,
    ) {
        for manifest_name in ["package.json", "lppo_package.json"] {
            let manifest_path = dir.join(manifest_name);
            if manifest_path.is_file() {
                if let Ok(contents) = fs::read_to_string(&manifest_path) {
                    if let Some(manifest) = try_parse_manifest(&contents) {
                        let id = &manifest.package.identifier;
                        if id.to_lowercase().contains(q) {
                            let installed = self.tracker.is_installed(id);
                            if !results.iter().any(|(n, _, _)| n == id) {
                                results.push((
                                    id.clone(),
                                    format!("v{}", manifest.package.version),
                                    installed,
                                ));
                            }
                        }
                    }
                }
            }
        }
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let sub_dir = entry.path();
                if sub_dir.is_dir() {
                    self.scan_dir_for_available(&sub_dir, q, results);
                }
            }
        }
    }

    /// Get information about an installed package.
    pub fn info(&self, name: &str) -> LppoResult<&InstalledPackage> {
        self.tracker
            .get(name)
            .ok_or_else(|| LppoError::package_not_found(name.to_string()))
    }

    /// List installed packages.
    pub fn list_installed(&self) -> Vec<&InstalledPackage> {
        self.tracker.all_packages()
    }

    /// Install a package by identifier. Resolves the package from the
    /// configured repositories, stages its source into `lppo_modules/`,
    /// builds it, and tracks it as installed.
    pub fn install(&mut self, identifier: &str) -> LppoResult<()> {
        // Reload tracker to ensure we have latest state.
        self.reload_tracker()?;

        if self.tracker.is_installed(identifier) {
            let installed = self.tracker.get(identifier).unwrap();
            println!("{identifier} is already installed (version {})", installed.version);
            return Ok(());
        }

        // Resolve the package from repositories.
        let repos = Repositories::load_with(&self.context)?;
        let mut found_source: Option<PackageSource> = None;

        for r in &repos.repositories {
            match r.r#type.as_str() {
                "path" => {
                    let repo_dir = Path::new(&r.url);
                    // Case 1: repo root IS the package (package.json at root).
                    if repo_dir.join("package.json").is_file() {
                        if let Ok(contents) = fs::read_to_string(&repo_dir.join("package.json")) {
                            if let Ok(manifest) = serde_json::from_str::<PackageManifest>(&contents) {
                                if manifest.package.identifier == identifier {
                                    found_source = Some(PackageSource::Path(crate::pkg::types::LocalSource {
                                        path: repo_dir.to_string_lossy().to_string(),
                                    }));
                                    break;
                                }
                            }
                        }
                    }
                    // Case 2: repo contains subdirectories, each a package.
                    let pkg_dir = repo_dir.join(identifier);
                    if pkg_dir.join("package.json").is_file() {
                        found_source = Some(PackageSource::Path(crate::pkg::types::LocalSource {
                            path: pkg_dir.to_string_lossy().to_string(),
                        }));
                        break;
                    }
                }
                "git" => {
                    if let Ok(pkg_dir) = self.check_git_repo_for_package(r, identifier) {
                        found_source = Some(PackageSource::Path(crate::pkg::types::LocalSource {
                            path: pkg_dir.to_string_lossy().to_string(),
                        }));
                        break;
                    }
                }
                _ => {}
            }
        }

        let source = match found_source {
            Some(s) => s,
            None => {
                return Err(LppoError::package_not_found(format!(
                    "package '{identifier}' not found in any repository. Use 'lppo pkg repo add' to add a repository."
                )));
            }
        };

        self.install_single(identifier, &source)?;
        Ok(())
    }

    /// Clone a git repository to a temp dir and check if it contains a package
    /// with the given identifier. Returns the path to the package subdirectory
    /// if found.
    fn check_git_repo_for_package(
        &self,
        repo: &Repository,
        identifier: &str,
    ) -> LppoResult<PathBuf> {
        use std::process::Command;
        let temp_dir = std::env::temp_dir().join(format!("lppo_repo_check_{}", repo.name));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).map_err(|e| {
            LppoError::io_error(format!(
                "failed to create temp dir '{}': {}",
                temp_dir.display(),
                e
            ))
        })?;

        let status = Command::new("git")
            .arg("clone")
            .arg("--depth")
            .arg("1")
            .arg(&repo.url)
            .arg(&temp_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| {
                LppoError::source_retrieval_failed(
                    identifier.to_string(),
                    format!("git clone failed: {e}"),
                )
            })?;
        if !status.success() {
            let _ = fs::remove_dir_all(&temp_dir);
            return Err(LppoError::source_retrieval_failed(
                identifier.to_string(),
                format!("git clone of '{}' failed", repo.url),
            ));
        }

        // Check if the repo root has package.json or lppo_package.json.
        if let Some(pkg_dir) = Self::find_manifest_in_dir(&temp_dir, identifier)? {
            let _ = fs::remove_dir_all(&temp_dir.join(".git"));
            return Ok(pkg_dir);
        }

        // Check subdirectories for package.json or lppo_package.json.
        if let Ok(entries) = fs::read_dir(&temp_dir) {
            for entry in entries.flatten() {
                let sub_dir = entry.path();
                if sub_dir.is_dir() {
                    if let Some(pkg_dir) = Self::find_manifest_in_dir(&sub_dir, identifier)? {
                        let _ = fs::remove_dir_all(&temp_dir.join(".git"));
                        return Ok(pkg_dir);
                    }
                }
            }
        }

        let _ = fs::remove_dir_all(&temp_dir);
        Err(LppoError::package_not_found(format!(
            "package '{identifier}' not found in repository '{}' ({})",
            repo.name, repo.url
        )))
    }

    /// Look for `package.json` or `lppo_package.json` in `dir`. If found and
    /// its identifier matches, return the directory path.
    fn find_manifest_in_dir(dir: &Path, identifier: &str) -> LppoResult<Option<PathBuf>> {
        for name in ["package.json", "lppo_package.json"] {
            let manifest_path = dir.join(name);
            if manifest_path.is_file() {
                if let Ok(contents) = fs::read_to_string(&manifest_path) {
                    if let Some(manifest) = try_parse_manifest(&contents) {
                        if manifest.package.identifier == identifier {
                            return Ok(Some(dir.to_path_buf()));
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    /// Install a single package (assumes dependencies already resolved).
    fn install_single(
        &mut self,
        identifier: &str,
        source: &PackageSource,
    ) -> LppoResult<()> {
        println!("Installing {identifier} ...");

        // 1. Retrieve source into the context's lppo_modules dir.
        let staged_dir = SourceRetriever::retrieve_with_context(&self.context, identifier, source)?;

        // 2. Read the package.json manifest.
        let manifest = load_package_manifest(&staged_dir)?;
        let data = &manifest.package;

        // 3. Copy/stage into lppo_modules/<identifier>/
        let module_dir = package_module_dir(&self.context, identifier)?;
        if module_dir.exists() {
            fs::remove_dir_all(&module_dir).map_err(|e| {
                LppoError::io_error(format!(
                    "failed to remove existing module dir '{}': {}",
                    module_dir.display(),
                    e
                ))
            })?;
        }
        fs::create_dir_all(&module_dir).map_err(|e| {
            LppoError::io_error(format!(
                "failed to create module dir '{}': {}",
                module_dir.display(),
                e
            ))
        })?;
        copy_dir_contents(&staged_dir, &module_dir)?;

        // If both package.json and lppo_package.json exist, prefer
        // lppo_package.json (the one we matched on) and remove package.json
        // so the generated index picks up the correct identifier.
        let pkg_json = module_dir.join("package.json");
        let lppo_pkg_json = module_dir.join("lppo_package.json");
        if lppo_pkg_json.is_file() && pkg_json.is_file() {
            let _ = fs::remove_file(&pkg_json);
        }

        // 4. Locate Cargo.toml and read build config.
        let build_config = BuildConfig {
            system: data.build.system.clone(),
            manifest: data.build.manifest.clone(),
            features: data.build.features.clone(),
            release: data.build.release,
            target: None,
        };
        let manifest_path = module_dir.join(&build_config.manifest);
        if !manifest_path.is_file() {
            return Err(LppoError::build_failed(
                identifier.to_string(),
                format!(
                    "Cargo.toml not found at {} in package source",
                    manifest_path.display()
                ),
            ));
        }

        // 5. Build with Cargo.
        let artifact_path = CargoBuilder::build(&module_dir, &build_config)?;

        // 6. Register as installed only after a successful build.
        self.tracker.mark_installed_with_context(
            &self.context,
            identifier,
            &data.version,
            source.clone(),
            &data.dependencies,
            build_config,
            module_dir.clone(),
            artifact_path.clone(),
        )?;

        if let Some(artifact) = &artifact_path {
            println!(
                "Built {identifier} {} -> {}",
                data.version,
                artifact.display()
            );
        } else {
            println!("Built {identifier} {} (no library artifact located)", data.version);
        }

        Ok(())
    }

    /// Remove an installed package.
    pub fn remove(&mut self, name: &str) -> LppoResult<()> {
        // Reload installed db to avoid stale state.
        self.tracker = InstalledTracker::load_with(self.context.clone())?;

        if !self.tracker.is_installed(name) {
            return Err(LppoError::package_not_found(name.to_string()));
        }

        let dependents = self.tracker.dependents_of(name);
        if !dependents.is_empty() {
            return Err(LppoError::build_failed(
                name.to_string(),
                format!(
                    "cannot remove: depended on by {}",
                    dependents.join(", ")
                ),
            ));
        }

        if let Some(entry) = self.tracker.get(name) {
            let _ = std::fs::remove_dir_all(&entry.source_path);
        }

        self.tracker.mark_removed_with_context(&self.context, name)?;
        println!("Removed {name}.");
        Ok(())
    }

    /// Update packages. With a name, updates just that package if a newer
    /// version is available on the remote repository. With no name, syncs the
    /// remote repositories.json and updates all installed packages.
    pub fn update(&mut self, name: Option<&str>) -> LppoResult<()> {
        match name {
            Some(n) => {
                if !self.tracker.is_installed(n) {
                    return Err(LppoError::package_not_found(n.to_string()));
                }
                self.update_single(n)?;
            }
            None => {
                // Fetch the remote repositories.json from GitHub.
                self.sync_remote_repositories()?;

                // Attempt to update each installed package.
                let installed_names: Vec<String> = self.tracker.installed_names();
                if installed_names.is_empty() {
                    println!("No packages installed to update.");
                    return Ok(());
                }
                for n in installed_names {
                    let _ = self.update_single(&n);
                }
                println!("Update complete.");
            }
        }
        Ok(())
    }

    /// Check a single installed package for updates and apply if out of date.
    fn update_single(&mut self, name: &str) -> LppoResult<()> {
        let installed = match self.tracker.get(name) {
            Some(pkg) => pkg,
            None => return Err(LppoError::package_not_found(name.to_string())),
        };

        let installed_version = &installed.version;
        let remote_version = match &installed.source {
            PackageSource::Git(git) => match self.fetch_remote_version(git) {
                Ok(v) => v,
                Err(e) => {
                    println!("Skipping update for '{name}': {e}");
                    return Ok(());
                }
            },
            PackageSource::Path(local) => {
                println!("Skipping update for '{name}' (path repository: no remote version available: {})", local.path);
                return Ok(());
            }
        };

        match semver_cmp(installed_version, &remote_version) {
            std::cmp::Ordering::Less => {
                println!(
                    "Updating '{name}' from v{installed_version} to v{remote_version}..."
                );
                self.remove(name)?;
                self.install(name)?;
                println!("Updated '{name}' to v{remote_version}.");
            }
            std::cmp::Ordering::Equal => {
                println!("'{name}' is up to date (v{installed_version}).");
            }
            std::cmp::Ordering::Greater => {
                println!(
                    "'{name}' is ahead of remote (local v{installed_version}, remote v{remote_version})."
                );
            }
        }
        Ok(())
    }

    /// Fetch the latest package version from a remote Git repository by
    /// cloning to a temp dir and reading package.json.
    fn fetch_remote_version(&self, git: &crate::pkg::types::GitSource) -> LppoResult<String> {
        use std::process::Command;

        let safe_name: String = git
            .url
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        let temp_dir = std::env::temp_dir().join(format!("lppo_update_{safe_name}"));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).map_err(|e| {
            LppoError::source_retrieval_failed(
                "update".to_string(),
                format!("failed to create temp dir: {e}"),
            )
        })?;

        let cache_dir = temp_dir.join("cache");
        fs::create_dir_all(&cache_dir).map_err(|e| {
            LppoError::source_retrieval_failed(
                "update".to_string(),
                format!("failed to create cache dir: {e}"),
            )
        })?;

        let status = Command::new("git")
            .current_dir(&cache_dir)
            .arg("init")
            .arg("--bare")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| {
                LppoError::source_retrieval_failed(
                    "update".to_string(),
                    format!("git init --bare failed: {e}"),
                )
            })?;
        if !status.success() {
            return Err(LppoError::source_retrieval_failed(
                "update".to_string(),
                "git init --bare failed".to_string(),
            ));
        }

        let _ = Command::new("git")
            .current_dir(&cache_dir)
            .arg("remote")
            .arg("add")
            .arg("origin")
            .arg(&git.url)
            .status();
        let status = Command::new("git")
            .current_dir(&cache_dir)
            .arg("fetch")
            .arg("--force")
            .arg("origin")
            .arg(&git.url)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| {
                LppoError::source_retrieval_failed(
                    "update".to_string(),
                    format!("git fetch failed: {e}"),
                )
            })?;
        if !status.success() {
            return Err(LppoError::source_retrieval_failed(
                "update".to_string(),
                format!("git fetch from '{}' failed", git.url),
            ));
        }

        let ref_spec = Self::resolve_git_ref_for_update(&cache_dir, git)?;

        let clone_dir = temp_dir.join("clone");
        let status = Command::new("git")
            .current_dir(&cache_dir)
            .arg("clone")
            .arg("--no-local")
            .arg("--branch")
            .arg(&ref_spec)
            .arg("--single-branch")
            .arg("--depth")
            .arg("1")
            .arg(".")
            .arg(&clone_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| {
                LppoError::source_retrieval_failed(
                    "update".to_string(),
                    format!("git clone failed: {e}"),
                )
            })?;
        if !status.success() {
            return Err(LppoError::source_retrieval_failed(
                "update".to_string(),
                format!("git clone of '{}' failed", git.url),
            ));
        }

        let source_root = match &git.subdirectory {
            Some(sub) => clone_dir.join(sub),
            None => clone_dir.clone(),
        };

        let manifest_path = if source_root.join("lppo_package.json").is_file() {
            source_root.join("lppo_package.json")
        } else {
            source_root.join("package.json")
        };

        let version = if manifest_path.is_file() {
            let contents = fs::read_to_string(&manifest_path).map_err(|e| {
                LppoError::source_retrieval_failed(
                    "update".to_string(),
                    format!("failed to read package.json: {e}"),
                )
            })?;
            try_parse_manifest(&contents)
                .map(|m| m.package.version)
                .ok_or_else(|| {
                    LppoError::source_retrieval_failed(
                        "update".to_string(),
                        format!("failed to parse package.json in '{}'", git.url),
                    )
                })?
        } else {
            // Scan subdirectories for package.json.
            let mut found = None;
            if let Ok(entries) = fs::read_dir(&source_root) {
                for entry in entries.flatten() {
                    let sub = entry.path();
                    if sub.is_dir() {
                        for mn in ["package.json", "lppo_package.json"] {
                            let mp = sub.join(mn);
                            if mp.is_file() {
                                if let Ok(contents) = fs::read_to_string(&mp) {
                                    if let Some(manifest) = try_parse_manifest(&contents) {
                                        found = Some(manifest.package.version);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    if found.is_some() {
                        break;
                    }
                }
            }
            found.ok_or_else(|| {
                LppoError::source_retrieval_failed(
                    "update".to_string(),
                    format!("package.json not found in repository '{}'", git.url),
                )
            })?
        };

        let _ = fs::remove_dir_all(&temp_dir);
        Ok(version)
    }

    /// Resolve a git ref for update checking (mirrors SourceRetriever logic).
    fn resolve_git_ref_for_update(
        cache_dir: &Path,
        git: &crate::pkg::types::GitSource,
    ) -> LppoResult<String> {
        if let Some(commit) = &git.commit {
            Ok(commit.clone())
        } else if let Some(tag) = &git.tag {
            Ok(format!("refs/tags/{tag}"))
        } else if let Some(branch) = &git.branch {
            Ok(format!("refs/heads/{branch}"))
        } else {
            Ok("refs/heads/main".to_string())
        }
    }

    /// Sync the remote repositories.json from GitHub into the local context.
    fn sync_remote_repositories(&mut self) -> LppoResult<()> {
        let raw_url = "https://raw.githubusercontent.com/AlexMatzen01/LPPO/main/repositories.json";
        println!("Fetching remote repositories from {raw_url}...");
        let output = std::process::Command::new("curl")
            .arg("-sL")
            .arg(raw_url)
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let contents = String::from_utf8_lossy(&out.stdout);
                if let Ok(repos) = serde_json::from_str::<Repositories>(&contents) {
                    let mut local_repos = repos.clone();
                    local_repos.save_with(&self.context)?;
                    println!("Updated repositories from GitHub.");
                    for r in &repos.repositories {
                        println!("  {} ({}) - {}", r.name, r.r#type, r.url);
                    }
                } else {
                    println!("Warning: could not parse remote repositories.json.");
                }
            } else {
                println!("Warning: could not fetch remote repositories (curl failed).");
            }
        } else {
            println!("Warning: `curl` not available, skipping remote repository sync.");
        }
        Ok(())
    }

    /// Build a package (and dependencies) without marking installed.
    pub fn build(&mut self, name: &str) -> LppoResult<()> {
        self.reload_tracker()?;
        self.tracker
            .get(name)
            .ok_or_else(|| LppoError::package_not_found(name.to_string()))?;

        let module_dir = package_module_dir(&self.context, name)?;
        let manifest_path = module_dir.join("Cargo.toml");
        if !manifest_path.is_file() {
            return Err(LppoError::build_failed(
                name.to_string(),
                format!("Cargo.toml not found at {}", manifest_path.display()),
            ));
        }

        // Read package.json for build config.
        let manifest = load_package_manifest(&module_dir)?;
        let build_config = BuildConfig {
            system: manifest.package.build.system.clone(),
            manifest: manifest.package.build.manifest.clone(),
            features: manifest.package.build.features.clone(),
            release: manifest.package.build.release,
            target: None,
        };

        let artifact_path = CargoBuilder::build(&module_dir, &build_config)?;
        if let Some(artifact) = &artifact_path {
            println!("Built {name} -> {}", artifact.display());
        } else {
            println!("Built {name} (no library artifact located)");
        }
        Ok(())
    }

    /// Returns the installed tracker (read-only access).
    pub fn tracker(&self) -> &InstalledTracker {
        &self.tracker
    }

    /// Returns the context path root for display/debugging.
    pub fn context_root(&self) -> LppoResult<PathBuf> {
        crate::pkg::library::lppo_dir(&self.context)
    }
}

/// Recursively copy the contents of `src_dir` into `dst_dir`.
fn copy_dir_contents(src_dir: &Path, dst_dir: &Path) -> LppoResult<()> {
    for entry in fs::read_dir(src_dir).map_err(|e| {
        LppoError::io_error(format!(
            "failed to read source dir '{}': {}",
            src_dir.display(),
            e
        ))
    })? {
        let entry = entry.map_err(|e| {
            LppoError::io_error(format!("failed to read dir entry: {e}"))
        })?;
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst_dir.join(file_name);

        if src_path.is_dir() {
            fs::create_dir_all(&dst_path).map_err(|e| {
                LppoError::io_error(format!(
                    "failed to create dir '{}': {}",
                    dst_path.display(),
                    e
                ))
            })?;
            copy_dir_contents(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| {
                LppoError::io_error(format!(
                    "failed to copy '{}' to '{}': {}",
                    src_path.display(),
                    dst_path.display(),
                    e
                ))
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pkg::types::{GitSource, PackageSource};
    use std::collections::BTreeMap;

    #[test]
    fn search_returns_matching_packages() {
        let mut tracker = InstalledTracker::default();
        tracker.db.insert(InstalledPackage {
            name: "LPPX2D".to_string(),
            version: "1.0.0".to_string(),
            source: PackageSource::Git(crate::pkg::types::GitSource {
                url: "https://example.com/LPPX".to_string(),
                branch: None,
                tag: None,
                commit: None,
                subdirectory: None,
            }),
            dependencies: BTreeMap::new(),
            build: BuildConfig::default(),
            source_path: PathBuf::from("/tmp/LPPX2D"),
            artifact_path: None,
            installed_at: None,
            interface: None,
        });
        let pm = PackageManager {
            context: PackageContext::Global,
            tracker,
        };
        let results = pm.search("2D");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "LPPX2D");
        let results = pm.search("nonexistent");
        assert!(results.is_empty());
    }
}

/// Compare two semantic version strings. Returns Ordering::Less if a < b,
/// Equal if a == b, Greater if a > b. Falls back to string comparison if
/// either version fails to parse as SemVer.
fn semver_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    match (
        crate::pkg::types::SemVer::parse(a),
        crate::pkg::types::SemVer::parse(b),
    ) {
        (Ok(va), Ok(vb)) => va.cmp(&vb),
        _ => a.cmp(b),
    }
}
