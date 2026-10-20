//! Refuse what Closure refuses (CCR-075, #15860; spec CLOC44).
//!
//! The owner decided on 2026-10-19 that when the pinned reference compiler
//! (`closure-compiler-v20260915`) rejects an input, closurec rejects it the same
//! way. That means the same diagnostic key and message, the same location, the
//! same exit status, and no JavaScript, output file or sidecar. closurec does
//! not compile a superset of what Closure compiles.
//!
//! This module is that gate. It scans every input before any compilation work
//! starts, so a refused program never reaches the pipeline and nothing is ever
//! written.
//!
//! # What is refused (slice 1)
//!
//! | Construct                         | Upstream diagnostic                     |
//! |-----------------------------------|-----------------------------------------|
//! | any private element (`#x`)        | `JSC_UNSUPPORTED_LANGUAGE_FEATURE` …    |
//! |                                   | "Private elements."                     |
//!
//! Upstream reports **every** occurrence: the `#x` of a field, method or
//! accessor declaration, every `this.#x` use, and `#x in o`. Each one is a
//! separate error at that token's position, at every compilation level.
//! Later slices add the rest of the refusals captured in
//! `tests/oracle/upstream-refusals-v20260915.json`.
//!
//! # The output format, byte for byte
//!
//! Measured against the jar:
//!
//! ```text
//! a.js:12:10: ERROR - [JSC_UNSUPPORTED_LANGUAGE_FEATURE] This language feature is not currently supported by the compiler: Private elements.
//!   12| class B { static #s = 1; … }
//!                 ^
//!
//! 1 error(s), 0 warning(s)
//! ```
//!
//! - The header is `file:line:column`, with a 1-based line and a 0-based
//!   column. The file is spelled as the user passed it.
//! - The excerpt is two spaces, the line number and `| `, then the source line
//!   verbatim. The caret is indented by that prefix's width plus the column.
//! - A blank line follows each error, and the run ends with
//!   `N error(s), 0 warning(s)`.
//! - Errors appear in input order, then by position. The exit status is
//!   `min(N, 127)`.

use coding_adventures_javascript_lexer::tokenize_javascript_typed;
use coding_adventures_javascript_tokens::EsVersion;

/// One refused construct, located the way upstream reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The input file, spelled as on the command line.
    pub file: String,
    /// 1-based line.
    pub line: usize,
    /// 0-based column, in characters.
    pub column: usize,
    /// The diagnostic key, e.g. `JSC_UNSUPPORTED_LANGUAGE_FEATURE`.
    pub key: &'static str,
    /// The diagnostic message, exactly as upstream prints it.
    pub message: &'static str,
    /// The source line the error points into, for the excerpt.
    pub excerpt: String,
}

const UNSUPPORTED_FEATURE: &str = "JSC_UNSUPPORTED_LANGUAGE_FEATURE";
const PRIVATE_ELEMENTS: &str =
    "This language feature is not currently supported by the compiler: Private elements.";

/// Every refusal in one input file, in source order.
///
/// A file the lexer can't tokenize yields none here, because the ordinary
/// pipeline reports that failure itself.
pub fn scan(file: &str, source: &str, version: EsVersion) -> Vec<Refusal> {
    let Ok(tokens) = tokenize_javascript_typed(source, version) else {
        return Vec::new();
    };
    let lines: Vec<&str> = source.lines().collect();
    let at = |t: &lexer::token::Token| Refusal {
        file: file.to_string(),
        line: t.line,
        // The lexer's columns are 1-based; upstream prints 0-based.
        column: t.column.saturating_sub(1),
        key: UNSUPPORTED_FEATURE,
        message: PRIVATE_ELEMENTS,
        excerpt: lines
            .get(t.line.saturating_sub(1))
            .copied()
            .unwrap_or_default()
            .to_string(),
    };
    let mut out = Vec::new();
    for (i, t) in tokens.iter().enumerate() {
        if t.type_name.as_deref() != Some("PRIVATE_NAME") {
            continue;
        }
        // A `static` private FIELD is reported twice upstream: once at the
        // `static` keyword (the member) and once at the name. A private
        // method, accessor or non-static field is reported once, at the
        // name. Measured: `static #x = 1` errors at both 10 and 17, while
        // `static #m(){}` errors only at 17.
        let is_method = tokens.get(i + 1).is_some_and(|n| n.value == "(");
        if !is_method && i > 0 && tokens[i - 1].value == "static" {
            out.push(at(&tokens[i - 1]));
        }
        out.push(at(t));
    }
    out
}

