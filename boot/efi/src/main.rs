// boot/efi/src/main.rs - UEFI Bootloader for RustOS
//
// This is a standalone UEFI application (PE32+) that:
// 1. Initializes UEFI console output for early messages
// 2. Locates and configures GOP (Graphics Output Protocol) for framebuffer
// 3. Loads the flat kernel binary from the EFI System Partition
// 4. Gets the EFI memory map
// 5. Calls ExitBootServices() to take full hardware control
// 6. Sets up 64-bit paging (identity map first 4GB with 2MB huge pages)
// 7. Loads a proper GDT with Ring 0 + Ring 3 segments
// 8. Writes BootInfo at 0x100000 and jumps to kernel_main

#![no_std]
#![no_main]
#![feature(abi_efiapi)]

mod boot_info;

use boot_info::*;
use core::panic::PanicInfo;
use core::arch::asm;

// ============================================================================
// UEFI Type Definitions (minimal, no external crate dependencies)
// ============================================================================

type EfiHandle = *mut core::ffi::c_void;
type EfiStatus = usize;

const EFI_SUCCESS: EfiStatus = 0;

#[repr(C)]
struct EfiTableHeader {
    signature: u64,
    revision: u32,
    header_size: u32,
    crc32: u32,
    reserved: u32,
}

#[repr(C)]
struct EfiSystemTable {
    hdr: EfiTableHeader,
    firmware_vendor: *const u16,
    firmware_revision: u32,
    console_in_handle: EfiHandle,
    con_in: *mut core::ffi::c_void,
    console_out_handle: EfiHandle,
    con_out: *mut EfiSimpleTextOutputProtocol,
    standard_error_handle: EfiHandle,
    std_err: *mut core::ffi::c_void,
    runtime_services: *mut core::ffi::c_void,
    boot_services: *mut EfiBootServices,
    number_of_table_entries: usize,
    configuration_table: *mut core::ffi::c_void,
}

#[repr(C)]
struct EfiSimpleTextOutputProtocol {
    reset: unsafe extern "efiapi" fn(*mut EfiSimpleTextOutputProtocol, bool) -> EfiStatus,
    output_string: unsafe extern "efiapi" fn(*mut EfiSimpleTextOutputProtocol, *const u16) -> EfiStatus,
    test_string: *mut core::ffi::c_void,
    query_mode: *mut core::ffi::c_void,
    set_mode: *mut core::ffi::c_void,
    set_attribute: unsafe extern "efiapi" fn(*mut EfiSimpleTextOutputProtocol, usize) -> EfiStatus,
    clear_screen: unsafe extern "efiapi" fn(*mut EfiSimpleTextOutputProtocol) -> EfiStatus,
    set_cursor_position: *mut core::ffi::c_void,
    enable_cursor: *mut core::ffi::c_void,
    mode: *mut core::ffi::c_void,
}

