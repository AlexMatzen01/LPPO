use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;

mod project;

use project::{cmd_build, cmd_debug, cmd_format, cmd_git, cmd_init, cmd_run};

fn main() {
    if let Err(error) = run() {
        // Try to get more detailed error information if it's an LppoError
        if let Some(lppo_err) = error.downcast_ref::<lppo::LppoError>() {
            // Use the detailed formatting if source is available
            eprint!("{}", lppo_err.format_detailed(None));
        } else {
            eprintln!("error: {error}");
        }
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
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

    let mut args_iter = raw_args.into_iter();

    // Project builder subcommand: `lppo project <init|build|format|run|debug|git> ...`
    if let Some(cmd) = args_iter.next() {
        if cmd == "project" {
            return run_project(&mut args_iter, debug);
        }
    }

    // compile-out / compile mode: bundle bytecode into a copy of the current
    // executable, producing a standalone runnable native ELF (.exe on Windows,
    // ELF on Linux). `--compile` is a shorthand for `--compile-out`.
    if let Some(flag) = args_iter.next() {
        if flag == "--compile-out" || flag == "--compile" {
            let out = args_iter.next().ok_or("missing output path")?;
            let src = args_iter.next().ok_or("missing source path")?;
            let source = fs::read_to_string(&src)?;
            let bytes =
                lppo::compile_to_bytes(&source).map_err(|e| -> Box<dyn Error> { Box::new(e) })?;

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
            return Ok(());
        }

        // compile to a bare metal executable: --compile-bare-metal <out> <source.lppo>
        if flag == "--compile-bare-metal" {
            let out = args_iter.next().ok_or("missing output path")?;
            let src = args_iter.next().ok_or("missing source path")?;
            let source = fs::read_to_string(&src)?;
            let bytes =
                lppo::compile_to_bytes(&source).map_err(|e| -> Box<dyn Error> { Box::new(e) })?;

            // Write the compiled bytecode to a temp file for the bare metal
            // runtime build script to embed into the x86_64-unknown-none binary.
            let temp =
                std::env::temp_dir().join(format!("lppo_bytecode_{}.json", std::process::id()));
            fs::write(&temp, &bytes)?;

            let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| {
                std::env::current_exe()
                    .ok()
                    .and_then(|e| e.parent().map(|p| p.to_string_lossy().into_owned()))
                    .unwrap_or_else(|| ".".to_string())
            });

            let status = std::process::Command::new("cargo")
                .current_dir(&manifest_dir)
                .env("LPPO_BYTECODE_FILE", &temp)
                .arg("build")
                .arg("-p")
                .arg("lppo-bare-rt")
                .arg("--target")
                .arg("x86_64-unknown-none")
                .arg("--release")
                .status()?;

            if !status.success() {
                return Err("bare metal build failed".into());
            }

            let built = format!(
                "{}/target/x86_64-unknown-none/release/lppo-bare-rt",
                manifest_dir
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
    }

    // run mode: if first arg given, treat it as a source path; otherwise check for embedded program
    if let Some(first) = std::env::args().nth(1) {
        let path_ref = Path::new(&first);
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
        } else {
            // if the arg doesn't exist, try to detect embedded program in current exe
            if debug {
                eprintln!("[debug] source '{first}' not found; attempting to run embedded program in executable...");
            }
        }
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
                    let payload = &tail[payload_start..payload_start + len];
                    let executor = lppo::load_executor_from_bytes(payload)
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
            let mut release = false;
            let mut entry: Option<String> = None;
            let mut output: Option<String> = None;
            while let Some(arg) = args_iter.next() {
                match arg.as_str() {
                    "--release" => release = true,
                    "--entry" => entry = Some(args_iter.next().ok_or("missing value for --entry")?),
                    "--output" => {
                        output = Some(args_iter.next().ok_or("missing value for --output")?)
                    }
                    other => return Err(format!("unknown build option: {other}").into()),
                }
            }
            cmd_build(release, entry.as_deref(), output.as_deref())
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
        other => return Err(format!("unknown project subcommand: {other}").into()),
    }

    Ok(())
}
