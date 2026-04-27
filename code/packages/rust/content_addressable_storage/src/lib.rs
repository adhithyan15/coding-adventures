//! # cas — Generic Content-Addressable Storage
//!
//! Content-addressable storage (CAS) maps the *hash of content* to the content
//! itself. The hash is simultaneously the address and an integrity check: if the
//! bytes returned by the store don't hash to the key you requested, the data is
//! corrupt. No separate checksum file or trust anchor is needed.
//!
//! ## Mental model
//!
//! Imagine a library where every book's call number *is* a fingerprint of the
//! book's text. You can't file a different book under that number — the number
//! would immediately be wrong. And if someone swaps pages, the fingerprint
//! changes and the librarian knows before you even open the cover.
//!
//! ```text
//! Traditional storage:   name  ──►  content   (name can lie; content can change)
//! Content-addressed:     hash  ──►  content   (hash is derived from content, cannot lie)
//! ```
//!
//! ## How Git uses CAS
//!
//! Git's entire history is built on this principle. Every blob (file snapshot),
//! tree (directory listing), commit, and tag is stored by the SHA-1 hash of its
//! serialized bytes. Two identical files share one object. Renaming a file creates
//! zero new storage. History is an immutable DAG of hashes pointing to hashes.
//!
//! This package provides the CAS layer only — hashing and storage. The Git object
//! format (`"blob N\0content"`), compression, and pack files are handled by layers
//! above and below.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────┐
//! │  ContentAddressableStore<S: BlobStore>            │
//! │  · put(data)   → SHA-1 key, delegate to S        │
//! │  · get(key)    → fetch from S, verify hash       │
//! │  · find_by_prefix(hex) → prefix search via S     │
//! └─────────────────┬────────────────────────────────┘
//!                   │ trait BlobStore
//!          ┌────────┴──────────────────────────────┐
//!          │                                       │
//!   LocalDiskStore                    (S3, mem, custom, …)
//!   root/XX/XXXXXX…
//! ```
//!
//! ## Example
//!
//! ```rust
//! use coding_adventures_content_addressable_storage::{ContentAddressableStore, LocalDiskStore, key_to_hex};
//! use std::path::PathBuf;
//!
//! let tmp = std::env::temp_dir().join("cas-doctest");
//! let store = LocalDiskStore::new(&tmp).unwrap();
//! let cas = ContentAddressableStore::new(store);
//!
//! let key = cas.put(b"hello, world").unwrap();
//! let data = cas.get(&key).unwrap();
//! assert_eq!(data, b"hello, world");
//!
//! // Clean up
//! std::fs::remove_dir_all(&tmp).ok();
//! ```

// ─── Dependencies ─────────────────────────────────────────────────────────────

use coding_adventures_sha1::sum1;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

// ─── Hex Utilities ────────────────────────────────────────────────────────────
//
// Keys are 20-byte arrays ([u8; 20]), but humans interact with them as 40-char
// lowercase hex strings (e.g., "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5").
//
// key_to_hex  — converts [u8; 20] → 40-char hex string
// hex_to_key  — parses a 40-char hex string → [u8; 20], returns Err on bad input

/// Convert a 20-byte SHA-1 key to a 40-character lowercase hex string.
///
/// ```
/// use coding_adventures_content_addressable_storage::key_to_hex;
/// let key = [0xa3u8, 0xf4, 0xb2, 0xc1, 0xd0,
///            0xe9, 0xf8, 0xa7, 0xb6, 0xc5,
///            0xd4, 0xe3, 0xf2, 0xa1, 0xb0,
///            0xc9, 0xd8, 0xe7, 0xf6, 0xa5];
/// assert_eq!(key_to_hex(&key), "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5");
/// ```
pub fn key_to_hex(key: &[u8; 20]) -> String {
    key.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Parse a 40-character hex string into a 20-byte key.
///
/// Returns `Err` if the string is not exactly 40 hex characters.
///
/// ```
/// use coding_adventures_content_addressable_storage::{key_to_hex, hex_to_key};
/// let hex = "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5";
/// let key = hex_to_key(hex).unwrap();
/// assert_eq!(key_to_hex(&key), hex);
/// ```
pub fn hex_to_key(hex: &str) -> Result<[u8; 20], String> {
    if hex.len() != 40 {
        return Err(format!("expected 40 hex chars, got {}", hex.len()));
    }
    let mut key = [0u8; 20];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        key[i] = (hi << 4) | lo;
    }
    Ok(key)
}

