//! # bounded-source-file: read a source file within a byte budget
//!
//! Compiler front ends take source *files* from users, and every one of
//! them in this repo used to start with `std::fs::read_to_string(path)`.
//! That call reads the whole file before anyone can measure it, so:
//!
//! | Input                         | `read_to_string`                     | [`read_to_string`] here                  |
//! |-------------------------------|--------------------------------------|------------------------------------------|
//! | a 10 GB file                  | allocates 10 GB, then the compiler refuses it | refused by its size, nothing read   |
//! | `/dev/zero`, `/dev/urandom`   | reads until memory runs out          | refused: not a regular file              |
//! | a FIFO with no writer, a tty  | blocks forever in `open` or `read`   | refused: not a regular file, never blocks |
//! | a file that grows after `stat`| reads whatever it grew to            | stops after `limit + 1` bytes, refused   |
//! | a file of exactly `limit` bytes | read                               | read                                     |
//!
//! ## The four steps
//!
//! ```text
//!   1. fs::metadata(path).is_file()     a FIFO, device or directory is refused
//!                                       before `open`, which would block on a FIFO
//!   2. open, with O_NONBLOCK and        if the path was swapped for a FIFO or tty
//!      O_NOCTTY on Unix                 between 1 and 2, `open` still returns, and
//!                                       a tty does not become the controlling one
//!   3. file.metadata() on the handle    the file measured is the file read:
//!        is_file(), len() <= limit      refuse a swap, refuse a file too big now
//!   4. read through take(limit + 1)     refuse a file that grew after step 3,
//!        and count                      having buffered at most limit + 1 bytes
//! ```
//!
//! Step 2's `O_NONBLOCK` is left set: a regular file ignores it (reads of
//! a regular file never return `EWOULDBLOCK`), and step 3 has refused
//! everything that is not a regular file.
//!
//! Errors are ordinary `io::Error`s, so callers keep their `?`:
//!
//! | Kind           | Meaning                                       |
//! |----------------|-----------------------------------------------|
//! | `InvalidInput` | not a regular file                            |
//! | `FileTooLarge` | more than `limit` bytes, by size or by reading |
//! | `InvalidData`  | not UTF-8 (as `read_to_string` reports it)    |
//! | anything else  | the underlying `open` / `read` failure        |
//!
//! The limit is the caller's: each front end passes its own source budget
//! (Twig's 1 MiB, `lang-aot`'s 16 MiB ceiling for every language).

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Read `path` as UTF-8, refusing anything that is not a regular file of at
/// most `limit` bytes. See the crate docs for the four steps.
pub fn read_to_string(path: &Path, limit: u64) -> io::Result<String> {
    if !std::fs::metadata(path)?.is_file() {
        return Err(not_a_regular_file(path));
    }
    let file = open_without_blocking(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(not_a_regular_file(path));
    }
    if metadata.len() > limit {
        return Err(too_large(limit));
    }
    read_bounded(file, limit)
}

/// Read at most `limit` bytes of UTF-8 from `reader`, refusing it as
/// `FileTooLarge` if it holds more. At most `limit + 1` bytes are ever
/// buffered, however long the reader is.
///
/// This is step 4 on its own, public so a caller with a reader that is not
/// a file (a pipe it chose to trust, a test) gets the same bound.
///
/// ```
/// use bounded_source_file::read_bounded;
/// assert_eq!(read_bounded(&b"(+ 1 2)"[..], 7).unwrap(), "(+ 1 2)");
/// let endless = std::io::repeat(b'a');
/// assert_eq!(read_bounded(endless, 7).unwrap_err().kind(), std::io::ErrorKind::FileTooLarge);
/// ```
pub fn read_bounded<R: Read>(reader: R, limit: u64) -> io::Result<String> {
    let mut bytes = Vec::new();
    reader.take(limit.saturating_add(1)).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(too_large(limit));
    }
    String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(unix)]
fn open_without_blocking(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(true)
        // O_NONBLOCK: `open` of a writerless FIFO returns instead of
        // waiting.  O_NOCTTY: opening a terminal device never makes it this
        // process's controlling terminal (step 3 refuses it anyway, but the
        // open itself must have no side effect).
        .custom_flags(libc::O_NONBLOCK | libc::O_NOCTTY)
        .open(path)
}

#[cfg(not(unix))]
fn open_without_blocking(path: &Path) -> io::Result<File> {
    // Windows has no FIFO special files in the filesystem namespace that a
    // plain open would block on; step 1 and step 3 still refuse anything
    // that is not a regular file.
    File::open(path)
}

fn not_a_regular_file(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("{} is not a regular file", path.display()),
    )
}

