//! LPPO Runtime Library (`lppo-rt`).
//!
//! Provides the `extern "C"` built-in functions that generated LLVM IR calls
//! into. These are linked into standalone executables produced by `lppo build`.
//!
//! ABI (System V AMD64 on x86-64):
//!   - Integer args: rdi, rsi, rdx, rcx, r8, r9
//!   - Float args: xmm0-xmm7
//!   - Return: rax / xmm0
//! Value representation:
//!   - Int64 -> i64
//!   - Bool -> i1 (zero-extended to i64 in FFI)
//!   - Float -> double
//!   - Str -> *mut i8 (null-terminated, heap-allocated)
//!   - Arr<T> -> *mut i8 (pointer to { data_ptr, len } struct)
//!   - Void -> ()

use std::collections::HashMap;
use std::ffi::CStr;
use std::fs;
use std::io::Write;
use std::os::raw::c_int;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH, Instant};

// ---------------------------------------------------------------------------
// String allocation helpers
// ---------------------------------------------------------------------------

/// Allocate a null-terminated C string on the heap. The caller must free it
/// with `free_string`.
fn cstr_to_owned(s: &str) -> *mut i8 {
    let c_string = std::ffi::CString::new(s).expect("string contained NUL");
    c_string.into_raw()
}

/// Free a C string allocated by `cstr_to_owned`.
unsafe fn free_string(ptr: *mut i8) {
    if !ptr.is_null() {
        let _ = std::ffi::CString::from_raw(ptr);
    }
}

/// Convert a runtime `i64` value to its string representation (Int tag).
fn int_to_cstring(n: i64) -> *mut i8 {
    cstr_to_owned(&n.to_string())
}

// ---------------------------------------------------------------------------
// Print value
// ---------------------------------------------------------------------------

/// Print a value to stdout. `tag` indicates the type:
///   0=Int, 1=Float, 2=Bool, 3=Str, 4=Array
/// `value` is the i64 payload (for Str/Array, it's a pointer).
#[no_mangle]
pub extern "C" fn lppo_rt_print_value(tag: i64, value: i64) {
    print!("{}", render(tag, value));
    let _ = std::io::stdout().flush();
}

#[no_mangle]
pub extern "C" fn lppo_rt_print_newline() {
    println!();
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

/// Read a line from stdin and parse according to `tag`.
/// Returns the parsed value as i64 (for Int/Bool) or a heap-allocated
/// string pointer (for Str).
#[no_mangle]
pub extern "C" fn lppo_rt_input(tag: i64, prompt: *mut i8) -> i64 {
    if !prompt.is_null() {
        unsafe {
            let cstr = CStr::from_ptr(prompt as *const i8);
            print!("{}", cstr.to_string_lossy());
            let _ = std::io::stdout().flush();
        }
    }

    let mut line = String::new();
    std::io::stdin().read_line(&mut line).ok();
    let line = line.trim_end_matches(['\n', '\r']).to_string();

    match tag {
        0 => match line.trim().parse::<i64>() {
            Ok(v) => v,
            Err(_) => {
                eprintln!("error: type error: Invalid Int64 input: '{}'", line.trim());
                std::process::exit(1);
            }
        },
        2 => { // Bool
            let lower = line.trim().to_lowercase();
            matches!(lower.as_str(), "true" | "t" | "yes" | "y" | "1") as i64
        }
        3 => { // Str - return pointer to heap-allocated string
            cstr_to_owned(&line) as i64
        }
        4 => {
            // Array input: "[1, 2, 3]" or "1,2,3"; strings if any element is not an int.
            let inner = if line.trim().starts_with('[') && line.trim().ends_with(']') {
                line.trim()[1..line.trim().len() - 1].to_string()
            } else {
                line.clone()
            };
            let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).filter(|p| !p.is_empty()).collect();
            let all_int = parts.iter().all(|p| p.parse::<i64>().is_ok());
            let arr = lppo_rt_array_make(parts.len() as i64);
            for (i, p) in parts.iter().enumerate() {
                let v = if all_int {
                    p.parse::<i64>().unwrap_or(0)
                } else {
                    cstr_to_owned(p.trim_matches('"')) as i64
                };
                lppo_rt_array_set(arr, i as i64, v);
            }
            arr as i64
        }
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Memory (heap allocation registry)
// ---------------------------------------------------------------------------

/// A heap block handed out by `lppo_rt_alloc`. The raw pointer is wrapped in a
/// newtype so the registry can be `Send`/`Sync` for use in a `OnceLock<Mutex<_>>`.
#[derive(Debug, Clone)]
struct HeapBlock(*mut u8, usize);

unsafe impl Send for HeapBlock {}
unsafe impl Sync for HeapBlock {}

static ALLOCS: OnceLock<Mutex<HashMap<i64, HeapBlock>>> = OnceLock::new();

fn allocs() -> &'static Mutex<HashMap<i64, HeapBlock>> {
    ALLOCS.get_or_init(|| Mutex::new(HashMap::new()))
}

static NEXT_ALLOC_ADDR: OnceLock<Mutex<i64>> = OnceLock::new();

fn next_alloc_addr() -> &'static Mutex<i64> {
    NEXT_ALLOC_ADDR.get_or_init(|| Mutex::new(4096))
}

/// Allocate `size` zeroed bytes on the heap and register the block so it can
/// be freed by `lppo_rt_dealloc` / `lppo_rt_cleanup`. Returns the heap
/// address (or 0 on failure).
#[no_mangle]
pub extern "C" fn lppo_rt_alloc(size: i64) -> i64 {
    if size <= 0 {
        return 0;
    }
    let mut buf = Vec::with_capacity(size as usize);
    buf.resize(size as usize, 0);
    let cap = buf.capacity();
    let ptr = buf.as_mut_ptr() as *mut u8;
    std::mem::forget(buf);

    let addr = {
        let mut next = next_alloc_addr().lock().unwrap();
        let addr = *next;
        *next += 1;
        addr
    };
    allocs().lock().unwrap().insert(addr, HeapBlock(ptr, cap));
    addr as i64
}

/// Free a previously allocated block by address. Returns 0.
#[no_mangle]
pub extern "C" fn lppo_rt_dealloc(addr: i64) {
    if addr == 0 {
        return;
    }
    if let Some(HeapBlock(ptr, cap)) = allocs().lock().unwrap().remove(&addr) {
        if !ptr.is_null() {
            // Recover ownership of the leaked Vec and drop it to free memory.
            drop(unsafe { Vec::from_raw_parts(ptr, 0, cap) });
        }
    }
}

/// Free every allocation tracked by the registry and clear it. Intended to be
/// called once at process exit (e.g. from `main` after the user program runs)
/// to avoid unbounded growth in long-running / repeated runs.
#[no_mangle]
pub extern "C" fn lppo_rt_cleanup() {
    let entries: Vec<HeapBlock> = {
        let mut map = allocs().lock().unwrap();
        let v: Vec<_> = map.values().cloned().collect();
        map.clear();
        v
    };
    for HeapBlock(ptr, cap) in entries {
        if !ptr.is_null() {
            drop(unsafe { Vec::from_raw_parts(ptr, 0, cap) });
        }
    }
}

/// Monotonic nanoseconds since process start (for benchmarks).
#[no_mangle]
pub extern "C" fn lppo_rt_clock_ns() -> i64 {
    static START: OnceLock<Instant> = OnceLock::new();
    let start = START.get_or_init(Instant::now);
    start.elapsed().as_nanos() as i64
}

/// Nanoseconds since the UNIX epoch (wall clock).
#[no_mangle]
pub extern "C" fn lppo_rt_time_ns() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Array built-ins
// ---------------------------------------------------------------------------

// Boxed object layout (shared by arrays, tuples, maps, sets, structs, enums, fns):
//
//   words: [ kind | len | name | variant | item0_tag | item0_value | ... ]
//
// `kind`: 0 array, 1 tuple, 2 map, 3 set, 4 struct, 5 enum, 6 fn.
// `name`/`variant` hold `char*` values (0 when unused).
// Maps store four words per entry: (key_tag, key, value_tag, value).

const OBJ_ARRAY: i64 = 0;
const OBJ_TUPLE: i64 = 1;
const OBJ_MAP: i64 = 2;
const OBJ_SET: i64 = 3;
const OBJ_STRUCT: i64 = 4;
const OBJ_ENUM: i64 = 5;
const OBJ_FN: i64 = 6;

const HDR_WORDS: usize = 4;

#[inline]
unsafe fn obj_words(p: *mut i8) -> *mut i64 {
    p as *mut i64
}

