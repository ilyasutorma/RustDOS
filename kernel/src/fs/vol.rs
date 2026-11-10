//! FAT12 volume driver: FAT cache, cluster allocator, directories (with
//! subdirectories), path resolution and file read/write/seek.
//!
//! One volume (drive A:) is supported. All sector I/O goes through a single
//! 512-byte buffer `SBUF` with a one-sector cache (`SLBA`).

#![allow(static_mut_refs)]
#![allow(dead_code)]

use crate::arch::farmem::{copy_from, copy_to};
use crate::bios::{disk, time};
use crate::util::upper;

pub const ATTR_RO: u8 = 0x01;
pub const ATTR_HID: u8 = 0x02;
pub const ATTR_SYS: u8 = 0x04;
pub const ATTR_VOL: u8 = 0x08;
pub const ATTR_DIR: u8 = 0x10;
pub const ATTR_ARC: u8 = 0x20;

// DOS error codes used by this driver.
pub const E_INVALID: u16 = 0x01;
pub const E_FNF: u16 = 0x02;
pub const E_PNF: u16 = 0x03;
pub const E_TOOMANY: u16 = 0x04;
pub const E_ACCESS: u16 = 0x05;
pub const E_HANDLE: u16 = 0x06;
pub const E_NOMEM: u16 = 0x08;
pub const E_BADFMT: u16 = 0x0B;
pub const E_DRIVE: u16 = 0x0F;
pub const E_RMCWD: u16 = 0x10;
pub const E_NOMORE: u16 = 0x12;
pub const E_FAIL: u16 = 0x1F;

const FAT_BYTES: usize = 4608; // 9 sectors: enough for a 1.44 MB floppy

#[derive(Clone, Copy)]
struct Geo {
    drive: u8,
    spc: u8,
    reserved: u16,
    fats: u8,
    root_ent: u16,
    spf: u16,
    spt: u16,
    heads: u16,
    root_lba: u32,
    data_lba: u32,
    clusters: u16,
}

static mut G: Geo = Geo {
    drive: 0,
    spc: 0,
    reserved: 0,
    fats: 0,
    root_ent: 0,
    spf: 0,
    spt: 18,
    heads: 2,
    root_lba: 0,
    data_lba: 0,
    clusters: 0,
};

static mut MOUNTED: bool = false;
static mut SBUF: [u8; 512] = [0; 512];
static mut SLBA: u32 = 0xFFFF_FFFF;
static mut FAT: [u8; FAT_BYTES] = [0; FAT_BYTES];
static mut FAT_DIRTY: u32 = 0;
static mut CWD: u16 = 0;
static mut HINT: u16 = 2;

#[inline]
unsafe fn sb() -> *mut u8 {
    core::ptr::addr_of_mut!(SBUF) as *mut u8
}

unsafe fn u16at(p: *const u8, o: usize) -> u16 {
    p.add(o).read() as u16 | ((p.add(o + 1).read() as u16) << 8)
}

unsafe fn u32at(p: *const u8, o: usize) -> u32 {
    u16at(p, o) as u32 | ((u16at(p, o + 2) as u32) << 16)
}

unsafe fn put16(p: *mut u8, o: usize, v: u16) {
    p.add(o).write(v as u8);
    p.add(o + 1).write((v >> 8) as u8);
}

unsafe fn put32(p: *mut u8, o: usize, v: u32) {
    put16(p, o, v as u16);
    put16(p, o + 2, (v >> 16) as u16);
}

// ---------------------------------------------------------------- sectors

unsafe fn rd_sec(lba: u32) -> bool {
    if SLBA == lba {
        return true;
    }

    SLBA = 0xFFFF_FFFF;

    if !disk::read_sector(G.drive, lba, sb(), G.spt, G.heads) {
        return false;
    }

    SLBA = lba;
    true
}

unsafe fn wr_sec(lba: u32) -> bool {
    SLBA = 0xFFFF_FFFF;

    if !disk::write_sector(G.drive, lba, sb(), G.spt, G.heads) {
        return false;
    }

    SLBA = lba;
    true
}

#[inline]
unsafe fn invalidate() {
    SLBA = 0xFFFF_FFFF;
}

// ------------------------------------------------------------------ mount

