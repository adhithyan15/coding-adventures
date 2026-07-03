//! `help_markdown` — `--help_markdown` markdown flag dump.
//!
//! # What CC does
//!
//! The upstream Java Closure Compiler's `--help_markdown` flag
//! prints the full flag surface in markdown format and exits.
//! It's intended for documentation tooling — pipe it into
//! `cat > docs/closurec-flags.md` and you have a generated
//! reference page that stays in sync with whatever the binary
//! actually accepts.
//!
//! # What we do
//!
//! We iterate the loaded [`cli_builder::types::CliSpec`] — the
//! same in-memory representation cli-builder parsed from
//! `cli.spec.json` — and emit a markdown document with one
//! heading per flag.
//!
//! Wire format:
//!
//! ```markdown
//! # closurec
//!
//! <one-line description from spec>
//!
//! Version: 0.13.0
//!
//! ## Flags
//!
//! ### `--<long>` (<type>, default: <default>)
//!
//! <description>
//!
//! ### `--<long>` ...
//! ```
//!
//! Why a simple heading-per-flag format rather than a table:
//!
//! - Diff-friendly. A pinned fixture stays readable when a flag
//!   is added or its description changes; tables make the diff
//!   noisy because cells reflow.
//! - GitHub renders heading anchors. Users get linkable section
//!   IDs like `#--checks_only` for free.
//! - No alignment math. We never have to decide what column
//!   width fits the longest flag name.

