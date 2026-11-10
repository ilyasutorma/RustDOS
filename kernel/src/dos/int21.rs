//! INT 21h dispatcher.

use super::{console, fail, files, memory, process};
use crate::arch::farmem::{rd16, wr16};
use crate::arch::regs::Regs;
use crate::bios::time;
use crate::fs::vol;

#[no_mangle]
pub extern "C" fn int21(regs: &mut Regs) {
    regs.clear_carry();

    unsafe {
        dispatch(regs);
    }
}

unsafe fn dispatch(regs: &mut Regs) {
    match regs.ah() {
        0x00 => process::terminate(0),
        0x01 => console::char_in_echo(regs),
        0x02 => console::char_out(regs),
        0x06 => console::direct_io(regs),
        0x07 | 0x08 => console::char_in(regs),
        0x09 => console::print_string(regs),
        0x0A => console::buffered_input(regs),
        0x0B => console::input_status(regs),
        0x0C => {
            // flush keyboard buffer, then run the function in AL
            while crate::bios::keyboard::key_ready() {
                crate::bios::keyboard::read_char();
            }

            let f = regs.al();
            regs.ax = (regs.ax & 0x00FF) | ((f as u16) << 8);

            if matches!(f, 0x01 | 0x06 | 0x07 | 0x08 | 0x0A) {
                dispatch(regs);
            }
        }
        0x0D => {
            vol::flush_fat();
        }
        0x0E => regs.ax = (regs.ax & 0xFF00) | 1, // one logical drive
        0x19 => regs.ax &= 0xFF00,                // current drive = A:
        0x1A => {
            process::DTA_SEG = regs.ds;
            process::DTA_OFF = regs.dx;
        }
        0x25 => {
            let at = (regs.al() as u16) * 4;
            wr16(0, at, regs.dx);
            wr16(0, at + 2, regs.ds);
        }
        0x2A => {
            let (y, m, d) = time::get_date();
            regs.cx = y;
            regs.dx = ((m as u16) << 8) | d as u16;
            regs.ax = (regs.ax & 0xFF00) | time::day_of_week(y, m, d) as u16;
        }
        0x2B => regs.ax &= 0xFF00, // setting the date is accepted and ignored
        0x2C => {
            let (h, m, s) = time::get_time();
            regs.cx = ((h as u16) << 8) | m as u16;
            regs.dx = (s as u16) << 8;
        }
        0x2D => regs.ax &= 0xFF00,
        0x2F => {
            regs.es = process::DTA_SEG;
            regs.bx = process::DTA_OFF;
        }
        0x30 => {
            regs.ax = 0x0005; // DOS 5.0
            regs.bx = 0xFD00; // OEM id: FreeDOS-compatible
            regs.cx = 0;
        }
        0x33 => match regs.al() {
            0x00 => regs.dx &= 0xFF00,
            0x01 => {}
            0x05 => regs.dx = (regs.dx & 0xFF00) | 1, // boot drive A:
            0x06 => {
                regs.bx = 0x0005;
                regs.dx = 0;
            }
            _ => fail(regs, vol::E_INVALID),
        },
        0x35 => {
            let at = (regs.al() as u16) * 4;
            regs.bx = rd16(0, at);
            regs.es = rd16(0, at + 2);
        }
        0x36 => files::free_space(regs),
        0x39 => files::mkdir(regs),
        0x3A => files::rmdir(regs),
        0x3B => files::chdir(regs),
        0x3C => files::create(regs),
        0x3D => files::open(regs),
        0x3E => files::close(regs),
        0x3F => files::read(regs),
        0x40 => files::write(regs),
        0x41 => files::delete(regs),
        0x42 => files::lseek(regs),
        0x43 => files::attrib(regs),
        0x44 => files::ioctl(regs),
        0x45 => files::dup(regs),
        0x46 => files::dup2(regs),
        0x47 => files::getcwd(regs),
        0x48 => memory::alloc(regs),
        0x49 => memory::free(regs),
        0x4A => memory::resize(regs),
        0x4B => process::exec_dos(regs),
        0x4C => process::terminate(regs.al()),
        0x4D => regs.ax = process::LAST_RC,
        0x4E => files::find_first(regs),
        0x4F => files::find_next(regs),
        0x50 => process::CURRENT_PSP = regs.bx,
        0x51 | 0x62 => regs.bx = process::CURRENT_PSP,
        0x56 => files::rename(regs),
        0x57 => files::file_time(regs),
        _ => fail(regs, vol::E_INVALID),
    }
}
