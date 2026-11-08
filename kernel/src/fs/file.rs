use crate::bios::disk;
use crate::bios::tty;
use crate::fs::fat;

pub unsafe fn type_file(name: &[u8]) {
    if !fat::mounted() {
        tty::print("No filesystem\r\n");
        return;
    }

    let fs = fat::parse();
    let root_sectors = (fs.root_ent as u32 * 32) / fs.bps as u32;

    for s in 0..root_sectors {
        let buf = core::ptr::addr_of_mut!(fat::BUF1) as *mut u8;

        if !disk::read_sector(fs.drive, fs.root_lba + s, buf, fs.spt, fs.heads) {
            break;
        }

        for i in 0..16 {
            let off = i * 32;
            let name0 = buf.add(off).read();

            if name0 == 0x00 {
                tty::print("File not found\r\n");
                return;
            }

            if name0 == 0xE5 {
                continue;
            }

            if match_name(buf.add(off), name) {
                let cluster = u16::from_le_bytes([
                    buf.add(off + 26).read(),
                                                 buf.add(off + 27).read(),
                ]);

                let size = u32::from_le_bytes([
                    buf.add(off + 28).read(),
                                              buf.add(off + 29).read(),
                                              buf.add(off + 30).read(),
                                              buf.add(off + 31).read(),
                ]);

                print_file(&fs, cluster, size);
                return;
            }
        }
    }

    tty::print("File not found\r\n");
}


unsafe fn print_file(fs: &fat::Fs, mut cluster: u16, size: u32) {
    let mut left = size;

    while cluster < 0x0FF8 && left > 0 {
        let lba = fs.data_lba + (cluster as u32 - 2) * fs.spc as u32;

        for s in 0..fs.spc as u32 {
            if left == 0 {
                break;
            }

            let buf = core::ptr::addr_of_mut!(fat::BUF1) as *mut u8;

            if !disk::read_sector(fs.drive, lba + s, buf, fs.spt, fs.heads) {
                return;
            }

            let n = if left < fs.bps as u32 { left } else { fs.bps as u32 };

            for i in 0..n as usize {
                tty::putc(buf.add(i).read());
            }

            left -= n;
        }

        cluster = read_fat12(fs, cluster);
    }
}

unsafe fn read_fat12(fs: &fat::Fs, cluster: u16) -> u16 {
    let offset = cluster as u32 + cluster as u32 / 2;
    let sector = fs.reserved as u32 + offset / fs.bps as u32;
    let idx = (offset % fs.bps as u32) as usize;

    let buf = core::ptr::addr_of_mut!(fat::BUF2) as *mut u8;

    if !disk::read_sector(fs.drive, sector, buf, fs.spt, fs.heads) {
        return 0x0FFF;
    }

    let lo = buf.add(idx).read() as u16;

    let hi = if idx + 1 < fs.bps as usize {
        buf.add(idx + 1).read() as u16
    } else {
        if !disk::read_sector(fs.drive, sector + 1, buf, fs.spt, fs.heads) {
            return 0x0FFF;
        }

        buf.read() as u16
    };

    let val = lo | (hi << 8);

    if cluster & 1 == 0 {
        val & 0x0FFF
    } else {
        val >> 4
    }
}

pub unsafe fn find_root(name: &[u8]) -> Option<(u16, u32)> {
    if !fat::mounted() {
        return None;
    }

    let fs = fat::parse();
    let root_sectors = (fs.root_ent as u32 * 32) / fs.bps as u32;

    for s in 0..root_sectors {
        let buf = core::ptr::addr_of_mut!(fat::BUF1) as *mut u8;

        if !disk::read_sector(fs.drive, fs.root_lba + s, buf, fs.spt, fs.heads) {
            break;
        }

        for i in 0..16 {
            let off = i * 32;
            let name0 = buf.add(off).read();

            if name0 == 0x00 {
                return None;
            }

            if name0 == 0xE5 {
                continue;
            }

            let attr = buf.add(off + 11).read();

            if attr & 0x10 != 0 {
                continue;
            }

            if match_name(buf.add(off), name) {
                let cluster = u16::from_le_bytes([
                    buf.add(off + 26).read(),
                                                 buf.add(off + 27).read(),
                ]);

                let size = u32::from_le_bytes([
                    buf.add(off + 28).read(),
                                              buf.add(off + 29).read(),
                                              buf.add(off + 30).read(),
                                              buf.add(off + 31).read(),
                ]);

                return Some((cluster, size));
            }
        }
    }

    None
}

pub unsafe fn load_to_mem(cluster: u16, size: u32, seg: u16, off: u32) -> bool {
    let fs = fat::parse();
    let mut c = cluster;
    let mut left = size;
    let mut pos = off;

    while c < 0x0FF8 && left > 0 {
        let lba = fs.data_lba + (c as u32 - 2) * fs.spc as u32;

        for s in 0..fs.spc as u32 {
            if left == 0 {
                break;
            }

            // Destination is seg:off (a program segment), not a DS-relative pointer.
            if !disk::read_sector_far(fs.drive, lba + s, seg, pos as u16, fs.spt, fs.heads) {
                return false;
            }

            let n = if left < fs.bps as u32 { left } else { fs.bps as u32 };

            pos += n;
            left -= n;
        }

        c = read_fat12(&fs, c);
    }

    true
}
unsafe fn match_name(entry: *const u8, name: &[u8]) -> bool {
    let mut buf = [b' '; 11];
    let mut dot = name.len();

    for i in 0..name.len() {
        if name[i] == b'.' {
            dot = i;
            break;
        }
    }

    for i in 0..8 {
        if i < dot {
            buf[i] = fat::upper(name[i]);
        }
    }

    for i in 0..3 {
        if dot + 1 + i < name.len() {
            buf[8 + i] = fat::upper(name[dot + 1 + i]);
        }
    }

    for i in 0..11 {
        let e = entry.add(i).read();
        let c = buf[i];

        if e != c && !(e == 0 && c == b' ') {
            return false;
        }
    }

    true
}
