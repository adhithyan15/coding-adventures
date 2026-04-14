use coding_adventures_nib_lexer::tokenize_nib;
use parser::grammar_parser::{GrammarASTNode, GrammarParseError, GrammarParser};

mod _grammar;

pub fn create_nib_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_nib(source);
    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar)
}

pub fn parse_nib(source: &str) -> Result<GrammarASTNode, GrammarParseError> {
    let mut parser = create_nib_parser(source);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    fn has_rule(node: &GrammarASTNode, expected: &str) -> bool {
        if node.rule_name == expected {
            return true;
        }

        node.children.iter().any(|child| match child {
            ASTNodeOrToken::Node(inner) => has_rule(inner, expected),
            ASTNodeOrToken::Token(_) => false,
        })
    }

    #[test]
    fn parses_function_declaration() {
        let ast = parse_nib("fn main() { let x: u4 = 1; }").unwrap();
        assert_eq!(ast.rule_name, "program");
        assert!(has_rule(&ast, "fn_decl"));
        assert!(has_rule(&ast, "let_stmt"));
    }

    #[test]
    fn parses_binary_expression() {
        let ast = parse_nib("fn main() { let x: u4 = 1 +% 2; }").unwrap();
        assert!(has_rule(&ast, "expr"));
    }
}
