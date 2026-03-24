//! String Scanner
//!
//! A cursor-based scanner over a string slice. Used by both the block parser
//! (to scan individual lines) and the inline parser (to scan inline content
//! character by character).
//!
//! # Design
//!
//! The scanner maintains a position `pos` (byte index) into the source string.
//! All read operations advance `pos`. The scanner never backtracks on its own —
//! callers must save and restore `pos` explicitly when lookahead fails.
//!
//! This is the same pattern used by hand-rolled recursive descent parsers
//! everywhere: try to match, if it fails, restore the saved position.
//!
//! ```text
//! let saved = scanner.pos;
//! if !scanner.match_str("```") {
//!     scanner.pos = saved; // backtrack
//! }
//! ```
//!
//! # Character classification
//!
//! CommonMark cares about several Unicode character categories:
//!   - ASCII punctuation: `!"#$%&'()*+,-./:;<=>?@[\]^_{|}~`
//!   - Unicode punctuation (for emphasis rules)
//!   - ASCII whitespace: space, tab, CR, LF, FF
//!   - Unicode whitespace

use unicode_general_category::{get_general_category, GeneralCategory};

// ─── Scanner ──────────────────────────────────────────────────────────────────

/// A cursor-based scanner over a string slice.
///
/// The scanner stores the source as a `String` so it owns the data, and uses
/// a `pos` byte index. Since we work with UTF-8, advancing requires care when
/// dealing with multi-byte characters.
pub struct Scanner {
    pub source: String,
    pub pos: usize,
}

impl Scanner {
    pub fn new(source: impl Into<String>) -> Self {
        Scanner { source: source.into(), pos: 0 }
    }

    pub fn new_at(source: impl Into<String>, start: usize) -> Self {
        Scanner { source: source.into(), pos: start }
    }

    /// True if the scanner has consumed all input.
    pub fn done(&self) -> bool {
        self.pos >= self.source.len()
    }

    /// Number of bytes remaining.
    pub fn remaining(&self) -> usize {
        self.source.len() - self.pos
    }

    /// Peek at the character at `pos + offset` (by character count, not bytes).
    /// Returns empty string if out of bounds.
    pub fn peek(&self, offset: usize) -> &str {
        let chars: Vec<char> = self.source[self.pos..].chars().collect();
        if offset >= chars.len() {
            return "";
        }
        // Return a slice of the char at offset
        let mut byte_pos = self.pos;
        for (i, ch) in self.source[self.pos..].char_indices() {
            if offset == 0 {
                let end = self.pos + ch.len_utf8();
                return &self.source[self.pos..end];
            }
            let _ = (i, ch);
            break;
        }
        // Count character offsets
        let mut count = 0;
        let mut start_byte = self.pos;
        for (i, ch) in self.source[self.pos..].char_indices() {
            if count == offset {
                start_byte = self.pos + i;
                let end_byte = start_byte + ch.len_utf8();
                return &self.source[start_byte..end_byte];
            }
            count += 1;
        }
        ""
    }

    /// Peek at a char at byte offset (fast path for ASCII-heavy paths).
    pub fn peek_char(&self, char_offset: usize) -> char {
        let s = self.peek(char_offset);
        s.chars().next().unwrap_or('\0')
    }

    /// Peek at `n` bytes starting at `pos` without advancing.
    pub fn peek_slice(&self, n: usize) -> &str {
        let end = (self.pos + n).min(self.source.len());
        &self.source[self.pos..end]
    }

    /// Advance `pos` by one character and return the consumed character.
    /// Returns `'\0'` if at end of input.
    pub fn advance(&mut self) -> char {
        if self.pos >= self.source.len() {
            return '\0';
        }
        let ch = self.source[self.pos..].chars().next().unwrap_or('\0');
        self.pos += ch.len_utf8();
        ch
    }

    /// Advance `pos` by `n` bytes. Clamps to source length.
    pub fn skip(&mut self, n: usize) {
        self.pos = (self.pos + n).min(self.source.len());
    }

    /// If the next bytes exactly match `s`, advance past them and return true.
    /// Otherwise leave `pos` unchanged and return false.
    pub fn match_str(&mut self, s: &str) -> bool {
        if self.source[self.pos..].starts_with(s) {
            self.pos += s.len();
            true
        } else {
            false
        }
    }

