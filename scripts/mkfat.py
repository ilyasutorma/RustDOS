#!/usr/bin/env python3
"""Build a 1.44 MB FAT12 floppy: boot sector + kernel in the reserved area,
then FAT/root/data holding the given files.

usage: mkfat.py OUT BOOT.BIN KERNEL.BIN RESERVED [FILE ...]
"""
import os, struct, sys

out, boot, kernel, reserved = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
files = sys.argv[5:]

import time as _t
_n = _t.localtime()
STAMP_D = ((_n.tm_year - 1980) << 9) | (_n.tm_mon << 5) | _n.tm_mday
STAMP_T = (_n.tm_hour << 11) | (_n.tm_min << 5) | (_n.tm_sec // 2)
BPS, SPF, FATS, ROOT_ENT, TOTAL = 512, 9, 2, 224, 2880
img = bytearray(BPS * TOTAL)

b = open(boot, "rb").read()
k = open(kernel, "rb").read()
assert len(b) == 512
assert len(k) <= (reserved - 1) * BPS, "kernel does not fit in reserved sectors"
img[0:512] = b
img[512:512 + len(k)] = k

fat = bytearray(SPF * BPS)
fat[0:3] = b"\xF0\xFF\xFF"

def fat_set(n, v):
    o = n + n // 2
    if n & 1:
        fat[o] = (fat[o] & 0x0F) | ((v << 4) & 0xF0)
        fat[o + 1] = (v >> 4) & 0xFF
    else:
        fat[o] = v & 0xFF
        fat[o + 1] = (fat[o + 1] & 0xF0) | ((v >> 8) & 0x0F)

root = bytearray(ROOT_ENT * 32)
root_lba = reserved + FATS * SPF
data_lba = root_lba + ROOT_ENT * 32 // BPS
nxt = 2

for i, path in enumerate(files):
    data = open(path, "rb").read()
    base, _, ext = os.path.basename(path).upper().partition(".")
    name = base[:8].ljust(8).encode() + ext[:3].ljust(3).encode()
    n = max(1, (len(data) + BPS - 1) // BPS)
    first = nxt
    for j in range(n):
        c = nxt + j
        fat_set(c, c + 1 if j < n - 1 else 0xFFF)
        lba = data_lba + (c - 2)
        chunk = data[j * BPS:(j + 1) * BPS]
        img[lba * BPS:lba * BPS + len(chunk)] = chunk
    nxt += n
    root[i * 32:(i + 1) * 32] = name + struct.pack("<BBBHHHHHHHI", 0x20, 0, 0, 0, STAMP_D, STAMP_D, 0, STAMP_T, STAMP_D, first, len(data))

for f in range(FATS):
    o = (reserved + f * SPF) * BPS
    img[o:o + len(fat)] = fat
img[root_lba * BPS:root_lba * BPS + len(root)] = root

open(out, "wb").write(img)
