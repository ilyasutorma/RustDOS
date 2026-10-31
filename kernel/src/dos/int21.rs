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
    let addr = ((regs.ds as u32) << 4) + regs.dx as u32;
    let mut p = addr as *const u8;

    loop {
        let c = unsafe { p.read_volatile() };

        if c == b'$' {
            break;
        }

        crate::bios::tty::putc(c);
        p = unsafe { p.add(1) };
    }
}

fn terminate(_regs: &Regs) {
    crate::bios::tty::print("\r\nExit.\r\n");

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}