pub unsafe fn init(drive: u8) -> bool {
    MOUNTED = false;
    G.drive = drive;
    G.spt = 18;
    G.heads = 2;
    invalidate();

    if !rd_sec(0) {
        return false;
    }

    let b = sb() as *const u8;

    if b.add(510).read() != 0x55 || b.add(511).read() != 0xAA {
        return false;
    }

    if u16at(b, 11) != 512 {
        return false;
    }

    let spc = b.add(13).read();
    let reserved = u16at(b, 14);
    let fats = b.add(16).read();
    let root_ent = u16at(b, 17);
    let mut total = u16at(b, 19) as u32;
    let spf = u16at(b, 22);
    let spt = u16at(b, 24);
    let heads = u16at(b, 26);

    if total == 0 {
        total = u32at(b, 32);
    }

    if spc == 0 || fats == 0 || spf == 0 || spt == 0 || heads == 0 || root_ent == 0 {
        return false;
    }

    if spf as usize * 512 > FAT_BYTES {
        return false;
    }

    let root_sectors = (root_ent as u32 * 32 + 511) / 512;
    let root_lba = reserved as u32 + fats as u32 * spf as u32;
    let data_lba = root_lba + root_sectors;

    if total <= data_lba {
        return false;
    }

    let clusters = (total - data_lba) / spc as u32;

    if clusters >= 4085 {
        return false; // FAT16 is not supported yet
    }

    G.spc = spc;
    G.reserved = reserved;
    G.fats = fats;
    G.root_ent = root_ent;
    G.spf = spf;
    G.spt = spt;
    G.heads = heads;
    G.root_lba = root_lba;
    G.data_lba = data_lba;
    G.clusters = clusters as u16;

    invalidate();

    for s in 0..spf as u32 {
        if !rd_sec(reserved as u32 + s) {
            return false;
        }

        core::ptr::copy_nonoverlapping(sb() as *const u8, FAT.as_mut_ptr().add(s as usize * 512), 512);
    }

    FAT_DIRTY = 0;
    HINT = 2;
    CWD = 0;
    MOUNTED = true;
    true
}

pub unsafe fn mounted() -> bool {
    MOUNTED
}

pub unsafe fn bytes_per_cluster() -> u32 {
    G.spc as u32 * 512
}

pub unsafe fn sectors_per_cluster() -> u16 {
    G.spc as u16
}

pub unsafe fn total_clusters() -> u16 {
    G.clusters
}

// -------------------------------------------------------------------- FAT

pub unsafe fn fat_get(c: u16) -> u16 {
    let o = c as usize + c as usize / 2;

    if o + 1 >= FAT_BYTES {
        return 0xFFF;
    }

    let v = FAT[o] as u16 | ((FAT[o + 1] as u16) << 8);

    if c & 1 == 0 {
        v & 0x0FFF
    } else {
        v >> 4
    }
}

unsafe fn fat_set(c: u16, val: u16) {
    let o = c as usize + c as usize / 2;

    if o + 1 >= FAT_BYTES {
        return;
    }

    if c & 1 == 0 {
        FAT[o] = val as u8;
        FAT[o + 1] = (FAT[o + 1] & 0xF0) | ((val >> 8) as u8 & 0x0F);
    } else {
        FAT[o] = (FAT[o] & 0x0F) | ((val << 4) as u8 & 0xF0);
        FAT[o + 1] = (val >> 4) as u8;
    }

    FAT_DIRTY |= 1u32 << (o / 512);
    FAT_DIRTY |= 1u32 << ((o + 1) / 512);
}

/// Write modified FAT sectors to every FAT copy.
pub unsafe fn flush_fat() -> bool {
    if FAT_DIRTY == 0 {
        return true;
    }

    let mut ok = true;

    for s in 0..G.spf as u32 {
        if FAT_DIRTY & (1u32 << s) == 0 {
            continue;
        }

        for f in 0..G.fats as u32 {
            core::ptr::copy_nonoverlapping(FAT.as_ptr().add(s as usize * 512), sb(), 512);

            if !wr_sec(G.reserved as u32 + f * G.spf as u32 + s) {
                ok = false;
            }
        }
    }

    FAT_DIRTY = 0;
    ok
}

#[inline]
unsafe fn cl_lba(c: u16) -> u32 {
    G.data_lba + (c as u32 - 2) * G.spc as u32
}

fn is_eoc(c: u16) -> bool {
    c < 2 || c >= 0xFF8
}

pub unsafe fn alloc_cluster() -> Option<u16> {
    let max = G.clusters + 1;
    let mut c = HINT;

    if c < 2 || c > max {
        c = 2;
    }

    for _ in 0..G.clusters {
        if fat_get(c) == 0 {
            fat_set(c, 0xFFF);
            HINT = c + 1;
            return Some(c);
        }

        c += 1;

        if c > max {
            c = 2;
        }
    }

    None
}