#[inline]
unsafe fn obj_kind(p: *mut i8) -> i64 {
    *obj_words(p)
}

#[inline]
unsafe fn obj_len(p: *mut i8) -> i64 {
    *obj_words(p).add(1)
}

#[inline]
unsafe fn obj_name(p: *mut i8) -> i64 {
    *obj_words(p).add(2)
}

#[inline]
unsafe fn obj_variant(p: *mut i8) -> i64 {
    *obj_words(p).add(3)
}

#[inline]
unsafe fn obj_items(p: *mut i8) -> *mut i64 {
    obj_words(p).add(HDR_WORDS)
}

/// Allocate an object with `len` entries of `words_per_entry` tagged words each.
unsafe fn obj_new(
    kind: i64,
    len: i64,
    name: i64,
    variant: i64,
    words_per_entry: usize,
) -> *mut i8 {
    if len < 0 {
        return std::ptr::null_mut();
    }
    let total_words = HDR_WORDS + (len as usize) * words_per_entry * 2;
    let mut buf: Vec<i64> = vec![0; total_words];
    let ptr = buf.as_mut_ptr() as *mut i8;
    std::ptr::write(obj_words(ptr), kind);
    std::ptr::write(obj_words(ptr).add(1), len);
    std::ptr::write(obj_words(ptr).add(2), name);
    std::ptr::write(obj_words(ptr).add(3), variant);
    std::mem::forget(buf);
    ptr
}

#[inline]
unsafe fn item_tag(p: *mut i8, i: usize) -> i64 {
    *obj_items(p).add(i * 2)
}

#[inline]
unsafe fn item_value(p: *mut i8, i: usize) -> i64 {
    *obj_items(p).add(i * 2 + 1)
}

#[inline]
unsafe fn set_item(p: *mut i8, i: usize, tag: i64, value: i64) {
    std::ptr::write(obj_items(p).add(i * 2), tag);
    std::ptr::write(obj_items(p).add(i * 2 + 1), value);
}

fn cstr_arg(ptr: *mut i8) -> String {
    cstr_of(ptr as i64)
}

fn cstr_of(ptr: i64) -> String {
    if ptr == 0 {
        String::new()
    } else {
        unsafe { CStr::from_ptr(ptr as *const i8).to_string_lossy().to_string() }
    }
}

/// Render a tagged value the same way the interpreter prints it.
fn render(tag: i64, value: i64) -> String {
    match tag {
        0 => value.to_string(),
        1 => f64::from_bits(value as u64).to_string(),
        2 => if value != 0 { "true" } else { "false" }.to_string(),
        3 => cstr_of(value),
        5 => {
            if let Some(c) = char::from_u32(value as u32) {
                format!("'{}'", c)
            } else {
                value.to_string()
            }
        }
        6 => value.to_string(),
        4 | 7 | 9 => {
            let (open, close) = match tag {
                4 => ("[", "]"),
                7 => ("(", ")"),
                _ => ("{", "}"),
            };
            if value == 0 {
                return format!("{}{}", open, close);
            }
            let len = unsafe { obj_len(value as *mut i8) };
            let mut items = Vec::with_capacity(len as usize);
            for i in 0..len as usize {
                let t = unsafe { item_tag(value as *mut i8, i) };
                let v = unsafe { item_value(value as *mut i8, i) };
                items.push(render(t, v));
            }
            format!("{}{}{}", open, items.join(", "), close)
        }
        8 => {
            if value == 0 {
                return "{}".to_string();
            }
            let len = unsafe { obj_len(value as *mut i8) };
            let mut items = Vec::with_capacity(len as usize);
            for i in 0..len as usize {
                let kt = unsafe { item_tag(value as *mut i8, i * 2) };
                let kv = unsafe { item_value(value as *mut i8, i * 2) };
                let vt = unsafe { item_tag(value as *mut i8, i * 2 + 1) };
                let vv = unsafe { item_value(value as *mut i8, i * 2 + 1) };
                items.push(format!("{}: {}", render(kt, kv), render(vt, vv)));
            }
            format!("{{{}}}", items.join(", "))
        }
        10 => {
            if value == 0 {
                return "{}".to_string();
            }
            let name = cstr_of(unsafe { obj_name(value as *mut i8) });
            let len = unsafe { obj_len(value as *mut i8) };
            let mut items = Vec::with_capacity(len as usize);
            for i in 0..len as usize {
                unsafe {
                    let base = obj_items(value as *mut i8).add(i * 4);
                    let field = cstr_of(*base);
                    let vtag = *base.add(1);
                    let vval = *base.add(2);
                    items.push(format!("{}: {}", field, render(vtag, vval)));
                }
            }
            format!("{} {{ {} }}", name, items.join(", "))
        }
        11 => {
            if value == 0 {
                return "None".to_string();
            }
            let variant = cstr_of(unsafe { obj_variant(value as *mut i8) });
            let len = unsafe { obj_len(value as *mut i8) };
            if len == 0 {
                return variant;
            }
            let mut items = Vec::with_capacity(len as usize);
            for i in 0..len as usize {
                let t = unsafe { item_tag(value as *mut i8, i) };
                let v = unsafe { item_value(value as *mut i8, i) };
                items.push(render(t, v));
            }
            format!("{}({})", variant, items.join(", "))
        }
        12 => format!("<fn/{}>", unsafe { obj_len(value as *mut i8) }),
        14 => {
            if value == 0 {
                "null".to_string()
            } else {
                unsafe {
                    match obj_kind(value as *mut i8) {
                        OBJ_ARRAY => render(4, value),
                        OBJ_TUPLE => render(7, value),
                        OBJ_MAP => render(8, value),
                        OBJ_SET => render(9, value),
                        OBJ_STRUCT => render(10, value),
                        OBJ_ENUM => render(11, value),
                        OBJ_FN => render(12, value),
                        _ => value.to_string(),
                    }
                }
            }
        }
        13 => {
            if value == 0 {
                "null".to_string()
            } else {
                let t = unsafe { item_tag(value as *mut i8, 0) };
                let v = unsafe { item_value(value as *mut i8, 0) };
                render(t, v)
            }
        }
        _ => format!("<unknown tag {}>", tag),
    }
}

#[no_mangle]
pub extern "C" fn lppo_rt_array_make(len: i64) -> *mut i8 {
    unsafe { obj_new(OBJ_ARRAY, len, 0, 0, 1) }
}

#[no_mangle]
pub extern "C" fn lppo_rt_array_get(arr: *mut i8, idx: i64) -> i64 {
    if arr.is_null() || idx < 0 {
        return 0;
    }
    let len = unsafe { obj_len(arr) };
    if idx >= len {
        return 0;
    }
    unsafe { item_value(arr, idx as usize) }
}

#[no_mangle]
pub extern "C" fn lppo_rt_array_get_tag(arr: *mut i8, idx: i64) -> i64 {
    if arr.is_null() || idx < 0 {
        return 0;
    }
    let len = unsafe { obj_len(arr) };
    if idx >= len {
        return 0;
    }
    unsafe { item_tag(arr, idx as usize) }
}

#[no_mangle]
pub extern "C" fn lppo_rt_array_set(arr: *mut i8, idx: i64, val: i64) -> *mut i8 {
    lppo_rt_array_set_tag(arr, idx, 0, val)
}

#[no_mangle]
pub extern "C" fn lppo_rt_array_set_tag(arr: *mut i8, idx: i64, tag: i64, val: i64) -> *mut i8 {
    if arr.is_null() || idx < 0 {
        return arr;
    }
    let len = unsafe { obj_len(arr) };
    if idx < len {
        unsafe { set_item(arr, idx as usize, tag, val) };
    }
    arr
}

fn array_copy_with(arr: *mut i8, extra_tag: i64, extra_val: i64) -> *mut i8 {
    let len = unsafe { obj_len(arr) };
    let new_ptr = unsafe { obj_new(OBJ_ARRAY, len + 1, 0, 0, 1) };
    for i in 0..len as usize {
        let t = unsafe { item_tag(arr, i) };
        let v = unsafe { item_value(arr, i) };
        unsafe { set_item(new_ptr, i, t, v) };
    }
    unsafe { set_item(new_ptr, len as usize, extra_tag, extra_val) };
    new_ptr
}

#[no_mangle]
pub extern "C" fn lppo_rt_array_push(arr: *mut i8, val: i64) -> *mut i8 {
    if arr.is_null() {
        let a = lppo_rt_array_make(0);
        return array_copy_with(a, 0, val);
    }
    array_copy_with(arr, 0, val)
}

