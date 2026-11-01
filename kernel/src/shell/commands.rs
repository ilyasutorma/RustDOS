#![allow(static_mut_refs)]
use crate::bios::tty;
use crate::mem::mcb;

static mut ENV: [[u8; 64]; 32] = [[0; 64]; 32];
static mut PATH: [u8; 128] = [0; 128];

pub fn run() -> ! {
    tty::print("\r\nRustDOS 0.0.1\r\n");
    tty::print("Type HELP for commands\r\n");

    unsafe {
        let p = PATH.as_mut_ptr();
        core::ptr::copy_nonoverlapping(b"C:\\".as_ptr(), p, 3);
    }

    loop {
        tty::print("\r\nC:\\>");

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
        set(trim(args));
    } else if eq(cmd, b"PATH") {
        path(trim(args));
    } else if eq(cmd, b"RUN") {
        crate::loader::com::test();
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
    tty::print("\r\n Volume in drive C has no label\r\n");
    tty::print(" Directory of C:\\\r\n");
    tty::print("\r\n");
    tty::print("COMMAND  COM    12,345  01-01-90\r\n");
    tty::print("AUTOEXEC BAT     1,234  01-01-90\r\n");
    tty::print("CONFIG   SYS     2,345  01-01-90\r\n");
    tty::print("        3 file(s)\r\n");
}

fn r#type(name: &[u8]) {
    if name.is_empty() {
        tty::print("Required parameter missing\r\n");
        return;
    }

    if eq(name, b"HELP.TXT") {
        tty::print("\r\nThis is RustDOS.\r\n");
        tty::print("Type HELP for commands.\r\n");
    } else if eq(name, b"README.TXT") {
        tty::print("\r\nWelcome to RustDOS!\r\n");
        tty::print("A DOS-compatible kernel written in Rust.\r\n");
    } else {
        tty::print("File not found\r\n");
    }
}

fn echo(msg: &[u8]) {
    if msg.is_empty() {
        tty::print("ECHO is on\r\n");
        return;
    }

    let mut out = [0u8; 128];
    let n = expand(msg, &mut out);

    for i in 0..n {
        tty::putc(out[i]);
    }

    tty::print("\r\n");
}

fn set(args: &[u8]) {
    if args.is_empty() {
        tty::print("\r\nEnvironment:\r\n");
        unsafe {
            for i in 0..32 {
                let e = &ENV[i];

                if e[0] != 0 {
                    for j in 0..64 {
                        if e[j] == 0 {
                            break;
                        }

                        tty::putc(e[j]);
                    }

                    tty::print("\r\n");
                }
            }
        }

        return;
    }

    let (name, val) = split_eq(args);

    if name.is_empty() {
        return;
    }

    unsafe {
        for i in 0..32 {
            let e = &mut ENV[i];

            if env_eq(e, name) {
                if val.is_empty() {
                    e[0] = 0;
                } else {
                    env_set(e, name, val);
                }

                return;
            }

            if e[0] == 0 {
                env_set(e, name, val);
                return;
            }
        }
    }
}

fn path(args: &[u8]) {
    if args.is_empty() {
        tty::print("\r\nPATH=");
        unsafe {
            let p = PATH.as_ptr();
            let mut i = 0;

            while i < 128 && p.add(i).read() != 0 {
                tty::putc(p.add(i).read());
                i += 1;
            }
        }

        tty::print("\r\n");
        return;
    }

    unsafe {
        let p = PATH.as_mut_ptr();
        let n = args.len().min(127);

        core::ptr::copy_nonoverlapping(args.as_ptr(), p, n);
        p.add(n).write(0);
    }
}

fn expand(src: &[u8], dst: &mut [u8]) -> usize {
    let mut i = 0;
    let mut j = 0;

    while i < src.len() && j < dst.len() {
        if src[i] == b'%' {
            i += 1;

            if i >= src.len() {
                break;
            }

            let start = i;

            while i < src.len() && src[i] != b'%' {
                i += 1;
            }

            if i < src.len() {
                let name = &src[start..i];
                let val = unsafe { env_get(name) };

                for k in 0..val.len() {
                    if j < dst.len() {
                        dst[j] = val[k];
                        j += 1;
                    }
                }

                i += 1;
            }
        } else {
            dst[j] = src[i];
            i += 1;
            j += 1;
        }
    }

    j
}

unsafe fn env_eq(env: &[u8; 64], name: &[u8]) -> bool {
    let mut i = 0;

    while i < name.len() && i < 64 {
        if env[i] != upper(name[i]) {
            return false;
        }

        i += 1;
    }

    i < 64 && env[i] == b'='
}

unsafe fn env_set(env: &mut [u8; 64], name: &[u8], val: &[u8]) {
    let mut i = 0;

    for k in 0..name.len().min(31) {
        env[i] = upper(name[k]);
        i += 1;
    }

    env[i] = b'=';
    i += 1;

    for k in 0..val.len().min(63 - i) {
        env[i] = val[k];
        i += 1;
    }

    env[i] = 0;
}

unsafe fn env_get(name: &[u8]) -> &'static [u8] {
    for i in 0..32 {
        let e = &ENV[i];

        if env_eq(e, name) {
            let mut j = 0;

            while j < 64 && e[j] != b'=' {
                j += 1;
            }

            if j < 64 {
                j += 1;

                let mut k = j;

                while k < 64 && e[k] != 0 {
                    k += 1;
                }

                return core::slice::from_raw_parts(e.as_ptr().add(j), k - j);
            }
        }
    }

    &[]
}

fn split_eq(s: &[u8]) -> (&[u8], &[u8]) {
    let mut i = 0;

    while i < s.len() && s[i] != b'=' {
        i += 1;
    }

    if i < s.len() {
        (&s[..i], &s[i + 1..])
    } else {
        (s, &[])
    }
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
        if upper(a[i]) != b[i] {
            return false;
        }
    }

    true
}

fn upper(c: u8) -> u8 {
    if c >= b'a' && c <= b'z' {
        c - 32
    } else {
        c
    }
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
