use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

mod project;

use project::{
    cmd_build, cmd_debug, cmd_format, cmd_git, cmd_init, cmd_pkg, cmd_run, BuildOptions,
};

use lppo::pkg::cli::PkgSubcommand;

fn lppo_version_major() -> u64 {
    option_env!("LPPO_VERSION_MAJOR")
        .unwrap_or("0")
        .parse()
        .unwrap_or(0)
}

fn lppo_version_minor() -> u64 {
    option_env!("LPPO_VERSION_MINOR")
        .unwrap_or("0")
        .parse()
        .unwrap_or(0)
}

fn lppo_version_patch() -> u64 {
    option_env!("LPPO_VERSION_PATCH")
        .unwrap_or("0")
        .parse()
        .unwrap_or(0)
}

fn lppo_version_build() -> u64 {
    option_env!("LPPO_VERSION_BUILD")
        .unwrap_or("0")
        .parse()
        .unwrap_or(0)
}

fn lppo_version_channel() -> &'static str {
    option_env!("LPPO_VERSION_CHANNEL").unwrap_or("unknown")
}

fn lppo_version_string() -> String {
    let base = format!(
        "{}.{}.{}",
        lppo_version_major(),
        lppo_version_minor(),
        lppo_version_patch()
    );
    match lppo_version_channel() {
        "release" => format!("{}+{}", base, lppo_version_build()),
        "beta" => format!("{}-beta.{}", base, lppo_version_build()),
        "alpha" => format!("{}-alpha.{}", base, lppo_version_build()),
        _ => format!("{}+{}", base, lppo_version_build()),
    }
}

fn ensure_installed_in_path() -> Result<(), Box<dyn Error>> {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("USERPROFILE").map(PathBuf::from))
        .map_err(|_| "could not determine home directory".to_string())?;
    let lppo_dir = home.join(".LPPO");
    let bin_dir = lppo_dir.join("bin");
    fs::create_dir_all(&bin_dir)?;

    let target = bin_dir.join("lppo");
    let current = std::env::current_exe()?;

    // Only copy if the target doesn't exist or differs.
    let needs_copy = match fs::metadata(&target) {
        Ok(meta) => {
            let cur_meta = fs::metadata(&current)?;
            meta.len() != cur_meta.len()
        }
        Err(_) => true,
    };
    if needs_copy {
        fs::copy(&current, &target)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&target)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&target, perms)?;
        }
    }

    // Copy default repositories.json to ~/.LPPO/repositories.json if absent.
    let repos_target = lppo_dir.join("repositories.json");
    if !repos_target.is_file() {
        // Try to find the bundled repositories.json next to the executable,
        // or fall back to the compile-time embedded default.
        let exe_dir = std::env::current_exe()?
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf();
        let bundled = exe_dir.join("repositories.json");
        if bundled.is_file() {
            fs::copy(&bundled, &repos_target)?;
        } else {
            let default_repos = include_str!("../repositories.json");
            fs::write(&repos_target, default_repos)?;
        }
    }

    // Add ~/.LPPO/bin to PATH by appending to shell rc files if not present.
    let path_entry = format!("export PATH=\"$HOME/.LPPO/bin:$PATH\"");
    let rc_files = [
        home.join(".bashrc"),
        home.join(".zshrc"),
        home.join(".profile"),
    ];
    for rc in &rc_files {
        if rc.is_file() {
            let contents = fs::read_to_string(rc)?;
            if !contents.contains("LPPO/bin") {
                let mut file = fs::OpenOptions::new().append(true).open(rc)?;
                writeln!(file, "\n# LPPO\n{}", path_entry)?;
            }
        }
    }

    Ok(())
}

fn main() {
    if let Err(error) = run() {
        if let Some(lppo_err) = error.downcast_ref::<lppo::LppoError>() {
            eprint!("{}", lppo_err.format_detailed(None));
        } else {
            eprintln!("error: {error}");
        }
        std::process::exit(1);
    }
}

