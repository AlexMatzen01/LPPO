//! LPPO runtime ABI declarations.
//!
//! Defines the `extern "C"` signatures of the runtime built-in functions that
//! generated LLVM IR calls into. The runtime itself (`lppo-rt` crate, or
//! `lppo-bare-rt` for freestanding) provides these symbols. Generated object
//! files declare them as external and link against the appropriate runtime.
//!
//! Calling convention: System V AMD64 ABI on x86-64 Linux (hosted) and the
//! same convention on `x86_64-unknown-none` (freestanding). Integer args in
//! rdi/rsi/rdx/rcx/r8/r9; float args in xmm0-xmm7; return in rax/xmm0.
//!
//! Value representation:
//!   - Int64   -> i64
//!   - Bool    -> i1
//!   - Float   -> double
//!   - Str     -> i8* (null-terminated, heap-allocated)
//!   - Arr<T>  -> %lppo.array = { i8*, i64 } passed by pointer
//!   - Void    -> void

use std::os::raw::c_char;
use std::os::raw::c_int;

/// Tag values passed as the first argument to `lppo_rt_print_value` so the
/// runtime knows how to interpret the second argument.
#[repr(i64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueTag {
    Int = 0,
    Float = 1,
    Bool = 2,
    Str = 3,
    Array = 4,
}

impl ValueTag {
    pub fn as_i64(self) -> i64 {
        self as i64
    }
}

// ---------------------------------------------------------------------------
// Extern "C" runtime function declarations.
// ---------------------------------------------------------------------------
// These are declaration-only; the runtime provides the definitions. The LLVM
// codegen emits calls matching these signatures.

#[link_name = "lppo_rt_print_value"]
extern "C" {
    pub fn lppo_rt_print_value(tag: i64, value: i64) -> ();
}

#[link_name = "lppo_rt_print_newline"]
extern "C" {
    pub fn lppo_rt_print_newline() -> ();
}

#[link_name = "lppo_rt_input"]
extern "C" {
    pub fn lppo_rt_input(tag: i64, prompt: *mut i8) -> i64;
}

#[link_name = "lppo_rt_alloc"]
extern "C" {
    pub fn lppo_rt_alloc(size: i64) -> i64;
}

#[link_name = "lppo_rt_dealloc"]
extern "C" {
    pub fn lppo_rt_dealloc(addr: i64) -> ();
}

#[link_name = "lppo_rt_cleanup"]
extern "C" {
    pub fn lppo_rt_cleanup() -> ();
}

#[link_name = "lppo_rt_clock_ns"]
extern "C" {
    pub fn lppo_rt_clock_ns() -> i64;
}

#[link_name = "lppo_rt_time_ns"]
extern "C" {
    pub fn lppo_rt_time_ns() -> i64;
}

#[link_name = "lppo_rt_array_set_tag"]
extern "C" {
    pub fn lppo_rt_array_set_tag(arr: *mut i8, idx: i64, tag: i64, val: i64) -> *mut i8;
}

#[link_name = "lppo_rt_array_make"]
extern "C" {
    pub fn lppo_rt_array_make(len: i64) -> *mut i8;
}

#[link_name = "lppo_rt_array_get"]
extern "C" {
    pub fn lppo_rt_array_get(arr: *mut i8, idx: i64) -> i64;
}

#[link_name = "lppo_rt_array_set"]
extern "C" {
    pub fn lppo_rt_array_set(arr: *mut i8, idx: i64, val: i64) -> *mut i8;
}

#[link_name = "lppo_rt_array_push"]
extern "C" {
    pub fn lppo_rt_array_push(arr: *mut i8, val: i64) -> *mut i8;
}

#[link_name = "lppo_rt_array_pop"]
extern "C" {
    pub fn lppo_rt_array_pop(arr: *mut i8) -> i64;
}

#[link_name = "lppo_rt_array_len"]
extern "C" {
    pub fn lppo_rt_array_len(arr: *mut i8) -> i64;
}

#[link_name = "lppo_rt_abs"]
extern "C" {
    pub fn lppo_rt_abs(v: i64) -> i64;
}

#[link_name = "lppo_rt_min"]
extern "C" {
    pub fn lppo_rt_min(a: i64, b: i64) -> i64;
}

#[link_name = "lppo_rt_max"]
extern "C" {
    pub fn lppo_rt_max(a: i64, b: i64) -> i64;
}

#[link_name = "lppo_rt_pow"]
extern "C" {
    pub fn lppo_rt_pow(base: i64, exp: i64) -> i64;
}

#[link_name = "lppo_rt_range"]
extern "C" {
    pub fn lppo_rt_range(start: i64, end: i64, inclusive: i64) -> *mut i8;
}

#[link_name = "lppo_rt_fn_make"]
extern "C" {
    pub fn lppo_rt_fn_make(index: i64, capture_count: i64) -> *mut i8;
}

#[link_name = "lppo_rt_fn_set_capture"]
extern "C" {
    pub fn lppo_rt_fn_set_capture(f: *mut i8, idx: i64, tag: i64, value: i64) -> ();
}

