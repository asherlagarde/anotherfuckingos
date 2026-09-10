// kernel/src/shell.rs - Interactive Kernel Shell with 70+ Brutally Honest Features
use core::arch::asm;
use crate::keyboard::Keyboard;
use crate::cpuid::{CpuInfo, get_random_u64};
use crate::rtc::DateTime;
use crate::pci;
use crate::speaker;
use crate::fs::VFS;

extern "C" {
    fn jump_to_user_mode(entry_point: u64, user_sp: u64) -> !;
}

// Embedded native Linux ELF binaries
static HELLO_ELF: &[u8] = include_bytes!("../../build/hello_linux.elf");
static COUNTER_ELF: &[u8] = include_bytes!("../../build/counter.elf");
static SHITHOLE_ELF: &[u8] = include_bytes!("../../build/shithole.elf");

// Shell state
static mut HOSTNAME: [u8; 32] = *b"shitbox-64\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
static mut HISTORY: [[u8; 64]; 8] = [[0; 64]; 8];
static mut HIST_COUNT: usize = 0;
static mut COOKIE_COUNT: u64 = 0;

// Environment variables
#[derive(Copy, Clone)]
struct EnvVar {
    key: [u8; 16],
    val: [u8; 32],
}
static mut ENV_VARS: [Option<EnvVar>; 8] = [None; 8];

unsafe fn init_env() {
    set_env("USER", "loser");
    set_env("SHELL", "/bin/shithole");
    set_env("TERM", "vga80x25");
    set_env("LIFE_CHOICES", "terrible");
    set_env("SALARY", "insufficient");
}

unsafe fn set_env(key: &str, val: &str) {
    for slot in ENV_VARS.iter_mut() {
        if let Some(ref mut ev) = slot {
            if str_eq_bytes(key, &ev.key) {
                copy_str_to_buf(val, &mut ev.val);
                return;
            }
        }
    }
    for slot in ENV_VARS.iter_mut() {
        if slot.is_none() {
            let mut ev = EnvVar { key: [0; 16], val: [0; 32] };
            copy_str_to_buf(key, &mut ev.key);
            copy_str_to_buf(val, &mut ev.val);
            *slot = Some(ev);
            return;
        }
    }
}

unsafe fn unset_env(key: &str) {
    for slot in ENV_VARS.iter_mut() {
        if let Some(ref ev) = slot {
            if str_eq_bytes(key, &ev.key) {
                *slot = None;
                return;
            }
        }
    }
}

fn str_eq_bytes(s: &str, b: &[u8]) -> bool {
    let s_bytes = s.as_bytes();
    let b_len = b.iter().position(|&x| x == 0).unwrap_or(b.len());
    s_bytes == &b[..b_len]
}

fn copy_str_to_buf(s: &str, buf: &mut [u8]) {
    let len = core::cmp::min(s.len(), buf.len() - 1);
    buf[..len].copy_from_slice(&s.as_bytes()[..len]);
    buf[len] = 0;
}

#[inline(never)]
unsafe fn dbg_com1(b: u8) {
    asm!("out dx, al", in("dx") 0x3F8u16, in("al") b, options(nomem, nostack));
}

pub fn run() -> ! {
    unsafe {
        dbg_com1(b'A');
        init_env();
        dbg_com1(b'B');
    }

    unsafe { dbg_com1(b'C'); }
    crate::println!();
    unsafe { dbg_com1(b'D'); }
    crate::println!("========================================================");
    unsafe { dbg_com1(b'E'); }
    crate::println!("   ANOTHER FUCKING OS - Linux ABI Shell (x86_64)       ");
    unsafe { dbg_com1(b'F'); }
    crate::println!("   Type 'help' if you're lost (you probably are).       ");
    unsafe { dbg_com1(b'G'); }
    crate::println!("   70+ features of pure unadulterated regret.           ");
    unsafe { dbg_com1(b'H'); }
    crate::println!("========================================================");
    unsafe { dbg_com1(b'I'); }

    let mut line_buf = [0u8; 64];
    let mut line_len: usize;

    loop {
        unsafe {
            let h_len = HOSTNAME.iter().position(|&b| b == 0).unwrap_or(HOSTNAME.len());
            let h_str = core::str::from_utf8(&HOSTNAME[..h_len]).unwrap_or("shitbox");
            crate::print!("loser@{}:~# ", h_str);
        }
        line_len = 0;

        loop {
            let c = Keyboard::get_char_blocking();
            if c == b'\n' || c == b'\r' {
                crate::println!();
                break;
            } else if c == 0x08 {
                // Backspace
                if line_len > 0 {
                    line_len -= 1;
                    crate::print!("{}", 0x08 as char);
                }
            } else if line_len < line_buf.len() - 1 && c >= 32 && c <= 126 {
                line_buf[line_len] = c;
                line_len += 1;
                crate::print!("{}", c as char);
            }
        }

        let cmd = core::str::from_utf8(&line_buf[..line_len]).unwrap_or("");
        let trimmed = cmd.trim();
        if !trimmed.is_empty() {
            unsafe {
                if HIST_COUNT < 8 {
                    copy_str_to_buf(trimmed, &mut HISTORY[HIST_COUNT]);
                    HIST_COUNT += 1;
                } else {
                    for i in 1..8 {
                        HISTORY[i - 1] = HISTORY[i];
                    }
                    copy_str_to_buf(trimmed, &mut HISTORY[7]);
                }
            }
            execute_command(trimmed);
        }
    }
}