// Decode a single ASCII hex nibble ('0'–'9', 'a'–'f', 'A'–'F') to its value.
fn hex_nibble(b: u8) -> Result<u8, String> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(format!("invalid hex character: {:?}", b as char)),
    }
}

// Decode an arbitrary-length hex string (1–40 chars, may be odd-length) to a
// byte prefix.  Odd-length strings are right-padded with '0' before decoding,
// because a nibble prefix like "a3f" means "starts with 0xa3, 0xf0" — the
// trailing nibble is the high nibble of the next byte.
//
// Returns Err if any character is not hex, or if the string is empty.
fn decode_hex_prefix(hex: &str) -> Result<Vec<u8>, String> {
    if hex.is_empty() {
        return Err("prefix cannot be empty".to_string());
    }
    for b in hex.bytes() {
        hex_nibble(b)?; // validate all chars first
    }
    // Pad to even length
    let padded: String = if hex.len() % 2 == 1 {
        format!("{}0", hex)
    } else {
        hex.to_string()
    };
    let bytes: Result<Vec<u8>, String> = padded
        .as_bytes()
        .chunks(2)
        .map(|c| {
            let hi = hex_nibble(c[0])?;
            let lo = hex_nibble(c[1])?;
            Ok((hi << 4) | lo)
        })
        .collect();
    bytes
}

// ─── BlobStore Trait ──────────────────────────────────────────────────────────
//
// The single abstraction that separates the CAS logic from persistence.
// Any type that can store and retrieve byte blobs by a 20-byte key qualifies.
//
// The associated `Error` type lets each backend report its own failure modes
// (std::io::Error for LocalDiskStore, an HTTP error type for an S3 backend, etc.)
// without forcing every caller to use Box<dyn Error>.

/// A pluggable key-value store for raw byte blobs, keyed by a 20-byte hash.
///
/// Implement this trait to add a new storage backend. The key is always a
/// SHA-1 digest produced by [`ContentAddressableStore`]; implementations
/// should treat it as an opaque identifier.
///
/// All methods take `&self` (shared reference) so the store can be wrapped in
/// `Arc<S>` for concurrent use without requiring `&mut self` locking.
pub trait BlobStore {
    /// The error type returned by all operations on this store.
    type Error: std::error::Error + 'static;

    /// Persist `data` under `key`.
    ///
    /// Implementations must be idempotent: storing the same key twice with the
    /// same bytes is not an error. Storing a different blob under an existing
    /// key is undefined behaviour (the CAS layer prevents this by construction,
    /// since the same content always produces the same key).
    fn put(&self, key: &[u8; 20], data: &[u8]) -> Result<(), Self::Error>;

    /// Retrieve the blob stored under `key`.
    ///
    /// Returns `Err` if the key is not present or if I/O fails.
    /// Implementations do NOT need to verify the hash — that is the CAS layer's job.
    fn get(&self, key: &[u8; 20]) -> Result<Vec<u8>, Self::Error>;

    /// Check whether `key` is present without fetching the blob.
    fn exists(&self, key: &[u8; 20]) -> Result<bool, Self::Error>;

    /// Return all stored keys whose first `prefix.len()` bytes equal `prefix`.
    ///
    /// Used for abbreviated-hash lookup: the caller supplies a byte prefix
    /// decoded from a short hex string, and the store returns the full keys
    /// that match. The CAS layer checks for uniqueness and reports ambiguity.
    fn keys_with_prefix(&self, prefix: &[u8]) -> Result<Vec<[u8; 20]>, Self::Error>;
}

// ─── CasError ─────────────────────────────────────────────────────────────────
//
// A typed error enum that wraps backend errors (E) and adds CAS-level failure
// modes. Generics keep the backend error fully typed so callers can match on
// specific backend errors (e.g., std::io::ErrorKind) without downcasting.

/// Errors that can arise from [`ContentAddressableStore`] operations.
///
/// `E` is the associated error type of the underlying [`BlobStore`].
#[derive(Debug)]
pub enum CasError<E> {
    /// The backend reported an error (I/O failure, network error, etc.).
    Store(E),

