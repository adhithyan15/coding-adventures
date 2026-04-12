//! Excel lexer backed by compiled token grammar and callback-based token reclassification.

use lexer::grammar_lexer::{GrammarLexer, LexerContext};
use lexer::token::Token;

mod _grammar;

fn next_non_space_char(ctx: &LexerContext) -> String {
    let mut offset = 1;
    loop {
        let ch = ctx.peek(offset);
        if ch.is_empty() || ch != " " {
            return ch.to_string();
        }
        offset += 1;
    }
}

fn excel_on_token(token: &Token, ctx: &mut LexerContext) {
    if token.effective_type_name() != "NAME" {
        return;
    }

    let next_char = next_non_space_char(ctx);
    if next_char == "(" {
        ctx.suppress();
        ctx.emit(Token {
            type_: token.type_,
            value: token.value.clone(),
            line: token.line,
            column: token.column,
            type_name: Some("FUNCTION_NAME".to_string()),
            flags: None,
        });
        return;
    }

    if next_char == "[" {
        ctx.suppress();
        ctx.emit(Token {
            type_: token.type_,
            value: token.value.clone(),
            line: token.line,
            column: token.column,
            type_name: Some("TABLE_NAME".to_string()),
            flags: None,
        });
    }
}

pub fn create_excel_lexer(source: &str) -> GrammarLexer<'_> {
    let mut grammar = _grammar::token_grammar();
    for definition in &mut grammar.definitions {
        if definition.is_regex && !definition.pattern.starts_with('^') {
            definition.pattern = format!("^(?:{})", definition.pattern);
        }
        if matches!(
            definition.name.as_str(),
            "FUNCTION_NAME" | "TABLE_NAME" | "COLUMN_REF" | "ROW_REF"
        ) {
            definition.pattern = "a^".to_string();
        }
    }
    for definition in &mut grammar.skip_definitions {
        if definition.is_regex && !definition.pattern.starts_with('^') {
            definition.pattern = format!("^(?:{})", definition.pattern);
        }
    }

    let mut lexer = GrammarLexer::new(source, &grammar);
    lexer.set_on_token(Some(Box::new(excel_on_token)));
    lexer
}

pub fn tokenize_excel_formula(source: &str) -> Vec<Token> {
    let mut lexer = create_excel_lexer(source);
    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("Excel tokenization failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token_types(tokens: &[Token]) -> Vec<String> {
        tokens
            .iter()
            .map(|token| token.effective_type_name().to_string())
            .collect()
    }

    #[test]
    fn test_function_name_reclassification() {
        let tokens = tokenize_excel_formula("=SUM(A1)");
        assert_eq!(token_types(&tokens)[1], "FUNCTION_NAME");
        assert_eq!(tokens[1].value, "SUM");
    }

    #[test]
    fn test_table_name_reclassification() {
        let tokens = tokenize_excel_formula("DeptSales[Sales Amount]");
        assert_eq!(token_types(&tokens)[0], "TABLE_NAME");
        assert_eq!(tokens[0].value, "DeptSales");
    }

    #[test]
    fn test_factory_exists() {
        let mut lexer = create_excel_lexer("A1");
        let tokens = lexer.tokenize().expect("tokenize");
        assert_eq!(tokens[0].effective_type_name(), "CELL");
    }
}

