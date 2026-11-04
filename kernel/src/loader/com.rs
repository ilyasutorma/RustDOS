use crate::arch::farmem::{copy_to, fill, wr16, wr8};
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

    // Program segment: zero 64 KB, build the PSP header, copy the image to :0100.
    fill(seg, 0, 0, 0x8000);
    fill(seg, 0x8000, 0, 0x8000);

    wr8(seg, 0, 0xCD);
    wr8(seg, 1, 0x20);
    wr16(seg, 2, seg + 0x1000);

    copy_to(seg, 0x100, bytes.as_ptr(), bytes.len() as u16);

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
