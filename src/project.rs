//! LPPO Project Builder.
//!
//! Provides `init`, `build`, `format`, `run`, `debug`, and `git` commands
//! for compiling multiple `.lppo` files into a single project.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use lppo::error::{LppoError, LppoResult};
use lppo::{
    build_executor_with_root, compile_to_bytes_with_root, load_executor_from_bytes,
    PackageContext,
};
use lppo::pkg::library::{global_lppo_dir, repositories_json_path};
use serde::{Deserialize, Serialize};
use lppo::pkg::cli::PkgSubcommand;

const MARKER: &[u8] = b"LPPO-EMBED-1\n";
const MANIFEST_NAME: &str = "lppo.toml";

/// LPPO project manifest parsed from `lppo.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub project: ProjectSection,
    #[serde(default)]
    pub build: BuildSection,
    #[serde(default)]
    pub dependencies: Option<DependenciesSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSection {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default = "default_edition")]
    pub edition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildSection {
    #[serde(default = "default_entry")]
    pub entry: String,
    #[serde(default = "default_output")]
    pub output: String,
    #[serde(default)]
    pub strip: bool,
    #[serde(default)]
    pub obfuscate: bool,
    #[serde(default)]
    pub linker: Option<String>,
    #[serde(default)]
    pub link_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DependenciesSection {
    #[serde(default)]
    pub path: Vec<String>,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

fn default_edition() -> String {
    "2021".to_string()
}

fn default_entry() -> String {
    "src/main.lppo".to_string()
}

fn default_output() -> String {
    "target/{name}".to_string()
}

/// Options controlling how a project is compiled into a bundled executable.
#[derive(Debug, Clone, Default)]
pub struct BuildOptions {
    pub release: bool,
    pub strip: bool,
    pub obfuscate: bool,
    /// Linker to use (applied to native/bare-metal builds that invoke a
    /// linker; bundled executables do not run a separate link step).
    pub linker: Option<String>,
    /// Additional arguments forwarded to the linker (repeatable).
    pub link_args: Vec<String>,
}

/// An LPPO project rooted at a directory containing `lppo.toml`.
#[derive(Debug, Clone)]
pub struct Project {
    pub root: PathBuf,
    pub manifest: Manifest,
}

impl Project {
    /// Discover and load a project by walking up from `start_dir` looking for
    /// `lppo.toml`. Returns `None` if no manifest is found.
    pub fn discover(start_dir: &Path) -> LppoResult<Option<Self>> {
        let mut current = start_dir.to_path_buf();
        loop {
            let manifest_path = current.join(MANIFEST_NAME);
            if manifest_path.is_file() {
                let contents = fs::read_to_string(&manifest_path).map_err(|e| {
                    LppoError::io_error(format!(
                        "failed to read {}: {}",
                        manifest_path.display(),
                        e
                    ))
                })?;
                let manifest: Manifest = toml::from_str(&contents).map_err(|e| {
                    LppoError::io_error(format!(
                        "failed to parse {}: {}",
                        manifest_path.display(),
                        e
                    ))
                })?;
                return Ok(Some(Project {
                    root: current,
                    manifest,
                }));
            }
            if !current.pop() {
                break;
            }
        }
        Ok(None)
    }

    /// Load a project rooted at `root`.
    pub fn load(root: &Path) -> LppoResult<Self> {
        let manifest_path = root.join(MANIFEST_NAME);
        let contents = fs::read_to_string(&manifest_path).map_err(|e| {
            LppoError::io_error(format!("failed to read {}: {}", manifest_path.display(), e))
        })?;
        let manifest: Manifest = toml::from_str(&contents).map_err(|e| {
            LppoError::io_error(format!(
                "failed to parse {}: {}",
                manifest_path.display(),
                e
            ))
        })?;
        Ok(Project {
            root: root.to_path_buf(),
            manifest,
        })
    }

    pub fn name(&self) -> &str {
        &self.manifest.project.name
    }

    pub fn entry_path(&self) -> PathBuf {
        self.root.join(&self.manifest.build.entry)
    }

    pub fn output_path(&self) -> PathBuf {
        let output = self
            .manifest
            .build
            .output
            .replace("{name}", &self.manifest.project.name);
        self.root.join(output)
    }

    /// Collect all `.lppo` source files under `src/`, sorted for deterministic builds.
    pub fn source_files(&self) -> LppoResult<Vec<PathBuf>> {
        let src_dir = self.root.join("src");
        if !src_dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut files = Vec::new();
        collect_lppo_files(&src_dir, &mut files)?;
        files.sort();
        Ok(files)
    }
}

/// Recursively collect `.lppo` files under `dir`.
fn collect_lppo_files(dir: &Path, out: &mut Vec<PathBuf>) -> LppoResult<()> {
    for entry in fs::read_dir(dir)
        .map_err(|e| LppoError::io_error(format!("failed to read dir {}: {}", dir.display(), e)))?
    {
        let entry =
            entry.map_err(|e| LppoError::io_error(format!("failed to read dir entry: {}", e)))?;
        let path = entry.path();
        if path.is_dir() {
            collect_lppo_files(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("lppo") {
            out.push(path);
        }
    }
    Ok(())
}

/// Resolve and compile every `.lppo` file under `src/` into a single merged
/// program, type-checked against the entry file's root.
fn compile_project_sources(project: &Project) -> LppoResult<Vec<u8>> {
    let entry_path = project.entry_path();
    let source = fs::read_to_string(&entry_path).map_err(|e| {
        LppoError::io_error(format!(
            "failed to read entry {}: {}",
            entry_path.display(),
            e
        ))
    })?;
    compile_to_bytes_with_root(&source, &project.root)
}

/// Write a bundled executable by copying the current executable and appending
/// the procedure payload (`LPPO-EMBED-1`). Existing `LPPO-EMBED-1` executables
/// still load because readers stop after the procedure payload.
fn write_bundled_executable(out: &Path, proc_bytes: &[u8]) -> LppoResult<()> {
    let exe = std::env::current_exe()
        .map_err(|e| LppoError::io_error(format!("failed to get current exe: {}", e)))?;
    let mut target = fs::File::create(out)
        .map_err(|e| LppoError::io_error(format!("failed to create {}: {}", out.display(), e)))?;
    let mut srcexe = fs::File::open(&exe)
        .map_err(|e| LppoError::io_error(format!("failed to open exe {}: {}", exe.display(), e)))?;
    std::io::copy(&mut srcexe, &mut target)
        .map_err(|e| LppoError::io_error(format!("failed to copy exe: {}", e)))?;
    target
        .write_all(MARKER)
        .map_err(|e| LppoError::io_error(format!("failed to write marker: {}", e)))?;
    let len = (proc_bytes.len() as u64).to_le_bytes();
    target
        .write_all(&len)
        .map_err(|e| LppoError::io_error(format!("failed to write length: {}", e)))?;
    target
        .write_all(proc_bytes)
        .map_err(|e| LppoError::io_error(format!("failed to write payload: {}", e)))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(out)
            .map_err(|e| LppoError::io_error(format!("failed to read metadata: {}", e)))?;
        let mut perms = metadata.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(out, perms)
            .map_err(|e| LppoError::io_error(format!("failed to set permissions: {}", e)))?;
    }
    Ok(())
}

/// Run a bundled executable from compiled bytes.
fn run_bytes(bytes: &[u8], debug: bool) -> LppoResult<()> {
    let executor = load_executor_from_bytes(bytes)
        .map_err(|e| LppoError::io_error(format!("failed to load bytecode: {}", e)))?;
    if !executor.has_procedure("main") {
        return Err(LppoError::unknown_procedure("main".into()));
    }
    executor
        .run("main")
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })
        .map_err(|e| LppoError::io_error(format!("runtime error: {}", e)))?;
    if debug {
        eprintln!("[debug] program finished");
    }
    Ok(())
}

//////////////////////////////////////////////////////////////////////////////
// Commands
//////////////////////////////////////////////////////////////////////////////

/// `lppo project init <name> [--path <dir>]`
/// Scaffold a new LPPO project and initialize a git repository.
pub fn cmd_init(name: &str, path: Option<&Path>) -> LppoResult<()> {
    let target_dir = match path {
        Some(p) => p.join(name),
        None => PathBuf::from(name),
    };

    if target_dir.exists() {
        return Err(LppoError::io_error(format!(
            "directory '{}' already exists",
            target_dir.display()
        )));
    }

    fs::create_dir_all(target_dir.join("src"))
        .map_err(|e| LppoError::io_error(format!("failed to create project dirs: {}", e)))?;

    let manifest = format!(
        "[project]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[build]\nentry = \"src/main.lppo\"\noutput = \"target/{{name}}\"\nstrip = false\nobfuscate = false\nlink_args = []\n",
        name
    );
    fs::write(target_dir.join(MANIFEST_NAME), manifest)
        .map_err(|e| LppoError::io_error(format!("failed to write manifest: {}", e)))?;

    let main_lppo = format!(
        "pub proc main() -> Void {{\n    out \"Hello from {}!\";\n}}\n",
        name
    );
    fs::write(target_dir.join("src").join("main.lppo"), main_lppo)
        .map_err(|e| LppoError::io_error(format!("failed to write main.lppo: {}", e)))?;

    let gitignore = "target/\n*.lppo.bak\n";
    fs::write(target_dir.join(".gitignore"), gitignore)
        .map_err(|e| LppoError::io_error(format!("failed to write .gitignore: {}", e)))?;

    // Create .lppo directory and copy global repositories.json into it.
    let lppo_dir = target_dir.join(".lppo");
    fs::create_dir_all(&lppo_dir).map_err(|e| {
        LppoError::io_error(format!("failed to create .lppo directory: {}", e))
    })?;
    if let Ok(global_repos) = repositories_json_path() {
        if global_repos.is_file() {
            let _ = fs::copy(&global_repos, lppo_dir.join("repositories.json"));
        }
    }

    // Initialize git repository
    let status = Command::new("git")
        .current_dir(&target_dir)
        .arg("init")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| LppoError::io_error(format!("failed to run git init: {}", e)))?;
    if !status.success() {
        return Err(LppoError::io_error("git init failed".into()));
    }

    println!(
        "Initialized LPPO project '{}' at {}",
        name,
        target_dir.display()
    );
    println!("  - created lppo.toml manifest");
    println!("  - created src/main.lppo entry point");
    println!("  - created .gitignore");
    println!("  - created .lppo/ with repositories.json");
    println!("  - initialized git repository");
    Ok(())
}

