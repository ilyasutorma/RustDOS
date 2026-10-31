use super::farptr::FarPtr;

const IVT: *mut FarPtr = 0 as *mut FarPtr;

pub unsafe fn set(vec: u8, handler: FarPtr) {
    IVT.add(vec as usize).write_volatile(handler);
}

pub unsafe fn init() {
    set(0x20, crate::arch::trampolines::int20_handler());
    set(0x21, crate::arch::trampolines::int21_handler());
}
