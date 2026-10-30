# Architecture

RustDOS is a DOS-compatible kernel written in Rust, running in 16-bit real mode on x86 (386+).

## Layers

```
 DOS programs (.COM / .EXE)
        │  INT 21h, INT 20h, ...
 ┌──────▼───────────────────────┐
 │ dos/      API dispatcher     │  safe Rust, match on AH
 ├──────────────────────────────┤
 │ loader/   EXEC, PSP          │
 │ mem/      MCB chain          │
 │ fs/       FAT12/16           │
 ├──────────────────────────────┤
 │ bios/     INT 10h, INT 13h   │  thin wrappers, unsafe isolated here
 │ arch/     entry, FarPtr, IVT │  asm trampolines, global_asm!
 └──────────────────────────────┘
        BIOS / hardware
```

## Principles

- `unsafe` lives only in `arch/` and `bios/`; the rest is safe Rust.
- Kernel image is one 64 KB segment (`CS=DS=ES=SS`), like a `.COM` file.
- Pointers in Rust are 32-bit offsets; `seg:off` is modelled explicitly by `FarPtr`.
- Interrupt entry/exit is assembly; handlers are `extern "C" fn(&mut Regs)`.
- Requires a 386+ CPU (LLVM code16 emits 0x66/0x67 prefixes).