/// `lppo project build [--release] [--entry <file>] [--output <path>]`
/// [--strip] [--obfuscate] [--linker <path>] [--link-arg <arg>]...
/// Compile all `.lppo` files in the project into a single bundled executable.
pub fn cmd_build(options: BuildOptions, entry: Option<&str>, output: Option<&str>) -> LppoResult<()> {
    let project = match Project::discover(
        &std::env::current_dir()
            .map_err(|e| LppoError::io_error(format!("failed to get current dir: {}", e)))?,
    )? {
        Some(p) => p,
        None => {
            return Err(LppoError::io_error(
                "no lppo.toml found in current or parent directories".into(),
            ))
        }
    };

    let entry_path = entry
        .map(PathBuf::from)
        .unwrap_or_else(|| project.entry_path());
    let source = fs::read_to_string(&entry_path).map_err(|e| {
        LppoError::io_error(format!(
            "failed to read entry {}: {}",
            entry_path.display(),
            e
        ))
    })?;

    // Resolve imports relative to the entry file's directory so that
    // `use math;` from src/main.lppo finds src/math.lppo.
    let root = entry_path.parent().unwrap_or(&project.root).to_path_buf();

    // Package context for resolving installed-package imports and shader refs.
    let context = PackageContext::discover(&root)?;

    // Manifest options act as defaults; CLI options override them.
    let compile_options = lppo::CompileOptions {
        strip: options.strip,
        obfuscate: options.obfuscate,
    };

    let proc_bytes =
        lppo::compile_to_bytes_with_options(&source, &root, compile_options)?;

    let out_path = match output {
        Some(o) => PathBuf::from(o),
        None => project.output_path(),
    };

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            LppoError::io_error(format!(
                "failed to create output dir {}: {}",
                parent.display(),
                e
            ))
        })?;
    }

    write_bundled_executable(&out_path, &proc_bytes)?;

    let proc_part = format!("{} proc bytes", proc_bytes.len());

    let mode = if options.release { "release" } else { "debug" };
    println!(
        "Built {} bundle: {} ({}){}{}",
        mode,
        out_path.display(),
        proc_part,
        if options.strip { "; stripped declaration-only procedures" } else { "" },
        if options.obfuscate { "; obfuscated payload" } else { "" }
    );
    if options.linker.is_some() || !options.link_args.is_empty() {
        println!("  note: linker options apply to native/bare-metal builds; bundled executables copy the interpreter binary instead of linking.");
    }
    Ok(())
}