#[no_mangle]
pub extern "C" fn lppo_rt_array_push_tag(arr: *mut i8, tag: i64, val: i64) -> *mut i8 {
    if arr.is_null() {
        let a = lppo_rt_array_make(0);
        return array_copy_with(a, tag, val);
    }
    array_copy_with(arr, tag, val)
}

#[no_mangle]
pub extern "C" fn lppo_rt_array_pop(arr: *mut i8) -> i64 {
    if arr.is_null() {
        return 0;
    }
    let len = unsafe { obj_len(arr) };
    if len == 0 {
        return 0;
    }
    unsafe { item_value(arr, (len - 1) as usize) }
}

#[no_mangle]
pub extern "C" fn lppo_rt_array_len(arr: *mut i8) -> i64 {
    if arr.is_null() {
        return 0;
    }
    unsafe { obj_len(arr) }
}

// ---------------------------------------------------------------------------
// Tuples / enums / structs
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn lppo_rt_tuple_make(len: i64) -> *mut i8 {
    unsafe { obj_new(OBJ_TUPLE, len, 0, 0, 1) }
}

#[no_mangle]
pub extern "C" fn lppo_rt_tuple_get(t: *mut i8, idx: i64) -> i64 {
    if t.is_null() || idx < 0 {
        return 0;
    }
    let len = unsafe { obj_len(t) };
    if idx >= len {
        return 0;
    }
    unsafe { item_value(t, idx as usize) }
}

#[no_mangle]
pub extern "C" fn lppo_rt_tuple_get_tag(t: *mut i8, idx: i64) -> i64 {
    if t.is_null() || idx < 0 {
        return 0;
    }
    let len = unsafe { obj_len(t) };
    if idx >= len {
        return 0;
    }
    unsafe { item_tag(t, idx as usize) }
}

#[no_mangle]
pub extern "C" fn lppo_rt_enum_make(name: *mut i8, variant: *mut i8, len: i64) -> *mut i8 {
    unsafe { obj_new(OBJ_ENUM, len, name as i64, variant as i64, 1) }
}

#[no_mangle]
pub extern "C" fn lppo_rt_enum_tag(value: *mut i8) -> *mut i8 {
    if value.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        if obj_kind(value) != OBJ_ENUM {
            return cstr_to_owned("");
        }
        cstr_to_owned(&cstr_of(obj_variant(value)))
    }
}

#[no_mangle]
pub extern "C" fn lppo_rt_enum_variant(value: *mut i8) -> *mut i8 {
    if value.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let name = cstr_of(obj_name(value));
        let variant = cstr_of(obj_variant(value));
        cstr_to_owned(&format!("{}::{}", name, variant))
    }
}

#[no_mangle]
pub extern "C" fn lppo_rt_enum_get(value: *mut i8, idx: i64) -> i64 {
    if value.is_null() || idx < 0 {
        return 0;
    }
    let len = unsafe { obj_len(value) };
    if idx >= len {
        return 0;
    }
    unsafe { item_value(value, idx as usize) }
}

#[no_mangle]
pub extern "C" fn lppo_rt_enum_get_tag(value: *mut i8, idx: i64) -> i64 {
    if value.is_null() || idx < 0 {
        return 0;
    }
    let len = unsafe { obj_len(value) };
    if idx >= len {
        return 0;
    }
    unsafe { item_tag(value, idx as usize) }
}

#[no_mangle]
pub extern "C" fn lppo_rt_object_set(obj: *mut i8, idx: i64, tag: i64, value: i64) {
    if obj.is_null() || idx < 0 {
        return;
    }
    let len = unsafe { obj_len(obj) };
    if idx < len {
        unsafe { set_item(obj, idx as usize, tag, value) };
    }
}

#[no_mangle]
pub extern "C" fn lppo_rt_struct_make(name: *mut i8, len: i64) -> *mut i8 {
    unsafe { obj_new(OBJ_STRUCT, len, name as i64, 0, 2) }
}

#[no_mangle]
pub extern "C" fn lppo_rt_struct_set(
    obj: *mut i8,
    idx: i64,
    field: *mut i8,
    tag: i64,
    value: i64,
) {
    if obj.is_null() || idx < 0 {
        return;
    }
    let len = unsafe { obj_len(obj) };
    if idx >= len {
        return;
    }
    unsafe {
        let base = obj_items(obj).add(idx as usize * 4);
        std::ptr::write(base, field as i64);
        std::ptr::write(base.add(1), tag);
        std::ptr::write(base.add(2), value);
    }
}