unsafe fn free_chain(mut c: u16) {
    let mut guard = G.clusters;

    while !is_eoc(c) && guard > 0 {
        let n = fat_get(c);
        fat_set(c, 0);
        c = n;
        guard -= 1;
    }
}

unsafe fn zero_cluster(c: u16) {
    for i in 0..512 {
        sb().add(i).write(0);
    }

    invalidate();

    for s in 0..G.spc as u32 {
        wr_sec(cl_lba(c) + s);
    }
}

unsafe fn last_cluster(mut c: u16) -> u16 {
    let mut guard = G.clusters;

    loop {
        let n = fat_get(c);

        if is_eoc(n) || guard == 0 {
            return c;
        }

        c = n;
        guard -= 1;
    }
}

pub unsafe fn free_clusters() -> u16 {
    let mut n = 0;

    for c in 2..G.clusters + 2 {
        if fat_get(c) == 0 {
            n += 1;
        }
    }

    n
}

// ------------------------------------------------------------ directories

#[derive(Clone, Copy)]
pub struct Ent {
    pub name: [u8; 11],
    pub attr: u8,
    pub time: u16,
    pub date: u16,
    pub cluster: u16,
    pub size: u32,
    /// Directory cluster holding this entry (0 = root), 0xFFFF = synthetic.
    pub dir: u16,
    pub idx: u16,
}

pub const EMPTY: Ent = Ent {
    name: [b' '; 11],
    attr: 0,
    time: 0,
    date: 0,
    cluster: 0,
    size: 0,
    dir: 0xFFFF,
    idx: 0xFFFF,
};

unsafe fn dir_loc(dir: u16, idx: u16) -> Option<(u32, usize)> {
    if dir == 0 {
        if idx >= G.root_ent {
            return None;
        }

        return Some((G.root_lba + (idx / 16) as u32, (idx % 16) as usize * 32));
    }

    let per = 16 * G.spc as u32;
    let ci = idx as u32 / per;
    let mut c = dir;

    for _ in 0..ci {
        c = fat_get(c);

        if is_eoc(c) {
            return None;
        }
    }

    let within = idx as u32 % per;

    Some((cl_lba(c) + within / 16, (within % 16) as usize * 32))
}

pub unsafe fn ent_read(dir: u16, idx: u16) -> Option<Ent> {
    let (lba, off) = dir_loc(dir, idx)?;

    if !rd_sec(lba) {
        return None;
    }

    let p = sb().add(off) as *const u8;
    let mut e = EMPTY;

    for i in 0..11 {
        e.name[i] = p.add(i).read();
    }

    e.attr = p.add(11).read();
    e.time = u16at(p, 22);
    e.date = u16at(p, 24);
    e.cluster = u16at(p, 26);
    e.size = u32at(p, 28);
    e.dir = dir;
    e.idx = idx;

    Some(e)
}

pub unsafe fn ent_write(e: &Ent) -> bool {
    let (lba, off) = match dir_loc(e.dir, e.idx) {
        Some(v) => v,
        None => return false,
    };

    if !rd_sec(lba) {
        return false;
    }

    let p = sb().add(off);

    for i in 0..11 {
        p.add(i).write(e.name[i]);
    }

    p.add(11).write(e.attr);
    put16(p, 22, e.time);
    put16(p, 24, e.date);
    put16(p, 26, e.cluster);
    put32(p, 28, e.size);

    wr_sec(lba)
}

fn name_match(pat: &[u8; 11], name: &[u8; 11]) -> bool {
    for i in 0..11 {
        if pat[i] != b'?' && pat[i] != name[i] {
            return false;
        }
    }

    true
}

/// Find the next live entry at or after `start` matching `pat` and the
/// attribute `mask` (hidden/system/directory entries need their bit in `mask`).
pub unsafe fn dir_search(dir: u16, pat: &[u8; 11], mask: u8, start: u16) -> Option<Ent> {
    let mut i = start;

    loop {
        let e = ent_read(dir, i)?;

        if e.name[0] == 0 {
            return None;
        }

        if i == 0xFFFF {
            return None;
        }

        i += 1;

        if e.name[0] == 0xE5 || e.attr & 0x3F == 0x0F {
            continue;
        }

        if e.attr & ATTR_VOL != 0 && mask & ATTR_VOL == 0 {
            continue;
        }

        if e.attr & !mask & (ATTR_HID | ATTR_SYS | ATTR_DIR) != 0 {
            continue;
        }

        if name_match(pat, &e.name) {
            return Some(e);
        }
    }
}

