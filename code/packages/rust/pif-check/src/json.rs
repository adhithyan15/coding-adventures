//! A tiny JSON writer for the machine-readable report.
//!
//! The report is data the release gate consumes, so it must be unambiguous: strings are
//! escaped for `"`, `\`, every control character, and **every non-ASCII character as
//! `\uXXXX`** (surrogate pairs above U+FFFF), so the output is pure ASCII whatever a name or a
//! profile label contained. Object keys keep insertion order, so the report is a pure
//! function of its inputs (byte-identical on re-runs). There is no parser: the gate never
//! reads JSON.

use std::fmt::Write as _;

/// A JSON value.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Bool(bool),
    Num(u64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(&'static str, Json)>),
}

impl Json {
    /// A string value.
    pub fn s(v: impl Into<String>) -> Json {
        Json::Str(v.into())
    }

    /// Render on one line, no spaces.
    pub fn render(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) => {
                let _ = write!(out, "{n}");
            }
            Json::Str(s) => escape(s, out),
            Json::Arr(items) => {
                out.push('[');
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write(out);
                }
                out.push(']');
            }
            Json::Obj(fields) => {
                out.push('{');
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    escape(k, out);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }
}

fn escape(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            ' '..='~' => out.push(c),
            _ => {
                let mut units = [0u16; 2];
                for u in c.encode_utf16(&mut units) {
                    let _ = write!(out, "\\u{u:04x}");
                }
            }
        }
    }
    out.push('"');
}
