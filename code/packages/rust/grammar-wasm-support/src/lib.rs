use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use serde::Serialize;

#[derive(Serialize)]
struct JsonToken {
    type_name: String,
    value: String,
    line: usize,
    column: usize,
    flags: Option<u32>,
}

#[derive(Serialize)]
struct JsonAstNode {
    rule_name: String,
    children: Vec<JsonAstChild>,
    start_line: Option<usize>,
    start_column: Option<usize>,
    end_line: Option<usize>,
    end_column: Option<usize>,
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
enum JsonAstChild {
    Node(JsonAstNode),
    Token(JsonToken),
}

fn token_to_json(token: Token) -> JsonToken {
    JsonToken {
        type_name: token.effective_type_name().to_string(),
        value: token.value,
        line: token.line,
        column: token.column,
        flags: token.flags,
    }
}

fn ast_to_json(node: GrammarASTNode) -> JsonAstNode {
    JsonAstNode {
        rule_name: node.rule_name,
        children: node
            .children
            .into_iter()
            .map(|child| match child {
                ASTNodeOrToken::Node(child_node) => JsonAstChild::Node(ast_to_json(child_node)),
                ASTNodeOrToken::Token(token) => JsonAstChild::Token(token_to_json(token)),
            })
            .collect(),
        start_line: node.start_line,
        start_column: node.start_column,
        end_line: node.end_line,
        end_column: node.end_column,
    }
}

pub fn tokens_to_json_string(tokens: Vec<Token>) -> Result<String, serde_json::Error> {
    let payload = tokens.into_iter().map(token_to_json).collect::<Vec<_>>();
    serde_json::to_string(&payload)
}

pub fn ast_to_json_string(ast: GrammarASTNode) -> Result<String, serde_json::Error> {
    serde_json::to_string(&ast_to_json(ast))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    #[test]
    fn serializes_tokens_to_json() {
        let json = tokens_to_json_string(vec![Token {
            type_: TokenType::Name,
            value: "answer".to_string(),
            line: 1,
            column: 1,
            type_name: Some("NAME".to_string()),
            flags: None,
        }])
        .unwrap();

        assert!(json.contains("\"type_name\":\"NAME\""));
        assert!(json.contains("\"value\":\"answer\""));
    }

    #[test]
    fn serializes_ast_to_json() {
        let ast = GrammarASTNode {
            rule_name: "root".to_string(),
            children: vec![ASTNodeOrToken::Token(Token {
                type_: TokenType::Name,
                value: "node".to_string(),
                line: 1,
                column: 1,
                type_name: Some("NAME".to_string()),
                flags: None,
            })],
            start_line: Some(1),
            start_column: Some(1),
            end_line: Some(1),
            end_column: Some(4),
        };

        let json = ast_to_json_string(ast).unwrap();
        assert!(json.contains("\"rule_name\":\"root\""));
        assert!(json.contains("\"kind\":\"token\""));
    }
}
