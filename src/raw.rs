//! Managing raw mode.
//!
//! Raw mode is a particular state a TTY can have. It signifies that:
//!
//! 1. No line buffering (the input is given byte-by-byte).
//! 2. The input is not written out, instead it has to be done manually by the programmer.
//! 3. The output is not canonicalized (for example, `\n` means "go one line down", not "line
//!    break").
//!
//! It is essential to design terminal programs.
//!
//! # Example
//!
//! ```rust,no_run
//! use termion::raw::IntoRawMode;
//! use std::io::{Write, stdout};
//!
//! fn main() {
//!     let mut stdout = stdout().into_raw_mode().unwrap();
//!
//!     write!(stdout, "Hey there.").unwrap();
//! }
//! ```

use std::io::{self, Write};
use std::ops;

/// The timeout of an escape code control sequence, in milliseconds.
pub const CONTROL_SEQUENCE_TIMEOUT: u64 = 100;

use sys::Termios;

#[cfg(windows)]
use ::sys::winapi::um::handleapi::{INVALID_HANDLE_VALUE, CloseHandle};

/// A terminal restorer, which keeps the previous state of the terminal, and restores it, when
/// dropped.
///
/// Restoring will entirely bring back the old TTY state.
pub struct RawTerminal<W: Write> {
    prev_ios: Termios,
    output: W,
}

impl<W: Write> Drop for RawTerminal<W> {
    fn drop(&mut self) {
        ::sys::attr::set_terminal_attr(&self.prev_ios).unwrap();

        #[cfg(windows)]
        {
            if self.prev_ios.handle != INVALID_HANDLE_VALUE {
                unsafe { CloseHandle(self.prev_ios.handle); }
            }
        }
    }
}

impl<W: Write> ops::Deref for RawTerminal<W> {
    type Target = W;

    fn deref(&self) -> &W {
        &self.output
    }
}

impl<W: Write> ops::DerefMut for RawTerminal<W> {
    fn deref_mut(&mut self) -> &mut W {
        &mut self.output
    }
}

impl<W: Write> Write for RawTerminal<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.output.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}

/// Types which can be converted into "raw mode".
///
/// # Why is this type defined on writers and not readers?
///
/// TTYs has their state controlled by the writer, not the reader. You use the writer to clear the
/// screen, move the cursor and so on, so naturally you use the writer to change the mode as well.
pub trait IntoRawMode: Write + Sized {
    /// Switch to raw mode.
    ///
    /// Raw mode means that stdin won't be printed (it will instead have to be written manually by
    /// the program). Furthermore, the input isn't canonicalised or buffered (that is, you can
    /// read from stdin one byte of a time). The output is neither modified in any way.
    fn into_raw_mode(self) -> io::Result<RawTerminal<Self>>;
}


impl<W: Write> IntoRawMode for W {
    fn into_raw_mode(self) -> io::Result<RawTerminal<W>> {
        use sys::attr::{get_terminal_attr, raw_terminal_attr, set_terminal_attr};

        let mut ios = get_terminal_attr()?;
        let prev_ios = ios;

        raw_terminal_attr(&mut ios);

        set_terminal_attr(&ios)?;

        Ok(RawTerminal {
            prev_ios: prev_ios,
            output: self,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::{stdout};

    #[test]
    fn test_into_raw_mode() {
        let out = stdout().into_raw_mode().unwrap();

        drop(out);
    }
}
