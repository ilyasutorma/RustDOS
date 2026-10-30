#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

exec qemu-system-i386 \
    -drive format=raw,file="$ROOT/build/rustdos.img" \
    -serial stdio \
    -no-reboot \
    "$@"
