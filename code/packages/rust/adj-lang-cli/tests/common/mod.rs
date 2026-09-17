//! Helpers shared by the adj-lang-cli end-to-end tests.
//!
//! Each file under `tests/` compiles as its own crate, so a test that needs
//! something here says `mod common;`. Keep this to what several tests share:
//! a helper no including crate calls is dead code, and CI builds with
//! `-D warnings`.
//!
//! EVERYTHING HERE FOLLOWS ADJ'S OWN LEXER (`adj-lang/src/_lexer_grammar.rs`),
//! token for token. Its word-shaped tokens, in the order it tries them:
//!
//! ```text
//! STRING   "([^"\\]|\\.)*"                                  may span lines
//! NUMBER   -?(?:\.[0-9]+|[0-9]+(?:\.[0-9]*)?)(?:[eE][+-]?[0-9]+)?
//! VAR      \$[A-Za-z_][A-Za-z0-9_]*
//! IDENT    [a-z_][a-z0-9_]*
//! skipped  [ \t\r\n]+   and   LINE_COMMENT %[^\n]*
//! ```
//!
//! There is no word boundary between tokens, and no token for an uppercase
//! letter or a bare `.` outside a VAR or a string. Earlier versions of these
//! helpers approximated that with string splitting, and each approximation
//! leaked: `cites` after a trailing comment, spaced with a tab, glued to `2`,
//! glued to `1.e5`. So this module does not split -- it scans.

/// The CODE on each line of an `.adj` text: comments removed, string contents
/// blanked (quotes kept), one entry per line -- the same lines `str::lines`
/// would give.
///
/// A comment can fill a line or follow code on it; the stdlib has both, over a
/// thousand trailing ones. A string may span lines, so the scan runs over the
/// whole text and carries "inside a string" across line breaks.
///
/// ```text
/// text                                        code, per line
/// -----------------------------------------   ---------------------------------
/// % this row cites nothing                     (empty)
///     trust consensus  % cites here                trust consensus
///     source "the author cites a study"            source ""
///     source "50% of \"cites\""                    source ""
///     locator "https://example.org/            locator "
///     page" cites "x"                          " cites ""
/// ```
///
/// A string still open at the end of the text runs to the end, which is where
/// the lexer would fail.
pub fn adj_code_lines(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut in_comment = false;
    // Whether the current line has ANY characters. A last line that is all
    // string content yields no code but is still a line, as `str::lines`
    // counts it; testing `cur.is_empty()` instead would drop it.
    let mut pending = false;
    for c in text.chars() {
        if c == '\n' {
            if cur.ends_with('\r') {
                cur.pop();
            }
            lines.push(std::mem::take(&mut cur));
            in_comment = false;
            escaped = false;
            pending = false;
            continue;
        }
        pending = true;
        if in_comment {
            continue;
        }
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
                cur.push('"');
            }
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                cur.push('"');
            }
            '%' => in_comment = true,
            _ => cur.push(c),
        }
    }
    if pending {
        if cur.ends_with('\r') {
            cur.pop();
        }
        lines.push(cur);
    }
    lines
}

/// The IDENT tokens the lexer would read from one line of CODE (a line from
/// [`adj_code_lines`], so no comments and blanked strings).
///
/// At each position the scan does what the lexer does: a whole NUMBER is
/// consumed first (sign, decimal point, exponent and all), a `$` VAR is
/// consumed and is not an identifier, and an IDENT is `[a-z_][a-z0-9_]*`.
/// Anything else is one character skipped.
///
/// ```text
/// code             identifiers
/// --------------   -----------------
/// cites "" x       cites, x
/// 2cites           cites
/// 1.e5cites        cites
/// 1ecites          ecites
/// 1e5e5cites       e5cites
/// 2_cites          _cites
/// 0x1cites         x1cites
/// $cites y         y
/// citesX           cites
/// ```
///
/// The last row is where this differs from the lexer, and only on the safe
/// side: an uppercase letter is a lex ERROR there, so a file containing
/// `citesX` is rejected outright; here the `cites` before it is still read.
pub fn code_idents(line: &str) -> Vec<&str> {
    let b = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let n = number_len(b, i);
        if n > 0 {
            i += n;
            continue;
        }
        let c = b[i];
        if c == b'$' && b.get(i + 1).is_some_and(|d| d.is_ascii_alphabetic() || *d == b'_') {
            i += 2;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            continue;
        }
        if c.is_ascii_lowercase() || c == b'_' {
            let start = i;
            while i < b.len() && (b[i].is_ascii_lowercase() || b[i].is_ascii_digit() || b[i] == b'_') {
                i += 1;
            }
            // Both ends sit on ASCII bytes, so both are char boundaries.
            out.push(&line[start..i]);
            continue;
        }
        i += 1;
    }
    out
}

/// Length of the NUMBER token starting at `i`, or 0 if none starts there.
fn number_len(b: &[u8], i: usize) -> usize {
    let digits = |mut j: usize| {
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        j
    };
    let mut j = i;
    if b.get(j) == Some(&b'-') {
        j += 1;
    }
    if b.get(j) == Some(&b'.') {
        let k = digits(j + 1);
        if k == j + 1 {
            return 0;
        }
        j = k;
    } else {
        let k = digits(j);
        if k == j {
            return 0;
        }
        j = k;
        if b.get(j) == Some(&b'.') {
            j = digits(j + 1);
        }
    }
    if matches!(b.get(j), Some(b'e' | b'E')) {
        let mut k = j + 1;
        if matches!(b.get(k), Some(b'+' | b'-')) {
            k += 1;
        }
        let m = digits(k);
        if m > k {
            j = m;
        }
    }
    j - i
}

/// True when `text` carries `word` as an IDENT token of CODE -- outside every
/// comment and string -- on any line.
///
/// The WHOLE text, always. Two narrower versions let a real corroboration
/// through. Slicing at `adj.find("table <name>")` could start inside a string
/// that quotes that text, and the scan then read every later string inside
/// out. Starting at the first CODE line whose first two identifiers are
/// `table <name>` fails too: the lexer ignores line breaks, so the real
/// declaration can be split across lines or follow another statement on its
/// line, and a lookalike line further down becomes the start. A security
/// review measured each -- 15 of 18 test files passed.
///
/// `cites` is NOT a reserved word in ADJ: an atom may be spelled `cites`, and
/// such an atom trips this check too. That errs on the safe side -- it can
/// only make a test stricter, never let a corroboration through.
pub fn has_code_word(text: &str, word: &str) -> bool {
    adj_code_lines(text).iter().any(|line| code_idents(line).contains(&word))
}
