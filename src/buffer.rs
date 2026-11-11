//! String buffer registry shared between LPPO and native packages.
//!
//! LPPO programs manipulate buffers via builtins (`buf_new`, `buf_set`,
//! `buf_get`, `buf_append`, `buf_len`). Native packages (e.g. LPPNet, LPPWeb)
//! receive buffer ids as `i64` and read/write buffer contents through the
//! `#[no_mangle]` C accessors exported below, resolved via `libloading` on
//! the main executable (`/proc/self/exe`). Both sides share the same
//! in-process registry, so string data can flow between LPPO and native
//! code without lossy hashing.

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

static BUFFERS: Lazy<Mutex<HashMap<u64, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static NEXT_BUF_ID: Lazy<Mutex<AtomicBufId>> = Lazy::new(|| Mutex::new(AtomicBufId::new(1)));

use std::sync::atomic::{AtomicU64, Ordering};

struct AtomicBufId(AtomicU64);

impl AtomicBufId {
    fn new(v: u64) -> Self {
        Self(AtomicU64::new(v))
    }
    fn next(&self) -> u64 {
        self.0.fetch_add(1, Ordering::Relaxed)
    }
}

fn next_buf_id() -> u64 {
    NEXT_BUF_ID.lock().unwrap().next()
}

////////////////////////////////////////////////////////////////////////////////
// LPPO-side builtin helpers
////////////////////////////////////////////////////////////////////////////////

pub fn buf_new() -> u64 {
    let id = next_buf_id();
    BUFFERS.lock().unwrap().insert(id, String::new());
    id
}

pub fn buf_set(id: u64, data: &str) -> bool {
    let mut buffers = BUFFERS.lock().unwrap();
    if buffers.contains_key(&id) {
        buffers.insert(id, data.to_string());
        true
    } else {
        false
    }
}

pub fn buf_get(id: u64) -> Option<String> {
    BUFFERS.lock().unwrap().get(&id).cloned()
}

pub fn buf_append(id: u64, data: &str) -> bool {
    let mut buffers = BUFFERS.lock().unwrap();
    match buffers.get_mut(&id) {
        Some(s) => {
            s.push_str(data);
            true
        }
        None => false,
    }
}

pub fn buf_len(id: u64) -> Option<i64> {
    BUFFERS
        .lock()
        .unwrap()
        .get(&id)
        .map(|s| s.len() as i64)
}

pub fn buf_clear(id: u64) -> bool {
    let mut buffers = BUFFERS.lock().unwrap();
    match buffers.get_mut(&id) {
        Some(s) => {
            s.clear();
            true
        }
        None => false,
    }
}

pub fn buf_byte(id: u64, index: i64) -> Option<i64> {
    let buffers = BUFFERS.lock().unwrap();
    let s = buffers.get(&id)?;
    let idx = if index < 0 {
        (s.len() as i64 + index) as usize
    } else {
        index as usize
    };
    s.as_bytes().get(idx).map(|b| *b as i64)
}

////////////////////////////////////////////////////////////////////////////////
// C accessors for native packages
////////////////////////////////////////////////////////////////////////////////

/// Create a new empty buffer. Returns its id.
#[no_mangle]
pub extern "C" fn lppo_buf_new() -> i64 {
    buf_new() as i64
}

/// Set buffer contents from a raw pointer + length. Returns 1 on success, 0 if unknown id.
#[no_mangle]
pub extern "C" fn lppo_buf_set(buf_id: i64, data: *const u8, len: i64) -> i64 {
    let data = unsafe {
        if data.is_null() || len < 0 {
            return 0;
        }
        std::slice::from_raw_parts(data, len as usize)
    };
    if buf_set(buf_id as u64, std::str::from_utf8(data).unwrap_or("")) {
        1
    } else {
        0
    }
}

/// Copy buffer contents into a native-owned buffer. Returns bytes written (or -1 if unknown id / too small).
#[no_mangle]
pub extern "C" fn lppo_buf_get(buf_id: i64, out: *mut u8, out_len: i64) -> i64 {
    let contents = match buf_get(buf_id as u64) {
        Some(s) => s,
        None => return -1,
    };
    let bytes = contents.as_bytes();
    if out.is_null() || out_len < 0 {
        return -1;
    }
    let copy_len = bytes.len().min(out_len as usize);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, copy_len);
    }
    copy_len as i64
}

/// Return the length of a buffer's contents (or -1 if unknown id).
#[no_mangle]
pub extern "C" fn lppo_buf_len(buf_id: i64) -> i64 {
    match buf_len(buf_id as u64) {
        Some(n) => n,
        None => -1,
    }
}

/// Append raw data to a buffer. Returns 1 on success, 0 if unknown id.
#[no_mangle]
pub extern "C" fn lppo_buf_append(buf_id: i64, data: *const u8, len: i64) -> i64 {
    let data = unsafe {
        if data.is_null() || len < 0 {
            return 0;
        }
        std::slice::from_raw_parts(data, len as usize)
    };
    if buf_append(buf_id as u64, std::str::from_utf8(data).unwrap_or("")) {
        1
    } else {
        0
    }
}

/// Clear a buffer's contents. Returns 1 on success, 0 if unknown id.
#[no_mangle]
pub extern "C" fn lppo_buf_clear(buf_id: i64) -> i64 {
    if buf_clear(buf_id as u64) {
        1
    } else {
        0
    }
}

/// Return the byte at `index` of a buffer (or -1 if unknown id / out of bounds).
#[no_mangle]
pub extern "C" fn lppo_buf_byte(buf_id: i64, index: i64) -> i64 {
    match buf_byte(buf_id as u64, index) {
        Some(b) => b,
        None => -1,
    }
}