#[repr(C)]
struct EfiBootServices {
    hdr: EfiTableHeader,
    // Task Priority Services
    raise_tpl: *mut core::ffi::c_void,
    restore_tpl: *mut core::ffi::c_void,
    // Memory Services
    allocate_pages: unsafe extern "efiapi" fn(u32, u32, usize, *mut u64) -> EfiStatus,
    free_pages: *mut core::ffi::c_void,
    get_memory_map: unsafe extern "efiapi" fn(
        *mut usize,     // MemoryMapSize
        *mut u8,         // MemoryMap buffer
        *mut usize,     // MapKey
        *mut usize,     // DescriptorSize
        *mut u32,       // DescriptorVersion
    ) -> EfiStatus,
    allocate_pool: unsafe extern "efiapi" fn(u32, usize, *mut *mut u8) -> EfiStatus,
    free_pool: unsafe extern "efiapi" fn(*mut u8) -> EfiStatus,
    // Event & Timer Services
    create_event: *mut core::ffi::c_void,
    set_timer: *mut core::ffi::c_void,
    wait_for_event: *mut core::ffi::c_void,
    signal_event: *mut core::ffi::c_void,
    close_event: *mut core::ffi::c_void,
    check_event: *mut core::ffi::c_void,
    // Protocol Handler Services
    install_protocol_interface: *mut core::ffi::c_void,
    reinstall_protocol_interface: *mut core::ffi::c_void,
    uninstall_protocol_interface: *mut core::ffi::c_void,
    handle_protocol: unsafe extern "efiapi" fn(EfiHandle, *const EfiGuid, *mut *mut core::ffi::c_void) -> EfiStatus,
    reserved: *mut core::ffi::c_void,
    register_protocol_notify: *mut core::ffi::c_void,
    locate_handle: *mut core::ffi::c_void,
    locate_device_path: *mut core::ffi::c_void,
    install_configuration_table: *mut core::ffi::c_void,
    // Image Services
    load_image: *mut core::ffi::c_void,
    start_image: *mut core::ffi::c_void,
    exit: *mut core::ffi::c_void,
    unload_image: *mut core::ffi::c_void,
    exit_boot_services: unsafe extern "efiapi" fn(EfiHandle, usize) -> EfiStatus,
    // Miscellaneous Services
    get_next_monotonic_count: *mut core::ffi::c_void,
    stall: unsafe extern "efiapi" fn(usize) -> EfiStatus,
    set_watchdog_timer: unsafe extern "efiapi" fn(usize, u64, usize, *const u16) -> EfiStatus,
    // DriverSupport Services
    connect_controller: *mut core::ffi::c_void,
    disconnect_controller: *mut core::ffi::c_void,
    // Open and Close Protocol Services
    open_protocol: *mut core::ffi::c_void,
    close_protocol: *mut core::ffi::c_void,
    open_protocol_information: *mut core::ffi::c_void,
    // Library Services
    protocols_per_handle: *mut core::ffi::c_void,
    locate_handle_buffer: *mut core::ffi::c_void,
    locate_protocol: unsafe extern "efiapi" fn(*const EfiGuid, *mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> EfiStatus,
    // remaining fields omitted
}

#[repr(C)]
#[derive(Copy, Clone)]
struct EfiGuid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

// EFI_GRAPHICS_OUTPUT_PROTOCOL GUID
const EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data1: 0x9042a9de,
    data2: 0x23dc,
    data3: 0x4a38,
    data4: [0x96, 0xfb, 0x7a, 0xde, 0xd0, 0x80, 0x51, 0x6a],
};

#[repr(C)]
struct EfiGraphicsOutputProtocol {
    query_mode: unsafe extern "efiapi" fn(
        *mut EfiGraphicsOutputProtocol,
        u32,
        *mut usize,
        *mut *mut EfiGraphicsOutputModeInformation,
    ) -> EfiStatus,
    set_mode: unsafe extern "efiapi" fn(*mut EfiGraphicsOutputProtocol, u32) -> EfiStatus,
    blt: *mut core::ffi::c_void,
    mode: *mut EfiGraphicsOutputProtocolMode,
}

