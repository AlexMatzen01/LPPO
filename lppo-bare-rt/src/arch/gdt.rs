//! Global Descriptor Table.

#[repr(C, align(8))]
struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_mid: u8,
    access: u8,
    flags: u8,
    base_high: u8,
}

#[repr(packed)]
struct GdtPtr {
    limit: u16,
    base: u64,
}

pub const GDT_NULL: u16 = 0x00;
pub const GDT_CODE64: u16 = 0x08;
pub const GDT_DATA64: u16 = 0x10;

static mut GDT: [GdtEntry; 3] = [
    GdtEntry { limit_low: 0, base_low: 0, base_mid: 0, access: 0, flags: 0, base_high: 0 },
    // 64-bit code segment: G=1, L=1, base=0, limit=0xFFFFF, present, DPL=0, exec/read.
    GdtEntry { limit_low: 0xFFFF, base_low: 0, base_mid: 0, access: 0x9A, flags: 0xA0, base_high: 0 },
    // Data segment: G=1, D/B=1, base=0, limit=0xFFFFF, present, DPL=0, write.
    GdtEntry { limit_low: 0xFFFF, base_low: 0, base_mid: 0, access: 0x92, flags: 0xCF, base_high: 0 },
];

pub fn init() {
    unsafe {
        let ptr = GdtPtr {
            limit: (core::mem::size_of::<[GdtEntry; 3]>() - 1) as u16,
            base: &GDT as *const _ as u64,
        };
        core::arch::asm!("lgdt [{0}]", in(reg) &ptr, options(nostack, preserves_flags));
        // Reload data segments with the flat data descriptor.
        core::arch::asm!(
            "mov ax, 0x10",
            "mov ds, ax",
            "mov es, ax",
            "mov ss, ax",
            "mov fs, ax",
            "mov gs, ax",
            options(nostack, preserves_flags)
        );
    }
}
