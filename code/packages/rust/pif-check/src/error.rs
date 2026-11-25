//! The gate's error type: one variant per way a check can fail, so a test (and a person
//! reading the report) sees the *reason* and not only the refusal.
//!
//! Every variant has a stable [`CheckError::code`], the name printed in the machine-readable
//! report. Reader rejections are wrapped, never flattened: the reader's own
//! [`PifError`] says which rule of VER32 5.3 fired and the gate adds *which file* of the chain
//! it fired on.

use lean_kernel::pif::PifError;

/// Why the gate refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckError {
    // ---- resolver (VER32 E5, R-IMP-2) ----------------------------------------------------
    /// A digest was not `[sha256:]` and 64 lowercase hex digits.
    BadDigestSyntax(String),
    /// The store has no file for this digest.
    FileMissing(String),
    /// The store handed over bytes whose SHA-256 is not the digest asked for (R-IMP-2).
    DigestMismatch { expected: String, actual: String },
    /// The file is larger than the gate's size cap.
    FileTooLarge(String),
    /// The store entry is not a regular file (a symbolic link, a directory).
    NotRegularFile(String),
    /// Any other I/O failure, with the `std::io::ErrorKind` as text.
    Io { digest: String, kind: String },
    // ---- chain shape (VER32 E1 to E4) ----------------------------------------------------
    /// A chain, sign-off or allow-list file is malformed (line number, why).
    BadControlFile {
        what: &'static str,
        line: usize,
        why: &'static str,
    },
    /// The chain has no file at all.
    EmptyChain,
    /// More than 65 files (64 imports plus the file itself, VER32 3.7).
    ChainTooLong,
    /// The first file is not the PIF prelude (E1).
    PreludeNotFirst,
    /// A `claim` file appears before the last position (E1: the claim file is last).
    ClaimNotLast,
    /// A role appears out of order (prelude, adl, library, model, axioms, claim).
    RoleOrder { index: usize },
    /// A role that may appear once appears twice (prelude, model, axioms, claim).
    DuplicateRole { index: usize },
    /// E4: a base-chain file is not on the gate's allow-list.
    NotAllowListed { index: usize, digest: String },
    /// E1: the chain file's base disagrees with the sign-off's base chain.
    SignoffChainMismatch,
    /// Neither an allow-list nor a sign-off vets the base chain, and `--unsafe-no-allow` was not given.
    Unvetted,
    /// The chain has no claim file, and `--allow-no-claim` was not given.
    NoClaimFile,
    /// E6: the sign-off's recorded base environment digest is not the one computed.
    SignoffBaseEnvironmentMismatch { expected: String, actual: String },
    // ---- the reader --------------------------------------------------------------------
    /// The reader rejected file `index` of the chain (`role` names its position).
    Read {
        index: usize,
        role: &'static str,
        error: PifError,
    },
}

impl CheckError {
    /// The stable name printed in the report.
    pub fn code(&self) -> &'static str {
        match self {
            CheckError::BadDigestSyntax(_) => "BadDigestSyntax",
            CheckError::FileMissing(_) => "FileMissing",
            CheckError::DigestMismatch { .. } => "DigestMismatch",
            CheckError::FileTooLarge(_) => "FileTooLarge",
            CheckError::NotRegularFile(_) => "NotRegularFile",
            CheckError::Io { .. } => "Io",
            CheckError::BadControlFile { .. } => "BadControlFile",
            CheckError::EmptyChain => "EmptyChain",
            CheckError::ChainTooLong => "ChainTooLong",
            CheckError::PreludeNotFirst => "PreludeNotFirst",
            CheckError::ClaimNotLast => "ClaimNotLast",
            CheckError::RoleOrder { .. } => "RoleOrder",
            CheckError::DuplicateRole { .. } => "DuplicateRole",
            CheckError::NotAllowListed { .. } => "NotAllowListed",
            CheckError::SignoffChainMismatch => "SignoffChainMismatch",
            CheckError::Unvetted => "Unvetted",
            CheckError::NoClaimFile => "NoClaimFile",
            CheckError::SignoffBaseEnvironmentMismatch { .. } => "SignoffBaseEnvironmentMismatch",
            CheckError::Read { .. } => "ReadRejected",
        }
    }

    /// One human-readable line; the machine-readable part is [`CheckError::code`].
    pub fn detail(&self) -> String {
        match self {
            CheckError::BadDigestSyntax(s) => format!("not a sha256 digest: {s:?}"),
            CheckError::FileMissing(d) => format!("no file in the store for {d}"),
            CheckError::DigestMismatch { expected, actual } => {
                format!("asked for {expected}, the bytes hash to {actual}")
            }
            CheckError::FileTooLarge(d) => format!("file {d} exceeds the size cap"),
            CheckError::NotRegularFile(d) => format!("store entry {d} is not a regular file"),
            CheckError::Io { digest, kind } => format!("reading {digest}: {kind}"),
            CheckError::BadControlFile { what, line, why } => {
                format!("{what} file, line {line}: {why}")
            }
            CheckError::EmptyChain => "the chain lists no file".to_string(),
            CheckError::ChainTooLong => "the chain has more than 65 files".to_string(),
            CheckError::PreludeNotFirst => {
                "the first file of the chain is not a prelude".to_string()
            }
            CheckError::ClaimNotLast => {
                "a claim file is not the last file of the chain".to_string()
            }
            CheckError::RoleOrder { index } => format!("file {index} is out of role order"),
            CheckError::DuplicateRole { index } => format!("file {index} repeats a once-only role"),
            CheckError::NotAllowListed { index, digest } => {
                format!("base file {index} ({digest}) is not on the allow-list")
            }
            CheckError::Unvetted => {
                "the base chain is neither allow-listed nor signed (give --allow or --signoff, or --unsafe-no-allow)".to_string()
            }
            CheckError::NoClaimFile => "the chain has no claim file (give --allow-no-claim to check a base chain alone)".to_string(),
            CheckError::SignoffChainMismatch => {
                "the chain's base files differ from the sign-off's base chain".to_string()
            }
            CheckError::SignoffBaseEnvironmentMismatch { expected, actual } => {
                format!("the sign-off records base environment {expected}, computed {actual}")
            }
            CheckError::Read { index, role, error } => {
                format!("file {index} ({role}) rejected by the reader: {error:?}")
            }
        }
    }
}
