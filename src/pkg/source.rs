//! Retrieves package source code from Git repositories and local paths into
//! the context's `packages/<name>` directory, honoring branches, tags,
//! commits, and subdirectories.

use crate::error::{LppoError, LppoResult};
use crate::pkg::library::{git_dir, packages_dir, PackageContext};
use crate::pkg::types::PackageSource;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Retrieves package source code and stages it under the context's
/// `packages/<name>` directory.
pub struct SourceRetriever;

impl SourceRetriever {
    /// Retrieve the package source described by `source` into the context's
    /// `packages/<name>` directory. If the package is already staged and the
    /// source revision matches, it is reused. Returns the path to the staged
    /// package directory (containing its Cargo.toml).
    pub fn retrieve_with_context(
        context: &PackageContext,
        name: &str,
        source: &PackageSource,
    ) -> LppoResult<PathBuf> {
        let staged_dir = packages_dir(context)?.join(name);
        let marker = staged_dir.join(".lppo_source_ok");

        // If already staged with a matching source marker, reuse it.
        if staged_dir.is_dir() && marker.is_file() {
            if let Ok(cached) = fs::read_to_string(&marker) {
                if cached == source_description(source) {
                    return Ok(staged_dir);
                }
            }
        }

        // Remove any stale staged directory.
        if staged_dir.exists() {
            fs::remove_dir_all(&staged_dir).map_err(|e| {
                LppoError::io_error(format!(
                    "failed to remove stale package dir '{}': {}",
                    staged_dir.display(),
                    e
                ))
            })?;
        }

        match source {
            PackageSource::Git(git) => Self::retrieve_git(context, name, git)?,
            PackageSource::Path(local) => Self::retrieve_local(context, name, local)?,
        }

        // Write the source marker so future retrievals can skip re-downloads
        // when the source description is unchanged.
        fs::create_dir_all(&staged_dir).map_err(|e| {
            LppoError::io_error(format!(
                "failed to create package dir '{}': {}",
                staged_dir.display(),
                e
            ))
        })?;
        fs::write(&marker, source_description(source)).map_err(|e| {
            LppoError::io_error(format!("failed to write source marker: {e}"))
        })?;

        Ok(staged_dir)
    }

    /// Retrieve a package from a Git repository.
    fn retrieve_git(
        context: &PackageContext,
        name: &str,
        git: &crate::pkg::types::GitSource,
    ) -> LppoResult<()> {
        let git_cache_root = git_dir(context)?;
        // Use a sanitized directory name derived from the repo URL.
        let cache_name = sanitize_dir_name(&git.url);
        let cache_dir = git_cache_root.join(cache_name);

        // Clone (or fetch) the bare-ish repository into the cache.
        if !cache_dir.is_dir() {
            fs::create_dir_all(&cache_dir).map_err(|e| {
                LppoError::io_error(format!(
                    "failed to create git cache dir '{}': {}",
                    cache_dir.display(),
                    e
                ))
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
                        name.to_string(),
                        format!("git is not available or failed to init cache: {e}"),
                    )
                })?;
            if !status.success() {
                return Err(LppoError::source_retrieval_failed(
                    name.to_string(),
                    format!("git init --bare failed in {}", cache_dir.display()),
                ));
            }
        }