fn print_help() {
    println!("lppo - Legible Programming Platform eXtensions");
    println!("version: {}", lppo_version_string());
    println!();
    println!("USAGE:");
    println!("    lppo [OPTIONS] <COMMAND> [ARGS]");
    println!("    lppo <source.lppo>");
    println!();
    println!("OPTIONS:");
    println!("    --debug        Enable debug output");
    println!("    --help, -h     Print this help message");
    println!("    --version, -V  Print version information");
    println!();
    println!("COMMANDS:");
    println!("    compile <out> <source.lppo>   Compile source to a bundled executable");
    println!("    compile-out <out> <source.lppo>  Alias for compile (alias)");
    println!("    compile-bare-metal <out> <source.lppo>  Compile to a bare-metal x86_64 executable");
    println!("    project <subcommand>          Project management (init, build, run, debug, format, git, pkg)");
    println!("    pkg <subcommand>              Package management");
    println!("    update                        Update lppo from GitHub");
    println!("    version                       Print version information");
    println!();
    println!("If no command is given and the running executable contains an embedded");
    println!("program (created with `lppo compile`), it is executed.");
    println!();
    println!("Run 'lppo project <subcommand> --help' for subcommand help.");
}

fn run() -> Result<(), Box<dyn Error>> {
    // Ensure lppo is installed to ~/.LPPO/bin/lppo and on PATH.
    if let Err(e) = ensure_installed_in_path() {
        eprintln!("warning: failed to auto-install lppo to PATH: {e}");
    }

    // Simple CLI modes:
    // - compile to a bundled executable: --compile-out <out> <source.lppo>
    // - run source: <source.lppo>
    // - if no source and the current executable contains an embedded program, run it.

    // Collect args and detect an optional global --debug flag.
    let mut raw_args: Vec<String> = std::env::args().skip(1).collect();
    let mut debug = false;
    raw_args.retain(|a| {
        if a == "--debug" {
            debug = true;
            false
        } else {
            true
        }
    });

    // Handle --help / -h before anything else.
    if raw_args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }

    // Handle --version / -V before anything else.
    if raw_args.iter().any(|a| a == "--version" || a == "-V") {
        println!("lppo {}", lppo_version_string());
        return Ok(());
    }

    let mut args_iter = raw_args.into_iter();

    // Dispatch on the first argument exactly once: project subcommand,
    // compile mode, version, update, or run mode.
    match args_iter.next().as_deref() {
        Some("project") => return run_project(&mut args_iter, debug),

        Some("pkg") => {
            let pkg_args: Vec<String> = args_iter.collect();
            let subcommand = PkgSubcommand::parse(&pkg_args)?;
            return subcommand.run().map_err(|e| -> Box<dyn Error> { Box::new(e) });
        }

        Some("version") => {
            println!("lppo {}", lppo_version_string());
            return Ok(());
        }

        Some("update") => {
            return cmd_update(debug);
        }

        Some("--compile-out") | Some("--compile") => {
            let mut strip = false;
            let mut obfuscate = false;
            let mut linker: Option<String> = None;
            let mut link_args: Vec<String> = Vec::new();
            let mut out: Option<String> = None;
            let mut src: Option<String> = None;
            while let Some(arg) = args_iter.next() {
                match arg.as_str() {
                    "--strip" => strip = true,
                    "--obfuscate" => obfuscate = true,
                    "--linker" => linker = Some(args_iter.next().ok_or("missing value for --linker")?),
                    "--link-arg" => link_args.push(args_iter.next().ok_or("missing value for --link-arg")?),
                    other if out.is_none() => out = Some(other.to_string()),
                    other if src.is_none() => src = Some(other.to_string()),
                    other => return Err(format!("unknown compile option: {other}").into()),
                }
            }
            let out = out.ok_or("missing output path")?;
            let src = src.ok_or("missing source path")?;
            let source = fs::read_to_string(&src)?;
            let bytes =
                lppo::compile_to_bytes_with_options(
                    &source,
                    &std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf()),
                    lppo::CompileOptions { strip, obfuscate },
                ).map_err(|e| -> Box<dyn Error> { Box::new(e) })?;

            // create bundled executable by copying current exe and appending marker + size + payload
            let exe = std::env::current_exe()?;
            let mut target = fs::File::create(&out)?;
            let mut srcexe = fs::File::open(&exe)?;
            std::io::copy(&mut srcexe, &mut target)?;

            // Write marker, length, payload
            const MARKER: &[u8] = b"LPPO-EMBED-1\n";
            target.write_all(MARKER)?;
            let len = (bytes.len() as u64).to_le_bytes();
            target.write_all(&len)?;
            target.write_all(&bytes)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&out)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&out, perms)?;
            }

            if debug {
                eprintln!("[debug] Wrote bundled executable: {out}");
            } else {
                println!("Wrote bundled executable: {out}");
            }
            let _ = (linker, link_args); // bundled executables copy the interpreter; no link step
            return Ok(());
        }

        Some("--compile-bare-metal") => {
            let mut strip = false;
            let mut obfuscate = false;
            let mut linker: Option<String> = None;
            let mut link_args: Vec<String> = Vec::new();
            let mut out: Option<String> = None;
            let mut src: Option<String> = None;
            while let Some(arg) = args_iter.next() {
                match arg.as_str() {
                    "--strip" => strip = true,
                    "--obfuscate" => obfuscate = true,
                    "--linker" => linker = Some(args_iter.next().ok_or("missing value for --linker")?),
                    "--link-arg" => link_args.push(args_iter.next().ok_or("missing value for --link-arg")?),
                    other if out.is_none() => out = Some(other.to_string()),
                    other if src.is_none() => src = Some(other.to_string()),
                    other => return Err(format!("unknown compile-bare-metal option: {other}").into()),
                }
            }
            let out = out.ok_or("missing output path")?;
            let src = src.ok_or("missing source path")?;
            let source = fs::read_to_string(&src)?;
            let bytes = lppo::compile_to_bytes_with_options(
                &source,
                &std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf()),
                lppo::CompileOptions { strip, obfuscate },
            )?;

            // Write the compiled bytecode to a temp file for the bare metal
            // runtime build script to embed into the x86_64-unknown-none binary.
            let temp =
                std::env::temp_dir().join(format!("lppo_bytecode_{}.json", std::process::id()));
            fs::write(&temp, &bytes)?;

            let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    let mut dir = std::env::current_exe()
                        .ok()
                        .and_then(|e| e.parent().map(|p| p.to_path_buf()))
                        .unwrap_or_else(|| PathBuf::from("."));
                    // Walk up to the directory containing Cargo.toml (the workspace root).
                    while !dir.join("Cargo.toml").is_file() {
                        if !dir.pop() {
                            break;
                        }
                    }
                    dir
                });

            // Forward linker options to the bare-metal cargo build via RUSTFLAGS.
            let mut rustflags: Vec<String> = Vec::new();
            if let Some(linker) = &linker {
                rustflags.push(format!("-C linker={linker}"));
            }
            for arg in &link_args {
                rustflags.push(format!("-C link-arg={arg}"));
            }

            let mut cargo = std::process::Command::new("cargo");
            cargo
                .current_dir(&manifest_dir)
                .env("LPPO_BYTECODE_FILE", &temp)
                .arg("build")
                .arg("-p")
                .arg("lppo-bare-rt")
                .arg("--target")
                .arg("x86_64-unknown-none")
                .arg("--release");
            if !rustflags.is_empty() {
                cargo.env("RUSTFLAGS", rustflags.join(" "));
            }
            let status = cargo.status()?;

            if !status.success() {
                return Err("bare metal build failed".into());
            }

            let built = format!(
                "{}/target/x86_64-unknown-none/release/lppo-bare-rt",
                manifest_dir.display()
            );
            fs::copy(&built, &out)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&out)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&out, perms)?;
            }

            let _ = fs::remove_file(&temp);

            if debug {
                eprintln!("[debug] Wrote bare metal executable: {out}");
            } else {
                println!("Wrote bare metal executable: {out}");
            }
            return Ok(());
        }

        Some(first) => {
            // Run mode: treat the first argument as a source path.
            let path_ref = Path::new(first);
            if path_ref.exists() {
                let source = fs::read_to_string(path_ref)?;
                let root = path_ref.parent().unwrap_or_else(|| Path::new("."));
                let executor = lppo::build_executor_with_root(&source, root)
                    .map_err(|err| -> Box<dyn Error> { Box::new(err) })?;
                if !executor.has_procedure("main") {
                    return Err(Box::new(lppo::LppoError::unknown_procedure("main".into())));
                }
                executor
                    .run("main")
                    .map_err(|err| -> Box<dyn Error> { Box::new(err) })?;
                return Ok(());
            }
            if debug {
                eprintln!("[debug] source '{first}' not found; attempting to run embedded program in executable...");
            }
        }

        None => {}
    }

    // Try to detect embedded payload in current exe
    let exe = std::env::current_exe()?;
    // Try to read the running executable to detect any embedded payload.
    if debug {
        eprintln!("[debug] attempting to read executable: {:?}", exe);
    }
    if let Ok(buf) = fs::read(&exe) {
        // Search only the last 1MB for marker to avoid scanning huge files
        let window = 1024 * 1024usize;
        let start = if buf.len() > window {
            buf.len() - window
        } else {
            0
        };
        let tail = &buf[start..];

        if let Some(pos) = find_marker_pos(tail) {
            let after = pos + MARKER_CONST.len();
            if tail.len() >= after + 8 {
                let len_bytes = &tail[after..after + 8];
                let len = u64::from_le_bytes(len_bytes.try_into().unwrap()) as usize;
                let payload_start = after + 8;
                if tail.len() >= payload_start + len {
                    // The bundle payload begins with `[u64 proc_len][proc JSON]`
                    // and may be followed by an optional `[LPPO-SHDR-1][shader
                    // IR]` section; `load_executor_from_bundle_bytes` parses
                    // both.
                    let executor =
                        lppo::load_executor_from_bundle_bytes(&tail[after..])
                            .map_err(|e| -> Box<dyn Error> { Box::new(e) })?;
                    if !executor.has_procedure("main") {
                        return Err(Box::new(lppo::LppoError::unknown_procedure("main".into())));
                    }
                    executor
                        .run("main")
                        .map_err(|err| -> Box<dyn Error> { Box::new(err) })?;
                    return Ok(());
                }
            }
        }
    } else if debug {
        eprintln!("[debug] failed to read executable: {:?}", exe);
    }

    Err("no source given and no embedded program found".into())
}