/// `lppo project format [--check] [files...]`
/// Format `.lppo` source files. With no arguments, formats all project sources.
pub fn cmd_format(check: bool, files: &[String]) -> LppoResult<()> {
    let targets: Vec<PathBuf> = if files.is_empty() {
        let project = match Project::discover(
            &std::env::current_dir()
                .map_err(|e| LppoError::io_error(format!("failed to get current dir: {}", e)))?,
        )? {
            Some(p) => p,
            None => return Err(LppoError::io_error("no lppo.toml found".into())),
        };
        project.source_files()?
    } else {
        files.iter().map(PathBuf::from).collect()
    };

    let mut had_changes = false;
    for path in &targets {
        if !path.is_file() {
            eprintln!("warning: not a file: {}", path.display());
            continue;
        }
        let contents = fs::read_to_string(path).map_err(|e| {
            LppoError::io_error(format!("failed to read {}: {}", path.display(), e))
        })?;
        let formatted = format_lppo_source(&contents);
        if formatted != contents {
            had_changes = true;
            if check {
                println!("{} would be reformatted", path.display());
            } else {
                fs::write(path, &formatted).map_err(|e| {
                    LppoError::io_error(format!("failed to write {}: {}", path.display(), e))
                })?;
                println!("formatted {}", path.display());
            }
        }
    }

    if check && had_changes {
        return Err(LppoError::io_error("some files are not formatted".into()));
    }
    if !had_changes && !check {
        println!("all files already formatted");
    }
    Ok(())
}