        // Fetch the latest refs.
        let mut fetch = Command::new("git");
        fetch
            .current_dir(&cache_dir)
            .arg("fetch")
            .arg("--force")
            .arg("origin")
            .arg(&git.url);
        // Only add the URL as a remote if origin isn't configured.
        let remote_check = Command::new("git")
            .current_dir(&cache_dir)
            .arg("remote")
            .arg("get-url")
            .arg("origin")
            .output();
        if let Ok(out) = remote_check {
            if !out.status.success() {
                let _ = Command::new("git")
                    .current_dir(&cache_dir)
                    .arg("remote")
                    .arg("add")
                    .arg("origin")
                    .arg(&git.url)
                    .status();
            }
        }
        let status = fetch
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| {
                LppoError::source_retrieval_failed(
                    name.to_string(),
                    format!("git fetch failed: {e}"),
                )
            })?;
        if !status.success() {
            return Err(LppoError::source_retrieval_failed(
                name.to_string(),
                format!("git fetch from '{}' failed", git.url),
            ));
        }

        // Resolve the ref to check out.
        let ref_spec = Self::resolve_git_ref(&cache_dir, git)?;

        // Clone the specific ref into a working tree in a temp location, then
        // copy the requested subdirectory into the staged package dir.
        let staged_dir = packages_dir(context)?.join(name);
        let tmp_clone = cache_dir.join(format!("worktree-{name}"));
        let _ = fs::remove_dir_all(&tmp_clone);

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
            .arg(&tmp_clone)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| {
                LppoError::source_retrieval_failed(
                    name.to_string(),
                    format!("git clone failed: {e}"),
                )
            })?;
        if !status.success() {
            // Fall back to a full clone if shallow/ref clone failed.
            let status = Command::new("git")
                .current_dir(&cache_dir)
                .arg("clone")
                .arg(".")
                .arg(&tmp_clone)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map_err(|e| {
                    LppoError::source_retrieval_failed(
                        name.to_string(),
                        format!("git clone fallback failed: {e}"),
                    )
                })?;
            if !status.success() {
                return Err(LppoError::source_retrieval_failed(
                    name.to_string(),
                    format!("git clone of '{}' failed", git.url),
                ));
            }
        }

        // Determine the source subdirectory within the clone.
        let source_root = match &git.subdirectory {
            Some(sub) => tmp_clone.join(sub),
            None => tmp_clone.clone(),
        };
        if !source_root.is_dir() {
            let _ = fs::remove_dir_all(&tmp_clone);
            return Err(LppoError::source_retrieval_failed(
                name.to_string(),
                format!(
                    "source subdirectory '{}' not found in repository '{}'",
                    git.subdirectory.as_deref().unwrap_or("."),
                    git.url
                ),
            ));
        }

        // Copy the package source into the staged directory.
        copy_dir(&source_root, &staged_dir).map_err(|e| {
            LppoError::source_retrieval_failed(
                name.to_string(),
                format!("failed to copy package source: {e}"),
            )
        })?;
        let _ = fs::remove_dir_all(&tmp_clone);

        Ok(())
    }

    /// Resolve a branch/tag/commit to a refspec suitable for `git clone --branch`.
    fn resolve_git_ref(cache_dir: &Path, git: &crate::pkg::types::GitSource) -> LppoResult<String> {
        // Commit takes priority, then tag, then branch, then default main/master.
        if let Some(commit) = &git.commit {
            // Verify the commit exists in the cache.
            let out = Command::new("git")
                .current_dir(cache_dir)
                .arg("cat-file")
                .arg("-t")
                .arg(commit)
                .output()
                .map_err(|e| {
                    LppoError::source_retrieval_failed(
                        "__internal__".to_string(),
                        format!("git cat-file failed: {e}"),
                    )
                })?;
            if !out.status.success() {
                return Err(LppoError::source_retrieval_failed(
                    "__internal__".to_string(),
                    format!("commit '{commit}' not found in repository"),
                ));
            }
            return Ok(commit.clone());
        }
        if let Some(tag) = &git.tag {
            return Ok(format!("refs/tags/{tag}"));
        }
        if let Some(branch) = &git.branch {
            return Ok(format!("refs/heads/{branch}"));
        }
        // Default: try main, then master.
        Ok("refs/heads/main".to_string())
    }

    /// Retrieve a package from a local filesystem path.
    fn retrieve_local(
        context: &PackageContext,
        name: &str,
        local: &crate::pkg::types::LocalSource,
    ) -> LppoResult<()> {
        let src = Path::new(&local.path);
        if !src.is_dir() {
            return Err(LppoError::source_retrieval_failed(
                name.to_string(),
                format!("local source path '{}' is not a directory", local.path),
            ));
        }
        let staged_dir = packages_dir(context)?.join(name);
        copy_dir(src, &staged_dir).map_err(|e| {
            LppoError::source_retrieval_failed(
                name.to_string(),
                format!("failed to copy local package source: {e}"),
            )
        })?;
        Ok(())
    }

    /// Install a package directly from a Git repository URL (CLI-driven).
    pub fn install_from_git_url(name: Option<&str>, url: &str) -> LppoResult<PathBuf> {
        let package_name = match name {
            Some(n) => n.to_string(),
            None => url
                .trim_end_matches(".git")
                .rsplit('/')
                .next()
                .unwrap_or("package")
                .to_string(),
        };
        let source = PackageSource::Git(crate::pkg::types::GitSource {
            url: url.to_string(),
            branch: None,
            tag: None,
            commit: None,
            subdirectory: None,
        });
        Self::retrieve_with_context(&PackageContext::Global, &package_name, &source)
    }

    /// Convenience: retrieve into the global package store.
    pub fn retrieve(name: &str, source: &PackageSource) -> LppoResult<PathBuf> {
        Self::retrieve_with_context(&PackageContext::Global, name, source)
    }
}

/// A human-readable, stable description of a source for cache invalidation.
fn source_description(source: &PackageSource) -> String {
    match source {
        PackageSource::Git(git) => {
            format!(
                "git|{}|{}|{}|{}|{}",
                git.url,
                git.branch.as_deref().unwrap_or(""),
                git.tag.as_deref().unwrap_or(""),
                git.commit.as_deref().unwrap_or(""),
                git.subdirectory.as_deref().unwrap_or("")
            )
        }
        PackageSource::Path(local) => format!("path|{}", local.path),
    }
}

/// Sanitize a repository URL into a cache directory name.
fn sanitize_dir_name(url: &str) -> String {
    let mut out = String::with_capacity(url.len());
    for ch in url.chars() {
        if ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    out
}

/// Recursively copy a directory tree.
fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pkg::types::GitSource;

    #[test]
    fn source_description_is_deterministic() {
        let a = PackageSource::Git(GitSource {
            url: "https://example.com/repo".to_string(),
            branch: Some("main".to_string()),
            tag: None,
            commit: None,
            subdirectory: Some("LPPX2D".to_string()),
        });
        let b = PackageSource::Git(GitSource {
            url: "https://example.com/repo".to_string(),
            branch: Some("main".to_string()),
            tag: None,
            commit: None,
            subdirectory: Some("LPPX2D".to_string()),
        });
        assert_eq!(source_description(&a), source_description(&b));
    }

    #[test]
    fn sanitize_dir_name_replaces_unsafe_chars() {
        let clean = sanitize_dir_name("https://github.com/user/repo.git");
        assert!(!clean.contains("://"));
        assert!(!clean.contains('/'));
    }
}