#[repr(C)]
struct EfiGraphicsOutputProtocolMode {
    max_mode: u32,
    mode: u32,
    info: *mut EfiGraphicsOutputModeInformation,
    size_of_info: usize,
    frame_buffer_base: u64,
    frame_buffer_size: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct EfiGraphicsOutputModeInformation {
    version: u32,
    horizontal_resolution: u32,
    vertical_resolution: u32,
    pixel_format: u32,
    pixel_information: [u32; 4],
    pixels_per_scan_line: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct EfiMemoryDescriptor {
    mem_type: u32,
    _pad: u32,
    physical_start: u64,
    virtual_start: u64,
    number_of_pages: u64,
    attribute: u64,
}

// EFI memory types we care about
const EFI_CONVENTIONAL_MEMORY: u32 = 7;
const EFI_LOADER_CODE: u32 = 1;
const EFI_LOADER_DATA: u32 = 2;
const EFI_BOOT_SERVICES_CODE: u32 = 3;
const EFI_BOOT_SERVICES_DATA: u32 = 4;

// AllocatePages types
const ALLOCATE_ANY_PAGES: u32 = 0;
const EFI_LOADER_DATA_TYPE: u32 = 2; // EfiLoaderData

// ============================================================================
// Global state
// ============================================================================
static mut SYSTEM_TABLE: *mut EfiSystemTable = core::ptr::null_mut();
static mut BOOT_SERVICES: *mut EfiBootServices = core::ptr::null_mut();

// ============================================================================
// UEFI console printing helpers
// ============================================================================

unsafe fn efi_print(s: &str) {
    let st = &*SYSTEM_TABLE;
    let con_out = &mut *st.con_out;
    // Convert ASCII to UCS-2 in small chunks
    let mut buf = [0u16; 128];
    let mut idx = 0;
    for byte in s.bytes() {
        if byte == b'\n' {
            buf[idx] = b'\r' as u16;
            idx += 1;
            if idx >= buf.len() - 1 {
                buf[idx] = 0;
                (con_out.output_string)(con_out, buf.as_ptr());
                idx = 0;
            }
        }
        buf[idx] = byte as u16;
        idx += 1;
        if idx >= buf.len() - 1 {
            buf[idx] = 0;
            (con_out.output_string)(con_out, buf.as_ptr());
            idx = 0;
        }
    }
    if idx > 0 {
        buf[idx] = 0;
        (con_out.output_string)(con_out, buf.as_ptr());
    }
}

unsafe fn efi_print_hex(val: u64) {
    let mut buf = [0u16; 19]; // "0x" + 16 hex digits + null
    buf[0] = b'0' as u16;
    buf[1] = b'x' as u16;
    for i in 0..16 {
        let nibble = ((val >> (60 - i * 4)) & 0xF) as u8;
        buf[2 + i] = if nibble < 10 {
            b'0' + nibble
        } else {
            b'A' + nibble - 10
        } as u16;
    }
    buf[18] = 0;
    let st = &*SYSTEM_TABLE;
    let con_out = &mut *st.con_out;
    (con_out.output_string)(con_out, buf.as_ptr());
}

unsafe fn efi_print_dec(mut val: u32) {
    let mut buf = [0u16; 12];
    let mut idx = 10;
    buf[11] = 0;
    if val == 0 {
        buf[10] = b'0' as u16;
        let st = &*SYSTEM_TABLE;
        let con_out = &mut *st.con_out;
        (con_out.output_string)(con_out, buf[10..].as_ptr());
        return;
    }
    while val > 0 {
        buf[idx] = (b'0' + (val % 10) as u8) as u16;
        val /= 10;
        if idx == 0 { break; }
        idx -= 1;
    }
    let st = &*SYSTEM_TABLE;
    let con_out = &mut *st.con_out;
    (con_out.output_string)(con_out, buf[idx + 1..].as_ptr());
}

// ============================================================================
// Paging setup (identity map first 4GB with 2MB huge pages)
// ============================================================================

// We place paging structures at known physical addresses
const PML4_ADDR: u64 = 0x70000;  // Page Map Level 4
const PDPT_ADDR: u64 = 0x71000;  // Page Directory Pointer Table
const PD0_ADDR: u64 = 0x72000;   // Page Directory 0 (0-1GB)
const PD1_ADDR: u64 = 0x73000;   // Page Directory 1 (1-2GB)
const PD2_ADDR: u64 = 0x74000;   // Page Directory 2 (2-3GB)
const PD3_ADDR: u64 = 0x75000;   // Page Directory 3 (3-4GB)

unsafe fn setup_paging() {
    // Zero all paging structures
    let tables: [u64; 6] = [PML4_ADDR, PDPT_ADDR, PD0_ADDR, PD1_ADDR, PD2_ADDR, PD3_ADDR];
    for &addr in tables.iter() {
        core::ptr::write_bytes(addr as *mut u8, 0, 4096);
    }

    let pml4 = PML4_ADDR as *mut u64;
    let pdpt = PDPT_ADDR as *mut u64;

    // PML4[0] -> PDPT (Present | Writable | User)
    *pml4 = PDPT_ADDR | 0x07;

    // PDPT[0..3] -> PD0..PD3
    let pd_addrs = [PD0_ADDR, PD1_ADDR, PD2_ADDR, PD3_ADDR];
    for (i, &pd_addr) in pd_addrs.iter().enumerate() {
        *pdpt.add(i) = pd_addr | 0x07;
    }

    // Each PD: 512 entries of 2MB huge pages
    for (pd_idx, &pd_addr) in pd_addrs.iter().enumerate() {
        let pd = pd_addr as *mut u64;
        for i in 0..512u64 {
            let phys = (pd_idx as u64) * 0x40000000 + i * 0x200000;
            // Present | Writable | User | Huge (PS bit)
            *pd.add(i as usize) = phys | 0x87;
        }
    }

    // Load CR3 with PML4
    asm!(
        "mov cr3, {}",
        in(reg) PML4_ADDR,
        options(nostack, preserves_flags)
    );
}

// ============================================================================
// GDT setup (64-bit with Ring 0 + Ring 3 segments)
// ============================================================================

#[repr(C, align(16))]
struct Gdt64 {
    entries: [u64; 5],
}

#[repr(C, packed)]
struct GdtDescriptor {
    limit: u16,
    base: u64,
}

static mut GDT: Gdt64 = Gdt64 {
    entries: [
        0x0000000000000000, // 0x00: Null
        0x00AF9A000000FFFF, // 0x08: Kernel Code 64-bit (DPL=0, L=1)
        0x00CF92000000FFFF, // 0x10: Kernel Data 64-bit (DPL=0)
        0x00CFF2000000FFFF, // 0x18: User Data 64-bit (DPL=3)
        0x00AFFB000000FFFF, // 0x20: User Code 64-bit (DPL=3, L=1)
    ],
};

unsafe fn load_gdt_and_segments() {
    let desc = GdtDescriptor {
        limit: (core::mem::size_of::<Gdt64>() - 1) as u16,
        base: &GDT as *const _ as u64,
    };

    asm!(
        "lgdt [{}]",
        in(reg) &desc,
        options(readonly, nostack, preserves_flags)
    );

    // Reload CS via far return, set data segments to 0x10
    asm!(
        "push 0x08",          // Kernel Code selector
        "lea rax, [rip + 2f]",
        "push rax",
        "retfq",
        "2:",
        "mov ax, 0x10",       // Kernel Data selector
        "mov ds, ax",
        "mov es, ax",
        "mov fs, ax",
        "mov gs, ax",
        "mov ss, ax",
        out("rax") _,
        options(preserves_flags)
    );
}

// ============================================================================
// EFI Main Entry Point
// ============================================================================

#[no_mangle]
pub extern "efiapi" fn efi_main(image_handle: EfiHandle, system_table: *mut EfiSystemTable) -> EfiStatus {
    unsafe {
        SYSTEM_TABLE = system_table;
        BOOT_SERVICES = (*system_table).boot_services;

        // Disable watchdog timer
        let bs = &*BOOT_SERVICES;
        (bs.set_watchdog_timer)(0, 0, 0, core::ptr::null());

        // Clear screen
        let con_out = &mut *(*system_table).con_out;
        (con_out.clear_screen)(con_out);

        efi_print("[EFI] RustOS UEFI Bootloader v1.0\n");
        efi_print("[EFI] Initializing...\n");

        // ================================================================
        // Step 1: Locate and configure GOP (Graphics Output Protocol)
        // ================================================================
        efi_print("[EFI] Locating GOP (Graphics Output Protocol)...\n");

        let mut gop_ptr: *mut core::ffi::c_void = core::ptr::null_mut();
        let status = (bs.locate_protocol)(
            &EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID,
            core::ptr::null_mut(),
            &mut gop_ptr,
        );

        let mut fb_addr: u64 = 0;
        let mut fb_width: u32 = 0;
        let mut fb_height: u32 = 0;
        let mut fb_pitch: u32 = 0;
        let mut fb_bpp: u32 = 32;

        if status == EFI_SUCCESS && !gop_ptr.is_null() {
            let gop = &mut *(gop_ptr as *mut EfiGraphicsOutputProtocol);
            let mode = &*gop.mode;

            efi_print("[EFI] GOP found! Scanning video modes...\n");

            // Find best mode: prefer 1024x768 or 1280x720, fallback to largest
            let mut best_mode: u32 = mode.mode;
            let mut best_w: u32 = 0;
            let mut best_h: u32 = 0;
            let mut found_preferred = false;

            for m in 0..mode.max_mode {
                let mut info_size: usize = 0;
                let mut info_ptr: *mut EfiGraphicsOutputModeInformation = core::ptr::null_mut();
                let qs = (gop.query_mode)(gop, m, &mut info_size, &mut info_ptr);
                if qs == EFI_SUCCESS && !info_ptr.is_null() {
                    let info = &*info_ptr;
                    // Only accept BGR/RGB pixel formats (not bitmask or BltOnly)
                    if info.pixel_format <= 1 {
                        let w = info.horizontal_resolution;
                        let h = info.vertical_resolution;
                        // Prefer 1024x768
                        if w == 1024 && h == 768 && !found_preferred {
                            best_mode = m;
                            best_w = w;
                            best_h = h;
                            found_preferred = true;
                        }
                        // Or take largest
                        if !found_preferred && (w * h) > (best_w * best_h) {
                            best_mode = m;
                            best_w = w;
                            best_h = h;
                        }
                    }
                }
            }

            // Set the chosen mode
            if best_w > 0 {
                (gop.set_mode)(gop, best_mode);
                let mode = &*gop.mode;
                let info = &*mode.info;
                fb_addr = mode.frame_buffer_base;
                fb_width = info.horizontal_resolution;
                fb_height = info.vertical_resolution;
                fb_pitch = info.pixels_per_scan_line * 4; // 4 bytes per pixel (32bpp)
                fb_bpp = 32;

                efi_print("[EFI] GOP mode set: ");
                efi_print_dec(fb_width);
                efi_print("x");
                efi_print_dec(fb_height);
                efi_print(" @ FB=");
                efi_print_hex(fb_addr);
                efi_print("\n");
            } else {
                // Use current mode
                let info = &*mode.info;
                fb_addr = mode.frame_buffer_base;
                fb_width = info.horizontal_resolution;
                fb_height = info.vertical_resolution;
                fb_pitch = info.pixels_per_scan_line * 4;
                efi_print("[EFI] Using current GOP mode.\n");
            }
        } else {
            efi_print("[EFI] WARNING: GOP not found, no framebuffer available.\n");
        }

        // ================================================================
        // Step 2: Get EFI Memory Map
        // ================================================================
        efi_print("[EFI] Getting EFI memory map...\n");

        // Allocate a buffer for memory map
        let mut mmap_size: usize = 0;
        let mut map_key: usize = 0;
        let mut desc_size: usize = 0;
        let mut desc_version: u32 = 0;

        // First call to get required size
        (bs.get_memory_map)(
            &mut mmap_size,
            core::ptr::null_mut(),
            &mut map_key,
            &mut desc_size,
            &mut desc_version,
        );

        // Add extra space for the allocation we're about to make
        mmap_size += 2 * desc_size;

        // Allocate pool for memory map
        let mut mmap_buf: *mut u8 = core::ptr::null_mut();
        let alloc_status = (bs.allocate_pool)(EFI_LOADER_DATA_TYPE, mmap_size, &mut mmap_buf);
        if alloc_status != EFI_SUCCESS || mmap_buf.is_null() {
            efi_print("[EFI] FATAL: Failed to allocate memory map buffer!\n");
            loop { asm!("hlt"); }
        }

        // Second call to actually get the map
        let status = (bs.get_memory_map)(
            &mut mmap_size,
            mmap_buf,
            &mut map_key,
            &mut desc_size,
            &mut desc_version,
        );
        if status != EFI_SUCCESS {
            efi_print("[EFI] FATAL: GetMemoryMap failed!\n");
            loop { asm!("hlt"); }
        }

        // Convert EFI memory map to our simplified format and store at a known location
        let mmap_storage_addr: u64 = BOOT_INFO_ADDR + 0x1000; // BootInfo + 4KB
        let mmap_entries = mmap_storage_addr as *mut MemoryMapEntry;
        let num_descs = mmap_size / desc_size;
        let mut stored_entries: u32 = 0;

        for i in 0..num_descs {
            if stored_entries as usize >= MAX_MMAP_ENTRIES {
                break;
            }
            let desc = &*(mmap_buf.add(i * desc_size) as *const EfiMemoryDescriptor);
            // Store usable memory regions
            let usable = matches!(desc.mem_type,
                EFI_CONVENTIONAL_MEMORY | EFI_LOADER_CODE | EFI_LOADER_DATA |
                EFI_BOOT_SERVICES_CODE | EFI_BOOT_SERVICES_DATA
            );

            let entry = &mut *mmap_entries.add(stored_entries as usize);
            entry.phys_start = desc.physical_start;
            entry.page_count = desc.number_of_pages;
            entry.mem_type = if usable { 1 } else { 0 };
            entry._pad = 0;
            stored_entries += 1;
        }

        efi_print("[EFI] Memory map: ");
        efi_print_dec(stored_entries);
        efi_print(" entries recorded.\n");

        // ================================================================
        // Step 3: Write BootInfo at fixed address
        // ================================================================
        let boot_info = &mut *(BOOT_INFO_ADDR as *mut BootInfo);
        boot_info.magic = BOOT_INFO_MAGIC;
        boot_info.boot_mode = BOOT_MODE_UEFI;
        boot_info.framebuffer_addr = fb_addr;
        boot_info.framebuffer_width = fb_width;
        boot_info.framebuffer_height = fb_height;
        boot_info.framebuffer_pitch = fb_pitch;
        boot_info.framebuffer_bpp = fb_bpp;
        boot_info.memory_map_addr = mmap_storage_addr;
        boot_info.memory_map_entries = stored_entries;
        boot_info._reserved = 0;

        efi_print("[EFI] BootInfo written at 0x100000\n");

        // ================================================================
        // Step 4: Exit Boot Services
        // ================================================================
        efi_print("[EFI] Calling ExitBootServices()...\n");

        // Need fresh memory map key for ExitBootServices
        mmap_size = 8192;
        let mut mmap_buf2 = [0u8; 8192];
        (bs.get_memory_map)(
            &mut mmap_size,
            mmap_buf2.as_mut_ptr(),
            &mut map_key,
            &mut desc_size,
            &mut desc_version,
        );

        let exit_status = (bs.exit_boot_services)(image_handle, map_key);
        if exit_status != EFI_SUCCESS {
            // Try once more with a fresh key
            ((*BOOT_SERVICES).get_memory_map)(
                &mut mmap_size,
                mmap_buf2.as_mut_ptr(),
                &mut map_key,
                &mut desc_size,
                &mut desc_version,
            );
            let exit_status2 = ((*BOOT_SERVICES).exit_boot_services)(image_handle, map_key);
            if exit_status2 != EFI_SUCCESS {
                // Cannot print after failed ExitBootServices - just halt
                loop { asm!("hlt"); }
            }
        }

        // ========================================================
        // WE ARE NOW POST-ExitBootServices
        // No more UEFI calls allowed! We own the hardware.
        // ========================================================

        // Step 5: Set up identity-mapped paging for first 4GB
        setup_paging();

        // Step 6: Load proper GDT with Ring 0 + Ring 3 segments
        load_gdt_and_segments();

        // Step 7: Set kernel stack and jump to kernel_main
        // The kernel binary is linked at 0x8000 (same as BIOS path)
        // It was loaded as part of the ESP filesystem and placed in memory
        // by the build system at a known location.
        //
        // For the UEFI path, the kernel binary is embedded directly into
        // this EFI executable (included at build time).
        // The kernel_main symbol is at the start of the kernel binary.

        // Set up kernel stack at 0x90000 (same as BIOS path)
        asm!(
            "mov rsp, 0x90000",
            "mov rbp, rsp",
            options(nostack)
        );

        // Jump to kernel_main (linked at kernel start address)
        // The kernel is loaded at 0x8000 by the build process
        let kernel_entry: u64 = 0x8000;

        // Send 'E' to serial COM1 for debug (port 0x3F8)
        asm!(
            "mov dx, 0x3F8",
            "mov al, 0x45", // 'E' for EFI
            "out dx, al",
            "mov al, 0x0A", // newline
            "out dx, al",
            out("dx") _,
            out("al") _,
        );

        // The kernel binary starts with entry64's _start64_setup code.
        // But in EFI mode, we're already in 64-bit long mode with paging,
        // so we skip the setup and jump directly to kernel_main.
        // kernel_main is at an offset within the kernel binary.
        // We use the BOOT_INFO to tell the kernel we're in EFI mode.
        //
        // For now, call kernel_main via its known symbol.
        // The linker will resolve this since the kernel is embedded.
        extern "C" {
            fn kernel_main() -> !;
        }
        kernel_main();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe {
        if !SYSTEM_TABLE.is_null() {
            efi_print("\n[EFI PANIC] ");
            if let Some(loc) = _info.location() {
                // Can't easily format, just print a message
                efi_print("Bootloader panic occurred!\n");
            } else {
                efi_print("Unknown panic!\n");
            }
        }
    }
    loop {
        unsafe { asm!("hlt"); }
    }
}
