use super::farptr::FarPtr;

pub fn int20_handler() -> FarPtr {
    extern "C" { static int20_start: u8; }
    unsafe { FarPtr::new(0x1000, &int20_start as *const u8 as u16) }
}

pub fn int21_handler() -> FarPtr {
    extern "C" { static int21_start: u8; }
    unsafe { FarPtr::new(0x1000, &int21_start as *const u8 as u16) }
}

core::arch::global_asm!(
    ".section .text.tramp, \"ax\"",
    ".code16",
    ".global int20_start",
    "int20_start:",
    "    pusha",
    "    push ds",
    "    push es",
    "    pushf",
    "    mov ax, sp",
    "    push ax",
    "    call int20_handler",
    "    add sp, 2",
    "    popf",
    "    pop es",
    "    pop ds",
    "    popa",
    "    iret",
    ".global int21_start",
    "int21_start:",
    "    pusha",
    "    push ds",
    "    push es",
    "    pushf",
    "    mov ax, sp",
    "    push ax",
    "    call int21",
    "    add sp, 2",
    "    popf",
    "    pop es",
    "    pop ds",
    "    popa",
    "    iret"
);
