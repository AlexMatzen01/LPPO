//! Physical memory manager.
//!
//! Bump-style frame allocator over the Multiboot2 memory map (with a fallback
//! default map when no boot info is available), backed by a freed-frame list.

use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::arch::multiboot::{boot_memory_regions, MemRegion, MULTIBOOT_MEMORY_AVAILABLE};

const PAGE_SIZE: u64 = 4096;

#[derive(Clone, Copy)]
struct ReservedRange {
    start: u64,
    end: u64,
}

static mut REGIONS: Option<Vec<MemRegion>> = None;
static mut RESERVED: Option<Vec<ReservedRange>> = None;
static mut NEXT_SCAN: AtomicUsize = AtomicUsize::new(0);
static mut FREE_LIST: Option<Vec<u64>> = None;

pub const PAGE_SIZE_CONST: u64 = PAGE_SIZE;

pub fn init() {
    unsafe {
        REGIONS = Some(boot_memory_regions());
        RESERVED = Some(Vec::new());
        FREE_LIST = Some(Vec::new());
    }
}

/// Mark `[start, end)` as reserved so the allocator never hands it out.
pub fn reserve(start: u64, end: u64) {
    unsafe {
        if let Some(r) = RESERVED.as_mut() {
            r.push(ReservedRange { start, end });
        }
    }
}

fn in_reserved(addr: u64) -> bool {
    unsafe {
        if let Some(r) = RESERVED.as_ref() {
            for range in r.iter() {
                if addr >= range.start && addr < range.end {
                    return true;
                }
            }
        }
        false
    }
}

/// Allocate one 4 KiB physical frame. Returns the physical address or `None`
/// if memory is exhausted.
pub fn alloc_frame() -> Option<u64> {
    unsafe {
        if let Some(free) = FREE_LIST.as_mut() {
            if let Some(addr) = free.pop() {
                return Some(addr);
            }
        }
        let regions = REGIONS.as_ref()?;
        let mut scan = NEXT_SCAN.load(Ordering::Relaxed) as u64;
        for region in regions.iter() {
            if region.r#type != MULTIBOOT_MEMORY_AVAILABLE {
                continue;
            }
            let mut addr = region.addr.max(scan);
            addr = (addr + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
            let region_end = region.addr.wrapping_add(region.len);
            while addr.wrapping_add(PAGE_SIZE) <= region_end {
                if !in_reserved(addr) {
                    NEXT_SCAN.store((addr + PAGE_SIZE) as usize, Ordering::Relaxed);
                    return Some(addr);
                }
                addr = addr.wrapping_add(PAGE_SIZE);
            }
            scan = 0;
        }
        None
    }
}

/// Return a previously allocated frame to the pool.
pub fn free_frame(addr: u64) {
    unsafe {
        if let Some(free) = FREE_LIST.as_mut() {
            free.push(addr);
        }
    }
}
