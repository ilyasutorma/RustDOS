//! File handles (SFT + per-process JFT) and INT 21h file/directory services.

#![allow(static_mut_refs)]

use crate::arch::farmem::{rd16, rd8, wr16, wr8};
use crate::arch::regs::Regs;
use crate::dos::console;
use crate::dos::fail;
use crate::dos::process::{CURRENT_PSP, DTA_OFF, DTA_SEG};
use crate::fs::vol::{self, File};

const SFT_N: usize = 24;
const JFT_OFF: u16 = 0x18;
const JFT_N: u16 = 20;

const DEV_FILE: u8 = 0;
const DEV_CON: u8 = 1;
const DEV_NUL: u8 = 2;

#[derive(Clone, Copy)]
struct Sft {
    refs: u8,
    dev: u8,
    /// 0 = read, 1 = write, 2 = read/write
    mode: u8,
    f: File,
}

const SFT_ZERO: Sft = Sft { refs: 0, dev: DEV_FILE, mode: 0, f: File::ZERO };

static mut SFT: [Sft; SFT_N] = [SFT_ZERO; SFT_N];

pub unsafe fn init() {
    SFT = [SFT_ZERO; SFT_N];
    SFT[0] = Sft { refs: 250, dev: DEV_CON, mode: 2, f: File::ZERO };
    SFT[1] = Sft { refs: 250, dev: DEV_NUL, mode: 2, f: File::ZERO };
}

/// Standard handles: 0,1,2 = CON, 3,4 = NUL (AUX/PRN are not implemented).
pub unsafe fn init_jft(psp: u16) {
    for i in 0..JFT_N {
        wr8(psp, JFT_OFF + i, 0xFF);
    }

    for i in 0..3 {
        wr8(psp, JFT_OFF + i, 0);
    }

    wr8(psp, JFT_OFF + 3, 1);
    wr8(psp, JFT_OFF + 4, 1);

    wr16(psp, 0x32, JFT_N);
    wr16(psp, 0x34, JFT_OFF);
    wr16(psp, 0x36, psp);
}

unsafe fn release(i: u8) {
    if i < 2 {
        return;
    }

    let s = &mut SFT[i as usize];

    if s.refs == 0 {
        return;
    }

    s.refs -= 1;

    if s.refs == 0 && s.dev == DEV_FILE {
        vol::close(&mut s.f);
    }
}

/// Close every handle a terminating process still holds.
pub unsafe fn close_all(psp: u16) {
    if psp == 0 {
        return;
    }

    for i in 0..JFT_N {
        let v = rd8(psp, JFT_OFF + i);

        if v != 0xFF {
            release(v);
            wr8(psp, JFT_OFF + i, 0xFF);
        }
    }
}

unsafe fn sft_of(h: u16) -> Option<usize> {
    let psp = CURRENT_PSP;

    if psp == 0 || h >= JFT_N {
        return None;
    }

    let v = rd8(psp, JFT_OFF + h);

    if v == 0xFF {
        None
    } else {
        Some(v as usize)
    }
}

unsafe fn alloc_sft() -> Option<u8> {
    for i in 2..SFT_N {
        if SFT[i].refs == 0 {
            return Some(i as u8);
        }
    }

    None
}

unsafe fn alloc_handle() -> Option<u16> {
    let psp = CURRENT_PSP;

    if psp == 0 {
        return None;
    }

    for i in 0..JFT_N {
        if rd8(psp, JFT_OFF + i) == 0xFF {
            return Some(i);
        }
    }

    None
}

/// Copy an ASCIIZ string from user memory.
pub unsafe fn read_path(seg: u16, off: u16, buf: &mut [u8; 96]) -> Option<usize> {
    for i in 0..95 {
        let c = rd8(seg, off.wrapping_add(i as u16));

        if c == 0 {
            return Some(i);
        }

        buf[i] = c;
    }

    None
}

macro_rules! path_arg {
    ($regs:expr, $buf:ident, $n:ident) => {
        let mut $buf = [0u8; 96];

        let $n = match read_path($regs.ds, $regs.dx, &mut $buf) {
            Some(n) => n,
            None => {
                fail($regs, vol::E_PNF);
                return;
            }
        };
    };
}

macro_rules! handle_arg {
    ($regs:expr, $idx:ident) => {
        let $idx = match sft_of($regs.bx) {
            Some(i) => i,
            None => {
                fail($regs, vol::E_HANDLE);
                return;
            }
        };
    };
}

// ------------------------------------------------------------- handle I/O

pub unsafe fn open(regs: &mut Regs) {
    path_arg!(regs, path, n);

    let mode = regs.al() & 3;

    let f = match vol::open_file(&path[..n], mode != 0) {
        Ok(f) => f,
        Err(e) => return fail(regs, e),
    };

    install(regs, f, mode);
}

