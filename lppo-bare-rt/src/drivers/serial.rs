//! PC COM1 (16550 UART) serial driver.

use crate::arch::io::{self, io_wait};

const COM1: u16 = 0x3F8;
const REG_DATA: u16 = 0x00; // DLAB=0
const REG_INTEN: u16 = 0x01;
const REG_FIFO: u16 = 0x02;
const REG_LCR: u16 = 0x03;
const REG_MCR: u16 = 0x04;
const REG_LSR: u16 = 0x05;
const LSR_TX_READY: u8 = 0x20;

pub fn init() {
    unsafe {
        // Disable interrupts.
        io::outb(COM1 + REG_INTEN, 0x00);
        // Enable DLAB to set baud rate divisor.
        io::outb(COM1 + REG_LCR, 0x80);
        // 115200 baud: divisor = 1 (with 1.8432 MHz clock / 16 / 1 = 115200).
        io::outb(COM1 + REG_DATA, 0x01);
        io::outb(COM1 + REG_DATA + 1, 0x00);
        // 8N1, clear DLAB.
        io::outb(COM1 + REG_LCR, 0x03);
        // Enable FIFOs, clear/rx reset.
        io::outb(COM1 + REG_FIFO, 0xC7);
        // RTS/DTR set.
        io::outb(COM1 + REG_MCR, 0x03);
        io_wait();
    }
}

fn tx_ready() -> bool {
    io::inb(COM1 + REG_LSR) & LSR_TX_READY != 0
}

pub fn write_byte(byte: u8) {
    unsafe {
        while !tx_ready() {
            io_wait();
        }
        io::outb(COM1 + REG_DATA, byte);
    }
}

pub fn write_str(s: &str) {
    for byte in s.bytes() {
        write_byte(byte);
    }
}

pub fn write_hex(value: u64) {
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    write_str("0x");
    let mut nibbles = [0u8; 16];
    let mut n = value;
    let mut len = 0;
    if n == 0 {
        write_byte(b'0');
        return;
    }
    while n > 0 {
        nibbles[len] = DIGITS[(n & 0xF) as usize];
        n >>= 4;
        len += 1;
    }
    while len > 0 {
        len -= 1;
        write_byte(nibbles[len]);
    }
}
