//! Python lexer backed by compiled token grammars.
//!
//! Two independent tables live in the generated `_grammar.rs`:
//!
//! | Table | Keyed by | Source files | Used by |
//! |-------|----------|--------------|---------|
//! | [`releases`] | an exact `releases.csv` ID (`"3.13.16"`) | `python<release>.tokens` (LANG84 release pairs) | `python-parser`'s release API, the IIR path |
//! | legacy minor lines | `"2.7"` … `"3.12"` | `python<minor>.tokens` | [`tokenize_python`], [`create_python_lexer`] |
//!
//! The two never mix. A release ID is not a minor-line version: asking the
//! legacy API for `"3.13.16"` is an error, and asking [`releases`] for
//! `"3.12"` is `None`. That is LANG84's "no silent fallback" (C4): a release
//! never borrows a neighbouring grammar, and a minor-line pair is never
//! mistaken for a tagged release.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

/// The legacy minor-line versions the version API accepts.
pub const SUPPORTED_VERSIONS: &[&str] = _grammar::legacy_versions::SUPPORTED_VERSIONS;
/// The legacy version an empty `version` string selects.
pub const DEFAULT_VERSION: &str = "3.12";

/// LANG84 release tables: one compiled `TokenGrammar` per distinct
/// `python<release>.tokens`, selected by exact inventory ID.
///
/// `token_grammar(release)` answers for a release with a pair and `None` for
/// everything else (an unknown ID, a pending row, a legacy minor line). The
/// release-selected lexer and parser are in `python-parser`
/// (`tokenize_python_at`, `parse_python_at`); this crate only owns the
/// tables, because `python-parser` depends on it and not the other way round.
pub mod releases {
    pub use crate::_grammar::{grammar_class, token_grammar, SUPPORTED_RELEASES};
}

