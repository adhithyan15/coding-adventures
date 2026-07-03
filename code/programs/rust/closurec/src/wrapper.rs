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

/// Reasons wrapper application can fail.
#[derive(Debug, Clone, PartialEq)]
pub enum WrapperError {
    /// Failed to read the file pointed to by `--output_wrapper_file`.
    WrapperFileReadError {
        path: PathBuf,
        kind: io::ErrorKind,
        message: String,
    },
    /// `--output_wrapper` / `--output_wrapper_file` was set but the
    /// resolved template contains no `%output%` placeholder. CC
    /// emits the exact message "ERROR - No %output% placeholder in
    /// the output wrapper" in this case (see
    /// `AbstractCommandLineRunner.java`); we mirror that error.
    ///
    /// Why CC enforces this: a wrapper without `%output%` would
    /// produce output that doesn't contain the compiled JS at
    /// all. That's almost certainly a user mistake (they meant to
    /// type the placeholder but didn't), and emitting JS-less
    /// output silently would be a hard-to-debug surprise. Better
    /// to fail loudly.
    MissingOutputPlaceholder,
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
            WrapperError::MissingOutputPlaceholder => {
                // Match CC's exact wording so toolchains that
                // grep for this error string keep working when
                // they swap closure-compiler.jar for closurec.
                write!(f, "ERROR - No %output% placeholder in the output wrapper")
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

    // CLOC11.32: validate that the resolved template contains the
    // `%output%` placeholder. CC's
    // `AbstractCommandLineRunner.checkFlags` performs this check
    // and emits the exact error string we mirror in
    // `WrapperError::MissingOutputPlaceholder`'s Display. Without
    // it, a typo'd wrapper (e.g. `"(function(){%otput%})()"`)
    // would silently produce output with no compiled JS in it,
    // and the user would chase a confusing empty-bundle bug.
    //
    // The check is a literal substring match. Closure's own check
    // is also a substring match (it's a `String.contains` on the
    // Java side), and tools that rely on `%output%` appearing
    // verbatim in the user-facing flag value can grep for it the
    // same way.
    if !template.contains("%output%") {
        return Err(WrapperError::MissingOutputPlaceholder);
    }

    Ok(substitute_template(&template, compiled))
}

// ---------------------------------------------------------------------------
// `--isolation_mode IIFE` (CLOC11.31)
// ---------------------------------------------------------------------------

/// Wrap `compiled` in an immediately-invoked function expression
/// (IIFE) per `--isolation_mode IIFE`.
///
/// The exact wrapper string we emit matches CC's `CompilerOptions`
/// IIFE wrapping: `(function(){<body>}).call(this);`. Using
/// `.call(this)` rather than the simpler `()` form preserves the
/// outer `this` binding inside the IIFE, which is what callers
/// rely on for browser globals vs strict mode and what CC has
/// emitted since the option was introduced.
///
/// # Layering with `--output_wrapper`
///
/// IIFE wrapping runs *after* the user-supplied
/// `--output_wrapper` (CC's pipeline order). So with both
/// flags:
///
/// - `--output_wrapper '// banner%n%%output%'`
/// - `--isolation_mode IIFE`
///
/// the output is:
///
/// ```text
/// (function(){// banner
/// <compiled>}).call(this);
/// ```
///
/// The banner sits *inside* the IIFE, not outside. Users who
/// want the banner outside should not pair these two flags;
/// they'd put the IIFE in their own `--output_wrapper`.
pub fn apply_iife_wrap(compiled: &str) -> String {
    let mut out = String::with_capacity(compiled.len() + 24);
    out.push_str("(function(){");
    out.push_str(compiled);
    out.push_str("}).call(this);");
    out
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
    fn wrapper_without_output_placeholder_errors_per_cc() {
        // CLOC11.32: superseded earlier "CC accepts this" behavior.
        // A non-empty wrapper without `%output%` would silently
        // drop the compiled JS, almost certainly a user typo.
        // CC's `AbstractCommandLineRunner.checkFlags` raises a
        // hard error; we mirror that.
        let err = apply_output_wrapper(
            "var x=1;",
            "// just a banner",
            None,
        )
        .expect_err("must error per CC compat");
        assert_eq!(err, WrapperError::MissingOutputPlaceholder);
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
            other => panic!("expected WrapperFileReadError, got {other:?}"),
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

    // -- CLOC11.31 IIFE wrap tests -----------------------------------------

    #[test]
    fn iife_wrap_basic() {
        assert_eq!(
            apply_iife_wrap("var x=1;"),
            "(function(){var x=1;}).call(this);",
        );
    }

    #[test]
    fn iife_wrap_empty_body() {
        // CC happily emits the IIFE wrapper around empty content.
        assert_eq!(apply_iife_wrap(""), "(function(){}).call(this);");
    }

    #[test]
    fn iife_wrap_preserves_content_verbatim() {
        // No escaping happens — the compiled JS is inserted as-is.
        let body = "var s = \"hello\"; if (true) { x++; }";
        let wrapped = apply_iife_wrap(body);
        assert!(wrapped.contains(body));
        assert!(wrapped.starts_with("(function(){"));
        assert!(wrapped.ends_with("}).call(this);"));
    }

    #[test]
    fn iife_wrap_uses_call_this_not_bare_invocation() {
        // CC emits `.call(this)`, not bare `()`. This matters for
        // preserving outer `this` binding. Pin the form so a
        // future refactor that "simplifies" the wrapper to `()`
        // breaks loudly.
        let out = apply_iife_wrap("x;");
        assert!(out.contains(".call(this)"), "got: {out}");
        assert!(!out.contains("}();"), "should NOT use bare invocation: {out}");
    }

    // -- CLOC11.32 missing-%output% validation tests ---------------------

    #[test]
    fn wrapper_missing_output_placeholder_errors() {
        // A non-empty wrapper without `%output%` is a CC error.
        let err = apply_output_wrapper("body", "(function(){})();", None)
            .expect_err("must error");
        assert_eq!(err, WrapperError::MissingOutputPlaceholder);
    }

    #[test]
    fn wrapper_missing_output_placeholder_uses_cc_message() {
        // Pin the exact wording: tools that grep CC's stderr for
        // this string keep working when they swap in closurec.
        let err = apply_output_wrapper("body", "no placeholder here", None)
            .expect_err("must error");
        let msg = err.to_string();
        assert_eq!(msg, "ERROR - No %output% placeholder in the output wrapper");
    }

    #[test]
    fn empty_wrapper_does_not_error_on_missing_placeholder() {
        // The fast-path no-wrapper case must still pass through.
        // Empty wrapper means "no wrapping requested," not "user
        // supplied an invalid wrapper."
        let out = apply_output_wrapper("body", "", None).expect("ok");
        assert_eq!(out, "body");
    }

    #[test]
    fn wrapper_file_missing_placeholder_also_errors() {
        // Validation runs after file content is resolved, so a
        // bad wrapper read from a file produces the same error.
        let dir = std::env::temp_dir().join(format!("cloc11-32-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("no-placeholder.txt");
        std::fs::write(&p, "/* just a comment */").unwrap();
        let err = apply_output_wrapper("body", "", Some(&p))
            .expect_err("must error");
        assert_eq!(err, WrapperError::MissingOutputPlaceholder);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn wrapper_with_placeholder_still_works() {
        // Regression-guard the happy path so the new validation
        // doesn't accidentally block legitimate wrappers.
        let out = apply_output_wrapper("body", "before %output% after", None)
            .expect("ok");
        assert_eq!(out, "before body after");
    }

    #[test]
    fn wrapper_with_placeholder_and_n_works() {
        // %n% still expands alongside %output%.
        let out = apply_output_wrapper("body", "//banner%n%%output%", None)
            .expect("ok");
        assert_eq!(out, "//banner\nbody");
    }

    #[test]
    fn missing_placeholder_error_display_implements_std_error() {
        let e = WrapperError::MissingOutputPlaceholder;
        let _: &dyn std::error::Error = &e;
        assert!(e.to_string().starts_with("ERROR -"));
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
