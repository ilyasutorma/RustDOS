#![allow(static_mut_refs)]

use crate::arch::farmem::ds_seg;
use crate::bios::tty;
use crate::fs::vol;
use crate::mem::mcb;
use crate::shell::env;
use crate::util::{print_2d, print_u16, print_u32, print_u32_pad, upper};

static mut COPYBUF: [u8; 512] = [0; 512];

/// Banner + command loop.
pub fn run() -> ! {
    tty::print("\r\nRustDOS 0.0.1\r\n");
    tty::print("Type HELP for commands\r\n");

    unsafe {
        env::init_path();
    }

    repl()
}

/// Command loop (also re-entered when a program terminates).
pub fn repl() -> ! {
    loop {
        prompt();

        let mut buf = [0u8; 127];
        let n = read_line(&mut buf);

        tty::print("\r\n");

        exec(&buf[..n]);
    }
}

fn prompt() {
    tty::print("\r\nA:\\");

    unsafe {
        if vol::mounted() {
            let mut cwd = [0u8; 64];
            let n = vol::cwd_string(&mut cwd);

            for i in 0..n {
                tty::putc(cwd[i]);
            }
        }
    }

    tty::putc(b'>');
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
        return;
    }

    if cmd.len() == 2 && cmd[1] == b':' {
        tty::print("Invalid drive specification\r\n");
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
        unsafe { dir(trim(args)) };
    } else if eq(cmd, b"TYPE") {
        unsafe { r#type(trim(args)) };
    } else if eq(cmd, b"COPY") {
        unsafe { copy(trim(args)) };
    } else if eq(cmd, b"DEL") || eq(cmd, b"ERASE") {
        unsafe { del(trim(args)) };
    } else if eq(cmd, b"REN") || eq(cmd, b"RENAME") {
        unsafe { ren(trim(args)) };
    } else if eq(cmd, b"MD") || eq(cmd, b"MKDIR") {
        unsafe { md(trim(args)) };
    } else if eq(cmd, b"RD") || eq(cmd, b"RMDIR") {
        unsafe { rd(trim(args)) };
    } else if eq(cmd, b"CD") || eq(cmd, b"CHDIR") {
        unsafe { cd(trim(args)) };
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
    } else if eq(cmd, b"RUN") && args.is_empty() {
        crate::loader::com::test();
    } else {
        let (c, a) = if eq(cmd, b"RUN") { token(trim(args)) } else { (cmd, args) };

        if !unsafe { run_external(c, a) } {
            tty::print("Bad command or file name\r\n");
        }
    }
}