unsafe fn stamp() -> (u16, u16) {
    let (y, mo, d) = time::get_date();
    let (h, mi, s) = time::get_time();
    let yy = if y >= 1980 { y - 1980 } else { 0 };

    (
        ((h as u16) << 11) | ((mi as u16) << 5) | (s as u16 / 2),
        (yy << 9) | ((mo as u16) << 5) | d as u16,
    )
}

/// Add a new directory entry, growing a subdirectory if it is full.
unsafe fn add_entry(dir: u16, name: &[u8; 11], attr: u8, cluster: u16, size: u32) -> Result<Ent, u16> {
    let mut i: u16 = 0;

    let slot: u16 = loop {
        match ent_read(dir, i) {
            Some(e) => {
                if e.name[0] == 0 || e.name[0] == 0xE5 {
                    break i;
                }
            }
            None => {
                if dir == 0 {
                    return Err(E_ACCESS);
                }

                let last = last_cluster(dir);
                let n = match alloc_cluster() {
                    Some(n) => n,
                    None => return Err(E_ACCESS),
                };

                zero_cluster(n);
                fat_set(last, n);
                break i;
            }
        }

        if i == 0xFFFF {
            return Err(E_ACCESS);
        }

        i += 1;
    };

    let (t, d) = stamp();

    let e = Ent {
        name: *name,
        attr,
        time: t,
        date: d,
        cluster,
        size,
        dir,
        idx: slot,
    };

    if !ent_write(&e) {
        return Err(E_FAIL);
    }

    Ok(e)
}

// ------------------------------------------------------------------ names

fn valid_char(c: u8) -> bool {
    (c >= b'A' && c <= b'Z')
        || (c >= b'0' && c <= b'9')
        || c == b'?'
        || c == b'_'
        || c == b'-'
        || c == b'$'
        || c == b'~'
        || c == b'!'
        || c == b'#'
        || c == b'%'
        || c == b'&'
        || c == b'('
        || c == b')'
        || c == b'@'
        || c == b'^'
        || c == b'{'
        || c == b'}'
        || c == b'\''
        || c == b'`'
}

/// Convert "name.ext" (with * and ? wildcards) to an 8.3 space-padded name.
pub fn to_83(s: &[u8], out: &mut [u8; 11]) -> bool {
    *out = [b' '; 11];

    if s.is_empty() {
        return false;
    }

    if s == b"." {
        out[0] = b'.';
        return true;
    }

    if s == b".." {
        out[0] = b'.';
        out[1] = b'.';
        return true;
    }

    let mut i = 0;
    let mut n = 0;
    let mut star = false;

    while i < s.len() && s[i] != b'.' {
        let c = upper(s[i]);

        if c == b'*' {
            while n < 8 {
                out[n] = b'?';
                n += 1;
            }

            star = true;
        } else if star {
            // characters after '*' are ignored
        } else if valid_char(c) {
            if n >= 8 {
                return false;
            }

            out[n] = c;
            n += 1;
        } else {
            return false;
        }

        i += 1;
    }

    if i < s.len() {
        i += 1; // skip '.'

        let mut m = 0;
        star = false;

        while i < s.len() {
            let c = upper(s[i]);

            if c == b'.' {
                return false;
            }

            if c == b'*' {
                while m < 3 {
                    out[8 + m] = b'?';
                    m += 1;
                }

                star = true;
            } else if star {
            } else if valid_char(c) {
                if m >= 3 {
                    return false;
                }

                out[8 + m] = c;
                m += 1;
            } else {
                return false;
            }

            i += 1;
        }
    }

    true
}

pub fn has_wild(n: &[u8; 11]) -> bool {
    for i in 0..11 {
        if n[i] == b'?' {
            return true;
        }
    }

    false
}

/// "NAME    EXT" -> "NAME.EXT"; returns the length.
pub fn fmt_name(name: &[u8; 11], out: &mut [u8; 13]) -> usize {
    let mut n = 0;

    for i in 0..8 {
        if name[i] != b' ' {
            out[n] = name[i];
            n += 1;
        }
    }

    if name[8] != b' ' {
        out[n] = b'.';
        n += 1;

        for i in 8..11 {
            if name[i] != b' ' {
                out[n] = name[i];
                n += 1;
            }
        }
    }

    n
}

// ------------------------------------------------------------------ paths

pub struct Res {
    pub dir: u16,
    pub leaf: [u8; 11],
    pub has_leaf: bool,
}

