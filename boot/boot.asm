; RustDOS boot sector (stage 1)
;
; Loads KERNEL_SECTORS sectors starting at LBA 1 to 1000:0000 using
; INT 13h extensions (AH=42h), then jumps to the kernel with DL = boot drive.
; KERNEL_SECTORS is passed by scripts/mkimage.sh: nasm -DKERNEL_SECTORS=N

bits 16
org 0x7C00

%ifndef KERNEL_SECTORS
%define KERNEL_SECTORS 1
%endif

KERNEL_SEG equ 0x1000

start:
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    sti
    cld

    mov [boot_drive], dl

    mov si, dap
    mov ah, 0x42
    int 0x13
    jc disk_error

    mov dl, [boot_drive]
    jmp KERNEL_SEG:0x0000

disk_error:
    mov si, msg_err
.next:
    lodsb
    test al, al
    jz .halt
    mov ah, 0x0E
    mov bx, 0x0007
    int 0x10
    jmp .next
.halt:
    hlt
    jmp .halt

boot_drive: db 0
msg_err:    db "RustDOS: disk read error", 0

align 4
dap:
    db 0x10, 0              ; packet size, reserved
    dw KERNEL_SECTORS       ; sector count
    dw 0x0000               ; buffer offset
    dw KERNEL_SEG           ; buffer segment
    dq 1                    ; start LBA (sector after the boot sector)

times 510 - ($ - $$) db 0
dw 0xAA55
