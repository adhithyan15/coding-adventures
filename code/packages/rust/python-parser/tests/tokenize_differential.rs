//! The release `.tokens` files against CPython's own tokenizer (LANG84
//! "Corpora and oracles": CPython `tokenize` as a lexing differential).
//!
//! `tests/fixtures/tokenize/<case>.pytokens` was recorded by
//! `tests/fixtures/tokenize_oracle.py` under host CPython 3.11.17, 3.12.3 and
//! 3.13.16, which agree on every case (the script refuses to overwrite a
//! fixture with a different stream). Each host release's compiled token table
//! must produce exactly that stream: same kinds, same spellings, same 1-based
//! positions. CPython's `NL` and `COMMENT` are trivia that the grammar-driven
//! lexer skips, and synthetic tokens are compared by kind only.

use std::path::{Path, PathBuf};

use coding_adventures_python_parser::{tokenize_python_at, FrontendBudgets, PythonFrontendError};

/// Releases whose host interpreter recorded the fixtures.
const HOST_RELEASES: &[&str] = &["3.11.17", "3.12.3", "3.13.16"];

fn cases() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tokenize");
    let mut cases: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "py"))
        .collect();
    cases.sort();
    assert!(cases.len() >= 6, "{cases:?}");
    cases
}

/// Our token stream in the fixture's line format.
fn stream(release: &str, source: &str) -> Result<String, PythonFrontendError> {
    let (_, tokens) = tokenize_python_at(source, Some(release), &FrontendBudgets::default())?;
    let mut out = String::new();
    for token in tokens {
        let kind = token.effective_type_name();
        match kind {
            "NEWLINE" | "INDENT" | "DEDENT" | "EOF" => out.push_str(kind),
            _ => out.push_str(&format!("{kind}\t{}\t{}\t{}", token.line, token.column, token.value)),
        }
        out.push('\n');
    }
    Ok(out)
}

#[test]
fn host_releases_lex_exactly_as_cpython_tokenize() {
    for case in cases() {
        let source = std::fs::read_to_string(&case).unwrap();
        let expected = std::fs::read_to_string(case.with_extension("pytokens")).unwrap();
        for release in HOST_RELEASES {
            let ours = stream(release, &source).unwrap_or_else(|e| panic!("{release} {case:?}: {e}"));
            assert_eq!(ours, expected, "{release}: {case:?}");
        }
    }
}

/// 3.16.0a1 shares 3.13.16's token table (identical `Grammar/Tokens`,
/// `keyword.py` and number rules), so it lexes every case identically.
#[test]
fn the_newest_release_lexes_like_the_default() {
    for case in cases() {
        let source = std::fs::read_to_string(&case).unwrap();
        assert_eq!(stream("3.16.0a1", &source).unwrap(), stream("3.13.16", &source).unwrap(), "{case:?}");
    }
}

/// 3.0a1 differs from the host releases exactly where its tokenizer did:
/// no PEP 515 `_`, no `:=`/`@=` operators, and `async`/`await` are names.
#[test]
fn release_3_0a1_differs_only_where_its_tokenizer_did() {
    for case in cases() {
        let name = case.file_stem().unwrap().to_string_lossy().into_owned();
        let source = std::fs::read_to_string(&case).unwrap();
        let expected = std::fs::read_to_string(case.with_extension("pytokens")).unwrap();
        match name.as_str() {
            "02_numbers" => assert!(matches!(
                stream("3.0a1", &source),
                Err(PythonFrontendError::Lex { line: 1, .. })
            )),
            "03_operators" => {
                let ours = stream("3.0a1", &source).unwrap();
                assert!(ours.contains("COLON\t4\t3\t:\nEQUALS\t4\t4\t="), "{ours}");
                assert!(ours.contains("AT\t3\t67\t@\nEQUALS\t3\t68\t="), "{ours}");
                assert!(!ours.contains("WALRUS") && !ours.contains("AT_EQUALS"));
            }
            "04_keywords" => {
                let ours = stream("3.0a1", &source).unwrap();
                let renamed = expected
                    .replace("KEYWORD\t1\t31\tasync", "NAME\t1\t31\tasync")
                    .replace("KEYWORD\t1\t37\tawait", "NAME\t1\t37\tawait");
                assert_ne!(renamed, expected, "the fixture must hold async and await");
                assert_eq!(ours, renamed);
            }
            _ => assert_eq!(stream("3.0a1", &source).unwrap(), expected, "{name}"),
        }
    }
}
