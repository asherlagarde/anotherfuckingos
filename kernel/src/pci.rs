// kernel/src/pci.rs - PCI Bus Scanner
// Scans PCI Configuration Space (Ports 0xCF8 / 0xCFC) to detect real hardware devices
use core::arch::asm;

const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u8,
    pub subclass: u8,
}

#[inline]
unsafe fn outl(port: u16, val: u32) {
    asm!("out dx, eax", in("dx") port, in("eax") val, options(nomem, nostack, preserves_flags));
}

#[inline]
unsafe fn inl(port: u16) -> u32 {
    let val: u32;
    asm!("in eax, dx", in("dx") port, out("eax") val, options(nomem, nostack, preserves_flags));
    val
}

pub unsafe fn pci_read_32(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let address = (1u32 << 31)
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | ((offset as u32) & 0xFC);
    outl(PCI_CONFIG_ADDRESS, address);
    inl(PCI_CONFIG_DATA)
}

pub fn vendor_name(vendor_id: u16) -> &'static str {
    match vendor_id {
        0x8086 => "Intel Corporation (overpriced silicon)",
        0x10DE => "NVIDIA Corporation (driver nightmare)",
        0x1002 => "AMD/ATI (furnace edition)",
        0x1234 => "QEMU Virtual Hardware (fake box)",
        0x1AF4 => "Red Hat VirtIO (cloud parasite)",
        0x10EC => "Realtek (cheap audio/nic)",
        _ => "Unknown shady vendor",
    }
}

pub fn class_name(class_code: u8, subclass: u8) -> &'static str {
    match class_code {
        0x01 => match subclass {
            0x01 => "IDE Interface",
            0x06 => "SATA Controller",
            _ => "Mass Storage Controller",
        },
        0x02 => "Network Controller (Ethernet)",
        0x03 => "Display Controller (VGA Compatible)",
        0x04 => "Multimedia Controller (Audio)",
        0x06 => match subclass {
            0x00 => "Host Bridge",
            0x01 => "ISA Bridge",
            0x04 => "PCI-to-PCI Bridge",
            _ => "Bridge Device",
        },
        0x0C => "Serial Bus Controller (USB)",
        _ => "Other hardware junk",
    }
}

pub fn scan_bus() -> ([Option<PciDevice>; 16], usize) {
    let mut devices = [None; 16];
    let mut count = 0;

    for dev in 0..32 {
        unsafe {
            let reg0 = pci_read_32(0, dev, 0, 0);
            let vendor_id = (reg0 & 0xFFFF) as u16;
            let device_id = ((reg0 >> 16) & 0xFFFF) as u16;

            if vendor_id != 0xFFFF && vendor_id != 0x0000 {
                let reg8 = pci_read_32(0, dev, 0, 8);
                let class_code = ((reg8 >> 24) & 0xFF) as u8;
                let subclass = ((reg8 >> 16) & 0xFF) as u8;

                if count < devices.len() {
                    devices[count] = Some(PciDevice {
                        bus: 0,
                        device: dev,
                        function: 0,
                        vendor_id,
                        device_id,
                        class_code,
                        subclass,
                    });
                    count += 1;
                }
            }
        }
    }
    (devices, count)
}
