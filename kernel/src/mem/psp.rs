//! Program Segment Prefix: 256-byte header at offset 0 of every program segment.

#[repr(C, packed)]
pub struct Psp {
    pub int20: [u8; 2],       // 00h: CD 20
    pub mem_top: u16,         // 02h: first segment past the program's memory
    pub reserved1: u8,        // 04h
    pub call5: [u8; 5],       // 05h: far call to DOS dispatcher
    pub int22: u32,           // 0Ah: terminate address
    pub int23: u32,           // 0Eh: Ctrl-C handler
    pub int24: u32,           // 12h: critical error handler
    pub parent: u16,          // 16h: parent PSP segment
    pub jft: [u8; 20],        // 18h: job file table
    pub env_seg: u16,         // 2Ch: environment segment
    pub ss_sp: u32,           // 2Eh
    pub jft_size: u16,        // 32h
    pub jft_ptr: u32,         // 34h
    pub prev_psp: u32,        // 38h
    pub reserved2: [u8; 20],  // 3Ch
    pub int21_retf: [u8; 3],  // 50h: CD 21 CB
    pub reserved3: [u8; 9],   // 53h
    pub fcb1: [u8; 16],       // 5Ch
    pub fcb2: [u8; 20],       // 6Ch
    pub cmd_len: u8,          // 80h: command tail length
    pub cmd_tail: [u8; 127],  // 81h
}

const _: () = assert!(core::mem::size_of::<Psp>() == 256);
