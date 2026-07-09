//! Errors this reader returns instead of panicking.
//!
//! Every input to `sqlite-file` is **untrusted** — the database bytes come from
//! a user-supplied Anki `.apkg`. A corrupt or hostile file must produce a clean
//! `Err`, never a panic or an out-of-bounds read. So the whole crate is
//! fallible: parsing functions return `Result<_, SqliteError>` and each variant
//! below names one concrete way a byte stream can fail to be the SQLite file we
//! expect.

use core::fmt;

/// Something is wrong with the SQLite bytes we were handed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SqliteError {
    /// The file does not begin with the 16-byte `"SQLite format 3\0"` magic.
    BadMagic,
    /// The file is shorter than the structure we were about to read (e.g. fewer
    /// than 100 bytes for the header, or a page that runs past end-of-file).
    /// The `&'static str` names what we were reading.
    Truncated(&'static str),
    /// The page-size field is not a valid SQLite page size (a power of two in
    /// `512..=65536`; the on-disk value `1` denotes 65536).
    BadPageSize(u32),
    /// A page number that is zero (pages are 1-based) or points past the file.
    BadPageNumber(u32),
    /// A structurally-valid file that uses a feature this reader does not
    /// implement (named by the `&'static str`) — e.g. a text encoding other
    /// than UTF-8.
    Unsupported(&'static str),
    /// The bytes are internally inconsistent in a way a well-formed database
    /// never is — a b-tree page of an unexpected type, a cell pointer past the
    /// page, or a page-link cycle. The `&'static str` names what was wrong.
    Corrupt(&'static str),
}

impl fmt::Display for SqliteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SqliteError::BadMagic => write!(f, "not a SQLite database (bad magic header)"),
            SqliteError::Truncated(what) => write!(f, "file truncated while reading {what}"),
            SqliteError::BadPageSize(n) => write!(f, "invalid page size {n}"),
            SqliteError::BadPageNumber(n) => write!(f, "invalid page number {n}"),
            SqliteError::Unsupported(what) => write!(f, "unsupported SQLite feature: {what}"),
            SqliteError::Corrupt(what) => write!(f, "corrupt SQLite database: {what}"),
        }
    }
}

impl std::error::Error for SqliteError {}
