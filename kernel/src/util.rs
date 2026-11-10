//! Small shared helpers: ASCII and number printing.

use crate::bios::tty;

pub fn upper(c: u8) -> u8 {
    if c >= b'a' && c <= b'z' {
        c - 32
    } else {
        c
    }
}

pub fn print_u32(mut n: u32) {
    if n == 0 {
        tty::putc(b'0');
        return;
    }

    let mut buf = [0u8; 10];
    let mut i = 0;

    while n > 0 {
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        tty::putc(buf[i]);
    }
}

pub fn print_u16(n: u16) {
    print_u32(n as u32);
}

fn digits(mut n: u32) -> usize {
    let mut d = 1;

    while n >= 10 {
        n /= 10;
        d += 1;
    }

    d
}

/// Right-aligned decimal in a field of `width` characters.
pub fn print_u32_pad(n: u32, width: usize) {
    let d = digits(n);

    for _ in d..width {
        tty::putc(b' ');
    }

    print_u32(n);
}

/// Two-digit zero-padded number.
pub fn print_2d(n: u16) {
    tty::putc(b'0' + ((n / 10) % 10) as u8);
    tty::putc(b'0' + (n % 10) as u8);
}
