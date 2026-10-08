//! # The freshness anchor (VLT01 F11, P1.20b)
//!
//! The freshness index (F1) remembers what the vault last wrote, but it lives
//! in the same directory as the records. Someone who can write that
//! directory can put back an *old* index together with an *old* record it
//! pins, and every file involved is authentic (F10).
//!
//! An anchor is the one fact kept somewhere else: for each namespace, the
//! highest index epoch the vault has written. Every index write advances the
//! epoch, so an index older than the anchor is an old copy put back, and an
//! absent index the anchor remembers was deleted.
//!
//! ```text
//!   storage directory (attacker may write)     anchor directory (owner only)
//!   ─────────────────────────────────────      ──────────────────────────────
//!   __vault__/freshness/chief-secrets          6368696566...  "41\n"
//!       epoch 37  ← put back by someone   ✗    37 < 41: Tamper
//! ```
//!
//! The anchor is trusted because of **where** it is, not because of
//! cryptography. A MAC would not help: an old copy of a MAC'd anchor is just
//! as valid as the current one.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Where a sealed store keeps the epochs it must never go below.
///
/// `load` returns the highest epoch recorded for a namespace, or `None` if
/// none was ever recorded. `advance` records an epoch and never lowers the
/// stored value. Both report failures as an [`AnchorError`], never by
/// pretending the anchor is absent: an anchor that cannot be read must not
/// quietly disable the check it exists for.
pub trait FreshnessAnchor: Send + Sync {
    /// The highest epoch recorded for `namespace`.
    fn load(&self, namespace: &str) -> Result<Option<u64>, AnchorError>;
    /// Record `epoch` for `namespace`, unless a higher one is already there.
    fn advance(&self, namespace: &str, epoch: u64) -> Result<(), AnchorError>;
}

/// Why an anchor could not be read or written. Carries no paths and no
/// content, because it ends up in errors that may be logged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorError {
    /// The filesystem refused (permissions, I/O, a full disk).
    Io,
    /// An anchor file exists but is not exactly a canonical epoch, or is not
    /// a regular file.
    Invalid,
    /// The anchor directory is writable by someone other than its owner, so
    /// it cannot be trusted to be outside the attacker's reach.
    InsecureDirectory,
}

impl std::fmt::Display for AnchorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Io => "freshness anchor: filesystem error",
            Self::Invalid => "freshness anchor: invalid anchor file",
            Self::InsecureDirectory => "freshness anchor: directory is writable by others",
        })
    }
}

impl std::error::Error for AnchorError {}

/// The longest valid anchor file: `u64::MAX` is 20 digits, plus a newline.
const MAX_ANCHOR_BYTES: u64 = 21;

/// One file per namespace in an owner-only directory.
///
/// - File name: the namespace, hex-encoded, so no namespace can name a path.
/// - Content: the epoch in decimal, then `\n`. Nothing else is accepted: no
///   leading zeros, no sign, no whitespace.
/// - Writes: a temporary file in the same directory, synced, then renamed
///   over the old one, so a reader sees either the old epoch or the new one.
///
/// One file per namespace means two processes writing *different*
/// namespaces of one store never touch the same file.
pub struct FileFreshnessAnchor {
    directory: PathBuf,
    /// Serializes this instance's own read-modify-write. Other instances and
    /// other processes are excluded by the OS lock in `advance`.
    lock: Mutex<()>,
}

/// The lock file every `advance` holds exclusively. Two writers that each
/// read 4 and then wrote 6 and 5 would otherwise leave the anchor at 5,
/// below an index at 6.
const LOCK_FILE: &str = ".lock";

/// Makes temporary file names unique within this process.
static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);

impl FileFreshnessAnchor {
    /// Open (creating if needed) the anchor directory.
    ///
    /// On Unix the directory is created `0700`. An existing one that is
    /// writable by group or others is refused: such an anchor would sit
    /// inside the attacker's reach, and an anchor anyone can lower is worse
    /// than none because it reads as protection.
    pub fn open(directory: impl Into<PathBuf>) -> Result<Self, AnchorError> {
        let directory = directory.into();
        match fs::symlink_metadata(&directory) {
            Ok(metadata) => {
                if !metadata.is_dir() {
                    return Err(AnchorError::Invalid);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                create_private_dir(&directory)?;
            }
            Err(_) => return Err(AnchorError::Io),
        }
        check_private_dir(&directory)?;
        Ok(Self {
            directory,
            lock: Mutex::new(()),
        })
    }

    fn path_for(&self, namespace: &str) -> PathBuf {
        let mut name = String::with_capacity(namespace.len() * 2);
        for byte in namespace.as_bytes() {
            name.push_str(&format!("{byte:02x}"));
        }
        self.directory.join(name)
    }

    fn read(path: &Path) -> Result<Option<u64>, AnchorError> {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(AnchorError::Io),
        };
        if !metadata.file_type().is_file() {
            return Err(AnchorError::Invalid);
        }
        let mut bytes = Vec::new();
        File::open(path)
            .map_err(|_| AnchorError::Io)?
            .take(MAX_ANCHOR_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| AnchorError::Io)?;
        parse_epoch(&bytes).map(Some).ok_or(AnchorError::Invalid)
    }
}

