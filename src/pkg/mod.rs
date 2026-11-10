//! LPPO integrated package manager.
//!
//! This module provides the `lppo pkg` subcommands and the underlying
//! package library, dependency resolution, source retrieval, and Cargo build
//! integration. All packages are Rust/Cargo projects built through Cargo.

pub mod builder;
pub mod cli;
pub mod graphics_ir;
pub mod installed;
pub mod library;
pub mod manager;
pub mod native;
pub mod resolver;
pub mod source;
pub mod types;

pub use builder::CargoBuilder;
pub use cli::PkgSubcommand;
pub use installed::InstalledTracker;
pub use library::{
    builds_dir, cache_dir, config_json_path, config_json_path_with, current_timestamp, downloads_dir,
    ensure_dirs, git_dir, global_lppo_dir, global_lppo_modules_dir, global_packages_dir,
    installed_json_path, installed_json_path_with, lppo_dir, lppo_modules_dir,
    package_module_dir, packages_dir, packages_json_path, packages_json_path_with,
    repositories_json_path, repositories_json_path_with, load_package_interface,
    load_package_manifest, PackageContext,
};
pub use manager::PackageManager;
pub use native::{NativeFunction, NativeRegistry};
pub use resolver::DependencyResolver;
pub use source::SourceRetriever;
pub use types::{
    BuildConfig, InstalledDb, InstalledPackage, InstalledPackageEntry, InstallPlan,
    PackageBuild, PackageEntry, PackageFunction, PackageInterface, PackageLibrary, PackageManifest,
    PackageManifestData, PackageSource, PackagesIndex, Repository, Repositories, SemVer,
    TypeAnnotationRef, VersionReq,
};
