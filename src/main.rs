use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
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

    // compile-out mode
    if let Some(flag) = args_iter.next() {
        if flag == "--compile-out" {
            let out = args_iter.next().ok_or("missing output path")?;
            let src = args_iter.next().ok_or("missing source path")?;
            let source = fs::read_to_string(&src)?;
            let bytes = lppo::compile_to_bytes(&source).map_err(|e| -> Box<dyn Error> { Box::new(e) })?;

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
    }

    // run mode: if first arg given, treat it as a source path; otherwise check for embedded program
    if let Some(first) = std::env::args().nth(1) {
        let path_ref = Path::new(&first);
        if path_ref.exists() {
            let source = fs::read_to_string(path_ref)?;
            let executor = lppo::build_executor(&source).map_err(|err| -> Box<dyn Error> { Box::new(err) })?;
            if !executor.has_procedure("main") {
                return Err(Box::new(lppo::LppoError::UnknownProcedure("main".into())));
            }
            executor.run("main").map_err(|err| -> Box<dyn Error> { Box::new(err) })?;
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
        let start = if buf.len() > window { buf.len() - window } else { 0 };
        let tail = &buf[start..];

        if let Some(pos) = find_marker_pos(tail) {
            let after = pos + MARKER_CONST.len();
            if tail.len() >= after + 8 {
                let len_bytes = &tail[after..after + 8];
                let len = u64::from_le_bytes(len_bytes.try_into().unwrap()) as usize;
                let payload_start = after + 8;
                if tail.len() >= payload_start + len {
                    let payload = &tail[payload_start..payload_start + len];
                    let executor = lppo::load_executor_from_bytes(payload).map_err(|e| -> Box<dyn Error> { Box::new(e) })?;
                    if !executor.has_procedure("main") {
                        return Err(Box::new(lppo::LppoError::UnknownProcedure("main".into())));
                    }
                    executor.run("main").map_err(|err| -> Box<dyn Error> { Box::new(err) })?;
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
