//! Grammar-driven lexer for a focused Mermaid flowchart subset.

pub const VERSION: &str = "0.1.0";

use grammar_tools::token_grammar::parse_token_grammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

const TOKEN_GRAMMAR_SOURCE: &str = include_str!("../../../../grammars/mermaid.tokens");

pub fn create_mermaid_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = parse_token_grammar(TOKEN_GRAMMAR_SOURCE)
        .unwrap_or_else(|e| panic!("Failed to parse mermaid.tokens: {e}"));
    GrammarLexer::new(source, &grammar)
}

pub fn tokenize_mermaid(source: &str) -> Vec<Token> {
    let mut lexer = create_mermaid_lexer(source);
    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("Mermaid tokenization failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    fn custom_name(token: &Token) -> Option<&str> {
        token.type_name.as_deref()
    }

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn tokenizes_header_shapes_and_edges() {
        let tokens = tokenize_mermaid("flowchart LR\nA[Start] -->|yes| B{Ship?}\n");

        assert_eq!(tokens[0].type_, TokenType::Keyword);
        assert_eq!(tokens[0].value, "flowchart");
        assert_eq!(custom_name(&tokens[1]), Some("DIRECTION"));
        assert_eq!(tokens[1].value, "LR");
        assert_eq!(tokens[2].type_, TokenType::Newline);

        assert_eq!(tokens[3].type_, TokenType::Name);
        assert_eq!(tokens[3].value, "A");
        assert_eq!(custom_name(&tokens[4]), Some("RECT"));
        assert_eq!(tokens[4].value, "[Start]");

        assert_eq!(custom_name(&tokens[5]), Some("ARROW"));
        assert_eq!(custom_name(&tokens[6]), Some("EDGE_LABEL"));
        assert_eq!(tokens[6].value, "|yes|");

        assert_eq!(tokens[7].type_, TokenType::Name);
        assert_eq!(tokens[7].value, "B");
        assert_eq!(custom_name(&tokens[8]), Some("DIAMOND"));
        assert_eq!(tokens[8].value, "{Ship?}");
    }

    #[test]
    fn comments_are_skipped() {
        let tokens =
            tokenize_mermaid("%% heading comment\nflowchart TD\n%% edge comment\nA --- B\n");
        let values: Vec<&str> = tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.value.as_str())
            .collect();

        assert!(!values.iter().any(|v| v.contains("comment")));
        assert!(values.contains(&"flowchart"));
        assert!(values.contains(&"TD"));
        assert!(values.contains(&"A"));
        assert!(values.contains(&"B"));
    }

    #[test]
    fn shape_tokens_preserve_delimiters() {
        let tokens =
            tokenize_mermaid("flowchart TB\nA((Circle)); B(Round); C[Rect]; D{Decision}\n");
        let custom_tokens: Vec<(&str, &str)> = tokens
            .iter()
            .filter_map(|t| t.type_name.as_deref().map(|name| (name, t.value.as_str())))
            .collect();
        let semicolon_count = tokens
            .iter()
            .filter(|t| t.type_ == TokenType::Semicolon)
            .count();

        assert!(custom_tokens.contains(&("CIRCLE", "((Circle))")));
        assert!(custom_tokens.contains(&("ROUND", "(Round)")));
        assert!(custom_tokens.contains(&("RECT", "[Rect]")));
        assert!(custom_tokens.contains(&("DIAMOND", "{Decision}")));
        assert_eq!(semicolon_count, 3);
    }
}
