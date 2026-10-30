# Roadmap

- [ ] **M0: Boot.** Boot sector loads the Rust kernel, prints a banner via INT 10h.
- [ ] **M1: Interrupts.** IVT setup, asm trampoline, `INT 21h` AH=09h and AH=4Ch.
- [ ] **M2: Memory.** MCB chain, allocate/free/resize (AH=48h/49h/4Ah).
- [ ] **M3: Programs.** PSP, `.COM` loader, EXEC (AH=4Bh), `programs/hello.asm` runs.
- [ ] **M4: Disk.** INT 13h driver, FAT12/16 read, file handles (AH=3Dh/3Fh/3Eh).
- [ ] **M5: Shell.** Built-in command interpreter or stock FreeDOS `COMMAND.COM`.
- [ ] **M6: EXE.** MZ loader with relocations.
- [ ] **M7: Compatibility.** Run common DOS tools and games; test suite.
