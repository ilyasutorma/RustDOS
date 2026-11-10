//! Date and time from the BIOS real-time clock (INT 1Ah).

fn bcd(x: u8) -> u8 {
    (x >> 4) * 10 + (x & 0x0F)
}

/// (year, month, day). Falls back to 1980-01-01 when the RTC is unavailable.
pub fn get_date() -> (u16, u8, u8) {
    let st: u16;
    let cx: u16;
    let dx: u16;

    unsafe {
        core::arch::asm!(
            "int 0x1a",
            "lahf",
            inout("ax") 0x0400u16 => st,
            inout("cx") 0u16 => cx,
            inout("dx") 0u16 => dx,
        );
    }

    let century = bcd((cx >> 8) as u8) as u16;
    let year = bcd(cx as u8) as u16;
    let month = bcd((dx >> 8) as u8);
    let day = bcd(dx as u8);

    if (st & 0x0100) != 0 || month == 0 || month > 12 || day == 0 || day > 31 {
        return (1980, 1, 1);
    }

    (century * 100 + year, month, day)
}

/// (hour, minute, second).
pub fn get_time() -> (u8, u8, u8) {
    let st: u16;
    let cx: u16;
    let dx: u16;

    unsafe {
        core::arch::asm!(
            "int 0x1a",
            "lahf",
            inout("ax") 0x0200u16 => st,
            inout("cx") 0u16 => cx,
            inout("dx") 0u16 => dx,
        );
    }

    let h = bcd((cx >> 8) as u8);
    let m = bcd(cx as u8);
    let s = bcd((dx >> 8) as u8);

    if (st & 0x0100) != 0 || h > 23 || m > 59 || s > 59 {
        return (0, 0, 0);
    }

    (h, m, s)
}

/// Day of week, 0 = Sunday.
pub fn day_of_week(year: u16, month: u8, day: u8) -> u8 {
    const T: [u16; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];

    let mut y = year;

    if month < 3 {
        y -= 1;
    }

    ((y + y / 4 - y / 100 + y / 400 + T[(month - 1) as usize] + day as u16) % 7) as u8
}
