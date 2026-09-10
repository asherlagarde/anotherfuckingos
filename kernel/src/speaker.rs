// kernel/src/speaker.rs - PC Speaker driver (PIT Channel 2, Port 0x61)
// Emits physical audio tones and beep sounds through the motherboard speaker
use core::arch::asm;

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

pub fn play_sound(n_frequence: u32) {
    if n_frequence == 0 {
        return;
    }
    let div = (1193180 / n_frequence) as u16;
    unsafe {
        // Set PIT channel 2 to square wave mode
        outb(0x43, 0xB6);
        outb(0x42, (div & 0xFF) as u8);
        outb(0x42, (div >> 8) as u8);

        // Turn on speaker and PIT 2 gate
        let tmp = inb(0x61);
        if tmp != (tmp | 3) {
            outb(0x61, tmp | 3);
        }
    }
}

pub fn nosound() {
    unsafe {
        let tmp = inb(0x61) & 0xFC;
        outb(0x61, tmp);
    }
}

pub fn beep(freq: u32, duration_loops: u32) {
    play_sound(freq);
    for _ in 0..duration_loops {
        core::hint::spin_loop();
    }
    nosound();
}

pub fn jingle() {
    // Annoying 3-tone jingle
    beep(440, 5_000_000);
    beep(554, 5_000_000);
    beep(659, 8_000_000);
}
