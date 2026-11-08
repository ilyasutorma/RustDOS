#![allow(static_mut_refs)]

use crate::bios::tty;
use crate::mem::mcb;
use crate::shell::env;

static mut CURRENT_DRIVE: u8 = 0;

pub fn run() -> ! {
    tty::print("\r\nRustDOS 0.0.1\r\n");
    tty::print("Type HELP for commands\r\n");

    unsafe {
        env::init_path();
    }

    loop {
        unsafe {
            if CURRENT_DRIVE == 0 {
                tty::print("\r\nA:\\>");
            } else {
                tty::print("\r\nB:\\>");
            }
        }

        let mut buf = [0u8; 127];
        let n = read_line(&mut buf);

        tty::print("\r\n");

        exec(&buf[..n]);
    }
}

fn read_line(buf: &mut [u8]) -> usize {
    let mut i = 0;

    loop {
        let c = crate::bios::keyboard::read_char();

        if c == 13 {
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

fn exec(line: &[u8]) {
    let line = trim(line);

    if line.is_empty() {
        return;
    }

    let (cmd, args) = token(line);

    if eq(cmd, b"A:") {
        unsafe { CURRENT_DRIVE = 0; }
        return;
    }

    if eq(cmd, b"B:") {
        unsafe { CURRENT_DRIVE = 1; }
        return;
    }

    if eq(cmd, b"HELP") {
        help();
    } else if eq(cmd, b"VER") {
        ver();
    } else if eq(cmd, b"MEM") {
        mem();
    } else if eq(cmd, b"CLS") {
        cls();
    } else if eq(cmd, b"DIR") {
        dir();
    } else if eq(cmd, b"TYPE") {
        r#type(trim(args));
    } else if eq(cmd, b"ECHO") {
        echo(trim(args));
    } else if eq(cmd, b"SET") {
        unsafe {
            if args.is_empty() {
                env::print_env();
            } else {
                env::set_env(trim(args));
            }
        }
    } else if eq(cmd, b"PATH") {
        unsafe {
            if args.is_empty() {
                env::print_path();
            } else {
                env::set_path(trim(args));
            }
        }
    } else if eq(cmd, b"RUN") {
        run_cmd(trim(args));
    } else {
        tty::print("Bad command or file name\r\n");
    }
}

fn help() {
    tty::print("Available commands:\r\n");
    tty::print("  HELP     - Show this help\r\n");
    tty::print("  VER      - Display version\r\n");
    tty::print("  MEM      - Display memory info\r\n");
    tty::print("  CLS      - Clear screen\r\n");
    tty::print("  DIR      - List directory\r\n");
    tty::print("  TYPE     - Display file contents\r\n");
    tty::print("  ECHO     - Display message\r\n");
    tty::print("  SET      - Display/set environment\r\n");
    tty::print("  PATH     - Display/set search path\r\n");
    tty::print("  A:       - Switch to drive A\r\n");
    tty::print("  B:       - Switch to drive B\r\n");
    tty::print("  RUN      - Run test program\r\n");
}

fn ver() {
    tty::print("\r\nRustDOS Version 0.0.1\r\n");
    tty::print("Compatible with FreeDOS\r\n");
}

fn mem() {
    unsafe {
        let p = mcb::largest();

        tty::print("\r\nMemory type:  Conventional\r\n");
        tty::print("Free:         ");
        print_u16(p);
        tty::print(" paragraphs\r\n");
        tty::print("              ");
        print_u16(p / 64);
        tty::print(" KB\r\n");
    }
}

fn cls() {
    unsafe {
        core::arch::asm!(
            "int 0x10",
            in("ax") 0x0600u16,
                         in("bx") 0x0700u16,
                         in("cx") 0x0000u16,
                         in("dx") 0x184Fu16
        );
    }
}

fn dir() {
    unsafe {
        let d = CURRENT_DRIVE;

        if !crate::fs::fat::init(d) {
            tty::print("Invalid drive\r\n");
            return;
        }

        crate::fs::fat::list_root();
    }
}

fn r#type(name: &[u8]) {
    if name.is_empty() {
        tty::print("Required parameter missing\r\n");
        return;
    }

    unsafe {
        let d = CURRENT_DRIVE;

        if !crate::fs::fat::init(d) {
            tty::print("Invalid drive\r\n");
            return;
        }

        crate::fs::file::type_file(name);
    }
}

fn echo(msg: &[u8]) {
    if msg.is_empty() {
        tty::print("ECHO is on\r\n");
        return;
    }

    let mut out = [0u8; 128];
    let n = unsafe { env::expand(msg, &mut out) };

    for i in 0..n {
        tty::putc(out[i]);
    }

    tty::print("\r\n");
}

fn trim(s: &[u8]) -> &[u8] {
    let mut a = 0;

    while a < s.len() && s[a] == b' ' {
        a += 1;
    }

    let mut b = s.len();

    while b > a && s[b - 1] == b' ' {
        b -= 1;
    }

    &s[a..b]
}

fn token(s: &[u8]) -> (&[u8], &[u8]) {
    let mut i = 0;

    while i < s.len() && s[i] != b' ' {
        i += 1;
    }

    (&s[..i], &s[i..])
}

fn eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    for i in 0..a.len() {
        if env::upper(a[i]) != b[i] {
            return false;
        }
    }

    true
}

fn print_u16(mut n: u16) {
    if n == 0 {
        tty::putc(b'0');
        return;
    }

    let mut buf = [0u8; 5];
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
fn run_cmd(args: &[u8]) {
    if args.is_empty() {
        crate::loader::com::test();
        return;
    }

    unsafe {
        let d = CURRENT_DRIVE;

        if !crate::fs::fat::init(d) {
            tty::print("Invalid drive\r\n");
            return;
        }

        if !crate::loader::com::run_from_disk(args) {
            tty::print("File not found\r\n");
        }
    }
}
