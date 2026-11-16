//! # Admitting a `.part` file
//!
//! The text of a generated document comes from files a contributor writes: the contract
//! package's `warrant/contract.part`, `warrant/obligations-<tier>.part` and the optional,
//! never-embedded `warrant/lemmas.part`, and an
//! implementation's `warrant/impl.part`.  Before one is copied into a document it is
//! **admitted**, and only an admitted [`Part`] can be assembled (the type is the proof).
//!
//! ```text
//!   bytes
//!     │ at most 1 MiB, lines at most 4,096 bytes           (VOS02 section 6.8)
//!     │ ASCII; the only control byte is LF (CR and tab are refused, never normalised:
//!     │   two byte strings must not share a digest by a line-ending trick, section 2.3)
//!     │ not empty; ends with LF
//!     ▼
//!   lexed with the marker token grammar (vos_document.tokens): every token must be a
//!   plain LINE or a NEWLINE.  A line that mentions `-- vos02:` ANYWHERE (an exact
//!   marker, a near miss, a marker inside a string literal or after code) lexes as a
//!   marker token and is refused.
//!     ▼
//!   Part
//! ```
//!
//! The marker rule is enforced with the same token grammar `check-doc` uses to read the
//! document, so the two can never disagree about what a marker looks like.

use lexer::grammar_lexer::GrammarLexer;

use crate::caps::{PART_MAX_BYTES, PART_MAX_LINE};
use crate::digest::sha256_hex;
use crate::grammar::{document_tokens, token_kind};

/// An admitted part: ASCII, LF line ends, no marker, within its caps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    text: String,
}

/// Why a part was not admitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartErrorKind {
    /// Over the byte cap.
    TooLarge,
    /// A line over the line cap.
    LineTooLong,
    /// Not valid UTF-8, or not ASCII.
    NotAscii,
    /// A control byte other than LF (CR and tab included).
    ControlByte,
    /// Zero bytes.
    Empty,
    /// The last byte is not LF.
    NoFinalLf,
    /// A line mentions a marker.
    Marker,
}

/// A part error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartError {
    /// What went wrong.
    pub kind: PartErrorKind,
    /// 1-based line (1 when no line is meaningful).
    pub line: u32,
    /// Human-readable reason.
    pub message: String,
}

impl PartError {
    /// True for a size cap, which is "no verdict", not "a rule failed".
    pub fn is_cap(&self) -> bool {
        matches!(
            self.kind,
            PartErrorKind::TooLarge | PartErrorKind::LineTooLong
        )
    }
}

impl std::fmt::Display for PartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for PartError {}

fn err<T>(kind: PartErrorKind, line: usize, message: impl Into<String>) -> Result<T, PartError> {
    Err(PartError {
        kind,
        line: u32::try_from(line).unwrap_or(u32::MAX),
        message: message.into(),
    })
}

/// The 1-based line of byte offset `at`.
fn line_of(bytes: &[u8], at: usize) -> usize {
    1 + bytes[..at.min(bytes.len())]
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
}

/// Check the byte-level admission rules that need no lexer: size, line length, ASCII,
/// control bytes.  Shared by parts and by whole documents.
pub(crate) fn admit_bytes(
    bytes: &[u8],
    max_bytes: usize,
    max_line: usize,
) -> Result<&str, PartError> {
    if bytes.len() > max_bytes {
        return err(
            PartErrorKind::TooLarge,
            1,
            format!("over the {max_bytes}-byte cap"),
        );
    }
    // A control byte is reported before an encoding problem: it is the more useful
    // message for a CR.
    if let Some(at) = bytes
        .iter()
        .position(|&b| (b < 0x20 && b != b'\n') || b == 0x7f)
    {
        return err(
            PartErrorKind::ControlByte,
            line_of(bytes, at),
            format!(
                "control byte 0x{:02x} (only LF is allowed; CR and tab are refused, never normalised)",
                bytes[at]
            ),
        );
    }
    if let Some(at) = bytes.iter().position(|&b| b >= 0x80) {
        return err(
            PartErrorKind::NotAscii,
            line_of(bytes, at),
            "a byte outside ASCII",
        );
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        // Unreachable for ASCII bytes, but total: refuse rather than assume.
        return err(PartErrorKind::NotAscii, 1, "not UTF-8");
    };
    for (n, line) in text.split('\n').enumerate() {
        if line.len() > max_line {
            return err(
                PartErrorKind::LineTooLong,
                n + 1,
                format!("a line is over {max_line} bytes"),
            );
        }
    }
    Ok(text)
}

/// Admit a part.
pub fn admit_part(bytes: &[u8]) -> Result<Part, PartError> {
    let text = admit_bytes(bytes, PART_MAX_BYTES, PART_MAX_LINE)?;
    if text.is_empty() {
        return err(PartErrorKind::Empty, 1, "an empty part");
    }
    if !text.ends_with('\n') {
        return err(
            PartErrorKind::NoFinalLf,
            text.split('\n').count(),
            "the last line has no LF",
        );
    }
    // The marker token grammar: anything but a plain line is a marker.
    let tokens = match GrammarLexer::new(text, document_tokens()).tokenize() {
        Ok(t) => t,
        // The token grammar covers every printable ASCII line, so this cannot happen
        // after the checks above; if it ever does, refuse.
        Err(e) => return err(PartErrorKind::Marker, e.line, e.message),
    };
    for t in &tokens {
        match token_kind(t).as_str() {
            "LINE" | "NEWLINE" | "EOF" => {}
            _ => {
                return err(
                    PartErrorKind::Marker,
                    t.line,
                    "the line contains `-- vos02:`: markers are written only by gen-doc",
                )
            }
        }
    }
    Ok(Part {
        text: text.to_string(),
    })
}

impl Part {
    /// The text: LF-terminated lines, exactly the bytes that are copied and hashed.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// SHA-256 of the text (the digest a marker carries).
    pub fn digest(&self) -> String {
        sha256_hex(self.text.as_bytes())
    }
}
