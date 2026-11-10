; FILETEST.COM - exercises the INT 21h file and directory services.
bits 16
org 0x100

%macro DOS 1
    mov ah, %1
    int 0x21
%endmacro

%macro CHK 1
    jnc %%ok
    mov byte [step], %1
    jmp fail
%%ok:
%endmacro

%macro PUTS 1
    mov dx, %1
    DOS 0x09
%endmacro

start:
    PUTS s_begin

    ; 1: mkdir TESTDIR
    mov dx, p_dir
    DOS 0x39
    CHK 1

    ; 2: chdir TESTDIR
    mov dx, p_dir
    DOS 0x3B
    CHK 2

    ; 3: create A.TXT, 4: write, 5: close
    mov dx, p_a
    xor cx, cx
    DOS 0x3C
    CHK 3
    mov [h], ax
    mov bx, ax
    mov dx, msg1
    mov cx, msg1_len
    DOS 0x40
    CHK 4
    cmp ax, msg1_len
    jne fail_len
    mov bx, [h]
    DOS 0x3E
    CHK 5

    ; 6: open A.TXT, 7: read, 8: echo to stdout
    mov dx, p_a
    mov al, 0
    DOS 0x3D
    CHK 6
    mov [h], ax
    mov bx, ax
    mov dx, buf
    mov cx, 64
    DOS 0x3F
    CHK 7
    cmp ax, msg1_len
    jne fail_len
    mov cx, ax
    mov bx, 1
    mov dx, buf
    DOS 0x40
    CHK 8

    ; 9: lseek to 7 and read 5 bytes ("file!")
    mov bx, [h]
    mov al, 0
    xor cx, cx
    mov dx, 7
    DOS 0x42
    CHK 9
    cmp ax, 7
    jne fail_len
    mov bx, [h]
    mov dx, buf
    mov cx, 5
    DOS 0x3F
    CHK 9
    mov cx, ax
    mov bx, 1
    mov dx, buf
    DOS 0x40
    PUTS crlf
    mov bx, [h]
    DOS 0x3E
    CHK 10

    ; 11: list with FindFirst/FindNext (attr: directories)
    PUTS s_list
    mov dx, p_all
    mov cx, 0x10
    DOS 0x4E
    CHK 11
.next:
    mov dx, 0x80 + 0x1E
    call print_asciiz
    PUTS crlf
    DOS 0x4F
    jnc .next

    ; 12: rename A.TXT -> B.TXT
    mov dx, p_a
    mov di, p_b
    DOS 0x56
    CHK 12

    ; 13: big file spanning several clusters: 3 x 1500 bytes
    mov dx, p_big
    xor cx, cx
    DOS 0x3C
    CHK 13
    mov [h], ax
    mov byte [fillc], 'A'
    mov byte [cnt], 3
.wr:
    mov di, buf
    mov cx, 1500
    mov al, [fillc]
    cld
    rep stosb
    mov bx, [h]
    mov dx, buf
    mov cx, 1500
    DOS 0x40
    CHK 14
    cmp ax, 1500
    jne fail_len
    inc byte [fillc]
    dec byte [cnt]
    jnz .wr
    mov bx, [h]
    DOS 0x3E
    CHK 15

    ; 16: read BIG.BIN back and verify every chunk
    mov dx, p_big
    mov al, 0
    DOS 0x3D
    CHK 16
    mov [h], ax
    mov byte [fillc], 'A'
    mov byte [cnt], 3
.rd:
    mov bx, [h]
    mov dx, buf
    mov cx, 1500
    DOS 0x3F
    CHK 17
    cmp ax, 1500
    jne fail_len
    mov di, buf2
    mov cx, 1500
    mov al, [fillc]
    cld
    rep stosb
    mov si, buf
    mov di, buf2
    mov cx, 1500
    repe cmpsb
    jne fail_cmp
    inc byte [fillc]
    dec byte [cnt]
    jnz .rd
    ; EOF: next read returns 0
    mov bx, [h]
    mov dx, buf
    mov cx, 10
    DOS 0x3F
    CHK 18
    or ax, ax
    jnz fail_len
    mov bx, [h]
    DOS 0x3E

    ; 19: getcwd should be "TESTDIR"
    mov si, cwdbuf
    xor dl, dl
    DOS 0x47
    CHK 19
    mov dx, cwdbuf
    call print_asciiz
    PUTS crlf

    ; 20: delete files, back to root, remove directory
    mov dx, p_b
    DOS 0x41
    CHK 20
    mov dx, p_big
    DOS 0x41
    CHK 21
    mov dx, p_up
    DOS 0x3B
    CHK 22
    mov dx, p_dir
    DOS 0x3A
    CHK 23

    ; 24: open of a missing file must fail with AX=2
    mov dx, p_a
    mov al, 0
    DOS 0x3D
    jnc fail_nofail
    cmp ax, 2
    jne fail_len

    PUTS s_ok
    mov ax, 0x4C00
    int 0x21

fail_len:
    mov byte [step], 99
    jmp fail
fail_cmp:
    mov byte [step], 98
    jmp fail
fail_nofail:
    mov byte [step], 97
fail:
    mov [err], ax
    PUTS s_fail
    mov al, [step]
    xor ah, ah
    mov bl, 10
    div bl
    add ax, '00'
    push ax
    mov dl, al
    DOS 0x02
    pop ax
    mov dl, ah
    DOS 0x02
    PUTS s_ax
    mov ax, [err]
    call print_hex16
    PUTS crlf
    mov ax, 0x4C01
    int 0x21

print_asciiz:                 ; DS:DX = string
    mov si, dx
.l: lodsb
    or al, al
    jz .d
    mov dl, al
    mov ah, 2
    int 0x21
    jmp .l
.d: ret

print_hex16:
    mov cx, 4
.l: rol ax, 4
    push ax
    and al, 0x0F
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

s_begin: db "FILETEST start", 13, 10, "$"
s_list:  db "dir:", 13, 10, "$"
s_ok:    db "FILETEST OK", 13, 10, "$"
s_fail:  db "FILETEST FAIL step=", "$"
s_ax:    db " ax=", "$"
crlf:    db 13, 10, "$"
p_dir:   db "TESTDIR", 0
p_up:    db "..", 0
p_a:     db "A.TXT", 0
p_b:     db "B.TXT", 0
p_big:   db "BIG.BIN", 0
p_all:   db "*.*", 0
msg1:    db "Hello, file!", 13, 10
msg1_len equ $ - msg1
h:       dw 0
step:    db 0
err:     dw 0
fillc:   db 0
cnt:     db 0
cwdbuf:  times 66 db 0
buf:     times 1500 db 0
buf2:    times 1500 db 0
