use super::farmem::wr16;
use super::farptr::FarPtr;

/// Install an interrupt vector. The IVT lives at physical 0000:0000, which is NOT
/// reachable through a Rust pointer (those are relative to DS = 0x1000), so it is
/// written with an explicit segment of 0. Offset first, then segment.
pub unsafe fn set(vec: u8, handler: FarPtr) {
    let at = vec as u16 * 4;

    wr16(0, at, handler.off);
    wr16(0, at + 2, handler.seg);
}

pub unsafe fn init() {
    set(0x20, crate::arch::trampolines::int20_handler());
    set(0x21, crate::arch::trampolines::int21_handler());
}
