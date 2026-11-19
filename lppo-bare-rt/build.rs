fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_dir = std::path::Path::new(&out_dir);

    // The LPPO bytecode (JSON) to embed is provided via the LPPO_BYTECODE_FILE
    // environment variable by the lppo compiler driver. When building standalone
    // (no bytecode provided), embed an empty program so the crate still compiles.
    let dest = out_dir.join("bytecode.json");
    if let Ok(bytecode_path) = std::env::var("LPPO_BYTECODE_FILE") {
        std::fs::copy(&bytecode_path, &dest)
            .unwrap_or_else(|e| panic!("failed to copy bytecode to {}: {}", dest.display(), e));
        println!("cargo:rerun-if-changed={}", bytecode_path);
    } else {
        std::fs::write(&dest, b"{}\n").expect("failed to write empty bytecode");
    }

    // Tell the linker to use our bare metal linker script (entry _start).
    // Use an absolute path so the linker finds it regardless of cwd.
    let linker_script = std::env::current_dir()
        .expect("cwd not available")
        .join("linker.ld");
    println!("cargo:rustc-link-arg=-T{}", linker_script.display());

    println!("cargo:rerun-if-env-changed=LPPO_BYTECODE_FILE");
    println!("cargo:rerun-if-changed=linker.ld");

    // Assemble the real-mode -> protected-mode -> long-mode bootstrap (boot.S).
    // It is linked into the binary and placed at 0x100000 by the linker script
    // (.text.start section). The CPU starts here in real mode and switches to
    // 64-bit long mode before jumping to Rust's kmain().
    let boot_s = std::env::current_dir()
        .expect("cwd not available")
        .join("src/arch/x86_64/boot.S");
    let boot_o = out_dir.join("boot.o");
    let status = std::process::Command::new("as")
        .arg(boot_s)
        .arg("-o")
        .arg(&boot_o)
        .status()
        .expect("failed to run `as` to assemble boot.S");
    if !status.success() {
        panic!("failed to assemble boot.S");
    }
    println!("cargo:rustc-link-arg={}", boot_o.display());
    println!("cargo:rerun-if-changed=src/arch/x86_64/boot.S");

    // Generate a small include file that exposes the bytecode as a static.
    let embedded = out_dir.join("embedded.rs");
    std::fs::write(
        &embedded,
        b"pub const BYTECODE: &[u8] = include_bytes!(\"bytecode.json\");\n",
    )
    .unwrap_or_else(|e| panic!("failed to write embedded.rs: {}", e));
}
