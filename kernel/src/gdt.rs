// kernel/src/gdt.rs - 64-bit Global Descriptor Table and TSS setup
use core::mem::size_of;

#[repr(C, packed)]
pub struct TaskStateSegment {
    reserved_1: u32,
    pub rsp0: u64,
    pub rsp1: u64,
    pub rsp2: u64,
    reserved_2: u64,
    pub ist1: u64,
    pub ist2: u64,
    pub ist3: u64,
    pub ist4: u64,
    pub ist5: u64,
    pub ist6: u64,
    pub ist7: u64,
    reserved_3: u64,
    reserved_4: u16,
    pub iomap_base: u16,
}

impl TaskStateSegment {
    pub const fn new() -> Self {
        TaskStateSegment {
            reserved_1: 0,
            rsp0: 0x90000,
            rsp1: 0,
            rsp2: 0,
            reserved_2: 0,
            ist1: 0,
            ist2: 0,
            ist3: 0,
            ist4: 0,
            ist5: 0,
            ist6: 0,
            ist7: 0,
            reserved_3: 0,
            reserved_4: 0,
            iomap_base: size_of::<TaskStateSegment>() as u16,
        }
    }
}

pub static mut TSS: TaskStateSegment = TaskStateSegment::new();

#[repr(C, packed)]
struct GdtDescriptor {
    limit: u16,
    base: u64,
}

#[repr(C, align(16))]
struct Gdt {
    entries: [u64; 7],
}

static mut GDT: Gdt = Gdt {
    entries: [
        0x0000000000000000, // 0x00: Null
        0x00AF9A000000FFFF, // 0x08: Kernel Code 64-bit
        0x00CF92000000FFFF, // 0x10: Kernel Data 64-bit
        0x00CFF2000000FFFF, // 0x18: User Data 64-bit (RPL 3)
        0x00AFFB000000FFFF, // 0x20: User Code 64-bit (RPL 3)
        0x0000000000000000, // 0x28: TSS Low (filled at runtime)
        0x0000000000000000, // 0x30: TSS High
    ],
};

pub fn init() {
    unsafe {
        TSS.rsp0 = 0x1FF000;
        let tss_addr = &raw const TSS as *const _ as u64;
        let tss_size = (size_of::<TaskStateSegment>() - 1) as u64;

        // TSS descriptor in Long Mode occupies 16 bytes (2 entries)
        let tss_low = (tss_size & 0xFFFF)
            | ((tss_addr & 0x00FFFFFF) << 16)
            | (0x89 << 40) // Present, DPL=0, Type=9 (Available 64-bit TSS)
            | ((tss_size & 0xF0000) << 32)
            | ((tss_addr & 0xFF000000) << 32);
        let tss_high = tss_addr >> 32;

        GDT.entries[5] = tss_low;
        GDT.entries[6] = tss_high;

        let descriptor = GdtDescriptor {
            limit: (size_of::<Gdt>() - 1) as u16,
            base: &GDT as *const _ as u64,
        };

        core::arch::asm!(
            "lgdt [{0}]",
            in(reg) &descriptor,
            options(readonly, nostack, preserves_flags)
        );

        // Load Task Register
        core::arch::asm!(
            "ltr {0:x}",
            in(reg) 0x28u16,
            options(nomem, nostack, preserves_flags)
        );
    }
}
