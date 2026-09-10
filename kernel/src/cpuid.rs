// kernel/src/cpuid.rs - x86 CPUID instruction detector & silicon inspector
use core::arch::asm;

pub struct CpuInfo {
    pub vendor: [u8; 13],
    pub has_sse: bool,
    pub has_sse2: bool,
    pub has_sse3: bool,
    pub has_avx: bool,
    pub has_rdrand: bool,
    pub is_hypervisor: bool,
    pub max_leaf: u32,
}

#[inline]
pub unsafe fn cpuid(leaf: u32, subleaf: u32) -> (u32, u32, u32, u32) {
    let eax: u32;
    let ebx: u32;
    let ecx: u32;
    let edx: u32;
    asm!(
        "push rbx",
        "cpuid",
        "mov {0:e}, ebx",
        "pop rbx",
        out(reg) ebx,
        inout("eax") leaf => eax,
        inout("ecx") subleaf => ecx,
        lateout("edx") edx,
        options(preserves_flags)
    );
    (eax, ebx, ecx, edx)
}

impl CpuInfo {
    pub fn get() -> Self {
        unsafe {
            // Leaf 0: Max leaf & Vendor string
            let (max_leaf, ebx, ecx, edx) = cpuid(0, 0);
            let mut vendor = [0u8; 13];
            vendor[0..4].copy_from_slice(&ebx.to_le_bytes());
            vendor[4..8].copy_from_slice(&edx.to_le_bytes());
            vendor[8..12].copy_from_slice(&ecx.to_le_bytes());
            vendor[12] = 0;

            // Leaf 1: Feature flags
            let (_eax, _ebx, ecx1, edx1) = if max_leaf >= 1 {
                cpuid(1, 0)
            } else {
                (0, 0, 0, 0)
            };

            let has_sse = (edx1 & (1 << 25)) != 0;
            let has_sse2 = (edx1 & (1 << 26)) != 0;
            let has_sse3 = (ecx1 & (1 << 0)) != 0;
            let has_avx = (ecx1 & (1 << 28)) != 0;
            let has_rdrand = (ecx1 & (1 << 30)) != 0;
            let is_hypervisor = (ecx1 & (1 << 31)) != 0;

            CpuInfo {
                vendor,
                has_sse,
                has_sse2,
                has_sse3,
                has_avx,
                has_rdrand,
                is_hypervisor,
                max_leaf,
            }
        }
    }

    pub fn vendor_str(&self) -> &str {
        core::str::from_utf8(&self.vendor[..12]).unwrap_or("Unknown")
    }

    pub fn honest_roast(&self) -> &'static str {
        let v = self.vendor_str();
        if self.is_hypervisor {
            "Running inside a VM/Hypervisor. Afraid of bare metal or just broke?"
        } else if v.starts_with("GenuineIntel") {
            "Intel silicon detected. Enjoy your high power bill and speculative vulnerabilities."
        } else if v.starts_with("AuthenticAMD") {
            "AMD silicon detected. It's either thermal throttling or compiling Rust right now."
        } else {
            "Some sketchy knock-off CPU. We don't even know how this is executing instructions."
        }
    }
}

/// Hardware Random Number generator using RDRAND or TSC fallback
pub fn get_random_u64() -> u64 {
    let mut val: u64;
    let success: u8;
    unsafe {
        asm!(
            "rdrand {0}",
            "setc {1}",
            out(reg) val,
            out(reg_byte) success,
            options(nomem, nostack)
        );
    }
    if success == 1 {
        val
    } else {
        // Fallback: Read Time-Stamp Counter (RDTSC) with XOR shift
        let tsc: u64;
        unsafe {
            let low: u32;
            let high: u32;
            asm!("rdtsc", out("eax") low, out("edx") high);
            tsc = ((high as u64) << 32) | (low as u64);
        }
        let mut x = tsc.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        x
    }
}
