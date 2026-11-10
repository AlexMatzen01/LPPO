//! Installed package tracking backed by `~/.LPPO/installed.json` (global) or
//! `<project>/.lppo/installed.json` (project-local).
//!
//! This is distinct from the package library index (`packages.json`). It
//! records what is installed locally, including resolved versions, source
//! revisions, dependencies, and build artifacts.

use crate::error::{LppoError, LppoResult};
use crate::pkg::library::{
    current_timestamp, installed_json_path_with, load_package_interface, PackageContext,
};
use crate::pkg::types::{
    BuildConfig, InstalledDb, InstalledPackage, PackageInterface, PackageSource,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Tracks locally installed packages.
pub struct InstalledTracker {
    pub(crate) context: PackageContext,
    pub(crate) db: InstalledDb,
}

impl InstalledTracker {
    pub fn load_with(context: PackageContext) -> LppoResult<Self> {
        let db = InstalledDb::load_with(&context)?;
        Ok(Self { context, db })
    }

    /// Load the global installed database.
    pub fn load() -> LppoResult<Self> {
        Self::load_with(PackageContext::Global)
    }

    pub fn save_with(&self, context: &PackageContext) -> LppoResult<()> {
        self.db.save_with(context)
    }

    /// Save the global installed database.
    pub fn save(&self) -> LppoResult<()> {
        self.save_with(&PackageContext::Global)
    }

    /// Returns the names of all installed packages.
    pub fn installed_names(&self) -> Vec<String> {
        self.db.package_names()
    }

    /// Check whether a package is installed.
    pub fn is_installed(&self, name: &str) -> bool {
        self.db.get(name).is_some()
    }

    /// Look up an installed package record.
    pub fn get(&self, name: &str) -> Option<&InstalledPackage> {
        self.db.get(name)
    }

    /// Returns all installed package records.
    pub fn all_packages(&self) -> Vec<&InstalledPackage> {
        self.db.packages.values().collect()
    }

    /// Returns names of installed packages that depend on `name` (excluding
    /// `name` itself).
    pub fn dependents_of(&self, name: &str) -> Vec<String> {
        self.db
            .packages
            .iter()
            .filter_map(|(other, entry)| {
                if other != name && entry.dependencies.contains_key(name) {
                    Some(other.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Record a package as installed after a successful build.
    pub fn mark_installed_with_context(
        &mut self,
        context: &PackageContext,
        name: &str,
        version: &str,
        source: PackageSource,
        dependencies: &BTreeMap<String, String>,
        build: BuildConfig,
        source_path: PathBuf,
        artifact_path: Option<PathBuf>,
    ) -> LppoResult<()> {
        let interface = load_package_interface(&source_path).ok().flatten();
        let entry = InstalledPackage {
            name: name.to_string(),
            version: version.to_string(),
            source,
            dependencies: dependencies.clone(),
            build,
            source_path,
            artifact_path,
            installed_at: Some(current_timestamp()),
            interface,
        };
        self.db.insert(entry);
        self.save_with(context)
    }

    /// Record a package as installed (global context).
    pub fn mark_installed(
        &mut self,
        name: &str,
        version: &str,
        source: PackageSource,
        dependencies: &BTreeMap<String, String>,
        build: BuildConfig,
        source_path: PathBuf,
        artifact_path: Option<PathBuf>,
    ) -> LppoResult<()> {
        self.mark_installed_with_context(
            &PackageContext::Global,
            name,
            version,
            source,
            dependencies,
            build,
            source_path,
            artifact_path,
        )
    }

    /// Remove a package from the installed database for the given context.
    pub fn mark_removed_with_context(&mut self, context: &PackageContext, name: &str) -> LppoResult<()> {
        self.db.remove(name);
        self.save_with(context)
    }

    /// Remove a package from the installed database (global context).
    pub fn mark_removed(&mut self, name: &str) -> LppoResult<()> {
        self.mark_removed_with_context(&PackageContext::Global, name)
    }

    /// Update the artifact path for an installed package.
    pub fn set_artifact_path(&mut self, name: &str, path: PathBuf) -> LppoResult<()> {
        if let Some(entry) = self.db.packages.get_mut(name) {
            entry.artifact_path = Some(path);
            return self.save_with(&self.context);
        }
        Err(LppoError::package_not_found(name.to_string()))
    }

    /// Clear the artifact path for an installed package (e.g. after removal).
    pub fn clear_artifact_path(&mut self, name: &str) -> LppoResult<()> {
        if let Some(entry) = self.db.packages.get_mut(name) {
            entry.artifact_path = None;
            return self.save_with(&self.context);
        }
        Err(LppoError::package_not_found(name.to_string()))
    }

    /// Returns installed packages whose interface exposes functions usable by LPPO.
    pub fn packages_with_interfaces(&self) -> Vec<(String, PackageInterface)> {
        let mut out = Vec::new();
        for (name, entry) in &self.db.packages {
            if let Some(iface) = &entry.interface {
                if !iface.functions.is_empty() {
                    out.push((name.clone(), iface.clone()));
                }
            }
        }
        out
    }
}

impl Default for InstalledTracker {
    fn default() -> Self {
        Self {
            context: PackageContext::Global,
            db: InstalledDb::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pkg::types::GitSource;
    use std::collections::BTreeMap;

    #[test]
    fn mark_installed_and_remove() {
        let mut tracker = InstalledTracker::default();
        let source = PackageSource::Git(GitSource {
            url: "https://example.com/LPPX".to_string(),
            branch: Some("main".to_string()),
            tag: None,
            commit: None,
            subdirectory: Some("LPPX2D".to_string()),
        });
        tracker
            .mark_installed(
                "LPPX2D",
                "1.0.0",
                source,
                &BTreeMap::new(),
                BuildConfig::default(),
                PathBuf::from("/tmp/LPPX2D"),
                None,
            )
            .unwrap();
        assert!(tracker.is_installed("LPPX2D"));
        assert_eq!(tracker.get("LPPX2D").unwrap().version, "1.0.0");
        tracker.mark_removed("LPPX2D").unwrap();
        assert!(!tracker.is_installed("LPPX2D"));
    }
}
