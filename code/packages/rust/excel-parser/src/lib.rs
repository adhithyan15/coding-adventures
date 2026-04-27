//! Excel parser backed by compiled parser grammar and reference token normalization.

use coding_adventures_excel_lexer::tokenize_excel_formula;
use lexer::token::{Token, TokenType};
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

fn previous_significant_token(tokens: &[Token], index: usize) -> Option<&Token> {
    let mut i = index;
    while i > 0 {
        i -= 1;
        if tokens[i].effective_type_name() != "SPACE" {
            return Some(&tokens[i]);
        }
    }
    None
}

fn next_significant_token(tokens: &[Token], index: usize) -> Option<&Token> {
    let mut i = index + 1;
    while i < tokens.len() {
        if tokens[i].effective_type_name() != "SPACE" {
            return Some(&tokens[i]);
        }
        i += 1;
    }
    None
}

fn normalize_excel_reference_tokens(tokens: Vec<Token>) -> Vec<Token> {
    let original = tokens.clone();
    tokens
        .into_iter()
        .enumerate()
        .map(|(index, token)| {
            let previous = previous_significant_token(&original, index);
            let next = next_significant_token(&original, index);
            let adjacent_to_colon = previous.map(|t| t.effective_type_name()) == Some("COLON")
                || next.map(|t| t.effective_type_name()) == Some("COLON");

            if token.effective_type_name() == "NAME" && adjacent_to_colon {
                return Token {
                    type_: TokenType::Name,
                    value: token.value,
                    line: token.line,
                    column: token.column,
                    type_name: Some("COLUMN_REF".to_string()),
                    flags: None,
                };
            }

            if token.effective_type_name() == "NUMBER" && adjacent_to_colon {
                return Token {
                    type_: TokenType::Name,
                    value: token.value,
                    line: token.line,
                    column: token.column,
                    type_name: Some("ROW_REF".to_string()),
                    flags: None,
                };
            }

            token
        })
        .collect()
}

pub fn create_excel_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_excel_formula(source);
    let grammar = _grammar::parser_grammar();

    let mut parser = GrammarParser::new(tokens, grammar);
    parser.add_pre_parse(Box::new(normalize_excel_reference_tokens));
    parser
}

pub fn parse_excel_formula(source: &str) -> GrammarASTNode {
    let mut parser = create_excel_parser(source);
    parser
        .parse()
        .unwrap_or_else(|e| panic!("Excel parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_function_formula() {
        assert_eq!(parse_excel_formula("=SUM(A1:B2)").rule_name, "formula");
    }

    #[test]
    fn test_parse_column_range() {
        assert_eq!(parse_excel_formula("A:C").rule_name, "formula");
    }

    #[test]
    fn test_parse_row_range() {
        assert_eq!(parse_excel_formula("1:3").rule_name, "formula");
    }

    #[test]
    fn test_factory_exists() {
        let mut parser = create_excel_parser("A1");
        assert_eq!(parser.parse().expect("parse").rule_name, "formula");
    }
}