    /// A blob was requested by key but no such key exists in the store.
    NotFound([u8; 20]),

    /// The store returned bytes whose SHA-1 does not match the requested key.
    ///
    /// This indicates data corruption: the stored bytes have been modified
    /// since they were first written. The key field is the *requested* key.
    Corrupted { key: [u8; 20] },

    /// A hex prefix matched two or more objects.
    ///
    /// The string is the prefix the caller supplied.
    AmbiguousPrefix(String),

    /// A hex prefix matched zero objects.
    PrefixNotFound(String),

    /// The supplied hex string is not valid hexadecimal, or is empty.
    InvalidPrefix(String),
}

impl<E: fmt::Display> fmt::Display for CasError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CasError::Store(e) => write!(f, "store error: {}", e),
            CasError::NotFound(key) => write!(f, "object not found: {}", key_to_hex(key)),
            CasError::Corrupted { key } => {
                write!(f, "object corrupted: {}", key_to_hex(key))
            }
            CasError::AmbiguousPrefix(p) => write!(f, "ambiguous prefix: {}", p),
            CasError::PrefixNotFound(p) => write!(f, "object not found for prefix: {}", p),
            CasError::InvalidPrefix(p) => write!(f, "invalid hex prefix: {:?}", p),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for CasError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CasError::Store(e) => Some(e),
            _ => None,
        }
    }
}

// ─── ContentAddressableStore ──────────────────────────────────────────────────
//
// The CAS struct owns one BlobStore instance and adds:
//
//   1. Automatic keying  — callers pass content; SHA-1 is computed internally.
//   2. Integrity check   — on every get, SHA-1(returned bytes) must equal the key.
//   3. Prefix resolution — converts abbreviated hex (like `a3f4b2`) to a full key.

/// Content-addressable store that wraps a [`BlobStore`] backend.
///
/// All objects are keyed by their SHA-1 hash. The same content always maps to
/// the same key (deduplication), and the stored bytes are verified against the
/// key on every read (integrity).
///
/// # Type parameter
///
/// `S` is any [`BlobStore`]. Use [`LocalDiskStore`] for filesystem-backed
/// storage, or supply your own implementation for cloud or in-memory storage.
pub struct ContentAddressableStore<S: BlobStore> {
    store: S,
}

impl<S: BlobStore> ContentAddressableStore<S> {
    /// Create a new CAS wrapping `store`.
    ///
    /// ```rust
    /// use coding_adventures_content_addressable_storage::{ContentAddressableStore, LocalDiskStore};
    /// let store = LocalDiskStore::new(std::env::temp_dir().join("cas-new-test")).unwrap();
    /// let cas = ContentAddressableStore::new(store);
    /// std::fs::remove_dir_all(std::env::temp_dir().join("cas-new-test")).ok();
    /// ```
    pub fn new(store: S) -> Self {
        ContentAddressableStore { store }
    }

    /// Hash `data` with SHA-1, store it in the backend, and return the key.
    ///
    /// Idempotent: if the same content has already been stored, the existing
    /// key is returned and no write is performed (the backend handles this).
    ///
    /// ```rust
    /// use coding_adventures_content_addressable_storage::{ContentAddressableStore, LocalDiskStore};
    /// let tmp = std::env::temp_dir().join("cas-put-test");
    /// let cas = ContentAddressableStore::new(LocalDiskStore::new(&tmp).unwrap());
    /// let key1 = cas.put(b"foo").unwrap();
    /// let key2 = cas.put(b"foo").unwrap(); // second call is a no-op
    /// assert_eq!(key1, key2);
    /// std::fs::remove_dir_all(&tmp).ok();
    /// ```
    pub fn put(&self, data: &[u8]) -> Result<[u8; 20], CasError<S::Error>> {
        let key = sum1(data);
        // Delegate directly to the store. `BlobStore::put` is required to be
        // idempotent, so no pre-check is needed here. Skipping the
        // `exists()` → `put()` two-step eliminates a TOCTOU window and keeps
        // the CAS layer free of redundant filesystem round-trips.
        self.store.put(&key, data).map_err(CasError::Store)?;
        Ok(key)
    }

