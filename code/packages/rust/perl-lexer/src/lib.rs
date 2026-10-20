//! Grammar-driven Perl-language lexer.
//!
//! This crate owns no lexing code. It owns *grammars*: compiled
//! [`TokenGrammar`] tables, generated from `code/grammars/perl/` by
//! `grammar-tools generate-rust-compiled-grammars perl`, which the generic
//! [`GrammarLexer`] runs over.
//!
//! There are two kinds of table (LANG84):
//!
//! | Table | Source | Reached through |
//! |---|---|---|
//! | per-release | `perl<release>.tokens`, one per non-pending row of `releases.csv` | [`release_token_grammar`] |
//! | unversioned | `perl.tokens`, LANG81's bounded execution grammar | [`tokenize_perl`] |
//!
//! The 117 checked release pairs collapse to a handful of distinct grammars,
//! so the generated `_grammar.rs` holds one module per distinct grammar and a
//! `match` from release ID to module. A release that is pending, or not in the
//! inventory at all, has no table: [`release_token_grammar`] returns `None`
//! and never substitutes a neighbour or the unversioned pair (LANG84 C4.3).

use grammar_tools::token_grammar::TokenGrammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

/// Every release with a compiled token grammar, in inventory order. The full
/// inventory, pending rows included, is `coding_adventures_perl_parser::INVENTORY`
/// (the generator emits it once, in the parser table).
pub use _grammar::SUPPORTED_RELEASES;

/// The deduplicated module (`g_<12 hex>`) that holds a release's table, or
/// `None` for a release without one. Two releases share a class exactly when
/// their compiled token grammars are equal apart from line numbers.
pub use _grammar::grammar_class;

/// Maximum source bytes before lexing the initial Perl subset.
pub const MAX_SOURCE_BYTES: usize = 64 * 1024;

/// The compiled token grammar of exactly one inventory release, or `None`
/// when the release is pending or unknown.
///
/// ```
/// use coding_adventures_perl_lexer::release_token_grammar;
/// assert!(release_token_grammar("1.000").is_some()); // the earliest release
/// assert!(release_token_grammar("1.001").is_none()); // pending: no pair yet
/// assert!(release_token_grammar("").is_none()); // no implicit default
/// ```
pub fn release_token_grammar(release: &str) -> Option<TokenGrammar> {
    _grammar::token_grammar(release)
}

/// The unversioned LANG81 token grammar (`perl.tokens`).
pub fn unversioned_token_grammar() -> TokenGrammar {
    _grammar::unversioned_token_grammar()
}

/// Tokenize the accepted Perl source syntax with the unversioned LANG81
/// grammar, without invoking a host Perl.
pub fn tokenize_perl(source: &str) -> Result<Vec<Token>, String> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(format!(
            "Perl source exceeds the {MAX_SOURCE_BYTES}-byte native pilot limit"
        ));
    }
    let grammar = unversioned_token_grammar();
    GrammarLexer::new(source, &grammar)
        .tokenize()
        .map_err(|error| format!("Perl tokenization failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use grammar_tools::token_grammar::parse_token_grammar;
    use std::path::Path;

    #[test]
    fn tokenizes_print_and_arithmetic() {
        let tokens = tokenize_perl("print(7 - 2 * 3);\n").unwrap();
        let values: Vec<_> = tokens.iter().map(|token| token.value.as_str()).collect();
        assert_eq!(
            values[..9],
            ["print", "(", "7", "-", "2", "*", "3", ")", ";"]
        );
    }

    #[test]
    fn source_limit_precedes_tokenization() {
        assert!(tokenize_perl(&" ".repeat(MAX_SOURCE_BYTES + 1))
            .unwrap_err()
            .contains("source exceeds"));
    }

    #[test]
    fn adjacent_minus_is_decrement_token() {
        let adjacent = tokenize_perl("print(1--2);").unwrap();
        assert!(adjacent
            .iter()
            .any(|token| token.type_name.as_deref() == Some("DECREMENT")));
        let spaced = tokenize_perl("print(1- -2);").unwrap();
        assert!(!spaced
            .iter()
            .any(|token| token.type_name.as_deref() == Some("DECREMENT")));
    }

    /// Zero the one field deduplication is allowed to disagree on. Every
    /// release in a class shares the FIRST release's module, so its line
    /// numbers belong to that file, not necessarily this one.
    fn without_line_numbers(mut grammar: TokenGrammar) -> TokenGrammar {
        for definition in grammar
            .definitions
            .iter_mut()
            .chain(grammar.skip_definitions.iter_mut())
            .chain(grammar.error_definitions.iter_mut())
        {
            definition.line_number = 0;
        }
        for group in grammar.groups.values_mut() {
            for definition in &mut group.definitions {
                definition.line_number = 0;
            }
        }
        for transition in &mut grammar.transitions {
            transition.line_number = 0;
        }
        grammar
    }

    /// The regeneration check, run where the tables are used: the compiled
    /// release list must equal the inventory's non-pending rows, and every
    /// compiled table must equal its own release's `.tokens` file parsed
    /// fresh. A pair edited without regenerating `_grammar.rs`, or a CSV row
    /// flipped to `partial` without one, fails here.
    #[test]
    fn compiled_tables_match_every_inventoried_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let inventory = std::fs::read_to_string(directory.join("releases.csv")).unwrap();
        let mut rows = inventory.lines().filter(|row| !row.starts_with('#'));
        let header: Vec<_> = rows.next().unwrap().split(',').collect();
        let release_col = header.iter().position(|h| *h == "release").unwrap();
        let status_col = header.iter().position(|h| *h == "grammar_status").unwrap();
        let mut all = Vec::new();
        let mut compiled = Vec::new();
        for row in rows {
            let fields: Vec<_> = row.split(',').collect();
            all.push(fields[release_col]);
            if fields[status_col] != "pending" {
                compiled.push(fields[release_col]);
            }
        }
        assert_eq!(SUPPORTED_RELEASES, compiled.as_slice());
        for release in SUPPORTED_RELEASES {
            assert!(
                grammar_class(release).unwrap().starts_with("g_"),
                "{release}"
            );
            let text =
                std::fs::read_to_string(directory.join(format!("perl{release}.tokens"))).unwrap();
            let expected = without_line_numbers(parse_token_grammar(&text).unwrap());
            let actual = without_line_numbers(release_token_grammar(release).unwrap());
            assert_eq!(actual, expected, "{release}: compiled table is stale");
        }
        for release in &all {
            assert_eq!(
                grammar_class(release).is_some(),
                SUPPORTED_RELEASES.contains(release)
            );
            assert_eq!(
                release_token_grammar(release).is_some(),
                SUPPORTED_RELEASES.contains(release),
                "{release}: a pending release must have no table"
            );
        }
        let text = std::fs::read_to_string(directory.join("perl.tokens")).unwrap();
        assert_eq!(
            without_line_numbers(unversioned_token_grammar()),
            without_line_numbers(parse_token_grammar(&text).unwrap())
        );
    }

    #[test]
    fn unknown_and_empty_release_ids_have_no_table() {
        for release in ["", "5.38", "perl5.38.2", "5.38.2 ", "../perl", "5.999.0"] {
            assert!(release_token_grammar(release).is_none(), "{release:?}");
        }
    }
}