pub unsafe fn create(regs: &mut Regs) {
    path_arg!(regs, path, n);

    let f = match vol::create_file(&path[..n], regs.cx as u8) {
        Ok(f) => f,
        Err(e) => return fail(regs, e),
    };

    install(regs, f, 2);
}

unsafe fn install(regs: &mut Regs, f: File, mode: u8) {
    let (s, h) = match (alloc_sft(), alloc_handle()) {
        (Some(s), Some(h)) => (s, h),
        _ => return fail(regs, vol::E_TOOMANY),
    };

    SFT[s as usize] = Sft { refs: 1, dev: DEV_FILE, mode, f };
    wr8(CURRENT_PSP, JFT_OFF + h, s);
    regs.ax = h;
}

pub unsafe fn close(regs: &mut Regs) {
    handle_arg!(regs, idx);

    release(idx as u8);
    wr8(CURRENT_PSP, JFT_OFF + regs.bx, 0xFF);
}

pub unsafe fn read(regs: &mut Regs) {
    handle_arg!(regs, idx);

    let (seg, off, count) = (regs.ds, regs.dx, regs.cx);
    let s = &mut SFT[idx];

    match s.dev {
        DEV_CON => regs.ax = console::read_stdin(seg, off, count),
        DEV_NUL => regs.ax = 0,
        _ => {
            if s.mode == 1 {
                return fail(regs, vol::E_ACCESS);
            }

            match vol::read(&mut s.f, seg, off, count) {
                Ok(n) => regs.ax = n,
                Err(e) => fail(regs, e),
            }
        }
    }
}

pub unsafe fn write(regs: &mut Regs) {
    handle_arg!(regs, idx);

    let (seg, off, count) = (regs.ds, regs.dx, regs.cx);
    let s = &mut SFT[idx];

    match s.dev {
        DEV_CON => {
            console::write_stdout(seg, off, count);
            regs.ax = count;
        }
        DEV_NUL => regs.ax = count,
        _ => {
            if s.mode == 0 {
                return fail(regs, vol::E_ACCESS);
            }

            if count == 0 {
                vol::truncate(&mut s.f);
                regs.ax = 0;
                return;
            }

            match vol::write(&mut s.f, seg, off, count) {
                Ok(n) => regs.ax = n,
                Err(e) => fail(regs, e),
            }
        }
    }
}

pub unsafe fn lseek(regs: &mut Regs) {
    handle_arg!(regs, idx);

    let s = &mut SFT[idx];

    if s.dev != DEV_FILE {
        regs.ax = 0;
        regs.dx = 0;
        return;
    }

    let off = (((regs.cx as u32) << 16) | regs.dx as u32) as i32;

    match vol::seek(&mut s.f, regs.al(), off) {
        Ok(p) => {
            regs.ax = p as u16;
            regs.dx = (p >> 16) as u16;
        }
        Err(e) => fail(regs, e),
    }
}

pub unsafe fn dup(regs: &mut Regs) {
    handle_arg!(regs, idx);

    let h = match alloc_handle() {
        Some(h) => h,
        None => return fail(regs, vol::E_TOOMANY),
    };

    if idx >= 2 {
        SFT[idx].refs = SFT[idx].refs.saturating_add(1);
    }

    wr8(CURRENT_PSP, JFT_OFF + h, idx as u8);
    regs.ax = h;
}

/// AH=46h: make CX refer to the same file as BX.
pub unsafe fn dup2(regs: &mut Regs) {
    handle_arg!(regs, idx);

    let target = regs.cx;

    if target >= JFT_N {
        return fail(regs, vol::E_HANDLE);
    }

    let old = rd8(CURRENT_PSP, JFT_OFF + target);

    if old != 0xFF {
        release(old);
    }

    if idx >= 2 {
        SFT[idx].refs = SFT[idx].refs.saturating_add(1);
    }

    wr8(CURRENT_PSP, JFT_OFF + target, idx as u8);
}

pub unsafe fn ioctl(regs: &mut Regs) {
    match regs.al() {
        0 => {
            handle_arg!(regs, idx);

            regs.dx = match SFT[idx].dev {
                DEV_CON => 0x80 | 0x03,
                DEV_NUL => 0x80 | 0x04,
                _ => 0x0000, // file on drive A:
            };
        }
        1 => {}
        6 => {
            handle_arg!(regs, _idx);
            regs.ax = (regs.ax & 0xFF00) | 0xFF;
        }
        7 => {
            regs.ax = (regs.ax & 0xFF00) | 0xFF;
        }
        8 => regs.ax = 0, // removable media
        _ => fail(regs, vol::E_INVALID),
    }
}

