pub unsafe fn read_sector(drive: u8, lba: u32, buf: *mut u8, spt: u16, heads: u16) -> bool {
    let sector = ((lba % spt as u32) + 1) as u32;
    let head = (lba / spt as u32) % heads as u32;
    let cyl = lba / (spt as u32 * heads as u32);

    let cx = (((cyl >> 2) & 0xC0) | (sector & 0x3F)) as u16 | (((cyl & 0xFF) as u16) << 8);
    let dx = drive as u16 | ((head as u16) << 8);

    let addr = buf as u32;
    let es = (addr >> 4) as u16;
    let bx = (addr & 0xF) as u16;

    let status: u16;

    core::arch::asm!(
        "push es",
        "mov es, {es:x}",
        "int 0x13",
        "lahf",
        "pop es",
        es = in(reg) es,
                     inout("ax") 0x0201u16 => status,
                     in("cx") cx,
                     in("dx") dx,
                     in("bx") bx,
    );

    (status & 0x0100) == 0
}
