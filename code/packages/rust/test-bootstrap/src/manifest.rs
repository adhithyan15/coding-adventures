//! # Evidence manifests (VER04h §8)
//!
//! An assurance report may only quote numbers that a run wrote down. This
//! module is the writing half: a tiny, dependency-free JSON value type and a
//! writer, shared by every harness that produces a manifest (the Warrant
//! generated test target, the proof driver, the microbenchmark target).
//!
//! Properties that matter for evidence:
//!
//! * **Deterministic bytes.** Object keys are sorted (`BTreeMap`), numbers are
//!   unsigned integers, there are no timestamps. Running the same suite twice
//!   writes the same file, so a checked-in manifest can be regenerated and
//!   diffed, and a hand edit shows up as drift.
//! * **No floating point.** Counts and seeds are integers; a seed that does not
//!   fit in a JSON-safe integer is written as a decimal *string* by the caller.
//! * **Safe strings.** Every control character is escaped, so a message can
//!   never break the document structure.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

/// A JSON value restricted to what manifests need.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Json {
    Bool(bool),
    Num(u64),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

impl Json {
    /// Build an object from `(key, value)` pairs.
    pub fn obj<const N: usize>(pairs: [(&str, Json); N]) -> Json {
        Json::Obj(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    /// A string value.
    pub fn str(text: impl Into<String>) -> Json {
        Json::Str(text.into())
    }

    /// Render with two-space indentation and a trailing newline.
    pub fn render(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out.push('\n');
        out
    }

    fn write(&self, out: &mut String, depth: usize) {
        match self {
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) => out.push_str(&n.to_string()),
            Json::Str(s) => quote(s, out),
            Json::Arr(items) if items.is_empty() => out.push_str("[]"),
            Json::Arr(items) => {
                out.push_str("[\n");
                for (i, item) in items.iter().enumerate() {
                    indent(out, depth + 1);
                    item.write(out, depth + 1);
                    if i + 1 < items.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                indent(out, depth);
                out.push(']');
            }
            Json::Obj(map) if map.is_empty() => out.push_str("{}"),
            Json::Obj(map) => {
                out.push_str("{\n");
                for (i, (key, value)) in map.iter().enumerate() {
                    indent(out, depth + 1);
                    quote(key, out);
                    out.push_str(": ");
                    value.write(out, depth + 1);
                    if i + 1 < map.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                indent(out, depth);
                out.push('}');
            }
        }
    }
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn quote(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// The path named by environment variable `var`, if set and non-empty.
///
/// Harnesses write a manifest only when asked to, so an ordinary
/// `cargo test` never modifies tracked files.
pub fn output_path(var: &str) -> Option<PathBuf> {
    match std::env::var_os(var) {
        Some(value) if !value.is_empty() => Some(PathBuf::from(value)),
        _ => None,
    }
}

/// Write `doc` to `path`, creating parent directories.
pub fn write(path: &Path, doc: &Json) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, doc.render())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_is_sorted_escaped_and_deterministic() {
        let doc = Json::obj([
            ("b", Json::Num(2)),
            ("a", Json::str("x\"y\n\u{1}")),
            ("c", Json::Arr(vec![Json::Bool(true), Json::Arr(vec![])])),
            ("d", Json::Obj(BTreeMap::new())),
        ]);
        let text = doc.render();
        assert_eq!(
            text,
            "{\n  \"a\": \"x\\\"y\\n\\u0001\",\n  \"b\": 2,\n  \"c\": [\n    true,\n    []\n  ],\n  \"d\": {}\n}\n"
        );
        assert_eq!(text, doc.render());
    }

    #[test]
    fn output_path_requires_a_non_empty_value() {
        assert_eq!(output_path("TB_MANIFEST_TEST_UNSET_VAR"), None);
        std::env::set_var("TB_MANIFEST_TEST_EMPTY_VAR", "");
        assert_eq!(output_path("TB_MANIFEST_TEST_EMPTY_VAR"), None);
        std::env::set_var("TB_MANIFEST_TEST_SET_VAR", "x/y.json");
        assert_eq!(
            output_path("TB_MANIFEST_TEST_SET_VAR"),
            Some(PathBuf::from("x/y.json"))
        );
    }

    #[test]
    fn write_creates_parent_directories() {
        let dir = std::env::temp_dir().join(format!("tb-manifest-{}", std::process::id()));
        let path = dir.join("nested/m.json");
        write(&path, &Json::obj([("k", Json::Num(1))])).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{\n  \"k\": 1\n}\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