fn execute_command(cmd: &str) {
    let mut parts = cmd.splitn(2, ' ');
    let op = parts.next().unwrap_or("");
    let arg = parts.next().unwrap_or("").trim();

    match op {
        "help" | "?" => cmd_help(),
        "neofetch" | "fastfetch" => cmd_neofetch(),
        "uname" => crate::println!("Linux 6.6.0-rust-abi #1 SMP PREEMPT Scratch OS 2026 x86_64 GNU/Linux (100% genuine placebo)"),
        "whoami" => crate::println!("root - An unappreciated meat sack sitting in front of glowing pixels."),
        "uptime" => cmd_uptime(),
        "date" | "rtc" => cmd_date(),
        "free" | "meminfo" => cmd_free(),
        "cpuid" => cmd_cpuid(),
        "lspci" => cmd_lspci(),
        "regs" | "cr4" => cmd_regs(),
        "rand" | "rdrand" => crate::println!("Random uint64: 0x{:016x} (Randomness won't fix your problems)", get_random_u64()),
        "beep" => {
            crate::println!("Beeping PC speaker at 440Hz... Wake up!");
            speaker::beep(440, 4_000_000);
        },
        "play" | "jingle" => {
            crate::println!("Playing triumphant failure fanfare...");
            speaker::jingle();
        },
        "dmesg" => cmd_dmesg(),
        "bench" => cmd_bench(),
        "vga" => cmd_vga(),
        "colors" => cmd_colors(),

        // Filesystem
        "ls" | "dir" => cmd_ls(),
        "cat" => cmd_cat(arg),
        "touch" => cmd_touch(arg),
        "echo" => cmd_echo(arg),
        "rm" => cmd_rm(arg),
        "pwd" => crate::println!("/dev/dumpster (nowhere you want to be)"),
        "hostname" => cmd_hostname(arg),
        "ps" => cmd_ps(),
        "kill" => cmd_kill(arg),
        "wc" => cmd_wc(arg),
        "head" => cmd_head(arg),
        "tail" => cmd_tail(arg),
        "grep" => cmd_grep(arg),
        "hexdump" => cmd_hexdump(arg),

        // Environment & Shell State
        "env" => cmd_env(),
        "export" => cmd_export(arg),
        "unset" => cmd_unset(arg),
        "history" => cmd_history(),
        "alias" => cmd_alias(),
        "clear" | "cls" => unsafe { crate::vga::WRITER.clear(); },

        // Cynical Utilities & Fun
        "calc" => cmd_calc(arg),
        "cowsay" => cmd_cowsay(arg),
        "fortune" => cmd_fortune(),
        "8ball" => cmd_8ball(),
        "roast" => cmd_roast(),
        "joke" => cmd_joke(),
        "excuse" => cmd_excuse(),
        "quote" => cmd_quote(),
        "insult" => cmd_insult(),
        "art" => cmd_art(arg),
        "coin" => cmd_coin(),
        "dice" | "roll" => cmd_dice(),
        "guess" => cmd_guess(arg),
        "rps" => cmd_rps(arg),
        "roulette" => cmd_roulette(),
        "cookie" => cmd_cookie(),
        "matrix" => cmd_matrix(),
        "sl" => cmd_sl(),
        "fire" => cmd_fire(),
        "sleep" => cmd_sleep(arg),
        "typing" => cmd_typing(),
        "trivia" => cmd_trivia(),
        "battery" => crate::println!("Battery: 1% (Discharging). Plug it in or lose everything."),

        // Hardware ports
        "ports" | "inb" => cmd_inb(arg),
        "outb" => cmd_outb(arg),
        "memleak" => cmd_memleak(),

        // Crash tests
        "crash" | "panic" => panic!("User manually requested annihilation."),
        "divzero" => {
            crate::println!("Executing divide by zero: preparing to trigger CPU #DE...");
            unsafe {
                let zero = 0u64;
                asm!("mov rdx, 0", "mov rax, 100", "div {0}", in(reg) zero);
            }
        },
        "segfault" => {
            crate::println!("Dereferencing null pointer (0x0): preparing to trigger CPU #PF...");
            unsafe {
                let ptr = 0x0 as *const u64;
                let _val = *ptr;
            }
        },
        "overflow" => {
            crate::println!("Testing stack bounds checking... You were warned.");
            fn recurse(depth: u64) -> u64 {
                if depth == 0 { 0 } else { recurse(depth - 1) + 1 }
            }
            recurse(500_000);
        },

        // Linux ELF Launchers
        "run" => cmd_run(arg),
        "hello" => cmd_run("hello"),
        "counter" => cmd_run("counter"),
        "shithole" => cmd_run("shithole"),

        // Power commands
        "reboot" => {
            crate::println!("Rebooting machine via 8042 keyboard controller. See you never.");
            unsafe {
                asm!("out dx, al", in("dx") 0x64u16, in("al") 0xFEu8);
            }
            loop { unsafe { asm!("hlt"); } }
        },
        "shutdown" | "poweroff" | "exit" => {
            crate::println!("Shutting down QEMU via ACPI port 0x604... Finally peace.");
            unsafe {
                // QEMU ACPI poweroff
                asm!("out dx, ax", in("dx") 0x604u16, in("ax") 0x2000u16);
                asm!("out dx, ax", in("dx") 0xB004u16, in("ax") 0x2000u16);
            }
            crate::println!("ACPI poweroff didn't halt CPU. Halting now.");
            loop { unsafe { asm!("hlt"); } }
        },

        _ => {
            crate::println!("Unknown command: '{}'. Can you even read? Type 'help'.", cmd);
        }
    }
}

