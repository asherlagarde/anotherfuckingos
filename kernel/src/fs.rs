pub const MAX_FILES: usize = 16;
pub const MAX_FILE_SIZE: usize = 512;
pub const MAX_NAME_LEN: usize = 32;

#[derive(Clone, Copy)]
pub struct VirtualFile {
    pub name: [u8; MAX_NAME_LEN],
    pub name_len: usize,
    pub data: [u8; MAX_FILE_SIZE],
    pub data_len: usize,
    pub is_dir: bool,
    pub is_readonly: bool,
}

pub struct RamFs {
    pub files: [Option<VirtualFile>; MAX_FILES],
}

pub static mut VFS: RamFs = RamFs::new();

impl RamFs {
    pub const fn new() -> Self {
        RamFs {
            files: [None; MAX_FILES],
        }
    }

    pub fn init(&mut self) {
        self.add_file("etc/motd", b"Welcome to AnotherFuckingOS (v6.6.0-rust-abi)!\nWhy are you here? Go outside. Touch grass.\nThere is nothing for you here except bugs and disappointment.\n", true);
        self.add_file("etc/os-release", b"NAME=\"AnotherFuckingOS\"\nID=anotherfuckingos\nVERSION_ID=\"6.6.0\"\nPRETTY_NAME=\"AnotherFuckingOS (x86_64 Long Mode)\"\nSUPPORT_URL=\"https://google.com?q=how+did+my+life+come+to+this\"\n", true);
        self.add_file("proc/version", b"Linux version 6.6.0-rust-abi #1 SMP PREEMPT Scratch OS 2026 x86_64 GNU/Linux\n", true);
        self.add_file("proc/cpuinfo", b"processor\t: 0\nvendor_id\t: RealOrVirtualSilicon\nmodel name\t: Silicon Trapped in Regret\nbogomips\t: 0.0001\nflags\t\t: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 sse3 avx rdrand hypervisor\n", true);
        self.add_file("proc/meminfo", b"MemTotal:        65536 kB\nMemFree:         48212 kB\nMemAvailable:    48212 kB\nBuffers:             0 kB\nCached:              0 kB\nSwapTotal:           0 kB\nSwapFree:            0 kB\nComment:         You are wasting all of it.\n", true);
        self.add_file("home/regrets.txt", b"1. Staying up past 2 AM writing an OS from scratch.\n2. Choosing Rust instead of getting 8 hours of sleep.\n3. Believing POSIX was a sane standard.\n4. Trusting compiler builtins.\n", false);
    }

    pub fn add_file(&mut self, name: &str, content: &[u8], readonly: bool) -> bool {
        // Find if file already exists
        for slot in self.files.iter_mut() {
            if let Some(ref mut f) = slot {
                if f.name_str() == name {
                    if f.is_readonly {
                        return false;
                    }
                    let copy_len = core::cmp::min(content.len(), MAX_FILE_SIZE);
                    f.data[..copy_len].copy_from_slice(&content[..copy_len]);
                    f.data_len = copy_len;
                    return true;
                }
            }
        }

        // Find empty slot
        for slot in self.files.iter_mut() {
            if slot.is_none() {
                let mut f = VirtualFile {
                    name: [0u8; MAX_NAME_LEN],
                    name_len: 0,
                    data: [0u8; MAX_FILE_SIZE],
                    data_len: 0,
                    is_dir: false,
                    is_readonly: readonly,
                };
                let n_len = core::cmp::min(name.len(), MAX_NAME_LEN);
                f.name[..n_len].copy_from_slice(&name.as_bytes()[..n_len]);
                f.name_len = n_len;

                let c_len = core::cmp::min(content.len(), MAX_FILE_SIZE);
                f.data[..c_len].copy_from_slice(&content[..c_len]);
                f.data_len = c_len;

                *slot = Some(f);
                return true;
            }
        }
        false
    }

    pub fn get_file(&self, name: &str) -> Option<&VirtualFile> {
        for slot in self.files.iter() {
            if let Some(ref f) = slot {
                if f.name_str() == name {
                    return Some(f);
                }
            }
        }
        None
    }

    pub fn delete_file(&mut self, name: &str) -> Result<(), &'static str> {
        for slot in self.files.iter_mut() {
            if let Some(ref f) = slot {
                if f.name_str() == name {
                    if f.is_readonly {
                        return Err("Permission denied: You can't delete system files, you anarchist.");
                    }
                    *slot = None;
                    return Ok(());
                }
            }
        }
        Err("File not found: It doesn't exist, just like your social life.")
    }
}

impl VirtualFile {
    pub fn name_str(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len]).unwrap_or("???")
    }

    pub fn content_str(&self) -> &str {
        core::str::from_utf8(&self.data[..self.data_len]).unwrap_or("<binary junk>")
    }
}