fn resolve_version(version: &str) -> Result<&str, String> {
    let resolved = if version.is_empty() {
        DEFAULT_VERSION
    } else {
        version
    };

    if SUPPORTED_VERSIONS.contains(&resolved) {
        Ok(resolved)
    } else {
        Err(format!(
            "Unsupported Python version '{version}'. Supported versions: {}",
            SUPPORTED_VERSIONS
                .iter()
                .map(|value| format!("\"{}\"", value))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

pub fn create_python_lexer<'src>(
    source: &'src str,
    version: &str,
) -> Result<GrammarLexer<'src>, String> {
    let version = resolve_version(version)?;
    let grammar = _grammar::legacy_versions::token_grammar(version)
        .expect("compiled Python token grammar missing supported version");
    Ok(GrammarLexer::new(source, &grammar))
}

pub fn tokenize_python(source: &str, version: &str) -> Result<Vec<Token>, String> {
    let version = resolve_version(version)?;
    let grammar = _grammar::legacy_versions::token_grammar(version)
        .expect("compiled Python token grammar missing supported version");
    let mut lexer = GrammarLexer::new(source, &grammar);
    lexer
        .tokenize()
        .map_err(|e| format!("Python tokenization failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    #[test]
    fn tokenizes_python_with_default_version() {
        let tokens = tokenize_python("x = 1\n", "").unwrap();
        assert_eq!(tokens[0].type_, TokenType::Name);
        assert_eq!(tokens[0].value, "x");
    }

    #[test]
    fn supports_indentation_tokens() {
        let tokens = tokenize_python("def f():\n    return 1\n", "3.12").unwrap();
        assert!(tokens.iter().any(|token| token.type_ == TokenType::Indent));
        assert!(tokens.iter().any(|token| token.type_ == TokenType::Dedent));
    }

    #[test]
    fn soft_keywords_remain_names() {
        let tokens = tokenize_python("match = 42\n", "3.12").unwrap();
        assert_eq!(tokens[0].type_, TokenType::Name);
        assert_eq!(tokens[0].value, "match");
    }

    #[test]
    fn all_supported_versions_load() {
        for version in SUPPORTED_VERSIONS {
            let tokens = tokenize_python("x = 1\n", version).unwrap();
            assert_eq!(tokens.last().unwrap().type_, TokenType::Eof, "version {version}");
        }
    }

    #[test]
    fn unsupported_version_returns_error() {
        let error = tokenize_python("x = 1\n", "4.0").unwrap_err();
        assert!(error.contains("4.0"));
    }

    // -------------------------------------------------------------------
    // LANG84 release tables
    // -------------------------------------------------------------------

    use super::releases::{grammar_class, token_grammar, SUPPORTED_RELEASES};

    #[test]
    fn release_tables_list_the_paired_releases_in_inventory_order() {
        // Inventory (date) order: 3.13.16 (2026-09-30) precedes 3.11.17
        // (2026-10-01), so this is not version-sorted, on purpose. 2.7.18
        // (2020-04-19) comes after 3.0a1 (2007-08-31), and 3.5.10
        // (2020-09-05) after 3.6.0 (2016-12-23), for the same reason.
        // 2.2 and 2.7.18 are the phase 7c slice 1 (Python 2) pairs; 2.4,
        // 2.6, 3.5.10 and 3.6.0 came in slice 2.
        assert_eq!(
            SUPPORTED_RELEASES,
            [
                "2.2", "2.4", "3.0a1", "2.6", "3.6.0", "2.7.18", "3.5.10", "3.12.3", "3.13.16", "3.11.17",
                "3.16.0a1"
            ]
        );
        for release in SUPPORTED_RELEASES {
            let grammar = token_grammar(release).expect("paired release has a table");
            let tokens = GrammarLexer::new("print(1 // 2)\n", &grammar).tokenize().unwrap();
            assert_eq!(tokens[0].value, "print", "{release}");
            assert_eq!(tokens[3].effective_type_name(), "FLOOR_DIV", "{release}");
        }
    }

    #[test]
    fn identical_token_files_share_one_class() {
        // 3.12.3, 3.13.16 and 3.16.0a1 lex this slice identically (48
        // operators, 35 keywords, PEP 515 `_`); 3.11.17 has no `!`; 3.0a1
        // has no `_`, no `@=`/`:=` and no async/await keywords.
        let class = |r| grammar_class(r).unwrap();
        assert_eq!(class("3.12.3"), class("3.13.16"));
        assert_eq!(class("3.13.16"), class("3.16.0a1"));
        assert_ne!(class("3.11.17"), class("3.13.16"));
        assert_ne!(class("3.0a1"), class("3.11.17"));
        // 3.5.10 is 3.0a1 plus `@=` and the barry_as_FLUFL `<>`; 3.6.0 is
        // 3.5.10 plus PEP 515 `_` separators.
        assert_ne!(class("3.5.10"), class("3.0a1"));
        assert_ne!(class("3.6.0"), class("3.5.10"));
        // Python 2: 2.4 adds `@` (decorators) to 2.2's table; 2.6 adds the
        // keywords `as` and `with` and the PEP 3127 `0o`/`0b` literals, and
        // 2.7.18 lexes this slice exactly as 2.6 does. None matches Python 3.
        assert_ne!(class("2.2"), class("2.4"));
        assert_ne!(class("2.4"), class("2.6"));
        assert_eq!(class("2.6"), class("2.7.18"));
        assert_ne!(class("2.7.18"), class("3.0a1"));
    }

    #[test]
    fn python_2_pairs_lex_print_as_a_keyword_and_keep_the_old_operators() {
        for release in ["2.2", "2.4", "2.6", "2.7.18"] {
            let grammar = token_grammar(release).unwrap();
            let tokens = GrammarLexer::new("print 7 / 2 <> 3
", &grammar).tokenize().unwrap();
            let kinds: Vec<&str> = tokens.iter().map(|t| t.effective_type_name()).collect();
            assert_eq!(kinds[..6], ["KEYWORD", "INT", "SLASH", "INT", "LEGACY_NOT_EQUALS", "INT"], "{release}");
            // Every Python 2 integer spelling is one INT token (slice 2);
            // a decimal digit after a leading `0`, a bare `0x`, `_` and a
            // second suffix are lexer errors.
            for source in ["010", "00", "0", "0x1F", "0XffL", "1L", "7l", "0777L"] {
                let tokens = GrammarLexer::new(source, &grammar).tokenize().unwrap();
                assert_eq!((tokens[0].effective_type_name(), tokens[0].value.as_str()), ("INT", source), "{release}");
            }
            for source in ["08\n", "0178\n", "0x\n", "1_0\n", "1LL\n", "0x1Fg\n", "1if\n"] {
                assert!(GrammarLexer::new(source, &grammar).tokenize().is_err(), "{release} {source:?}");
            }
        }
        // PEP 3127 `0o`/`0b` are tokens from 2.6 on; before that `0o17` is
        // `0` running into a name, a lexer error here.
        let lexes = |r, source| GrammarLexer::new(source, &token_grammar(r).unwrap()).tokenize().is_ok();
        for source in ["0o17\n", "0O17L\n", "0b101\n", "0B1l\n"] {
            assert!(!lexes("2.2", source) && !lexes("2.4", source), "{source:?}");
            assert!(lexes("2.6", source) && lexes("2.7.18", source), "{source:?}");
        }
        for source in ["0o8\n", "0b2\n", "0o\n", "0b\n"] {
            assert!(!lexes("2.6", source) && !lexes("2.7.18", source), "{source:?}");
        }
        // `@` is a token from 2.4 on: 2.2 refuses it, 2.4 and later lex it.
        assert!(!lexes("2.2", "@\n") && lexes("2.4", "@\n") && lexes("2.7.18", "@\n"));
    }

    #[test]
    fn release_ids_and_minor_lines_never_cross() {
        // A minor-line version is not a release, and a release is not a
        // minor-line version (LANG84 C4).
        for version in SUPPORTED_VERSIONS {
            assert!(token_grammar(version).is_none(), "{version}");
        }
        for release in SUPPORTED_RELEASES {
            assert!(tokenize_python("x\n", release).is_err(), "{release}");
        }
        assert!(token_grammar("").is_none() && token_grammar("3.13").is_none());
    }
}
