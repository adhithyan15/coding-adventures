//! # A canonical JSON writer, written by hand
//!
//! `corpus.json` is a digest input: one CRLF, one reordered key, one unescaped
//! character is a different corpus.  VER04r section 1.4 therefore asks for the writer
//! of VER04i section 9.1, and this module is that writer.  It is about a hundred lines
//! because the rules are few, and it replaces a `serde` dependency whose defaults
//! (insertion-ordered maps, shortest-float printing, lenient escapes) are exactly what
//! a canonical form must not inherit.
//!
//! ## The rules (VER04i section 9.1, "Canonical writer profile")
//!
//! ```text
//!   object keys      sorted by Unicode code point (UTF-8 byte order for valid UTF-8)
//!   indent           two spaces per level
//!   separators       "," then LF between members; ": " after a key
//!   empty container  "[]" or "{}"
//!   line endings     LF only, and exactly one final LF
//!   integers         plain decimal
//!   strings          every char outside U+0020..U+007E is \uXXXX with LOWERCASE hex
//!                    (an astral char is a lowercase surrogate pair); `"` -> \" and
//!                    `\` -> \\ ; \b \f \n \r \t in short form; `/` is never escaped
//! ```
//!
//! A tiny example.  The value
//!
//! ```text
//!   {"b": [1, "x"], "a": {}}
//! ```
//!
//! renders as
//!
//! ```text
//!   {
//!     "a": {},
//!     "b": [
//!       1,
//!       "x"
//!     ]
//!   }
//! ```
//!
//! Python's `json.dumps(value, sort_keys=True, indent=2)` produces the same bytes (its
//! `ensure_ascii` escaping is the one above), which is how the oracle parity fixtures
//! of this crate are produced and compared.
//!
//! The only JSON numbers a v2 corpus holds are counts and sizes, so [`Json::Int`] is a
//! `u64`; every other integer is written as a decimal *string* (VER04r section 2.3).
//! There is deliberately no float, no `null` and no negative number.

use crate::error::{code, GenError};

