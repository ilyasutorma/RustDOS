//! DOS API layer: interrupt services, handles, processes.

pub mod console;
pub mod errors;
pub mod files;
pub mod int20;
pub mod int21;
pub mod memory;
pub mod process;

use crate::arch::regs::Regs;

/// Report a DOS error: AX = code, CF = 1.
pub fn fail(regs: &mut Regs, code: u16) {
    regs.ax = code;
    regs.set_carry();
}
