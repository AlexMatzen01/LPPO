//! lppo-bare-rt library: bare-metal kernel runtime for LPPO.
//!
//! The binary target (`src/main.rs`) is the bare-metal kernel runtime
//! (interpreter-based). The `codegen` module is scrapped — LPPO no longer
//! compiles to native machine code; it emits bytecode executed by this
//! runtime instead.

#![no_std]
extern crate alloc;

// codegen is scrapped and not built. Keep the module source for reference
// but do not expose it.
