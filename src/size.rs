use std::io;

#[cfg(unix)]
use libc::c_ushort;

#[cfg(unix)]
#[repr(C)]
struct TermSize {
    row: c_ushort,
    col: c_ushort,
    _x: c_ushort,
    _y: c_ushort,
}

// Since attributes on non-item statements is not stable yet, we use a function.
#[cfg(not(target_os = "android"))]
#[cfg(not(target_os = "redox"))]
#[cfg(target_pointer_width = "64")]
#[cfg(not(target_env = "musl"))]
#[cfg(not(windows))]
fn tiocgwinsz() -> u64 {
    use termios::TIOCGWINSZ;
    TIOCGWINSZ as u64
}
#[cfg(not(target_os = "android"))]
#[cfg(not(target_os = "redox"))]
#[cfg(target_pointer_width = "32")]
#[cfg(not(target_env = "musl"))]
#[cfg(not(windows))]
fn tiocgwinsz() -> u32 {
    use termios::TIOCGWINSZ;
    TIOCGWINSZ as u32
}

#[cfg(any(target_env = "musl", target_os = "android"))]
fn tiocgwinsz() -> i32 {
    use termios::TIOCGWINSZ;
    TIOCGWINSZ as i32
}

/// Get the size of the terminal.
#[cfg(unix)]
pub fn terminal_size() -> io::Result<(u16, u16)> {
    use libc::ioctl;
    use libc::STDOUT_FILENO;

    use std::mem;
    unsafe {
        let mut size: TermSize = mem::zeroed();

        if ioctl(STDOUT_FILENO, tiocgwinsz(), &mut size as *mut _) == 0 {
            Ok((size.col as u16, size.row as u16))
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "Unable to get the terminal size."))
        }
    }
}

/// Get the size of the terminal.
#[cfg(windows)]
pub fn terminal_size() -> io::Result<(u16, u16)> {

    use kernel32::{GetStdHandle, GetConsoleScreenBufferInfo};
    use winapi::TRUE;
    use winapi::winbase::STD_OUTPUT_HANDLE;
    use std::mem;

    unsafe {
        let stdout_handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut csbi = mem::zeroed();
        if GetConsoleScreenBufferInfo(stdout_handle, &mut csbi) == TRUE {
            Ok(((csbi.srWindow.Right - csbi.srWindow.Left + 1) as u16,
                (csbi.srWindow.Bottom - csbi.srWindow.Top + 1) as u16))
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "Unable to get the terminal size."))
        }
    }
}

#[cfg(target_os = "redox")]
pub fn terminal_size() -> io::Result<(u16, u16)> {
    use std::env;

    let width = try!(env::var("COLUMNS").map_err(|x| io::Error::new(io::ErrorKind::NotFound, x)))
        .parse()
        .unwrap_or(0);
    let height = try!(env::var("LINES").map_err(|x| io::Error::new(io::ErrorKind::NotFound, x)))
        .parse()
        .unwrap_or(0);

    Ok((width, height))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_size() {
        assert!(terminal_size().is_ok());
    }
}
