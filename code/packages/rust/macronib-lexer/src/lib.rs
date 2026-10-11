//! Grammar-driven MacroNib lexer: Nib's tokens plus assembler-style directives.
//!
//! The preprocessor consumes the added directive and string tokens before
//! handing the remaining stream to Nib's own guarded parser.

use grammar_tools::token_grammar::TokenGrammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

fn grammar() -> TokenGrammar {
    _grammar::token_grammar()
}

/// Create a lexer over MacroNib source.
pub fn create_macronib_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = grammar();
    GrammarLexer::new(source, &grammar)
}

/// Tokenize MacroNib source without panicking on malformed input.
pub fn try_tokenize_macronib(source: &str) -> Result<Vec<Token>, String> {
    let grammar = grammar();
    let mut lexer = GrammarLexer::new(source, &grammar);
    let mut tokens = lexer.tokenize().map_err(|e| e.to_string())?;
    for token in &mut tokens {
        if token.type_name.as_deref() == Some("KEYWORD") {
            token.type_name = Some(token.value.clone());
        }
    }
    Ok(tokens)
}

/// Tokenize trusted MacroNib source, panicking if it is malformed.
pub fn tokenize_macronib(source: &str) -> Vec<Token> {
    try_tokenize_macronib(source).unwrap_or_else(|e| panic!("MacroNib tokenization failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use grammar_tools::token_grammar::{parse_token_grammar, TokenDefinition};
    use lexer::token::TokenType;
    use std::path::PathBuf;

    const ONLY: &[&str] = &[
        "DOT_INCLUDE",
        "DOT_IFDEF",
        "DOT_ENDIF",
        "DOT_ELSE",
        "DOT_SET",
        "STR_LIT",
    ];

    fn parse(language: &str, file: &str) -> TokenGrammar {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../grammars")
            .join(language)
            .join(file);
        let source = std::fs::read_to_string(&path).unwrap();
        parse_token_grammar(&source).unwrap()
    }

    fn shape(definition: &TokenDefinition) -> (String, String, bool) {
        (
            definition.name.clone(),
            definition.pattern.clone(),
            definition.is_regex,
        )
    }

    #[test]
    fn directives_and_quoted_include_lex_as_own_tokens() {
        for (text, kind) in [
            (".include", "DOT_INCLUDE"),
            (".ifdef", "DOT_IFDEF"),
            (".endif", "DOT_ENDIF"),
            (".else", "DOT_ELSE"),
            (".set", "DOT_SET"),
            ("\"helper.nib\"", "STR_LIT"),
        ] {
            let tokens = tokenize_macronib(text);
            assert_eq!(tokens.len(), 2);
            assert_eq!(tokens[0].value, text);
            assert_eq!(tokens[0].type_name.as_deref(), Some(kind));
            assert_eq!(tokens[1].type_, TokenType::Eof);
        }
    }

    #[test]
    fn ordinary_nib_tokens_keep_their_values_and_types() {
        let source = "fn main() { let x: u4 = 7; }";
        let reference = coding_adventures_nib_lexer::tokenize_nib(source);
        let actual = tokenize_macronib(source);
        assert_eq!(actual, reference);
    }

    #[test]
    fn every_non_directive_rule_matches_nib_in_order() {
        let nib = parse("nib", "nib.tokens");
        let macro_nib = parse("macronib", "macronib.tokens");
        assert_eq!(
            macro_nib
                .definitions
                .iter()
                .filter(|d| !ONLY.contains(&d.name.as_str()))
                .map(shape)
                .collect::<Vec<_>>(),
            nib.definitions.iter().map(shape).collect::<Vec<_>>()
        );
        for extra in ONLY {
            assert!(macro_nib.definitions.iter().any(|d| d.name == *extra));
        }
        assert_eq!(macro_nib.keywords, nib.keywords);
        assert_eq!(
            macro_nib
                .skip_definitions
                .iter()
                .map(shape)
                .collect::<Vec<_>>(),
            nib.skip_definitions.iter().map(shape).collect::<Vec<_>>()
        );
        assert_eq!(macro_nib.case_sensitive, nib.case_sensitive);
        assert_eq!(macro_nib.case_insensitive, nib.case_insensitive);
    }

    #[test]
    fn compiled_grammar_matches_source_file() {
        let source = parse("macronib", "macronib.tokens");
        let compiled = grammar();
        assert_eq!(
            compiled.definitions.iter().map(shape).collect::<Vec<_>>(),
            source.definitions.iter().map(shape).collect::<Vec<_>>()
        );
        assert_eq!(compiled.keywords, source.keywords);
        assert_eq!(
            compiled
                .skip_definitions
                .iter()
                .map(shape)
                .collect::<Vec<_>>(),
            source
                .skip_definitions
                .iter()
                .map(shape)
                .collect::<Vec<_>>()
        );
    }
}
