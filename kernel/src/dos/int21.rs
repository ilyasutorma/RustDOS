//! INT 21h dispatcher. Called from an assembly trampoline that saves registers
//! into a `Regs` frame and does `iret` afterwards.

use super::errors::DosError;
use crate::arch::regs::Regs;

#[no_mangle]
pub extern "C" fn int21(regs: &mut Regs) {
    match regs.ah() {
        // 09h: print '$'-terminated string at DS:DX
        0x09 => { /* TODO */ }
        // 4Ch: terminate process with return code AL
        0x4C => { /* TODO */ }
        _ => {
            regs.ax = DosError::InvalidFunction as u16;
            regs.set_carry();
        }
    }
}