/// A JSON value of the closed profile of a v2 corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Json {
    /// `true` / `false`.
    Bool(bool),
    /// A count or size (never a value; values are strings).
    Int(u64),
    /// A string (any Unicode; escaped on output).
    Str(String),
    /// An array.
    Arr(Vec<Json>),
    /// An object, as pairs in any order; keys are sorted on output and a duplicate key
    /// is a [`GenError`] with code `json`.
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// A string value.
    pub fn str(s: impl Into<String>) -> Json {
        Json::Str(s.into())
    }

    /// An object from `(key, value)` pairs.
    pub fn obj<K: Into<String>>(pairs: impl IntoIterator<Item = (K, Json)>) -> Json {
        Json::Obj(pairs.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    /// An array from values.
    pub fn arr(items: impl IntoIterator<Item = Json>) -> Json {
        Json::Arr(items.into_iter().collect())
    }

    /// An array of strings.
    pub fn strs<S: AsRef<str>>(items: impl IntoIterator<Item = S>) -> Json {
        Json::Arr(items.into_iter().map(|s| Json::str(s.as_ref())).collect())
    }

    /// Render in the canonical form: two-space indent, sorted keys, LF, one final LF.
    ///
    /// # Errors
    ///
    /// `json` if an object has a duplicate key.
    pub fn render(&self) -> Result<String, GenError> {
        let mut out = String::new();
        self.write(&mut out, 0)?;
        out.push('\n');
        Ok(out)
    }

    fn write(&self, out: &mut String, level: usize) -> Result<(), GenError> {
        match self {
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Int(n) => out.push_str(&n.to_string()),
            Json::Str(s) => write_string(out, s),
            Json::Arr(items) => {
                if items.is_empty() {
                    out.push_str("[]");
                    return Ok(());
                }
                out.push_str("[\n");
                for (i, item) in items.iter().enumerate() {
                    indent(out, level + 1);
                    item.write(out, level + 1)?;
                    out.push_str(if i + 1 == items.len() { "\n" } else { ",\n" });
                }
                indent(out, level);
                out.push(']');
            }
            Json::Obj(pairs) => {
                if pairs.is_empty() {
                    out.push_str("{}");
                    return Ok(());
                }
                // Sort by UTF-8 bytes: for valid UTF-8 that is code point order, which
                // is *not* UTF-16 code-unit order (VER04i 9.1 item 6).
                let mut sorted: Vec<&(String, Json)> = pairs.iter().collect();
                sorted.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
                if sorted.windows(2).any(|w| w[0].0 == w[1].0) {
                    return Err(GenError::new(
                        code::JSON,
                        "",
                        "an object has a duplicate key",
                    ));
                }
                out.push_str("{\n");
                for (i, (key, value)) in sorted.iter().enumerate() {
                    indent(out, level + 1);
                    write_string(out, key);
                    out.push_str(": ");
                    value.write(out, level + 1)?;
                    out.push_str(if i + 1 == sorted.len() { "\n" } else { ",\n" });
                }
                indent(out, level);
                out.push('}');
            }
        }
        Ok(())
    }
}

fn indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

/// Write `s` as a JSON string literal under the VER04i 9.1 escaping rules.
fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // Printable ASCII passes through (`/` included: it is never escaped).
            ' '..='~' => out.push(ch),
            // Everything else: lowercase \uXXXX, an astral char as a surrogate pair.
            _ => {
                let mut units = [0u16; 2];
                for unit in ch.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_documented_example_renders_exactly() {
        let v = Json::obj([
            ("b", Json::arr([Json::Int(1), Json::str("x")])),
            ("a", Json::Obj(vec![])),
        ]);
        assert_eq!(
            v.render().unwrap(),
            "{\n  \"a\": {},\n  \"b\": [\n    1,\n    \"x\"\n  ]\n}\n"
        );
    }

    #[test]
    fn scalars_and_empty_containers() {
        assert_eq!(Json::Bool(true).render().unwrap(), "true\n");
        assert_eq!(Json::Bool(false).render().unwrap(), "false\n");
        assert_eq!(Json::Int(0).render().unwrap(), "0\n");
        assert_eq!(
            Json::Int(u64::MAX).render().unwrap(),
            "18446744073709551615\n"
        );
        assert_eq!(Json::Arr(vec![]).render().unwrap(), "[]\n");
        assert_eq!(Json::Obj(vec![]).render().unwrap(), "{}\n");
    }

    #[test]
    fn string_escaping_follows_ver04i_9_1() {
        // quote, backslash, the five short escapes, a slash, DEL, a control char, a BMP
        // char and an astral char (surrogate pair, lowercase hex).
        let s = "q\"b\\s \u{8}\u{c}\n\r\t/ \u{7f}\u{1}\u{e9}\u{1F600}";
        assert_eq!(
            Json::str(s).render().unwrap(),
            "\"q\\\"b\\\\s \\b\\f\\n\\r\\t/ \\u007f\\u0001\\u00e9\\ud83d\\ude00\"\n"
        );
    }

    #[test]
    fn keys_sort_by_code_point_not_insertion_or_utf16() {
        // U+FF5E (BMP, high) sorts BEFORE U+1F600 (astral) by code point, whereas
        // UTF-16 code units would put the astral surrogate (0xD83D) first.
        let v = Json::obj([
            ("\u{1F600}", Json::Int(2)),
            ("\u{ff5e}", Json::Int(1)),
            ("B", Json::Int(0)),
            ("a", Json::Int(9)),
        ]);
        let out = v.render().unwrap();
        let positions: Vec<usize> = ["\"B\"", "\"a\"", "\"\\uff5e\"", "\"\\ud83d\\ude00\""]
            .iter()
            .map(|k| out.find(k).unwrap())
            .collect();
        assert!(positions.windows(2).all(|w| w[0] < w[1]), "{out}");
    }

    #[test]
    fn keys_are_escaped_like_strings() {
        let v = Json::obj([("k\"\n", Json::Bool(true))]);
        assert_eq!(v.render().unwrap(), "{\n  \"k\\\"\\n\": true\n}\n");
    }

    #[test]
    fn a_duplicate_key_is_refused_even_when_nested() {
        let dup = Json::Obj(vec![("a".into(), Json::Int(1)), ("a".into(), Json::Int(2))]);
        assert_eq!(dup.render().unwrap_err().code, code::JSON);
        let nested = Json::arr([dup]);
        assert_eq!(nested.render().unwrap_err().code, code::JSON);
    }

    #[test]
    fn nesting_indents_two_spaces_per_level_and_uses_lf_only() {
        let v = Json::arr([Json::arr([Json::str("x")]), Json::Arr(vec![])]);
        let out = v.render().unwrap();
        assert_eq!(out, "[\n  [\n    \"x\"\n  ],\n  []\n]\n");
        assert!(!out.contains('\r'));
        assert!(out.ends_with("]\n") && !out.ends_with("\n\n"));
    }

    #[test]
    fn helpers_build_what_they_say() {
        assert_eq!(
            Json::strs(["a", "b"]),
            Json::arr([Json::str("a"), Json::str("b")])
        );
        assert_eq!(Json::str(String::from("z")), Json::Str("z".into()));
    }
}
