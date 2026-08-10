use super::CliHostError;
use coding_adventures_zeroize::Zeroizing;
use std::fs::File;
use std::io::{Read, Write};
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

pub(super) fn read_secret(
    prompt: &str,
    max_bytes: usize,
) -> Result<Zeroizing<Vec<u8>>, CliHostError> {
    let raw = unsafe {
        libc::open(
            c"/dev/tty".as_ptr(),
            libc::O_RDWR | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if raw < 0 {
        return Err(match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::ENXIO) | Some(libc::ENODEV) | Some(libc::ENOENT) | Some(libc::ENOTTY) => {
                CliHostError::TerminalUnavailable
            }
            _ => CliHostError::TerminalAccessFailed,
        });
    }
    let mut terminal = File::from(owned_fd(raw).map_err(|_| CliHostError::TerminalAccessFailed)?);
    verify_terminal(&terminal)?;
    read_secret_from_terminal(&mut terminal, prompt.as_bytes(), max_bytes)
}

fn read_secret_from_terminal(
    terminal: &mut File,
    prompt: &[u8],
    max_bytes: usize,
) -> Result<Zeroizing<Vec<u8>>, CliHostError> {
    verify_terminal(terminal)?;
    let guard = EchoGuard::disable(terminal.as_raw_fd())?;
    let operation = (|| {
        terminal
            .write_all(prompt)
            .and_then(|()| terminal.flush())
            .map_err(|_| CliHostError::TerminalAccessFailed)?;
        read_bounded_line(terminal, max_bytes)
    })();
    let restored = guard.restore();
    let newline = terminal
        .write_all(b"\n")
        .and_then(|()| terminal.flush())
        .map_err(|_| CliHostError::TerminalAccessFailed);
    let secret = operation?;
    restored?;
    newline?;
    Ok(secret)
}

fn read_bounded_line(
    terminal: &mut File,
    max_bytes: usize,
) -> Result<Zeroizing<Vec<u8>>, CliHostError> {
    let mut secret = Zeroizing::new(Vec::with_capacity(max_bytes));
    let mut too_long = false;
    loop {
        let mut byte = Zeroizing::new([0u8; 1]);
        match terminal.read(&mut *byte) {
            Ok(0) => return Err(CliHostError::SecretInputFailed),
            Ok(_) if byte[0] == b'\n' || byte[0] == b'\r' => break,
            Ok(_) if secret.len() < max_bytes => secret.push(byte[0]),
            Ok(_) => too_long = true,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(CliHostError::SecretInputFailed),
        }
    }
    if too_long {
        Err(CliHostError::SecretTooLong)
    } else {
        Ok(secret)
    }
}

fn verify_terminal(file: &File) -> Result<(), CliHostError> {
    if unsafe { libc::isatty(file.as_raw_fd()) } != 1 {
        return Err(CliHostError::TerminalUnavailable);
    }
    let mut stat = MaybeUninit::<libc::stat>::uninit();
    if unsafe { libc::fstat(file.as_raw_fd(), stat.as_mut_ptr()) } != 0 {
        return Err(CliHostError::TerminalAccessFailed);
    }
    if unsafe { stat.assume_init() }.st_mode & libc::S_IFMT != libc::S_IFCHR {
        return Err(CliHostError::TerminalUnavailable);
    }
    Ok(())
}

struct EchoGuard {
    descriptor: libc::c_int,
    original: libc::termios,
    active: bool,
}

impl EchoGuard {
    fn disable(descriptor: libc::c_int) -> Result<Self, CliHostError> {
        let mut original = MaybeUninit::<libc::termios>::uninit();
        if unsafe { libc::tcgetattr(descriptor, original.as_mut_ptr()) } != 0 {
            return Err(CliHostError::TerminalModeFailed);
        }
        let original = unsafe { original.assume_init() };
        let mut hidden = original;
        hidden.c_lflag &= !(libc::ECHO | libc::ECHONL);
        if unsafe { libc::tcsetattr(descriptor, libc::TCSAFLUSH, &hidden) } != 0 {
            return Err(CliHostError::TerminalModeFailed);
        }
        Ok(Self {
            descriptor,
            original,
            active: true,
        })
    }

