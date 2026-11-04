/// Read one sector (LBA) into `buf`, which is an offset in the kernel segment (DS).
pub unsafe fn read_sector(drive: u8, lba: u32, buf: *mut u8, spt: u16, heads: u16) -> bool {
    if spt == 0 || heads == 0 {
        return false;
    }

    let sector = (lba % spt as u32) + 1;
    let head = (lba / spt as u32) % heads as u32;
    let cyl = lba / (spt as u32 * heads as u32);

    let cx = (((cyl >> 2) & 0xC0) | (sector & 0x3F)) as u16 | (((cyl & 0xFF) as u16) << 8);
    let dx = drive as u16 | ((head as u16) << 8);

    // Pointers are DS-relative: ES must be DS and BX the plain 16-bit offset.
    let off = buf as usize as u16;

    let status: u16;

    core::arch::asm!(
        "push es",
        "push ds",
        "pop es",
        "int 0x13",
        "lahf",
        "pop es",
        inout("ax") 0x0201u16 => status,
        inout("cx") cx => _,
        inout("dx") dx => _,
        inout("bx") off => _,
    );

    (status & 0x0100) == 0
}
