bits 16
org 0x7C00

%ifndef KERNEL_SECTORS
%define KERNEL_SECTORS 16
%endif

KERNEL_SEG equ 0x1000

RESERVED equ KERNEL_SECTORS + 1     ; boot sector + kernel, then FAT

    jmp short start
    nop

; ---- BIOS Parameter Block (FAT12, 1.44 MB floppy) ----
bpb_oem:        db "RUSTDOS "
bpb_bps:        dw 512
bpb_spc:        db 1
bpb_reserved:   dw RESERVED
bpb_fats:       db 2
bpb_root_ent:   dw 224
bpb_total:      dw 2880
bpb_media:      db 0xF0
bpb_spf:        dw 9
bpb_spt:        dw 18
bpb_heads:      dw 2
bpb_hidden:     dd 0
bpb_total32:    dd 0
bpb_drive:      db 0
                db 0
                db 0x29
                dd 0x52444F53
                db "RUSTDOS    "
                db "FAT12   "

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

    ; Debug: print "B"
    mov ah, 0x0E
    mov al, 'B'
    mov bx, 0x0007
    int 0x10

    ; Reset disk
    xor ax, ax
    int 0x13
    jc disk_error

    ; Debug: print "L"
    mov ah, 0x0E
    mov al, 'L'
    int 0x10

    ; Load kernel to KERNEL_SEG:0000, one sector at a time.
    ; BX/CX/DX are clobbered by the LBA->CHS math and by INT 10h,
    ; so the buffer offset lives in BP and is saved around the print.
    mov bx, KERNEL_SEG
    mov es, bx
    xor bp, bp          ; buffer offset within ES

    mov si, 1           ; LBA of next sector (LBA 0 = boot sector)
    mov di, KERNEL_SECTORS

.load_loop:
    ; LBA -> CHS (18 sectors/track, 2 heads): no "dec", LBA is 0-based
    mov ax, si
    xor dx, dx
    mov bx, 18
    div bx              ; ax = LBA / 18, dx = LBA % 18
    mov cl, dl
    inc cl              ; sector is 1-based

    xor dx, dx
    mov bx, 2
    div bx              ; ax = cylinder, dx = head
    mov ch, al
    mov dh, dl

    mov dl, [boot_drive]
    mov bx, bp          ; ES:BX = destination
    mov ax, 0x0201      ; read 1 sector
    int 0x13
    jc disk_error

    ; Debug: print "."
    pusha
    mov ax, 0x0E2E
    mov bx, 0x0007
    int 0x10
    popa

    add bp, 512
    inc si
    dec di
    jnz .load_loop

    ; Debug: print "J"
    mov ah, 0x0E
    mov al, 'J'
    mov bx, 0x0007
    int 0x10

    ; Jump to kernel
    mov dl, [boot_drive]
    jmp KERNEL_SEG:0x0000

disk_error:
    mov si, msg_err
.print_err:
    lodsb
    test al, al
    jz .halt
    mov ah, 0x0E
    mov bx, 0x0007
    int 0x10
    jmp .print_err
.halt:
    hlt
    jmp .halt

boot_drive: db 0
msg_err: db "RustDOS: disk read error", 0

times 510 - ($ - $$) db 0
dw 0xAA55
