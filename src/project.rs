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
    /// Build a native executable through the C transpiler (default).
    /// Set to `false` to embed the interpreter bundle instead.
    #[serde(default)]
    pub native: Option<bool>,
    /// Optimization level (0-3) for the C transpiler.
    #[serde(default)]
    pub opt_level: Option<u8>,
    /// Optimize for code size (`-Os`).
    #[serde(default)]
    pub os_opt: bool,
    /// Optimize aggressively for code size (`-Oz`).
    #[serde(default)]
    pub oz_opt: bool,
    /// `-ffunction-sections`: one ELF section per function.
    #[serde(default)]
    pub function_sections: bool,
    /// `-fdata-sections`: one ELF section per global/static.
    #[serde(default)]
    pub data_sections: bool,
    /// `-Wl,--gc-sections`: drop unreferenced sections at link time.
    #[serde(default)]
    pub gc_sections: bool,
    /// `-static`: fully static link (dynamic is the default).
    #[serde(default)]
    pub static_link: bool,
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
    /// Force the bundled-interpreter build instead of the native C backend.
    pub bundle: bool,
    /// Force the native C backend, overriding a `[build] native = false` manifest.
    pub force_native: bool,
    /// Write the generated C source to this path instead of an executable.
    pub emit_c: Option<String>,
/// Optimization level for the C transpiler (0-3). Takes precedence over
/// the manifest setting.
pub opt_level: Option<u8>,
/// Optimize for code size (`-Os`).
pub os_opt: bool,
/// Optimize aggressively for code size (`-Oz`).
pub oz_opt: bool,
/// `-ffunction-sections`: one ELF section per function.
pub function_sections: bool,
/// `-fdata-sections`: one ELF section per global/static.
pub data_sections: bool,
/// `-Wl,--gc-sections`: drop unreferenced sections at link time.
pub gc_sections: bool,
/// `-static`: fully static link (dynamic is the default).
pub static_link: bool,
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
/// [--bundle] [--emit-c <path>] [-O <0-3>]
///
/// Compiles the project into a native executable via the C transpiler.
/// Pass `--bundle` to embed the interpreter bundle instead (legacy behavior).
pub fn cmd_build(options: BuildOptions, entry: Option<&str>, output: Option<&str>) -> LppoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| LppoError::io_error(format!("failed to get current dir: {}", e)))?;
    let project = match Project::discover(&cwd)? {
        Some(p) => p,
        // No manifest: treat the current directory as an implicit project so
        // `lppo project build` works in plain source folders.
        None => implicit_project(&cwd)?,
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

    // Manifest options act as defaults; CLI options override them.
    let native = (options.force_native || project.manifest.build.native.unwrap_or(true))
        && !options.bundle;

    if !native {
        let compile_options = lppo::CompileOptions {
            strip: options.strip,
            obfuscate: options.obfuscate,
        };
        let proc_bytes =
            lppo::compile_to_bytes_with_options(&source, &root, compile_options)?;
        write_bundled_executable(&out_path, &proc_bytes)?;

        let mode = if options.release { "release" } else { "debug" };
        println!(
            "Built {} bundle: {} ({} proc bytes){}{}",
            mode,
            out_path.display(),
            proc_bytes.len(),
            if options.strip { "; stripped declaration-only procedures" } else { "" },
            if options.obfuscate { "; obfuscated payload" } else { "" }
        );
        if options.linker.is_some() || !options.link_args.is_empty() {
            println!("  note: linker options apply to native builds; bundled executables copy the interpreter binary instead of linking.");
        }
        return Ok(());
    }

    // Native path: lower the resolved program to C and invoke the system compiler.
    let manifest_opt = project
        .manifest
        .build
        .opt_level
        .or(options.opt_level);
    let opt_level = if options.os_opt {
        lppo::codegen_c::OptimizationLevel::Os
    } else if options.oz_opt {
        lppo::codegen_c::OptimizationLevel::Oz
    } else {
        match manifest_opt.or(if options.release { Some(2) } else { Some(0) }) {
            Some(n) => lppo::codegen_c::OptimizationLevel::from_u8(n)
                .map_err(LppoError::type_error)?,
            None => lppo::codegen_c::OptimizationLevel::default(),
        }
    };

    let procedures = lppo::compile_to_procedures(&source, &root)?;
    let config = lppo::codegen_c::TargetConfig {
        opt_level,
        function_sections: options.function_sections
            || project.manifest.build.function_sections,
        data_sections: options.data_sections || project.manifest.build.data_sections,
        gc_sections: options.gc_sections || project.manifest.build.gc_sections,
        strip: options.strip,
        static_link: options.static_link || project.manifest.build.static_link,
        ..Default::default()
    };

    if let Some(c_path) = options.emit_c.as_deref() {
        let c_src = lppo::codegen_c::compile_to_c_source(&procedures)
            .map_err(LppoError::type_error)?;
        let c_path = Path::new(c_path);
        if let Some(parent) = c_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                LppoError::io_error(format!(
                    "failed to create output dir {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }
        fs::write(c_path, c_src).map_err(|e| {
            LppoError::io_error(format!("failed to write {}: {}", c_path.display(), e))
        })?;
        println!("Wrote C source: {}", c_path.display());
        return Ok(());
    }

    lppo::codegen_c::compile_to_executable(&procedures, &config, &out_path)
        .map_err(LppoError::type_error)?;

    let mode = if options.release { "release" } else { "debug" };
    println!(
        "Built {} native executable: {} ({} procedures, -O{})",
        mode,
        out_path.display(),
        procedures.len(),
        opt_level
    );
    if options.linker.is_some() || !options.link_args.is_empty() {
        println!("  note: --linker/--link-arg are not yet forwarded to the C backend.");
    }
    Ok(())
}

/// Build a minimal in-memory project for a directory without `lppo.toml`.
fn implicit_project(root: &Path) -> LppoResult<Project> {
    let name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("app")
        .to_string();

    let is_main = |p: &Path| p.is_file();
    let entry = if is_main(&root.join("src/main.lppo")) {
        "src/main.lppo".to_string()
    } else if is_main(&root.join("main.lppo")) {
        "main.lppo".to_string()
    } else {
        let mut candidates: Vec<PathBuf> = fs::read_dir(root)
            .map_err(|e| LppoError::io_error(format!("failed to read {}: {}", root.display(), e)))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().map(|x| x == "lppo").unwrap_or(false))
            .collect();
        candidates.sort();
        match candidates.first() {
            Some(p) => p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("main.lppo")
                .to_string(),
            None => {
                return Err(LppoError::io_error(format!(
                    "no lppo.toml found and no main.lppo/src/main.lppo in {}",
                    root.display()
                )))
            }
        }
    };

    Ok(Project {
        root: root.to_path_buf(),
        manifest: Manifest {
            project: ProjectSection {
                name,
                version: default_version(),
                edition: default_edition(),
            },
            build: BuildSection {
                entry,
                output: default_output(),
                ..Default::default()
            },
            dependencies: None,
        },
    })
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
