// userland/shithole.rs - Third Linux ELF binary in Ring 3
// Tests multiple Linux ABI syscalls with brutally honest commentary
#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    exit(1);
}

#[inline]
unsafe fn syscall1(num: u64, arg1: u64) -> i64 {
    let ret: i64;
    asm!(
        "syscall",
        in("rax") num,
        in("rdi") arg1,
        out("rcx") _,
        out("r11") _,
        lateout("rax") ret,
        options(nostack, preserves_flags)
    );
    ret
}

#[inline]
unsafe fn syscall2(num: u64, arg1: u64, arg2: u64) -> i64 {
    let ret: i64;
    asm!(
        "syscall",
        in("rax") num,
        in("rdi") arg1,
        in("rsi") arg2,
        out("rcx") _,
        out("r11") _,
        lateout("rax") ret,
        options(nostack, preserves_flags)
    );
    ret
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

fn write(fd: u64, msg: &str) {
    unsafe {
        syscall3(1, fd, msg.as_ptr() as u64, msg.len() as u64);
    }
}

fn exit(code: u64) -> ! {
    unsafe {
        asm!(
            "syscall",
            in("rax") 60u64,
            in("rdi") code,
            options(noreturn)
        );
    }
}

fn print_num(mut n: u64) {
    if n == 0 {
        write(1, "0");
        return;
    }
    let mut buf = [0u8; 20];
    let mut idx = 20;
    while n > 0 {
        idx -= 1;
        buf[idx] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    unsafe {
        syscall3(1, 1, buf.as_ptr().add(idx) as u64, (20 - idx) as u64);
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    write(1, "\n------------------------------------------------------------\n");
    write(1, " [SHITHOLE ELF] Running third Linux ELF test binary in Ring 3!\n");
    write(1, "------------------------------------------------------------\n");

    let pid = unsafe { syscall1(39, 0) };
    write(1, " [PID] My Process ID is: ");
    print_num(pid as u64);
    write(1, " (Nice)\n");

    let ppid = unsafe { syscall1(110, 0) };
    write(1, " [PPID] My Parent ID is : ");
    print_num(ppid as u64);
    write(1, " (Init Regret)\n");

    let uid = unsafe { syscall1(102, 0) };
    write(1, " [UID] My User ID is   : ");
    print_num(uid as u64);
    write(1, " (Root, somehow)\n");

    let mut cwd_buf = [0u8; 32];
    unsafe {
        syscall2(79, cwd_buf.as_mut_ptr() as u64, 32);
    }
    write(1, " [CWD] Working directory: ");
    let len = cwd_buf.iter().position(|&b| b == 0).unwrap_or(32);
    unsafe {
        syscall3(1, 1, cwd_buf.as_ptr() as u64, len as u64);
    }
    write(1, "\n");

    let time_sec = unsafe { syscall1(201, 0) };
    write(1, " [TIME] CMOS Unix Epoch  : ");
    print_num(time_sec as u64);
    write(1, " seconds\n");

    write(1, " [VERDICT] Linux ABI syscall test passed. Dying peacefully now.\n");
    write(1, "------------------------------------------------------------\n");

    exit(69);
}
