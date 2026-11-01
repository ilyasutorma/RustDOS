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

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    bios::tty::print("RustDOS 0.0.1\r\n");

    unsafe {
        arch::ivt::init();
        mem::mcb::init(0x3000, 0xA000);
        fs::fat::init(0x00);
    }

    shell::commands::run();
}
