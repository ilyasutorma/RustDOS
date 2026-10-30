//! `segment:offset` far pointer, the core of the real-mode memory model.

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FarPtr {
    pub off: u16,
    pub seg: u16,
}

impl FarPtr {
    pub const fn new(seg: u16, off: u16) -> Self {
        Self { off, seg }
    }

    /// Physical address: `seg * 16 + off` (up to 0x10FFEF with the A20 gate open).
    pub const fn linear(self) -> u32 {
        ((self.seg as u32) << 4) + self.off as u32
    }
}
