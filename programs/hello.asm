; Test program: prints a string via INT 21h/09h and exits.
; Build: nasm -f bin programs/hello.asm -o build/HELLO.COM

org 0x100

    mov ah, 0x09
    mov dx, msg
    int 0x21

    mov ax, 0x4C00
    int 0x21

msg db "Hello from RustDOS!", 13, 10, "$"
