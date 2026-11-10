//! Cargo build integration.
//!
//! Builds LPPO packages (Rust/Cargo projects) through `cargo build`,
//! capturing output and locating the produced library artifact.

use crate::error::{LppoError, LppoResult};
use crate::pkg::types::BuildConfig;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Builds a single Cargo package and locates its library artifact.
pub struct CargoBuilder;

impl CargoBuilder {
    /// Build the package at `package_dir` (the directory containing Cargo.toml).
    ///
    /// Returns the path to the built library artifact (cdylib/rlib) when one
    /// is produced, plus any captured build output is printed to stderr/stdout
    /// as the command runs.
    pub fn build(package_dir: &Path, config: &BuildConfig) -> LppoResult<Option<PathBuf>> {
        let manifest = package_dir.join(&config.manifest);
        if !manifest.is_file() {
            return Err(LppoError::build_failed(
                package_dir
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                format!("Cargo.toml not found at {}", manifest.display()),
            ));
        }

        // Ensure a Cargo.lock exists (offline-friendly). `cargo build` generates
        // one if missing, but we try `cargo generate-lockfile` first when the
        // manifest has dependencies so the build is deterministic.
        let _ = Command::new("cargo")
            .current_dir(package_dir)
            .arg("generate-lockfile")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        let mut cargo = Command::new("cargo");
        cargo.current_dir(package_dir);
        cargo.arg("build");
        if config.release {
            cargo.arg("--release");
        }
        if !config.features.is_empty() {
            cargo.arg("--features");
            cargo.arg(config.features.join(","));
        }
        if let Some(target) = &config.target {
            cargo.arg("--target");
            cargo.arg(target);
        }
        cargo.stdout(Stdio::inherit());
        cargo.stderr(Stdio::inherit());

        let status = cargo.status().map_err(|e| {
            LppoError::build_failed(
                package_dir
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                format!("cargo is not available or failed to run: {e}"),
            )
        })?;

        if !status.success() {
            return Err(LppoError::build_failed(
                package_dir
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                "cargo build exited with non-zero status".to_string(),
            ));
        }

        // Locate the built library artifact.
        let profile = if config.release { "release" } else { "debug" };
        let artifact = Self::find_library_artifact(package_dir, profile);
        Ok(artifact)
    }

    /// Build multiple packages, returning each package's artifact path.
    pub fn build_many(
        packages: &[(String, PathBuf, BuildConfig)],
    ) -> LppoResult<Vec<(String, Option<PathBuf>)>> {
        let mut results = Vec::new();
        for (name, dir, config) in packages {
            let artifact = Self::build(dir, config)?;
            results.push((name.clone(), artifact));
        }
        Ok(results)
    }

    /// Locate the cdylib/rlib artifact produced by the build.
    fn find_library_artifact(package_dir: &Path, profile: &str) -> Option<PathBuf> {
        let target_dir = package_dir.join("target").join(profile);
        if !target_dir.is_dir() {
            return None;
        }
        // Preferred order: cdylib (dynamic), then rlib (static).
        let extensions = if cfg!(target_os = "windows") {
            ["dll", "rlib"]
        } else if cfg!(target_os = "macos") {
            ["dylib", "rlib"]
        } else {
            ["so", "rlib"]
        };
        // lib<crate_name>.<ext>. We don't know the crate name here, so scan.
        // Prefer the cdylib/shared artifact (.so/.dylib/.dll) over the static
        // rlib, so check extensions in priority order.
        if let Ok(entries) = fs::read_dir(&target_dir) {
            let files: Vec<_> = entries
                .flatten()
                .filter(|e| e.path().is_file())
                .collect();
            for ext in &extensions {
                let suffix = format!(".{ext}");
                if let Some(entry) = files.iter().find(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .to_string()
                        .starts_with("lib")
                        && e.file_name().to_string_lossy().ends_with(&suffix)
                }) {
                    return Some(entry.path());
                }
            }
        }
        None
    }

    /// Run `cargo build --release` in a workspace directory and forward output.
    pub fn build_release(package_dir: &Path) -> LppoResult<()> {
        let mut cargo = Command::new("cargo");
        cargo.current_dir(package_dir).arg("build").arg("--release");
        cargo.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        let status = cargo.status().map_err(|e| {
            LppoError::io_error(format!("failed to run cargo: {e}"))
        })?;
        if !status.success() {
            return Err(LppoError::io_error("cargo build --release failed".to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pkg::types::BuildConfig;

    #[test]
    fn build_manifest_missing_is_error() {
        let dir = std::env::temp_dir().join("lppo_pkg_no_manifest");
        let _ = fs::remove_dir_all(&dir);
        let config = BuildConfig::default();
        let result = CargoBuilder::build(&dir, &config);
        assert!(result.is_err());
    }

    #[test]
    fn find_library_artifact_scans_target_dir() {
        // Create a fake target/release dir with a libfoo.so.
        let dir = std::env::temp_dir().join("lppo_pkg_artifact_scan");
        let _ = fs::remove_dir_all(&dir);
        let release = dir.join("target").join("release");
        fs::create_dir_all(&release).unwrap();
        fs::write(release.join("libfoo.so"), b"fake").unwrap();
        let found = CargoBuilder::find_library_artifact(&dir, "release");
        assert!(found.is_some());
        let p = found.unwrap();
        assert!(p.file_name().unwrap().to_string_lossy().contains("libfoo.so"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_library_artifact_prefers_cdylib_over_rlib() {
        let dir = std::env::temp_dir().join("lppo_pkg_artifact_pref");
        let _ = fs::remove_dir_all(&dir);
        let release = dir.join("target").join("release");
        fs::create_dir_all(&release).unwrap();
        fs::write(release.join("libfoo.rlib"), b"fake-rlib").unwrap();
        fs::write(release.join("libfoo.so"), b"fake-so").unwrap();
        let found = CargoBuilder::find_library_artifact(&dir, "release");
        assert!(found.is_some());
        let p = found.unwrap();
        assert!(p.extension().and_then(|e| e.to_str()) == Some("so"));
        let _ = fs::remove_dir_all(&dir);
    }
}