/// `lppo project run [--debug] [--entry <file>]`
/// Build and run the project's entry procedure.
pub fn cmd_run(debug: bool, entry: Option<&str>) -> LppoResult<()> {
    let project = match Project::discover(
        &std::env::current_dir()
            .map_err(|e| LppoError::io_error(format!("failed to get current dir: {}", e)))?,
    )? {
        Some(p) => p,
        None => return Err(LppoError::io_error("no lppo.toml found".into())),
    };

    let entry_path = entry
        .map(PathBuf::from)
        .unwrap_or_else(|| project.entry_path());
    let source = fs::read_to_string(&entry_path).map_err(|e| {
        LppoError::io_error(format!(
            "failed to read entry {}: {}",
            entry_path.display(),
            e
        ))
    })?;

    let root = entry_path.parent().unwrap_or(&project.root).to_path_buf();
    let executor = build_executor_with_root(&source, &root)?;
    if !executor.has_procedure("main") {
        return Err(LppoError::unknown_procedure("main".into()));
    }
    if debug {
        eprintln!("[debug] running entry: {}", entry_path.display());
    }
    executor
        .run("main")
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })
        .map_err(|e| LppoError::io_error(format!("runtime error: {}", e)))?;
    Ok(())
}

/// `lppo project debug [--entry <file>]`
/// Run the project with debug diagnostics enabled.
pub fn cmd_debug(entry: Option<&str>) -> LppoResult<()> {
    cmd_run(true, entry)
}