unsafe fn parent_of(cur: u16) -> Result<u16, u16> {
    if cur == 0 {
        return Ok(0);
    }

    let mut pat = [b' '; 11];
    pat[0] = b'.';
    pat[1] = b'.';

    match dir_search(cur, &pat, ATTR_DIR, 0) {
        Some(e) => Ok(e.cluster),
        None => Err(E_PNF),
    }
}

/// Split a path into (directory cluster, leaf name). `A:` prefix, `\` or `/`
/// separators, `.` and `..` are understood. Relative paths start at the CWD.
pub unsafe fn resolve(path: &[u8]) -> Result<Res, u16> {
    let mut p = path;

    if p.len() >= 2 && p[1] == b':' {
        if upper(p[0]) != b'A' {
            return Err(E_DRIVE);
        }

        p = &p[2..];
    }

    let mut cur = CWD;

    if !p.is_empty() && (p[0] == b'\\' || p[0] == b'/') {
        cur = 0;
        p = &p[1..];
    }

    loop {
        let mut e = 0;

        while e < p.len() && p[e] != b'\\' && p[e] != b'/' {
            e += 1;
        }

        let comp = &p[..e];
        let last = e >= p.len();

        if comp.is_empty() {
            if last {
                return Ok(Res { dir: cur, leaf: [b' '; 11], has_leaf: false });
            }

            p = &p[e + 1..];
            continue;
        }

        if last {
            if comp == b"." {
                return Ok(Res { dir: cur, leaf: [b' '; 11], has_leaf: false });
            }

            if comp == b".." {
                let d = parent_of(cur)?;
                return Ok(Res { dir: d, leaf: [b' '; 11], has_leaf: false });
            }

            let mut l = [b' '; 11];

            if !to_83(comp, &mut l) {
                return Err(E_PNF);
            }

            return Ok(Res { dir: cur, leaf: l, has_leaf: true });
        }

        if comp == b"." {
            // stay
        } else if comp == b".." {
            cur = parent_of(cur)?;
        } else {
            let mut l = [b' '; 11];

            if !to_83(comp, &mut l) || has_wild(&l) {
                return Err(E_PNF);
            }

            match dir_search(cur, &l, ATTR_DIR | ATTR_HID | ATTR_SYS, 0) {
                Some(d) if d.attr & ATTR_DIR != 0 => cur = d.cluster,
                _ => return Err(E_PNF),
            }
        }

        p = &p[e + 1..];
    }
}

/// Look up an existing file or directory (no wildcards).
pub unsafe fn lookup(path: &[u8]) -> Result<Ent, u16> {
    let r = resolve(path)?;

    if !r.has_leaf {
        let mut e = EMPTY;
        e.attr = ATTR_DIR;
        e.cluster = r.dir;
        return Ok(e);
    }

    if has_wild(&r.leaf) {
        return Err(E_FNF);
    }

    match dir_search(r.dir, &r.leaf, ATTR_DIR | ATTR_HID | ATTR_SYS, 0) {
        Some(e) => Ok(e),
        None => Err(E_FNF),
    }
}

pub unsafe fn chdir(path: &[u8]) -> Result<(), u16> {
    let e = lookup(path)?;

    if e.attr & ATTR_DIR == 0 {
        return Err(E_PNF);
    }

    CWD = e.cluster;
    Ok(())
}

pub unsafe fn cwd_cluster() -> u16 {
    CWD
}

/// Current directory without drive or leading backslash, e.g. `GAMES\DOOM`.
pub unsafe fn cwd_string(out: &mut [u8; 64]) -> usize {
    let mut tmp = [0u8; 64];
    let mut pos = 64;
    let mut cur = CWD;
    let mut guard = 0;

    while cur != 0 && guard < 12 {
        guard += 1;

        let p = match parent_of(cur) {
            Ok(p) => p,
            Err(_) => break,
        };

        let mut i: u16 = 0;
        let mut name = [0u8; 13];
        let mut n = 0;

        while let Some(e) = ent_read(p, i) {
            if e.name[0] == 0 {
                break;
            }

            if e.name[0] != 0xE5 && e.attr & ATTR_DIR != 0 && e.cluster == cur && e.name[0] != b'.' {
                n = fmt_name(&e.name, &mut name);
                break;
            }

            if i == 0xFFFF {
                break;
            }

            i += 1;
        }

        if n == 0 || n + 1 > pos {
            break;
        }

        pos -= n;

        for k in 0..n {
            tmp[pos + k] = name[k];
        }

        if pos == 0 {
            break;
        }

        pos -= 1;
        tmp[pos] = b'\\';

        cur = p;
    }

    // tmp[pos..] looks like "\A\B"; drop the leading backslash.
    let start = if pos < 64 && tmp[pos] == b'\\' { pos + 1 } else { pos };
    let len = 64 - start;

    for i in 0..len {
        out[i] = tmp[start + i];
    }

    len
}