const MARKER_CONST: &[u8] = b"LPPO-EMBED-1\n";

fn find_marker_pos(buf: &[u8]) -> Option<usize> {
    // Search for marker starting from the end backwards
    let marker = MARKER_CONST;
    buf.windows(marker.len()).rposition(|w| w == marker)
}

/// Check GitHub for the latest lppo release and update the local executable.
fn cmd_update(debug: bool) -> Result<(), Box<dyn Error>> {
    let current = lppo_version_string();
    if debug {
        eprintln!("[debug] current version: {current}");
    }

    let output = std::process::Command::new("gh")
        .arg("release")
        .arg("list")
        .arg("--limit")
        .arg("1")
        .arg("--json")
        .arg("tagName,name,assets,url")
        .output();

    let release_info = match output {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        _ => {
            let tags = std::process::Command::new("git")
                .arg("ls-remote")
                .arg("--tags")
                .arg("origin")
                .output();
            match tags {
                Ok(out) if out.status.success() => {
                    String::from_utf8_lossy(&out.stdout).trim().to_string()
                }
                _ => return Err("could not query GitHub for latest release (is `gh` or `git` available?)".into()),
            }
        }
    };

    if release_info.is_empty() {
        println!("No releases found on GitHub.");
        return Ok(());
    }

    println!("Latest release info:\n{release_info}");
    println!("Current version: {current}");
    println!("Use `gh release download <tag> --pattern lppo` to manually download, then replace this executable.");
    println!("Or run: lppo version-cli Set <major> <minor> <patch> && lppo version-cli BumpBuild");

    Ok(())
}