fn help() {
    tty::print("Available commands:\r\n");
    tty::print("  HELP     - Show this help\r\n");
    tty::print("  VER      - Display version\r\n");
    tty::print("  MEM      - Display memory info\r\n");
    tty::print("  CLS      - Clear screen\r\n");
    tty::print("  DIR      - List files (wildcards allowed)\r\n");
    tty::print("  TYPE     - Display file contents\r\n");
    tty::print("  COPY     - Copy a file\r\n");
    tty::print("  DEL      - Delete files (ERASE)\r\n");
    tty::print("  REN      - Rename a file (RENAME)\r\n");
    tty::print("  MD, RD   - Make / remove a directory\r\n");
    tty::print("  CD       - Change / show directory\r\n");
    tty::print("  ECHO     - Display message\r\n");
    tty::print("  SET      - Display/set environment\r\n");
    tty::print("  PATH     - Display/set search path\r\n");
    tty::print("  RUN      - Run a program (no argument: built-in test)\r\n");
    tty::print("  NAME     - Run NAME.COM from the current directory or PATH\r\n");
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

// ------------------------------------------------------------ file commands

fn dos_error(code: u16) {
    match code {
        vol::E_INVALID => tty::print("Invalid function\r\n"),
        vol::E_FNF => tty::print("File not found\r\n"),
        vol::E_PNF => tty::print("Path not found\r\n"),
        vol::E_TOOMANY => tty::print("Too many open files\r\n"),
        vol::E_ACCESS => tty::print("Access denied\r\n"),
        vol::E_NOMEM => tty::print("Insufficient memory\r\n"),
        vol::E_BADFMT => tty::print("Invalid program format (EXE is not supported yet)\r\n"),
        vol::E_DRIVE => tty::print("Invalid drive specification\r\n"),
        vol::E_RMCWD => tty::print("Attempt to remove current directory\r\n"),
        _ => {
            tty::print("Error ");
            print_u16(code);
            tty::print("\r\n");
        }
    }
}

unsafe fn need_fs() -> bool {
    if !vol::mounted() {
        tty::print("No filesystem\r\n");
        return false;
    }

    true
}

fn print_ent(e: &vol::Ent) {
    if e.name[0] == b'.' {
        let mut n = 0;

        while n < 11 && e.name[n] == b'.' {
            tty::putc(b'.');
            n += 1;
        }

        for _ in n..13 {
            tty::putc(b' ');
        }
    } else {
        for i in 0..8 {
            tty::putc(e.name[i]);
        }

        tty::putc(b' ');

        for i in 8..11 {
            tty::putc(e.name[i]);
        }

        tty::putc(b' ');
    }

    if e.attr & vol::ATTR_DIR != 0 {
        tty::print("     <DIR>");
    } else {
        print_u32_pad(e.size, 10);
    }

    tty::print("  ");
    print_2d((e.date >> 5) & 15);
    tty::putc(b'-');
    print_2d(e.date & 31);
    tty::putc(b'-');
    print_2d(((e.date >> 9) + 80) % 100);
    tty::print("  ");
    print_2d(e.time >> 11);
    tty::putc(b':');
    print_2d((e.time >> 5) & 63);
    tty::print("\r\n");
}

unsafe fn dir(args: &[u8]) {
    if !need_fs() {
        return;
    }

    let spec: &[u8] = if args.is_empty() { b"*.*" } else { args };

    tty::print(" Volume in drive A has no label\r\n Directory of A:\\");

    // Show the directory part of the spec, or the current directory.
    let mut cut = 0;

    for i in 0..spec.len() {
        if spec[i] == b'\\' || spec[i] == b'/' {
            cut = i + 1;
        }
    }

    if cut > 0 {
        for i in 0..cut {
            tty::putc(upper(spec[i]));
        }
    } else {
        let mut cwd = [0u8; 64];
        let n = vol::cwd_string(&mut cwd);

        for i in 0..n {
            tty::putc(cwd[i]);
        }
    }

    tty::print("\r\n\r\n");

    let (mut st, first) = match vol::find_first(spec, vol::ATTR_DIR) {
        Ok(v) => v,
        Err(e) => {
            dos_error(e);
            return;
        }
    };

    let mut files = 0u32;
    let mut dirs = 0u32;
    let mut bytes = 0u32;
    let mut cur = Some(first);

    while let Some(e) = cur {
        print_ent(&e);

        if e.attr & vol::ATTR_DIR != 0 {
            dirs += 1;
        } else {
            files += 1;
            bytes += e.size;
        }

        cur = vol::find_next(&mut st);
    }

    tty::print("      ");
    print_u32_pad(files, 4);
    tty::print(" file(s) ");
    print_u32_pad(bytes, 10);
    tty::print(" bytes\r\n      ");
    print_u32_pad(dirs, 4);
    tty::print(" dir(s)  ");
    print_u32_pad(vol::free_clusters() as u32 * vol::bytes_per_cluster(), 10);
    tty::print(" bytes free\r\n");
}

unsafe fn r#type(name: &[u8]) {
    if !need_fs() {
        return;
    }

    if name.is_empty() {
        tty::print("Required parameter missing\r\n");
        return;
    }

    let mut f = match vol::open_file(name, false) {
        Ok(f) => f,
        Err(e) => return dos_error(e),
    };

    let off = COPYBUF.as_ptr() as usize as u16;

    loop {
        let n = match vol::read(&mut f, ds_seg(), off, 512) {
            Ok(n) => n,
            Err(e) => return dos_error(e),
        };

        if n == 0 {
            break;
        }

        for i in 0..n as usize {
            tty::putc(COPYBUF[i]);
        }
    }
}

fn basename(p: &[u8]) -> &[u8] {
    let mut s = 0;

    for i in 0..p.len() {
        if p[i] == b'\\' || p[i] == b'/' || p[i] == b':' {
            s = i + 1;
        }
    }

    &p[s..]
}

unsafe fn copy(args: &[u8]) {
    if !need_fs() {
        return;
    }

    let (a, rest) = token(args);
    let b = trim(rest);

    if a.is_empty() || b.is_empty() {
        tty::print("Required parameter missing\r\n");
        return;
    }

    let src_ent = match vol::lookup(a) {
        Ok(e) => e,
        Err(e) => return dos_error(e),
    };

    // If the target is an existing directory, copy into it.
    let mut dst = [0u8; 96];
    let mut dn = b.len().min(80);
    dst[..dn].copy_from_slice(&b[..dn]);

    if let Ok(d) = vol::lookup(b) {
        if d.attr & vol::ATTR_DIR != 0 {
            if dn > 0 && dst[dn - 1] != b'\\' && dst[dn - 1] != b'/' {
                dst[dn] = b'\\';
                dn += 1;
            }

            let base = basename(a);
            let m = base.len().min(12);
            dst[dn..dn + m].copy_from_slice(&base[..m]);
            dn += m;
        } else if d.dir == src_ent.dir && d.idx == src_ent.idx {
            tty::print("File cannot be copied onto itself\r\n");
            return;
        }
    }

    let mut src = match vol::open_file(a, false) {
        Ok(f) => f,
        Err(e) => return dos_error(e),
    };

    let mut out = match vol::create_file(&dst[..dn], vol::ATTR_ARC) {
        Ok(f) => f,
        Err(e) => return dos_error(e),
    };

    let off = COPYBUF.as_ptr() as usize as u16;
    let ds = ds_seg();

    loop {
        let n = match vol::read(&mut src, ds, off, 512) {
            Ok(n) => n,
            Err(e) => return dos_error(e),
        };

        if n == 0 {
            break;
        }

        match vol::write(&mut out, ds, off, n) {
            Ok(w) if w == n => {}
            Ok(_) => {
                tty::print("Insufficient disk space\r\n");
                break;
            }
            Err(e) => return dos_error(e),
        }
    }

    vol::close(&mut out);
    tty::print("        1 file(s) copied\r\n");
}

