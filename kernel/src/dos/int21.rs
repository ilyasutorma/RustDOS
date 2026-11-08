use super::errors::DosError;
use crate::arch::regs::Regs;

#[no_mangle]
pub extern "C" fn int21(regs: &mut Regs) {
    match regs.ah() {
        0x09 => print_string(regs),
        0x48 => super::memory::alloc(regs),
        0x49 => super::memory::free(regs),
        0x4A => super::memory::resize(regs),
        0x4C => terminate(regs),
        _ => {
            regs.ax = DosError::InvalidFunction as u16;
            regs.set_carry();
        }
    }
}

fn print_string(regs: &Regs) {
    let seg = regs.ds;
    let mut off = regs.dx;

    loop {
        let c = unsafe { crate::arch::farmem::rd8(seg, off) };

        if c == b'$' {
            break;
        }

        crate::bios::tty::putc(c);
        off = off.wrapping_add(1);
    }
}

fn terminate(_regs: &Regs) {
    crate::loader::com::terminate();
}
