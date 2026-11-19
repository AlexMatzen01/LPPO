//! Multiboot2 header emission and boot-info parsing.

use alloc::vec::Vec;
use core::slice;

const MULTIBOOT2_MAGIC: u32 = 0xE852_50D6;
const MULTIBOOT_ARCH_I386: u32 = 0;
const MULTIBOOT_TAG_TYPE_END: u16 = 0;
const MULTIBOOT_TAG_TYPE_CMDLINE: u16 = 1;
const MULTIBOOT_TAG_TYPE_BOOT_LOADER_NAME: u16 = 2;
const MULTIBOOT_TAG_TYPE_MODULE: u16 = 3;
const MULTIBOOT_TAG_TYPE_BASIC_MEMINFO: u16 = 4;
const MULTIBOOT_TAG_TYPE_BOOTDEV: u16 = 5;
const MULTIBOOT_TAG_TYPE_MMAP: u16 = 6;
const MULTIBOOT_TAG_TYPE_FRAMEBUFFER: u16 = 8;
const MULTIBOOT_TAG_TYPE_ELF_SECTIONS: u16 = 9;
const MULTIBOOT_TAG_TYPE_APM: u16 = 10;
const MULTIBOOT_TAG_TYPE_EFI32: u16 = 11;
const MULTIBOOT_TAG_TYPE_EFI64: u16 = 12;
const MULTIBOOT_TAG_TYPE_SMBIOS: u16 = 13;
const MULTIBOOT_TAG_TYPE_ACPI_OLD: u16 = 14;
const MULTIBOOT_TAG_TYPE_ACPI_NEW: u16 = 15;
const MULTIBOOT_TAG_TYPE_NETWORK: u16 = 16;
const MULTIBOOT_TAG_TYPE_EFI_MMAP: u16 = 17;
const MULTIBOOT_TAG_TYPE_EFI_BS: u16 = 18;
const MULTIBOOT_TAG_TYPE_EFI32_IH: u16 = 19;
const MULTIBOOT_TAG_TYPE_EFI64_IH: u16 = 20;
const MULTIBOOT_TAG_TYPE_LOAD_BASE_ADDR: u16 = 21;

pub const MULTIBOOT_MEMORY_AVAILABLE: u32 = 1;
const MULTIBOOT_MEMORY_RESERVED: u32 = 2;
const MULTIBOOT_MEMORY_ACPI: u32 = 3;
const MULTIBOOT_MEMORY_NVS: u32 = 4;
const MULTIBOOT_MEMORY_BADRAM: u32 = 5;

const MAX_MEM_REGIONS: usize = 64;

#[repr(C, align(8))]
struct MultibootHeader {
    magic: u32,
    architecture: u32,
    header_length: u32,
    checksum: u32,
    end_tag_type: u16,
    end_tag_flags: u16,
    end_tag_size: u32,
}

const fn multiboot_checksum(magic: u32, arch: u32, header_len: u32) -> u32 {
    let mut sum: u32 = magic.wrapping_add(arch).wrapping_add(header_len);
    sum = sum.wrapping_add(8); // end tag size field
    sum.wrapping_neg()
}

const HEADER: MultibootHeader = MultibootHeader {
    magic: MULTIBOOT2_MAGIC,
    architecture: MULTIBOOT_ARCH_I386,
    header_length: core::mem::size_of::<MultibootHeader>() as u32,
    checksum: multiboot_checksum(MULTIBOOT2_MAGIC, MULTIBOOT_ARCH_I386, 40),
    end_tag_type: MULTIBOOT_TAG_TYPE_END,
    end_tag_flags: 0,
    end_tag_size: 8,
};

#[link_section = ".multiboot"]
#[used]
static MULTIBOOT_HEADER: MultibootHeader = HEADER;

#[repr(C, align(8))]
struct MultibootTag {
    typ: u16,
    flags: u16,
    size: u32,
}

#[repr(C, align(8))]
struct MultibootMmapEntry {
    addr: u64,
    len: u64,
    r#type: u32,
    zero: u32,
}

#[repr(C)]
struct BootInfo {
    total_size: u32,
    reserved: u32,
}

/// Parsed physical memory region from the Multiboot2 mmap tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemRegion {
    pub addr: u64,
    pub len: u64,
    pub r#type: u32,
}

/// Parsed boot information, best-effort. All fields are `None` when no valid
/// Multiboot2 boot info structure is present (e.g. QEMU `-kernel` without GRUB).
pub struct BootInfoParsed {
    pub cmdline: Option<&'static str>,
    pub boot_loader_name: Option<&'static str>,
    pub mem_regions: Vec<MemRegion>,
    pub mem_lower_kib: Option<u32>,
    pub mem_upper_kib: Option<u32>,
}

/// Read the boot-info pointer passed by the bootloader (in RBX/EBX).
fn boot_info_ptr() -> Option<usize> {
    let ptr: usize;
    unsafe {
        core::arch::asm!("mov {0}, rbx", out(reg) ptr, options(nostack));
    }
    if ptr == 0 {
        None
    } else {
        Some(ptr)
    }
}

