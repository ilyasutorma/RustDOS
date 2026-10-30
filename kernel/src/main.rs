#![no_std]
#![no_main]
#![allow(dead_code)]

mod arch;
mod bios;
mod dos;
mod fs;
mod loader;
mod mem;
mod panic;
mod shell;

/// Rust entry point, reached from `arch::entry::_start`.
#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    bios::tty::print("RustDOS 0.0.1\r\n");

    // TODO: install IVT vectors, init MCB chain, mount FAT, start shell.

    loop {
        unsafe { core::arch::asm!("hlt") };
    }
}
