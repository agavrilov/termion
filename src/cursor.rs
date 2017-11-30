//! Cursor movement.

use failure::Error;

use std::fmt;
use std::io::{self, Write, Read};
use std::str;
use async::async_stdin_until;
use std::time::{SystemTime, Duration};
use raw::CONTROL_SEQUENCE_TIMEOUT;

derive_csi_sequence!("Hide the cursor.", Hide, "?25l");
derive_csi_sequence!("Show the cursor.", Show, "?25h");

derive_csi_sequence!("Restore the cursor.", Restore, "u");
derive_csi_sequence!("Save the cursor.", Save, "s");

/// Goto some position ((1,1)-based).
///
/// # Why one-based?
///
/// ANSI escapes are very poorly designed, and one of the many odd aspects is being one-based. This
/// can be quite strange at first, but it is not that big of an obstruction once you get used to
/// it.
///
/// # Example
///
/// ```rust
/// extern crate termion;
///
/// fn main() {
///     print!("{}{}Stuff", termion::clear::All, termion::cursor::Goto(5, 3));
/// }
/// ```
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Goto(pub u16, pub u16);

#[derive(Debug, Fail)]
#[fail(display = "unable to set cursor position")]
pub enum GotoError {
    #[fail(display = "IO error retrieving stdout")]
    StdOutRetrieval(#[cause] io::Error),
    #[fail(display = "call to set position failed")]
    PositionSetterFailed(#[cause] io::Error)
}

impl Goto {
    pub fn apply(&self) -> Result<(), GotoError> {
        if cfg!(debug_assertions) {
            assert!(self.0 != 0 && self.1 != 0, "invalid indicies for Goto; Goto is one-based.");
        }

        if cfg!(windows) {
            use sys::kernel32::SetConsoleCursorPosition;
            use sys::tty::{get_std_handle, StdStream};
            use sys::winapi::FALSE;
            use sys::winapi::wincon::COORD;

            unsafe {
                let new_coordinates = COORD {
                    X: (self.0 as i16) - 1,
                    Y: (self.1 as i16) - 1
                };

                let mut handle = get_std_handle(StdStream::OUT).map_err(|e| GotoError::StdOutRetrieval(e))?;

                if SetConsoleCursorPosition(handle, new_coordinates) == FALSE {
                    return Err(GotoError::PositionSetterFailed(::std::io::Error::last_os_error()));
                }
            }
        }
        else {
            print!(csi!("{};{}H"), self.1, self.0);
        }

        Ok(())
    }
}

impl Default for Goto {
    fn default() -> Goto {
        Goto(1, 1)
    }
}

/// Move cursor left.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Left(pub u16);

impl fmt::Display for Left {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, csi!("{}D"), self.0)
    }
}

/// Move cursor right.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Right(pub u16);

impl fmt::Display for Right {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, csi!("{}C"), self.0)
    }
}

/// Move cursor up.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Up(pub u16);

impl fmt::Display for Up {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, csi!("{}A"), self.0)
    }
}

/// Move cursor down.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Down(pub u16);

impl fmt::Display for Down {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, csi!("{}B"), self.0)
    }
}

#[derive(Debug, Fail)]
#[fail(display = "error reading cursor position")]
pub enum CursorPositionReadError {
    #[fail(display = "could not write to stdout")]
    Io(#[cause] io::Error),
    #[fail(display = "cursor position detection timed out")]
    TimedOut,
    #[fail(display = "unable to parse cursor escape sequence \"{}\"", _0)]
    CouldNotParseFormat(String),
    #[fail(display = "unable to parse cursor coordinate: {}", _0)]
    CouldNotParseCoordinate(::std::num::ParseIntError)
}

impl From<io::Error> for CursorPositionReadError {
    fn from(e: io::Error) -> Self {
        CursorPositionReadError::Io(e)
    }
}

impl From<::std::num::ParseIntError> for CursorPositionReadError {
    fn from(e: ::std::num::ParseIntError) -> Self {
        CursorPositionReadError::CouldNotParseCoordinate(e)
    }
}

/// Types that allow detection of the cursor position.
pub trait DetectCursorPos {
    /// Get the (1,1)-based cursor position from the terminal.
    fn cursor_pos(&mut self) -> Result<(u16, u16), CursorPositionReadError>;
}

impl<W: Write> DetectCursorPos for W {
    fn cursor_pos(&mut self) -> Result<(u16, u16), CursorPositionReadError> {
        let delimiter = b'R';
        let mut stdin = async_stdin_until(delimiter);

        // Where is the cursor?
        // Use `ESC [ 6 n`.
        write!(self, "\x1B[6n")?;
        self.flush()?;

        let mut buf: [u8; 1] = [0];
        let mut read_chars = Vec::new();

        let timeout = Duration::from_millis(CONTROL_SEQUENCE_TIMEOUT);
        let now = SystemTime::now();

        // Either consume all data up to R or wait for a timeout.
        while buf[0] != delimiter && now.elapsed().unwrap() < timeout {
            if stdin.read(&mut buf)? > 0 {
                read_chars.push(buf[0]);
            }
        }

        if read_chars.len() == 0 {
            return Err(CursorPositionReadError::TimedOut);
        }

        // The answer will look like `ESC [ Cy ; Cx R`.

        read_chars.pop(); // remove trailing R.
        let read_str = String::from_utf8(read_chars).unwrap();
        let beg = read_str.rfind('[').unwrap();
        let coords: String = read_str.chars().skip(beg + 1).collect();
        let mut nums = coords.split(';');

        let mut parse_num = move || -> Result<u16, CursorPositionReadError> {
            let num = nums.next().ok_or(CursorPositionReadError::CouldNotParseFormat(read_str.clone()))?;
            Ok(num.parse::<u16>()?)
        };

        let cy = parse_num()?;
        let cx = parse_num()?;

        Ok((cx, cy))
    }
}
