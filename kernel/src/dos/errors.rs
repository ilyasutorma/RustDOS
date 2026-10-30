//! DOS extended error codes returned in AX with CF set.

#[repr(u16)]
#[derive(Clone, Copy)]
pub enum DosError {
    InvalidFunction = 0x01,
    FileNotFound = 0x02,
    PathNotFound = 0x03,
    TooManyOpenFiles = 0x04,
    AccessDenied = 0x05,
    InvalidHandle = 0x06,
    InsufficientMemory = 0x08,
}
