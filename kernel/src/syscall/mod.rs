// kernel/src/syscall/mod.rs - Linux x86_64 Syscall Dispatcher with Brutally Honest Dialog
use core::arch::asm;
use crate::keyboard::Keyboard;
use crate::mm::{FrameAllocator, PageTableManager, PAGE_PRESENT, PAGE_USER, PAGE_WRITABLE};
use crate::rtc::DateTime;

// Linux x86_64 Syscall Numbers
pub const SYS_READ: u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_OPEN: u64 = 2;
pub const SYS_CLOSE: u64 = 3;
pub const SYS_MMAP: u64 = 9;
pub const SYS_MUNMAP: u64 = 11;
pub const SYS_BRK: u64 = 12;
pub const SYS_IOCTL: u64 = 16;
pub const SYS_SCHED_YIELD: u64 = 24;
pub const SYS_NANOSLEEP: u64 = 35;
pub const SYS_GETPID: u64 = 39;
pub const SYS_EXIT: u64 = 60;
pub const SYS_UNAME: u64 = 63;
pub const SYS_GETCWD: u64 = 79;
pub const SYS_CHDIR: u64 = 80;
pub const SYS_GETTIMEOFDAY: u64 = 96;
pub const SYS_SYSINFO: u64 = 99;
pub const SYS_GETUID: u64 = 102;
pub const SYS_GETGID: u64 = 104;
pub const SYS_GETEUID: u64 = 107;
pub const SYS_GETEGID: u64 = 108;
pub const SYS_GETPPID: u64 = 110;
pub const SYS_ARCH_PRCTL: u64 = 158;
pub const SYS_REBOOT: u64 = 169;
pub const SYS_TIME: u64 = 201;
pub const SYS_CLOCK_GETTIME: u64 = 228;
pub const SYS_EXIT_GROUP: u64 = 231;

// MSR registers
const IA32_EFER: u32 = 0xC0000080;
const IA32_STAR: u32 = 0xC0000081;
const IA32_LSTAR: u32 = 0xC0000082;
const IA32_FMASK: u32 = 0xC0000084;
const IA32_FS_BASE: u32 = 0xC0000100;

pub static mut CURRENT_BRK: u64 = 0x600000;
pub static mut USER_EXITED: bool = false;
pub static mut LAST_EXIT_CODE: i64 = 0;

extern "C" {
    fn syscall_entry();
}

