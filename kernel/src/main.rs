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
mod util;

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    bios::tty::print("RustDOS 0.0.1\r\n");

    unsafe {
        arch::ivt::init();
        bios::tty::print("[ivt ok]\r\n");
        mem::mcb::init(0x3000, 0x9000);
        bios::tty::print("[mcb ok]\r\n");
        dos::files::init();
        let fat = fs::vol::init(0x00);
        bios::tty::print(if fat { "[fat ok]\r\n" } else { "[fat FAIL]\r\n" });
    }

    shell::commands::run();
}
