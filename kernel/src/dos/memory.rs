use crate::arch::regs::Regs;
use crate::dos::errors::DosError;
use crate::dos::process::CURRENT_PSP;
use crate::mem::mcb;

pub fn alloc(r: &mut Regs) {
    unsafe {
        match mcb::alloc(r.bx, CURRENT_PSP, b"PROG") {
            Some(seg) => {
                r.ax = seg;
                r.clear_carry();
            }
            None => {
                r.ax = DosError::InsufficientMemory as u16;
                r.bx = mcb::largest();
                r.set_carry();
            }
        }
    }
}

pub fn free(r: &mut Regs) {
    unsafe {
        if mcb::free(r.es) {
            r.clear_carry();
        } else {
            r.ax = DosError::InvalidMemoryBlock as u16;
            r.set_carry();
        }
    }
}

pub fn resize(r: &mut Regs) {
    unsafe {
        match mcb::resize(r.es, r.bx) {
            Ok(n) => {
                r.ax = r.es;
                r.bx = n;
                r.clear_carry();
            }
            Err(n) => {
                r.ax = DosError::InsufficientMemory as u16;
                r.bx = n;
                r.set_carry();
            }
        }
    }
}