    fn restore(mut self) -> Result<(), CliHostError> {
        if unsafe { libc::tcsetattr(self.descriptor, libc::TCSANOW, &self.original) } != 0 {
            return Err(CliHostError::TerminalModeFailed);
        }
        self.active = false;
        Ok(())
    }
}

impl Drop for EchoGuard {
    fn drop(&mut self) {
        if self.active {
            unsafe {
                libc::tcsetattr(self.descriptor, libc::TCSANOW, &self.original);
            }
        }
    }
}

fn owned_fd(raw: libc::c_int) -> Result<OwnedFd, ()> {
    if raw < 0 {
        Err(())
    } else {
        Ok(unsafe { OwnedFd::from_raw_fd(raw) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;
    use std::sync::mpsc;
    use std::thread;

    fn pseudo_terminal() -> (File, File) {
        let mut master = -1;
        let mut slave = -1;
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            },
            0
        );
        (
            File::from(owned_fd(master).unwrap()),
            File::from(owned_fd(slave).unwrap()),
        )
    }

    fn terminal_mode(file: &File) -> libc::termios {
        let mut mode = MaybeUninit::<libc::termios>::uninit();
        assert_eq!(
            unsafe { libc::tcgetattr(file.as_raw_fd(), mode.as_mut_ptr()) },
            0
        );
        unsafe { mode.assume_init() }
    }

    #[test]
    fn pseudo_terminal_roundtrip_hides_input_and_restores_echo() {
        let (mut master, mut slave) = pseudo_terminal();
        let original = terminal_mode(&slave);
        let prompt = b"Vault passphrase: ";
        let (prompt_seen_tx, prompt_seen_rx) = mpsc::channel();
        let peer = thread::spawn(move || {
            let mut seen = vec![0; prompt.len()];
            master.read_exact(&mut seen).unwrap();
            prompt_seen_tx.send(seen).unwrap();
            master.write_all(b"hidden value\n").unwrap();
            master
        });

        let secret = read_secret_from_terminal(&mut slave, prompt, 64).unwrap();
        assert_eq!(&*secret, b"hidden value");
        assert_eq!(prompt_seen_rx.recv().unwrap(), prompt);
        let restored = terminal_mode(&slave);
        assert_eq!(
            restored.c_lflag & (libc::ECHO | libc::ECHONL),
            original.c_lflag & (libc::ECHO | libc::ECHONL)
        );

        let mut master = peer.join().unwrap();
        let flags = unsafe { libc::fcntl(master.as_raw_fd(), libc::F_GETFL) };
        assert!(flags >= 0);
        assert_eq!(
            unsafe { libc::fcntl(master.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) },
            0
        );
        let mut output = Vec::new();
        let mut chunk = [0u8; 128];
        loop {
            match master.read(&mut chunk) {
                Ok(0) => break,
                Ok(count) => output.extend_from_slice(&chunk[..count]),
                Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                Err(error) => panic!("unexpected pseudo-terminal read: {error}"),
            }
        }
        assert!(!output
            .windows(b"hidden value".len())
            .any(|part| part == b"hidden value"));
    }

    #[test]
    fn oversized_input_is_drained_before_echo_restoration() {
        let (mut master, mut slave) = pseudo_terminal();
        let original = terminal_mode(&slave);
        let peer = thread::spawn(move || {
            let mut prompt = [0u8; 2];
            master.read_exact(&mut prompt).unwrap();
            master.write_all(b"12345\n").unwrap();
            master
        });
        assert!(matches!(
            read_secret_from_terminal(&mut slave, b"> ", 4),
            Err(CliHostError::SecretTooLong)
        ));
        drop(peer.join().unwrap());
        assert_eq!(
            terminal_mode(&slave).c_lflag & (libc::ECHO | libc::ECHONL),
            original.c_lflag & (libc::ECHO | libc::ECHONL)
        );
    }

    #[test]
    fn non_terminal_descriptor_fails_closed() {
        let mut descriptors = [-1; 2];
        assert_eq!(unsafe { libc::pipe(descriptors.as_mut_ptr()) }, 0);
        let mut read_end = File::from(owned_fd(descriptors[0]).unwrap());
        let write_end = owned_fd(descriptors[1]).unwrap();
        assert!(matches!(
            read_secret_from_terminal(&mut read_end, b"> ", 16),
            Err(CliHostError::TerminalUnavailable)
        ));
        drop(write_end);
    }
}