/// `digits "\n"`, canonical: `0`, or no leading zero.
fn parse_epoch(bytes: &[u8]) -> Option<u64> {
    let digits = bytes.strip_suffix(b"\n")?;
    if digits.is_empty()
        || !digits.iter().all(u8::is_ascii_digit)
        || (digits.len() > 1 && digits[0] == b'0')
    {
        return None;
    }
    std::str::from_utf8(digits).ok()?.parse().ok()
}

impl FreshnessAnchor for FileFreshnessAnchor {
    fn load(&self, namespace: &str) -> Result<Option<u64>, AnchorError> {
        Self::read(&self.path_for(namespace))
    }

    fn advance(&self, namespace: &str, epoch: u64) -> Result<(), AnchorError> {
        let _guard = self.lock.lock().map_err(|_| AnchorError::Io)?;
        // Held across read, compare and rename, and released when dropped.
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.directory.join(LOCK_FILE))
            .map_err(|_| AnchorError::Io)?;
        lock.lock().map_err(|_| AnchorError::Io)?;
        let path = self.path_for(namespace);
        if Self::read(&path)?.is_some_and(|current| current >= epoch) {
            return Ok(());
        }
        let temporary = self.directory.join(format!(
            ".tmp-{}-{}",
            std::process::id(),
            NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
        ));
        let written = (|| {
            let mut file = create_private_file(&temporary)?;
            file.write_all(format!("{epoch}\n").as_bytes())?;
            file.sync_all()?;
            fs::rename(&temporary, &path)?;
            sync_dir(&self.directory)
        })();
        if written.is_err() {
            let _ = fs::remove_file(&temporary);
            return Err(AnchorError::Io);
        }
        Ok(())
    }
}

#[cfg(unix)]
fn create_private_dir(path: &Path) -> Result<(), AnchorError> {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new()
        .mode(0o700)
        .create(path)
        .map_err(|_| AnchorError::Io)
}

#[cfg(not(unix))]
fn create_private_dir(path: &Path) -> Result<(), AnchorError> {
    fs::create_dir(path).map_err(|_| AnchorError::Io)
}

