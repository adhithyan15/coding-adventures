//! Text Core
//!
//! Phase 1 implementation of `code/specs/backend-crate-catalog.md`'s
//! `text-core` Layer-1 crate: a pure-Rust, no-I/O, WASM-compatible library of
//! Excel / Lotus 1-2-3 / R string functions used by any spreadsheet or
//! text-processing frontend (VisiCalc, Multiplan, modern reconstruction,
//! Macsyma, Twig, etc).
//!
//! # Design notes
//!
//! ## Positions are 1-based
//!
//! Excel and VisiCalc both use 1-based string indexing: `MID("hello", 1, 3)`
//! is `"hel"`, not `"ell"`. Functions in this crate translate to Rust's
//! 0-based world at the boundary so the *public* API matches the spreadsheet
//! semantics that calling code expects.
//!
//! ## Unicode handling
//!
//! - `LEN`, `LEFT`, `RIGHT`, `MID` count **Unicode scalar values** (Rust
//!   `char`s). This matches modern Excel.
//! - `LENB` counts UTF-8 bytes.
//! - `UPPER` / `LOWER` use Rust's default case-folding (which is Unicode-aware
//!   and may produce multi-char outputs, e.g. `"ß".to_uppercase() == "SS"`).
//! - `PROPER` segments words using `char::is_alphabetic`; everything else is a
//!   word-break.
//!
//! ## NA propagation
//!
//! Each function exposes two variants where vector inputs matter:
//!
//! - `foo(s: &str, ...) -> Result<..., TextError>` — the scalar form.
//! - `foo_vec(x: &Character, ...) -> Character` — vector form that propagates
//!   NA element-wise. An NA input produces an NA output. Errors on a single
//!   element are rendered as NA in the vector form (the scalar form is the
//!   place to surface `#VALUE!`).
//!
//! ## No regex, no external crates
//!
//! `numeric-tower` and `r-vector` are the only dependencies. The SEARCH
//! wildcard matcher is hand-rolled (~30 lines, no captures).

pub mod case;
pub mod chars;
pub mod compare;
pub mod concat;
pub mod convert;
pub mod extract;
pub mod find;
pub mod length;
pub mod predicates;
pub mod repeat;
pub mod split;
pub mod substitute;
pub mod trim;

pub use numeric_tower::Number;
pub use r_vector::{Character, Vector};

/// Errors produced by `text-core` functions.
///
/// These map onto Excel's `#VALUE!`, `#NUM!`, and `#N/A` conditions. The
/// bridge layer that wraps this crate for a particular frontend is responsible
/// for translating `TextError` into the right cell error.
#[derive(Debug, Clone, PartialEq)]
pub enum TextError {
    /// A parameter was outside its legal domain (e.g. `LEFT(s, -1)`,
    /// `MID(s, 0, ...)`). Equivalent to Excel `#VALUE!`.
    BadParameter {
        name: &'static str,
        value: String,
    },
    /// A substring lookup that the caller requested to be strict (e.g.
    /// `FIND`) failed to find its needle. Equivalent to Excel `#VALUE!`.
    NotFound {
        function: &'static str,
        needle: String,
    },
    /// A string failed to parse as the requested numeric / typed value.
    /// Equivalent to Excel `#VALUE!`.
    ParseError {
        function: &'static str,
        input: String,
    },
    /// A format code was not recognised or was malformed.
    /// Equivalent to Excel `#VALUE!`.
    FormatError {
        function: &'static str,
        format: String,
    },
}

impl std::fmt::Display for TextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextError::BadParameter { name, value } => {
                write!(f, "bad parameter {name}={value}")
            }
            TextError::NotFound { function, needle } => {
                write!(f, "{function}: could not find {needle:?}")
            }
            TextError::ParseError { function, input } => {
                write!(f, "{function}: cannot parse {input:?}")
            }
            TextError::FormatError { function, format } => {
                write!(f, "{function}: bad format code {format:?}")
            }
        }
    }
}

impl std::error::Error for TextError {}

// ----------------------------------------------------------------------------
// Helpers shared across modules.
// ----------------------------------------------------------------------------

/// Iterate the elements of a `Character` vector as `Option<&str>`.
///
/// This is the canonical access pattern: NA is `None`, present strings are
/// `Some(&str)`. We do not expose this in `r-vector` itself because the public
/// vector contract there returns `Option<&Element>` (the outer option being
/// "in-range vs out-of-range"), so callers need a small adapter to flatten
/// that into the NA-aware view we want here.
pub(crate) fn iter_character(x: &Character) -> impl Iterator<Item = Option<&str>> + '_ {
    (0..x.len()).map(move |i| match x.get(i) {
        Some(Some(s)) => Some(s.as_str()),
        _ => None,
    })
}

