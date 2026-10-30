//! Text output via INT 10h (teletype, AH=0Eh).

pub fn putc(c: u8) {
    unsafe {
        core::arch::asm!(
            "int 0x10",
            in("ax") 0x0E00u16 | c as u16,
            in("bx") 0x0007u16,
            options(nostack),
        );
    }
}

pub fn print(s: &str) {
    for b in s.bytes() {
        putc(b);
    }
}
