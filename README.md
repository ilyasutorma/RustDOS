# RustDOS

A from-scratch reimplementation of a DOS-compatible kernel in Rust, running in 16-bit real mode on x86 (386+). It keeps the classic `segment:offset` memory model with 64 KB segments, the INT 21h API, MCB memory chains and PSP, while moving the kernel logic to safe Rust.

inspired by FreeDOS

> Status: early skeleton, nothing works yet. See [docs/roadmap.md](docs/roadmap.md).

## Layout

```
boot/        boot sector (NASM)
kernel/      the kernel (Rust, no_std, code16)
  src/arch/    entry, FarPtr, register frames
  src/bios/    INT 10h / INT 13h wrappers
  src/dos/     INT 21h dispatcher, error codes
  src/mem/     MCB, PSP
  src/fs/      FAT12/16
  src/loader/  .COM / .EXE loaders
  src/shell/   built-in command interpreter
targets/     custom rustc target (i386 code16)
programs/    test DOS programs
scripts/     image building, QEMU runner
docs/        architecture, memory map, roadmap
```



## License

MIT
