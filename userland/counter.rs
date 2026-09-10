// userland/counter.rs - Native Linux ELF calculating numbers & heap allocations via brk
#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe {
        asm!("syscall", in("rax") 60u64, in("rdi") 1u64, options(noreturn));
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

fn write(msg: &str) {
    unsafe {
        syscall3(1, 1, msg.as_ptr() as u64, msg.len() as u64);
    }
}

fn print_num(mut n: u64) {
    if n == 0 {
        write("0");
        return;
    }
    let mut buf = [0u8; 32];
    let mut idx = 32;
    while n > 0 {
        idx -= 1;
        buf[idx] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    let slice = &buf[idx..];
    unsafe {
        syscall3(1, 1, slice.as_ptr() as u64, slice.len() as u64);
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    write("\n>>> [COUNTER ELF] Fibonacci Sequence Demo in Ring 3:\n");

    let mut a: u64 = 0;
    let mut b: u64 = 1;

    for i in 1..=12 {
        write("    Fib(");
        print_num(i);
        write(") = ");
        print_num(b);
        write("\n");

        let c = a + b;
        a = b;
        b = c;
    }

    write(">>> [COUNTER ELF] Requesting heap memory via sys_brk (syscall #12)...\n");
    let cur_brk = unsafe { syscall1(12, 0) };
    let new_brk = unsafe { syscall1(12, (cur_brk + 4096) as u64) };

    if new_brk > cur_brk {
        write(">>> [COUNTER ELF] sys_brk successfully expanded heap page!\n");
    }

    write(">>> [COUNTER ELF] Finished. Exiting with code 0.\n");
    unsafe {
        asm!("syscall", in("rax") 60u64, in("rdi") 0u64, options(noreturn));
    }
}
