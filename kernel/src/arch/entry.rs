//! Startup code. The boot sector jumps here with CS = 0x1000, DL = boot drive.
//!
//! NOTE: `jmp kernel_main` is used instead of `call` on purpose: `call` in
//! hand-written `.code16` pushes a 2-byte return address, while Rust code
//! compiled for code16 uses 4-byte ones. `kernel_main` never returns.

core::arch::global_asm!(
    ".section .text.entry, \"ax\"",
    ".code16",
    ".global _start",
    "_start:",
    "    cli",
    "    mov ax, cs",
    "    mov ds, ax",
    "    mov es, ax",
    "    mov ss, ax",
    "    mov esp, 0xFFF0",
    "    sti",
    "    cld",
    "    mov byte ptr [BOOT_DRIVE], dl",
    // zero .bss
    "    mov di, offset __bss_start",
    "    mov cx, offset __bss_end",
    "    sub cx, di",
    "    xor al, al",
    "    rep stosb",
    "    jmp kernel_main",
);

#[no_mangle]
#[link_section = ".data"]
pub static mut BOOT_DRIVE: u8 = 0;