pub fn init() {
    unsafe {
        // 1. Enable SCE (Syscall Enable) bit in EFER (bit 0)
        let mut efer_low: u32;
        let mut efer_high: u32;
        asm!("rdmsr", in("ecx") IA32_EFER, out("eax") efer_low, out("edx") efer_high);
        efer_low |= 1; // SCE
        asm!("wrmsr", in("ecx") IA32_EFER, in("eax") efer_low, in("edx") efer_high);

        // 2. Setup STAR MSR:
        // [63:48] User CS/SS base selector = 0x18 (User Code is 0x20, User Data is 0x18)
        // [47:32] Kernel CS/SS base selector = 0x08 (Kernel Code is 0x08, Kernel Data is 0x10)
        let star_high: u32 = (0x18 << 16) | 0x08;
        let star_low: u32 = 0;
        asm!("wrmsr", in("ecx") IA32_STAR, in("eax") star_low, in("edx") star_high);

        // 3. Setup LSTAR MSR (Entry point address)
        let entry_addr = syscall_entry as *const () as u64;
        let lstar_low = (entry_addr & 0xFFFFFFFF) as u32;
        let lstar_high = (entry_addr >> 32) as u32;
        asm!("wrmsr", in("ecx") IA32_LSTAR, in("eax") lstar_low, in("edx") lstar_high);

        // 4. Setup FMASK MSR (Mask RFLAGS on syscall entry: disable interrupts TF/IF)
        let fmask_low: u32 = 0x200; // Mask IF
        let fmask_high: u32 = 0;
        asm!("wrmsr", in("ecx") IA32_FMASK, in("eax") fmask_low, in("edx") fmask_high);
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

#[repr(C)]
struct TimeVal {
    tv_sec: i64,
    tv_usec: i64,
}

#[repr(C)]
struct TimeSpec {
    tv_sec: i64,
    tv_nsec: i64,
}

#[repr(C)]
struct SysInfo {
    uptime: i64,
    loads: [u64; 3],
    totalram: u64,
    freeram: u64,
    sharedram: u64,
    bufferram: u64,
    totalswap: u64,
    freeswap: u64,
    procs: u16,
    pad: u16,
    totalhigh: u64,
    freehigh: u64,
    mem_unit: u32,
    _f: [u8; 20],
}

fn copy_cstr(dest: &mut [u8], src: &[u8]) {
    let len = core::cmp::min(dest.len() - 1, src.len());
    dest[..len].copy_from_slice(&src[..len]);
    dest[len] = 0;
}

#[no_mangle]
pub extern "C" fn syscall_dispatcher(
    num: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    _a4: u64,
    _a5: u64,
    _a6: u64,
) -> i64 {
    match num {
        SYS_READ => {
            let fd = a1 as i32;
            let buf = a2 as *mut u8;
            let count = a3 as usize;

            if fd == 0 {
                // Read from stdin (keyboard)
                if count == 0 || buf.is_null() {
                    return 0;
                }
                let ch = Keyboard::get_char_blocking();
                unsafe {
                    *buf = ch;
                }
                crate::print!("{}", ch as char);
                return 1;
            }
            -9 // -EBADF
        }

        SYS_WRITE => {
            let fd = a1 as i32;
            let buf = a2 as *const u8;
            let count = a3 as usize;

            if fd == 1 || fd == 2 {
                if buf.is_null() {
                    return -14; // -EFAULT
                }
                let slice = unsafe { core::slice::from_raw_parts(buf, count) };
                for &b in slice {
                    if b == b'\n' {
                        crate::print!("\n");
                    } else {
                        crate::print!("{}", b as char);
                    }
                }
                return count as i64;
            }
            -9 // -EBADF
        }

        SYS_OPEN => 3, // Dummy fd
        SYS_CLOSE => 0,

        SYS_MMAP => {
            let addr = a1;
            let length = a2 as usize;
            let mut pt = PageTableManager::from_cr3();
            let base_addr = if addr != 0 {
                addr & !0xFFF
            } else {
                0x70000000
            };

            let pages = (length + 4095) / 4096;
            for i in 0..pages {
                let v = base_addr + (i * 4096) as u64;
                let phys = FrameAllocator::allocate_frame() as u64;
                pt.map_page(v, phys, PAGE_USER | PAGE_WRITABLE | PAGE_PRESENT);
            }
            base_addr as i64
        }

        SYS_MUNMAP => 0,

        SYS_BRK => {
            let req_brk = a1;
            unsafe {
                if req_brk == 0 {
                    return CURRENT_BRK as i64;
                }
                if req_brk > CURRENT_BRK {
                    let mut pt = PageTableManager::from_cr3();
                    let start = (CURRENT_BRK + 4095) & !4095;
                    let end = (req_brk + 4095) & !4095;
                    let pages = (end - start) / 4096;
                    for i in 0..pages {
                        let v = start + (i * 4096);
                        let phys = FrameAllocator::allocate_frame() as u64;
                        pt.map_page(v, phys, PAGE_USER | PAGE_WRITABLE | PAGE_PRESENT);
                    }
                    CURRENT_BRK = req_brk;
                }
                CURRENT_BRK as i64
            }
        }

        SYS_IOCTL => 0, // TCGETS stub

        SYS_SCHED_YIELD => 0, // Yield to imaginary threads

        SYS_NANOSLEEP => {
            let req = a1 as *const TimeSpec;
            if !req.is_null() {
                let ts = unsafe { &*req };
                let loops = (ts.tv_sec as u64 * 10_000_000) + (ts.tv_nsec as u64 * 10);
                for _ in 0..loops {
                    core::hint::spin_loop();
                }
            }
            0
        }

        SYS_GETPID => 420, // Weed joke PID
        SYS_GETPPID => 1,  // PID 1 (init_regret)
        SYS_GETUID | SYS_GETEUID => 0, // Root (unearned)
        SYS_GETGID | SYS_GETEGID => 0,

        SYS_UNAME => {
            let buf = a1 as *mut UtsName;
            if buf.is_null() {
                return -14;
            }
            unsafe {
                let uts = &mut *buf;
                copy_cstr(&mut uts.sysname, b"Linux");
                copy_cstr(&mut uts.nodename, b"shitbox-64");
                copy_cstr(&mut uts.release, b"6.6.0-rust-abi");
                copy_cstr(&mut uts.version, b"#1 SMP PREEMPT Scratch OS 2026");
                copy_cstr(&mut uts.machine, b"x86_64");
                copy_cstr(&mut uts.domainname, b"dumpster.local");
            }
            0
        }

        SYS_GETCWD => {
            let buf = a1 as *mut u8;
            let size = a2 as usize;
            if buf.is_null() || size < 14 {
                return -34; // -ERANGE
            }
            let s = b"/dev/dumpster\0";
            unsafe {
                core::ptr::copy_nonoverlapping(s.as_ptr(), buf, s.len());
            }
            buf as i64
        }

        SYS_CHDIR => 0, // Pretend to change dir

        SYS_GETTIMEOFDAY => {
            let tv_ptr = a1 as *mut TimeVal;
            if !tv_ptr.is_null() {
                let now = DateTime::now();
                unsafe {
                    (*tv_ptr).tv_sec = now.unix_timestamp() as i64;
                    (*tv_ptr).tv_usec = 0;
                }
            }
            0
        }

        SYS_TIME => {
            let tloc = a1 as *mut i64;
            let t = DateTime::now().unix_timestamp() as i64;
            if !tloc.is_null() {
                unsafe { *tloc = t };
            }
            t
        }

        SYS_CLOCK_GETTIME => {
            let tp_ptr = a2 as *mut TimeSpec;
            if !tp_ptr.is_null() {
                let t = DateTime::now().unix_timestamp() as i64;
                unsafe {
                    (*tp_ptr).tv_sec = t;
                    (*tp_ptr).tv_nsec = 0;
                }
            }
            0
        }

        SYS_SYSINFO => {
            let s_ptr = a1 as *mut SysInfo;
            if !s_ptr.is_null() {
                unsafe {
                    (*s_ptr).uptime = 42;
                    (*s_ptr).loads = [9999, 9999, 9999]; // 99.99 load avg
                    (*s_ptr).totalram = 64 * 1024 * 1024;
                    (*s_ptr).freeram = 48 * 1024 * 1024;
                    (*s_ptr).procs = 1;
                    (*s_ptr).mem_unit = 1;
                }
            }
            0
        }

        SYS_REBOOT => {
            crate::println!("\n[SYS] Reboot requested by userland. Goodbye cruelty...\n");
            unsafe {
                // Pulse CPU reset line via 8042 Keyboard Controller
                asm!("out dx, al", in("dx") 0x64u16, in("al") 0xFEu8);
            }
            loop {
                unsafe { asm!("hlt") };
            }
        }

        SYS_ARCH_PRCTL => {
            let code = a1;
            let addr = a2;
            const ARCH_SET_FS: u64 = 0x1002;
            const ARCH_GET_FS: u64 = 0x1003;

            if code == ARCH_SET_FS {
                let low = (addr & 0xFFFFFFFF) as u32;
                let high = (addr >> 32) as u32;
                unsafe {
                    asm!("wrmsr", in("ecx") IA32_FS_BASE, in("eax") low, in("edx") high);
                }
                return 0;
            } else if code == ARCH_GET_FS {
                return -22;
            }
            -22
        }

        SYS_EXIT | SYS_EXIT_GROUP => {
            let code = a1 as i64;
            crate::println!("\n[SYS] Process died with exit code: {}. Good riddance.", code);
            unsafe {
                LAST_EXIT_CODE = code;
                USER_EXITED = true;
            }
            crate::shell::run();
        }

        unknown => {
            crate::println!("\n[SYS WARNING] Unimplemented Linux syscall #{}. Go cry to Linus Torvalds.\n", unknown);
            -38 // -ENOSYS
        }
    }
}
