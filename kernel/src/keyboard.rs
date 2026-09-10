// kernel/src/keyboard.rs - PS/2 Keyboard Controller & ring buffer
use core::arch::asm;

const KEYBOARD_DATA_PORT: u16 = 0x60;
const KEYBOARD_STATUS_PORT: u16 = 0x64;

#[inline]
unsafe fn inb(port: u16) -> u8 {
    let val: u8;
    asm!("in al, dx", in("dx") port, out("al") val, options(nomem, nostack, preserves_flags));
    val
}

// US QWERTY Scancode Set 1 table
static SCANCODES: [u8; 128] = {
    let mut table = [0u8; 128];
    table[1] = 27; // ESC
    table[2] = b'1'; table[3] = b'2'; table[4] = b'3'; table[5] = b'4';
    table[6] = b'5'; table[7] = b'6'; table[8] = b'7'; table[9] = b'8';
    table[10] = b'9'; table[11] = b'0'; table[12] = b'-'; table[13] = b'=';
    table[14] = 0x08; // Backspace
    table[15] = b'\t';
    table[16] = b'q'; table[17] = b'w'; table[18] = b'e'; table[19] = b'r';
    table[20] = b't'; table[21] = b'y'; table[22] = b'u'; table[23] = b'i';
    table[24] = b'o'; table[25] = b'p'; table[26] = b'['; table[27] = b']';
    table[28] = b'\n';
    table[30] = b'a'; table[31] = b's'; table[32] = b'd'; table[33] = b'f';
    table[34] = b'g'; table[35] = b'h'; table[36] = b'j'; table[37] = b'k';
    table[38] = b'l'; table[39] = b';'; table[40] = b'\''; table[41] = b'`';
    table[43] = b'\\';
    table[44] = b'z'; table[45] = b'x'; table[46] = b'c'; table[47] = b'v';
    table[48] = b'b'; table[49] = b'n'; table[50] = b'm'; table[51] = b',';
    table[52] = b'.'; table[53] = b'/';
    table[57] = b' '; // Space
    table
};

pub struct Keyboard;

impl Keyboard {
    pub fn has_key() -> bool {
        unsafe { (inb(KEYBOARD_STATUS_PORT) & 1) != 0 }
    }

    pub fn read_char() -> Option<u8> {
        // 1. Check COM1 serial port for terminal input
        unsafe {
            if (inb(0x3FD) & 0x01) != 0 {
                let byte = inb(0x3F8);
                if byte == b'\r' {
                    return Some(b'\n');
                }
                return Some(byte);
            }
        }

        // 2. Check PS/2 keyboard controller
        if !Self::has_key() {
            return None;
        }
        let scancode = unsafe { inb(KEYBOARD_DATA_PORT) };
        if scancode < 128 {
            let ch = SCANCODES[scancode as usize];
            if ch != 0 {
                return Some(ch);
            }
        }
        None
    }

    pub fn get_char_blocking() -> u8 {
        loop {
            if let Some(c) = Self::read_char() {
                return c;
            }
            core::hint::spin_loop();
        }
    }
}
