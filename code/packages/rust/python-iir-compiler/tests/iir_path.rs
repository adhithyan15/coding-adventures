//! LANG84 C2 and C4.5 for the release path.
//!
//! The crate also holds LANG79's float pilot, which parses with the legacy
//! minor-line grammar (`parse_python(source, "3.12")`). The release path must
//! never reach that grammar, nor the unversioned `python.grammar`: it gets
//! tokens and trees only through `python-parser`'s release API
//! (`parse_python_at`). A source scan is crude, but it is exactly the check
//! LANG84 asks CI to make, and it fails the moment someone "just" calls the
//! old function from a release-path module.

use std::path::Path;

/// A source file's non-test code without its comment lines: the docs may
/// name the legacy grammar to explain why it is not used, and a unit test may
/// probe that a minor line is refused, but the code may not use one.
fn code_of(file: &str) -> String {
    let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(file)).unwrap();
    let code = text.split("#[cfg(test)]").next().unwrap_or_default();
    code.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The release-path modules.
const RELEASE_PATH: &[&str] = &["release_api.rs", "int_lower.rs", "profiles.rs", "frontend_error.rs"];

/// Legacy entry points that select a minor line or the unversioned pair.
const FORBIDDEN: &[&str] = &[
    "parse_python(",
    "create_python_parser",
    "tokenize_python(",
    "create_python_lexer",
    "DEFAULT_VERSION",
    "SUPPORTED_VERSIONS",
    "legacy_versions",
    "\"3.12\"",
];

#[test]
fn the_release_path_never_calls_a_legacy_grammar() {
    for file in RELEASE_PATH {
        let text = code_of(file);
        for needle in FORBIDDEN {
            assert!(!text.contains(needle), "{file} mentions {needle}");
        }
    }
    // The release path does call the release API.
    assert!(code_of("release_api.rs").contains("parse_python_at(source, Some(release), budgets)"));
}

/// LANG84 C2: no version comparisons in the compiler. Profiles are looked
/// up by exact ID; nothing parses, orders or compares release strings.
#[test]
fn the_release_path_never_compares_release_ids() {
    for file in RELEASE_PATH {
        let text = code_of(file);
        for needle in ["release >", "release <", "release.split", "release.starts_with", "parse::<f"] {
            assert!(!text.contains(needle), "{file} compares release IDs: {needle}");
        }
    }
}