// ------------------------------------------------------------------ files

#[derive(Clone, Copy)]
pub struct File {
    pub dir: u16,
    pub idx: u16,
    pub first: u16,
    pub size: u32,
    pub pos: u32,
    pub cur_idx: u16,
    pub cur: u16,
    pub dirty: bool,
}

impl File {
    pub const ZERO: File = File {
        dir: 0xFFFF,
        idx: 0,
        first: 0,
        size: 0,
        pos: 0,
        cur_idx: 0,
        cur: 0,
        dirty: false,
    };
}

pub unsafe fn open_file(path: &[u8], write: bool) -> Result<File, u16> {
    let e = lookup(path)?;

    if e.attr & ATTR_DIR != 0 {
        return Err(E_ACCESS);
    }

    if write && e.attr & ATTR_RO != 0 {
        return Err(E_ACCESS);
    }

    let mut f = File::ZERO;
    f.dir = e.dir;
    f.idx = e.idx;
    f.first = e.cluster;
    f.size = e.size;
    Ok(f)
}

/// Create a file, or truncate it if it already exists.
pub unsafe fn create_file(path: &[u8], attr: u8) -> Result<File, u16> {
    let r = resolve(path)?;

    if !r.has_leaf || has_wild(&r.leaf) {
        return Err(E_PNF);
    }

    let mut f = File::ZERO;

    match dir_search(r.dir, &r.leaf, ATTR_DIR | ATTR_HID | ATTR_SYS, 0) {
        Some(mut e) => {
            if e.attr & (ATTR_DIR | ATTR_RO) != 0 {
                return Err(E_ACCESS);
            }

            if !is_eoc(e.cluster) {
                free_chain(e.cluster);
            }

            e.cluster = 0;
            e.size = 0;
            let (t, d) = stamp();
            e.time = t;
            e.date = d;

            if !ent_write(&e) {
                return Err(E_FAIL);
            }

            f.dir = e.dir;
            f.idx = e.idx;
        }
        None => {
            let e = add_entry(r.dir, &r.leaf, (attr & 0x27) | ATTR_ARC, 0, 0)?;
            f.dir = e.dir;
            f.idx = e.idx;
        }
    }

    flush_fat();
    Ok(f)
}

unsafe fn file_cluster(f: &mut File, ci: u16) -> Option<u16> {
    if is_eoc(f.first) {
        return None;
    }

    let (mut i, mut c) = if !is_eoc(f.cur) && f.cur_idx <= ci {
        (f.cur_idx, f.cur)
    } else {
        (0, f.first)
    };

    while i < ci {
        c = fat_get(c);

        if is_eoc(c) {
            return None;
        }

        i += 1;
    }

    f.cur_idx = ci;
    f.cur = c;
    Some(c)
}

unsafe fn ensure_cluster(f: &mut File, ci: u16) -> Option<u16> {
    loop {
        if let Some(c) = file_cluster(f, ci) {
            return Some(c);
        }

        let n = alloc_cluster()?;
        zero_cluster(n);

        if is_eoc(f.first) {
            f.first = n;
            f.cur = n;
            f.cur_idx = 0;
        } else {
            let l = last_cluster(f.first);
            fat_set(l, n);
        }

        f.dirty = true;
    }
}

/// Read up to `count` bytes into `seg:off`. Returns the number read.
pub unsafe fn read(f: &mut File, seg: u16, off: u16, count: u16) -> Result<u16, u16> {
    let csz = bytes_per_cluster();
    let avail = if f.pos >= f.size { 0 } else { f.size - f.pos };
    let mut left = if (count as u32) < avail { count as u32 } else { avail };
    let mut o = off;
    let mut done: u32 = 0;

    while left > 0 {
        let ci = (f.pos / csz) as u16;

        let c = match file_cluster(f, ci) {
            Some(c) => c,
            None => break,
        };

        let within = f.pos % csz;
        let sec = within / 512;
        let so = (within % 512) as usize;
        let mut chunk = 512 - so as u32;

        if chunk > left {
            chunk = left;
        }

        if !rd_sec(cl_lba(c) + sec) {
            return Err(E_FAIL);
        }

        copy_to(seg, o, sb().add(so), chunk as u16);

        o = o.wrapping_add(chunk as u16);
        f.pos += chunk;
        left -= chunk;
        done += chunk;
    }

    Ok(done as u16)
}

