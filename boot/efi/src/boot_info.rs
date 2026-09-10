// boot/efi/src/boot_info.rs - Shared boot information protocol between bootloader and kernel
//
// The UEFI bootloader writes this struct to a fixed physical address (BOOT_INFO_ADDR)
// so the kernel can detect boot mode and access framebuffer/memory map info.

/// Fixed physical address where BootInfo is placed
pub const BOOT_INFO_ADDR: u64 = 0x100000; // 1 MB mark

/// Magic value identifying a valid BootInfo ("EFIB")
pub const BOOT_INFO_MAGIC: u32 = 0x45464942;

pub const BOOT_MODE_BIOS: u32 = 0;
pub const BOOT_MODE_UEFI: u32 = 1;

/// Maximum memory map entries we store
pub const MAX_MMAP_ENTRIES: usize = 128;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct BootInfo {
    /// Magic number (BOOT_INFO_MAGIC) to validate presence
    pub magic: u32,
    /// Boot mode: 0 = Legacy BIOS, 1 = UEFI
    pub boot_mode: u32,
    /// Linear framebuffer physical address (from UEFI GOP)
    pub framebuffer_addr: u64,
    /// Framebuffer width in pixels
    pub framebuffer_width: u32,
    /// Framebuffer height in pixels
    pub framebuffer_height: u32,
    /// Framebuffer pitch (bytes per scanline)
    pub framebuffer_pitch: u32,
    /// Bits per pixel (typically 32)
    pub framebuffer_bpp: u32,
    /// Physical address of memory map array
    pub memory_map_addr: u64,
    /// Number of memory map entries
    pub memory_map_entries: u32,
    /// Padding for alignment
    pub _reserved: u32,
}

/// Simplified memory map entry (derived from EFI memory descriptors)
#[repr(C)]
#[derive(Copy, Clone)]
pub struct MemoryMapEntry {
    /// Physical start address
    pub phys_start: u64,
    /// Number of 4KB pages
    pub page_count: u64,
    /// Memory type (0 = Reserved, 1 = Usable/Conventional, 7 = Boot Services)
    pub mem_type: u32,
    pub _pad: u32,
}
