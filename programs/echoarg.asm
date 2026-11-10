; ECHOARG.COM - prints the command tail from PSP:0080.
bits 16
org 0x100
    mov si, 0x81
    xor cx, cx
    mov cl, [0x80]
    mov dl, '['
    mov ah, 2
    int 0x21
    jcxz .done
.l: lodsb
    mov dl, al
    mov ah, 2
    int 0x21
    loop .l
.done:
    mov dl, ']'
    mov ah, 2
    int 0x21
    mov ah, 9
    mov dx, crlf
    int 0x21
    ret
crlf: db 13, 10, "$"