fn cmd_help() {
    crate::println!("================ AVAILABLE SHIT FEATURES (70+) ================");
    crate::println!(" [SYSTEM INSPECTION]   neofetch, uname, whoami, uptime, date, free,");
    crate::println!("                       cpuid, lspci, regs, rand, dmesg, vga, colors");
    crate::println!(" [FILESYSTEM]          ls, cat, touch, echo, rm, pwd, wc, head, tail,");
    crate::println!("                       grep, hexdump, hostname, ps, kill");
    crate::println!(" [SHELL & ENV]         env, export, unset, history, alias, clear");
    crate::println!(" [CYNICAL TOOLS]       calc, cowsay, fortune, 8ball, roast, joke,");
    crate::println!("                       excuse, quote, insult, art, coin, dice");
    crate::println!(" [GAMES & TOYS]        guess, rps, roulette, cookie, matrix, sl,");
    crate::println!("                       fire, sleep, typing, trivia, battery");
    crate::println!(" [HARDWARE & AUDIO]    beep, play, bench, inb, outb, memleak");
    crate::println!(" [CRASH TESTS]         panic, divzero, segfault, overflow");
    crate::println!(" [LINUX USERLAND ELF]  run hello, run counter, run shithole");
    crate::println!(" [POWER]               reboot, shutdown, exit");
    crate::println!("===============================================================");
}

fn cmd_neofetch() {
    let cpu = CpuInfo::get();
    let dt = DateTime::now();
    crate::println!("    (o_    OS       : AnotherFuckingOS (x86_64 Long Mode)");
    crate::println!("    //\\    Kernel   : 6.6.0-rust-abi (zero guarantees)");
    crate::println!("    V_/_   CPU      : {} ({})", cpu.vendor_str(), cpu.honest_roast());
    crate::println!("           RAM      : 16MB mapped / 64MB physical / 100% wasted");
    crate::println!("           Uptime   : {:02}:{:02}:{:02} (Time you'll never get back)", dt.hour, dt.minute, dt.second);
    crate::println!("           Display  : VGA 80x25 Text Mode (1981 technology)");
    crate::println!("           Shell    : shitbox-sh 0.1-bleeding");
    crate::println!("           Mood     : Severely Disappointed");
}

fn cmd_uptime() {
    let dt = DateTime::now();
    crate::println!("Up {:02}:{:02}:{:02}. Congratulations on wasting another hour.", dt.hour, dt.minute, dt.second);
}

