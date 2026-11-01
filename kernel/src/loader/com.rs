use crate::mem::mcb;

static mut CURRENT: u16 = 0;

pub fn test() {
    let prog: &[u8] = &[
        0xB4, 0x0E, 0xB0, 0x48, 0xCD, 0x10,
        0xB0, 0x49, 0xCD, 0x10,
        0xB0, 0x0D, 0xCD, 0x10,
        0xB0, 0x0A, 0xCD, 0x10,
        0xC3,
    ];

    unsafe {
        run_com(prog);
    }
}

pub unsafe fn run_com(bytes: &[u8]) {
    if bytes.len() > 0xFF00 {
        return;
    }

    let seg = match mcb::alloc(0x1000, 8, b"COM") {
        Some(s) => s,
        None => return,
    };

    CURRENT = seg;

    let base = ((seg as u32) << 4) as *mut u8;

    core::ptr::write_bytes(base, 0, 0x10000);

    base.write(0xCD);
    base.add(1).write(0x20);

    let top = seg + 0x1000;
    base.add(2).write(top as u8);
    base.add(3).write((top >> 8) as u8);

    core::ptr::copy_nonoverlapping(
        bytes.as_ptr(),
                                   base.add(0x100),
                                   bytes.len(),
    );

    exec(seg);
}

unsafe fn exec(seg: u16) -> ! {
    let ret = ret_to_shell as *const () as usize as u16;

    core::arch::asm!(
        "cli",
        "mov ds, ax",
        "mov es, ax",
        "mov ss, ax",
        "mov sp, 0xFFFE",
        "sti",
        "push 0x1000",
        "push {ret:x}",
        "push ax",
        "push 0x0100",
        "retf",
        in("ax") seg,
                     ret = in(reg) ret,
                     clobber_abi("C"),
                     options(noreturn)
    );
}

extern "C" fn ret_to_shell() -> ! {
    unsafe {
        core::arch::asm!(
            "cli",
            "mov ax, 0x1000",
            "mov ds, ax",
            "mov es, ax",
            "mov ss, ax",
            "mov sp, 0xFFF0",
            "sti",
            clobber_abi("C")
        );

        let seg = CURRENT;

        if seg != 0 {
            mcb::free(seg);
            CURRENT = 0;
        }
    }

    crate::bios::tty::print("\r\nProgram returned\r\n");

    crate::shell::commands::run();
}
