//! `charset` — `--charset` output-encoding normalization (CLOC11.16).
//!
//! # What CC does
//!
//! The upstream Java Closure Compiler's `--charset` flag controls
//! both input and output character encoding. The flag is
//! documented as defaulting to "UTF-8 in, US_ASCII out": CC
//! reads input files as UTF-8, but **escapes every non-ASCII
//! character in the output as `\uXXXX`** so the emitted JS is
//! pure 7-bit ASCII and safe to embed in any HTTP transport,
//! HTML attribute, or filesystem with iffy encoding behavior.
//!
//! Passing `--charset UTF-8` opts out of the output escaping —
//! the compiled JS is emitted as raw UTF-8 bytes.
//!
//! # What we do
//!
//! Pre-CLOC11.16 closurec ignored `--charset` for output and
//! always passed non-ASCII through verbatim. That diverged from
//! CC's documented default and surprised anyone who relied on
//! ASCII-only output (the typical case for tools that ingest
//! closurec output into HTML/JSON/email).
//!
//! Now:
//!
//! | `--charset` value | Output behavior                              |
//! |-------------------|----------------------------------------------|
//! | (unset)           | US_ASCII — escape non-ASCII (matches CC)     |
//! | `US_ASCII`        | same as unset                                |
//! | `US-ASCII`        | accepted as alias (CC accepts both forms)    |
//! | `UTF-8`           | pass-through (raw UTF-8 bytes)               |
//! | `UTF8`            | accepted as alias                            |
//! | anything else     | pass-through (CC ignores unknown values)     |
//!
//! # Escape format
//!
//! BMP codepoints (`U+0000..U+FFFF`) emit `\uXXXX` with 4 hex
//! digits. Astral codepoints (`U+10000..U+10FFFF`) emit a
//! surrogate pair (`\uXXXX\uXXXX`) — JS string literals require
//! either form to encode astral characters and `\u{XXXXX}` is
//! the ES2015+ alternative we deliberately avoid for maximum
//! compatibility with any consumer.
//!
//! # Pipeline position
//!
//! Charset normalization is the **last** transform before write.
//! It runs *after* the output wrapper and IIFE wrap, so any
//! non-ASCII the user injected via `--output_wrapper` (e.g. a
//! comment banner containing a copyright `©`) also gets
//! escaped. That matches CC: the charset is applied to the
//! final emitted file, not to any intermediate stage.

/// Resolved charset behavior. The string `--charset` argument is
/// projected into this typed enum so the run pipeline doesn't
/// have to re-parse user input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputCharset {
    /// US-ASCII output: every codepoint > 0x7F is `\uXXXX`-escaped.
    /// Default per CC.
    UsAscii,
    /// UTF-8 output: pass-through, no escaping.
    Utf8,
}

impl OutputCharset {
    /// Resolve a raw `--charset` value into the typed enum.
    ///
    /// Case-insensitive. Accepts hyphenated and unhyphenated
    /// forms (`US_ASCII`, `US-ASCII`, `UTF-8`, `UTF8`) since CC
    /// itself accepts both. Empty string → default (US-ASCII)
    /// per CC's documented default. Unknown values fall back to
    /// `Utf8` (pass-through) — CC silently ignores unknown
    /// charsets too rather than erroring.
    pub fn from_raw(raw: &str) -> Self {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            // CC's documented default.
            return Self::UsAscii;
        }
        // Normalize: lower-case, strip `_` and `-`.
        let normalized: String = trimmed
            .chars()
            .filter(|c| *c != '_' && *c != '-')
            .map(|c| c.to_ascii_lowercase())
            .collect();
        match normalized.as_str() {
            "usascii" | "ascii" => Self::UsAscii,
            "utf8" => Self::Utf8,
            // Anything else: pass-through. Matches CC's
            // permissive behavior on unknown charsets.
            _ => Self::Utf8,
        }
    }
}

/// Apply the charset transform to the final output text.
///
/// Fast path: `Utf8` returns `text` ownership unchanged via
/// `text.to_string()` — no escape scan, no allocation beyond the
/// caller's. `UsAscii` walks the string char-by-char and emits
/// `\uXXXX` (or a surrogate pair for astral codepoints) for any
/// codepoint > 0x7F.
///
/// # Why a single pass over chars
///
/// `text.is_ascii()` would let us skip the allocation for the
/// common pure-ASCII case. We don't bother — the existing
/// pipeline already does several `to_string()` copies, and a
/// single linear scan over the final output is cheap relative
/// to the rest of the pipeline.
pub fn apply_charset(text: &str, mode: OutputCharset) -> String {
    match mode {
        OutputCharset::Utf8 => text.to_string(),
        OutputCharset::UsAscii => escape_non_ascii(text),
    }
}

