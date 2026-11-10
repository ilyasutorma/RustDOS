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

/// True when a key is waiting in the BIOS buffer (INT 16h, AH=01h).
pub fn key_ready() -> bool {
    let st: u16;

    unsafe {
        core::arch::asm!(
            "mov ah, 1",
            "int 0x16",
            "lahf",
            out("ax") st,
        );
    }

    (st & 0x4000) == 0
}
