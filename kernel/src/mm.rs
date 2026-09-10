// kernel/src/mm.rs - Physical Frame Allocator and Paging Manager
use core::arch::asm;

pub const PAGE_SIZE: usize = 4096;
const HEAP_START: usize = 0x200000; // 2MB start for dynamic allocations
const HEAP_END: usize = 0x4000000;  // 64MB frame pool

static mut NEXT_FREE_FRAME: usize = HEAP_START;

pub struct FrameAllocator;

impl FrameAllocator {
    pub fn allocate_frame() -> usize {
        unsafe {
            if NEXT_FREE_FRAME >= HEAP_END {
                panic!("Out of physical memory frames!");
            }
            let frame = NEXT_FREE_FRAME;
            NEXT_FREE_FRAME += PAGE_SIZE;

            // Zero out allocated physical frame
            let ptr = frame as *mut u8;
            core::ptr::write_bytes(ptr, 0, PAGE_SIZE);

            frame
        }
    }
}

// Page Table Entry flags
pub const PAGE_PRESENT: u64 = 1 << 0;
pub const PAGE_WRITABLE: u64 = 1 << 1;
pub const PAGE_USER: u64 = 1 << 2;

pub struct PageTableManager {
    pml4_addr: u64,
}

impl PageTableManager {
    pub fn from_cr3() -> Self {
        let pml4: u64;
        unsafe {
            asm!("mov {}, cr3", out(reg) pml4, options(nomem, nostack));
        }
        PageTableManager {
            pml4_addr: pml4 & 0xFFFFFFFFFFFFF000,
        }
    }

    pub fn map_page(&mut self, virt: u64, phys: u64, flags: u64) {
        let pml4_idx = ((virt >> 39) & 0x1FF) as usize;
        let pdpt_idx = ((virt >> 30) & 0x1FF) as usize;
        let pd_idx   = ((virt >> 21) & 0x1FF) as usize;
        let pt_idx   = ((virt >> 12) & 0x1FF) as usize;

        let pml4 = self.pml4_addr as *mut u64;

        // PML4 -> PDPT
        let pdpt_addr = unsafe {
            let entry = *pml4.add(pml4_idx);
            if entry & PAGE_PRESENT == 0 {
                let frame = FrameAllocator::allocate_frame() as u64;
                *pml4.add(pml4_idx) = frame | PAGE_PRESENT | PAGE_WRITABLE | PAGE_USER;
                frame
            } else {
                // Ensure user flag is set up the chain if mapping user page
                if flags & PAGE_USER != 0 {
                    *pml4.add(pml4_idx) |= PAGE_USER | PAGE_WRITABLE;
                }
                entry & 0xFFFFFFFFFFFFF000
            }
        };

        let pdpt = pdpt_addr as *mut u64;

        // PDPT -> PD
        let pd_addr = unsafe {
            let entry = *pdpt.add(pdpt_idx);
            if entry & PAGE_PRESENT == 0 {
                let frame = FrameAllocator::allocate_frame() as u64;
                *pdpt.add(pdpt_idx) = frame | PAGE_PRESENT | PAGE_WRITABLE | PAGE_USER;
                frame
            } else {
                if flags & PAGE_USER != 0 {
                    *pdpt.add(pdpt_idx) |= PAGE_USER | PAGE_WRITABLE;
                }
                entry & 0xFFFFFFFFFFFFF000
            }
        };

        let pd = pd_addr as *mut u64;

        // PD -> PT
        let pt_addr = unsafe {
            let entry = *pd.add(pd_idx);
            if entry & PAGE_PRESENT == 0 {
                let frame = FrameAllocator::allocate_frame() as u64;
                *pd.add(pd_idx) = frame | PAGE_PRESENT | PAGE_WRITABLE | PAGE_USER;
                frame
            } else {
                if flags & PAGE_USER != 0 {
                    *pd.add(pd_idx) |= PAGE_USER | PAGE_WRITABLE;
                }
                entry & 0xFFFFFFFFFFFFF000
            }
        };

        let pt = pt_addr as *mut u64;

        unsafe {
            *pt.add(pt_idx) = phys | flags | PAGE_PRESENT;
            // Invalidate TLB for this virtual address
            asm!("invlpg [{0}]", in(reg) virt, options(nostack, preserves_flags));
        }
    }
}