use cli_builder::types::{CliSpec, FlagDef};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Format the spec's flag surface as a markdown document.
///
/// Output ends with a newline so concatenating fixtures stays
/// well-formed. Pure-function: no I/O, no global state, fully
/// deterministic given `spec`.
pub fn format_help_markdown(spec: &CliSpec) -> String {
    let mut out = String::with_capacity(8192);

    // ---------------------- title + description ----------------------
    out.push_str("# ");
    out.push_str(&spec.name);
    out.push_str("\n\n");
    out.push_str(&spec.description);
    out.push_str("\n\n");

    if let Some(v) = &spec.version {
        out.push_str("Version: ");
        out.push_str(v);
        out.push_str("\n\n");
    }

    // ---------------------- flags --------------------------------------
    out.push_str("## Flags\n\n");

    // Two flag lists in the spec: global + root. CC's
    // CommandLineRunner has only one list, so we emit both in
    // source order — globals first (they apply everywhere),
    // root flags second. Both lists are already in author order
    // from cli.spec.json; we preserve it so a hand-edited spec
    // produces predictable docs.
    for f in &spec.global_flags {
        append_flag_section(&mut out, f);
    }
    for f in &spec.flags {
        append_flag_section(&mut out, f);
    }

    out
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Emit one `### `--long` (type, default: X)\n\n<desc>\n\n` section.
fn append_flag_section(out: &mut String, f: &FlagDef) {
    // Heading — prefer long form (always present for closurec
    // flags), fall back to short or single-dash-long if a
    // hand-edited spec ever lacks the long form.
    out.push_str("### `");
    if let Some(long) = &f.long {
        out.push_str("--");
        out.push_str(long);
    } else if let Some(short) = &f.short {
        out.push('-');
        out.push_str(short);
    } else if let Some(sdl) = &f.single_dash_long {
        out.push('-');
        out.push_str(sdl);
    } else {
        // Defensive — every well-formed spec must have at least
        // one form; if it didn't, the binary wouldn't have
        // accepted the flag at all.
        out.push_str("(unnamed)");
    }
    out.push('`');

    // Type + default annotation in the heading so a quick scan
    // doesn't need the body.
    out.push_str(" (");
    out.push_str(&f.flag_type);
    if let Some(d) = &f.default {
        out.push_str(", default: ");
        out.push_str(&format_default(d));
    }
    out.push_str(")\n\n");

    // Body — the human-readable description verbatim, then a
    // blank line. We don't try to escape `*` or `_` because the
    // upstream descriptions don't contain markdown emphasis
    // markers — they're plain English sentences.
    out.push_str(&f.description);
    out.push_str("\n\n");
}

/// Render a JSON `Value` default as a short literal.
///
/// We intentionally don't try to pretty-print arrays/objects —
/// they don't appear as defaults in cli.spec.json. The match
/// covers what cli.spec.json actually uses: bool, integer, float,
/// string.
fn format_default(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            // Empty strings show up a lot (e.g. unset path
            // defaults). Display them as `""` so the reader can
            // tell "unset" from "the literal string 'unset'".
            if s.is_empty() {
                "\"\"".to_string()
            } else {
                s.clone()
            }
        }
        serde_json::Value::Null => "null".to_string(),
        // Anything else: fall back to serde's debug form. Should
        // never trigger for closurec's spec, but graceful is
        // better than panicking.
        other => other.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use cli_builder::load_spec_from_str;

    const MINI_SPEC: &str = r#"{
      "cli_builder_spec_version": "1.0",
      "name": "demo",
      "description": "Demo CLI for testing.",
      "version": "9.9.9",
      "parsing_mode": "gnu",
      "flags": [
        { "id": "verbose", "long": "verbose", "short": "v", "type": "boolean", "default": false, "description": "Be loud." },
        { "id": "out", "long": "out", "type": "string", "default": "out.txt", "description": "Output path." },
        { "id": "level", "long": "level", "type": "integer", "default": 1, "description": "Numeric level." }
      ]
    }"#;

    #[test]
    fn emits_title_and_description() {
        let spec = load_spec_from_str(MINI_SPEC).unwrap();
        let md = format_help_markdown(&spec);
        assert!(md.starts_with("# demo\n"));
        assert!(md.contains("Demo CLI for testing."));
    }

    #[test]
    fn emits_version_when_present() {
        let spec = load_spec_from_str(MINI_SPEC).unwrap();
        let md = format_help_markdown(&spec);
        assert!(md.contains("Version: 9.9.9"));
    }

    #[test]
    fn emits_one_section_per_flag() {
        let spec = load_spec_from_str(MINI_SPEC).unwrap();
        let md = format_help_markdown(&spec);
        // Three flags → three `### ` headings.
        assert_eq!(md.matches("### `").count(), 3);
        assert!(md.contains("### `--verbose`"));
        assert!(md.contains("### `--out`"));
        assert!(md.contains("### `--level`"));
    }

    #[test]
    fn heading_includes_type_and_default() {
        let spec = load_spec_from_str(MINI_SPEC).unwrap();
        let md = format_help_markdown(&spec);
        assert!(md.contains("(boolean, default: false)"));
        assert!(md.contains("(string, default: out.txt)"));
        assert!(md.contains("(integer, default: 1)"));
    }

    #[test]
    fn body_carries_description() {
        let spec = load_spec_from_str(MINI_SPEC).unwrap();
        let md = format_help_markdown(&spec);
        assert!(md.contains("Be loud."));
        assert!(md.contains("Output path."));
        assert!(md.contains("Numeric level."));
    }

    #[test]
    fn empty_string_default_is_quoted_to_disambiguate_from_unset() {
        let spec_json = r#"{
          "cli_builder_spec_version": "1.0",
          "name": "demo",
          "description": "x",
          "flags": [
            { "id": "p", "long": "p", "type": "string", "default": "", "description": "Path." }
          ]
        }"#;
        let spec = load_spec_from_str(spec_json).unwrap();
        let md = format_help_markdown(&spec);
        assert!(md.contains("default: \"\""), "got: {md}");
    }

    #[test]
    fn flag_without_default_omits_default_clause() {
        let spec_json = r#"{
          "cli_builder_spec_version": "1.0",
          "name": "demo",
          "description": "x",
          "flags": [
            { "id": "n", "long": "n", "type": "integer", "description": "No default." }
          ]
        }"#;
        let spec = load_spec_from_str(spec_json).unwrap();
        let md = format_help_markdown(&spec);
        assert!(md.contains("(integer)"), "got: {md}");
        assert!(!md.contains("default:"));
    }
}