fn too_large(limit: u64) -> io::Error {
    io::Error::new(
        io::ErrorKind::FileTooLarge,
        format!("source file is larger than the {limit}-byte limit"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;
    use std::path::PathBuf;

    /// A scratch directory unique to this test process and tag.
    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bounded_source_file_{tag}_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_file_of_exactly_the_limit_is_read_and_one_more_byte_is_refused() {
        let dir = scratch("limit");
        let at = dir.join("at.tw");
        std::fs::write(&at, "a".repeat(100)).unwrap();
        assert_eq!(read_to_string(&at, 100).unwrap().len(), 100);
        assert_eq!(read_to_string(&at, 99).unwrap_err().kind(), ErrorKind::FileTooLarge);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Step 3: the size is checked before reading. A sparse file has no
    /// data blocks, so this costs no disk, but `read_to_string` would have
    /// allocated the full gigabyte.
    #[test]
    fn a_file_too_big_by_its_size_is_refused_before_reading() {
        let dir = scratch("sparse");
        let huge = dir.join("huge.tw");
        File::create(&huge).unwrap().set_len(1 << 30).unwrap();
        let error = read_to_string(&huge, 1 << 20).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::FileTooLarge);
        assert!(error.to_string().contains("1048576-byte limit"), "{error}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Step 4 alone: a reader longer than its size claimed (a file that
    /// grew after the size check) is cut off at `limit + 1` bytes.
    #[test]
    fn a_reader_longer_than_the_limit_is_refused_after_limit_plus_one_bytes() {
        /// An endless reader of `a`s that counts every byte it hands out.
        struct Endless {
            handed_out: u64,
        }
        impl Read for Endless {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                buf.fill(b'a');
                self.handed_out += buf.len() as u64;
                Ok(buf.len())
            }
        }
        let mut endless = Endless { handed_out: 0 };
        let error = read_bounded(&mut endless, 4096).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::FileTooLarge);
        assert_eq!(endless.handed_out, 4097, "exactly limit + 1 bytes were read");
        assert_eq!(read_bounded(&b"abc"[..], 3).unwrap(), "abc");
    }

    #[test]
    fn bytes_that_are_not_utf8_are_invalid_data() {
        assert_eq!(read_bounded(&[b'1', 0xFF][..], 10).unwrap_err().kind(), ErrorKind::InvalidData);
    }

    #[test]
    fn a_directory_is_not_a_regular_file() {
        let dir = scratch("dir");
        assert_eq!(read_to_string(&dir, 100).unwrap_err().kind(), ErrorKind::InvalidInput);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_missing_file_is_the_open_error() {
        let missing = scratch("missing").join("nope.tw");
        assert_eq!(read_to_string(&missing, 100).unwrap_err().kind(), ErrorKind::NotFound);
    }

    /// `/dev/zero` reports a size of 0 and never ends; it is a character
    /// device, so step 1 refuses it before it is opened.
    #[cfg(unix)]
    #[test]
    fn a_device_is_refused_before_it_is_opened() {
        let error = read_to_string(Path::new("/dev/zero"), 1 << 20).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidInput);
        assert!(error.to_string().contains("not a regular file"), "{error}");
    }

    /// A FIFO with no writer blocks a plain `open` for reading forever. It
    /// is refused, and quickly: the call runs on a thread, and the test
    /// fails rather than hangs if it does not come back.
    #[cfg(unix)]
    #[test]
    fn a_fifo_is_refused_without_blocking() {
        let dir = scratch("fifo");
        let fifo = dir.join("pipe.tw");
        let made = std::process::Command::new("mkfifo").arg(&fifo).status();
        if !made.map(|s| s.success()).unwrap_or(false) {
            eprintln!("mkfifo unavailable; skipping");
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let path = fifo.clone();
        std::thread::spawn(move || {
            let _ = tx.send(read_to_string(&path, 1 << 20).map_err(|e| e.kind()));
        });
        let outcome = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("reading a FIFO must not block");
        assert_eq!(outcome, Err(ErrorKind::InvalidInput));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Step 2 on its own: the race step 1 cannot close (the path becomes a
    /// FIFO after the check) is closed by opening non-blocking, so even a
    /// direct open of a writerless FIFO returns, and step 3 refuses it.
    #[cfg(unix)]
    #[test]
    fn opening_a_fifo_does_not_block_and_the_handle_check_refuses_it() {
        let dir = scratch("fifo_open");
        let fifo = dir.join("pipe.tw");
        let made = std::process::Command::new("mkfifo").arg(&fifo).status();
        if !made.map(|s| s.success()).unwrap_or(false) {
            eprintln!("mkfifo unavailable; skipping");
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let path = fifo.clone();
        std::thread::spawn(move || {
            let opened = open_without_blocking(&path).and_then(|f| f.metadata());
            let _ = tx.send(opened.map(|m| m.is_file()).map_err(|e| e.kind()));
        });
        let outcome = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("a non-blocking open of a FIFO returns");
        assert_eq!(outcome, Ok(false), "the handle is not a regular file");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
