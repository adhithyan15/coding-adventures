//! Grammar-driven parser for a focused Mermaid flowchart subset.

pub const VERSION: &str = "0.1.0";

use std::collections::HashMap;

use diagram_ir::{
    DiagramDirection, DiagramLabel, DiagramShape, EdgeKind, GraphDiagram, GraphEdge, GraphNode,
};
use grammar_tools::parser_grammar::parse_parser_grammar;
use lexer::token::{Token, TokenType};
use mermaid_lexer::tokenize_mermaid;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

const PARSER_GRAMMAR_SOURCE: &str = include_str!("../../../../grammars/mermaid.grammar");

#[derive(Clone, Debug, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.col, self.message)
    }
}

impl std::error::Error for ParseError {}

#[derive(Clone, Debug, PartialEq)]
struct MermaidNodeRef {
    id: String,
    label: Option<String>,
    shape: Option<DiagramShape>,
}

struct TokenCursor {
    tokens: Vec<Token>,
    index: usize,
}

#[derive(Default)]
struct DiagramBuilder {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    node_indices: HashMap<String, usize>,
}

impl DiagramBuilder {
    fn upsert_node(&mut self, node_ref: MermaidNodeRef) {
        if let Some(index) = self.node_indices.get(&node_ref.id).copied() {
            if let Some(label) = node_ref.label {
                self.nodes[index].label = DiagramLabel::new(label);
            }
            if let Some(shape) = node_ref.shape {
                self.nodes[index].shape = Some(shape);
            }
            return;
        }

        let label = node_ref.label.unwrap_or_else(|| node_ref.id.clone());
        let index = self.nodes.len();
        self.node_indices.insert(node_ref.id.clone(), index);
        self.nodes.push(GraphNode {
            id: node_ref.id,
            label: DiagramLabel::new(label),
            shape: node_ref.shape,
            style: None,
        });
    }
}

impl TokenCursor {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, index: 0 }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.index]
    }

    fn advance(&mut self) -> &Token {
        let token = &self.tokens[self.index];
        if token.type_ != TokenType::Eof {
            self.index += 1;
        }
        token
    }

    fn consume_if(&mut self, name: &str) -> Option<Token> {
        if token_name(self.current()) == name {
            Some(self.advance().clone())
        } else {
            None
        }
    }

    fn skip_terminators(&mut self) {
        while matches!(token_name(self.current()), "NEWLINE" | "SEMICOLON") {
            self.advance();
        }
    }

    fn at_eof(&self) -> bool {
        self.current().type_ == TokenType::Eof
    }

    fn expect_keyword(&mut self, value: &str) -> Result<Token, ParseError> {
        let token = self.current();
        if token.type_ == TokenType::Keyword && token.value == value {
            Ok(self.advance().clone())
        } else {
            Err(token_error(
                token,
                format!("expected Mermaid keyword {value:?}, got {:?}", token.value),
            ))
        }
    }

    fn expect_name_or_node_ref(&self) -> Result<(), ParseError> {
        let token = self.current();
        if token_name(token) == "NAME" {
            Ok(())
        } else {
            Err(token_error(
                token,
                format!("expected NAME or node_ref, got {:?}", token.value),
            ))
        }
    }
}

pub fn create_mermaid_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_mermaid(source);
    let grammar = parse_parser_grammar(PARSER_GRAMMAR_SOURCE)
        .unwrap_or_else(|e| panic!("Failed to parse mermaid.grammar: {e}"));
    GrammarParser::new(tokens, grammar)
}

pub fn parse_mermaid_ast(source: &str) -> Result<GrammarASTNode, ParseError> {
    let mut parser = create_mermaid_parser(source);
    parser.parse().map_err(|e| ParseError {
        message: e.message,
        line: e.token.line,
        col: e.token.column,
    })
}

pub fn parse_to_diagram(source: &str) -> Result<GraphDiagram, ParseError> {
    let mut cursor = TokenCursor::new(tokenize_mermaid(source));
    cursor.skip_terminators();

    let direction = parse_header(&mut cursor)?;
    let mut builder = DiagramBuilder::default();

    cursor.skip_terminators();

    while !cursor.at_eof() {
        lower_statement(&mut cursor, &mut builder)?;
        cursor.skip_terminators();
    }

    Ok(GraphDiagram {
        direction,
        title: None,
        nodes: builder.nodes,
        edges: builder.edges,
    })
}

