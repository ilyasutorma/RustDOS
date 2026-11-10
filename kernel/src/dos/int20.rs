use crate::arch::regs::Regs;

#[no_mangle]
pub extern "C" fn int20_handler(_regs: &mut Regs) {
    unsafe { crate::dos::process::terminate(0) }
}
