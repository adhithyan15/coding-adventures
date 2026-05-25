//! `wrapper` — `--output_wrapper` / `--output_wrapper_file` template
//! substitution.
//!
//! # What CC does
//!
//! The upstream Java Closure Compiler's `--output_wrapper` flag takes
//! a string template. The compiled JS is substituted for the
//! placeholder `%output%`, and `%n%` expands to a literal newline.
//! For example, with `--output_wrapper "(function(){%output%})();"`
//! and compiled output `var x=1;`, the user gets
//! `(function(){var x=1;})();` written to the output destination.
//!
//! The companion flag `--output_wrapper_file <path>` reads the
//! wrapper template from a file instead of an inline string. When
//! both are passed, `--output_wrapper_file` wins (CC's behavior — its
//! file flag is documented as "loads the specified file and passes
//! its contents to `--output_wrapper`," so the file effectively
//! replaces the inline string).
//!
//! # Where this runs in the pipeline
//!
//! Wrapping is the *last* transform before write. The order:
//!
//! 1. `transform_source` per-input (compilation-level + defines).
//! 2. Concatenate transformed inputs into `combined`.
//! 3. **Wrap `combined` into the template** ← this module.
//! 4. Write to `--js_output_file` or stdout.
//!
//! This ordering matches CC's pipeline and means the wrapper sees
//! the final compiled JS, not the original source.

use std::path::{Path, PathBuf};
use std::io;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Reasons wrapper application can fail. The only failure path
/// today is `fs::read_to_string` on `--output_wrapper_file`.
#[derive(Debug, Clone, PartialEq)]
pub enum WrapperError {
    /// Failed to read the file pointed to by `--output_wrapper_file`.
    WrapperFileReadError {
        path: PathBuf,
        kind: io::ErrorKind,
        message: String,
    },
}

impl std::fmt::Display for WrapperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WrapperError::WrapperFileReadError { path, message, .. } => {
                write!(
                    f,
                    "failed to read --output_wrapper_file {}: {message}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for WrapperError {}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Wrap `compiled` per `--output_wrapper` / `--output_wrapper_file`.
///
/// Returns `compiled` unchanged when no wrapper is configured. Otherwise
/// applies template substitution to the resolved wrapper string
/// (with `%output%` → `compiled`, `%n%` → newline) and returns the
/// result.
///
/// Resolution rule: if `wrapper_file` is `Some`, its contents win
/// over `inline_wrapper` (CC's behavior — the file replaces the
/// inline string).
pub fn apply_output_wrapper(
    compiled: &str,
    inline_wrapper: &str,
    wrapper_file: Option<&Path>,
) -> Result<String, WrapperError> {
    // Resolve the effective wrapper template.
    let template = match wrapper_file {
        Some(path) => {
            std::fs::read_to_string(path).map_err(|e| {
                WrapperError::WrapperFileReadError {
                    path: path.to_path_buf(),
                    kind: e.kind(),
                    message: e.to_string(),
                }
            })?
        }
        None => inline_wrapper.to_string(),
    };

    // Fast path: no wrapper → pass-through (avoid allocating a
    // new String for the common case).
    if template.is_empty() {
        return Ok(compiled.to_string());
    }

    Ok(substitute_template(&template, compiled))
}

// ---------------------------------------------------------------------------
// Template substitution
// ---------------------------------------------------------------------------

/// Apply the `%output%` / `%n%` substitutions to a template.
///
/// We do a single forward scan over `template`, copying characters
/// to the output and recognizing the two placeholders. Anything
/// that *looks* like a placeholder but isn't one of the known names
/// (e.g. `%foo%`) is emitted verbatim — CC's behavior is to leave
/// unrecognized `%...%` sequences alone.
///
/// Note that `%output%` is *not* guaranteed to appear in the
/// template. CC accepts wrappers that don't reference it (e.g.
/// `"(function(){})();"`) and just emits the wrapper verbatim. We
/// match that.
fn substitute_template(template: &str, compiled: &str) -> String {
    // Allocate roughly enough space — template length plus the
    // compiled JS appearing once. Reallocation is fine if there
    // are multiple `%output%` occurrences (rare; not optimised).
    let mut out = String::with_capacity(template.len() + compiled.len());
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            // Look for a placeholder. We accept ASCII alphanumeric +
            // underscore between the two `%`s. If the closing `%`
            // isn't found before end-of-string or whitespace, we
            // emit the leading `%` verbatim and move on.
            if let Some(end) = find_closing_percent(&template[i + 1..]) {
                // `end` is the offset within the rest of the
                // template of the closing `%`. The placeholder
                // name is `template[i+1 .. i+1+end]`.
                let name = &template[i + 1..i + 1 + end];
                match name {
                    "output" => {
                        out.push_str(compiled);
                        i += 1 + end + 1;
                        continue;
                    }
                    "n" => {
                        out.push('\n');
                        i += 1 + end + 1;
                        continue;
                    }
                    _ => {
                        // Unrecognized placeholder: emit verbatim,
                        // including the surrounding `%`s.
                        out.push_str(&template[i..i + 1 + end + 1]);
                        i += 1 + end + 1;
                        continue;
                    }
                }
            }
            // Lone `%` with no closing partner: emit verbatim.
            out.push('%');
            i += 1;
        } else {
            // Find the next `%` and copy everything before it in one
            // shot. This is the hot path; avoid char-by-char copies.
            let next_pct = template[i..].find('%');
            match next_pct {
                Some(offset) => {
                    out.push_str(&template[i..i + offset]);
                    i += offset;
                }
                None => {
                    out.push_str(&template[i..]);
                    break;
                }
            }
        }
    }
    out
}