pub unsafe fn file_time(regs: &mut Regs) {
    handle_arg!(regs, idx);

    let s = &mut SFT[idx];

    if s.dev != DEV_FILE {
        return fail(regs, vol::E_INVALID);
    }

    match regs.al() {
        0 => {
            let (t, d) = vol::file_stamp(&s.f);
            regs.cx = t;
            regs.dx = d;
        }
        1 => {
            vol::set_file_stamp(&s.f, regs.cx, regs.dx);
        }
        _ => fail(regs, vol::E_INVALID),
    }
}

// ---------------------------------------------------- names and directories

pub unsafe fn delete(regs: &mut Regs) {
    path_arg!(regs, path, n);

    if let Err(e) = vol::delete_path(&path[..n]) {
        fail(regs, e);
    }
}

pub unsafe fn rename(regs: &mut Regs) {
    path_arg!(regs, path, n);

    let mut new = [0u8; 96];

    let m = match read_path(regs.es, regs.di, &mut new) {
        Some(m) => m,
        None => return fail(regs, vol::E_PNF),
    };

    if let Err(e) = vol::rename(&path[..n], &new[..m]) {
        fail(regs, e);
    }
}

pub unsafe fn mkdir(regs: &mut Regs) {
    path_arg!(regs, path, n);

    if let Err(e) = vol::mkdir(&path[..n]) {
        fail(regs, e);
    }
}

pub unsafe fn rmdir(regs: &mut Regs) {
    path_arg!(regs, path, n);

    if let Err(e) = vol::rmdir(&path[..n]) {
        fail(regs, e);
    }
}

pub unsafe fn chdir(regs: &mut Regs) {
    path_arg!(regs, path, n);

    if let Err(e) = vol::chdir(&path[..n]) {
        fail(regs, e);
    }
}

pub unsafe fn getcwd(regs: &mut Regs) {
    let mut out = [0u8; 64];
    let n = vol::cwd_string(&mut out);

    for i in 0..n {
        wr8(regs.ds, regs.si.wrapping_add(i as u16), out[i]);
    }

    wr8(regs.ds, regs.si.wrapping_add(n as u16), 0);
}

pub unsafe fn attrib(regs: &mut Regs) {
    path_arg!(regs, path, n);

    match regs.al() {
        0 => match vol::get_attr(&path[..n]) {
            Ok(a) => regs.cx = a as u16,
            Err(e) => fail(regs, e),
        },
        1 => {
            if let Err(e) = vol::set_attr(&path[..n], regs.cx as u8) {
                fail(regs, e);
            }
        }
        _ => fail(regs, vol::E_INVALID),
    }
}

// --------------------------------------------------------------- find files

unsafe fn put_dta_entry(e: &vol::Ent) {
    let (s, o) = (DTA_SEG, DTA_OFF);

    wr8(s, o + 0x15, e.attr);
    wr16(s, o + 0x16, e.time);
    wr16(s, o + 0x18, e.date);
    wr16(s, o + 0x1A, e.size as u16);
    wr16(s, o + 0x1C, (e.size >> 16) as u16);

    let mut name = [0u8; 13];
    let n = vol::fmt_name(&e.name, &mut name);

    for i in 0..n {
        wr8(s, o + 0x1E + i as u16, name[i]);
    }

    wr8(s, o + 0x1E + n as u16, 0);
}

unsafe fn put_dta_state(st: &vol::Search) {
    let (s, o) = (DTA_SEG, DTA_OFF);

    wr8(s, o, st.mask);

    for i in 0..11 {
        wr8(s, o + 1 + i as u16, st.pat[i]);
    }

    wr16(s, o + 12, st.dir);
    wr16(s, o + 14, st.next);
}

pub unsafe fn find_first(regs: &mut Regs) {
    path_arg!(regs, path, n);

    match vol::find_first(&path[..n], regs.cx as u8) {
        Ok((st, e)) => {
            put_dta_state(&st);
            put_dta_entry(&e);
        }
        Err(e) => fail(regs, e),
    }
}

pub unsafe fn find_next(regs: &mut Regs) {
    let (s, o) = (DTA_SEG, DTA_OFF);

    let mut st = vol::Search {
        dir: rd16(s, o + 12),
        pat: [0; 11],
        mask: rd8(s, o),
        next: rd16(s, o + 14),
    };

    for i in 0..11 {
        st.pat[i] = rd8(s, o + 1 + i as u16);
    }

    match vol::find_next(&mut st) {
        Some(e) => {
            put_dta_state(&st);
            put_dta_entry(&e);
        }
        None => fail(regs, vol::E_NOMORE),
    }
}

/// AH=36h: disk free space.
pub unsafe fn free_space(regs: &mut Regs) {
    regs.ax = vol::sectors_per_cluster();
    regs.bx = vol::free_clusters();
    regs.cx = 512;
    regs.dx = vol::total_clusters();
}
