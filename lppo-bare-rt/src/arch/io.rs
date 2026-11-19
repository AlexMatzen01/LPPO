//! Raw port-mapped I/O primitives.

pub fn outb(port: u16, val: u8) {
    unsafe {
        core::arch::asm!("outb %al, %dx", in("al") val, in("dx") port, options(nostack, att_syntax));
    }
}

pub fn outw(port: u16, val: u16) {
    unsafe {
        core::arch::asm!("outw %ax, %dx", in("ax") val, in("dx") port, options(nostack, att_syntax));
    }
}

pub fn outl(port: u16, val: u32) {
    unsafe {
        core::arch::asm!("outl %eax, %dx", in("eax") val, in("dx") port, options(nostack, att_syntax));
    }
}

pub fn inb(port: u16) -> u8 {
    let val: u8;
    unsafe {
        core::arch::asm!("inb %dx, %al", out("al") val, in("dx") port, options(nostack, att_syntax));
    }
    val
}

pub fn inw(port: u16) -> u16 {
    let val: u16;
    unsafe {
        core::arch::asm!("inw %dx, %ax", out("ax") val, in("dx") port, options(nostack, att_syntax));
    }
    val
}

pub fn inl(port: u16) -> u32 {
    let val: u32;
    unsafe {
        core::arch::asm!("inl %dx, %eax", out("eax") val, in("dx") port, options(nostack, att_syntax));
    }
    val
}

/// Brief I/O delay (port 0x80 is a safe, unused port on most chipsets).
pub fn io_wait() {
    unsafe {
        core::arch::asm!("mov $0x80, %dx; mov $0, %al; outb %al, %dx", options(nostack, att_syntax));
    }
}
