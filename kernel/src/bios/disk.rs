//! Disk access via INT 13h (CHS and extended LBA read/write).
//!
//! TODO: `read_sectors(drive, lba, count, buf: FarPtr) -> Result<(), DiskError>`