#[cfg(unix)]
fn check_private_dir(path: &Path) -> Result<(), AnchorError> {
    use std::os::unix::fs::PermissionsExt;
    let mode = fs::metadata(path)
        .map_err(|_| AnchorError::Io)?
        .permissions()
        .mode();
    if mode & 0o022 != 0 {
        return Err(AnchorError::InsecureDirectory);
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_private_dir(_path: &Path) -> Result<(), AnchorError> {
    // Windows ACLs are not inspected here. The directory sits next to the
    // owner-only KEK file, whose own checks cover the same location.
    Ok(())
}

#[cfg(unix)]
fn create_private_file(path: &Path) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

#[cfg(not(unix))]
fn create_private_file(path: &Path) -> std::io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

#[cfg(unix)]
fn sync_dir(path: &Path) -> std::io::Result<()> {
    // The rename is durable only once the directory entry is.
    File::open(path)?.sync_all()
}

#[cfg(not(unix))]
fn sync_dir(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "vault-anchor-{label}-{}-{}",
                std::process::id(),
                NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&path);
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn epochs_round_trip_and_never_go_down() {
        let dir = TempDir::new("monotonic");
        // `open` creates only the anchor directory itself, never missing
        // parents: a typo in a path should fail, not build a tree.
        assert_eq!(
            FileFreshnessAnchor::open(dir.0.join("a")).err(),
            Some(AnchorError::Io)
        );
        fs::create_dir(&dir.0).unwrap();
        let anchor = FileFreshnessAnchor::open(dir.0.join("a")).unwrap();
        assert_eq!(anchor.load("ns/x").unwrap(), None);
        anchor.advance("ns/x", 0).unwrap();
        assert_eq!(anchor.load("ns/x").unwrap(), Some(0));
        anchor.advance("ns/x", 7).unwrap();
        anchor.advance("ns/x", 3).unwrap();
        assert_eq!(anchor.load("ns/x").unwrap(), Some(7));
        anchor.advance("ns/x", u64::MAX).unwrap();
        assert_eq!(anchor.load("ns/x").unwrap(), Some(u64::MAX));
        // Namespaces do not share a file.
        assert_eq!(anchor.load("ns").unwrap(), None);
        // A fresh handle sees the same state.
        let again = FileFreshnessAnchor::open(dir.0.join("a")).unwrap();
        assert_eq!(again.load("ns/x").unwrap(), Some(u64::MAX));
    }

    #[test]
    fn concurrent_writers_never_lower_the_anchor() {
        // Separate instances, as separate processes would be. Each thread
        // advances its own increasing sequence, and the anchor must end at
        // the overall maximum and never be seen below an epoch already
        // acknowledged.
        let dir = TempDir::new("concurrent");
        FileFreshnessAnchor::open(&dir.0).unwrap();
        let threads: Vec<_> = (0..4u64)
            .map(|t| {
                let path = dir.0.clone();
                std::thread::spawn(move || {
                    let anchor = FileFreshnessAnchor::open(path).unwrap();
                    for i in 0..200u64 {
                        let epoch = i * 4 + t;
                        anchor.advance("ns", epoch).unwrap();
                        assert!(anchor.load("ns").unwrap().unwrap() >= epoch);
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        let anchor = FileFreshnessAnchor::open(&dir.0).unwrap();
        assert_eq!(anchor.load("ns").unwrap(), Some(199 * 4 + 3));
    }

    #[test]
    fn only_a_canonical_epoch_is_accepted() {
        for good in [&b"0\n"[..], b"41\n", b"18446744073709551615\n"] {
            assert!(parse_epoch(good).is_some(), "{good:?}");
        }
        for bad in [
            &b""[..],
            b"\n",
            b"41",
            b"041\n",
            b"+4\n",
            b" 4\n",
            b"4\n\n",
            b"18446744073709551616\n",
            b"4x\n",
        ] {
            assert_eq!(parse_epoch(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_damaged_anchor_is_an_error_not_absent() {
        let dir = TempDir::new("damaged");
        let anchor = FileFreshnessAnchor::open(&dir.0).unwrap();
        anchor.advance("ns", 5).unwrap();
        fs::write(anchor.path_for("ns"), b"five\n").unwrap();
        assert_eq!(anchor.load("ns"), Err(AnchorError::Invalid));
        assert_eq!(anchor.advance("ns", 6), Err(AnchorError::Invalid));
        fs::remove_file(anchor.path_for("ns")).unwrap();
        fs::create_dir(anchor.path_for("ns")).unwrap();
        assert_eq!(anchor.load("ns"), Err(AnchorError::Invalid));
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_anchor_file_is_refused() {
        let dir = TempDir::new("symlink");
        let anchor = FileFreshnessAnchor::open(&dir.0).unwrap();
        let elsewhere = dir.0.join("elsewhere");
        fs::write(&elsewhere, b"9\n").unwrap();
        std::os::unix::fs::symlink(&elsewhere, anchor.path_for("ns")).unwrap();
        assert_eq!(anchor.load("ns"), Err(AnchorError::Invalid));
    }

    #[cfg(unix)]
    #[test]
    fn the_directory_must_be_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new("private");
        let anchor = FileFreshnessAnchor::open(&dir.0).unwrap();
        let mode = fs::metadata(&dir.0).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700);
        anchor.advance("ns", 1).unwrap();
        let file_mode = fs::metadata(anchor.path_for("ns"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(file_mode, 0o600);

        fs::set_permissions(&dir.0, fs::Permissions::from_mode(0o777)).unwrap();
        assert!(matches!(
            FileFreshnessAnchor::open(&dir.0),
            Err(AnchorError::InsecureDirectory)
        ));
        // A path that is a file, not a directory.
        let file = dir.0.join("not-a-dir");
        fs::set_permissions(&dir.0, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(&file, b"").unwrap();
        assert!(matches!(
            FileFreshnessAnchor::open(&file),
            Err(AnchorError::Invalid)
        ));
    }
}
