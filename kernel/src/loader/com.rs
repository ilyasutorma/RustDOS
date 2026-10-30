//! .COM loader: allocate a segment via MCB, build the PSP, load the file at
//! seg:0100h, set CS=DS=ES=SS=seg, SP=FFFEh, jump to seg:0100h.
//!
//! TODO: MZ (.EXE) loader goes next to this file as `exe.rs`.
