// kernel/src/elf.rs - ELF64 Loader for Linux x86_64 Executables
use crate::mm::{FrameAllocator, PageTableManager, PAGE_PRESENT, PAGE_USER, PAGE_WRITABLE, PAGE_SIZE};

const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const PT_LOAD: u32 = 1;

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct Elf64Header {
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct Elf64Phdr {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

pub struct LoadedElf {
    pub entry_point: u64,
    pub user_sp: u64,
    pub brk_base: u64,
}

pub fn load_elf(elf_data: &[u8]) -> Result<LoadedElf, &'static str> {
    if elf_data.len() < core::mem::size_of::<Elf64Header>() {
        return Err("ELF image too small");
    }

    let header = unsafe { &*(elf_data.as_ptr() as *const Elf64Header) };

    if header.e_ident[0..4] != ELF_MAGIC {
        return Err("Invalid ELF magic");
    }
    if header.e_ident[4] != ELFCLASS64 {
        return Err("Not a 64-bit ELF");
    }
    if header.e_ident[5] != ELFDATA2LSB {
        return Err("Not a little-endian ELF");
    }
    if header.e_machine != 0x3E {
        // x86_64
        return Err("Not x86_64 architecture");
    }

    let mut pt_manager = PageTableManager::from_cr3();
    let phdr_offset = header.e_phoff as usize;
    let phdr_size = header.e_phentsize as usize;
    let phdr_count = header.e_phnum as usize;

    let mut max_vaddr: u64 = 0x400000;

    for i in 0..phdr_count {
        let cur_offset = phdr_offset + i * phdr_size;
        if cur_offset + phdr_size > elf_data.len() {
            return Err("ELF program header out of bounds");
        }
        let phdr = unsafe { &*(elf_data.as_ptr().add(cur_offset) as *const Elf64Phdr) };

        if phdr.p_type == PT_LOAD {
            let vaddr_start = phdr.p_vaddr;
            let memsz = phdr.p_memsz;
            let filesz = phdr.p_filesz;
            let offset = phdr.p_offset as usize;

            let page_start = vaddr_start & !0xFFF;
            let page_end = (vaddr_start + memsz + 0xFFF) & !0xFFF;
            let num_pages = ((page_end - page_start) / 4096) as usize;

            if vaddr_start + memsz > max_vaddr {
                max_vaddr = vaddr_start + memsz;
            }

            for p in 0..num_pages {
                let page_virt = page_start + (p * 4096) as u64;
                let page_phys = FrameAllocator::allocate_frame() as u64;
                pt_manager.map_page(
                    page_virt,
                    page_phys,
                    PAGE_USER | PAGE_WRITABLE | PAGE_PRESENT,
                );
            }

            // Copy file content to virtual address
            unsafe {
                let dest = vaddr_start as *mut u8;
                let src = elf_data.as_ptr().add(offset);
                core::ptr::copy_nonoverlapping(src, dest, filesz as usize);

                // Zero out .bss if memsz > filesz
                if memsz > filesz {
                    let bss_start = dest.add(filesz as usize);
                    let bss_len = (memsz - filesz) as usize;
                    core::ptr::write_bytes(bss_start, 0, bss_len);
                }
            }
        }
    }

    // Allocate User Stack at 0x00007FFF00000000 (standard high address)
    // 4 pages (16 KB stack)
    let user_stack_base: u64 = 0x7FFF0000;
    let stack_pages = 4;
    for p in 0..stack_pages {
        let vaddr = user_stack_base + (p * 4096) as u64;
        let phys = FrameAllocator::allocate_frame() as u64;
        pt_manager.map_page(vaddr, phys, PAGE_USER | PAGE_WRITABLE | PAGE_PRESENT);
    }
    let mut user_sp: u64 = user_stack_base + (stack_pages * 4096) as u64 - 16;

    // Set up Linux ABI Initial Stack:
    // [sp] = argc = 1
    // [sp+8] = argv[0] pointer
    // [sp+16] = NULL
    // [sp+24] = NULL (envp empty)
    // [sp+32] = auxv AT_NULL (0, 0)
    unsafe {
        let sp_ptr = user_sp as *mut u64;
        *sp_ptr.add(0) = 1; // argc
        *sp_ptr.add(1) = 0; // argv[0] (null string)
        *sp_ptr.add(2) = 0; // argv terminator
        *sp_ptr.add(3) = 0; // envp terminator
        *sp_ptr.add(4) = 0; // AT_NULL
        *sp_ptr.add(5) = 0;
    }

    Ok(LoadedElf {
        entry_point: header.e_entry,
        user_sp,
        brk_base: (max_vaddr + 0xFFF) & !0xFFF,
    })
}
