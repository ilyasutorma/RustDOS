use crate::bios::tty;
use core::panic::PanicInfo;

fn print_dec(mut n: u32) {
    let mut buf = [0u8; 10];
    let mut i = 0;

    if n == 0 {
        tty::putc(b'0');
        return;
    }

    while n > 0 {
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        tty::putc(buf[i]);
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    tty::print("\r\nKERNEL PANIC");

    if let Some(l) = info.location() {
        tty::print(" at ");
        tty::print(l.file());
        tty::putc(b':');
        print_dec(l.line());
    }

    if let Some(m) = info.message().as_str() {
        tty::print("\r\n");
        tty::print(m);
    }

    tty::print("\r\n");

    loop {
        unsafe { core::arch::asm!("hlt") };
    }
}
