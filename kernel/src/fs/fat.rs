use crate::bios::disk;
use crate::bios::tty;

pub static mut BUF1: [u8; 512] = [0; 512];
pub static mut BUF2: [u8; 512] = [0; 512];
static mut BPB: [u8; 512] = [0; 512];
static mut DRIVE: u8 = 0;
static mut VALID: bool = false;

pub struct Fs {
    pub drive: u8,
    pub bps: u16,
    pub spc: u8,
    pub reserved: u16,
    pub fats: u8,
    pub root_ent: u16,
    pub spf: u16,
    pub spt: u16,
    pub heads: u16,
    pub root_lba: u32,
    pub data_lba: u32,
}

pub unsafe fn parse() -> Fs {
    let b = core::ptr::addr_of!(BPB) as *const u8;
    let bps = u16::from_le_bytes([b.add(11).read(), b.add(12).read()]);
    let spc = b.add(13).read();
    let reserved = u16::from_le_bytes([b.add(14).read(), b.add(15).read()]);
    let fats = b.add(16).read();
    let root_ent = u16::from_le_bytes([b.add(17).read(), b.add(18).read()]);
    let spf = u16::from_le_bytes([b.add(22).read(), b.add(23).read()]);
    let spt = u16::from_le_bytes([b.add(24).read(), b.add(25).read()]);
    let heads = u16::from_le_bytes([b.add(26).read(), b.add(27).read()]);

    let root_sectors = (root_ent as u32 * 32 + bps as u32 - 1) / bps as u32;
    let root_lba = reserved as u32 + fats as u32 * spf as u32;
    let data_lba = root_lba + root_sectors;

    Fs {
        drive: DRIVE,
        bps,
        spc,
        reserved,
        fats,
        root_ent,
        spf,
        spt,
        heads,
        root_lba,
        data_lba,
    }
}

pub unsafe fn init(drive: u8) -> bool {
    DRIVE = drive;

    let buf = core::ptr::addr_of_mut!(BUF1) as *mut u8;

    if !disk::read_sector(drive, 0, buf, 18, 2) {
        VALID = false;
        return false;
    }

    if buf.add(510).read() != 0x55 || buf.add(511).read() != 0xAA {
        VALID = false;
        return false;
    }

    let bps = u16::from_le_bytes([buf.add(11).read(), buf.add(12).read()]);

    if bps != 512 {
        VALID = false;
        return false;
    }

    core::ptr::copy_nonoverlapping(buf, core::ptr::addr_of_mut!(BPB) as *mut u8, 512);

    VALID = true;
    true
}

pub unsafe fn mounted() -> bool {
    VALID
}

pub unsafe fn list_root() {
    if !VALID {
        tty::print("No filesystem\r\n");
        return;
    }

    let fs = parse();
    let root_sectors = (fs.root_ent as u32 * 32) / fs.bps as u32;

    tty::print("\r\n Volume in drive has no label\r\n");
    tty::print(" Directory of root\r\n\r\n");

    let mut count = 0u16;
    let mut total = 0u32;

    for s in 0..root_sectors {
        let buf = core::ptr::addr_of_mut!(BUF1) as *mut u8;

        if !disk::read_sector(fs.drive, fs.root_lba + s, buf, fs.spt, fs.heads) {
            break;
        }

        for i in 0..16 {
            let off = i * 32;
            let name0 = buf.add(off).read();

            if name0 == 0x00 {
                print_total(count, total);
                return;
            }

            if name0 == 0xE5 {
                continue;
            }

            let attr = buf.add(off + 11).read();

            if attr & 0x0F == 0x0F {
                continue;
            }

            let size = u32::from_le_bytes([
                buf.add(off + 28).read(),
                                          buf.add(off + 29).read(),
                                          buf.add(off + 30).read(),
                                          buf.add(off + 31).read(),
            ]);

            print_entry(buf.add(off), attr, size);

            count += 1;
            total += size;
        }
    }

    print_total(count, total);
}

unsafe fn print_entry(e: *const u8, attr: u8, size: u32) {
    for i in 0..8 {
        let c = e.add(i).read();

        if c == 0 || c == b' ' {
            tty::putc(b' ');
        } else {
            tty::putc(c);
        }
    }

    let ext0 = e.add(8).read();

    if ext0 != 0 && ext0 != b' ' {
        tty::putc(b' ');

        for i in 0..3 {
            let c = e.add(8 + i).read();

            if c == 0 || c == b' ' {
                break;
            }

            tty::putc(c);
        }
    } else {
        tty::print("    ");
    }

    tty::print("  ");

    if attr & 0x10 != 0 {
        tty::print("<DIR>       ");
    } else {
        print_u32(size);
        tty::print("  ");
    }

    tty::print("\r\n");
}

unsafe fn print_total(count: u16, total: u32) {
    tty::print("       ");
    print_u16(count);
    tty::print(" file(s)  ");
    print_u32(total);
    tty::print(" bytes\r\n");
}

pub fn upper(c: u8) -> u8 {
    if c >= b'a' && c <= b'z' {
        c - 32
    } else {
        c
    }
}

pub fn print_u16(mut n: u16) {
    if n == 0 {
        tty::putc(b'0');
        return;
    }

    let mut buf = [0u8; 5];
    let mut i = 0;

    while n > 0 {
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        tty::putc(buf[i]);
    }
}

pub fn print_u32(mut n: u32) {
    if n == 0 {
        tty::putc(b'0');
        return;
    }

    let mut buf = [0u8; 10];
    let mut i = 0;

    while n > 0 {
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        tty::putc(buf[i]);
    }
}
