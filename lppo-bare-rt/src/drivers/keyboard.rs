//! Polled PS/2 keyboard driver.

use crate::arch::io::{self, io_wait};

const KEYBOARD_DATA: u16 = 0x60;
const KEYBOARD_STATUS: u16 = 0x64;
const STATUS_OUTPUT_FULL: u8 = 0x01;

pub fn init() {
    // Enable keyboard scanning (IRQ1). Command 0xAE to keyboard controller enables the port.
    unsafe {
        io::outb(KEYBOARD_STATUS, 0xAE);
        io_wait();
    }
}

/// Returns `Some(scan_code)` when a key event is available, else `None`.
pub fn poll_scan() -> Option<u8> {
    unsafe {
        let status = io::inb(KEYBOARD_STATUS);
        if status & STATUS_OUTPUT_FULL != 0 {
            return Some(io::inb(KEYBOARD_DATA));
        }
        None
    }
}

/// Block until a scan code is available, then return it.
pub fn wait_scan() -> u8 {
    loop {
        if let Some(sc) = poll_scan() {
            return sc;
        }
    }
}
