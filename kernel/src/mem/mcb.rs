//! Memory Control Blocks. Every function works on MCB *segments* and reaches
//! the header through `arch::farmem` (Rust pointers are DS-relative and cannot
//! address conventional memory above the kernel segment).

use crate::arch::farmem::{fill, rd16, rd8, wr16, wr8};

// Header layout (16 bytes): kind u8, owner u16, size u16, reserved [u8;3], name [u8;8]
const OFF_KIND: u16 = 0;
const OFF_OWNER: u16 = 1;
const OFF_SIZE: u16 = 3;
const OFF_RESERVED: u16 = 5;
const OFF_NAME: u16 = 8;

static mut FIRST: u16 = 0;

unsafe fn kind(s: u16) -> u8 { rd8(s, OFF_KIND) }
unsafe fn owner(s: u16) -> u16 { rd16(s, OFF_OWNER) }
unsafe fn size(s: u16) -> u16 { rd16(s, OFF_SIZE) }
unsafe fn set_kind(s: u16, v: u8) { wr8(s, OFF_KIND, v) }
unsafe fn set_owner(s: u16, v: u16) { wr16(s, OFF_OWNER, v) }
unsafe fn set_size(s: u16, v: u16) { wr16(s, OFF_SIZE, v) }

unsafe fn clear_name(s: u16) {
    fill(s, OFF_NAME, 0, 8);
}

unsafe fn set_name(s: u16, name: &[u8]) {
    clear_name(s);
    let n = name.len().min(8);
    for i in 0..n {
        wr8(s, OFF_NAME + i as u16, name[i]);
    }
}

/// Write a fresh free-block header.
unsafe fn make_free(s: u16, k: u8, sz: u16) {
    set_kind(s, k);
    set_owner(s, 0);
    set_size(s, sz);
    fill(s, OFF_RESERVED, 0, 3);
    clear_name(s);
}

unsafe fn bad(s: u16) -> bool {
    let k = kind(s);
    k != b'M' && k != b'Z'
}

unsafe fn join(seg: u16) {
    let s = seg;

    loop {
        if bad(s) || kind(s) != b'M' || owner(s) != 0 {
            return;
        }

        let ns = s + 1 + size(s);

        if bad(ns) || owner(ns) != 0 {
            return;
        }

        let k = kind(ns);
        set_size(s, size(s) + 1 + size(ns));
        set_kind(s, k);

        if k == b'Z' {
            return;
        }
    }
}

pub unsafe fn init(first: u16, end: u16) {
    FIRST = first;
    make_free(first, b'Z', end - first - 1);
}

pub unsafe fn alloc(need: u16, owner_seg: u16, name: &[u8]) -> Option<u16> {
    let mut s = FIRST;

    loop {
        if bad(s) {
            return None;
        }

        let sz = size(s);

        if owner(s) == 0 && sz >= need {
            if sz > need + 1 {
                let k = kind(s);
                make_free(s + 1 + need, k, sz - need - 1);

                set_kind(s, b'M');
                set_size(s, need);
            }

            set_owner(s, owner_seg);
            set_name(s, name);

            return Some(s + 1);
        }

        if kind(s) == b'Z' {
            return None;
        }

        s += 1 + sz;
    }
}

pub unsafe fn free(seg: u16) -> bool {
    if seg == 0 {
        return false;
    }

    let s = seg - 1;

    if bad(s) {
        return false;
    }

    set_owner(s, 0);
    clear_name(s);
    collect();

    true
}

pub unsafe fn resize(seg: u16, need: u16) -> Result<u16, u16> {
    if seg == 0 {
        return Err(largest());
    }

    let s = seg - 1;

    if bad(s) {
        return Err(largest());
    }

    let old = size(s);

    if need <= old {
        let left = old - need;

        if left > 1 {
            let k = kind(s);
            make_free(s + 1 + need, k, left - 1);

            set_kind(s, b'M');
            set_size(s, need);
        }

        return Ok(size(s));
    }

    if kind(s) != b'M' {
        return Err(largest());
    }

    let ns = s + 1 + old;

    if bad(ns) || owner(ns) != 0 {
        return Err(largest());
    }

    let total = old + 1 + size(ns);

    if total < need {
        return Err(largest());
    }

    let left = total - need;

    if left > 1 {
        let nk = kind(ns);
        make_free(s + 1 + need, nk, left - 1);

        set_kind(s, b'M');
        set_size(s, need);
    } else {
        set_kind(s, kind(ns));
        set_size(s, total);
    }

    Ok(size(s))
}

pub unsafe fn largest() -> u16 {
    let mut s = FIRST;
    let mut max = 0;

    loop {
        if bad(s) {
            return max;
        }

        if owner(s) == 0 && size(s) > max {
            max = size(s);
        }

        if kind(s) == b'Z' {
            return max;
        }

        s += 1 + size(s);
    }
}

pub unsafe fn collect() {
    let mut s = FIRST;

    loop {
        if bad(s) {
            return;
        }

        if owner(s) == 0 {
            join(s);
        }

        if kind(s) == b'Z' {
            return;
        }

        s += 1 + size(s);
    }
}
