#!/usr/bin/env bash
# Builds build/rustdos.img: boot sector + flat kernel binary.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/build"
ELF="$ROOT/target/i386-code16/release/kernel"

mkdir -p "$OUT"

# Find llvm-objcopy (rustup llvm-tools-preview) or fall back to system one.
OBJCOPY="$(command -v llvm-objcopy || true)"
if [ -z "$OBJCOPY" ]; then
    OBJCOPY="$(find "$(rustc --print sysroot)" -name llvm-objcopy -type f | head -n1)"
fi
[ -n "$OBJCOPY" ] || { echo "llvm-objcopy not found" >&2; exit 1; }

"$OBJCOPY" -O binary "$ELF" "$OUT/kernel.bin"

SIZE=$(stat -c %s "$OUT/kernel.bin")
SECTORS=$(( (SIZE + 511) / 512 ))
if [ "$SECTORS" -gt 112 ]; then
    echo "kernel too big for a single 64 KB segment: $SIZE bytes" >&2
    exit 1
fi

nasm -f bin -DKERNEL_SECTORS="$SECTORS" "$ROOT/boot/boot.asm" -o "$OUT/boot.bin"

# Root directory contents: every program in build/ plus test.txt.
FILES=()
for f in "$OUT"/*.COM; do [ -f "$f" ] && FILES+=("$f"); done
[ -f "$ROOT/test.txt" ] && FILES+=("$ROOT/test.txt")

python3 "$ROOT/scripts/mkfat.py" "$OUT/rustdos.img" "$OUT/boot.bin" "$OUT/kernel.bin" \
    $((SECTORS + 1)) "${FILES[@]}"

echo "OK: $OUT/rustdos.img (kernel: $SIZE bytes, $SECTORS sectors)"