/// Render `refusals` exactly as upstream prints them to stderr, without the
/// final newline (the CLI adds it).
pub fn render(refusals: &[Refusal]) -> String {
    let mut out = String::new();
    for r in refusals {
        let gutter = format!("  {}| ", r.line);
        out.push_str(&format!(
            "{}:{}:{}: ERROR - [{}] {}\n{gutter}{}\n{}^\n\n",
            r.file,
            r.line,
            r.column,
            r.key,
            r.message,
            r.excerpt,
            " ".repeat(gutter.chars().count() + r.column),
        ));
    }
    out.push_str(&format!("{} error(s), 0 warning(s)", refusals.len()));
    out
}

/// The file name as upstream prints it. Java's `java.io.File` collapses runs
/// of `/`, so `T//a.js` is reported as `T/a.js`.
pub fn display_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for c in path.chars() {
        if c == '/' && out.ends_with('/') {
            continue;
        }
        out.push(c);
    }
    out
}

/// Upstream's process exit status for `errors` errors.
pub fn exit_status(errors: usize) -> u8 {
    errors.min(127) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan_es(src: &str) -> Vec<Refusal> {
        scan("a.js", src, EsVersion::Es2025)
    }

    #[test]
    fn every_private_element_occurrence_is_refused() {
        // Field declaration and the `this.#x` read: two errors, like upstream.
        let r = scan_es("class A { #x = 1; get(){ return this.#x; } }");
        let cols: Vec<usize> = r.iter().map(|r| r.column).collect();
        assert_eq!(cols, vec![10, 37]);
        assert!(r.iter().all(|r| r.line == 1 && r.key == UNSUPPORTED_FEATURE));
    }

    #[test]
    fn methods_accessors_statics_and_brand_checks_are_refused() {
        let r = scan_es("class B { static #s = 1; #m() { return #s in this; } get #g() { return 1; } }");
        let cols: Vec<usize> = r.iter().map(|r| r.column).collect();
        assert_eq!(cols, vec![10, 17, 25, 39, 57]);
    }

    #[test]
    fn a_static_private_field_is_also_reported_at_static() {
        // Measured against v20260915.
        let cols = |src| scan_es(src).iter().map(|r| r.column).collect::<Vec<_>>();
        assert_eq!(cols("class A { static #x = 1; }"), vec![10, 17]);
        assert_eq!(cols("class A { static #x; }"), vec![10, 17]);
        assert_eq!(cols("class A { static #m() {} }"), vec![17]);
        assert_eq!(cols("class A { static get #g() {} }"), vec![21]);
        assert_eq!(cols("class A { get #g() {} }"), vec![14]);
        assert_eq!(cols("class A { static async *#g() {} }"), vec![24]);
    }

    #[test]
    fn programs_without_private_elements_are_not_refused() {
        assert!(scan_es("class A { x = 1; m() { return '#x'; } } // #y").is_empty());
    }

    #[test]
    fn render_matches_upstream_bytes() {
        // Captured from closure-compiler-v20260915 (two errors, exit 2).
        let src = "class A { #x = 1; get(){ return this.#x; } }\nconsole.log(new A().get());\n";
        let out = render(&scan("t.js", src, EsVersion::Es2025));
        let line = "class A { #x = 1; get(){ return this.#x; } }";
        let expected = format!(
            "t.js:1:10: ERROR - [{UNSUPPORTED_FEATURE}] {PRIVATE_ELEMENTS}\n  1| {line}\n{}^\n\n\
             t.js:1:37: ERROR - [{UNSUPPORTED_FEATURE}] {PRIVATE_ELEMENTS}\n  1| {line}\n{}^\n\n\
             2 error(s), 0 warning(s)",
            " ".repeat(15),
            " ".repeat(42),
        );
        assert_eq!(out, expected);
    }

    #[test]
    fn multi_digit_lines_widen_the_gutter() {
        let src = format!("{}class C {{\n  #y;\n}}\n", "var v;\n".repeat(11));
        let r = scan("b.js", &src, EsVersion::Es2025);
        assert_eq!((r[0].line, r[0].column), (13, 2));
        assert!(render(&r).starts_with("b.js:13:2: ERROR"));
        assert!(render(&r).contains("\n  13|   #y;\n        ^\n"));
    }

    #[test]
    fn display_path_collapses_repeated_slashes_like_java_file() {
        assert_eq!(display_path("/tmp//cc/a.js"), "/tmp/cc/a.js");
        assert_eq!(display_path("tests/diff/x/input/a.js"), "tests/diff/x/input/a.js");
    }

    #[test]
    fn exit_status_is_the_error_count_capped_at_127() {
        assert_eq!(exit_status(1), 1);
        assert_eq!(exit_status(6), 6);
        assert_eq!(exit_status(200), 127);
    }
}
