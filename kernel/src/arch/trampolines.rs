use super::farptr::FarPtr;

pub fn int20_handler() -> FarPtr {
    extern "C" {
        static int20_start: u8;
    }

    unsafe { FarPtr::new(0x1000, &int20_start as *const u8 as u16) }
}

pub fn int21_handler() -> FarPtr {
    extern "C" {
        static int21_start: u8;
    }

    unsafe { FarPtr::new(0x1000, &int21_start as *const u8 as u16) }
}

core::arch::global_asm!(
    ".section .bss.istack, \"aw\", @nobits",
    "istack_bottom:",
    ".space 1024",
    "istack_top:",

    ".section .text.tramp, \"ax\"",
    ".code16",

    ".global int21_start",
    "int21_start:",
    "    cli",
    "    mov cs:[saved_ss], ss",
    "    mov cs:[saved_sp], sp",
    "    mov ss, cs:[kern_seg]",
    "    mov sp, cs:[kern_sp]",
    "    pushf",
    "    push es",
    "    push ds",
    "    push bp",
    "    push di",
    "    push si",
    "    push dx",
    "    push cx",
    "    push bx",
    "    push ax",
    "    mov ax, cs:[kern_seg]",
    "    mov ds, ax",
    "    mov es, ax",
    "    sti",
    "    xor eax, eax",
    "    mov ax, sp",
    "    push eax",
    "    data32 call int21",
    "    add sp, 4",
    "    cli",
    "    pop ax",
    "    pop bx",
    "    pop cx",
    "    pop dx",
    "    pop si",
    "    pop di",
    "    pop bp",
    "    pop ds",
    "    pop es",
    "    popf",
    "    sti",
    "    mov ss, cs:[saved_ss]",
    "    mov sp, cs:[saved_sp]",
    "    iret",

    ".global int20_start",
    "int20_start:",
    "    cli",
    "    mov cs:[saved_ss], ss",
    "    mov cs:[saved_sp], sp",
    "    mov ss, cs:[kern_seg]",
    "    mov sp, cs:[kern_sp]",
    "    pushf",
    "    push es",
    "    push ds",
    "    push bp",
    "    push di",
    "    push si",
    "    push dx",
    "    push cx",
    "    push bx",
    "    push ax",
    "    mov ax, cs:[kern_seg]",
    "    mov ds, ax",
    "    mov es, ax",
    "    sti",
    "    xor eax, eax",
    "    mov ax, sp",
    "    push eax",
    "    data32 call int20_handler",
    "    add sp, 4",
    "    cli",
    "    pop ax",
    "    pop bx",
    "    pop cx",
    "    pop dx",
    "    pop si",
    "    pop di",
    "    pop bp",
    "    pop ds",
    "    pop es",
    "    popf",
    "    sti",
    "    mov ss, cs:[saved_ss]",
    "    mov sp, cs:[saved_sp]",
    "    iret",

    ".section .data.tramp, \"aw\"",
    "kern_seg: .word 0x1000",
    "kern_sp: .word 0xEFF0",
    "saved_ss: .word 0",
    "saved_sp: .word 0",
);
