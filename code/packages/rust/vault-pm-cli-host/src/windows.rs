use super::CliHostError;
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use std::char::decode_utf16;
use std::ffi::c_void;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle};
use std::ptr::null;
use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Console::{
    GetConsoleMode, ReadConsoleW, SetConsoleMode, WriteConsoleW, CONSOLE_MODE, ENABLE_ECHO_INPUT,
};

const CONSOLE_INPUT: &[u16] = &[
    b'C' as u16,
    b'O' as u16,
    b'N' as u16,
    b'I' as u16,
    b'N' as u16,
    b'$' as u16,
    0,
];
const CONSOLE_OUTPUT: &[u16] = &[
    b'C' as u16,
    b'O' as u16,
    b'N' as u16,
    b'O' as u16,
    b'U' as u16,
    b'T' as u16,
    b'$' as u16,
    0,
];

pub(super) fn read_secret(
    prompt: &str,
    max_bytes: usize,
) -> Result<Zeroizing<Vec<u8>>, CliHostError> {
    let input = open_console(CONSOLE_INPUT, GENERIC_READ | GENERIC_WRITE)?;
    let output = open_console(CONSOLE_OUTPUT, GENERIC_READ | GENERIC_WRITE)?;
    verify_console(&input)?;
    verify_console(&output)?;

    let guard = ConsoleModeGuard::disable_echo(&input)?;
    let operation = (|| {
        write_console(&output, prompt)?;
        read_bounded_line(&input, max_bytes)
    })();
    let restored = guard.restore();
    let newline = write_console(&output, "\r\n");
    let secret = operation?;
    restored?;
    newline?;
    Ok(secret)
}

pub(super) fn read_text(
    prompt: &str,
    max_bytes: usize,
) -> Result<Zeroizing<Vec<u8>>, CliHostError> {
    let input = open_console(CONSOLE_INPUT, GENERIC_READ | GENERIC_WRITE)?;
    let output = open_console(CONSOLE_OUTPUT, GENERIC_READ | GENERIC_WRITE)?;
    verify_console(&input)?;
    verify_console(&output)?;
    write_console(&output, prompt)?;
    read_bounded_line(&input, max_bytes).map_err(|error| match error {
        CliHostError::SecretInputFailed => CliHostError::TextInputFailed,
        CliHostError::SecretTooLong => CliHostError::InvalidText,
        other => other,
    })
}

/// Read one echoed console line, distinguishing end of input from failure.
///
/// The Windows sibling of the Unix reader with the same name. `Ctrl-Z` on an
/// empty console line ends the read with zero units, which is the console's
/// end-of-input signal and therefore `Ok(None)` rather than a failure.
pub(super) fn read_line_or_eof(
    prompt: &str,
    max_bytes: usize,
) -> Result<Option<Zeroizing<Vec<u8>>>, CliHostError> {
    let input = open_console(CONSOLE_INPUT, GENERIC_READ | GENERIC_WRITE)?;
    let output = open_console(CONSOLE_OUTPUT, GENERIC_READ | GENERIC_WRITE)?;
    verify_console(&input)?;
    verify_console(&output)?;
    write_console(&output, prompt)?;
    read_bounded_line_or_eof(&input, max_bytes).map_err(|error| match error {
        CliHostError::SecretInputFailed => CliHostError::TextInputFailed,
        CliHostError::SecretTooLong => CliHostError::InvalidText,
        other => other,
    })
}

pub(super) fn write_revealed_text(value: &str) -> Result<(), CliHostError> {
    let output = open_console(CONSOLE_OUTPUT, GENERIC_READ | GENERIC_WRITE)?;
    verify_console(&output)?;
    write_console(&output, "Secret: ")?;
    write_console(&output, value)?;
    write_console(&output, "\r\n")
}