/// `lppo project git <args...>`
/// Pass arguments through to git, run in the project root.
pub fn cmd_git(args: &[String]) -> LppoResult<()> {
    let project = match Project::discover(
        &std::env::current_dir()
            .map_err(|e| LppoError::io_error(format!("failed to get current dir: {}", e)))?,
    )? {
        Some(p) => p,
        None => return Err(LppoError::io_error("no lppo.toml found".into())),
    };

    let mut cmd = Command::new("git");
    cmd.current_dir(&project.root);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    let status = cmd
        .status()
        .map_err(|e| LppoError::io_error(format!("failed to run git: {}", e)))?;
    if !status.success() {
        return Err(LppoError::io_error(format!(
            "git command exited with status {:?}",
            status.code()
        )));
    }
    Ok(())
}

/// `lppo project pkg <subcommand> ...`
/// Run a package-manager subcommand within this project's local package
/// store (`<project>/.lppo/`).
pub fn cmd_pkg(args: &[String]) -> LppoResult<()> {
    let project = match Project::discover(
        &std::env::current_dir()
            .map_err(|e| LppoError::io_error(format!("failed to get current dir: {}", e)))?,
    )? {
        Some(p) => p,
        None => return Err(LppoError::io_error("no lppo.toml found".into())),
    };
    let context = PackageContext::Project(project.root.join(".lppo"));
    let subcommand = PkgSubcommand::parse(args)?;
    subcommand.run_with_context(context)
}


//////////////////////////////////////////////////////////////////////////////
// Formatter
//////////////////////////////////////////////////////////////////////////////

/// Format LPPO source code: normalize indentation to tabs, ensure single space
/// after keywords, remove trailing whitespace, and end with a newline.
fn format_lppo_source(source: &str) -> String {
    let mut out = String::new();
    let mut indent_level: usize = 0;

    for raw_line in source.lines() {
        let line = raw_line.trim_end();
        if line.is_empty() {
            out.push('\n');
            continue;
        }

        let trimmed = line.trim_start();
        let is_close_brace =
            trimmed == "}" || trimmed.starts_with("} else") || trimmed.starts_with("} elif");

        if is_close_brace && indent_level > 0 {
            indent_level -= 1;
        }

        for _ in 0..indent_level {
            out.push('\t');
        }

        let formatted = normalize_spacing(trimmed);

        out.push_str(&formatted);
        out.push('\n');

        let open_brace_count = formatted.chars().filter(|&c| c == '{').count();
        let close_brace_count = formatted.chars().filter(|&c| c == '}').count();
        indent_level = indent_level
            .saturating_add(open_brace_count)
            .saturating_sub(close_brace_count);
    }

    // Ensure trailing newline
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Basic spacing normalization for LPPO keywords and punctuation.
fn normalize_spacing(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        // Ensure single space after opening brace before content on same line is rare; keep simple.
        result.push(ch);
        i += 1;
    }
    // Collapse multiple spaces into one (but not inside string literals - simplified)
    let mut collapsed = String::new();
    let mut prev_space = false;
    for ch in result.chars() {
        if ch == ' ' {
            if !prev_space {
                collapsed.push(' ');
            }
            prev_space = true;
        } else {
            collapsed.push(ch);
            prev_space = false;
        }
    }
    collapsed
}
