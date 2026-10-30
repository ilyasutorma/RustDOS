/* Kernel image lives in a single 64 KB segment (1000:0000), origin 0.
   CS = DS = ES = SS = 0x1000, stack grows down from 0xFFF0. */

ENTRY(_start)

SECTIONS
{
    . = 0x0000;

    .text : {
        KEEP(*(.text.entry))
        *(.text .text.*)
    }

    .rodata : { *(.rodata .rodata.*) }

    .data : { *(.data .data.*) }

    .bss : {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    /DISCARD/ : {
        *(.eh_frame*)
        *(.comment)
        *(.note*)
    }
}
