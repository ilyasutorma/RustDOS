/// Read one sector (LBA) into `buf`, which is an offset in the kernel segment (DS).
pub unsafe fn read_sector(drive: u8, lba: u32, buf: *mut u8, spt: u16, heads: u16) -> bool {
    let ds: u16;
    core::arch::asm!("mov {0:x}, ds", out(reg) ds);

    read_sector_far(drive, lba, ds, buf as usize as u16, spt, heads)
}

/// Read one sector (LBA) into `seg:off` (any segment, e.g. a program's).
pub unsafe fn read_sector_far(
    drive: u8,
    lba: u32,
    seg: u16,
    off: u16,
    spt: u16,
    heads: u16,
) -> bool {
    if spt == 0 || heads == 0 {
        return false;
    }

    let sector = (lba % spt as u32) + 1;
    let head = (lba / spt as u32) % heads as u32;
    let cyl = lba / (spt as u32 * heads as u32);

    let cx = (((cyl >> 2) & 0xC0) | (sector & 0x3F)) as u16 | (((cyl & 0xFF) as u16) << 8);
    let dx = drive as u16 | ((head as u16) << 8);

    let status: u16;

    core::arch::asm!(
        "push es",
        "mov es, {s:x}",
        "int 0x13",
        "lahf",
        "pop es",
        s = in(reg) seg,
        inout("ax") 0x0201u16 => status,
        inout("cx") cx => _,
        inout("dx") dx => _,
        inout("bx") off => _,
    );

    (status & 0x0100) == 0
}