fn parse_header(cursor: &mut TokenCursor) -> Result<DiagramDirection, ParseError> {
    let token = cursor.current();
    if token.type_ == TokenType::Keyword && token.value == "flowchart" {
        cursor.expect_keyword("flowchart")?;
    } else {
        cursor.expect_keyword("graph")?;
    }

    cursor
        .consume_if("DIRECTION")
        .map(|token| direction_from_token(&token))
        .transpose()?
        .map_or(Ok(DiagramDirection::Tb), Ok)
}

fn lower_statement(
    cursor: &mut TokenCursor,
    builder: &mut DiagramBuilder,
) -> Result<(), ParseError> {
    cursor.expect_name_or_node_ref()?;
    let mut previous = parse_node_ref(cursor)?;
    builder.upsert_node(previous.clone());

    while is_edge_operator(cursor.current()) {
        let (kind, label) = parse_edge_op(cursor)?;
        let target = parse_node_ref(cursor)?;
        builder.upsert_node(target.clone());
        builder.edges.push(GraphEdge {
            id: None,
            from: previous.id.clone(),
            to: target.id.clone(),
            label: label.map(DiagramLabel::new),
            kind,
            style: None,
        });
        previous = target;
    }

    Ok(())
}

fn parse_edge_op(cursor: &mut TokenCursor) -> Result<(EdgeKind, Option<String>), ParseError> {
    let token = cursor.advance().clone();
    let kind = match token_name(&token) {
        "ARROW" => EdgeKind::Directed,
        "LINE" => EdgeKind::Undirected,
        other => {
            return Err(token_error(
                &token,
                format!("unsupported Mermaid edge operator: {other}"),
            ));
        }
    };

    let label = cursor
        .consume_if("EDGE_LABEL")
        .map(|token| strip_edge_label(&token.value));

    Ok((kind, label))
}

fn parse_node_ref(cursor: &mut TokenCursor) -> Result<MermaidNodeRef, ParseError> {
    let token = cursor.current();
    if token_name(token) != "NAME" {
        return Err(token_error(token, "missing node id"));
    }

    let id = cursor.advance().value.clone();
    let mut result = MermaidNodeRef {
        id,
        label: None,
        shape: None,
    };

    if let Some(token) = cursor
        .consume_if("CIRCLE")
        .or_else(|| cursor.consume_if("ROUND"))
        .or_else(|| cursor.consume_if("RECT"))
        .or_else(|| cursor.consume_if("DIAMOND"))
    {
        let (label, shape) = parse_node_shape_token(&token)?;
        result.label = Some(label);
        result.shape = Some(shape);
    }

    Ok(result)
}

fn parse_node_shape_token(token: &Token) -> Result<(String, DiagramShape), ParseError> {
    match token_name(token) {
        "RECT" => Ok((strip_wrapped(&token.value, 1, 1), DiagramShape::Rect)),
        "ROUND" => Ok((strip_wrapped(&token.value, 1, 1), DiagramShape::RoundedRect)),
        "CIRCLE" => Ok((strip_wrapped(&token.value, 2, 2), DiagramShape::Ellipse)),
        "DIAMOND" => Ok((strip_wrapped(&token.value, 1, 1), DiagramShape::Diamond)),
        other => Err(token_error(
            token,
            format!("unsupported Mermaid node shape: {other}"),
        )),
    }
}

fn direction_from_token(token: &Token) -> Result<DiagramDirection, ParseError> {
    match token.value.as_str() {
        "TB" | "TD" => Ok(DiagramDirection::Tb),
        "BT" => Ok(DiagramDirection::Bt),
        "LR" => Ok(DiagramDirection::Lr),
        "RL" => Ok(DiagramDirection::Rl),
        other => Err(token_error(
            token,
            format!("unsupported Mermaid direction: {other}"),
        )),
    }
}

fn strip_wrapped(raw: &str, prefix: usize, suffix: usize) -> String {
    raw[prefix..raw.len() - suffix].trim().to_string()
}

