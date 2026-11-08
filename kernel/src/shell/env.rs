#![allow(static_mut_refs)]

use crate::bios::tty;

static mut ENV: [[u8; 64]; 32] = [[0; 64]; 32];
static mut PATH: [u8; 128] = [0; 128];

pub fn upper(c: u8) -> u8 {
    if c >= b'a' && c <= b'z' {
        c - 32
    } else {
        c
    }
}

pub unsafe fn init_path() {
    let p = PATH.as_mut_ptr();
    core::ptr::copy_nonoverlapping(b"C:\\".as_ptr(), p, 3);
}

pub unsafe fn print_path() {
    tty::print("\r\nPATH=");

    let p = PATH.as_ptr();
    let mut i = 0;

    while i < 128 && p.add(i).read() != 0 {
        tty::putc(p.add(i).read());
        i += 1;
    }

    tty::print("\r\n");
}

pub unsafe fn set_path(args: &[u8]) {
    let p = PATH.as_mut_ptr();
    let n = args.len().min(127);

    core::ptr::copy_nonoverlapping(args.as_ptr(), p, n);
    p.add(n).write(0);
}

pub unsafe fn print_env() {
    tty::print("\r\nEnvironment:\r\n");

    for i in 0..32 {
        let e = &ENV[i];

        if e[0] != 0 {
            for j in 0..64 {
                if e[j] == 0 {
                    break;
                }

                tty::putc(e[j]);
            }

            tty::print("\r\n");
        }
    }
}

pub unsafe fn set_env(args: &[u8]) {
    let (name, val) = split_eq(args);

    if name.is_empty() {
        return;
    }

    for i in 0..32 {
        let e = &mut ENV[i];

        if env_eq(e, name) {
            if val.is_empty() {
                e[0] = 0;
            } else {
                env_set(e, name, val);
            }

            return;
        }

        if e[0] == 0 {
            env_set(e, name, val);
            return;
        }
    }
}

pub unsafe fn expand(src: &[u8], dst: &mut [u8]) -> usize {
    let mut i = 0;
    let mut j = 0;

    while i < src.len() && j < dst.len() {
        if src[i] == b'%' {
            i += 1;

            if i >= src.len() {
                break;
            }

            let start = i;

            while i < src.len() && src[i] != b'%' {
                i += 1;
            }

            if i < src.len() {
                let val = env_get(&src[start..i]);

                for k in 0..val.len() {
                    if j < dst.len() {
                        dst[j] = val[k];
                        j += 1;
                    }
                }

                i += 1;
            }
        } else {
            dst[j] = src[i];
            i += 1;
            j += 1;
        }
    }

    j
}

unsafe fn env_eq(env: &[u8; 64], name: &[u8]) -> bool {
    let mut i = 0;

    while i < name.len() && i < 64 {
        if env[i] != upper(name[i]) {
            return false;
        }

        i += 1;
    }

    i < 64 && env[i] == b'='
}

unsafe fn env_set(env: &mut [u8; 64], name: &[u8], val: &[u8]) {
    let mut i = 0;

    for k in 0..name.len().min(31) {
        env[i] = upper(name[k]);
        i += 1;
    }

    env[i] = b'=';
    i += 1;

    for k in 0..val.len().min(63 - i) {
        env[i] = val[k];
        i += 1;
    }

    env[i] = 0;
}

unsafe fn env_get(name: &[u8]) -> &'static [u8] {
    for i in 0..32 {
        let e = &ENV[i];

        if env_eq(e, name) {
            let mut j = 0;

            while j < 64 && e[j] != b'=' {
                j += 1;
            }

            if j < 64 {
                j += 1;

                let mut k = j;

                while k < 64 && e[k] != 0 {
                    k += 1;
                }

                return core::slice::from_raw_parts(e.as_ptr().add(j), k - j);
            }
        }
    }

    &[]
}

pub fn split_eq(s: &[u8]) -> (&[u8], &[u8]) {
    let mut i = 0;

    while i < s.len() && s[i] != b'=' {
        i += 1;
    }

    if i < s.len() {
        (&s[..i], &s[i + 1..])
    } else {
        (s, &[])
    }
}
