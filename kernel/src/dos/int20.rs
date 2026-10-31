use crate::arch::regs::Regs;

#[no_mangle]
pub extern "C" fn int20_handler(_regs: &mut Regs) {
    crate::bios::tty::print("\r\nTerminated.\r\n");
    loop { unsafe { core::arch::asm!("hlt") } }
}
