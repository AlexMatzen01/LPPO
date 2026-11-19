//! Interrupt Descriptor Table.

use crate::arch::gdt;

#[repr(C, align(8))]
#[derive(Copy, Clone)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    zero: u32,
}

#[repr(packed)]
struct IdtPtr {
    limit: u16,
    base: u64,
}

const TYPE_INT_GATE: u8 = 0x8E; // present, DPL=0, 64-bit interrupt gate

static mut IDT: [IdtEntry; 256] = [IdtEntry {
    offset_low: 0,
    selector: 0,
    ist: 0,
    type_attr: 0,
    offset_mid: 0,
    offset_high: 0,
    zero: 0,
}; 256];

pub fn idt_set(index: u16, handler: u64) {
    unsafe {
        let entry = &mut IDT[index as usize];
        entry.offset_low = (handler & 0xFFFF) as u16;
        entry.selector = gdt::GDT_CODE64;
        entry.ist = 0;
        entry.type_attr = TYPE_INT_GATE;
        entry.offset_mid = ((handler >> 16) & 0xFFFF) as u16;
        entry.offset_high = (handler >> 32) as u32;
        entry.zero = 0;
    }
}

pub fn init() {
    unsafe {
        let default: u64 = default_handler as *const extern "C" fn() as u64;
        for i in 0..256usize {
            idt_set(i as u16, default);
        }
        let ptr = IdtPtr {
            limit: (core::mem::size_of::<[IdtEntry; 256]>() - 1) as u16,
            base: &IDT as *const _ as u64,
        };
        core::arch::asm!("lidt [{0}]", in(reg) &ptr, options(nostack, preserves_flags));
    }
}

extern "C" fn default_handler() -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nostack));
        }
    }
}

/// Remap the dual 8259 PICs to vectors 0x20..0x2F and clear masks.
pub fn pic_remap() {
    unsafe {
        crate::arch::io::outb(0x20, 0x11);
        crate::arch::io::outb(0xA0, 0x11);
        crate::arch::io::outb(0x21, 0x20);
        crate::arch::io::outb(0xA1, 0x28);
        crate::arch::io::outb(0x21, 0x04);
        crate::arch::io::outb(0xA1, 0x02);
        crate::arch::io::outb(0x21, 0x01);
        crate::arch::io::outb(0xA1, 0x01);
        crate::arch::io::outb(0x21, 0x00);
        crate::arch::io::outb(0xA1, 0x00);
    }
}

/// Send End-Of-Interrupt to the PIC(s).
pub fn pic_eoi(irq: u8) {
    unsafe {
        crate::arch::io::outb(0x20, 0x20);
        if irq >= 8 {
            crate::arch::io::outb(0xA0, 0x20);
        }
    }
}
