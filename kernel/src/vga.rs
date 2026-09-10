// kernel/src/vga.rs - Color VGA Text Mode (80x25) driver with cursor, colors and scrolling
use core::arch::asm;
use core::fmt;

const VGA_BUFFER: *mut u16 = 0xB8000 as *mut u16;
pub const BUFFER_WIDTH: usize = 80;
pub const BUFFER_HEIGHT: usize = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(dead_code)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    pub const fn new(foreground: Color, background: Color) -> Self {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

pub struct VgaWriter {
    pub col: usize,
    pub row: usize,
    pub color: ColorCode,
}

pub static mut WRITER: VgaWriter = VgaWriter {
    col: 0,
    row: 0,
    color: ColorCode::new(Color::White, Color::Black),
};

impl VgaWriter {
    pub fn clear(&mut self) {
        let blank = 0x20u16 | ((self.color.0 as u16) << 8);
        for i in 0..(BUFFER_WIDTH * BUFFER_HEIGHT) {
            unsafe {
                VGA_BUFFER.add(i).write_volatile(blank);
            }
        }
        self.col = 0;
        self.row = 0;
        self.update_cursor();
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            b'\r' => {
                self.col = 0;
            }
            0x08 => {
                // Backspace
                if self.col > 0 {
                    self.col -= 1;
                    let offset = self.row * BUFFER_WIDTH + self.col;
                    let blank = 0x20u16 | ((self.color.0 as u16) << 8);
                    unsafe {
                        VGA_BUFFER.add(offset).write_volatile(blank);
                    }
                }
            }
            byte => {
                if self.col >= BUFFER_WIDTH {
                    self.new_line();
                }
                let offset = self.row * BUFFER_WIDTH + self.col;
                let char_code = (byte as u16) | ((self.color.0 as u16) << 8);
                unsafe {
                    VGA_BUFFER.add(offset).write_volatile(char_code);
                }
                self.col += 1;
            }
        }
        self.update_cursor();
    }

    pub fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }

    fn new_line(&mut self) {
        self.col = 0;
        if self.row < BUFFER_HEIGHT - 1 {
            self.row += 1;
        } else {
            // Scroll up
            for r in 1..BUFFER_HEIGHT {
                for c in 0..BUFFER_WIDTH {
                    let from = r * BUFFER_WIDTH + c;
                    let to = (r - 1) * BUFFER_WIDTH + c;
                    unsafe {
                        let ch = VGA_BUFFER.add(from).read_volatile();
                        VGA_BUFFER.add(to).write_volatile(ch);
                    }
                }
            }
            // Clear last row
            let blank = 0x20u16 | ((self.color.0 as u16) << 8);
            for c in 0..BUFFER_WIDTH {
                let offset = (BUFFER_HEIGHT - 1) * BUFFER_WIDTH + c;
                unsafe {
                    VGA_BUFFER.add(offset).write_volatile(blank);
                }
            }
        }
    }

    fn update_cursor(&self) {
        let pos = (self.row * BUFFER_WIDTH + self.col) as u16;
        unsafe {
            asm!("out dx, al", in("dx") 0x3D4u16, in("al") 0x0Fu8, options(nomem, nostack));
            asm!("out dx, al", in("dx") 0x3D5u16, in("al") (pos & 0xFF) as u8, options(nomem, nostack));
            asm!("out dx, al", in("dx") 0x3D4u16, in("al") 0x0Eu8, options(nomem, nostack));
            asm!("out dx, al", in("dx") 0x3D5u16, in("al") ((pos >> 8) & 0xFF) as u8, options(nomem, nostack));
        }
    }
}

impl fmt::Write for VgaWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            unsafe {
                let _ = write!($crate::vga::WRITER, $($arg)*);
            }
            $crate::serial_print!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
