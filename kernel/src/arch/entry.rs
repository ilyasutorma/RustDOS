core::arch::global_asm!(
    ".section .text.entry, \"ax\"",
    ".code16",
    ".global _start",
    "_start:",
    " cli",
    " mov ax, cs",
    " mov ds, ax",
    " mov es, ax",
    " mov ss, ax",
    " mov esp, 0xFFF0",
    " sti",
    " cld",
    " mov byte ptr [BOOT_DRIVE], dl",
    " mov di, offset __bss_start",
    " mov cx, offset __bss_end",
    " sub cx, di",
    " xor al, al",
    " rep stosb",
    " mov ax, offset kernel_main",
    " jmp ax",
);

#[no_mangle]
#[link_section = ".data"]
pub static mut BOOT_DRIVE: u8 = 0;
