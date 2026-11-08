bits 16
org 0x100

    mov ah, 0x09
    mov dx, msg
    int 0x21
    ret

msg: db "Hello from HELLO.COM via INT 21h!", 13, 10, "$"
