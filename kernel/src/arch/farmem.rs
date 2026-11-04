//! Access to memory by `segment:offset`.
//!
//! Rust pointers are 32-bit and are always relative to DS (= kernel segment
//! 0x1000). A "linear" pointer such as `(seg << 4) as *mut u8` therefore does
//! NOT reach physical memory in real mode. These helpers load ES explicitly.

use core::arch::asm;

pub unsafe fn rd8(seg: u16, off: u16) -> u8 {
    let v: u8;
    asm!(
        "push es",
        "mov es, {s:x}",
        "mov {v}, byte ptr es:[{o:e}]",
        "pop es",
        s = in(reg) seg,
        o = in(reg) off as u32,
        v = out(reg_byte) v,
    );
    v
}

pub unsafe fn rd16(seg: u16, off: u16) -> u16 {
    let v: u16;
    asm!(
        "push es",
        "mov es, {s:x}",
        "mov {v:x}, word ptr es:[{o:e}]",
        "pop es",
        s = in(reg) seg,
        o = in(reg) off as u32,
        v = out(reg) v,
    );
    v
}

pub unsafe fn wr8(seg: u16, off: u16, val: u8) {
    asm!(
        "push es",
        "mov es, {s:x}",
        "mov byte ptr es:[{o:e}], {v}",
        "pop es",
        s = in(reg) seg,
        o = in(reg) off as u32,
        v = in(reg_byte) val,
    );
}

pub unsafe fn wr16(seg: u16, off: u16, val: u16) {
    asm!(
        "push es",
        "mov es, {s:x}",
        "mov word ptr es:[{o:e}], {v:x}",
        "pop es",
        s = in(reg) seg,
        o = in(reg) off as u32,
        v = in(reg) val,
    );
}

/// Fill `count` bytes at `seg:off` with `val` (count <= 0xFFFF).
pub unsafe fn fill(seg: u16, off: u16, val: u8, count: u16) {
    asm!(
        "push es",
        "mov es, {s:x}",
        "cld",
        "rep stosb",
        "pop es",
        s = in(reg) seg,
        inout("di") off => _,
        inout("cx") count => _,
        in("al") val,
    );
}

/// Copy `count` bytes from kernel memory (DS:src) to `seg:off`.
/// (ESI is reserved by LLVM, so `rep movsb` cannot be used here.)
pub unsafe fn copy_to(seg: u16, off: u16, src: *const u8, count: u16) {
    let mut i: u16 = 0;

    while i < count {
        wr8(seg, off.wrapping_add(i), src.add(i as usize).read());
        i += 1;
    }
}