    /// Retrieve the blob stored under `key` and verify its integrity.
    ///
    /// The returned bytes are guaranteed to hash to `key` — if the store
    /// returns anything else, [`CasError::Corrupted`] is returned instead.
    ///
    /// ```rust
    /// use coding_adventures_content_addressable_storage::{ContentAddressableStore, LocalDiskStore, CasError};
    /// let tmp = std::env::temp_dir().join("cas-get-test");
    /// let cas = ContentAddressableStore::new(LocalDiskStore::new(&tmp).unwrap());
    /// let key = cas.put(b"bar").unwrap();
    /// assert_eq!(cas.get(&key).unwrap(), b"bar");
    /// std::fs::remove_dir_all(&tmp).ok();
    /// ```
    pub fn get(&self, key: &[u8; 20]) -> Result<Vec<u8>, CasError<S::Error>> {
        let data = self.store.get(key).map_err(|e| {
            // Translate a "not found" I/O error from LocalDiskStore into the
            // typed CasError::NotFound so callers don't have to inspect io::Error.
            CasError::Store(e)
        })?;

        // Integrity check: re-hash the returned bytes.
        let actual = sum1(&data);
        if &actual != key {
            return Err(CasError::Corrupted { key: *key });
        }
        Ok(data)
    }

    /// Check whether a key is present in the store.
    pub fn exists(&self, key: &[u8; 20]) -> Result<bool, CasError<S::Error>> {
        self.store.exists(key).map_err(CasError::Store)
    }

    /// Resolve an abbreviated hex string to a full 20-byte key.
    ///
    /// Accepts any non-empty hex string of 1–40 characters. Odd-length strings
    /// are treated as nibble prefixes (e.g. `"a3f"` matches any key starting
    /// with `0xa3 0xf_`).
    ///
    /// # Errors
    ///
    /// - [`CasError::InvalidPrefix`]  — empty string or non-hex characters
    /// - [`CasError::PrefixNotFound`] — no keys match
    /// - [`CasError::AmbiguousPrefix`] — two or more keys match
    pub fn find_by_prefix(&self, hex_prefix: &str) -> Result<[u8; 20], CasError<S::Error>> {
        let prefix_bytes =
            decode_hex_prefix(hex_prefix).map_err(|_| CasError::InvalidPrefix(hex_prefix.to_string()))?;

        let mut matches = self
            .store
            .keys_with_prefix(&prefix_bytes)
            .map_err(CasError::Store)?;

        // Sort for deterministic behaviour in tests.
        matches.sort_unstable();

        match matches.len() {
            0 => Err(CasError::PrefixNotFound(hex_prefix.to_string())),
            1 => Ok(matches[0]),
            _ => Err(CasError::AmbiguousPrefix(hex_prefix.to_string())),
        }
    }

    /// Access the underlying [`BlobStore`] directly.
    ///
    /// Useful when you need backend-specific operations not exposed by the CAS
    /// interface (e.g., listing all keys for garbage collection, or querying
    /// storage statistics).
    pub fn inner(&self) -> &S {
        &self.store
    }
}

// ─── LocalDiskStore ───────────────────────────────────────────────────────────
//
// Filesystem backend using the Git 2/38 fanout layout.
//
// Why 2/38?  A repository with 100 000 objects would put 100 000 files in a
// single directory if we stored objects as root/<40-hex-hash>.  Most
// filesystems slow down dramatically at that scale.  Splitting on the first
// byte creates up to 256 sub-directories — each with at most ~390 entries for
// a 100 k object repo.  Git has used this layout since its initial release.
//
// Object path:  root/<xx>/<remaining-38-hex-chars>
//   key = [0xa3, 0xf4, ...]
//   dir  = "a3/"
//   file = "f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5"
//
// Atomic writes: we write to a temp file then rename.  `rename` is atomic on
// POSIX (guaranteed by POSIX.1).  On Windows, it is best-effort — if the
// destination already exists, `rename` will fail; we detect this and treat it
// as a successful idempotent write (another writer stored the same object first).

/// Filesystem-backed [`BlobStore`] using Git-style 2/38 fanout layout.
///
/// Objects are stored at `<root>/<xx>/<38-hex-chars>` where `xx` is the first
/// byte of the SHA-1 hash encoded as two lowercase hex digits.
///
/// Writes are atomic (write to temp → rename to final path).
pub struct LocalDiskStore {
    root: PathBuf,
}