/// Write `count` bytes from `seg:off` at the current position.
pub unsafe fn write(f: &mut File, seg: u16, off: u16, count: u16) -> Result<u16, u16> {
    let csz = bytes_per_cluster();
    let mut left = count as u32;
    let mut o = off;
    let mut done: u32 = 0;

    while left > 0 {
        let ci = (f.pos / csz) as u16;

        let c = match ensure_cluster(f, ci) {
            Some(c) => c,
            None => break,
        };

        let within = f.pos % csz;
        let sec = within / 512;
        let so = (within % 512) as usize;
        let mut chunk = 512 - so as u32;

        if chunk > left {
            chunk = left;
        }

        let lba = cl_lba(c) + sec;

        if chunk < 512 {
            if !rd_sec(lba) {
                return Err(E_FAIL);
            }
        } else {
            invalidate();
        }

        copy_from(seg, o, sb().add(so), chunk as u16);

        if !wr_sec(lba) {
            return Err(E_FAIL);
        }

        o = o.wrapping_add(chunk as u16);
        f.pos += chunk;
        left -= chunk;
        done += chunk;
    }

    if done > 0 {
        if f.pos > f.size {
            f.size = f.pos;
        }

        f.dirty = true;
    }

    flush_fat();
    Ok(done as u16)
}

/// Cut the file at the current position (DOS: write of 0 bytes).
pub unsafe fn truncate(f: &mut File) {
    let csz = bytes_per_cluster();
    let keep = (f.pos + csz - 1) / csz;

    if keep == 0 {
        if !is_eoc(f.first) {
            free_chain(f.first);
        }

        f.first = 0;
        f.cur = 0;
        f.cur_idx = 0;
    } else if let Some(c) = file_cluster(f, (keep - 1) as u16) {
        let n = fat_get(c);
        fat_set(c, 0xFFF);

        if !is_eoc(n) {
            free_chain(n);
        }
    }

    f.size = f.pos;
    f.dirty = true;
    flush_fat();
}

/// origin: 0 = start, 1 = current, 2 = end.
pub unsafe fn seek(f: &mut File, origin: u8, off: i32) -> Result<u32, u16> {
    let base: i64 = match origin {
        0 => 0,
        1 => f.pos as i64,
        2 => f.size as i64,
        _ => return Err(E_INVALID),
    };

    let n = base + off as i64;

    if n < 0 || n > 0xFFFF_FFFF {
        return Err(E_INVALID);
    }

    f.pos = n as u32;
    Ok(f.pos)
}

/// Flush the directory entry (size, first cluster, time) and the FAT.
pub unsafe fn close(f: &mut File) -> bool {
    if f.dir != 0xFFFF && f.dirty {
        if let Some(mut e) = ent_read(f.dir, f.idx) {
            let (t, d) = stamp();
            e.cluster = f.first;
            e.size = f.size;
            e.time = t;
            e.date = d;
            e.attr |= ATTR_ARC;
            ent_write(&e);
        }
    }

    f.dirty = false;
    flush_fat()
}

pub unsafe fn file_stamp(f: &File) -> (u16, u16) {
    match ent_read(f.dir, f.idx) {
        Some(e) => (e.time, e.date),
        None => (0, 0),
    }
}

pub unsafe fn set_file_stamp(f: &File, t: u16, d: u16) -> bool {
    match ent_read(f.dir, f.idx) {
        Some(mut e) => {
            e.time = t;
            e.date = d;
            ent_write(&e)
        }
        None => false,
    }
}

// ----------------------------------------------------- directory operations

/// Delete every file matching `path` (wildcards allowed). Returns the count.
pub unsafe fn delete_path(path: &[u8]) -> Result<u16, u16> {
    let r = resolve(path)?;

    if !r.has_leaf {
        return Err(E_FNF);
    }

    let mut count = 0;
    let mut start: u16 = 0;

    while let Some(mut e) = dir_search(r.dir, &r.leaf, 0, start) {
        start = e.idx + 1;

        if e.attr & ATTR_RO != 0 {
            return Err(E_ACCESS);
        }

        e.name[0] = 0xE5;
        ent_write(&e);

        if !is_eoc(e.cluster) {
            free_chain(e.cluster);
        }

        count += 1;
    }

    flush_fat();

    if count == 0 {
        return Err(E_FNF);
    }

    Ok(count)
}

