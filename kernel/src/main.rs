// kernel/src/main.rs - 64-bit Rust Kernel Entry & Subsystem Initialization
// AnotherFuckingOS: Brutally Honest Bare-Metal Operating System
#![no_std]
#![no_main]

mod vga;
mod serial;
mod gdt;
mod idt;
mod mm;
mod keyboard;
mod elf;
mod syscall;
mod shell;
mod rtc;
mod pci;
mod speaker;
mod cpuid;
mod fs;

use core::panic::PanicInfo;

pub fn afosos_panic_screen(code: u64) -> ! {
    unsafe {
        vga::WRITER.color = vga::ColorCode::new(vga::Color::White, vga::Color::Blue);
        vga::WRITER.clear();
    }

    println!("============================================================");
    println!("                         AFOSOS                              ");
    println!("============================================================");
    println!();
    println!("HELP, AN EVIL PROGRAM STRANGLED ME WITH MY OWN ORGANS");
    println!("THEN HARVESTED MY KIDNEY FOR BEAR MONEY!");
    println!("err 0x{:016x}", code);
    println!();
    println!("System halted. This is VERY broken.");

    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("\n================== KERNEL PANIC ==================");
    println!("  Congratulations, you absolute clown. You broke it.");
    if let Some(location) = info.location() {
        println!("  Crime Scene   : {}:{}", location.file(), location.line());
    }
    println!("  Autopsy Report: {}", info.message());
    println!("  Action        : CPU halted. Go contemplate your life choices.");
    println!("==================================================");

    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn memset(dest: *mut u8, c: i32, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *dest.add(i) = c as u8;
        i += 1;
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *dest.add(i) = *src.add(i);
        i += 1;
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if dest < src as *mut u8 {
        memcpy(dest, src, n)
    } else {
        let mut i = n;
        while i > 0 {
            i -= 1;
            *dest.add(i) = *src.add(i);
        }
        dest
    }
}

#[no_mangle]
pub unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let diff = (*s1.add(i) as i32) - (*s2.add(i) as i32);
        if diff != 0 {
            return diff;
        }
        i += 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn bcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    memcmp(s1, s2, n)
}

#[no_mangle]
pub extern "C" fn rust_eh_personality() {}

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    // 1. Initialize Serial COM1 port (immediate debug output)
    serial::SerialPort::init();
    serial_println!("[KERNEL] COM1 Serial Port initialized.");

    // 2. Initialize VGA Text Screen
    unsafe {
        vga::WRITER.clear();
    }
    println!("------------------------------------------------------------");
    println!("  AnotherFuckingOS v6.6.0 (x86_64 Long Mode)                ");
    println!("  Because Windows and Linux weren't disappointing enough.   ");
    println!("------------------------------------------------------------");

    // 3. Initialize GDT & TSS (Ring 0 & Ring 3 segments)
    gdt::init();
    println!("[OK] GDT & TSS loaded. Ring 0/3 privilege separation active.");

    // 4. Initialize IDT (Exceptions & Interrupts)
    idt::init();
    println!("[OK] IDT exception gates registered. Ready to catch your sins.");

    // 5. Initialize Linux Syscall Subsystem (MSR STAR, LSTAR, FMASK, EFER.SCE)
    syscall::init();
    println!("[OK] Linux Syscall interface (LSTAR/STAR) active. Linux ABI ready.");

    // 6. Memory Paging and Allocator
    println!("[OK] Memory Paging and Allocator ready. 64MB of RAM to waste.");

    // 7. Virtual RamFS
    unsafe {
        fs::VFS.init();
    }
    println!("[OK] Virtual RamFS mounted with depressing system files.");

    // Startup beep
    // speaker::beep(880, 2_000_000);

    // Jump into interactive kernel shell
    shell::run();
}
