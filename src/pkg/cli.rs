//! CLI entry point for the `lppo pkg` subcommand.
//!
//! Example usage:
//!
//! ```text
//! lppo pkg repo list
//! lppo pkg repo add myrepo https://github.com/user/packages.git git
//! lppo pkg repo remove myrepo
//! lppo pkg install mypackage
//! lppo pkg list
//! lppo pkg info mypackage
//! lppo pkg remove mypackage
//! ```

use crate::error::{LppoError, LppoResult};
use crate::pkg::manager::PackageManager;
use crate::pkg::types::{PackagesIndex, Repositories, Repository};
use std::path::Path;

/// Subcommands supported by `lppo pkg`.
#[derive(Debug, Clone)]
pub enum PkgSubcommand {
    Search { query: String },
    Install { target: String },
    Remove { name: String },
    List,
    Info { name: String },
    Build { name: String },
    /// Repository management
    RepoList,
    RepoAdd { name: String, url: String, r#type: String },
    RepoRemove { name: String },
}

impl PkgSubcommand {
    /// Parse `lppo pkg <args...>` into a subcommand.
    pub fn parse(args: &[String]) -> LppoResult<Self> {
        let Some(sub) = args.first() else {
            return Err(LppoError::io_error(
                "missing pkg subcommand. Usage: lppo pkg <search|install|remove|list|info|build|repo> ...".to_string(),
            ));
        };
        match sub.as_str() {
            "search" => {
                let query = args.get(1).ok_or_else(|| {
                    LppoError::io_error("usage: lppo pkg search <query>".to_string())
                })?;
                Ok(PkgSubcommand::Search {
                    query: query.clone(),
                })
            }
            "install" => {
                let target = args.get(1).ok_or_else(|| {
                    LppoError::io_error("usage: lppo pkg install <package>".to_string())
                })?;
                Ok(PkgSubcommand::Install {
                    target: target.clone(),
                })
            }
            "remove" => {
                let name = args.get(1).ok_or_else(|| {
                    LppoError::io_error("usage: lppo pkg remove <package>".to_string())
                })?;
                Ok(PkgSubcommand::Remove {
                    name: name.clone(),
                })
            }
            "list" => Ok(PkgSubcommand::List),
            "info" => {
                let name = args.get(1).ok_or_else(|| {
                    LppoError::io_error("usage: lppo pkg info <package>".to_string())
                })?;
                Ok(PkgSubcommand::Info {
                    name: name.clone(),
                })
            }
            "build" => {
                let name = args.get(1).ok_or_else(|| {
                    LppoError::io_error("usage: lppo pkg build <package>".to_string())
                })?;
                Ok(PkgSubcommand::Build {
                    name: name.clone(),
                })
            }
            "repo" => {
                let repo_sub = args.get(1).ok_or_else(|| {
                    LppoError::io_error(
                        "usage: lppo pkg repo <list|add|remove> ...".to_string(),
                    )
                })?;
                match repo_sub.as_str() {
                    "list" => Ok(PkgSubcommand::RepoList),
                    "add" => {
                        let name = args.get(2).ok_or_else(|| {
                            LppoError::io_error("usage: lppo pkg repo add <name> <url> <git|path>".to_string())
                        })?;
                        let url = args.get(3).ok_or_else(|| {
                            LppoError::io_error("usage: lppo pkg repo add <name> <url> <git|path>".to_string())
                        })?;
                        let r#type = args.get(4).ok_or_else(|| {
                            LppoError::io_error("usage: lppo pkg repo add <name> <url> <git|path>".to_string())
                        })?;
                        Ok(PkgSubcommand::RepoAdd {
                            name: name.clone(),
                            url: url.clone(),
                            r#type: r#type.clone(),
                        })
                    }
                    "remove" => {
                        let name = args.get(2).ok_or_else(|| {
                            LppoError::io_error("usage: lppo pkg repo remove <name>".to_string())
                        })?;
                        Ok(PkgSubcommand::RepoRemove {
                            name: name.clone(),
                        })
                    }
                    other => Err(LppoError::io_error(format!(
                        "unknown repo subcommand '{other}'. Expected one of: list, add, remove"
                    ))),
                }
            }
            other => Err(LppoError::io_error(format!(
                "unknown pkg subcommand '{other}'. Expected one of: search, install, remove, list, info, build, repo"
            ))),
        }
    }

    /// Run the subcommand, auto-detecting the package context from the
    /// current directory (project-local `.lppo/` if inside an LPPO project,
    /// otherwise global `~/.LPPO/`).
    pub fn run(self) -> LppoResult<()> {
        let cwd = std::env::current_dir()
            .unwrap_or_else(|_| Path::new(".").to_path_buf());
        let context = crate::pkg::library::PackageContext::discover(&cwd)?;
        self.run_with_context(context)
    }

    /// Run the subcommand in the given package context.
    pub fn run_with_context(self, context: crate::pkg::library::PackageContext) -> LppoResult<()> {
        match self {
            PkgSubcommand::RepoList => {
                let repos = Repositories::load_with(&context)?;
                if repos.repositories.is_empty() {
                    println!("No repositories configured.");
                } else {
                    println!("Configured repositories:");
                    for r in &repos.repositories {
                        println!(
                            "  {} ({}) - {} {}",
                            r.name,
                            r.r#type,
                            r.url,
                            r.description.as_deref().map(|d| format!("- {d}")).unwrap_or_default()
                        );
                    }
                }
            }
            PkgSubcommand::RepoAdd { name, url, r#type } => {
                let mut repos = Repositories::load_with(&context)?;
                let repo = Repository {
                    name: name.clone(),
                    url,
                    r#type,
                    description: None,
                };
                if repos.add(repo) {
                    repos.save_with(&context)?;
                    println!("Added repository '{name}'.");
                } else {
                    println!("Repository '{name}' already exists.");
                }
            }
            PkgSubcommand::RepoRemove { name } => {
                let mut repos = Repositories::load_with(&context)?;
                if repos.remove(&name) {
                    repos.save_with(&context)?;
                    println!("Removed repository '{name}'.");
                } else {
                    println!("Repository '{name}' not found.");
                }
            }
            PkgSubcommand::Search { query } => {
                let pm = PackageManager::load_with_context(context.clone())?;
                let results = pm.search_available(&query);
                if results.is_empty() {
                    println!("No packages matching '{query}'.");
                } else {
                    println!("Packages matching '{query}':");
                    for (id, version, installed) in results {
                        let status = if installed { " (installed)" } else { "" };
                        println!("  {} {}{}", id, version, status);
                    }
                }
            }
            PkgSubcommand::Install { target } => {
                let mut pm = PackageManager::load_with_context(context.clone())?;
                pm.install(&target)?;
                // Regenerate packages index after install.
                PackagesIndex::regenerate_with(&context)?;
            }
            PkgSubcommand::Remove { name } => {
                let mut pm = PackageManager::load_with_context(context.clone())?;
                pm.remove(&name)?;
                PackagesIndex::regenerate_with(&context)?;
            }
            PkgSubcommand::List => {
                let index = PackagesIndex::load_with(&context)?;
                if index.packages.is_empty() {
                    println!("No packages installed.");
                } else {
                    println!("Installed packages:");
                    for (id, entry) in &index.packages {
                        println!("  {} v{} ({})", id, entry.version, entry.path.display());
                    }
                }
            }
            PkgSubcommand::Info { name } => {
                let pm = PackageManager::load_with_context(context.clone())?;
                // First try installed packages.
                if let Ok(entry) = pm.info(&name) {
                    println!("Package: {}", entry.name);
                    println!("  Version: {}", entry.version);
                    println!("  Source: {:?}", entry.source);
                    println!("  Path: {}", entry.source_path.display());
                    if !entry.dependencies.is_empty() {
                        println!("  Dependencies:");
                        for (dep, req) in &entry.dependencies {
                            println!("    {dep}: {req}");
                        }
                    }
                    return Ok(());
                }
                // Fall back to searching repositories for available packages.
                let results = pm.search_available(&name);
                let matches: Vec<&(String, String, bool)> = results
                    .iter()
                    .filter(|(id, _, _)| id == &name)
                    .collect();
                if let Some((id, version, _)) = matches.first() {
                    println!("Package: {id}");
                    println!("  Version: {version}");
                    println!("  Status: available (not installed)");
                    println!("  Run 'lppo pkg install {id}' to install it.");
                    return Ok(());
                }
                return Err(LppoError::package_not_found(name.clone()));
            }
            PkgSubcommand::Build { name } => {
                let mut pm = PackageManager::load_with_context(context.clone())?;
                pm.build(&name)?;
            }
        }
        Ok(())
    }
}