fn read_cstr<'a>(ptr: usize, max_len: usize) -> Option<&'a str> {
    if ptr == 0 {
        return None;
    }
    let bytes = unsafe { slice::from_raw_parts(ptr as *const u8, max_len) };
    let len = bytes.iter().position(|&b| b == 0).unwrap_or(max_len);
    core::str::from_utf8(&bytes[..len]).ok()
}

fn parse_boot_info() -> BootInfoParsed {
    let mut regions: Vec<MemRegion> = Vec::new();
    let ptr = match boot_info_ptr() {
        Some(p) => p,
        None => return BootInfoParsed::empty(),
    };
    let bi: &BootInfo = unsafe { &*(ptr as *const BootInfo) };
    if bi.total_size < 8 {
        return BootInfoParsed::empty();
    }
    let end = ptr.wrapping_add(bi.total_size as usize);
    let mut off = 8usize;
    while off + 8 <= end.saturating_sub(ptr) {
        let tag_ptr = ptr.wrapping_add(off);
        let tag: &MultibootTag = unsafe { &*(tag_ptr as *const MultibootTag) };
        if tag.typ == MULTIBOOT_TAG_TYPE_END {
            break;
        }
        let size = tag.size as usize;
        if size < 8 || tag_ptr.checked_add(size).map_or(true, |s| s > end) {
            break;
        }
        match tag.typ {
            MULTIBOOT_TAG_TYPE_MMAP => {
                let entry_size = unsafe { *((tag_ptr.wrapping_add(8)) as *const u32) } as usize;
                let mut entry_off = 16usize;
                while entry_off
                    .checked_add(core::mem::size_of::<MultibootMmapEntry>())
                    .map_or(false, |e| e <= size)
                {
                    let e: &MultibootMmapEntry =
                        unsafe { &*((tag_ptr.wrapping_add(entry_off)) as *const MultibootMmapEntry) };
                    if regions.len() < MAX_MEM_REGIONS {
                        regions.push(MemRegion {
                            addr: e.addr,
                            len: e.len,
                            r#type: e.r#type,
                        });
                    }
                    entry_off = entry_off.wrapping_add(entry_size);
                }
            }
            MULTIBOOT_TAG_TYPE_BASIC_MEMINFO => {
                let _lower = unsafe { *((tag_ptr.wrapping_add(8)) as *const u32) };
                let _upper = unsafe { *((tag_ptr.wrapping_add(12)) as *const u32) };
            }
            _ => {}
        }
        off = off.wrapping_add((size + 7) & !7);
    }
    BootInfoParsed {
        cmdline: None,
        boot_loader_name: None,
        mem_regions: regions,
        mem_lower_kib: None,
        mem_upper_kib: None,
    }
}

impl BootInfoParsed {
    fn empty() -> Self {
        BootInfoParsed {
            cmdline: None,
            boot_loader_name: None,
            mem_regions: Vec::new(),
            mem_lower_kib: None,
            mem_upper_kib: None,
        }
    }
}

/// Default identity-mapped physical memory range used when no Multiboot2 info
/// is available (e.g. QEMU `-kernel`). Covers 1 MiB .. 4 GiB in 1 GiB chunks.
pub fn default_memory_regions() -> Vec<MemRegion> {
    let mut regions: Vec<MemRegion> = Vec::new();
    let mut addr: u64 = 0x100000;
    while addr < 0x1_0000_0000 {
        let len = 0x4000_0000u64.min(0x1_0000_0000u64.wrapping_sub(addr));
        regions.push(MemRegion { addr, len, r#type: MULTIBOOT_MEMORY_AVAILABLE });
        addr = addr.wrapping_add(len);
    }
    regions
}

pub fn boot_memory_regions() -> Vec<MemRegion> {
    let info = parse_boot_info();
    if info.mem_regions.is_empty() {
        default_memory_regions()
    } else {
        info.mem_regions
    }
}

// Xen PVH ELF note so QEMU's `-kernel` loader accepts this ELF image as a
// bootable kernel (QEMU 11+ requires a PVH note for uncompressed ELF kernels).
// QEMU looks for a PT_NOTE entry of type XEN_ELFNOTE_PHYS32_ENTRY (18) whose
// descriptor is the kernel entry point address.
#[repr(C, align(4))]
struct XenNoteHeader {
    namesz: u32,
    descsz: u32,
    type_: u32,
}

const XEN_ELFNOTE_PHYS32_ENTRY: u32 = 18;

/// Physical entry point of the kernel. `_start` is placed at 0x100000 (the
/// start of `.text.start`) by the linker script, so the PVH note references
/// that address directly.
const KERNEL_ENTRY: u64 = 0x100000;

#[link_section = ".note.xen"]
#[used]
static XEN_PVH_NOTE: (XenNoteHeader, [u8; 4], [u8; 8]) = (
    XenNoteHeader { namesz: 4, descsz: 8, type_: XEN_ELFNOTE_PHYS32_ENTRY },
    [b'X', b'e', b'n', 0],
    KERNEL_ENTRY.to_le_bytes(),
);

