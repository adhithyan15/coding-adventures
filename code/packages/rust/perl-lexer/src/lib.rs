//! Grammar-driven Perl-language lexer for LANG81's bounded print pilot.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

/// Maximum source bytes before lexing the initial Perl subset.
pub const MAX_SOURCE_BYTES: usize = 64 * 1024;

/// Tokenize the accepted Perl source syntax without invoking a host Perl.
pub fn tokenize_perl(source: &str) -> Result<Vec<Token>, String> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(format!(
            "Perl source exceeds the {MAX_SOURCE_BYTES}-byte native pilot limit"
        ));
    }
    let grammar = _grammar::token_grammar();
    GrammarLexer::new(source, &grammar)
        .tokenize()
        .map_err(|error| format!("Perl tokenization failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
