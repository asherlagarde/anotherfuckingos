// kernel/src/rtc.rs - CMOS Real-Time Clock (RTC) Driver
// Reads physical hardware real-time clock from I/O ports 0x70 / 0x71
use core::arch::asm;

const CMOS_ADDR: u16 = 0x70;
const CMOS_DATA: u16 = 0x71;

#[derive(Debug, Clone, Copy)]
pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

#[inline]
unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
}

#[inline]
unsafe fn inb(port: u16) -> u8 {
    let val: u8;
    asm!("in al, dx", in("dx") port, out("al") val, options(nomem, nostack, preserves_flags));
    val
}

unsafe fn get_update_in_progress_flag() -> bool {
    outb(CMOS_ADDR, 0x0A);
    (inb(CMOS_DATA) & 0x80) != 0
}

unsafe fn read_register(reg: u8) -> u8 {
    outb(CMOS_ADDR, reg);
    inb(CMOS_DATA)
}

fn bcd_to_bin(bcd: u8) -> u8 {
    (bcd & 0x0F) + ((bcd >> 4) * 10)
}

impl DateTime {
    pub fn now() -> Self {
        unsafe {
            // Wait for update in progress to clear
            while get_update_in_progress_flag() {
                core::hint::spin_loop();
            }

            let mut second = read_register(0x00);
            let mut minute = read_register(0x02);
            let mut hour = read_register(0x04);
            let mut day = read_register(0x07);
            let mut month = read_register(0x08);
            let mut year = read_register(0x09) as u16;
            let century = read_register(0x32);
            let register_b = read_register(0x0B);

            // Convert BCD to binary if needed (bit 2 of reg B is 0 for BCD)
            if (register_b & 0x04) == 0 {
                second = bcd_to_bin(second);
                minute = bcd_to_bin(minute);
                hour = ((hour & 0x0F) + (((hour & 0x70) >> 4) * 10)) | (hour & 0x80);
                day = bcd_to_bin(day);
                month = bcd_to_bin(month);
                year = bcd_to_bin(year as u8) as u16;
            }

            // Convert 12 hour to 24 hour if needed
            if (register_b & 0x02) == 0 && (hour & 0x80) != 0 {
                hour = ((hour & 0x7F) + 12) % 24;
            }

            // Calculate full 4-digit year
            let full_year = if century > 0 && century != 0xFF {
                let cent = if (register_b & 0x04) == 0 { bcd_to_bin(century) } else { century };
                (cent as u16 * 100) + year
            } else {
                2000 + year
            };

            DateTime {
                year: full_year,
                month,
                day,
                hour,
                minute,
                second,
            }
        }
    }

    /// Approximate Unix timestamp (seconds since 1970-01-01 00:00:00 UTC)
    pub fn unix_timestamp(&self) -> u64 {
        let y = self.year as u64;
        let m = self.month as u64;
        let d = self.day as u64;

        // Days from 1970 to beginning of current year (with leap year approximation)
        let mut days = (y - 1970) * 365 + ((y - 1969) / 4);

        // Days in months for non-leap year
        let days_in_months = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        for i in 1..m {
            if (i as usize) < days_in_months.len() {
                days += days_in_months[i as usize];
            }
        }
        if m > 2 && (y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)) {
            days += 1;
        }
        days += d - 1;

        days * 86400 + (self.hour as u64) * 3600 + (self.minute as u64) * 60 + (self.second as u64)
    }
}
