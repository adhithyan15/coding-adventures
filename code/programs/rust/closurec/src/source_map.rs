//! `source_map` — `--create_source_map` minimal v3 emission (CLOC11.42).
//!
//! # What CC does
//!
//! The upstream Java Closure Compiler's `--create_source_map=path`
//! flag writes a Source Map v3 JSON document at `path` after the
//! compile completes. The document maps positions in the
//! compiled output back to positions in the input files, so
//! browsers/debuggers can show the user the original source
//! when they hit a breakpoint or read a stack trace.
//!
//! # What we do today
//!
//! Real source-map generation needs the parser to track byte
//! positions through every transform — that bridge lands with
//! CLOC11.07+. Until then we emit a **minimal valid v3 source
//! map**: empty `sources` / `sourcesContent` / `names` arrays
//! and an empty `mappings` VLQ string. Build pipelines that
//! check for the file's existence (Bazel rules, `webpack`
//! `source-map-loader` shims) see what they expect; debuggers
//! that try to use the map for a position lookup get the
//! correct response of "no information available."
//!
//! Wire format:
//!
//! ```json
//! {
//!   "version": 3,
//!   "file": "<--js_output_file basename or empty>",
//!   "lineCount": 0,
//!   "sourceRoot": "",
//!   "sources": [],
//!   "sourcesContent": [],
//!   "names": [],
//!   "mappings": ""
//! }
//! ```
//!
//! When real generation lands, the public function signature
//! stays the same — only the inner content evolves.
//!
//! # Why hand-roll the JSON
//!
//! Same reason as `print_tree::format_token_dump_json`: the
//! structure is fixed (eight known keys), zero new dependencies,
//! and the byte-stable output keeps diff fixtures readable.

use std::path::Path;

/// Format the minimal v3 source map JSON for a given output file.
///
/// `output_path` is the resolved `--js_output_file` (when set);
/// its basename becomes the `file` key. When `--js_output_file`
/// is absent (stdout output), pass `None` and `file` is `""`.
///
/// Pure function: no I/O, fully deterministic given the input.
/// Output ends with a trailing newline so concatenation stays
/// well-formed.
pub fn format_minimal_v3(output_path: Option<&Path>) -> String {
    let file = match output_path {
        Some(p) => p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        None => String::new(),
    };

    // 8 keys × ~20 bytes baseline + the file basename.
    let mut out = String::with_capacity(256);
    out.push_str("{\n");
    out.push_str("  \"version\": 3,\n");
    out.push_str("  \"file\": \"");
    append_json_escaped(&mut out, &file);
    out.push_str("\",\n");
    out.push_str("  \"lineCount\": 0,\n");
    out.push_str("  \"sourceRoot\": \"\",\n");
    out.push_str("  \"sources\": [],\n");
    out.push_str("  \"sourcesContent\": [],\n");
    out.push_str("  \"names\": [],\n");
    out.push_str("  \"mappings\": \"\"\n");
    out.push_str("}\n");
    out
}

/// JSON-escape `s` per RFC 8259 §7 for embedding inside a string.
///
/// Identical policy to [`crate::print_tree::append_json_escaped`]
/// — duplicated here to keep `source_map` independent of the
/// `print_tree` module's internals. The two formatters could
/// share a helper if a third caller appears.
fn append_json_escaped(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn minimal_v3_with_no_output_path_has_empty_file_key() {
        let s = format_minimal_v3(None);
        assert!(s.contains("\"file\": \"\""));
    }

    #[test]
    fn minimal_v3_uses_basename_of_output_path() {
        // The Source Map v3 `file` field is the basename of the
        // compiled output, not the full path — keeps the map
        // portable across CDN paths.
        let p = PathBuf::from("build/dist/app.min.js");
        let s = format_minimal_v3(Some(&p));
        assert!(s.contains("\"file\": \"app.min.js\""), "got: {s}");
        assert!(!s.contains("build/"), "should not contain dir path: {s}");
    }

    #[test]
    fn minimal_v3_declares_version_three() {
        // The v3 marker is what distinguishes this from v2/v1;
        // tools dispatch on this field.
        let s = format_minimal_v3(None);
        assert!(s.contains("\"version\": 3"));
    }

    #[test]
    fn minimal_v3_has_all_eight_required_keys() {
        let s = format_minimal_v3(None);
        for key in &[
            "version",
            "file",
            "lineCount",
            "sourceRoot",
            "sources",
            "sourcesContent",
            "names",
            "mappings",
        ] {
            assert!(s.contains(&format!("\"{key}\"")), "missing key {key}: {s}");
        }
    }

    #[test]
    fn minimal_v3_empty_arrays_are_well_formed() {
        // Empty arrays as `[]`, not `null`. Some debuggers reject
        // `null` for these fields, and the spec requires arrays.
        let s = format_minimal_v3(None);
        assert!(s.contains("\"sources\": []"));
        assert!(s.contains("\"sourcesContent\": []"));
        assert!(s.contains("\"names\": []"));
    }

    #[test]
    fn minimal_v3_empty_mappings_string() {
        // The `mappings` field is a VLQ-encoded string. Empty
        // mappings = empty string. Some tools accept the field
        // being absent, but emitting "" is more robust and lets
        // a consumer that always reads it not have to branch.
        let s = format_minimal_v3(None);
        assert!(s.contains("\"mappings\": \"\""));
    }

    #[test]
    fn minimal_v3_ends_with_newline() {
        // Trailing newline so the file works with line-counting
        // tools (`wc -l`) and concatenation.
        let s = format_minimal_v3(None);
        assert!(s.ends_with("}\n"));
    }

    #[test]
    fn minimal_v3_escapes_quotes_and_backslashes_in_file_name() {
        // A file basename containing a `"` is a pathological
        // case but we should still emit valid JSON. RFC 8259
        // forbids unescaped `"` and `\` inside strings.
        let p = PathBuf::from("weird\"name.js");
        let s = format_minimal_v3(Some(&p));
        // The output should contain `weird\"name.js` (escaped)
        // not a raw quote.
        assert!(s.contains("weird\\\"name.js"), "got: {s}");
        // Balanced quotes: every `"` not preceded by `\` is part
        // of the JSON framing. Count must be even.
        let unescaped_quotes: usize = s
            .char_indices()
            .filter(|(i, c)| {
                *c == '"' && (*i == 0 || s.as_bytes()[*i - 1] != b'\\')
            })
            .count();
        assert!(unescaped_quotes.is_multiple_of(2), "balanced quotes: {s}");
    }

    #[test]
    fn minimal_v3_byte_stable_across_invocations() {
        // Same input → byte-identical output. Diff fixtures
        // depend on this.
        let p = PathBuf::from("a.js");
        let a = format_minimal_v3(Some(&p));
        let b = format_minimal_v3(Some(&p));
        assert_eq!(a, b);
    }
}