fn strip_edge_label(raw: &str) -> String {
    strip_wrapped(raw, 1, 1)
}

fn token_error(token: &Token, message: impl Into<String>) -> ParseError {
    ParseError {
        message: message.into(),
        line: token.line,
        col: token.column,
    }
}

fn is_edge_operator(token: &Token) -> bool {
    matches!(token_name(token), "ARROW" | "LINE")
}

fn token_name(token: &Token) -> &str {
    token
        .type_name
        .as_deref()
        .unwrap_or_else(|| match token.type_ {
            TokenType::Name => "NAME",
            TokenType::Keyword => "KEYWORD",
            TokenType::Newline => "NEWLINE",
            TokenType::Semicolon => "SEMICOLON",
            TokenType::Eof => "EOF",
            _ => "TOKEN",
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_ir::{DiagramDirection, DiagramShape, EdgeKind};

    fn find_node<'a>(diagram: &'a GraphDiagram, id: &str) -> &'a GraphNode {
        diagram
            .nodes
            .iter()
            .find(|node| node.id == id)
            .expect("missing node")
    }

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn parses_minimal_flowchart() {
        let diagram = parse_to_diagram("flowchart LR\nA --> B\n").unwrap();
        assert_eq!(diagram.direction, DiagramDirection::Lr);
        assert_eq!(diagram.nodes.len(), 2);
        assert_eq!(diagram.edges.len(), 1);
        assert_eq!(diagram.edges[0].from, "A");
        assert_eq!(diagram.edges[0].to, "B");
        assert_eq!(diagram.edges[0].kind, EdgeKind::Directed);
    }

    #[test]
    fn parses_graph_keyword_and_undirected_edge() {
        let diagram = parse_to_diagram("graph RL\nA --- B\n").unwrap();
        assert_eq!(diagram.direction, DiagramDirection::Rl);
        assert_eq!(diagram.edges[0].kind, EdgeKind::Undirected);
    }

    #[test]
    fn parses_shapes_and_labels() {
        let diagram = parse_to_diagram(
            "flowchart TB\nA[Start] --> B{Ship?}\nB -->|yes| C((Done))\nD(Retry)\n",
        )
        .unwrap();

        assert_eq!(find_node(&diagram, "A").label.text, "Start");
        assert_eq!(find_node(&diagram, "A").shape, Some(DiagramShape::Rect));
        assert_eq!(find_node(&diagram, "B").label.text, "Ship?");
        assert_eq!(find_node(&diagram, "B").shape, Some(DiagramShape::Diamond));
        assert_eq!(find_node(&diagram, "C").shape, Some(DiagramShape::Ellipse));
        assert_eq!(
            find_node(&diagram, "D").shape,
            Some(DiagramShape::RoundedRect)
        );
        assert_eq!(
            diagram.edges[1].label.as_ref().map(|l| l.text.as_str()),
            Some("yes")
        );
    }

    #[test]
    fn edge_chains_expand() {
        let diagram = parse_to_diagram("flowchart LR\nA --> B --> C[Done]\n").unwrap();
        assert_eq!(diagram.nodes.len(), 3);
        assert_eq!(diagram.edges.len(), 2);
        assert_eq!(diagram.edges[0].from, "A");
        assert_eq!(diagram.edges[0].to, "B");
        assert_eq!(diagram.edges[1].from, "B");
        assert_eq!(diagram.edges[1].to, "C");
        assert_eq!(find_node(&diagram, "C").label.text, "Done");
    }

    #[test]
    fn comments_and_semicolons_are_supported() {
        let diagram =
            parse_to_diagram("%% header\nflowchart TD; A[Start]; A --> B[Finish]\n").unwrap();
        assert_eq!(diagram.direction, DiagramDirection::Tb);
        assert_eq!(diagram.nodes.len(), 2);
        assert_eq!(diagram.edges.len(), 1);
    }

    #[test]
    fn invalid_source_reports_location() {
        let err = parse_to_diagram("flowchart LR\nA -->\n").unwrap_err();
        assert!(err.line >= 2);
        assert!(err.col >= 1);
    }
}