#[no_mangle]
pub extern "C" fn lppo_rt_struct_get(obj: *mut i8, field: *mut i8) -> i64 {
    if obj.is_null() {
        return 0;
    }
    let want = cstr_of(field as i64);
    let len = unsafe { obj_len(obj) };
    for i in 0..len as usize {
        unsafe {
            let base = obj_items(obj).add(i * 4);
            if cstr_of(*base) == want {
                return *base.add(2);
            }
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn lppo_rt_struct_get_tag(obj: *mut i8, field: *mut i8) -> i64 {
    if obj.is_null() {
        return 0;
    }
    let want = cstr_of(field as i64);
    let len = unsafe { obj_len(obj) };
    for i in 0..len as usize {
        unsafe {
            let base = obj_items(obj).add(i * 4);
            if cstr_of(*base) == want {
                return *base.add(1);
            }
        }
    }
    0
}

// ---------------------------------------------------------------------------
// Maps (insertion ordered, scalar keys)
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn lppo_rt_map_new() -> *mut i8 {
    unsafe { obj_new(OBJ_MAP, 0, 0, 0, 2) }
}

fn map_find(map: *mut i8, key_tag: i64, key: i64) -> Option<usize> {
    let len = unsafe { obj_len(map) };
    for i in 0..len as usize {
        unsafe {
            let kt = item_tag(map, i * 2);
            let kv = item_value(map, i * 2);
            if kt == key_tag && render(kt, kv) == render(key_tag, key) {
                return Some(i);
            }
        }
    }
    None
}

#[no_mangle]
pub extern "C" fn lppo_rt_map_insert(
    map: *mut i8,
    key_tag: i64,
    key: i64,
    val_tag: i64,
    val: i64,
) -> *mut i8 {
    if map.is_null() {
        return map;
    }
    unsafe {
        if let Some(i) = map_find(map, key_tag, key) {
            set_item(map, i * 2 + 1, val_tag, val);
            return map;
        }
        let len = obj_len(map);
        let new_ptr = obj_new(OBJ_MAP, len + 1, 0, 0, 2);
        for i in 0..len as usize {
            set_item(new_ptr, i * 2, item_tag(map, i * 2), item_value(map, i * 2));
            set_item(
                new_ptr,
                i * 2 + 1,
                item_tag(map, i * 2 + 1),
                item_value(map, i * 2 + 1),
            );
        }
        set_item(new_ptr, len as usize * 2, key_tag, key);
        set_item(new_ptr, len as usize * 2 + 1, val_tag, val);
        new_ptr
    }
}

#[no_mangle]
pub extern "C" fn lppo_rt_map_get(map: *mut i8, key_tag: i64, key: i64) -> i64 {
    if map.is_null() {
        return 0;
    }
    unsafe {
        if let Some(i) = map_find(map, key_tag, key) {
            return item_value(map, i * 2 + 1);
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn lppo_rt_map_get_tag(map: *mut i8, key_tag: i64, key: i64) -> i64 {
    if map.is_null() {
        return 0;
    }
    unsafe {
        if let Some(i) = map_find(map, key_tag, key) {
            return item_tag(map, i * 2 + 1);
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn lppo_rt_map_len(map: *mut i8) -> i64 {
    if map.is_null() {
        return 0;
    }
    unsafe { obj_len(map) }
}

#[no_mangle]
pub extern "C" fn lppo_rt_map_remove(map: *mut i8, key_tag: i64, key: i64) -> *mut i8 {
    if map.is_null() {
        return map;
    }
    unsafe {
        let Some(pos) = map_find(map, key_tag, key) else {
            return map;
        };
        let len = obj_len(map) as usize;
        let new_ptr = obj_new(OBJ_MAP, len as i64 - 1, 0, 0, 2);
        let mut out = 0usize;
        for i in 0..len {
            if i == pos {
                continue;
            }
            set_item(new_ptr, out * 2, item_tag(map, i * 2), item_value(map, i * 2));
            set_item(
                new_ptr,
                out * 2 + 1,
                item_tag(map, i * 2 + 1),
                item_value(map, i * 2 + 1),
            );
            out += 1;
        }
        new_ptr
    }
}

#[no_mangle]
pub extern "C" fn lppo_rt_map_keys(map: *mut i8) -> *mut i8 {
    if map.is_null() {
        return lppo_rt_array_make(0);
    }
    unsafe {
        let len = obj_len(map);
        let out = obj_new(OBJ_ARRAY, len, 0, 0, 1);
        for i in 0..len as usize {
            set_item(out, i, item_tag(map, i * 2), item_value(map, i * 2));
        }
        out
    }
}

#[no_mangle]
pub extern "C" fn lppo_rt_map_values(map: *mut i8) -> *mut i8 {
    if map.is_null() {
        return lppo_rt_array_make(0);
    }
    unsafe {
        let len = obj_len(map);
        let out = obj_new(OBJ_ARRAY, len, 0, 0, 1);
        for i in 0..len as usize {
            set_item(
                out,
                i,
                item_tag(map, i * 2 + 1),
                item_value(map, i * 2 + 1),
            );
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Sets (insertion ordered)
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn lppo_rt_set_new() -> *mut i8 {
    unsafe { obj_new(OBJ_SET, 0, 0, 0, 1) }
}

fn set_find(set: *mut i8, tag: i64, value: i64) -> Option<usize> {
    let len = unsafe { obj_len(set) };
    for i in 0..len as usize {
        unsafe {
            let t = item_tag(set, i);
            let v = item_value(set, i);
            if t == tag && render(t, v) == render(tag, value) {
                return Some(i);
            }
        }
    }
    None
}

#[no_mangle]
pub extern "C" fn lppo_rt_set_insert(set: *mut i8, tag: i64, value: i64) -> *mut i8 {
    if set.is_null() {
        return set;
    }
    unsafe {
        if set_find(set, tag, value).is_some() {
            return set;
        }
        let len = obj_len(set);
        let new_ptr = obj_new(OBJ_SET, len + 1, 0, 0, 1);
        for i in 0..len as usize {
            set_item(new_ptr, i, item_tag(set, i), item_value(set, i));
        }
        set_item(new_ptr, len as usize, tag, value);
        new_ptr
    }
}

#[no_mangle]
pub extern "C" fn lppo_rt_set_contains(set: *mut i8, tag: i64, value: i64) -> i64 {
    if set.is_null() {
        return 0;
    }
    set_find(set, tag, value).is_some() as i64
}

#[no_mangle]
pub extern "C" fn lppo_rt_set_remove(set: *mut i8, tag: i64, value: i64) -> *mut i8 {
    if set.is_null() {
        return set;
    }
    unsafe {
        let Some(pos) = set_find(set, tag, value) else {
            return set;
        };
        let len = obj_len(set) as usize;
        let new_ptr = obj_new(OBJ_SET, len as i64 - 1, 0, 0, 1);
        let mut out = 0usize;
        for i in 0..len {
            if i == pos {
                continue;
            }
            set_item(new_ptr, out, item_tag(set, i), item_value(set, i));
            out += 1;
        }
        new_ptr
    }
}

#[no_mangle]
pub extern "C" fn lppo_rt_set_len(set: *mut i8) -> i64 {
    if set.is_null() {
        return 0;
    }
    unsafe { obj_len(set) }
}

#[no_mangle]
pub extern "C" fn lppo_rt_set_to_array(set: *mut i8) -> *mut i8 {
    if set.is_null() {
        return lppo_rt_array_make(0);
    }
    unsafe {
        let len = obj_len(set);
        let out = obj_new(OBJ_ARRAY, len, 0, 0, 1);
        for i in 0..len as usize {
            set_item(out, i, item_tag(set, i), item_value(set, i));
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Functions (closures)
// ---------------------------------------------------------------------------

static FN_CAPTURES: OnceLock<Mutex<Vec<Vec<i64>>>> = OnceLock::new();

fn fn_captures() -> &'static Mutex<Vec<Vec<i64>>> {
    FN_CAPTURES.get_or_init(|| Mutex::new(Vec::new()))
}

/// Register a closure body index with its captured values (packed as tagged
/// pairs). Returns a boxed function value.
#[no_mangle]
pub extern "C" fn lppo_rt_fn_make(index: i64, capture_count: i64) -> *mut i8 {
    let obj = unsafe { obj_new(OBJ_FN, capture_count, 0, 0, 1) };
    unsafe {
        let mut caps = fn_captures().lock().unwrap();
        while caps.len() <= index as usize {
            caps.push(Vec::new());
        }
        let mut packed = Vec::with_capacity(capture_count as usize * 2);
        for i in 0..capture_count as usize {
            packed.push(item_tag(obj, i));
            packed.push(item_value(obj, i));
        }
        caps[index as usize] = packed;
    }
    unsafe { std::ptr::write(obj_items(obj), index) };
    obj
}

#[no_mangle]
pub extern "C" fn lppo_rt_fn_index(f: *mut i8) -> i64 {
    if f.is_null() {
        return -1;
    }
    unsafe { *obj_items(f) }
}

#[no_mangle]
pub extern "C" fn lppo_rt_fn_capture(f: *mut i8, idx: i64) -> i64 {
    if f.is_null() || idx < 0 {
        return 0;
    }
    let index = lppo_rt_fn_index(f);
    let caps = fn_captures().lock().unwrap();
    caps.get(index as usize)
        .and_then(|c| c.get((idx as usize * 2 + 1) as usize).copied())
        .unwrap_or(0)
}

/// Set capture `idx` (tag, value) on closure `f`. Updates both the object
/// items and the `fn_captures` registry so `lppo_rt_fn_capture` reads back
/// the new value.
#[no_mangle]
pub extern "C" fn lppo_rt_fn_set_capture(f: *mut i8, idx: i64, tag: i64, value: i64) {
    if f.is_null() || idx < 0 {
        return;
    }
    let index = lppo_rt_fn_index(f);
    unsafe {
        // Object layout: item 0 = index, items 1.. = capture tag/value pairs.
        // Capture idx: tag at item idx*2+1, value at item idx*2+2.
        std::ptr::write(obj_items(f).add((idx as usize * 2 + 1) as usize), tag);
        std::ptr::write(obj_items(f).add((idx as usize * 2 + 2) as usize), value);
    }
    let mut caps = fn_captures().lock().unwrap();
    if let Some(c) = caps.get_mut(index as usize) {
        let tag_idx = idx as usize * 2;
        let val_idx = tag_idx + 1;
        if val_idx < c.len() {
            c[tag_idx] = tag;
            c[val_idx] = value;
        } else {
            c.resize(val_idx + 1, 0);
            c[tag_idx] = tag;
            c[val_idx] = value;
        }
    }
}

// ---------------------------------------------------------------------------
// Arithmetic built-ins
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn lppo_rt_abs(v: i64) -> i64 {
    v.abs()
}

#[no_mangle]
pub extern "C" fn lppo_rt_min(a: i64, b: i64) -> i64 {
    a.min(b)
}

#[no_mangle]
pub extern "C" fn lppo_rt_max(a: i64, b: i64) -> i64 {
    a.max(b)
}

#[no_mangle]
pub extern "C" fn lppo_rt_pow(base: i64, exp: i64) -> i64 {
    if exp < 0 {
        return 0;
    }
    let mut result: i64 = 1;
    let mut b = base;
    let mut e = exp;
    while e > 0 {
        if e & 1 == 1 {
            result = result.wrapping_mul(b);
        }
        b = b.wrapping_mul(b);
        e >>= 1;
    }
    result
}

#[no_mangle]
pub extern "C" fn lppo_rt_sqrt(v: i64) -> i64 {
    if v <= 0 {
        return 0;
    }
    let mut x = v;
    loop {
        let y = (x + v / x) / 2;
        if y >= x {
            return x;
        }
        x = y;
    }
}

// ---------------------------------------------------------------------------
// String built-ins
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn lppo_rt_str_concat(a: *mut i8, b: *mut i8) -> *mut i8 {
    if a.is_null() || b.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let sa = CStr::from_ptr(a as *const i8).to_string_lossy();
        let sb = CStr::from_ptr(b as *const i8).to_string_lossy();
        cstr_to_owned(&format!("{}{}", sa, sb))
    }
}

#[no_mangle]
pub extern "C" fn lppo_rt_int_to_str(n: i64) -> *mut i8 {
    cstr_to_owned(&n.to_string())
}

#[no_mangle]
pub extern "C" fn lppo_rt_str_eq(a: *mut i8, b: *mut i8) -> i64 {
    (cstr_arg(a) == cstr_arg(b)) as i64
}

#[no_mangle]
pub extern "C" fn lppo_rt_str_cmp(a: *mut i8, b: *mut i8) -> i64 {
    match cstr_arg(a).cmp(&cstr_arg(b)) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

#[no_mangle]
pub extern "C" fn lppo_rt_str_to_int(s: *mut i8) -> i64 {
    if s.is_null() {
        return 0;
    }
    unsafe {
        let text = CStr::from_ptr(s as *const i8).to_string_lossy().to_string();
        match text.trim().parse::<i64>() {
            Ok(v) => v,
            Err(_) => {
                eprintln!("error: type error: Failed to parse '{}' as integer", text);
                std::process::exit(1);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn lppo_rt_str_len(s: *mut i8) -> i64 {
    if s.is_null() {
        return 0;
    }
    unsafe { CStr::from_ptr(s as *const i8).to_bytes().len() as i64 }
}

#[no_mangle]
pub extern "C" fn lppo_rt_str_upper(s: *mut i8) -> *mut i8 {
    if s.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let st = CStr::from_ptr(s as *const i8).to_string_lossy();
        cstr_to_owned(&st.to_uppercase())
    }
}

#[no_mangle]
pub extern "C" fn lppo_rt_str_lower(s: *mut i8) -> *mut i8 {
    if s.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let st = CStr::from_ptr(s as *const i8).to_string_lossy();
        cstr_to_owned(&st.to_lowercase())
    }
}

#[no_mangle]
pub extern "C" fn lppo_rt_str_trim(s: *mut i8) -> *mut i8 {
    if s.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let st = CStr::from_ptr(s as *const i8).to_string_lossy();
        cstr_to_owned(&st.trim().to_string())
    }
}

#[no_mangle]
pub extern "C" fn lppo_rt_str_split(s: *mut i8, delim: *mut i8) -> *mut i8 {
    if s.is_null() || delim.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let st = CStr::from_ptr(s as *const i8).to_string_lossy();
        let dl = CStr::from_ptr(delim as *const i8).to_string_lossy();
        let parts: Vec<&str> = st.split(dl.as_ref()).collect();
        let arr = lppo_rt_array_make(parts.len() as i64);
        for (i, p) in parts.iter().enumerate() {
            lppo_rt_array_set(arr, i as i64, cstr_to_owned(p) as i64);
        }
        arr
    }
}

// ---------------------------------------------------------------------------
// Math expression evaluator (eval)
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn lppo_rt_eval(s: *mut i8) -> i64 {
    if s.is_null() {
        return 0;
    }
    unsafe {
        let st = CStr::from_ptr(s as *const i8).to_string_lossy();
        match eval_math_expr(st.trim()) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("error: type error: eval failed: {}", e);
                std::process::exit(1);
            }
        }
    }
}

/// Tiny recursive-descent math expression evaluator.
/// Supports integers, + - * / ^ ( ) with standard precedence.
fn eval_math_expr(s: &str) -> Result<i64, String> {
    let chars: Vec<char> = s.chars().collect();
    let mut pos = 0;

    fn peek(chars: &[char], pos: usize) -> Option<char> {
        chars.get(pos).copied()
    }

    fn next(chars: &[char], pos: &mut usize) -> Option<char> {
        let c = chars.get(*pos).copied();
        if c.is_some() {
            *pos += 1;
        }
        c
    }

    fn skip_ws(chars: &[char], pos: &mut usize) {
        while matches!(peek(chars, *pos), Some(c) if c.is_whitespace()) {
            next(chars, pos);
        }
    }

    fn parse_expr(chars: &[char], pos: &mut usize) -> Result<i64, String> {
        let mut left = parse_term(chars, pos)?;
        loop {
            skip_ws(chars, pos);
            match peek(chars, *pos) {
                Some('+') => {
                    next(chars, pos);
                    let right = parse_term(chars, pos)?;
                    left = left.wrapping_add(right);
                }
                Some('-') => {
                    next(chars, pos);
                    let right = parse_term(chars, pos)?;
                    left = left.wrapping_sub(right);
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_term(chars: &[char], pos: &mut usize) -> Result<i64, String> {
        let mut left = parse_factor(chars, pos)?;
        loop {
            skip_ws(chars, pos);
            match peek(chars, *pos) {
                Some('*') => {
                    next(chars, pos);
                    let right = parse_factor(chars, pos)?;
                    left = left.wrapping_mul(right);
                }
                Some('/') => {
                    next(chars, pos);
                    let right = parse_factor(chars, pos)?;
                    if right == 0 {
                        return Err("division by zero".into());
                    }
                    left = left.wrapping_div(right);
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_factor(chars: &[char], pos: &mut usize) -> Result<i64, String> {
        skip_ws(chars, pos);
        if peek(chars, *pos) == Some('-') {
            next(chars, pos);
            let v = parse_factor(chars, pos)?;
            return Ok(-v);
        }
        parse_power(chars, pos)
    }

    fn parse_power(chars: &[char], pos: &mut usize) -> Result<i64, String> {
        let base = parse_primary(chars, pos)?;
        skip_ws(chars, pos);
        if peek(chars, *pos) == Some('^') {
            next(chars, pos);
            let exp = parse_factor(chars, pos)?;
            return Ok(base.pow(exp as u32));
        }
        Ok(base)
    }

    fn parse_primary(chars: &[char], pos: &mut usize) -> Result<i64, String> {
        skip_ws(chars, pos);
        if peek(chars, *pos) == Some('(') {
            next(chars, pos);
            let v = parse_expr(chars, pos)?;
            skip_ws(chars, pos);
            if peek(chars, *pos) != Some(')') {
                return Err("expected ')'".into());
            }
            next(chars, pos);
            Ok(v)
        } else {
            let start = *pos;
            while matches!(peek(chars, *pos), Some(c) if c.is_ascii_digit()) {
                next(chars, pos);
            }
            if start == *pos {
                return Err("expected number".into());
            }
            let num: String = chars[start..*pos].iter().collect();
            num.parse::<i64>().map_err(|e| e.to_string())
        }
    }

    skip_ws(&chars, &mut pos);
    let result = parse_expr(&chars, &mut pos)?;
    skip_ws(&chars, &mut pos);
    if pos < chars.len() {
        return Err("unexpected trailing characters".into());
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// either(a, b) -> [a+b, a-b]
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn lppo_rt_either(a: i64, b: i64) -> *mut i8 {
    let arr = lppo_rt_array_make(2);
    lppo_rt_array_set(arr, 0, a.wrapping_add(b));
    lppo_rt_array_set(arr, 1, a.wrapping_sub(b));
    return arr;
}

/// Build an integer array covering `[start, end)` (exclusive) or
/// `[start, end]` (inclusive). Returns a boxed array pointer (0 on empty).
#[no_mangle]
pub extern "C" fn lppo_rt_range(start: i64, end: i64, inclusive: i64) -> *mut i8 {
    let len = if inclusive != 0 {
        if end < start { 0 } else { (end - start + 1) as usize }
    } else {
        if end <= start { 0 } else { (end - start) as usize }
    };
    let arr = lppo_rt_array_make(len as i64);
    for i in 0..len {
        lppo_rt_array_set(arr, i as i64, start + i as i64);
    }
    arr
}

#[allow(dead_code)]
fn either_unused(a: i64, b: i64) -> *mut i8 {
    cstr_to_owned(&format!("[{}, {}]", a + b, a - b))
}

// ---------------------------------------------------------------------------
// Random
// ---------------------------------------------------------------------------

static RNG_SEED: Mutex<u64> = Mutex::new(1);

#[no_mangle]
pub extern "C" fn lppo_rt_random(min: i64, max: i64) -> i64 {
    let mut seed = RNG_SEED.lock().unwrap();
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
        % (1u64 << 63);
    let range = max - min;
    if range <= 0 {
        return min;
    }
    min + (*seed as i64).rem_euclid(range)
}

// ---------------------------------------------------------------------------
// JSON
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn lppo_rt_json_parse(s: *mut i8) -> i64 {
    if s.is_null() {
        return 0;
    }
    unsafe {
        let st = CStr::from_ptr(s as *const i8).to_string_lossy();
        match serde_json::from_str::<serde_json::Value>(&st) {
            Ok(v) => json_to_boxed(&v),
            Err(_) => cstr_to_owned("null") as i64,
        }
    }
}

/// Wrap a (tag, value) pair in a JSON box so printing can recover the tag.
fn json_box(tag: i64, value: i64) -> i64 {
    let obj = unsafe { obj_new(OBJ_TUPLE, 1, 0, 0, 1) };
    unsafe { set_item(obj, 0, tag, value) };
    obj as i64
}

/// Convert a serde JSON value into a boxed LPPO value.
fn json_to_boxed(v: &serde_json::Value) -> i64 {
    match v {
        serde_json::Value::Null => json_box(3, cstr_to_owned("null") as i64),
        serde_json::Value::Bool(b) => json_box(2, *b as i64),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                json_box(0, i)
            } else if let Some(f) = n.as_f64() {
                json_box(1, f64::to_bits(f) as i64)
            } else {
                json_box(0, 0)
            }
        }
        serde_json::Value::String(text) => json_box(3, cstr_to_owned(text) as i64),
        serde_json::Value::Array(items) => {
            let arr = lppo_rt_array_make(items.len() as i64);
            for (i, item) in items.iter().enumerate() {
                let (tag, val) = json_unwrap(json_to_boxed(item));
                lppo_rt_array_set_tag(arr, i as i64, tag, val);
            }
            json_box(4, arr as i64)
        }
        serde_json::Value::Object(map) => {
            let obj = lppo_rt_struct_make(cstr_to_owned("Object"), map.len() as i64);
            for (i, (k, val)) in map.iter().enumerate() {
                let (tag, v) = json_unwrap(json_to_boxed(val));
                lppo_rt_struct_set(obj, i as i64, cstr_to_owned(k), tag, v);
            }
            json_box(10, obj as i64)
        }
    }
}

fn json_unwrap(boxed: i64) -> (i64, i64) {
    if boxed == 0 {
        return (0, 0);
    }
    unsafe {
        let t = item_tag(boxed as *mut i8, 0);
        let v = item_value(boxed as *mut i8, 0);
        (t, v)
    }
}


/// Convert a boxed tagged value back into serde JSON.
fn boxed_to_json(tag: i64, value: i64) -> serde_json::Value {
    match tag {
        0 | 6 => serde_json::Value::Number(serde_json::Number::from(value)),
        1 => serde_json::Number::from_f64(f64::from_bits(value as u64))
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        2 => serde_json::Value::Bool(value != 0),
        3 => {
            if value == 0 {
                serde_json::Value::Null
            } else {
                match serde_json::from_str::<serde_json::Value>(&cstr_of(value)) {
                    Ok(v) => v,
                    Err(_) => serde_json::Value::String(cstr_of(value)),
                }
            }
        }
        5 => serde_json::Value::String(render(tag, value)),
        4 | 7 | 9 => {
            if value == 0 {
                return serde_json::Value::Array(Vec::new());
            }
            let obj = value as *mut i8;
            let len = unsafe { obj_len(obj) };
            let mut items = Vec::with_capacity(len as usize);
            for i in 0..len as usize {
                let t = unsafe { item_tag(obj, i) };
                let v = unsafe { item_value(obj, i) };
                items.push(boxed_to_json(t, v));
            }
            serde_json::Value::Array(items)
        }
        8 => {
            if value == 0 {
                return serde_json::Value::Object(serde_json::Map::new());
            }
            let obj = value as *mut i8;
            let len = unsafe { obj_len(obj) };
            let mut map = serde_json::Map::new();
            for i in 0..len as usize {
                let kt = unsafe { item_tag(obj, i * 2) };
                let kv = unsafe { item_value(obj, i * 2) };
                let vt = unsafe { item_tag(obj, i * 2 + 1) };
                let vv = unsafe { item_value(obj, i * 2 + 1) };
                map.insert(render(kt, kv), boxed_to_json(vt, vv));
            }
            serde_json::Value::Object(map)
        }
        10 => {
            if value == 0 {
                return serde_json::Value::Object(serde_json::Map::new());
            }
            let obj = value as *mut i8;
            let len = unsafe { obj_len(obj) };
            let mut map = serde_json::Map::new();
            for i in 0..len as usize {
                unsafe {
                    let base = obj_items(obj).add(i * 4);
                    let field = cstr_of(*base);
                    let vt = *base.add(1);
                    let vv = *base.add(2);
                    map.insert(field, boxed_to_json(vt, vv));
                }
            }
            serde_json::Value::Object(map)
        }
        11 => {
            let obj = value as *mut i8;
            let len = unsafe { obj_len(obj) };
            let mut items = Vec::with_capacity(len as usize);
            for i in 0..len as usize {
                unsafe {
                    items.push(boxed_to_json(item_tag(obj, i), item_value(obj, i)));
                }
            }
            serde_json::Value::Array(items)
        }
        _ => serde_json::Value::Null,
    }
}

#[no_mangle]
pub extern "C" fn lppo_rt_json_stringify(tag: i64, value: i64) -> *mut i8 {
    if tag == 13 {
        let (t, v) = json_unwrap(value);
        return cstr_to_owned(&boxed_to_json(t, v).to_string());
    }
    if matches!(tag, 4 | 7 | 8 | 9 | 10 | 11) {
        return cstr_to_owned(&boxed_to_json(tag, value).to_string());
    }
    match tag {
        0 => cstr_to_owned(&serde_json::to_string(&value).unwrap_or_else(|_| "0".into())),
        2 => cstr_to_owned(
            &serde_json::to_string(&(value != 0)).unwrap_or_else(|_| "false".into()),
        ),
        3 => {
            if value == 0 {
                cstr_to_owned("null")
            } else {
                let st = cstr_of(value);
                match serde_json::from_str::<serde_json::Value>(&st) {
                    Ok(v) => cstr_to_owned(&v.to_string()),
                    Err(_) => cstr_to_owned(
                        &serde_json::to_string(&st.as_str()).unwrap_or_else(|_| "null".into()),
                    ),
                }
            }
        }
        _ => cstr_to_owned("null"),
    }
}

// ---------------------------------------------------------------------------
// System
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn lppo_rt_system(cmd: *mut i8) -> *mut i8 {
    if cmd.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let c = CStr::from_ptr(cmd as *const i8).to_string_lossy();
        match std::process::Command::new("sh").arg("-c").arg(c.as_ref()).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                cstr_to_owned(&stdout.to_string())
            }
            Err(_) => cstr_to_owned(""),
        }
    }
}

// ---------------------------------------------------------------------------
// Color
// ---------------------------------------------------------------------------

fn color_code(name: &str) -> Option<&'static str> {
    Some(match name {
        "black" => "30", "red" => "31", "green" => "32", "yellow" => "33",
        "blue" => "34", "magenta" => "35", "cyan" => "36", "white" => "37",
        "bold" => "1", "dim" => "2", "italic" => "3", "underline" => "4",
        "blink" => "5", "reverse" => "7", "hidden" => "8", "strike" => "9",
        "bright_black" => "90", "bright_red" => "91", "bright_green" => "92",
        "bright_yellow" => "93", "bright_blue" => "94", "bright_magenta" => "95",
        "bright_cyan" => "96", "bright_white" => "97",
        _ => return None,
    })
}

#[no_mangle]
pub extern "C" fn lppo_rt_color(text: *mut i8, color: *mut i8) -> *mut i8 {
    if text.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let t = CStr::from_ptr(text as *const i8).to_string_lossy();
        if color.is_null() {
            return cstr_to_owned(&t.to_string());
        }
        let c = CStr::from_ptr(color as *const i8).to_string_lossy();
        if c.chars().all(|ch| ch.is_ascii_digit() || ch == ';') && !c.is_empty() {
            return cstr_to_owned(&format!("\x1b[{}m{}\x1b[0m", c, t));
        }
        if c.contains(' ') || c.contains(';') || c.contains(',') {
            let mut codes: Vec<String> = Vec::new();
            for part in c.split(|ch| ch == ' ' || ch == ';' || ch == ',') {
                let p = part.trim();
                if p.is_empty() {
                    continue;
                }
                if let Some(code) = color_code(p) {
                    codes.push(code.to_string());
                }
            }
            if !codes.is_empty() {
                return cstr_to_owned(&format!("\x1b[{}m{}\x1b[0m", codes.join(";"), t));
            }
            return cstr_to_owned(&format!("\x1b[0m{}\x1b[0m", t));
        }
        let code = match c.as_ref() {
            "black" => "30", "red" => "31", "green" => "32", "yellow" => "33",
            "blue" => "34", "magenta" => "35", "cyan" => "36", "white" => "37",
            "bold" => "1", "dim" => "2", "italic" => "3", "underline" => "4",
            "blink" => "5", "reverse" => "7", "hidden" => "8", "strike" => "9",
            "bright_black" => "90", "bright_red" => "91", "bright_green" => "92",
            "bright_yellow" => "93", "bright_blue" => "94", "bright_magenta" => "95",
            "bright_cyan" => "96", "bright_white" => "97",
            _ => "0",
        };
        cstr_to_owned(&format!("\x1b[{}m{}\x1b[0m", code, t))
    }
}

// ---------------------------------------------------------------------------
// File I/O
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn lppo_rt_read_file(path: *mut i8) -> *mut i8 {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let p = CStr::from_ptr(path as *const i8).to_string_lossy();
        match fs::read_to_string(p.as_ref()) {
            Ok(content) => cstr_to_owned(&content),
            Err(_) => std::ptr::null_mut(),
        }
    }
}

#[no_mangle]
pub extern "C" fn lppo_rt_write_file(path: *mut i8, content: *mut i8) -> c_int {
    if path.is_null() || content.is_null() {
        return 0;
    }
    unsafe {
        let p = CStr::from_ptr(path as *const i8).to_string_lossy();
        let c = CStr::from_ptr(content as *const i8).to_string_lossy();
        match fs::write(p.as_ref(), c.as_ref()) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }
}

#[no_mangle]
pub extern "C" fn lppo_rt_append_file(path: *mut i8, content: *mut i8) -> c_int {
    if path.is_null() || content.is_null() {
        return 0;
    }
    unsafe {
        let p = CStr::from_ptr(path as *const i8).to_string_lossy();
        let c = CStr::from_ptr(content as *const i8).to_string_lossy();
        match fs::OpenOptions::new().create(true).append(true).open(p.as_ref()) {
            Ok(mut f) => match f.write_all(c.as_bytes()) {
                Ok(_) => 1,
                Err(_) => 0,
            },
            Err(_) => 0,
        }
    }
}

#[no_mangle]
pub extern "C" fn lppo_rt_file_exists(path: *mut i8) -> c_int {
    if path.is_null() {
        return 0;
    }
    unsafe {
        let p = CStr::from_ptr(path as *const i8).to_string_lossy();
        if std::path::Path::new(p.as_ref()).exists() { 1 } else { 0 }
    }
}

#[no_mangle]
pub extern "C" fn lppo_rt_delete_file(path: *mut i8) -> c_int {
    if path.is_null() {
        return 0;
    }
    unsafe {
        let p = CStr::from_ptr(path as *const i8).to_string_lossy();
        match fs::remove_file(p.as_ref()) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Memory pools
// ---------------------------------------------------------------------------

static POOLS: OnceLock<Mutex<HashMap<u64, Vec<u8>>>> = OnceLock::new();
static NEXT_POOL_ID: OnceLock<Mutex<u64>> = OnceLock::new();

fn pools() -> &'static Mutex<HashMap<u64, Vec<u8>>> {
    POOLS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn next_pool_id() -> &'static Mutex<u64> {
    NEXT_POOL_ID.get_or_init(|| Mutex::new(1))
}

#[no_mangle]
pub extern "C" fn lppo_rt_pool_alloc(size: i64) -> i64 {
    if size <= 0 {
        return 0;
    }
    let id = {
        let mut next = next_pool_id().lock().unwrap();
        let id = *next;
        *next += 1;
        id
    };
    pools().lock().unwrap().insert(id, vec![0u8; size as usize]);
    id as i64
}

#[no_mangle]
pub extern "C" fn lppo_rt_pool_free(id: i64) -> i64 {
    pools().lock().unwrap().remove(&(id as u64));
    0
}

#[no_mangle]
pub extern "C" fn lppo_rt_pool_used() -> i64 {
    pools().lock().unwrap().len() as i64
}


// ---------------------------------------------------------------------------
// Simulated raw pointers / volatile memory
// ---------------------------------------------------------------------------

static PTR_MEM: OnceLock<Mutex<HashMap<i64, i64>>> = OnceLock::new();

fn ptr_mem() -> &'static Mutex<HashMap<i64, i64>> {
    PTR_MEM.get_or_init(|| Mutex::new(HashMap::new()))
}

fn fnv1a(s: &str) -> i64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h as i64
}

static NEXT_ADDR: OnceLock<Mutex<i64>> = OnceLock::new();
static ADDR_MAP: OnceLock<Mutex<HashMap<i64, i64>>> = OnceLock::new();

fn next_addr() -> &'static Mutex<i64> {
    NEXT_ADDR.get_or_init(|| Mutex::new(4096))
}

fn addr_map() -> &'static Mutex<HashMap<i64, i64>> {
    ADDR_MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

#[no_mangle]
pub extern "C" fn lppo_rt_addr_of(value: i64) -> i64 {
    if value == 0 {
        return 0;
    }
    let key = unsafe { fnv1a(&CStr::from_ptr(value as *const i8).to_string_lossy()) };
    let mut map = addr_map().lock().unwrap();
    if let Some(addr) = map.get(&key) {
        return *addr;
    }
    let mut next = next_addr().lock().unwrap();
    let addr = *next;
    *next += 1;
    map.insert(key, addr);
    addr
}

#[no_mangle]
pub extern "C" fn lppo_rt_ptr_read(addr: i64) -> i64 {
    *ptr_mem().lock().unwrap().get(&addr).unwrap_or(&0)
}

#[no_mangle]
pub extern "C" fn lppo_rt_ptr_write(addr: i64, value: i64) {
    ptr_mem().lock().unwrap().insert(addr, value);
}

#[no_mangle]
pub extern "C" fn lppo_rt_ptr_offset(addr: i64, offset: i64) -> i64 {
    let _ = ptr_mem().lock().unwrap().entry(addr + offset).or_insert(0);
    addr + offset
}

#[no_mangle]
pub extern "C" fn lppo_rt_volatile_read(addr: i64) -> i64 {
    lppo_rt_ptr_read(addr)
}

#[no_mangle]
pub extern "C" fn lppo_rt_volatile_write(addr: i64, value: i64) {
    lppo_rt_ptr_write(addr, value);
}

#[no_mangle]
pub extern "C" fn lppo_rt_inline_asm(code: i64, args: i64) -> i64 {
    let text = cstr_arg(code as *mut i8);
    let first = text.split_whitespace().next().unwrap_or("");
    let allowed = ["nop", "mov", "add", "sub", "xor", "and", "or", "ret"];
    if !allowed.iter().any(|w| first.eq_ignore_ascii_case(w)) {
        return 0;
    }
    let mut total: i64 = 0;
    for v in unsafe { arr_values(args as *mut i8) } {
        total = total.wrapping_add(v);
    }
    total
}

// ---------------------------------------------------------------------------
// SIMD built-ins (array-backed)
// ---------------------------------------------------------------------------

unsafe fn arr_values(arr: *mut i8) -> Vec<i64> {
    let len = lppo_rt_array_len(arr);
    (0..len).map(|i| lppo_rt_array_get(arr, i)).collect()
}

fn make_array(values: &[i64]) -> *mut i8 {
    let arr = lppo_rt_array_make(values.len() as i64);
    for (i, v) in values.iter().enumerate() {
        lppo_rt_array_set(arr, i as i64, *v);
    }
    arr
}

#[no_mangle]
pub extern "C" fn lppo_rt_simd_splat(value: i64, n: i64) -> *mut i8 {
    let n = if n < 0 { 0 } else { n as usize };
    make_array(&vec![value; n])
}

fn simd_zip(a: *mut i8, b: *mut i8, f: impl Fn(i64, i64) -> i64) -> *mut i8 {
    let av = unsafe { arr_values(a) };
    let bv = unsafe { arr_values(b) };
    let mut out = Vec::with_capacity(av.len());
    for (i, x) in av.iter().enumerate() {
        let y = bv.get(i).copied().unwrap_or(0);
        out.push(f(*x, y));
    }
    make_array(&out)
}

#[no_mangle]
pub extern "C" fn lppo_rt_simd_add(a: *mut i8, b: *mut i8) -> *mut i8 {
    simd_zip(a, b, |x, y| x.wrapping_add(y))
}

#[no_mangle]
pub extern "C" fn lppo_rt_simd_sub(a: *mut i8, b: *mut i8) -> *mut i8 {
    simd_zip(a, b, |x, y| x.wrapping_sub(y))
}

#[no_mangle]
pub extern "C" fn lppo_rt_simd_mul(a: *mut i8, b: *mut i8) -> *mut i8 {
    simd_zip(a, b, |x, y| x.wrapping_mul(y))
}

#[no_mangle]
pub extern "C" fn lppo_rt_simd_div(a: *mut i8, b: *mut i8) -> *mut i8 {
    simd_zip(a, b, |x, y| if y == 0 { 0 } else { x / y })
}

#[no_mangle]
pub extern "C" fn lppo_rt_simd_extract(arr: *mut i8, idx: i64) -> i64 {
    lppo_rt_array_get(arr, idx)
}

#[no_mangle]
pub extern "C" fn lppo_rt_simd_insert(arr: *mut i8, idx: i64, value: i64) -> *mut i8 {
    let mut v = unsafe { arr_values(arr) };
    if idx >= 0 && (idx as usize) < v.len() {
        v[idx as usize] = value;
    }
    make_array(&v)
}

#[no_mangle]
pub extern "C" fn lppo_rt_simd_shuffle(arr: *mut i8, indices: *mut i8) -> *mut i8 {
    let v = unsafe { arr_values(arr) };
    let idx = unsafe { arr_values(indices) };
    let out: Vec<i64> = idx
        .iter()
        .map(|i| {
            if *i >= 0 && (*i as usize) < v.len() {
                v[*i as usize]
            } else {
                0
            }
        })
        .collect();
    make_array(&out)
}

#[no_mangle]
pub extern "C" fn lppo_rt_simd_reduce(arr: *mut i8, op: *mut i8) -> i64 {
    let v = unsafe { arr_values(arr) };
    let name = if op == std::ptr::null_mut() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(op as *const i8).to_string_lossy().to_string() }
    };
    match name.as_str() {
        "mul" => v.iter().fold(1i64, |a, b| a.wrapping_mul(*b)),
        "min" => v.iter().copied().min().unwrap_or(0),
        "max" => v.iter().copied().max().unwrap_or(0),
        _ => v.iter().fold(0i64, |a, b| a.wrapping_add(*b)),
    }
}

// ---------------------------------------------------------------------------
// Terminal UI helpers
// ---------------------------------------------------------------------------

fn print_flush(text: &str) {
    print!("{}", text);
    let _ = std::io::stdout().flush();
}

fn color_prefix(code: &str) -> String {
    if code.chars().all(|c| c.is_ascii_digit() || c == ';') && !code.is_empty() {
        return format!("\x1b[{}m", code);
    }
    let c = match code {
        "reset" | "" => return String::new(),
        "bold" => "1",
        "dim" => "2",
        "italic" => "3",
        "underline" => "4",
        "blink" => "5",
        "reverse" => "7",
        "hidden" => "8",
        "black" => "30",
        "red" => "31",
        "green" => "32",
        "yellow" => "33",
        "blue" => "34",
        "magenta" => "35",
        "cyan" => "36",
        "white" => "37",
        "bright_black" => "90",
        "bright_red" => "91",
        "bright_green" => "92",
        "bright_yellow" => "93",
        "bright_blue" => "94",
        "bright_magenta" => "95",
        "bright_cyan" => "96",
        "bright_white" => "97",
        _ => return String::new(),
    };
    format!("\x1b[{}m", c)
}

fn terminal_dims() -> (i64, i64) {
    if let Ok(out) = std::process::Command::new("tput").arg("lines").output() {
        if let Ok(v) = String::from_utf8_lossy(&out.stdout).trim().parse::<i64>() {
            if v > 0 {
                if let Ok(out2) = std::process::Command::new("tput").arg("cols").output() {
                    if let Ok(c) = String::from_utf8_lossy(&out2.stdout).trim().parse::<i64>() {
                        if c > 0 {
                            return (v, c);
                        }
                    }
                }
            }
        }
    }
    (24, 80)
}

#[no_mangle]
pub extern "C" fn lppo_rt_tui_init() {
    print_flush("\x1b[?1049h\x1b[?25l\x1b[2J\x1b[H");
}

#[no_mangle]
pub extern "C" fn lppo_rt_tui_end() {
    print_flush("\x1b[2J\x1b[H\x1b[?25h\x1b[?1049l");
}

#[no_mangle]
pub extern "C" fn lppo_rt_tui_clear() {
    print_flush("\x1b[2J\x1b[H");
}

#[no_mangle]
pub extern "C" fn lppo_rt_tui_move(x: i64, y: i64) {
    print_flush(&format!("\x1b[{};{}H", y, x));
}

#[no_mangle]
pub extern "C" fn lppo_rt_tui_put(text: i64, x: i64, y: i64) {
    print_flush(&format!("\x1b[{};{}H{}", y, x, cstr_arg(text as *mut i8)));
}

#[no_mangle]
pub extern "C" fn lppo_rt_tui_puts(text: i64, x: i64, y: i64, color: i64) {
    let col = cstr_arg(color as *mut i8);
    print_flush(&format!(
        "\x1b[{};{}H{}{}\x1b[0m",
        y,
        x,
        color_prefix(&col),
        cstr_arg(text as *mut i8)
    ));
}

#[no_mangle]
pub extern "C" fn lppo_rt_tui_cls(x: i64, y: i64, w: i64, h: i64) {
    let mut buf = String::new();
    for row in 0..h {
        buf.push_str(&format!(
            "\x1b[{};{}H{}",
            y + row,
            x,
            " ".repeat(w.max(0) as usize)
        ));
    }
    print_flush(&buf);
}

#[no_mangle]
pub extern "C" fn lppo_rt_tui_box(x: i64, y: i64, w: i64, h: i64, title: i64) {
    let w = w.max(0) as usize;
    let h = h.max(0) as usize;
    if w < 2 || h < 2 {
        return;
    }
    let title = cstr_arg(title as *mut i8);
    let mut buf = String::new();
    buf.push_str(&format!("\x1b[{};{}H┌{}┐", y, x, "─".repeat(w - 2)));
    for row in 1..(h - 1) {
        buf.push_str(&format!(
            "\x1b[{};{}H│{}│",
            y + row as i64,
            x,
            " ".repeat(w - 2)
        ));
    }
    buf.push_str(&format!("\x1b[{};{}H└{}┘", y + (h - 1) as i64, x, "─".repeat(w - 2)));
    if !title.is_empty() && w > title.len() + 2 {
        let t = &title[..title.len().min(w - 4)];
        buf.push_str(&format!(
            "\x1b[{};{}H {} {}",
            y,
            x + 1,
            t,
            " ".repeat(w.saturating_sub(t.len() + 3))
        ));
    }
    print_flush(&buf);
}

#[no_mangle]
pub extern "C" fn lppo_rt_tui_title(title: i64) {
    print_flush(&format!("\x1b]0;{}\x07", cstr_arg(title as *mut i8)));
}

#[no_mangle]
pub extern "C" fn lppo_rt_tui_style(style: i64) {
    let st = cstr_arg(style as *mut i8);
    let mut buf = String::new();
    for part in st.split([',', ' ']).filter(|p| !p.is_empty()) {
        buf.push_str(&color_prefix(part));
    }
    print_flush(&buf);
}

#[no_mangle]
pub extern "C" fn lppo_rt_tui_reset() {
    print_flush("\x1b[0m");
}

#[no_mangle]
pub extern "C" fn lppo_rt_tui_progress(value: i64, max: i64, x: i64, y: i64, w: i64) {
    let max_v = if max == 0 { 1 } else { max };
    let w = w.max(0) as usize;
    let ratio = (value as f64).clamp(0.0, max_v as f64) / max_v as f64;
    let filled = (ratio * w as f64) as usize;
    let bar = format!(
        "{}{}",
        "█".repeat(filled.min(w)),
        "░".repeat(w.saturating_sub(filled))
    );
    print_flush(&format!("\x1b[{};{}H{}", y, x, bar));
}

#[no_mangle]
pub extern "C" fn lppo_rt_tui_size() -> *mut i8 {
    let (rows, cols) = terminal_dims();
    make_array(&[rows, cols])
}

#[no_mangle]
pub extern "C" fn lppo_rt_tui_fg(color: i64) {
    print_flush(&color_prefix(&cstr_arg(color as *mut i8)));
}

#[no_mangle]
pub extern "C" fn lppo_rt_tui_bold(on: i64) {
    print_flush(if on != 0 { "\x1b[1m" } else { "\x1b[22m" });
}

#[no_mangle]
pub extern "C" fn lppo_rt_tui_underline(on: i64) {
    print_flush(if on != 0 { "\x1b[4m" } else { "\x1b[24m" });
}

#[no_mangle]
pub extern "C" fn lppo_rt_tui_hide_cursor() {
    print_flush("\x1b[?25l");
}

#[no_mangle]
pub extern "C" fn lppo_rt_tui_show_cursor() {
    print_flush("\x1b[?25h");
}

#[no_mangle]
pub extern "C" fn lppo_rt_tui_refresh() {
    let _ = std::io::stdout().flush();
}

// ---------------------------------------------------------------------------
// Panic
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn lppo_rt_panic(message: *mut i8) {
    if !message.is_null() {
        unsafe {
            let cstr = CStr::from_ptr(message as *const i8);
            eprintln!("LPPO runtime panic: {}", cstr.to_string_lossy());
        }
    } else {
        eprintln!("LPPO runtime panic");
    }
    std::process::abort();
}
