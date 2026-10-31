#[repr(C, packed)]
pub struct Mcb {
    pub kind: u8,
    pub owner: u16,
    pub size: u16,
    pub reserved: [u8; 3],
    pub name: [u8; 8],
}

const _: () = assert!(core::mem::size_of::<Mcb>() == 16);

static mut FIRST: u16 = 0;

#[inline]
unsafe fn ptr(seg: u16) -> *mut Mcb {
    (((seg as u32) << 4) as *mut Mcb)
}

unsafe fn clear_name(p: *mut Mcb) {
    let d = core::ptr::addr_of_mut!((*p).name) as *mut u8;
    core::ptr::write_bytes(d, 0, 8);
}

unsafe fn set_name(p: *mut Mcb, s: &[u8]) {
    let d = core::ptr::addr_of_mut!((*p).name) as *mut u8;
    let n = s.len().min(8);
    core::ptr::write_bytes(d, 0, 8);
    core::ptr::copy_nonoverlapping(s.as_ptr(), d, n);
}

unsafe fn bad(p: *mut Mcb) -> bool {
    let k = (*p).kind;
    k != b'M' && k != b'Z'
}

unsafe fn join(seg: u16) {
    let mut s = seg;

    loop {
        let p = ptr(s);

        if bad(p) || (*p).kind != b'M' || (*p).owner != 0 {
            return;
        }

        let ns = s + 1 + (*p).size;
        let q = ptr(ns);

        if bad(q) || (*q).owner != 0 {
            return;
        }

        let k = (*q).kind;
        (*p).size = (*p).size + 1 + (*q).size;
        (*p).kind = k;

        if k == b'Z' {
            return;
        }

        s = seg;
    }
}

pub unsafe fn init(first: u16, end: u16) {
    FIRST = first;

    let p = ptr(first);
    (*p).kind = b'Z';
    (*p).owner = 0;
    (*p).size = end - first - 1;
    (*p).reserved = [0; 3];
    clear_name(p);
}

pub unsafe fn alloc(need: u16, owner: u16, name: &[u8]) -> Option<u16> {
    let mut s = FIRST;

    loop {
        let p = ptr(s);

        if bad(p) {
            return None;
        }

        let size = (*p).size;

        if (*p).owner == 0 && size >= need {
            if size > need + 1 {
                let k = (*p).kind;
                let ns = s + 1 + need;
                let q = ptr(ns);

                (*q).kind = k;
                (*q).owner = 0;
                (*q).size = size - need - 1;
                (*q).reserved = [0; 3];
                clear_name(q);

                (*p).kind = b'M';
                (*p).size = need;
            }

            (*p).owner = owner;
            set_name(p, name);

            return Some(s + 1);
        }

        if (*p).kind == b'Z' {
            return None;
        }

        s += 1 + size;
    }
}

pub unsafe fn free(seg: u16) -> bool {
    if seg == 0 {
        return false;
    }

    let s = seg - 1;
    let p = ptr(s);

    if bad(p) {
        return false;
    }

    (*p).owner = 0;
    clear_name(p);
    collect();

    true
}

pub unsafe fn resize(seg: u16, need: u16) -> Result<u16, u16> {
    if seg == 0 {
        return Err(largest());
    }

    let s = seg - 1;
    let p = ptr(s);

    if bad(p) {
        return Err(largest());
    }

    let old = (*p).size;

    if need <= old {
        let left = old - need;

        if left > 1 {
            let k = (*p).kind;
            let ns = s + 1 + need;
            let q = ptr(ns);

            (*q).kind = k;
            (*q).owner = 0;
            (*q).size = left - 1;
            (*q).reserved = [0; 3];
            clear_name(q);

            (*p).kind = b'M';
            (*p).size = need;
        }

        return Ok((*p).size);
    }

    if (*p).kind != b'M' {
        return Err(largest());
    }

    let ns = s + 1 + old;
    let q = ptr(ns);

    if bad(q) || (*q).owner != 0 {
        return Err(largest());
    }

    let total = old + 1 + (*q).size;

    if total < need {
        return Err(largest());
    }

    let left = total - need;

    if left > 1 {
        let rs = s + 1 + need;
        let r = ptr(rs);

        (*r).kind = (*q).kind;
        (*r).owner = 0;
        (*r).size = left - 1;
        (*r).reserved = [0; 3];
        clear_name(r);

        (*p).kind = b'M';
        (*p).size = need;
    } else {
        (*p).kind = (*q).kind;
        (*p).size = total;
    }

    Ok((*p).size)
}

pub unsafe fn largest() -> u16 {
    let mut s = FIRST;
    let mut max = 0;

    loop {
        let p = ptr(s);

        if bad(p) {
            return max;
        }

        if (*p).owner == 0 && (*p).size > max {
            max = (*p).size;
        }

        if (*p).kind == b'Z' {
            return max;
        }

        s += 1 + (*p).size;
    }
}

pub unsafe fn collect() {
    let mut s = FIRST;

    loop {
        let p = ptr(s);

        if bad(p) {
            return;
        }

        if (*p).owner == 0 {
            join(s);
        }

        if (*p).kind == b'Z' {
            return;
        }

        s += 1 + (*p).size;
    }
}
