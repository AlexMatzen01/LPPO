//! VGA text-mode display driver (0xB8000).

use alloc::string::String;
use alloc::borrow::ToOwned;

const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;
const VGA_MEMORY: *mut u16 = 0xB8000 as *mut u16;

struct Cursor {
    x: usize,
    y: usize,
}

static mut CURSOR: Cursor = Cursor { x: 0, y: 0 };

pub fn vga_put_char(ch: char) {
    unsafe {
        match ch {
            '\n' => {
                CURSOR.x = 0;
                CURSOR.y += 1;
            }
            '\t' => {
                for _ in 0..4 {
                    vga_put_char(' ');
                }
            }
            ch => {
                let offset = (CURSOR.y * VGA_WIDTH + CURSOR.x) * 2;
                let byte = ch as u8;
                let color: u8 = 0x0F; // white on black
                core::ptr::write_volatile(
                    VGA_MEMORY.add(offset),
                    (byte as u16) | ((color as u16) << 8),
                );
                CURSOR.x += 1;
                if CURSOR.x >= VGA_WIDTH {
                    CURSOR.x = 0;
                    CURSOR.y += 1;
                }
            }
        }
        if CURSOR.y >= VGA_HEIGHT {
            vga_scroll();
            CURSOR.y = VGA_HEIGHT - 1;
        }
    }
}

fn vga_scroll() {
    unsafe {
        for row in 1..VGA_HEIGHT {
            for col in 0..VGA_WIDTH {
                let src = ((row * VGA_WIDTH + col) * 2) as isize;
                let dst = (((row - 1) * VGA_WIDTH + col) * 2) as isize;
                let val = core::ptr::read_volatile(VGA_MEMORY.offset(src));
                core::ptr::write_volatile(VGA_MEMORY.offset(dst), val);
            }
        }
        for col in 0..VGA_WIDTH {
            let offset = (((VGA_HEIGHT - 1) * VGA_WIDTH + col) * 2) as isize;
            core::ptr::write_volatile(
                VGA_MEMORY.offset(offset),
                (0x20 as u16) | ((0x0F as u16) << 8),
            );
        }
    }
}

pub fn vga_write_str(s: &str) {
    for ch in s.chars() {
        vga_put_char(ch);
    }
}

pub fn vga_clear() {
    unsafe {
        for i in 0..(VGA_WIDTH * VGA_HEIGHT) {
            core::ptr::write_volatile(VGA_MEMORY.add(i), (0x20 as u16) | ((0x0F as u16) << 8));
        }
        CURSOR.x = 0;
        CURSOR.y = 0;
    }
}

pub fn i64_to_string(n: i64) -> String {
    if n == i64::MIN {
        return "-9223372036854775808".to_owned();
    }
    let mut s = String::new();
    let neg = n < 0;
    let mut m: u64 = if neg { (0i64 - n) as u64 } else { n as u64 };
    if neg {
        s.push('-');
    }
    if m == 0 {
        s.push('0');
    } else {
        let mut buf = [0u8; 20];
        let mut len = 0usize;
        while m > 0 {
            buf[len] = b'0' + (m % 10) as u8;
            m /= 10;
            len += 1;
        }
        while len > 0 {
            len -= 1;
            s.push(buf[len] as char);
        }
    }
    s
}
