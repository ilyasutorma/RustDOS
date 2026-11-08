use crate::arch::regs::Regs;

#[no_mangle]
pub extern "C" fn int20_handler(_regs: &mut Regs) {
    crate::loader::com::terminate();
}
