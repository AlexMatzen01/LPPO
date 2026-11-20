//! Target machine, optimization passes, and object/IR emission.
//!
//! Compiles C source code to native object files or linked executables
//! using the system C compiler (`cc`).

use crate::runtime::ProcedureBody;
use std::collections::HashMap;
use std::path::Path;

/// Optimization level, mapped from the CLI `-O` flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptimizationLevel {
    #[default]
    O0,
    O1,
    O2,
    O3,
    /// Optimize for code size.
    Os,
    /// Optimize aggressively for code size (Clang).
    Oz,
}

impl std::fmt::Display for OptimizationLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_cc_flag())
    }
}

impl OptimizationLevel {
    pub fn from_u8(n: u8) -> Result<Self, String> {
        match n {
            0 => Ok(OptimizationLevel::O0),
            1 => Ok(OptimizationLevel::O1),
            2 => Ok(OptimizationLevel::O2),
            3 => Ok(OptimizationLevel::O3),
            n => Err(format!("invalid optimization level: {}", n)),
        }
    }

    /// Parse a `-O*` style flag (`-O2`, `-Os`, `-Oz`, ...).
    pub fn parse(flag: &str) -> Result<Self, String> {
        match flag {
            "-O0" => Ok(OptimizationLevel::O0),
            "-O1" => Ok(OptimizationLevel::O1),
            "-O2" => Ok(OptimizationLevel::O2),
            "-O3" => Ok(OptimizationLevel::O3),
            "-Os" => Ok(OptimizationLevel::Os),
            "-Oz" => Ok(OptimizationLevel::Oz),
            other => Err(format!("unknown optimization flag: {}", other)),
        }
    }
    pub fn to_cc_flag(self) -> &'static str {
        match self {
            OptimizationLevel::O0 => "-O0",
            OptimizationLevel::O1 => "-O1",
            OptimizationLevel::O2 => "-O2",
            OptimizationLevel::O3 => "-O3",
            OptimizationLevel::Os => "-Os",
            OptimizationLevel::Oz => "-Oz",
        }
    }
}

/// Target configuration.
#[derive(Debug, Clone)]
pub struct TargetConfig {
    pub triple: String,
    pub cpu: String,
    pub features: String,
    pub opt_level: OptimizationLevel,
    /// `-ffunction-sections`: one ELF section per function.
    pub function_sections: bool,
    /// `-fdata-sections`: one ELF section per global/static.
    pub data_sections: bool,
    /// `-Wl,--gc-sections`: drop unreferenced sections at link time.
    pub gc_sections: bool,
    /// `-Wl,-s`: strip symbols/debug info at link time.
    pub strip: bool,
    /// `-static`: fully static link (dynamic is the default).
    pub static_link: bool,
    /// `-fno-asynchronous-unwind-tables`: drop unwind tables for smaller binaries.
    pub no_unwind_tables: bool,
    /// `-fno-exceptions`: disable exception handling tables for smaller binaries.
    pub no_exceptions: bool,
}

impl TargetConfig {
    /// Code-generation flags shared by object and executable builds.
    pub fn codegen_flags(&self) -> Vec<&'static str> {
        let mut flags = vec![self.opt_level.to_cc_flag()];
        if self.function_sections {
            flags.push("-ffunction-sections");
        }
        if self.data_sections {
            flags.push("-fdata-sections");
        }
        if self.no_unwind_tables {
            flags.push("-fno-asynchronous-unwind-tables");
        }
        if self.no_exceptions {
            flags.push("-fno-exceptions");
        }
        flags
    }

    /// Extra flags applied only when linking an executable.
    pub fn link_flags(&self) -> Vec<&'static str> {
        let mut flags = Vec::new();
        if self.gc_sections {
            flags.push("-Wl,--gc-sections");
        }
        if self.strip {
            flags.push("-Wl,-s");
        }
        if self.static_link {
            flags.push("-static");
        }
        flags
    }
}

