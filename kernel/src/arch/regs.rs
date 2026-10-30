//! Register frame saved by interrupt stubs and passed to Rust handlers.

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Regs {
    pub ax: u16,
    pub bx: u16,
    pub cx: u16,
    pub dx: u16,
    pub si: u16,
    pub di: u16,
    pub bp: u16,
    pub ds: u16,
    pub es: u16,
    pub flags: u16,
}

impl Regs {
    pub const CF: u16 = 1;

    pub fn ah(&self) -> u8 {
        (self.ax >> 8) as u8
    }

    pub fn al(&self) -> u8 {
        self.ax as u8
    }

    pub fn set_carry(&mut self) {
        self.flags |= Self::CF;
    }

    pub fn clear_carry(&mut self) {
        self.flags &= !Self::CF;
    }
}
