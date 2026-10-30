use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    crate::bios::tty::print("\r\nKERNEL PANIC\r\n");
    loop {
        unsafe { core::arch::asm!("hlt") };
    }
}
