// kernel/src/idt.rs - Interrupt Descriptor Table for CPU Exceptions and Interrupts
// Brutally honest CPU fault reporters that will ruin your day
use core::arch::asm;
use core::mem::size_of;

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    zero: u32,
}

impl IdtEntry {
    pub const fn missing() -> Self {
        IdtEntry {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            zero: 0,
        }
    }

    pub fn set_handler(&mut self, handler: u64, dpl: u8) {
        self.offset_low = (handler & 0xFFFF) as u16;
        self.selector = 0x08; // Kernel Code Segment
        self.ist = 0;
        self.type_attr = 0x8E | (dpl << 5); // Present, 64-bit Interrupt Gate
        self.offset_mid = ((handler >> 16) & 0xFFFF) as u16;
        self.offset_high = ((handler >> 32) & 0xFFFFFFFF) as u32;
        self.zero = 0;
    }
}

#[repr(C, packed)]
struct IdtDescriptor {
    limit: u16,
    base: u64,
}

static mut IDT: [IdtEntry; 256] = [IdtEntry::missing(); 256];

extern "C" {
    fn default_exception_handler();
    fn page_fault_handler();
    fn gpf_handler();
}

pub fn init() {
    unsafe {
        for i in 0..32 {
            IDT[i].set_handler(default_exception_handler as *const () as u64, 0);
        }
        IDT[13].set_handler(gpf_handler as *const () as u64, 0); // GPF
        IDT[14].set_handler(page_fault_handler as *const () as u64, 0); // Page Fault

        let descriptor = IdtDescriptor {
            limit: (size_of::<[IdtEntry; 256]>() - 1) as u16,
            base: (&raw const IDT) as *const _ as u64,
        };

        asm!("lidt [{0}]", in(reg) &descriptor, options(readonly, nostack, preserves_flags));
    }
}

#[no_mangle]
pub extern "C" fn rust_page_fault(fault_addr: u64, error_code: u64, rip: u64) {
    unsafe { asm!("cli"); }
    crate::println!("\n========================================================");
    crate::println!("  [CRITICAL DISASTER] #PF Page Fault Exception (#14)    ");
    crate::println!("========================================================");
    crate::println!(" Fault Address: 0x{:016x} (You touched unmapped void)", fault_addr);
    crate::println!(" Faulting RIP : 0x{:016x} (Look at your bad code here)", rip);
    crate::println!(" Error Code   : 0x{:x} (P={}", error_code, error_code & 1);
    crate::println!(" Verdict      : You dereferenced a pointer that hates you.");
    crate::println!("                Rust's borrow checker couldn't save your bare-metal sins.");
    crate::println!("========================================================");
    loop {
        unsafe { asm!("hlt") };
    }
}

#[no_mangle]
pub extern "C" fn rust_general_protection(error_code: u64, rip: u64) {
    unsafe { asm!("cli"); }
    crate::println!("\n========================================================");
    crate::println!("  [CRITICAL DISASTER] #GP General Protection Fault (#13)");
    crate::println!("========================================================");
    crate::println!(" Faulting RIP : 0x{:016x}", rip);
    crate::println!(" Error Code   : 0x{:x}", error_code);
    crate::println!(" Verdict      : You tried to violate CPU segmentation/privilege rules.");
    crate::println!("                The MMU laughed in your face and pulled the plug.");
    crate::println!("========================================================");
    loop {
        unsafe { asm!("hlt") };
    }
}

#[no_mangle]
pub extern "C" fn rust_default_exception(int_num: u64, rip: u64) {
    unsafe { asm!("cli"); }
    crate::println!("\n========================================================");
    crate::println!("  [CPU EXCEPTION #{}] Kernel Shat Itself               ", int_num);
    crate::println!("========================================================");
    crate::println!(" Instruction Pointer: 0x{:016x}", rip);
    crate::println!(" Diagnosis          : Whatever you just executed was a crime against computing.");
    crate::println!(" Action             : CPU halted. Stare at your mistakes.");
    crate::println!("========================================================");
    loop {
        unsafe { asm!("hlt") };
    }
}
