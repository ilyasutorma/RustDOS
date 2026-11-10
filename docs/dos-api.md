# RustDOS API (stage 1)

## INT 21h

| AH | Function | AH | Function |
|----|----------|----|----------|
| 00 | terminate | 3F | read handle |
| 01 | char in, echo | 40 | write handle (CX=0 truncates) |
| 02 | char out | 41 | delete (wildcards) |
| 06 | direct console I/O | 42 | lseek |
| 07/08 | char in, no echo | 43 | get/set attributes |
| 09 | print `$` string | 44 | IOCTL 0,1,6,7,8 |
| 0A | buffered input | 45/46 | dup / dup2 |
| 0B | input status | 47 | get current directory |
| 0C | flush + input | 48/49/4A | allocate / free / resize memory |
| 0D | disk reset (flush FAT) | 4B | EXEC AL=0 (`.COM` only), returns to caller |
| 0E/19 | select / get drive (A: only) | 4C/4D | exit with code / get return code |
| 1A/2F | set / get DTA | 4E/4F | FindFirst / FindNext |
| 25/35 | set / get vector | 50/51/62 | set / get PSP |
| 2A-2D | date and time (RTC) | 56 | rename / move |
| 30 | version (5.0) | 57 | file date/time |
| 33 | ctrl-break, boot drive, true version | 39/3A/3B | mkdir / rmdir / chdir |
| 36 | free disk space | 3C/3D/3E | create / open / close |

Errors: CF=1, AX=DOS error code (02 file not found, 03 path not found, 04 too many
open files, 05 access denied, 06 invalid handle, 08 no memory, 0B bad format,
0F invalid drive, 10 remove current dir, 12 no more files).

## Handles

Per-process JFT (20 bytes at PSP:0018) maps to a system file table of 24 entries.
Handles 0-2 are CON, 3-4 are NUL. A child gets fresh standard handles (no
inheritance of redirected handles yet).

## Filesystem

FAT12, one volume (A:), 8.3 names, subdirectories, read/write, `.`/`..`,
wildcards `*` and `?`. The FAT is cached in memory and written through to both
copies after every metadata change.

## Not yet implemented

`.EXE` (MZ) programs (EXEC returns error 0B), FCB functions, AUX/PRN devices,
environment block, FAT16/hard disks, long file names, batch files.
