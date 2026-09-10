// kernel/src/boot_info.rs - Kernel-side boot information reader
//
// Reads the BootInfo struct placed at 0x100000 by the UEFI bootloader.
// For legacy BIOS boot, the magic won't match and we fall back to BIOS mode.

/// Fixed physical address where BootInfo is placed
pub const BOOT_INFO_ADDR: u64 = 0x100000;

/// Magic value identifying a valid BootInfo ("EFIB")
pub const BOOT_INFO_MAGIC: u32 = 0x45464942;

pub const BOOT_MODE_BIOS: u32 = 0;
pub const BOOT_MODE_UEFI: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct BootInfo {
    pub magic: u32,
    pub boot_mode: u32,
    pub framebuffer_addr: u64,
    pub framebuffer_width: u32,
    pub framebuffer_height: u32,
    pub framebuffer_pitch: u32,
    pub framebuffer_bpp: u32,
    pub memory_map_addr: u64,
    pub memory_map_entries: u32,
    pub _reserved: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MemoryMapEntry {
    pub phys_start: u64,
    pub page_count: u64,
    pub mem_type: u32,
    pub _pad: u32,
}

/// Global boot mode flag
pub static mut BOOT_MODE: u32 = BOOT_MODE_BIOS;

/// Cached boot info (valid only if BOOT_MODE == UEFI)
pub static mut CACHED_BOOT_INFO: BootInfo = BootInfo {
    magic: 0,
    boot_mode: 0,
    framebuffer_addr: 0,
    framebuffer_width: 0,
    framebuffer_height: 0,
    framebuffer_pitch: 0,
    framebuffer_bpp: 0,
    memory_map_addr: 0,
    memory_map_entries: 0,
    _reserved: 0,
};

/// Detect boot mode by reading BootInfo at the known physical address.
/// Must be called very early in kernel_main before any other initialization.
pub fn detect_boot_mode() {
    unsafe {
        let info_ptr = BOOT_INFO_ADDR as *const BootInfo;
        let info = &*info_ptr;
        if info.magic == BOOT_INFO_MAGIC && info.boot_mode == BOOT_MODE_UEFI {
            BOOT_MODE = BOOT_MODE_UEFI;
            // Copy the boot info so we don't rely on that memory region later
            CACHED_BOOT_INFO = *info;
        } else {
            BOOT_MODE = BOOT_MODE_BIOS;
        }
    }
}

/// Returns true if booted via UEFI
pub fn is_uefi() -> bool {
    unsafe { BOOT_MODE == BOOT_MODE_UEFI }
}

/// Returns true if booted via legacy BIOS
pub fn is_bios() -> bool {
    unsafe { BOOT_MODE == BOOT_MODE_BIOS }
}

/// Get the cached boot info (only valid after detect_boot_mode() is called)
pub fn get_boot_info() -> &'static BootInfo {
    unsafe { &CACHED_BOOT_INFO }
}