impl Default for TargetConfig {
    fn default() -> Self {
        TargetConfig {
            triple: "x86_64-unknown-linux-gnu".to_string(),
            cpu: "x86-64".to_string(),
            features: "".to_string(),
            opt_level: OptimizationLevel::default(),
            function_sections: false,
            data_sections: false,
            gc_sections: false,
            strip: false,
            static_link: false,
            no_unwind_tables: false,
            no_exceptions: false,
        }
    }
}

/// Compile procedures to C source code string.
pub fn compile_to_c_source(
    procedures: &HashMap<String, ProcedureBody>,
) -> Result<String, String> {
    crate::codegen_c::compile_program_to_c(procedures)
}

/// Compile procedures to a native object file at `out_path`.
pub fn compile_to_object(
    procedures: &HashMap<String, ProcedureBody>,
    config: &TargetConfig,
    out_path: &Path,
) -> Result<(), String> {
    let c_source = crate::codegen_c::compile_program_to_c(procedures)?;

    // Write C source to a temp file
    let temp_c = std::env::temp_dir().join(format!("lppo_{}.c", std::process::id()));
    std::fs::write(&temp_c, &c_source)
        .map_err(|e| format!("failed to write temp C file: {}", e))?;

    // Compile to object file
    let mut cmd = std::process::Command::new("cc");
    cmd.arg("-c").arg(&temp_c).arg("-o").arg(out_path);
    for flag in config.codegen_flags() {
        cmd.arg(flag);
    }

    let status = cmd.status().map_err(|e| format!("compiler failed: {}", e))?;
    if !status.success() {
        return Err(format!("compilation failed with status: {:?}", status));
    }

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_c);

    Ok(())
}

/// Compile procedures to a linked executable at `out_path`,
/// linking against the LPPO runtime (`lppo-rt`).
pub fn compile_to_executable(
    procedures: &HashMap<String, ProcedureBody>,
    config: &TargetConfig,
    out_path: &Path,
) -> Result<(), String> {
    let c_source = crate::codegen_c::compile_program_to_c(procedures)?;

    // Write C source to a temp file
    let temp_c = std::env::temp_dir().join(format!("lppo_{}.c", std::process::id()));
    std::fs::write(&temp_c, &c_source)
        .map_err(|e| format!("failed to write temp C file: {}", e))?;

    // Locate the lppo-rt static library
    let rt_lib = locate_lppo_rt()?;

    // Compile and link
    let mut cmd = std::process::Command::new("cc");
    cmd.arg(&temp_c).arg(&rt_lib).arg("-o").arg(out_path).arg("-lm");
    for flag in config.codegen_flags() {
        cmd.arg(flag);
    }
    for flag in config.link_flags() {
        cmd.arg(flag);
    }

    let status = cmd.status().map_err(|e| format!("linker failed: {}", e))?;
    if !status.success() {
        return Err(format!("linking failed with status: {:?}", status));
    }

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_c);

    Ok(())
}

/// Locate the lppo-rt static library.
fn locate_lppo_rt() -> Result<std::path::PathBuf, String> {
    if let Ok(path) = std::env::var("LPPO_RT_LIB") {
        let p = std::path::PathBuf::from(&path);
        if p.exists() {
            return Ok(p);
        }
        return Err(format!("LPPO_RT_LIB points at a missing file: {}", path));
    }

    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("liblppo_rt.a"));
        }
    }
    if let Ok(dir) = std::env::current_dir() {
        for sub in ["target/debug", "target/release", "target/debug/deps", "target/release/deps"] {
            candidates.push(dir.join(sub).join("liblppo_rt.a"));
        }
    }

    for c in candidates {
        if c.exists() {
            return Ok(c);
        }
    }

    Err("lppo_rt library not found. Set LPPO_RT_LIB or run `cargo build -p lppo-rt`.".to_string())
}