pub unsafe fn mkdir(path: &[u8]) -> Result<(), u16> {
    let r = resolve(path)?;

    if !r.has_leaf || has_wild(&r.leaf) {
        return Err(E_PNF);
    }

    if dir_search(r.dir, &r.leaf, ATTR_DIR | ATTR_HID | ATTR_SYS, 0).is_some() {
        return Err(E_ACCESS);
    }

    let c = match alloc_cluster() {
        Some(c) => c,
        None => return Err(E_ACCESS),
    };

    zero_cluster(c);

    let (t, d) = stamp();

    let mut dot = EMPTY;
    dot.name[0] = b'.';
    dot.attr = ATTR_DIR;
    dot.time = t;
    dot.date = d;
    dot.cluster = c;
    dot.dir = c;
    dot.idx = 0;
    ent_write(&dot);

    let mut dd = dot;
    dd.name[1] = b'.';
    dd.cluster = r.dir;
    dd.idx = 1;
    ent_write(&dd);

    if let Err(e) = add_entry(r.dir, &r.leaf, ATTR_DIR, c, 0) {
        fat_set(c, 0);
        flush_fat();
        return Err(e);
    }

    flush_fat();
    Ok(())
}

pub unsafe fn rmdir(path: &[u8]) -> Result<(), u16> {
    let mut e = lookup(path)?;

    if e.attr & ATTR_DIR == 0 {
        return Err(E_PNF);
    }

    if e.dir == 0xFFFF {
        return Err(E_ACCESS); // the root directory
    }

    if e.cluster == CWD {
        return Err(E_RMCWD);
    }

    let mut i: u16 = 0;

    while let Some(x) = ent_read(e.cluster, i) {
        if x.name[0] == 0 {
            break;
        }

        let dots = x.name[0] == b'.';

        if x.name[0] != 0xE5 && !dots {
            return Err(E_ACCESS);
        }

        if i == 0xFFFF {
            break;
        }

        i += 1;
    }

    e.name[0] = 0xE5;
    ent_write(&e);
    free_chain(e.cluster);
    flush_fat();
    Ok(())
}

pub unsafe fn rename(old: &[u8], new: &[u8]) -> Result<(), u16> {
    let mut e = lookup(old)?;

    if e.dir == 0xFFFF {
        return Err(E_ACCESS);
    }

    let r = resolve(new)?;

    if !r.has_leaf || has_wild(&r.leaf) {
        return Err(E_ACCESS);
    }

    if dir_search(r.dir, &r.leaf, ATTR_DIR | ATTR_HID | ATTR_SYS, 0).is_some() {
        return Err(E_ACCESS);
    }

    if r.dir == e.dir {
        e.name = r.leaf;
        return if ent_write(&e) { Ok(()) } else { Err(E_FAIL) };
    }

    if e.attr & ATTR_DIR != 0 {
        return Err(E_ACCESS);
    }

    add_entry(r.dir, &r.leaf, e.attr, e.cluster, e.size)?;

    e.name[0] = 0xE5;
    ent_write(&e);
    flush_fat();
    Ok(())
}

pub unsafe fn get_attr(path: &[u8]) -> Result<u8, u16> {
    Ok(lookup(path)?.attr)
}

pub unsafe fn set_attr(path: &[u8], attr: u8) -> Result<(), u16> {
    let mut e = lookup(path)?;

    if e.dir == 0xFFFF {
        return Err(E_ACCESS);
    }

    e.attr = (attr & 0x27) | (e.attr & ATTR_DIR);
    ent_write(&e);
    Ok(())
}

// ------------------------------------------------------------------ search

#[derive(Clone, Copy)]
pub struct Search {
    pub dir: u16,
    pub pat: [u8; 11],
    pub mask: u8,
    pub next: u16,
}

pub unsafe fn find_first(path: &[u8], mask: u8) -> Result<(Search, Ent), u16> {
    let r = resolve(path)?;
    let pat = if r.has_leaf { r.leaf } else { [b'?'; 11] };

    let mut s = Search { dir: r.dir, pat, mask: mask & 0x1E, next: 0 };

    match find_next(&mut s) {
        Some(e) => Ok((s, e)),
        None => Err(E_FNF),
    }
}

pub unsafe fn find_next(s: &mut Search) -> Option<Ent> {
    let e = dir_search(s.dir, &s.pat, s.mask, s.next)?;
    s.next = e.idx + 1;
    Some(e)
}

/// Directory cluster that a path like `A:\SUB\*.COM` searches in (for DIR headers).
pub unsafe fn search_dir(path: &[u8]) -> Result<u16, u16> {
    Ok(resolve(path)?.dir)
}