fn open_console(name: &[u16], access: u32) -> Result<OwnedHandle, CliHostError> {
    let raw = unsafe {
        CreateFileW(
            name.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            0,
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        Err(CliHostError::TerminalUnavailable)
    } else {
        Ok(unsafe { OwnedHandle::from_raw_handle(raw as RawHandle) })
    }
}

fn verify_console(handle: &OwnedHandle) -> Result<CONSOLE_MODE, CliHostError> {
    let mut mode = 0;
    if unsafe { GetConsoleMode(raw_handle(handle), &mut mode) } == 0 {
        Err(CliHostError::TerminalUnavailable)
    } else {
        Ok(mode)
    }
}

fn write_console(handle: &OwnedHandle, value: &str) -> Result<(), CliHostError> {
    let value = WideSecret(value.encode_utf16().collect());
    let mut written = 0;
    let succeeded = unsafe {
        WriteConsoleW(
            raw_handle(handle),
            value.0.as_ptr().cast::<c_void>(),
            value.0.len() as u32,
            &mut written,
            null(),
        )
    };
    if succeeded == 0 || written as usize != value.0.len() {
        Err(CliHostError::TerminalAccessFailed)
    } else {
        Ok(())
    }
}

fn read_bounded_line(
    input: &OwnedHandle,
    max_bytes: usize,
) -> Result<Zeroizing<Vec<u8>>, CliHostError> {
    read_bounded_line_or_eof(input, max_bytes)?.ok_or(CliHostError::SecretInputFailed)
}

fn read_bounded_line_or_eof(
    input: &OwnedHandle,
    max_bytes: usize,
) -> Result<Option<Zeroizing<Vec<u8>>>, CliHostError> {
    let mut units = WideSecret(Vec::with_capacity(max_bytes));
    let mut too_long = false;
    'line: loop {
        let mut buffer = WideBuffer([0u16; 128]);
        let mut read = 0;
        let succeeded = unsafe {
            ReadConsoleW(
                raw_handle(input),
                buffer.0.as_mut_ptr().cast::<c_void>(),
                buffer.0.len() as u32,
                &mut read,
                null(),
            )
        };
        if succeeded == 0 {
            return Err(CliHostError::SecretInputFailed);
        }
        if read == 0 {
            return Ok(None);
        }
        for unit in &buffer.0[..read as usize] {
            if *unit == b'\r' as u16 || *unit == b'\n' as u16 {
                break 'line;
            }
            if units.0.len() < max_bytes {
                units.0.push(*unit);
            } else {
                too_long = true;
            }
        }
    }
    finish_line(units, too_long, max_bytes).map(Some)
}

fn finish_line(
    units: WideSecret,
    too_long: bool,
    max_bytes: usize,
) -> Result<Zeroizing<Vec<u8>>, CliHostError> {
    if too_long {
        return Err(CliHostError::SecretTooLong);
    }
    let mut output = Zeroizing::new(Vec::with_capacity(units.0.len().saturating_mul(3)));
    for decoded in decode_utf16(units.0.iter().copied()) {
        let character = decoded.map_err(|_| CliHostError::SecretInputFailed)?;
        let mut encoded = Zeroizing::new([0u8; 4]);
        output.extend_from_slice(character.encode_utf8(&mut *encoded).as_bytes());
        if output.len() > max_bytes {
            return Err(CliHostError::SecretTooLong);
        }
    }
    Ok(output)
}

struct WideSecret(Vec<u16>);

impl Drop for WideSecret {
    fn drop(&mut self) {
        for unit in &mut self.0 {
            unit.zeroize();
        }
    }
}

struct WideBuffer([u16; 128]);

impl Drop for WideBuffer {
    fn drop(&mut self) {
        for unit in &mut self.0 {
            unit.zeroize();
        }
    }
}

fn raw_handle(handle: &OwnedHandle) -> HANDLE {
    handle.as_raw_handle() as HANDLE
}

struct ConsoleModeGuard<'handle> {
    handle: &'handle OwnedHandle,
    original: CONSOLE_MODE,
    active: bool,
}

impl<'handle> ConsoleModeGuard<'handle> {
    fn disable_echo(handle: &'handle OwnedHandle) -> Result<Self, CliHostError> {
        let original = verify_console(handle).map_err(|_| CliHostError::TerminalModeFailed)?;
        if unsafe { SetConsoleMode(raw_handle(handle), original & !ENABLE_ECHO_INPUT) } == 0 {
            return Err(CliHostError::TerminalModeFailed);
        }
        Ok(Self {
            handle,
            original,
            active: true,
        })
    }

    fn restore(mut self) -> Result<(), CliHostError> {
        if unsafe { SetConsoleMode(raw_handle(self.handle), self.original) } == 0 {
            return Err(CliHostError::TerminalModeFailed);
        }
        self.active = false;
        Ok(())
    }
}

impl Drop for ConsoleModeGuard<'_> {
    fn drop(&mut self) {
        if self.active {
            unsafe {
                SetConsoleMode(raw_handle(self.handle), self.original);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn console_names_are_nul_terminated() {
        assert_eq!(CONSOLE_INPUT.last(), Some(&0));
        assert_eq!(CONSOLE_OUTPUT.last(), Some(&0));
    }

    #[test]
    fn utf16_console_input_becomes_utf8_passphrase_bytes() {
        let units: Vec<u16> = "correct horse 🐎".encode_utf16().collect();
        assert_eq!(
            &*finish_line(WideSecret(units), false, 64).unwrap(),
            "correct horse 🐎".as_bytes()
        );
    }

    #[test]
    fn invalid_or_oversized_console_input_fails_closed() {
        assert!(matches!(
            finish_line(WideSecret(vec![0xd800]), false, 64),
            Err(CliHostError::SecretInputFailed)
        ));
        assert!(matches!(
            finish_line(WideSecret(Vec::new()), true, 64),
            Err(CliHostError::SecretTooLong)
        ));
        assert!(matches!(
            finish_line(WideSecret("horse".encode_utf16().collect()), false, 4),
            Err(CliHostError::SecretTooLong)
        ));
    }
}
