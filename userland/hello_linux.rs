// userland/hello_linux.rs - Native Linux ELF binary in Rust
// Uses pure Linux x86_64 syscalls (write, uname, exit)
#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    exit(1);
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

#[inline]
unsafe fn syscall3(num: u64, arg1: u64, arg2: u64, arg3: u64) -> i64 {
    let ret: i64;
    asm!(
        "syscall",
        in("rax") num,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        out("rcx") _,
        out("r11") _,
        lateout("rax") ret,
        options(nostack, preserves_flags)
    );
    ret
}

#[inline]
unsafe fn syscall1(num: u64, arg1: u64) -> ! {
    asm!(
        "syscall",
        in("rax") num,
        in("rdi") arg1,
        options(noreturn)
    );
}

fn write(fd: u64, msg: &str) {
    unsafe {
        syscall3(1, fd, msg.as_ptr() as u64, msg.len() as u64);
    }
}

fn exit(code: u64) -> ! {
    unsafe {
        syscall1(60, code);
    }
}

#[repr(C)]
struct UtsName {
    sysname: [u8; 65],
    nodename: [u8; 65],
    release: [u8; 65],
    version: [u8; 65],
    machine: [u8; 65],
    domainname: [u8; 65],
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    write(1, "\n>>> [USERLAND ELF] Hello from native Linux ELF running in Ring 3!\n");
    write(1, ">>> [USERLAND ELF] Executing Linux sys_uname (syscall #63)...\n");

    let mut uts: UtsName = unsafe { core::mem::zeroed() };
    let res = unsafe { syscall3(63, &mut uts as *mut _ as u64, 0, 0) };

    if res == 0 {
        write(1, ">>> [USERLAND ELF] uname reported sysname: ");
        let sysname_len = uts.sysname.iter().position(|&c| c == 0).unwrap_or(65);
        if let Ok(s) = core::str::from_utf8(&uts.sysname[..sysname_len]) {
            write(1, s);
        }
        write(1, "\n>>> [USERLAND ELF] uname reported release: ");
        let rel_len = uts.release.iter().position(|&c| c == 0).unwrap_or(65);
        if let Ok(s) = core::str::from_utf8(&uts.release[..rel_len]) {
            write(1, s);
        }
        write(1, "\n");
    }

    write(1, ">>> [USERLAND ELF] Testing Linux sys_exit(42) (syscall #60)...\n");
    exit(42);
}
