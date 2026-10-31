use super::errors::DosError;
use crate::arch::regs::Regs;

#[no_mangle]
pub extern "C" fn int21(regs: &mut Regs) {
    match regs.ah() {
        0x09 => print_string(regs),
        0x4C => terminate(regs),
        _ => {
            regs.ax = DosError::InvalidFunction as u16;
            regs.set_carry();
        }
    }
}

fn print_string(regs: &Regs) {
    let addr = ((regs.ds as u32) << 4) + regs.dx as u32;
    let mut ptr = addr as *const u8;
    loop {
        let c = unsafe { ptr.read() };
        if c == b'$' { break; }
        crate::bios::tty::putc(c);
        ptr = unsafe { ptr.add(1) };
    }
}

fn terminate(_regs: &Regs) {
    crate::bios::tty::print("\r\nExit.\r\n");
    loop { unsafe { core::arch::asm!("hlt") } }
}
