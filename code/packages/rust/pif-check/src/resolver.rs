//! The import resolver (VER32 1.1 and E5): digest in, verified bytes out.
//!
//! # Why this is trusted
//!
//! A chain is a list of *digests*. The resolver is the code that turns each into bytes, and
//! it is part of `pif.reader.correct`: if it handed the reader a *different* file than the one
//! the sign-off names, the kernel would check the wrong environment. So it follows three rules.
//!
//! 1. **Verify before use.** [`Resolver::resolve`] hashes the bytes it was given and compares
//!    with the digest it was asked for ([`CheckError::DigestMismatch`], rule R-IMP-2). Nothing
//!    reaches the reader unverified, whatever the [`Source`] is.
//! 2. **Where the bytes come from is untrusted.** A [`Source`] only *offers* bytes. The
//!    directory source ([`DirSource`]) maps a digest to `<store>/<64 hex>.pif`; because a
//!    [`Digest`] is exactly 64 lowercase hex digits, the file name can contain no path
//!    separator and no `..`, so the digest cannot be used to escape the store.
//! 3. **No network, no search path.** Only the explicit store directory is consulted. A
//!    symbolic link in the store is refused (it could point outside), and the file size is
//!    capped *before* the file is read into memory.

use crate::digest::Digest;
use crate::error::CheckError;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// The default cap on one file: the reader's own `PifLimits::file_bytes` (64 MiB).
pub const DEFAULT_MAX_FILE_BYTES: u64 = 64 << 20;

/// Something that can offer the bytes it believes have a given digest. It is **not trusted**:
/// [`Resolver`] re-hashes whatever it returns.
pub trait Source {
    /// The bytes filed under `d`, or why there are none.
    fn get(&self, d: &Digest) -> Result<Vec<u8>, CheckError>;
}

/// A directory of `<64 hex>.pif` files.
pub struct DirSource {
    root: PathBuf,
    max_file_bytes: u64,
}

impl DirSource {
    /// A store rooted at `root` with the default size cap.
    pub fn new(root: impl Into<PathBuf>) -> DirSource {
        DirSource {
            root: root.into(),
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
        }
    }

    /// Lower (or raise) the per-file size cap.
    pub fn with_max_file_bytes(mut self, max: u64) -> DirSource {
        self.max_file_bytes = max;
        self
    }

    /// The path a digest maps to. Safe by construction: 64 hex digits and a fixed suffix.
    pub fn path_for(&self, d: &Digest) -> PathBuf {
        self.root.join(format!("{}.pif", d.hex()))
    }

    fn io(d: &Digest, e: &std::io::Error) -> CheckError {
        if e.kind() == std::io::ErrorKind::NotFound {
            CheckError::FileMissing(d.hex())
        } else {
            CheckError::Io {
                digest: d.hex(),
                kind: format!("{:?}", e.kind()),
            }
        }
    }
}

impl Source for DirSource {
    fn get(&self, d: &Digest) -> Result<Vec<u8>, CheckError> {
        let path: &Path = &self.path_for(d);
        // `symlink_metadata` does not follow a link, so a link is seen as a link and refused.
        let meta = fs::symlink_metadata(path).map_err(|e| DirSource::io(d, &e))?;
        if !meta.is_file() {
            return Err(CheckError::NotRegularFile(d.hex()));
        }
        if meta.len() > self.max_file_bytes {
            return Err(CheckError::FileTooLarge(d.hex()));
        }
        let file = fs::File::open(path).map_err(|e| DirSource::io(d, &e))?;
        // Read at most cap + 1 bytes: a file that grew after the metadata check is caught
        // without ever buffering more than the cap.
        let mut bytes = Vec::new();
        file.take(self.max_file_bytes + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| DirSource::io(d, &e))?;
        if bytes.len() as u64 > self.max_file_bytes {
            return Err(CheckError::FileTooLarge(d.hex()));
        }
        Ok(bytes)
    }
}

/// An in-memory store, for tests and for callers that already hold the bytes. It files bytes
/// under the digest the caller *claims*, so a test can plant a wrong file.
#[derive(Default)]
pub struct MemSource(pub BTreeMap<Digest, Vec<u8>>);

impl MemSource {
    /// File `bytes` under their own digest (the honest case); returns that digest.
    pub fn add(&mut self, bytes: &[u8]) -> Digest {
        let d = Digest::of(bytes);
        self.0.insert(d, bytes.to_vec());
        d
    }
}

impl Source for MemSource {
    fn get(&self, d: &Digest) -> Result<Vec<u8>, CheckError> {
        self.0
            .get(d)
            .cloned()
            .ok_or_else(|| CheckError::FileMissing(d.hex()))
    }
}

/// The resolver proper: a [`Source`] plus the verification that makes its answers usable.
pub struct Resolver<'a> {
    source: &'a dyn Source,
}

impl<'a> Resolver<'a> {
    /// Wrap a source.
    pub fn new(source: &'a dyn Source) -> Resolver<'a> {
        Resolver { source }
    }

    /// The bytes whose SHA-256 is `d`, verified (E5, R-IMP-2).
    pub fn resolve(&self, d: &Digest) -> Result<Vec<u8>, CheckError> {
        let bytes = self.source.get(d)?;
        let actual = Digest::of(&bytes);
        if actual != *d {
            return Err(CheckError::DigestMismatch {
                expected: d.hex(),
                actual: actual.hex(),
            });
        }
        Ok(bytes)
    }
}
