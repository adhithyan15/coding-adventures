use grammar_tools::token_grammar::TokenGrammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

fn grammar() -> TokenGrammar {
    _grammar::token_grammar()
}

pub fn create_nib_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = grammar();
    GrammarLexer::new(source, &grammar)
}

pub fn tokenize_nib(source: &str) -> Vec<Token> {
    let grammar = grammar();
    let mut lexer = GrammarLexer::new(source, &grammar);
    let mut tokens = lexer
        .tokenize()
        .unwrap_or_else(|err| panic!("Nib tokenization failed: {err}"));

    for token in &mut tokens {
        if token.type_name.as_deref() == Some("KEYWORD") {
            token.type_name = Some(token.value.clone());
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_simple_function() {
        let tokens = tokenize_nib("fn main() { let x: u4 = 7; }");
        assert!(tokens.iter().any(|token| token.value == "fn"));
        assert!(tokens.iter().any(|token| token.value == "main"));
        assert!(tokens.iter().any(|token| token.value == "7"));
    }

    #[test]
    fn keyword_type_names_are_promoted_to_literal_keyword_values() {
        let tokens = tokenize_nib("fn main() {}");
        let fn_token = tokens.iter().find(|token| token.value == "fn").unwrap();
        assert_eq!(fn_token.type_name.as_deref(), Some("fn"));
    }
}
