//! # Diagnostics — every refusal names what it refused and where.
//!
//! The engine's contract is that **no input, malformed or otherwise, may cause
//! a panic, an abort, an out-of-bounds index, a non-terminating loop, or a
//! silent truncation of the token stream.** Every bound and every malformed
//! construct produces one of these instead.
//!
//! "Silent truncation" is worth separating into two cases, because the rule
//! points opposite ways:
//!
//! - Truncating a **token stream** is forbidden. A program that silently
//!   compiles to less than it says is the worst possible failure mode.
//! - Truncating **diagnostic text** is required. A stringized megabyte-long
//!   token would otherwise render a megabyte-long message, and a cascade would
//!   emit one per token — unbounded diagnostics are themselves a well-worn
//!   denial-of-service vector, and when they echo file contents they are an
//!   incidental disclosure channel on a shared builder.

use crate::source_map::Position;

/// A preprocessing failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PpError {
    message: String,
    position: Option<Position>,
}

impl PpError {
    pub fn new(message: impl Into<String>) -> PpError {
        PpError { message: message.into(), position: None }
    }

    #[must_use]
    pub fn at(mut self, position: Position) -> PpError {
        self.position = Some(position);
        self
    }

    #[must_use]
    pub fn position(&self) -> Option<Position> {
        self.position
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Quote source text into a message: escaped, then truncated to `limit`
    /// bytes.
    ///
    /// Two separate hazards, both reachable from hostile input by definition.
    ///
    /// **Escaping.** The text is an include spelling or a token, so it can
    /// contain anything the program's author typed — including control
    /// characters. A raw `ESC [ 2 J` in a diagnostic clears the screen of
    /// whoever reads the build log, and richer sequences can rewrite earlier
    /// lines: terminal-escape injection into a shared builder's CI output. A
    /// security review found this reachable end-to-end through
    /// `compile_source`.
    ///
    /// An earlier version of this comment said "escaping here rather than at
    /// each call site means a new interpolation cannot forget." That is false,
    /// and three review rounds found it false in a different place each time —
    /// most importantly `MemoryFs`, which is not test-only. Centralising a
    /// helper centralises the *implementation*, not the *decision to call it*.
    /// The invariant is therefore asserted over OUTPUTS, by
    /// `no_diagnostic_leaks_a_raw_control_character_or_runs_unbounded` in
    /// `macrooct-iir-compiler`, which drives hostile input through the public
    /// entry point — a missed call site shows up there whether or not anyone
    /// remembered the site exists.
    ///
    /// **Truncation.** On a char boundary, because slicing a `String`
    /// mid-UTF-8 panics, and a diagnostic path that panics on hostile input
    /// defeats the whole no-panic contract. Note the order: escape first, then
    /// truncate, so the byte cap applies to what is actually printed rather
    /// than to a pre-expansion length.
    #[must_use]
    pub fn quote(text: &str, limit: u32) -> String {
        let escaped: String = text
            .chars()
            .flat_map(|c| {
                // Keep ordinary printable text and plain spaces as-is; escape
                // every control character, including newline and tab, since a
                // quoted fragment belongs on one line of a diagnostic.
                if c.is_control() {
                    c.escape_default().collect::<Vec<_>>()
                } else {
                    vec![c]
                }
            })
            .collect();

        let limit = limit as usize;
        if escaped.len() <= limit {
            return escaped;
        }
        let mut end = limit;
        while end > 0 && !escaped.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}… ({} bytes total)", &escaped[..end], escaped.len())
    }
}

impl std::fmt::Display for PpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.position {
            Some(p) => write!(f, "{}:{}: {}", p.line, p.column, self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for PpError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_map::FileId;

    #[test]
    fn a_positioned_error_shows_line_and_column() {
        let e = PpError::new("unterminated @if")
            .at(Position { file: FileId::new(0), line: 12, column: 3 });
        assert_eq!(e.to_string(), "12:3: unterminated @if");
    }

    #[test]
    fn quote_leaves_short_text_alone() {
        assert_eq!(PpError::quote("abc", 16), "abc");
    }

    #[test]
    fn quote_truncates_long_text_and_says_so() {
        let q = PpError::quote(&"x".repeat(100), 10);
        assert!(q.starts_with("xxxxxxxxxx"));
        assert!(q.contains("100 bytes total"));
    }

    #[test]
    fn quote_never_panics_on_a_multibyte_boundary() {
        // The regression this guards: naive `&text[..limit]` panics when the
        // limit lands inside a multi-byte character. The text here is supplied
        // by the program being preprocessed, so this is reachable from input.
        for limit in 0..16u32 {
            let _ = PpError::quote("héllo wörld ünicode", limit);
        }
        // And specifically: a limit landing mid-character backs off rather
        // than slicing.
        let q = PpError::quote("é", 1);
        assert!(q.contains("bytes total"));
    }
}