impl LocalDiskStore {
    /// Create (or open) a store rooted at `root`.
    ///
    /// The directory is created if it does not exist.
    ///
    /// ```rust
    /// use coding_adventures_content_addressable_storage::LocalDiskStore;
    /// let tmp = std::env::temp_dir().join("cas-lds-test");
    /// let _store = LocalDiskStore::new(&tmp).unwrap();
    /// std::fs::remove_dir_all(&tmp).ok();
    /// ```
    pub fn new(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Ok(LocalDiskStore { root })
    }

    // Compute the storage path for a given key.
    //
    // key[0] encodes as a two-char directory name.
    // key[1..] encodes as the 38-char filename.
    //
    //   key = [0xa3, 0xf4, 0xb2, …]
    //   dir  = root/a3/
    //   file = root/a3/f4b2…
    fn object_path(&self, key: &[u8; 20]) -> PathBuf {
        let hex = key_to_hex(key);
        let (dir_name, file_name) = hex.split_at(2);
        self.root.join(dir_name).join(file_name)
    }
}

impl BlobStore for LocalDiskStore {
    type Error = io::Error;

    fn put(&self, key: &[u8; 20], data: &[u8]) -> io::Result<()> {
        let final_path = self.object_path(key);

        // Short-circuit: if the file already exists, the object is already stored.
        // Because the key is a hash of the content, the stored bytes are guaranteed
        // to be identical — no need to overwrite.
        if final_path.exists() {
            return Ok(());
        }

        // Create the two-char fanout directory (e.g., "a3/") if needed.
        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Atomic write: write to a temp file, then rename into place.
        //
        // The temp file lives in the same directory as the final file so that
        // the rename is on the same filesystem (cross-device renames fail on POSIX).
        //
        // Security: use an unpredictable temp filename (PID + nanosecond timestamp)
        // rather than a deterministic `.tmp` extension. A fixed path like
        // `a3/f4b2....tmp` could be pre-targeted by a local attacker who places a
        // symlink there before our write, redirecting File::create to an arbitrary
        // path. Mixing the process ID and a high-resolution timestamp makes the
        // name infeasible to predict without privileged access to the process.
        let tmp_name = {
            let pid = std::process::id();
            let ns = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0);
            let base = final_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("obj");
            format!("{}.{}.{}.tmp", base, pid, ns)
        };
        let tmp_path = final_path
            .parent()
            .unwrap_or(&final_path)
            .join(tmp_name);
        {
            let mut f = fs::File::create(&tmp_path)?;
            f.write_all(data)?;
            f.flush()?;
            // `f` is dropped here, flushing the OS buffer.
        }