/// Dispatch the `lppo project <subcommand>` CLI.
fn run_project(
    args_iter: &mut std::vec::IntoIter<String>,
    debug: bool,
) -> Result<(), Box<dyn Error>> {
    let subcommand = args_iter.next().ok_or("missing project subcommand")?;

    match subcommand.as_str() {
        "init" => {
            let name = args_iter.next().ok_or("missing project name")?;
            let mut path: Option<String> = None;
            while let Some(arg) = args_iter.next() {
                if arg == "--path" {
                    path = Some(args_iter.next().ok_or("missing value for --path")?);
                }
            }
            cmd_init(&name, path.as_deref().map(Path::new))
                .map_err(|e| -> Box<dyn Error> { Box::new(e) })?;
        }
        "build" => {
            let mut options = BuildOptions::default();
            let mut entry: Option<String> = None;
            let mut output: Option<String> = None;
            while let Some(arg) = args_iter.next() {
                match arg.as_str() {
                    "--release" => options.release = true,
                    "--strip" => options.strip = true,
                    "--obfuscate" => options.obfuscate = true,
                    "--linker" => options.linker = Some(args_iter.next().ok_or("missing value for --linker")?),
                    "--link-arg" => options.link_args.push(args_iter.next().ok_or("missing value for --link-arg")?),
                    "--entry" => entry = Some(args_iter.next().ok_or("missing value for --entry")?),
                    "--output" => {
                        output = Some(args_iter.next().ok_or("missing value for --output")?)
                    }
                    other => return Err(format!("unknown build option: {other}").into()),
                }
            }
            cmd_build(options, entry.as_deref(), output.as_deref())
                .map_err(|e| -> Box<dyn Error> { Box::new(e) })?;
        }
        "format" => {
            let mut check = false;
            let mut files: Vec<String> = Vec::new();
            while let Some(arg) = args_iter.next() {
                match arg.as_str() {
                    "--check" => check = true,
                    other => files.push(other.to_string()),
                }
            }
            cmd_format(check, &files).map_err(|e| -> Box<dyn Error> { Box::new(e) })?;
        }
        "run" => {
            let mut entry: Option<String> = None;
            while let Some(arg) = args_iter.next() {
                if arg == "--entry" {
                    entry = Some(args_iter.next().ok_or("missing value for --entry")?);
                } else {
                    return Err(format!("unknown run option: {arg}").into());
                }
            }
            cmd_run(debug, entry.as_deref()).map_err(|e| -> Box<dyn Error> { Box::new(e) })?;
        }
        "debug" => {
            let mut entry: Option<String> = None;
            while let Some(arg) = args_iter.next() {
                if arg == "--entry" {
                    entry = Some(args_iter.next().ok_or("missing value for --entry")?);
                } else {
                    return Err(format!("unknown debug option: {arg}").into());
                }
            }
            cmd_debug(entry.as_deref()).map_err(|e| -> Box<dyn Error> { Box::new(e) })?;
        }
        "git" => {
            let git_args: Vec<String> = args_iter.collect();
            cmd_git(&git_args).map_err(|e| -> Box<dyn Error> { Box::new(e) })?;
        }
        "pkg" => {
            let pkg_args: Vec<String> = args_iter.collect();
            cmd_pkg(&pkg_args).map_err(|e| -> Box<dyn Error> { Box::new(e) })?;
        }
        other => return Err(format!("unknown project subcommand: {other}").into()),
    }

    Ok(())
}
