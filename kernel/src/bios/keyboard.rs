pub fn read_char() -> u8 {
    loop {
        let ax: u16;

        unsafe {
            core::arch::asm!(
                "int 0x16",
                inout("ax") 0x0000u16 => ax
            );
        }

        let c = ax as u8;

        if c != 0 {
            return c;
        }
    }
}

pub fn getc() -> u8 {
    read_char()
}
