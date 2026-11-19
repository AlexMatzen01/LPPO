//! Virtual memory and paging (x86_64, 4-level page tables, 2 MiB huge pages).
//!
//! Identity-maps the lower 4 GiB (virtual == physical).

use crate::arch::pmm::alloc_frame;

const HUGE_PAGE_SIZE: u64 = 0x200000; // 2 MiB
const ENTRY_COUNT: usize = 512;
const PRESENT: u64 = 1 << 0;
const WRITABLE: u64 = 1 << 1;
const HUGE: u64 = 1 << 7; // PS bit

#[repr(C, align(4096))]
struct PageTable {
    entries: [u64; ENTRY_COUNT],
}

impl PageTable {
    const fn zeroed() -> Self {
        PageTable { entries: [0; ENTRY_COUNT] }
    }
}

static mut PML4_ADDR: u64 = 0;

pub fn init_tables() {
    unsafe {
        let pml4 = alloc_frame().expect("paging: no frame for PML4") as *mut PageTable;
        core::ptr::write(pml4, PageTable::zeroed());
        PML4_ADDR = pml4 as u64;

        let pdpt = alloc_frame().expect("paging: no frame for PDPT") as *mut PageTable;
        core::ptr::write(pdpt, PageTable::zeroed());
        (*pml4).entries[0] = pdpt as u64 | PRESENT | WRITABLE;

        core::arch::asm!("mov cr3, {0}", in(reg) pml4 as u64, options(nostack));

        // Identity-map the lower 4 GiB with 2 MiB huge pages.
        for gb in 0..4usize {
            let pd = alloc_frame().expect("paging: no frame for PD") as *mut PageTable;
            core::ptr::write(pd, PageTable::zeroed());
            (*pdpt).entries[gb] = pd as u64 | PRESENT | WRITABLE;
            for i in 0..ENTRY_COUNT {
                let vaddr = (gb as u64) * 0x4000_0000 + (i as u64) * HUGE_PAGE_SIZE;
                (*pd).entries[i] = vaddr | PRESENT | WRITABLE | HUGE;
            }
        }
    }
}

/// Map a 2 MiB huge page: `vaddr` -> `paddr` (both must be 2 MiB aligned).
/// Returns an error if the addresses are misaligned or outside the identity
/// range covered by the bootstrap page tables.
pub fn map_page(vaddr: u64, paddr: u64) -> Result<(), &'static str> {
    if vaddr % HUGE_PAGE_SIZE != 0 || paddr % HUGE_PAGE_SIZE != 0 {
        return Err("map_page: addresses must be 2 MiB aligned");
    }
    if vaddr >= 0x1_0000_0000 {
        return Err("map_page: vaddr outside identity-mapped range");
    }
    unsafe {
        let pml4 = PML4_ADDR as *mut PageTable;
        let pdpt_addr = ((*pml4).entries[0] & !0xFFF) as *mut PageTable;
        let gb = (vaddr / 0x4000_0000) as usize;
        let pd_addr = ((*pdpt_addr).entries[gb] & !0xFFF) as *mut PageTable;
        let i = ((vaddr % 0x4000_0000) / HUGE_PAGE_SIZE) as usize;
        (*pd_addr).entries[i] = paddr | PRESENT | WRITABLE | HUGE;
    }
    Ok(())
}

pub fn enable_pae_lme() {
    unsafe {
        let mut cr4: u64;
        core::arch::asm!("mov {0}, cr4", out(reg) cr4, options(nostack));
        cr4 |= 1 << 5; // PAE
        core::arch::asm!("mov cr4, {0}", in(reg) cr4, options(nostack));

        // IA32_EFER MSR (0xC0000080): set the LME bit (8) to enable long mode.
        let ecx: u32 = 0xC0000080;
        let mut eax: u32;
        let mut edx: u32;
        core::arch::asm!("rdmsr",
            in("ecx") ecx,
            out("eax") eax,
            out("edx") edx,
            options(nostack));
        eax |= 1 << 8;
        core::arch::asm!("wrmsr",
            in("ecx") ecx,
            in("eax") eax,
            in("edx") edx,
            options(nostack));
    }
}

pub fn enable_paging() {
    unsafe {
        let mut cr0: u64;
        core::arch::asm!("mov {0}, cr0", out(reg) cr0, options(nostack));
        cr0 &= !(1 << 16); // clear WP
        cr0 |= 1 << 31;    // PG (PE and LME already set)
        core::arch::asm!("mov cr0, {0}", in(reg) cr0, options(nostack));
    }
}