    /// Consume characters while the predicate returns true.
    /// Returns the consumed string slice.
    pub fn consume_while<F: Fn(char) -> bool>(&mut self, pred: F) -> &str {
        let start = self.pos;
        while self.pos < self.source.len() {
            let ch = self.source[self.pos..].chars().next().unwrap();
            if !pred(ch) {
                break;
            }
            self.pos += ch.len_utf8();
        }
        &self.source[start..self.pos]
    }

    /// Consume the rest of the line (up to but not including the newline).
    pub fn consume_line(&mut self) -> &str {
        let start = self.pos;
        while self.pos < self.source.len() && self.source.as_bytes()[self.pos] != b'\n' {
            self.pos += 1;
        }
        &self.source[start..self.pos]
    }

    /// Return the rest of the input from current pos without advancing.
    pub fn rest(&self) -> &str {
        &self.source[self.pos..]
    }

    /// Return a slice of source from `start` to current pos.
    pub fn slice_from(&self, start: usize) -> &str {
        &self.source[start..self.pos]
    }

    /// Skip ASCII spaces and tabs. Returns number of bytes skipped.
    pub fn skip_spaces(&mut self) -> usize {
        let start = self.pos;
        while self.pos < self.source.len() {
            let b = self.source.as_bytes()[self.pos];
            if b == b' ' || b == b'\t' {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.pos - start
    }

    /// Count leading spaces/tabs without advancing. Returns virtual column.
    pub fn count_indent(&self) -> usize {
        let mut indent = 0usize;
        let mut i = self.pos;
        while i < self.source.len() {
            match self.source.as_bytes()[i] {
                b' ' => { indent += 1; i += 1; }
                b'\t' => { indent += 4 - (indent % 4); i += 1; }
                _ => break,
            }
        }
        indent
    }

    /// Advance past exactly `n` virtual spaces of indentation (expanding tabs).
    pub fn skip_indent(&mut self, n: usize) {
        let mut remaining = n;
        while remaining > 0 && self.pos < self.source.len() {
            match self.source.as_bytes()[self.pos] {
                b' ' => { self.pos += 1; remaining -= 1; }
                b'\t' => {
                    let tab_width = 4 - (self.pos % 4);
                    if tab_width <= remaining {
                        self.pos += 1;
                        remaining -= tab_width;
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
    }

    /// Check if `pos + offset` (in bytes) has a specific byte.
    pub fn byte_at(&self, offset: usize) -> u8 {
        self.source.as_bytes().get(self.pos + offset).copied().unwrap_or(0)
    }

    /// Check if current position starts with a given byte.
    pub fn starts_with_byte(&self, b: u8) -> bool {
        self.source.as_bytes().get(self.pos).copied() == Some(b)
    }
}

// ─── Character Classification ─────────────────────────────────────────────────

/// ASCII punctuation characters as defined by CommonMark.
/// These are exactly: `!"#$%&'()*+,-./:;<=>?@[\]^_`{|}~`
pub fn is_ascii_punctuation(ch: char) -> bool {
    matches!(ch,
        '!' | '"' | '#' | '$' | '%' | '&' | '\'' | '(' | ')' | '*' |
        '+' | ',' | '-' | '.' | '/' | ':' | ';' | '<' | '=' | '>' |
        '?' | '@' | '[' | '\\' | ']' | '^' | '_' | '`' | '{' | '|' |
        '}' | '~'
    )
}

/// True if `ch` is a Unicode punctuation character for CommonMark flanking.
///
/// CommonMark defines this (per the cmark reference implementation) as any
/// ASCII punctuation character OR any character in Unicode categories:
///   Pc, Pd, Pe, Pf, Pi, Po, Ps (punctuation) or Sm, Sc, Sk, So (symbols).
///
/// The symbol categories (S*) are included because cmark treats them as
/// punctuation for delimiter flanking (e.g. £ U+00A3 Sc, € U+20AC Sc).
pub fn is_unicode_punctuation(ch: char) -> bool {
    if ch == '\0' {
        return false;
    }
    if is_ascii_punctuation(ch) {
        return true;
    }
    // Check Unicode general categories
    match get_general_category(ch) {
        GeneralCategory::ConnectorPunctuation |
        GeneralCategory::DashPunctuation |
        GeneralCategory::ClosePunctuation |
        GeneralCategory::FinalPunctuation |
        GeneralCategory::InitialPunctuation |
        GeneralCategory::OtherPunctuation |
        GeneralCategory::OpenPunctuation |
        GeneralCategory::MathSymbol |
        GeneralCategory::CurrencySymbol |
        GeneralCategory::ModifierSymbol |
        GeneralCategory::OtherSymbol => true,
        _ => false,
    }
}

/// True if `ch` is ASCII whitespace: space (U+0020), tab (U+0009),
/// newline (U+000A), form feed (U+000C), carriage return (U+000D).
pub fn is_ascii_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\r' | '\x0C')
}

/// True if `ch` is Unicode whitespace.
pub fn is_unicode_whitespace(ch: char) -> bool {
    if ch == '\0' {
        return false;
    }
    // Common ASCII whitespace
    if is_ascii_whitespace(ch) {
        return true;
    }
    // Additional Unicode whitespace codepoints matching the TypeScript implementation
    matches!(ch,
        '\u{00A0}' | '\u{1680}' |
        '\u{2000}'..='\u{200A}' |
        '\u{202F}' | '\u{205F}' | '\u{3000}'
    ) || ch == '\u{FEFF}'
}

/// True if `ch` is an ASCII digit (0-9).
pub fn is_digit(ch: char) -> bool {
    ch.is_ascii_digit()
}

/// Normalize a link label per CommonMark:
///   - Strip leading and trailing whitespace
///   - Collapse internal whitespace runs to a single space
///   - Fold to lowercase
///
/// Two labels are equivalent if their normalized forms are equal.
///
/// Note: JavaScript's `toLowerCase()` handles most cases but does not apply
/// the Unicode *full* case fold for ß (U+00DF) which folds to "ss". We replicate
/// that behavior here.
pub fn normalize_link_label(label: &str) -> String {
    // Trim, collapse internal whitespace, lowercase
    let trimmed = label.trim();
    let collapsed: String = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
    // Lowercase and replace ß → ss (Unicode full case folding)
    collapsed.to_lowercase().replace('ß', "ss")
}

/// Normalize a URL: percent-encode characters that should not appear
/// unencoded in HTML href/src attributes.
pub fn normalize_url(url: &str) -> String {
    // Encode characters that need percent-encoding in HTML attributes
    // but are not already encoded. Matches the TypeScript regex:
    // /[^\w\-._~:/?#@!$&'()*+,;=%]/g
    let mut result = String::with_capacity(url.len());
    for ch in url.chars() {
        if matches!(ch,
            'a'..='z' | 'A'..='Z' | '0'..='9' |
            '-' | '_' | '.' | '~' | ':' | '/' | '?' | '#' | '@' |
            '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' |
            ';' | '=' | '%'
        ) {
            result.push(ch);
        } else {
            // Percent-encode each byte of the UTF-8 encoding
            for byte in ch.to_string().as_bytes() {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_basic() {
        let mut s = Scanner::new("hello");
        assert!(!s.done());
        assert_eq!(s.advance(), 'h');
        assert_eq!(s.advance(), 'e');
        assert_eq!(s.pos, 2);
    }

    #[test]
    fn test_scanner_match_str() {
        let mut s = Scanner::new("hello world");
        assert!(s.match_str("hello"));
        assert!(!s.match_str("xyz"));
        assert!(s.match_str(" world"));
    }

    #[test]
    fn test_scanner_consume_while() {
        let mut s = Scanner::new("   abc");
        let spaces = s.consume_while(|c| c == ' ');
        assert_eq!(spaces, "   ");
        assert_eq!(s.pos, 3);
    }

    #[test]
    fn test_is_ascii_punctuation() {
        assert!(is_ascii_punctuation('!'));
        assert!(is_ascii_punctuation('*'));
        assert!(is_ascii_punctuation('_'));
        assert!(!is_ascii_punctuation('a'));
        assert!(!is_ascii_punctuation('1'));
    }

    #[test]
    fn test_is_unicode_whitespace() {
        assert!(is_unicode_whitespace(' '));
        assert!(is_unicode_whitespace('\t'));
        assert!(is_unicode_whitespace('\n'));
        assert!(is_unicode_whitespace('\u{00A0}'));
        assert!(!is_unicode_whitespace('a'));
    }

    #[test]
    fn test_normalize_link_label() {
        assert_eq!(normalize_link_label("Example"), "example");
        assert_eq!(normalize_link_label("  EXAMPLE  "), "example");
        assert_eq!(normalize_link_label("foo  bar"), "foo bar");
    }

    #[test]
    fn test_normalize_url() {
        // Spaces should be encoded
        assert_eq!(normalize_url("hello world"), "hello%20world");
        // Already valid chars pass through
        assert_eq!(normalize_url("https://example.com"), "https://example.com");
    }
}
