; EXECTEST.COM - runs other programs with INT 21h AH=4Bh and reads the return code.
bits 16
org 0x100

%macro PUTS 1
    mov dx, %1
    mov ah, 9
    int 0x21
%endmacro

start:
    mov [pb + 4], cs            ; command tail segment
    mov [pb + 8], cs            ; FCB1 segment
    mov [pb + 12], cs           ; FCB2 segment

    PUTS s_1
    mov dx, p_hello
    mov bx, pb
    mov ax, 0x4B00
    int 0x21
    jc fail

    PUTS s_2
    mov dx, p_echo
    mov bx, pb
    mov ax, 0x4B00
    int 0x21
    jc fail

    PUTS s_3
    mov dx, p_rc
    mov bx, pb
    mov ax, 0x4B00
    int 0x21
    jc fail
    mov ah, 0x4D
    int 0x21
    cmp ax, 42
    jne fail_rc
    PUTS s_back

    ; a missing program must set CF and return AX=2
    mov dx, p_none
    mov bx, pb
    mov ax, 0x4B00
    int 0x21
    jnc fail_rc
    cmp ax, 2
    jne fail_rc

    PUTS s_ok
    mov ax, 0x4C00
    int 0x21

fail_rc:
fail:
    PUTS s_fail
    mov ax, 0x4C01
    int 0x21

s_1:     db "exec HELLO", 13, 10, "$"
s_2:     db "exec ECHOARG", 13, 10, "$"
s_3:     db "exec RCTEST", 13, 10, "$"
s_back:  db "back, rc=42", 13, 10, "$"
s_ok:    db "EXECTEST OK", 13, 10, "$"
s_fail:  db "EXECTEST FAIL", 13, 10, "$"
p_hello: db "HELLO.COM", 0
p_echo:  db "ECHOARG.COM", 0
p_rc:    db "RCTEST.COM", 0
p_none:  db "NOPE.COM", 0
tail:    db 9, " abc def", 13
fcb:     times 16 db 0
pb:      dw 0
         dw tail, 0
         dw fcb, 0
         dw fcb, 0
