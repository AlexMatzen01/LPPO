use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

fn lppo_bin() -> PathBuf {
    PathBuf::from(env::var("CARGO_BIN_EXE_lppo").expect("CARGO_BIN_EXE_lppo not set"))
}

fn ensure_lppo_rt_built() {
    let candidates = [
        PathBuf::from(env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string()))
            .join("debug")
            .join("liblppo_rt.a"),
        PathBuf::from("target/debug/liblppo_rt.a"),
    ];
    if candidates.iter().any(|p| p.exists()) {
        return;
    }
    let _ = Command::new("cargo")
        .args(["build", "-p", "lppo-rt"])
        .status();
}

fn run_lppo(args: &[&str]) -> String {
    let out = Command::new(lppo_bin()).args(args).output().expect("lppo binary run failed");
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn run_program(source: &str, label: &str) {
    ensure_lppo_rt_built();
    let tmpdir = env::temp_dir();
    let src = tmpdir.join(format!("lppo_parity_{}.lppo", label));
    fs::write(&src, source).unwrap();

    // Interpreted path.
    let interpreted = run_lppo(&[src.to_str().unwrap()]);

    // Transpiled path.
    let bin = tmpdir.join(format!("lppo_parity_{}_bin", label));
    let build_out = run_lppo(&["build", src.to_str().unwrap(), "-o", bin.to_str().unwrap()]);
    assert!(
        build_out.contains("Wrote native executable") || bin.exists(),
        "transpile failed for {label}:\n{build_out}"
    );
    let transpiled = {
        let out = Command::new(&bin).output().expect("transpiled binary run failed");
        String::from_utf8_lossy(&out.stdout).to_string()
    };

    let interpreted = interpreted.trim();
    let transpiled = transpiled.trim();
    assert_eq!(
        interpreted, transpiled,
        "parity mismatch for {label}\ninterpreted: {interpreted:?}\ntranspiled: {transpiled:?}"
    );
}

#[test]
fn parity_range() {
    run_program(
        "proc main() -> Void {\n    for i: Int64 in 1..5 {\n        out i;\n    }\n}\n",
        "range",
    );
}

#[test]
fn parity_indexed_assignment() {
    run_program(
        "proc main() -> Void {\n    let mut arr: Arr<Int64> = vec![10, 20, 30];\n    arr[1] = 99;\n    out arr[1];\n    out arr[0];\n    out arr[2];\n}\n",
        "indexed",
    );
}

#[test]
fn parity_match() {
    run_program(
        "proc main() -> Void {\n    let x: Int64 = 2;\n    match x {\n        2 => out \"two\"\n    }\n    out \"done\";\n}\n",
        "match",
    );
}

#[test]
fn parity_closure() {
    run_program(
        "proc main() -> Void {\n    out (|x: Int64| -> Int64 { x * 2 })(5);\n    let base: Int64 = 10;\n    out (|x: Int64| -> Int64 { x + base })(5);\n}\n",
        "closure",
    );
}

#[test]
fn parity_closure_as_value() {
    run_program(
        "proc main() -> Void {\n    let base: Int64 = 10;\n    let f: Fn(Int64) -> Int64 = |x: Int64| -> Int64 { x + base };\n    out f(5);\n    out f(3);\n}\n",
        "closure_value",
    );
}

#[test]
fn parity_closure_indirect() {
    run_program(
        "proc main() -> Void {\n    let f: Fn(Int64) -> Int64 = |x: Int64| -> Int64 { x * 2 };\n    out f(7);\n    let g: Fn(Int64) -> Int64 = |x: Int64| -> Int64 { x + 100 };\n    out g(1);\n}\n",
        "closure_indirect",
    );
}

#[test]
fn parity_clock_time() {
    ensure_lppo_rt_built();
    let tmpdir = env::temp_dir();
    let src = tmpdir.join("lppo_parity_clock.lppo");
    fs::write(
        &src,
        "proc main() -> Void {\n    out clock_ns();\n    out time_ns();\n}\n",
    )
    .unwrap();

    fn parse_two_lines(output: &str) -> (i64, i64) {
        let mut lines = output.trim().lines();
        let clock: i64 = lines.next().unwrap().parse().unwrap();
        let time: i64 = lines.next().unwrap().parse().unwrap();
        (clock, time)
    }

    // Interpreted path.
    let interpreted = run_lppo(&[src.to_str().unwrap()]);
    let (iclock, itime) = parse_two_lines(&interpreted);
    assert!(iclock >= 0);
    assert!(itime > 1_000_000_000_000_000_000, "epoch nanos too small: {itime}");

    // Transpiled path.
    let bin = tmpdir.join("lppo_parity_clock_bin");
    let build_out = run_lppo(&["build", src.to_str().unwrap(), "-o", bin.to_str().unwrap()]);
    assert!(
        build_out.contains("Wrote native executable") || bin.exists(),
        "transpile failed:\n{build_out}"
    );
    let out = Command::new(&bin).output().expect("transpiled binary run failed");
    let transpiled = String::from_utf8_lossy(&out.stdout);
    let (tclock, ttime) = parse_two_lines(&transpiled);
    assert!(tclock >= 0);
    assert!(ttime > 1_000_000_000_000_000_000, "epoch nanos too small: {ttime}");
}