/// Given the substring after an opening `%`, return the offset of
/// the closing `%`. The placeholder name (between the two `%`s)
/// must be ASCII alphanumeric or underscore. Returns `None` if no
/// closing `%` is found before end-of-string or before a character
/// that can't be in a placeholder name.
///
/// This is intentionally strict: a wrapper template like
/// `"50% off"` (where the `%` is just a percent sign, not a
/// placeholder) won't try to match the `%` against `off ` and
/// fail mid-word — we stop scanning the moment we see a
/// non-placeholder-name character.
fn find_closing_percent(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'%' {
            return Some(i);
        }
        if !(b.is_ascii_alphanumeric() || b == b'_') {
            // Hit a non-placeholder character → not a placeholder.
            return None;
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_wrapper_returns_compiled_verbatim() {
        let out = apply_output_wrapper("var x=1;", "", None).expect("ok");
        assert_eq!(out, "var x=1;");
    }

    #[test]
    fn output_placeholder_substitutes() {
        let out = apply_output_wrapper(
            "var x=1;",
            "(function(){%output%})();",
            None,
        )
        .expect("ok");
        assert_eq!(out, "(function(){var x=1;})();");
    }

    #[test]
    fn n_placeholder_expands_to_newline() {
        let out = apply_output_wrapper(
            "var x=1;",
            "// banner%n%%output%",
            None,
        )
        .expect("ok");
        assert_eq!(out, "// banner\nvar x=1;");
    }

    #[test]
    fn unrecognized_placeholder_passes_through_verbatim() {
        // CC leaves unknown `%name%` placeholders alone — they
        // appear in output as written.
        let out =
            apply_output_wrapper("body", "before %unknown% after %output%", None)
                .expect("ok");
        assert_eq!(out, "before %unknown% after body");
    }

    #[test]
    fn lone_percent_passes_through() {
        // `50% off` is just text — the `%` has no closing partner
        // before a non-placeholder-name character.
        let out = apply_output_wrapper(
            "body",
            "50% off %output%",
            None,
        )
        .expect("ok");
        assert_eq!(out, "50% off body");
    }

    #[test]
    fn wrapper_without_output_placeholder_drops_compiled_js() {
        // CC accepts this (the wrapper is emitted as-is). It's
        // unusual but not an error.
        let out = apply_output_wrapper(
            "var x=1;",
            "// just a banner",
            None,
        )
        .expect("ok");
        assert_eq!(out, "// just a banner");
    }

    #[test]
    fn multiple_output_placeholders_all_substitute() {
        let out = apply_output_wrapper(
            "X",
            "%output%-%output%",
            None,
        )
        .expect("ok");
        assert_eq!(out, "X-X");
    }

    #[test]
    fn wrapper_file_overrides_inline() {
        // Write a wrapper to a temp file; ensure its contents win.
        let dir = std::env::temp_dir().join(format!(
            "closurec-cloc11-30-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("wrap.js");
        std::fs::write(&file, "/* from-file */%output%/* end */").unwrap();

        let out = apply_output_wrapper(
            "var x=1;",
            "INLINE %output%",  // should be ignored
            Some(&file),
        )
        .expect("ok");
        assert_eq!(out, "/* from-file */var x=1;/* end */");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_wrapper_file_returns_typed_error() {
        let err = apply_output_wrapper(
            "body",
            "",
            Some(Path::new("/nonexistent/path/wrapper.txt")),
        )
        .expect_err("missing file must error");
        match err {
            WrapperError::WrapperFileReadError { kind, .. } => {
                assert_eq!(kind, io::ErrorKind::NotFound);
            }
        }
    }

    #[test]
    fn wrapper_with_only_n_works() {
        // Banner-style wrappers often include trailing newlines
        // via %n%. Make sure the substitution chains right.
        let out = apply_output_wrapper(
            "code",
            "%output%%n%",
            None,
        )
        .expect("ok");
        assert_eq!(out, "code\n");
    }

    #[test]
    fn empty_compiled_with_wrapper_still_wraps() {
        let out = apply_output_wrapper("", "header%output%footer", None)
            .expect("ok");
        assert_eq!(out, "headerfooter");
    }

    #[test]
    fn substitute_template_unicode_in_template() {
        // Non-ASCII content in the template is copied verbatim;
        // the placeholder scanner only looks at ASCII placeholder
        // names so it can't mis-read `é` as part of a name.
        let out = substitute_template("héllo %output% wörld", "JS");
        assert_eq!(out, "héllo JS wörld");
    }

    #[test]
    fn wrapper_error_display() {
        let e = WrapperError::WrapperFileReadError {
            path: PathBuf::from("/x/y"),
            kind: io::ErrorKind::PermissionDenied,
            message: "permission denied".into(),
        };
        let s = e.to_string();
        assert!(s.contains("/x/y"));
        assert!(s.contains("permission denied"));
        let _: &dyn std::error::Error = &e;
    }

    #[test]
    fn find_closing_percent_basic() {
        assert_eq!(find_closing_percent("output%rest"), Some(6));
        assert_eq!(find_closing_percent("n%"), Some(1));
        // No closing percent before end-of-string.
        assert_eq!(find_closing_percent("output"), None);
        // Non-name character stops the scan.
        assert_eq!(find_closing_percent("50 off%"), None);
        // Closing percent must come before any non-name char.
        assert_eq!(find_closing_percent("foo bar%"), None);
    }
}
