# Memory map (real mode)

| Address (seg:off) | Physical | Purpose |
|---|---|---|
| 0000:0000 | 0x00000 | Interrupt vector table (256 × 4 bytes) |
| 0040:0000 | 0x00400 | BIOS data area |
| 0000:7C00 | 0x07C00 | Boot sector (stage 1) |
| 1000:0000 | 0x10000 | RustDOS kernel segment (64 KB, stack at top) |
| 2000:0000 … 9FFF:000F | 0x20000 … 0x9FFFF | Conventional memory, managed by the MCB chain |
| A000:0000 | 0xA0000 | Video memory |
| C000:0000 | 0xC0000 | Option ROMs |
| F000:0000 | 0xF0000 | System BIOS |

## Program layout (.COM)

```
seg:0000  PSP (256 bytes)
seg:0100  program code, entry point
seg:FFFE  initial stack top
```
