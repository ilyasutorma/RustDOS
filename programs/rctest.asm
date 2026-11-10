; RCTEST.COM - exits with return code 42.
bits 16
org 0x100
    mov ah, 0x09
    mov dx, msg
    int 0x21
    mov ax, 0x4C2A
    int 0x21
msg: db "rctest running", 13, 10, "$"
