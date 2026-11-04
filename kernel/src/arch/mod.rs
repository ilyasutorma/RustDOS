//! CPU-level glue: startup code, far pointers, register frames, interrupt stubs.

pub mod entry;
pub mod farptr;
pub mod regs;
pub mod ivt;
pub mod trampolines;
pub mod a20;
pub mod farmem;
