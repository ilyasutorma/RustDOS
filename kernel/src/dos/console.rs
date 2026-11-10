//! Console services (INT 21h AH=01..0C) and the CON device.

#![allow(static_mut_refs)]

use crate::arch::farmem::{rd8, wr8};
use crate::arch::regs::Regs;
use crate::bios::{keyboard, tty};

static mut LINE: [u8; 130] = [0; 130];
static mut LPOS: usize = 0;
static mut LLEN: usize = 0;

/// Read a line with echo and backspace editing; returns its length (no CR).
pub fn read_line_echo(buf: &mut [u8]) -> usize {
    let mut i = 0;

    loop {
        let c = keyboard::read_char();

        if c == 13 {
            tty::print("\r\n");
            return i;
        }

        if c == 8 {
            if i > 0 {
                i -= 1;
                tty::print("\x08 \x08");
            }

            continue;
        }

        if i < buf.len() {
            buf[i] = c;
            i += 1;
            tty::putc(c);
        }
    }
}

/// `read()` on the console handle: delivers whole lines terminated by CR LF.
pub unsafe fn read_stdin(seg: u16, off: u16, count: u16) -> u16 {
    if LPOS >= LLEN {
        let n = read_line_echo(&mut LINE[..127]);

        LINE[n] = 13;
        LINE[n + 1] = 10;
        LLEN = n + 2;
        LPOS = 0;
    }

    let mut k: u16 = 0;

    while k < count && LPOS < LLEN {
        wr8(seg, off.wrapping_add(k), LINE[LPOS]);
        LPOS += 1;
        k += 1;
    }

    k
}

pub unsafe fn write_stdout(seg: u16, off: u16, count: u16) {
    for i in 0..count {
        tty::putc(rd8(seg, off.wrapping_add(i)));
    }
}

/// AH=01h: read a character with echo.
pub fn char_in_echo(regs: &mut Regs) {
    let c = keyboard::read_char();
    tty::putc(c);
    regs.ax = (regs.ax & 0xFF00) | c as u16;
}

/// AH=02h: write DL.
pub fn char_out(regs: &mut Regs) {
    tty::putc(regs.dx as u8);
    regs.ax = (regs.ax & 0xFF00) | (regs.dx & 0xFF);
}

/// AH=06h: direct console I/O.
pub fn direct_io(regs: &mut Regs) {
    if regs.dx as u8 == 0xFF {
        if keyboard::key_ready() {
            let c = keyboard::read_char();
            regs.ax = (regs.ax & 0xFF00) | c as u16;
            regs.flags &= !0x40; // ZF = 0: a character is returned
        } else {
            regs.ax &= 0xFF00;
            regs.flags |= 0x40; // ZF = 1: nothing ready
        }
    } else {
        tty::putc(regs.dx as u8);
    }
}

/// AH=07h / 08h: read a character without echo.
pub fn char_in(regs: &mut Regs) {
    let c = keyboard::read_char();
    regs.ax = (regs.ax & 0xFF00) | c as u16;
}

/// AH=09h: print a `$`-terminated string at DS:DX.
pub unsafe fn print_string(regs: &Regs) {
    let mut off = regs.dx;

    loop {
        let c = rd8(regs.ds, off);

        if c == b'$' {
            break;
        }

        tty::putc(c);
        off = off.wrapping_add(1);
    }
}

/// AH=0Ah: buffered keyboard input into DS:DX ([max][len][chars...CR]).
pub unsafe fn buffered_input(regs: &mut Regs) {
    let seg = regs.ds;
    let off = regs.dx;
    let max = rd8(seg, off) as usize;

    if max == 0 {
        return;
    }

    let mut tmp = [0u8; 255];
    let cap = if max - 1 < 254 { max - 1 } else { 254 };
    let n = read_line_echo(&mut tmp[..cap]);

    wr8(seg, off.wrapping_add(1), n as u8);

    for i in 0..n {
        wr8(seg, off.wrapping_add(2 + i as u16), tmp[i]);
    }

    wr8(seg, off.wrapping_add(2 + n as u16), 13);
}

/// AH=0Bh: AL = FF if a key is waiting.
pub fn input_status(regs: &mut Regs) {
    let v: u16 = if keyboard::key_ready() { 0xFF } else { 0x00 };
    regs.ax = (regs.ax & 0xFF00) | v;
}
