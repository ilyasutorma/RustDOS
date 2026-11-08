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

/// Allocate a 64 KB program segment and build a PSP in it.
unsafe fn prepare() -> Option<u16> {
    let seg = mcb::alloc(0x1000, 8, b"COM")?;

    CURRENT = seg;

    // Pointers are DS-relative, so the program segment is reached via seg:off helpers.
    fill(seg, 0, 0, 0x8000);
    fill(seg, 0x8000, 0, 0x8000);

    wr8(seg, 0, 0xCD);
    wr8(seg, 1, 0x20);
    wr16(seg, 2, seg + 0x1000);

    Some(seg)
}

pub unsafe fn run_com(bytes: &[u8]) {
    if bytes.len() > 0xFE00 {
        return;
    }

    let seg = match prepare() {
        Some(s) => s,
        None => return,
    };

    copy_to(seg, 0x100, bytes.as_ptr(), bytes.len() as u16);

    exec(seg);
}

pub unsafe fn run_from_disk(name: &[u8]) -> bool {
    let (cluster, size) = match crate::fs::file::find_root(name) {
        Some(v) => v,
        None => return false,
    };

    if size > 0xFE00 {
        return false;
    }

    let seg = match prepare() {
        Some(s) => s,
        None => return false,
    };

    if !crate::fs::file::load_to_mem(cluster, size, seg, 0x100) {
        mcb::free(seg);
        CURRENT = 0;
        return false;
    }

    exec(seg);
}

/// Jump to seg:0100 with DS=ES=SS=seg and SP=FFFE. The word at SS:FFFE is 0 (the PSP is
/// zeroed), so a plain `ret` from the program lands on the `INT 20h` at PSP:0000.
unsafe fn exec(seg: u16) -> ! {
    core::arch::asm!(
        "cli",
        "mov ds, ax",
        "mov es, ax",
        "mov ss, ax",
        "mov sp, 0xFFFE",
        "sti",
        "push ax",
        "push 0x0100",
        ".byte 0xCB",
        in("ax") seg,
        options(noreturn)
    );
}

/// Called from INT 20h / INT 21h AH=4Ch. Those handlers already run on the kernel
/// stack with DS=ES=SS=0x1000, so no segment switching is needed here.
pub fn terminate() -> ! {
    unsafe {
        let seg = CURRENT;

        if seg != 0 {
            mcb::free(seg);
            CURRENT = 0;
        }
    }

    crate::bios::tty::print("\r\nProgram returned\r\n");

    crate::shell::commands::run();
}
