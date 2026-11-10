; FILLTEST.COM - fills the disk with FILL.BIN, then deletes it. Checks the disk-full path.
bits 16
org 0x100
    mov ah, 0x3C
    xor cx, cx
    mov dx, name
    int 0x21
    jc fail
    mov [h], ax
.loop:
    mov bx, [h]
    mov dx, buf
    mov cx, 8192
    mov ah, 0x40
    int 0x21
    jc fail
    add [total], ax
    adc word [total+2], 0
    cmp ax, 8192
    je .loop
    ; short write = disk full
    mov bx, [h]
    mov ah, 0x3E
    int 0x21
    jc fail
    mov dx, s_full
    mov ah, 9
    int 0x21
    mov ax, [total+2]
    call hex
    mov ax, [total]
    call hex
    mov dx, crlf
    mov ah, 9
    int 0x21
    mov dx, name
    mov ah, 0x41
    int 0x21
    jc fail
    mov dx, s_ok
    mov ah, 9
    int 0x21
    ret
fail:
    mov dx, s_fail
    mov ah, 9
    int 0x21
    ret
hex:
    mov cx, 4
.l: rol ax, 4
    push ax
    and al, 15
    add al, '0'
    cmp al, '9'
    jbe .p
    add al, 7
.p: mov dl, al
    mov ah, 2
    int 0x21
    pop ax
    loop .l
    ret
name:   db "FILL.BIN", 0
s_full: db "disk full after 0x", "$"
s_ok:   db "FILLTEST OK", 13, 10, "$"
s_fail: db "FILLTEST FAIL", 13, 10, "$"
crlf:   db 13, 10, "$"
total:  dd 0
h:      dw 0
buf:    times 8192 db 0x5A