#[link_name = "lppo_rt_fn_index"]
extern "C" {
    pub fn lppo_rt_fn_index(f: *mut i8) -> i64;
}

#[link_name = "lppo_rt_fn_capture"]
extern "C" {
    pub fn lppo_rt_fn_capture(f: *mut i8, idx: i64) -> i64;
}

#[link_name = "lppo_rt_sqrt"]
extern "C" {
    pub fn lppo_rt_sqrt(v: i64) -> i64;
}

#[link_name = "lppo_rt_str_concat"]
extern "C" {
    pub fn lppo_rt_str_concat(a: *mut i8, b: *mut i8) -> *mut i8;
}

#[link_name = "lppo_rt_int_to_str"]
extern "C" {
    pub fn lppo_rt_int_to_str(n: i64) -> *mut i8;
}

#[link_name = "lppo_rt_str_to_int"]
extern "C" {
    pub fn lppo_rt_str_to_int(s: *mut i8) -> i64;
}

#[link_name = "lppo_rt_str_len"]
extern "C" {
    pub fn lppo_rt_str_len(s: *mut i8) -> i64;
}

#[link_name = "lppo_rt_str_upper"]
extern "C" {
    pub fn lppo_rt_str_upper(s: *mut i8) -> *mut i8;
}

#[link_name = "lppo_rt_str_lower"]
extern "C" {
    pub fn lppo_rt_str_lower(s: *mut i8) -> *mut i8;
}

#[link_name = "lppo_rt_str_trim"]
extern "C" {
    pub fn lppo_rt_str_trim(s: *mut i8) -> *mut i8;
}

#[link_name = "lppo_rt_str_split"]
extern "C" {
    pub fn lppo_rt_str_split(s: *mut i8, delim: *mut i8) -> *mut i8;
}

#[link_name = "lppo_rt_eval"]
extern "C" {
    pub fn lppo_rt_eval(s: *mut i8) -> i64;
}

#[link_name = "lppo_rt_either"]
extern "C" {
    pub fn lppo_rt_either(a: i64, b: i64) -> *mut i8;
}

#[link_name = "lppo_rt_random"]
extern "C" {
    pub fn lppo_rt_random(min: i64, max: i64) -> i64;
}

#[link_name = "lppo_rt_json_parse"]
extern "C" {
    pub fn lppo_rt_json_parse(s: *mut i8) -> *mut i8;
}

#[link_name = "lppo_rt_json_stringify"]
extern "C" {
    pub fn lppo_rt_json_stringify(tag: i64, value: i64) -> *mut i8;
}

#[link_name = "lppo_rt_system"]
extern "C" {
    pub fn lppo_rt_system(cmd: *mut i8) -> *mut i8;
}

#[link_name = "lppo_rt_color"]
extern "C" {
    pub fn lppo_rt_color(text: *mut i8, color: *mut i8) -> *mut i8;
}

#[link_name = "lppo_rt_read_file"]
extern "C" {
    pub fn lppo_rt_read_file(path: *mut i8) -> *mut i8;
}

#[link_name = "lppo_rt_write_file"]
extern "C" {
    pub fn lppo_rt_write_file(path: *mut i8, content: *mut i8) -> c_int;
}

#[link_name = "lppo_rt_append_file"]
extern "C" {
    pub fn lppo_rt_append_file(path: *mut i8, content: *mut i8) -> c_int;
}

#[link_name = "lppo_rt_file_exists"]
extern "C" {
    pub fn lppo_rt_file_exists(path: *mut i8) -> c_int;
}

#[link_name = "lppo_rt_delete_file"]
extern "C" {
    pub fn lppo_rt_delete_file(path: *mut i8) -> c_int;
}

#[link_name = "lppo_rt_pool_alloc"]
extern "C" {
    pub fn lppo_rt_pool_alloc(size: i64) -> i64;
}

#[link_name = "lppo_rt_pool_free"]
extern "C" {
    pub fn lppo_rt_pool_free(id: i64) -> i64;
}

#[link_name = "lppo_rt_pool_used"]
extern "C" {
    pub fn lppo_rt_pool_used() -> i64;
}

#[link_name = "lppo_rt_panic"]
extern "C" {
    pub fn lppo_rt_panic(message: *mut i8) -> ();
}

/// Entry point signature expected by the runtime: `main` takes no args and
/// returns i32. Generated `main` follows this.
pub type MainFn = extern "C" fn() -> c_int;

/// Native package calling convention carrying C string arguments.
///
/// The default native ABI is `extern "C" fn(i64 x8) -> i64` (all args and the
/// return are i64; string args are passed by FNV-hashed id). A native function
/// declared with `string_args` opts into this convention instead: the first
/// `n_string_params` arguments are `*const c_char` (null-terminated, owned by
/// the caller), followed by the remaining i64 args. Return is i64.
pub type NativeFnString =
    extern "C" fn(*const c_char, *const c_char, i64, i64, i64, i64, i64, i64) -> i64;
