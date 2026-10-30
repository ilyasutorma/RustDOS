//! Memory Control Block: 16-byte header in front of every memory arena.

#[repr(C, packed)]
pub struct Mcb {
    /// 'M' (0x4D) = more blocks follow, 'Z' (0x5A) = last block.
    pub kind: u8,
    /// PSP segment of the owner, 0 = free.
    pub owner: u16,
    /// Block size in paragraphs (16 bytes), not counting this header.
    pub size: u16,
    pub reserved: [u8; 3],
    /// Owner program name.
    pub name: [u8; 8],
}

const _: () = assert!(core::mem::size_of::<Mcb>() == 16);
