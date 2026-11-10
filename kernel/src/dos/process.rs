//! Processes: PSP construction, `.COM` loading, EXEC (4Bh) and termination.

#![allow(static_mut_refs)]
#![allow(non_upper_case_globals)]

use crate::arch::farmem::{fill, rd16, rd8, wr16, wr8};
use crate::arch::regs::Regs;
use crate::dos::fail;
use crate::dos::files;
use crate::fs::vol;
use crate::mem::mcb;

pub static mut CURRENT_PSP: u16 = 0;
pub static mut DTA_SEG: u16 = 0;
pub static mut DTA_OFF: u16 = 0x80;
pub static mut LAST_RC: u16 = 0;

#[derive(Clone, Copy)]
struct Ctx {
    regs: [u16; 9], // ax bx cx dx si di bp ds es
    ss: u16,
    sp: u16,
    child: u16,
    parent: u16,
    dta_seg: u16,
    dta_off: u16,
}

const CTX_ZERO: Ctx = Ctx { regs: [0; 9], ss: 0, sp: 0, child: 0, parent: 0, dta_seg: 0, dta_off: 0 };

static mut CTX: [Ctx; 4] = [CTX_ZERO; 4];
static mut CTX_N: usize = 0;

/// Read by `resume_start` (trampolines.rs): ss, sp, ax, bx, cx, dx, si, di, bp, ds, es.
#[no_mangle]
#[used]
pub static mut RESUME: [u16; 11] = [0; 11];

extern "C" {
    static saved_ss: u16;
    static saved_sp: u16;
    fn resume_start() -> !;
}

/// Build a PSP for a freshly loaded program.
pub unsafe fn build_psp(seg: u16, tail: &[u8]) {
    wr8(seg, 0, 0xCD);
    wr8(seg, 1, 0x20);
    wr16(seg, 2, seg + 0x1000);

    wr16(seg, 0x16, CURRENT_PSP);

    wr8(seg, 0x50, 0xCD);
    wr8(seg, 0x51, 0x21);
    wr8(seg, 0x52, 0xCB);

    files::init_jft(seg);

    let n = if tail.len() < 126 { tail.len() } else { 126 };

    wr8(seg, 0x80, n as u8);

    for i in 0..n {
        wr8(seg, 0x81 + i as u16, tail[i]);
    }

    wr8(seg, 0x81 + n as u16, 0x0D);
}

/// Allocate and zero a 64 KB program segment owned by itself.
pub unsafe fn alloc_program(name: &[u8]) -> Result<u16, u16> {
    let seg = match mcb::alloc(0x1000, 0, name) {
        Some(s) => s,
        None => return Err(vol::E_NOMEM),
    };

    mcb::set_owner_of(seg, seg);

    fill(seg, 0, 0, 0x8000);
    fill(seg, 0x8000, 0, 0x8000);

    Ok(seg)
}

/// Load a `.COM` file into a new 64 KB segment and return its segment.
pub unsafe fn load(path: &[u8], tail: &[u8]) -> Result<u16, u16> {
    let mut f = vol::open_file(path, false)?;

    if f.size > 0xFE00 {
        return Err(vol::E_NOMEM);
    }

    // Program name for the MCB: base name without extension.
    let mut name = [0u8; 8];
    let mut start = 0;

    for i in 0..path.len() {
        if path[i] == b'\\' || path[i] == b'/' || path[i] == b':' {
            start = i + 1;
        }
    }

    let mut k = 0;

    for i in start..path.len() {
        if path[i] == b'.' || k >= 8 {
            break;
        }

        name[k] = crate::util::upper(path[i]);
        k += 1;
    }

    let seg = alloc_program(&name[..k])?;

    let sz = f.size as u16;

    match vol::read(&mut f, seg, 0x100, sz) {
        Ok(n) if n == sz => {}
        _ => {
            mcb::free(seg);
            return Err(vol::E_FAIL);
        }
    }

    vol::close(&mut f);

    // An MZ executable is not a .COM image.
    if sz >= 2 && rd8(seg, 0x100) == b'M' && rd8(seg, 0x101) == b'Z' {
        mcb::free(seg);
        return Err(vol::E_BADFMT);
    }

    build_psp(seg, tail);
    Ok(seg)
}

/// Jump to a loaded program: DS=ES=SS=seg, SP=FFFEh, CS:IP=seg:0100h.
/// The word at SS:FFFE is 0, so a plain `ret` lands on the INT 20h at PSP:0000.
pub unsafe fn start(seg: u16) -> ! {
    CURRENT_PSP = seg;
    DTA_SEG = seg;
    DTA_OFF = 0x80;

    core::arch::asm!(
        "cli",
        "mov ds, ax",
        "mov es, ax",
        "mov ss, ax",
        "mov sp, 0xFFFE",
        "sti",
        "push ax",
        "push 0x0100",
        ".byte 0xCB",
        in("ax") seg,
        options(noreturn)
    );
}

/// Run a program from the shell (no parent program to return to).
pub unsafe fn run_from_shell(path: &[u8], tail: &[u8]) -> Result<(), u16> {
    let seg = load(path, tail)?;
    start(seg)
}

/// INT 21h AH=4Bh, AL=0: load and execute; control returns to the caller
/// when the child terminates.
pub unsafe fn exec_dos(regs: &mut Regs) {
    if regs.al() != 0 {
        return fail(regs, vol::E_INVALID);
    }

    let mut path = [0u8; 96];

    let n = match files::read_path(regs.ds, regs.dx, &mut path) {
        Some(n) => n,
        None => return fail(regs, vol::E_PNF),
    };

    let (es, bx) = (regs.es, regs.bx);
    let toff = rd16(es, bx.wrapping_add(2));
    let tseg = rd16(es, bx.wrapping_add(4));

    let mut len = rd8(tseg, toff) as usize;

    if len > 126 {
        len = 126;
    }

    let mut tail = [0u8; 127];

    for i in 0..len {
        tail[i] = rd8(tseg, toff.wrapping_add(1 + i as u16));
    }

    let seg = match load(&path[..n], &tail[..len]) {
        Ok(s) => s,
        Err(e) => return fail(regs, e),
    };

    if CTX_N >= CTX.len() {
        mcb::free(seg);
        return fail(regs, vol::E_NOMEM);
    }

    CTX[CTX_N] = Ctx {
        regs: [regs.ax, regs.bx, regs.cx, regs.dx, regs.si, regs.di, regs.bp, regs.ds, regs.es],
        ss: saved_ss,
        sp: saved_sp,
        child: seg,
        parent: CURRENT_PSP,
        dta_seg: DTA_SEG,
        dta_off: DTA_OFF,
    };

    CTX_N += 1;

    start(seg)
}

/// End the current program (INT 20h, AH=00h, AH=4Ch).
pub unsafe fn terminate(code: u8) -> ! {
    let psp = CURRENT_PSP;

    files::close_all(psp);
    mcb::free_owned(psp);
    LAST_RC = code as u16;

    if CTX_N > 0 && CTX[CTX_N - 1].child == psp {
        CTX_N -= 1;

        let c = CTX[CTX_N];

        CURRENT_PSP = c.parent;
        DTA_SEG = c.dta_seg;
        DTA_OFF = c.dta_off;

        RESUME[0] = c.ss;
        RESUME[1] = c.sp;

        for i in 0..9 {
            RESUME[2 + i] = c.regs[i];
        }

        resume_start();
    }

    CTX_N = 0;
    CURRENT_PSP = 0;
    DTA_SEG = 0;

    crate::shell::commands::repl()
}