/// Walk `text` and emit `\uXXXX` for every codepoint > 0x7F.
///
/// BMP codepoints fit in one `\uXXXX` escape. Astral codepoints
/// (U+10000..U+10FFFF) emit a UTF-16 surrogate pair:
///
///   high = 0xD800 + ((cp - 0x10000) >> 10)
///   low  = 0xDC00 + ((cp - 0x10000) & 0x3FF)
///
/// We use surrogate pairs rather than the ES2015 `\u{XXXXX}`
/// form to keep the emitted JS compatible with the broadest set
/// of consumers (legacy minifiers, ES5-only environments).
fn escape_non_ascii(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        let cp = c as u32;
        if cp < 0x80 {
            // ASCII passes through verbatim.
            out.push(c);
        } else if cp <= 0xFFFF {
            // BMP: single `\uXXXX`.
            out.push_str(&format!("\\u{:04x}", cp));
        } else {
            // Astral: surrogate pair. JavaScript string literals
            // accept this and reassemble into the original
            // codepoint at runtime.
            let adjusted = cp - 0x10000;
            let high = 0xD800 + (adjusted >> 10);
            let low = 0xDC00 + (adjusted & 0x3FF);
            out.push_str(&format!("\\u{:04x}\\u{:04x}", high, low));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_charset_string_defaults_to_us_ascii() {
        // CC's documented default: US_ASCII out when --charset unset.
        assert_eq!(OutputCharset::from_raw(""), OutputCharset::UsAscii);
    }

    #[test]
    fn us_ascii_underscore_and_hyphen_forms_both_accepted() {
        assert_eq!(OutputCharset::from_raw("US_ASCII"), OutputCharset::UsAscii);
        assert_eq!(OutputCharset::from_raw("US-ASCII"), OutputCharset::UsAscii);
        assert_eq!(OutputCharset::from_raw("ASCII"), OutputCharset::UsAscii);
    }

    #[test]
    fn utf8_hyphen_and_unhyphenated_forms_both_accepted() {
        assert_eq!(OutputCharset::from_raw("UTF-8"), OutputCharset::Utf8);
        assert_eq!(OutputCharset::from_raw("UTF8"), OutputCharset::Utf8);
    }

    #[test]
    fn case_insensitive_resolution() {
        assert_eq!(OutputCharset::from_raw("utf-8"), OutputCharset::Utf8);
        assert_eq!(OutputCharset::from_raw("us_ascii"), OutputCharset::UsAscii);
        assert_eq!(OutputCharset::from_raw("uTf-8"), OutputCharset::Utf8);
    }

    #[test]
    fn unknown_charset_falls_back_to_utf8_passthrough() {
        // CC silently ignores unknown charsets — we match.
        assert_eq!(OutputCharset::from_raw("KOI-8"), OutputCharset::Utf8);
        assert_eq!(OutputCharset::from_raw("bogus"), OutputCharset::Utf8);
    }

    #[test]
    fn utf8_mode_passes_through_verbatim() {
        let input = "var é = '日本語'; // copyright ©";
        assert_eq!(apply_charset(input, OutputCharset::Utf8), input);
    }

    #[test]
    fn us_ascii_mode_passes_through_ascii_only_text() {
        let input = "var x = 1;\nalert(\"hello\");";
        // No non-ASCII chars → identity.
        assert_eq!(apply_charset(input, OutputCharset::UsAscii), input);
    }

    #[test]
    fn us_ascii_mode_escapes_bmp_codepoint_as_4hex() {
        // `é` is U+00E9.
        let out = apply_charset("var x = 'é';", OutputCharset::UsAscii);
        assert_eq!(out, "var x = '\\u00e9';");
    }

    #[test]
    fn us_ascii_mode_escapes_cjk_codepoints() {
        // 日 is U+65E5, 本 is U+672C, 語 is U+8A9E.
        let out = apply_charset("'日本語'", OutputCharset::UsAscii);
        assert_eq!(out, "'\\u65e5\\u672c\\u8a9e'");
    }

    #[test]
    fn us_ascii_mode_escapes_astral_codepoint_as_surrogate_pair() {
        // U+1F600 (grinning face emoji) sits in the astral plane;
        // emit a UTF-16 surrogate pair.
        // high = 0xD800 + ((0x1F600 - 0x10000) >> 10) = 0xD83D
        // low  = 0xDC00 + ((0x1F600 - 0x10000) & 0x3FF) = 0xDE00
        let out = apply_charset("😀", OutputCharset::UsAscii);
        assert_eq!(out, "\\ud83d\\ude00");
    }

    #[test]
    fn us_ascii_mode_preserves_existing_backslash_u_escapes_verbatim() {
        // Input that already has a JS `é` escape sequence
        // is treated as 7 plain ASCII chars (`\`, `u`, `0`,
        // `0`, `e`, `9`). All ASCII → no further escaping.
        // (The double-escape "`\\u00e9` → `\\u005cu00e9`" would
        // be wrong; we don't escape backslashes themselves.)
        let input = "var x = '\\u00e9';";
        assert_eq!(
            apply_charset(input, OutputCharset::UsAscii),
            input,
            "existing \\u escapes must pass through verbatim"
        );
    }

    #[test]
    fn us_ascii_escape_uses_lowercase_hex() {
        // Pin the hex case — lowercase matches CC's emitter.
        let out = apply_charset("ä", OutputCharset::UsAscii);
        assert_eq!(out, "\\u00e4");
        assert!(!out.contains("E4"), "hex should be lowercase");
    }

    #[test]
    fn ascii_only_with_us_ascii_mode_is_byte_identical() {
        // Pure-ASCII input + US_ASCII mode → output bytes
        // identical to input. The minimum-change invariant for
        // the common case.
        let input = "function(){return 1;}";
        assert_eq!(
            apply_charset(input, OutputCharset::UsAscii).as_bytes(),
            input.as_bytes()
        );
    }
}
