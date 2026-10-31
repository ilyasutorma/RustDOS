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
    }

    bios::tty::print("MCB ready\r\n");

    let msg = b"Hello from DOS!$";

    unsafe {
        core::arch::asm!(
            "int 0x21",
            in("ax") 0x0900u16,
                         in("dx") msg.as_ptr() as u16
        );
    }

    bios::tty::print("\r\nDone.\r\n");

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}
