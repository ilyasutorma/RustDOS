//! `.COM` programs started from the shell.

use crate::arch::farmem::copy_to;
use crate::dos::process;

/// Built-in smoke test: prints "HI" through BIOS and returns with `ret`.
pub fn test() {
    let prog: &[u8] = &[
        0xB4, 0x0E, 0xB0, 0x48, 0xCD, 0x10,
        0xB0, 0x49, 0xCD, 0x10,
        0xB0, 0x0D, 0xCD, 0x10,
        0xB0, 0x0A, 0xCD, 0x10,
        0xC3,
    ];

    unsafe {
        let seg = match process::alloc_program(b"TEST") {
            Ok(s) => s,
            Err(_) => return,
        };

        copy_to(seg, 0x100, prog.as_ptr(), prog.len() as u16);
        process::build_psp(seg, b"");
        process::start(seg);
    }
}

/// Load and run `path` with the given command tail. Only returns on failure.
pub unsafe fn run_path(path: &[u8], tail: &[u8]) -> Result<(), u16> {
    process::run_from_shell(path, tail)
}