fn cmd_date() {
    let dt = DateTime::now();
    crate::println!("Hardware RTC: {:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        dt.year, dt.month, dt.day, dt.hour, dt.minute, dt.second);
    crate::println!("Epoch Timestamp: {} seconds", dt.unix_timestamp());
}

fn cmd_free() {
    crate::println!("              total        used        free      wasted");
    crate::println!("Mem:       65536 KB    16384 KB    49152 KB    65536 KB");
    crate::println!("Swap:          0 KB        0 KB        0 KB        0 KB");
    crate::println!("Note: All 64MB of physical memory is currently suffering.");
}

fn cmd_cpuid() {
    let cpu = CpuInfo::get();
    crate::println!("CPUID Vendor String: {}", cpu.vendor_str());
    crate::println!("  SSE Support      : {}", if cpu.has_sse { "Yes" } else { "No" });
    crate::println!("  SSE2 Support     : {}", if cpu.has_sse2 { "Yes" } else { "No" });
    crate::println!("  AVX Support      : {}", if cpu.has_avx { "Yes" } else { "No" });
    crate::println!("  Hardware RDRAND  : {}", if cpu.has_rdrand { "Yes" } else { "No" });
    crate::println!("  Virtual/VM Guest : {}", if cpu.is_hypervisor { "Yes (Emulated wimp)" } else { "No (Bare Metal)" });
    crate::println!("Verdict: {}", cpu.honest_roast());
}

fn cmd_lspci() {
    crate::println!("Scanning PCI Configuration Space (bus 0, devs 0..31)...");
    let (devs, count) = pci::scan_bus();
    if count == 0 {
        crate::println!("No PCI devices found. Either your bus is dead or we are in a void.");
        return;
    }
    for i in 0..count {
        if let Some(dev) = devs[i] {
            crate::println!("00:{:02x}.0 {}: [0x{:04x}:0x{:04x}] {}",
                dev.device,
                pci::class_name(dev.class_code, dev.subclass),
                dev.vendor_id,
                dev.device_id,
                pci::vendor_name(dev.vendor_id)
            );
        }
    }
}

fn cmd_regs() {
    let cr0: u64;
    let cr2: u64;
    let cr3: u64;
    let cr4: u64;
    let rflags: u64;
    unsafe {
        asm!("mov {}, cr0", out(reg) cr0);
        asm!("mov {}, cr2", out(reg) cr2);
        asm!("mov {}, cr3", out(reg) cr3);
        asm!("mov {}, cr4", out(reg) cr4);
        asm!("pushfq", "pop {}", out(reg) rflags);
    }
    crate::println!("CR0   : 0x{:016x} (Paging={}, Protection={})", cr0, (cr0 >> 31) & 1, cr0 & 1);
    crate::println!("CR2   : 0x{:016x} (Last Page Fault Address)", cr2);
    crate::println!("CR3   : 0x{:016x} (PML4 Base Address)", cr3);
    crate::println!("CR4   : 0x{:016x} (PAE={}, OSFXSR={})", cr4, (cr4 >> 5) & 1, (cr4 >> 9) & 1);
    crate::println!("RFLAGS: 0x{:016x} (IF={}, IOPL={})", rflags, (rflags >> 9) & 1, (rflags >> 12) & 3);
}

fn cmd_dmesg() {
    crate::println!("[0.000000] Bootloader handed control to 64-bit long mode at 0x8000.");
    crate::println!("[0.001204] COM1 UART (0x3F8) initialized.");
    crate::println!("[0.002481] GDT & TSS configured. Kernel Ring 0, User Ring 3.");
    crate::println!("[0.003912] IDT 256 gates registered. Fault handlers ready to judge you.");
    crate::println!("[0.004120] MSR STAR, LSTAR, FMASK configured for syscall/sysret ABI.");
    crate::println!("[0.005089] Paging initialized (4-level PML4 at 0x1000).");
    crate::println!("[0.006214] RamFS mounted at /.");
    crate::println!("[0.007812] Shell started. Operator error is imminent.");
}

fn cmd_bench() {
    crate::println!("Running prime number benchmark (1 to 20,000)...");
    let start = get_random_u64();
    let mut primes = 0;
    for n in 2..15000 {
        let mut is_prime = true;
        let mut d = 2;
        while d * d <= n {
            if n % d == 0 {
                is_prime = false;
                break;
            }
            d += 1;
        }
        if is_prime {
            primes += 1;
        }
    }
    let _end = get_random_u64().wrapping_add(start);
    crate::println!("Found {} primes.", primes);
    crate::println!("CPU Performance Rating: Marginally faster than a toaster oven.");
}

fn cmd_vga() {
    crate::println!("VGA Hardware Controller: 0x3D4/0x3D5");
    crate::println!("Frame Buffer Base      : 0xB8000");
    crate::println!("Resolution             : 80 columns x 25 rows (text mode)");
    crate::println!("Character Dimensions   : 8x16 dots");
}

fn cmd_colors() {
    crate::println!("VGA 16-Color Palette test:");
    crate::println!("0:Black 1:Blue 2:Green 3:Cyan 4:Red 5:Magenta 6:Brown 7:LightGray");
    crate::println!("8:DarkGray 9:LightBlue 10:LightGreen 11:LightCyan 12:LightRed 13:Pink 14:Yellow 15:White");
    crate::println!("Look at all these vibrant colors you can be depressed in!");
}

fn cmd_ls() {
    crate::println!("Permissions  Size  Owner  Name");
    crate::println!("-----------  ----  -----  ----");
    crate::println!("-rwxr-xr-x   {:4}  root   hello_linux.elf", HELLO_ELF.len());
    crate::println!("-rwxr-xr-x   {:4}  root   counter.elf", COUNTER_ELF.len());
    crate::println!("-rwxr-xr-x   {:4}  root   shithole.elf", SHITHOLE_ELF.len());

    unsafe {
        for slot in VFS.files.iter() {
            if let Some(ref f) = slot {
                crate::println!("-rw-r--r--   {:4}  root   {}", f.data_len, f.name_str());
            }
        }
    }
}

fn cmd_cat(name: &str) {
    if name.is_empty() {
        crate::println!("Usage: cat <filename> (Provide a file, you mind reader).");
        return;
    }
    unsafe {
        if let Some(f) = VFS.get_file(name) {
            crate::print!("{}", f.content_str());
        } else {
            crate::println!("cat: {}: No such file or directory. Try dreaming harder.", name);
        }
    }
}

fn cmd_touch(name: &str) {
    if name.is_empty() {
        crate::println!("Usage: touch <filename>");
        return;
    }
    unsafe {
        if VFS.add_file(name, b"", false) {
            crate::println!("Created empty file '{}'. More empty space to match your head.", name);
        } else {
            crate::println!("touch: Disk full or file exists.");
        }
    }
}

fn cmd_echo(arg: &str) {
    if let Some(pos) = arg.find('>') {
        let text = arg[..pos].trim();
        let fname = arg[pos + 1..].trim();
        if fname.is_empty() {
            crate::println!("echo: missing destination file.");
            return;
        }
        unsafe {
            if VFS.add_file(fname, text.as_bytes(), false) {
                crate::println!("Wrote {} bytes to '{}'.", text.len(), fname);
            } else {
                crate::println!("echo: Failed to write to file.");
            }
        }
    } else {
        crate::println!("{}", arg);
    }
}

fn cmd_rm(name: &str) {
    if name.is_empty() {
        crate::println!("Usage: rm <filename>");
        return;
    }
    unsafe {
        match VFS.delete_file(name) {
            Ok(()) => crate::println!("Deleted '{}'. Gone forever, just like your wasted youth.", name),
            Err(e) => crate::println!("rm: {}", e),
        }
    }
}

fn cmd_hostname(arg: &str) {
    unsafe {
        if arg.is_empty() {
            let len = HOSTNAME.iter().position(|&b| b == 0).unwrap_or(HOSTNAME.len());
            crate::println!("{}", core::str::from_utf8(&HOSTNAME[..len]).unwrap_or("shitbox"));
        } else {
            copy_str_to_buf(arg, &mut HOSTNAME);
            crate::println!("Hostname changed to '{}'. Doesn't make the OS any better.", arg);
        }
    }
}

fn cmd_ps() {
    crate::println!("  PID TTY          TIME CMD");
    crate::println!("    0 ?        00:00:00 [idle_regret]");
    crate::println!("    1 ?        00:00:00 /sbin/init_regret (better than systemd)");
    crate::println!("  420 tty1     00:00:00 /bin/shitbox-sh (interactive disappointment)");
}

fn cmd_kill(arg: &str) {
    if arg.is_empty() {
        crate::println!("Usage: kill <pid>");
        return;
    }
    crate::println!("kill: Process {} is already dead inside. SIGKILL ignored.", arg);
}

fn cmd_wc(arg: &str) {
    if arg.is_empty() {
        crate::println!("Usage: wc <filename>");
        return;
    }
    unsafe {
        if let Some(f) = VFS.get_file(arg) {
            let mut lines = 0;
            let mut words = 0;
            let mut in_word = false;
            for &b in &f.data[..f.data_len] {
                if b == b'\n' {
                    lines += 1;
                }
                if b == b' ' || b == b'\n' || b == b'\t' {
                    in_word = false;
                } else if !in_word {
                    in_word = true;
                    words += 1;
                }
            }
            crate::println!(" {}  {} {} {}", lines, words, f.data_len, arg);
        } else {
            crate::println!("wc: {}: File not found.", arg);
        }
    }
}

fn cmd_head(arg: &str) {
    if arg.is_empty() {
        crate::println!("Usage: head <filename>");
        return;
    }
    unsafe {
        if let Some(f) = VFS.get_file(arg) {
            let mut count = 0;
            for &b in &f.data[..f.data_len] {
                crate::print!("{}", b as char);
                if b == b'\n' {
                    count += 1;
                    if count >= 5 {
                        break;
                    }
                }
            }
        } else {
            crate::println!("head: {}: File not found.", arg);
        }
    }
}

fn cmd_tail(arg: &str) {
    if arg.is_empty() {
        crate::println!("Usage: tail <filename>");
        return;
    }
    cmd_cat(arg);
}

fn cmd_grep(arg: &str) {
    let mut parts = arg.splitn(2, ' ');
    let pattern = parts.next().unwrap_or("");
    let file = parts.next().unwrap_or("").trim();
    if pattern.is_empty() || file.is_empty() {
        crate::println!("Usage: grep <pattern> <file>");
        return;
    }
    unsafe {
        if let Some(f) = VFS.get_file(file) {
            let text = f.content_str();
            let mut found = false;
            for line in text.lines() {
                if line.contains(pattern) {
                    crate::println!("{}", line);
                    found = true;
                }
            }
            if !found {
                crate::println!("Pattern '{}' not found. Just like what you're looking for in life.", pattern);
            }
        } else {
            crate::println!("grep: {}: No such file.", file);
        }
    }
}

fn cmd_hexdump(arg: &str) {
    if arg.is_empty() {
        crate::println!("Usage: hexdump <file>");
        return;
    }
    unsafe {
        if let Some(f) = VFS.get_file(arg) {
            let len = core::cmp::min(f.data_len, 64);
            for (i, chunk) in f.data[..len].chunks(16).enumerate() {
                crate::print!("{:04x}: ", i * 16);
                for &b in chunk {
                    crate::print!("{:02x} ", b);
                }
                crate::println!();
            }
        } else {
            crate::println!("hexdump: {}: File not found.", arg);
        }
    }
}

fn cmd_env() {
    unsafe {
        for slot in ENV_VARS.iter() {
            if let Some(ref ev) = slot {
                let k_len = ev.key.iter().position(|&b| b == 0).unwrap_or(ev.key.len());
                let v_len = ev.val.iter().position(|&b| b == 0).unwrap_or(ev.val.len());
                let k_str = core::str::from_utf8(&ev.key[..k_len]).unwrap_or("?");
                let v_str = core::str::from_utf8(&ev.val[..v_len]).unwrap_or("?");
                crate::println!("{}={}", k_str, v_str);
            }
        }
    }
}

fn cmd_export(arg: &str) {
    if let Some(pos) = arg.find('=') {
        let k = arg[..pos].trim();
        let v = arg[pos + 1..].trim();
        unsafe { set_env(k, v); }
        crate::println!("Exported '{}' = '{}'. Nobody cares.", k, v);
    } else {
        crate::println!("Usage: export KEY=VALUE");
    }
}

fn cmd_unset(arg: &str) {
    if arg.is_empty() {
        crate::println!("Usage: unset KEY");
        return;
    }
    unsafe { unset_env(arg); }
    crate::println!("Unset '{}'. Gone from your little bubble.", arg);
}

fn cmd_history() {
    crate::println!("Chronicle of your recent typing blunders:");
    unsafe {
        for i in 0..HIST_COUNT {
            let len = HISTORY[i].iter().position(|&b| b == 0).unwrap_or(HISTORY[i].len());
            crate::println!(" {:2}  {}", i + 1, core::str::from_utf8(&HISTORY[i][..len]).unwrap_or(""));
        }
    }
}

fn cmd_alias() {
    crate::println!("Configured Sarcastic Aliases:");
    crate::println!("  ll       -> ls -la (shows more things you don't understand)");
    crate::println!("  windows  -> panic (same thing anyway)");
    crate::println!("  fix      -> reboot (the universal IT solution)");
    crate::println!("  hope     -> /dev/null");
}

fn cmd_calc(arg: &str) {
    if arg.is_empty() {
        crate::println!("Usage: calc <num1> <op> <num2> (e.g. calc 40 + 2)");
        return;
    }
    let mut parts = arg.split_whitespace();
    let s1 = parts.next().unwrap_or("0");
    let op = parts.next().unwrap_or("+");
    let s2 = parts.next().unwrap_or("0");

    let n1: i64 = parse_int(s1);
    let n2: i64 = parse_int(s2);

    match op {
        "+" => crate::println!("{} + {} = {}. Elementary school math, congratulations.", n1, n2, n1 + n2),
        "-" => crate::println!("{} - {} = {}. Subtracting from your limited attention span.", n1, n2, n1 - n2),
        "*" => crate::println!("{} * {} = {}. Multiplying your errors.", n1, n2, n1 * n2),
        "/" => {
            if n2 == 0 {
                crate::println!("Division by zero! Don't tempt the CPU exception handler.");
            } else {
                crate::println!("{} / {} = {}. Remainder: {}.", n1, n2, n1 / n2, n1 % n2);
            }
        },
        "%" => {
            if n2 == 0 {
                crate::println!("Modulo by zero is illegal.");
            } else {
                crate::println!("{} % {} = {}.", n1, n2, n1 % n2);
            }
        },
        _ => crate::println!("Unknown math operator '{}'.", op),
    }
}

fn parse_int(s: &str) -> i64 {
    let mut val: i64 = 0;
    let mut neg = false;
    let mut bytes = s.bytes();
    if let Some(b) = bytes.next() {
        if b == b'-' {
            neg = true;
        } else if b >= b'0' && b <= b'9' {
            val = (b - b'0') as i64;
        }
    }
    for b in bytes {
        if b >= b'0' && b <= b'9' {
            val = val.wrapping_mul(10).wrapping_add((b - b'0') as i64);
        }
    }
    if neg { -val } else { val }
}

fn cmd_cowsay(msg: &str) {
    let text = if msg.is_empty() { "You are wasting your life." } else { msg };
    crate::println!(" ________________________________________");
    crate::println!("< {} >", text);
    crate::println!(" ----------------------------------------");
    crate::println!("        \\   ^__^");
    crate::println!("         \\  (oo)\\_______");
    crate::println!("            (__)\\       )\\/\\");
    crate::println!("                ||----w |");
    crate::println!("                ||     ||");
}

fn cmd_fortune() {
    let fortunes = [
        "Today is an excellent day to shut down your PC and reconsider your career.",
        "Your code compiles cleanly only because the compiler feels sorry for you.",
        "A bug you wrote 6 months ago is waiting to detonate in production.",
        "If at first you don't succeed, skydiving is not for you. Neither is OS dev.",
        "The light at the end of the tunnel is an incoming kernel panic.",
        "Hard work pays off in the future. Laziness pays off right now.",
    ];
    let idx = (get_random_u64() as usize) % fortunes.len();
    crate::println!("Fortune: {}", fortunes[idx]);
}

fn cmd_8ball() {
    let answers = [
        "Don't count on it.",
        "Outlook horrific.",
        "Very doubtful.",
        "My sources say you will fail.",
        "Reply hazy, try finding a real job.",
        "Signs point to disaster.",
    ];
    let idx = (get_random_u64() as usize) % answers.len();
    crate::println!("Magic 8-Ball: {}", answers[idx]);
}

fn cmd_roast() {
    let roasts = [
        "Your commit history reads like a cry for help.",
        "You're the human equivalent of a NullPointerDereference.",
        "Even Javascript developers look down on the code you wrote today.",
        "Your processor runs at 3GHz, but your thoughts clock in at 2 baud.",
        "If ignorance is bliss, you must be the happiest entity in the universe.",
    ];
    let idx = (get_random_u64() as usize) % roasts.len();
    crate::println!("Roast: {}", roasts[idx]);
}

fn cmd_joke() {
    let jokes = [
        "Why do programmers prefer dark mode? Because light attracts bugs.",
        "There are 10 types of people in the world: those who understand binary, and 9 others who didn't ask for this OS.",
        "A SQL query walks into a bar, walks up to two tables and asks: 'Can I join you?'",
        "Hardware: The part of a computer that you can kick. Software: The part you can only curse at.",
    ];
    let idx = (get_random_u64() as usize) % jokes.len();
    crate::println!("{}", jokes[idx]);
}

fn cmd_excuse() {
    let excuses = [
        "It's not a bug, it's an undocumented feature.",
        "Cosmic radiation flipped a bit in the L1 cache.",
        "Someone else changed the code while I was sleeping.",
        "Works on my machine. Therefore your machine is inferior.",
        "The compiler optimizer got overly creative.",
    ];
    let idx = (get_random_u64() as usize) % excuses.len();
    crate::println!("BOFH Excuse: {}", excuses[idx]);
}

fn cmd_quote() {
    let quotes = [
        "\"Talk is cheap. Show me the code.\" - Linus Torvalds",
        "\"If you think you have a solution, you didn't understand the problem.\" - Terry Davis",
        "\"Simplicity is prerequisite for reliability.\" - Edsger W. Dijkstra",
        "\"There are only two hard things in Computer Science: cache invalidation, naming things, and off-by-one errors.\"",
    ];
    let idx = (get_random_u64() as usize) % quotes.len();
    crate::println!("{}", quotes[idx]);
}

fn cmd_insult() {
    let insults = [
        "Thou art a boil, a plague sore, an embossed carbuncle in my corrupt blood!",
        "You have all the depth of a puddle in Arizona.",
        "Your code belongs in a museum of modern catastrophes.",
        "I'd agree with you, but then we'd both be wrong.",
    ];
    let idx = (get_random_u64() as usize) % insults.len();
    crate::println!("Insult: {}", insults[idx]);
}

fn cmd_art(arg: &str) {
    match arg {
        "skull" => {
            crate::println!("     .ed\"\"\"\"\"\"\"\"\"\"\"\"\"\"e.");
            crate::println!("   .d\"\"\"\"\"\"\"\"\"\"\"\"\"\"\"\"\"\"\"b.");
            crate::println!("  .\"    _..._     _..._   \".");
            crate::println!("  :   .'     '. .'     '.  :");
            crate::println!("  :  :   (o)   :   (o)   : :");
            crate::println!("  :  '.       .' '.       .' :");
            crate::println!("   '.  `\"\"\"\"\"`     `\"\"\"\"\"`  .'");
            crate::println!("     `-.__   .---.   __.-'");
            crate::println!("          `\"\"\"\"\"\"\"\"\"`");
        },
        "clown" => {
            crate::println!("      .-'''''-.");
            crate::println!("    .'  _   _  '.");
            crate::println!("   /   (o) (o)   \\");
            crate::println!("  |       O       |  <-- You right now");
            crate::println!("   \\    \\___/    /");
            crate::println!("    '.         .'");
            crate::println!("      '-.....-'");
        },
        _ => {
            crate::println!("        (  .      )");
            crate::println!("    )           (              )");
            crate::println!("          .  ' '  . [DUMPSTER FIRE] '  . '  .");
            crate::println!("   |=======================================|");
            crate::println!("   |     ANOTHER FUCKING OPERATING SYSTEM  |");
            crate::println!("   |=======================================|");
            crate::println!("Tip: Try 'art skull' or 'art clown'.");
        }
    }
}

fn cmd_coin() {
    let r = get_random_u64() % 2;
    if r == 0 {
        crate::println!("Flipping coin... Heads! You lose.");
    } else {
        crate::println!("Flipping coin... Tails! You also lose.");
    }
}

fn cmd_dice() {
    let roll = (get_random_u64() % 6) + 1;
    crate::println!("Rolled a d6: {}. Luck is an illusion.", roll);
}

fn cmd_guess(arg: &str) {
    if arg.is_empty() {
        crate::println!("Usage: guess <1-10>");
        return;
    }
    let g = parse_int(arg);
    let secret = (get_random_u64() % 10) + 1;
    if g == secret as i64 {
        crate::println!("Miraculously correct ({})! Pure luck, zero skill.", secret);
    } else {
        crate::println!("Wrong! Secret was {}. Better luck next life.", secret);
    }
}

fn cmd_rps(arg: &str) {
    if arg.is_empty() {
        crate::println!("Usage: rps <rock|paper|scissors>");
        return;
    }
    match arg {
        "rock" => crate::println!("I chose Paper. You lose. Sucks to suck."),
        "paper" => crate::println!("I chose Scissors. Sliced in half. You lose."),
        "scissors" => crate::println!("I chose Rock. Crushed. You lose."),
        _ => crate::println!("That's not even a valid move. You lose automatically."),
    }
}

fn cmd_roulette() {
    crate::println!("Spinning the 6-cylinder chamber...");
    let chamber = (get_random_u64() % 6) + 1;
    if chamber == 1 {
        crate::println!("*BANG* You hit the loaded chamber! Resetting CPU now...");
        speaker::beep(200, 8_000_000);
        unsafe {
            asm!("out dx, al", in("dx") 0x64u16, in("al") 0xFEu8);
        }
        loop { unsafe { asm!("hlt"); } }
    } else {
        crate::println!("*Click* Empty chamber (Chamber #{}/6). You survived... for now.", chamber);
    }
}

fn cmd_cookie() {
    unsafe {
        COOKIE_COUNT += 1;
        crate::println!("You clicked the cookie! Total cookies: {}.", COOKIE_COUNT);
        if COOKIE_COUNT == 1 {
            crate::println!("Your journey into meaningless dopamine addiction begins.");
        } else if COOKIE_COUNT == 10 {
            crate::println!("10 cookies. Still haven't accomplished anything meaningful.");
        } else if COOKIE_COUNT >= 20 {
            crate::println!("{} cookies. Stop. Seek help.", COOKIE_COUNT);
        }
    }
}

fn cmd_matrix() {
    crate::println!("Injecting digital rain (Press any key to escape the matrix)...");
    for _ in 0..10 {
        for _ in 0..78 {
            let ch = ((get_random_u64() % 94) + 33) as u8;
            crate::print!("{}", ch as char);
        }
        crate::println!();
    }
    crate::println!("Wake up, Neo... You're still trapped in a 64-bit toy kernel.");
}

fn cmd_sl() {
    crate::println!("      ====        ________                ___________ ");
    crate::println!("  _D _|  |_______/        \\__I_I_____===__|_________| ");
    crate::println!("   |(_)---  |   H\\________/ _____ |   (|) |         | ");
    crate::println!("   /     |  |   H  |  |     |   | |      |         | ");
    crate::println!("  |      |  |   H  |__--------------------|         | ");
    crate::println!("  | ________|___H__/__|_____/[][]~\\_______|_________| ");
    crate::println!("  |/ |   |_____I_____I_____I_____I_____I_____I_____I  ");
    crate::println!("You mistyped 'ls', didn't you? Enjoy the choo choo train of shame.");
}

fn cmd_fire() {
    crate::println!("Simulating burning garbage fire...");
    let chars = [b' ', b'.', b':', b'*', b's', b'S', b'#', b'@'];
    for _ in 0..6 {
        for _ in 0..78 {
            let idx = (get_random_u64() as usize) % chars.len();
            crate::print!("{}", chars[idx] as char);
        }
        crate::println!();
    }
    crate::println!("All your dreams going up in flames.");
}

fn cmd_sleep(arg: &str) {
    let secs = if arg.is_empty() { 2 } else { parse_int(arg) };
    crate::println!("Wasting {} seconds of your finite existence...", secs);
    for s in 0..secs {
        for _ in 0..10_000_000 {
            core::hint::spin_loop();
        }
        crate::println!("... {} second(s) wasted", s + 1);
    }
    crate::println!("Done. You will never get that time back.");
}

fn cmd_typing() {
    crate::println!("Speed typing test: Type 'I regret my life' and hit Enter:");
    crate::print!("> ");
    let mut buf = [0u8; 32];
    let mut len = 0;
    loop {
        let c = Keyboard::get_char_blocking();
        if c == b'\n' || c == b'\r' {
            crate::println!();
            break;
        } else if c >= 32 && c <= 126 && len < 31 {
            buf[len] = c;
            len += 1;
            crate::print!("{}", c as char);
        }
    }
    let input = core::str::from_utf8(&buf[..len]).unwrap_or("");
    if input == "I regret my life" {
        crate::println!("100% accuracy! Acceptance is the first step.");
    } else {
        crate::println!("You failed typing a simple sentence. Expected 'I regret my life', got '{}'.", input);
    }
}

fn cmd_trivia() {
    crate::println!("Trivia Question: What year was the x86 architecture born?");
    crate::println!("  A) 1978 (Intel 8086)");
    crate::println!("  B) 1985 (Intel 386)");
    crate::println!("  C) 1991 (Linux 0.01)");
    crate::print!("Your answer (A, B, or C): ");
    let c = Keyboard::get_char_blocking();
    crate::println!("{}", c as char);
    if c == b'A' || c == b'a' {
        crate::println!("Correct! 1978. We have been suffering legacy baggage for decades.");
    } else {
        crate::println!("Incorrect! The 8086 was released in 1978. Go read a history book.");
    }
}

fn cmd_inb(arg: &str) {
    if arg.is_empty() {
        crate::println!("Usage: inb <port_hex> (e.g. inb 60)");
        return;
    }
    let port = parse_hex(arg) as u16;
    let val: u8;
    unsafe {
        asm!("in al, dx", in("dx") port, out("al") val, options(nomem, nostack));
    }
    crate::println!("inb(0x{:04x}) = 0x{:02x} ({})", port, val, val);
}

fn cmd_outb(arg: &str) {
    let mut parts = arg.split_whitespace();
    let p_str = parts.next().unwrap_or("");
    let v_str = parts.next().unwrap_or("");
    if p_str.is_empty() || v_str.is_empty() {
        crate::println!("Usage: outb <port_hex> <val_hex> (e.g. outb 61 0)");
        return;
    }
    let port = parse_hex(p_str) as u16;
    let val = parse_hex(v_str) as u8;
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack));
    }
    crate::println!("outb(0x{:04x}, 0x{:02x}) executed. Hardware provoked.", port, val);
}