unsafe fn del(args: &[u8]) {
    if !need_fs() {
        return;
    }

    if args.is_empty() {
        tty::print("Required parameter missing\r\n");
        return;
    }

    if let Err(e) = vol::delete_path(args) {
        dos_error(e);
    }
}

unsafe fn ren(args: &[u8]) {
    if !need_fs() {
        return;
    }

    let (a, rest) = token(args);
    let b = trim(rest);

    if a.is_empty() || b.is_empty() {
        tty::print("Required parameter missing\r\n");
        return;
    }

    // The new name has no path: it stays in the directory of the old name.
    let has_path = b.iter().any(|&c| c == b'\\' || c == b'/' || c == b':');
    let mut full = [0u8; 96];
    let mut n = 0;

    if !has_path {
        let dirpart = a.len() - basename(a).len();

        if dirpart + b.len() < 96 {
            full[..dirpart].copy_from_slice(&a[..dirpart]);
            n = dirpart;
        }
    }

    let m = b.len().min(95 - n);
    full[n..n + m].copy_from_slice(&b[..m]);
    n += m;

    if let Err(e) = vol::rename(a, &full[..n]) {
        dos_error(e);
    }
}

unsafe fn md(args: &[u8]) {
    if !need_fs() {
        return;
    }

    if args.is_empty() {
        tty::print("Required parameter missing\r\n");
        return;
    }

    if let Err(e) = vol::mkdir(args) {
        dos_error(e);
    }
}

unsafe fn rd(args: &[u8]) {
    if !need_fs() {
        return;
    }

    if args.is_empty() {
        tty::print("Required parameter missing\r\n");
        return;
    }

    if let Err(e) = vol::rmdir(args) {
        dos_error(e);
    }
}

unsafe fn cd(args: &[u8]) {
    if !need_fs() {
        return;
    }

    if args.is_empty() {
        tty::print("A:\\");

        let mut cwd = [0u8; 64];
        let n = vol::cwd_string(&mut cwd);

        for i in 0..n {
            tty::putc(cwd[i]);
        }

        tty::print("\r\n");
        return;
    }

    if let Err(e) = vol::chdir(args) {
        dos_error(e);
    }
}

// ---------------------------------------------------------- running programs

/// Try `prefix + name` (adding `.COM` if the name has no extension).
unsafe fn try_run(prefix: &[u8], name: &[u8], args: &[u8]) -> bool {
    let mut p = [0u8; 96];
    let mut n = 0;

    if prefix.len() + name.len() + 4 >= 96 {
        return false;
    }

    p[..prefix.len()].copy_from_slice(prefix);
    n += prefix.len();

    if n > 0 && p[n - 1] != b'\\' && p[n - 1] != b'/' && p[n - 1] != b':' {
        p[n] = b'\\';
        n += 1;
    }

    p[n..n + name.len()].copy_from_slice(name);
    n += name.len();

    let base = basename(&p[..n]);
    let has_ext = base.iter().any(|&c| c == b'.');

    if !has_ext {
        p[n..n + 4].copy_from_slice(b".COM");
        n += 4;
    }

    match vol::lookup(&p[..n]) {
        Ok(e) if e.attr & vol::ATTR_DIR == 0 => {}
        _ => return false,
    }

    if let Err(e) = crate::loader::com::run_path(&p[..n], args) {
        dos_error(e);
    }

    true
}

unsafe fn run_external(cmd: &[u8], args: &[u8]) -> bool {
    if !vol::mounted() || cmd.is_empty() {
        return false;
    }

    let has_path = cmd.iter().any(|&c| c == b'\\' || c == b'/' || c == b':');

    if has_path {
        return try_run(b"", cmd, args);
    }

    if try_run(b"", cmd, args) {
        return true;
    }

    let path = env::path_bytes();
    let mut start = 0;

    while start <= path.len() {
        let mut end = start;

        while end < path.len() && path[end] != b';' {
            end += 1;
        }

        if end > start && try_run(&path[start..end], cmd, args) {
            return true;
        }

        start = end + 1;
    }

    false
}

// ------------------------------------------------------------------ helpers

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

#[allow(dead_code)]
fn print_size(n: u32) {
    print_u32(n);
}
