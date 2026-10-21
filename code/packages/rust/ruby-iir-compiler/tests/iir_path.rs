//! LANG84 C2: the IIR path never touches a hand-written lexer or parser.
//!
//! `ruby-parser` still exports the legacy entry points that drive the
//! hand-written `ruby-lexer` state machine (`parse_ruby`,
//! `try_create_ruby_parser`, `tokenize_ruby_for_version`, …) for its other
//! callers. This crate must reach tokens only through the release API
//! (`parse_ruby_at`, `GrammarLexer` over `ruby<release>.tokens`). A source
//! scan is crude, but it is exactly the check LANG84 asks CI to make, and it
//! fails the moment someone "just" calls the old function.

use std::path::Path;

/// Every legacy entry point that runs the hand-written lexer.
const FORBIDDEN: &[&str] = &[
    "try_create_ruby_parser",
    "create_ruby_parser",
    "parse_ruby(",
    "tokenize_ruby(",
    "tokenize_ruby_for_version",
    "DEFAULT_RUBY_ERA",
    "unversioned_parser_grammar",
];

fn rust_sources(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn the_compiler_never_calls_the_hand_written_lexer() {
    let mut files = Vec::new();
    rust_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
    assert!(files.len() >= 4, "{files:?}");
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for needle in FORBIDDEN {
            assert!(!text.contains(needle), "{} mentions {needle}", file.display());
        }
    }
    // And the manifest does not depend on the lexer crate outside tests.
    let manifest = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    let dependencies = manifest.split("[dev-dependencies]").next().unwrap();
    assert!(!dependencies.contains("ruby-lexer"), "ruby-lexer must stay a dev-dependency");
}