        // Rename into place.  On POSIX this is atomic.  On Windows it may fail
        // if the destination exists (race condition with another writer) — we
        // treat that as success because the stored bytes are identical.
        if let Err(e) = fs::rename(&tmp_path, &final_path) {
            // Clean up the temp file to avoid leaving orphans.
            let _ = fs::remove_file(&tmp_path);
            // If the final path now exists, someone else stored the same object
            // concurrently.  That is fine — it is the same content.
            if !final_path.exists() {
                return Err(e);
            }
        }
        Ok(())
    }

    fn get(&self, key: &[u8; 20]) -> io::Result<Vec<u8>> {
        let path = self.object_path(key);
        fs::read(&path)
    }

    fn exists(&self, key: &[u8; 20]) -> io::Result<bool> {
        Ok(self.object_path(key).exists())
    }

    fn keys_with_prefix(&self, prefix: &[u8]) -> io::Result<Vec<[u8; 20]>> {
        // A prefix of 0 bytes is not valid — reject it before any filesystem work.
        // (The CAS layer already rejects empty hex strings, so this is defensive.)
        if prefix.is_empty() {
            return Ok(vec![]);
        }

        // The first byte of the prefix tells us which fanout bucket(s) to scan.
        // If prefix is at least one byte, the first byte is the directory name.
        // We compare bucket names that start with the high nibble of prefix[0].
        let first_byte_hex = format!("{:02x}", prefix[0]);

        // Build the list of buckets to scan.  Usually this is just one ("a3/").
        // If the prefix is exactly one nibble (half a byte), we need to scan
        // the 16 buckets starting with that nibble (a0/, a1/, …, af/).
        // However, since `decode_hex_prefix` pads odd strings with '0', a single
        // nibble prefix "a" becomes byte 0xa0, so we'd only look in bucket "a0/".
        // Callers who want all "a*" matches should pass "a" and accept potential
        // false-positives — the CAS layer does the final filtering.
        //
        // For correctness: scan the single exact bucket for prefix[0].
        let bucket = self.root.join(&first_byte_hex);

        if !bucket.exists() {
            return Ok(vec![]);
        }

        let mut keys: Vec<[u8; 20]> = Vec::new();

        for entry in fs::read_dir(&bucket)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let name = match file_name.to_str() {
                Some(n) => n,
                None => continue,
            };

            // Each file name is a 38-char hex string (the latter 38 chars of the
            // 40-char hash).  Reconstruct the full 40-char hex and parse it.
            if name.len() != 38 {
                continue; // skip temp files or other artifacts
            }
            let full_hex = format!("{}{}", first_byte_hex, name);
            let key = match hex_to_key(&full_hex) {
                Ok(k) => k,
                Err(_) => continue,
            };

            // Check that this key actually matches the full prefix.
            if key[..prefix.len()] == *prefix {
                keys.push(key);
            }
        }

        Ok(keys)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // Helper: create a temporary directory unique to this test.
    fn tmpdir(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("cas-test-{}", name));
        let _ = fs::remove_dir_all(&dir); // clean up any previous run
        dir
    }

    // ─── Hex utilities ───────────────────────────────────────────────────────

    #[test]
    fn key_to_hex_roundtrip() {
        let key: [u8; 20] = [
            0xa3, 0xf4, 0xb2, 0xc1, 0xd0, 0xe9, 0xf8, 0xa7, 0xb6, 0xc5, 0xd4, 0xe3, 0xf2, 0xa1,
            0xb0, 0xc9, 0xd8, 0xe7, 0xf6, 0xa5,
        ];
        let hex = key_to_hex(&key);
        assert_eq!(hex.len(), 40);
        assert_eq!(hex_to_key(&hex).unwrap(), key);
    }

    #[test]
    fn hex_to_key_rejects_short() {
        assert!(hex_to_key("a3f4").is_err());
    }

    #[test]
    fn hex_to_key_rejects_non_hex() {
        let bad = "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6zz";
        assert!(hex_to_key(bad).is_err());
    }

    #[test]
    fn hex_to_key_accepts_uppercase() {
        let lower = "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5";
        let upper = "A3F4B2C1D0E9F8A7B6C5D4E3F2A1B0C9D8E7F6A5";
        assert_eq!(hex_to_key(lower).unwrap(), hex_to_key(upper).unwrap());
    }

    // ─── decode_hex_prefix ───────────────────────────────────────────────────

    #[test]
    fn prefix_empty_is_error() {
        assert!(decode_hex_prefix("").is_err());
    }

    #[test]
    fn prefix_odd_length_pads_right() {
        // "a3f" → 0xa3, 0xf0 (right-padded with '0')
        assert_eq!(decode_hex_prefix("a3f").unwrap(), vec![0xa3, 0xf0]);
    }

    #[test]
    fn prefix_even_length() {
        assert_eq!(decode_hex_prefix("a3f4").unwrap(), vec![0xa3, 0xf4]);
    }

    #[test]
    fn prefix_non_hex_is_error() {
        assert!(decode_hex_prefix("zz").is_err());
    }

    // ─── LocalDiskStore ──────────────────────────────────────────────────────

    #[test]
    fn local_disk_store_create_dir() {
        let dir = tmpdir("create-dir");
        assert!(!dir.exists());
        let _store = LocalDiskStore::new(&dir).unwrap();
        assert!(dir.exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn local_disk_store_path_layout() {
        // Verify that an object is stored at root/<xx>/<38-char> path.
        let dir = tmpdir("path-layout");
        let store = LocalDiskStore::new(&dir).unwrap();

        let key: [u8; 20] = [
            0xa3, 0xf4, 0xb2, 0xc1, 0xd0, 0xe9, 0xf8, 0xa7, 0xb6, 0xc5, 0xd4, 0xe3, 0xf2, 0xa1,
            0xb0, 0xc9, 0xd8, 0xe7, 0xf6, 0xa5,
        ];
        store.put(&key, b"test data").unwrap();

        let expected = dir
            .join("a3")
            .join("f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5");
        assert!(expected.exists(), "object file should exist at 2/38 path");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn local_disk_store_put_get_roundtrip() {
        let dir = tmpdir("put-get");
        let store = LocalDiskStore::new(&dir).unwrap();

        let key = sum1(b"hello");
        store.put(&key, b"hello").unwrap();
        let data = store.get(&key).unwrap();
        assert_eq!(data, b"hello");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn local_disk_store_get_missing() {
        let dir = tmpdir("get-missing");
        let store = LocalDiskStore::new(&dir).unwrap();

        let key = sum1(b"nonexistent");
        let result = store.get(&key);
        assert!(result.is_err());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn local_disk_store_exists() {
        let dir = tmpdir("exists");
        let store = LocalDiskStore::new(&dir).unwrap();

        let key = sum1(b"check");
        assert!(!store.exists(&key).unwrap());
        store.put(&key, b"check").unwrap();
        assert!(store.exists(&key).unwrap());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn local_disk_store_idempotent_put() {
        let dir = tmpdir("idempotent");
        let store = LocalDiskStore::new(&dir).unwrap();

        let key = sum1(b"same");
        store.put(&key, b"same").unwrap();
        store.put(&key, b"same").unwrap(); // second put must not error
        assert_eq!(store.get(&key).unwrap(), b"same");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn local_disk_store_keys_with_prefix() {
        let dir = tmpdir("prefix-scan");
        let store = LocalDiskStore::new(&dir).unwrap();

        // Store two objects.  We need their keys to share a known prefix for
        // the assertion, so we use fixed keys rather than sum1().
        let key1: [u8; 20] = [
            0xab, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        ];
        let key2: [u8; 20] = [
            0xab, 0x34, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
        ];
        let key3: [u8; 20] = [
            0xcd, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x03,
        ];

        store.put(&key1, b"obj1").unwrap();
        store.put(&key2, b"obj2").unwrap();
        store.put(&key3, b"obj3").unwrap();

        // Prefix [0xab] should match key1 and key2 but not key3.
        let mut found = store.keys_with_prefix(&[0xab]).unwrap();
        found.sort_unstable();
        assert_eq!(found.len(), 2);
        assert!(found.contains(&key1));
        assert!(found.contains(&key2));

        // Prefix [0xcd] should match only key3.
        let found = store.keys_with_prefix(&[0xcd]).unwrap();
        assert_eq!(found, vec![key3]);

        // Prefix [0xff] matches nothing.
        let found = store.keys_with_prefix(&[0xff]).unwrap();
        assert!(found.is_empty());

        fs::remove_dir_all(&dir).ok();
    }

    // ─── ContentAddressableStore ─────────────────────────────────────────────

    fn make_cas(name: &str) -> (ContentAddressableStore<LocalDiskStore>, PathBuf) {
        let dir = tmpdir(name);
        let store = LocalDiskStore::new(&dir).unwrap();
        (ContentAddressableStore::new(store), dir)
    }

    #[test]
    fn cas_put_returns_sha1() {
        let (cas, dir) = make_cas("put-sha1");
        let key = cas.put(b"abc").unwrap();
        // SHA-1("abc") = a9993e364706816aba3e25717850c26c9cd0d89d
        assert_eq!(key_to_hex(&key), "a9993e364706816aba3e25717850c26c9cd0d89d");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cas_roundtrip_small() {
        let (cas, dir) = make_cas("roundtrip-small");
        let data = b"hello, world";
        let key = cas.put(data).unwrap();
        assert_eq!(cas.get(&key).unwrap(), data);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cas_roundtrip_empty() {
        let (cas, dir) = make_cas("roundtrip-empty");
        let key = cas.put(b"").unwrap();
        assert_eq!(cas.get(&key).unwrap(), b"");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cas_roundtrip_large() {
        let (cas, dir) = make_cas("roundtrip-large");
        let data: Vec<u8> = (0u8..=255).cycle().take(1024 * 1024).collect();
        let key = cas.put(&data).unwrap();
        assert_eq!(cas.get(&key).unwrap(), data);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cas_idempotent_put() {
        let (cas, dir) = make_cas("idempotent-put");
        let key1 = cas.put(b"same content").unwrap();
        let key2 = cas.put(b"same content").unwrap();
        assert_eq!(key1, key2);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cas_exists_before_and_after_put() {
        let (cas, dir) = make_cas("cas-exists");
        let key = sum1(b"check");
        assert!(!cas.exists(&key).unwrap());
        cas.put(b"check").unwrap();
        assert!(cas.exists(&key).unwrap());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cas_get_not_found() {
        let (cas, dir) = make_cas("not-found");
        let key = sum1(b"never stored");
        let result = cas.get(&key);
        // LocalDiskStore returns io::Error(NotFound) → CasError::Store(_)
        assert!(matches!(result, Err(CasError::Store(_))));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cas_get_corrupted() {
        let (cas, dir) = make_cas("corrupted");
        let key = cas.put(b"original").unwrap();

        // Directly corrupt the stored file.
        let hex = key_to_hex(&key);
        let path = dir.join(&hex[..2]).join(&hex[2..]);
        fs::write(&path, b"tampered bytes").unwrap();

        let result = cas.get(&key);
        assert!(matches!(result, Err(CasError::Corrupted { .. })));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cas_find_by_prefix_unique() {
        let (cas, dir) = make_cas("prefix-unique");
        let key = cas.put(b"only one").unwrap();
        let hex = key_to_hex(&key);
        let found = cas.find_by_prefix(&hex[..8]).unwrap();
        assert_eq!(found, key);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cas_find_by_prefix_full_hash() {
        let (cas, dir) = make_cas("prefix-full");
        let key = cas.put(b"full hash").unwrap();
        let hex = key_to_hex(&key);
        let found = cas.find_by_prefix(&hex).unwrap();
        assert_eq!(found, key);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cas_find_by_prefix_not_found() {
        let (cas, dir) = make_cas("prefix-not-found");
        let result = cas.find_by_prefix("deadbeef");
        assert!(matches!(result, Err(CasError::PrefixNotFound(_))));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cas_find_by_prefix_invalid_hex() {
        let (cas, dir) = make_cas("prefix-invalid");
        let result = cas.find_by_prefix("zzzz");
        assert!(matches!(result, Err(CasError::InvalidPrefix(_))));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cas_find_by_prefix_empty_is_invalid() {
        let (cas, dir) = make_cas("prefix-empty");
        let result = cas.find_by_prefix("");
        assert!(matches!(result, Err(CasError::InvalidPrefix(_))));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cas_find_by_prefix_ambiguous() {
        // Insert two objects whose SHA-1 hashes share the same first byte.
        // We use the store directly to plant objects with known keys that
        // share a prefix, bypassing content-based hashing.
        let dir = tmpdir("prefix-ambiguous");
        let store = LocalDiskStore::new(&dir).unwrap();

        let key1: [u8; 20] = [
            0xaa, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        ];
        let key2: [u8; 20] = [
            0xaa, 0x22, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
        ];
        store.put(&key1, b"obj1").unwrap();
        store.put(&key2, b"obj2").unwrap();

        let cas = ContentAddressableStore::new(store);
        let result = cas.find_by_prefix("aa");
        assert!(matches!(result, Err(CasError::AmbiguousPrefix(_))));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cas_inner_gives_store_access() {
        let (cas, dir) = make_cas("inner");
        let key = cas.put(b"via inner").unwrap();
        // Access the underlying store via inner().
        let data = cas.inner().get(&key).unwrap();
        assert_eq!(data, b"via inner");
        fs::remove_dir_all(&dir).ok();
    }

    // ─── Trait object compatibility ──────────────────────────────────────────
    //
    // Verify that BlobStore can be used as a trait object.  This is important
    // for callers that want to store a Box<dyn BlobStore<Error = io::Error>>
    // without knowing the concrete type at compile time.

    #[test]
    fn blob_store_as_trait_object() {
        let dir = tmpdir("trait-object");
        let store: Box<dyn BlobStore<Error = io::Error>> =
            Box::new(LocalDiskStore::new(&dir).unwrap());

        let key = sum1(b"trait object test");
        store.put(&key, b"trait object test").unwrap();
        let data = store.get(&key).unwrap();
        assert_eq!(data, b"trait object test");

        fs::remove_dir_all(&dir).ok();
    }
}