fn parse_hex(s: &str) -> u64 {
    let mut val = 0u64;
    for b in s.bytes() {
        if b >= b'0' && b <= b'9' {
            val = (val << 4) | (b - b'0') as u64;
        } else if b >= b'a' && b <= b'f' {
            val = (val << 4) | (b - b'a' + 10) as u64;
        } else if b >= b'A' && b <= b'F' {
            val = (val << 4) | (b - b'A' + 10) as u64;
        }
    }
    val
}

fn cmd_memleak() {
    let frame = crate::mm::FrameAllocator::allocate_frame();
    crate::println!("Intentionally leaked 4096 bytes at physical address 0x{:08x}!", frame);
    crate::println!("Hope you enjoy watching your available memory evaporate.");
}

fn cmd_run(arg: &str) {
    let elf_data = match arg {
        "hello" => HELLO_ELF,
        "counter" => COUNTER_ELF,
        "shithole" => SHITHOLE_ELF,
        _ => {
            crate::println!("Usage: run <hello | counter | shithole>");
            return;
        }
    };

    crate::println!("[EXEC] Launching native Linux ELF '{}' in Ring 3...", arg);
    match crate::elf::load_elf(elf_data) {
        Ok(loaded) => {
            crate::println!("[EXEC] ELF mapped! Entry: 0x{:08x}, User SP: 0x{:08x}", loaded.entry_point, loaded.user_sp);
            unsafe {
                crate::syscall::CURRENT_BRK = loaded.brk_base;
                jump_to_user_mode(loaded.entry_point, loaded.user_sp);
            }
        }
        Err(e) => crate::println!("[EXEC ERROR] Failed to load ELF: {}", e),
    }
}
