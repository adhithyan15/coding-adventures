//! Grammar-driven parser for a focused Mermaid flowchart subset.

pub const VERSION: &str = "0.1.0";

use std::collections::HashMap;

use diagram_ir::{
    DiagramDirection, DiagramLabel, DiagramShape, EdgeKind, GraphDiagram, GraphEdge, GraphNode,
};
use grammar_tools::parser_grammar::parse_parser_grammar;
use lexer::token::{Token, TokenType};
use mermaid_lexer::tokenize_mermaid;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode, GrammarParser};

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
    let ast = parse_mermaid_ast(source)?;
    lower_document(&ast)
}

fn lower_document(document: &GrammarASTNode) -> Result<GraphDiagram, ParseError> {
    let header = child_nodes_named(document, "header")
        .into_iter()
        .next()
        .ok_or_else(|| node_error(document, "missing Mermaid header"))?;

    let direction = parse_header(header)?;
    let mut builder = DiagramBuilder::default();

    for statement in child_nodes_named(document, "statement") {
        lower_statement(statement, &mut builder)?;
    }

    Ok(GraphDiagram {
        direction,
        title: None,
        nodes: builder.nodes,
        edges: builder.edges,
    })
}

fn lower_statement(
    statement: &GrammarASTNode,
    builder: &mut DiagramBuilder,
) -> Result<(), ParseError> {
    for child in child_nodes(statement) {
        match child.rule_name.as_str() {
            "node_stmt" => {
                let node_ref = child_nodes_named(child, "node_ref")
                    .into_iter()
                    .next()
                    .ok_or_else(|| node_error(child, "missing node reference"))?;
                builder.upsert_node(parse_node_ref(node_ref)?);
                return Ok(());
            }
            "edge_stmt" => {
                let mut node_refs = child_nodes_named(child, "node_ref").into_iter();
                let start_node = node_refs
                    .next()
                    .ok_or_else(|| node_error(child, "missing edge source"))?;
                let mut previous = parse_node_ref(start_node)?;
                builder.upsert_node(previous.clone());

                for segment in child_nodes_named(child, "edge_segment") {
                    let (kind, label, target) = parse_edge_segment(segment)?;
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
                return Ok(());
            }
            _ => {}
        }
    }

    Err(node_error(statement, "unsupported Mermaid statement"))
}

fn parse_header(header: &GrammarASTNode) -> Result<DiagramDirection, ParseError> {
    let direction = descendant_tokens(header)
        .into_iter()
        .find(|token| token_name(token) == "DIRECTION")
        .map(|token| direction_from_value(&token.value))
        .transpose()?
        .unwrap_or(DiagramDirection::Tb);

    Ok(direction)
}

fn parse_edge_segment(
    segment: &GrammarASTNode,
) -> Result<(EdgeKind, Option<String>, MermaidNodeRef), ParseError> {
    let kind = descendant_nodes_named(segment, "edge_op")
        .into_iter()
        .next()
        .ok_or_else(|| node_error(segment, "missing edge operator"))
        .and_then(parse_edge_kind)?;

    let label = descendant_tokens(segment)
        .into_iter()
        .find(|token| token_name(token) == "EDGE_LABEL")
        .map(|token| strip_edge_label(&token.value));

    let node_ref = descendant_nodes_named(segment, "node_ref")
        .into_iter()
        .next()
        .ok_or_else(|| node_error(segment, "missing edge target"))
        .and_then(parse_node_ref)?;

    Ok((kind, label, node_ref))
}

fn parse_edge_kind(edge_op: &GrammarASTNode) -> Result<EdgeKind, ParseError> {
    let token = descendant_tokens(edge_op)
        .into_iter()
        .next()
        .ok_or_else(|| node_error(edge_op, "missing edge operator token"))?;

    match token_name(token) {
        "ARROW" => Ok(EdgeKind::Directed),
        "LINE" => Ok(EdgeKind::Undirected),
        other => Err(ParseError {
            message: format!("unsupported Mermaid edge operator: {other}"),
            line: token.line,
            col: token.column,
        }),
    }
}

fn parse_node_ref(node_ref: &GrammarASTNode) -> Result<MermaidNodeRef, ParseError> {
    let id_token = descendant_tokens(node_ref)
        .into_iter()
        .find(|token| token_name(token) == "NAME")
        .ok_or_else(|| node_error(node_ref, "missing node id"))?;

    let mut result = MermaidNodeRef {
        id: id_token.value.clone(),
        label: None,
        shape: None,
    };

    if let Some(shape_node) = descendant_nodes_named(node_ref, "node_shape").into_iter().next() {
        let (label, shape) = parse_node_shape(shape_node)?;
        result.label = Some(label);
        result.shape = Some(shape);
    }

    Ok(result)
}

fn parse_node_shape(node_shape: &GrammarASTNode) -> Result<(String, DiagramShape), ParseError> {
    let token = descendant_tokens(node_shape)
        .into_iter()
        .next()
        .ok_or_else(|| node_error(node_shape, "missing node shape token"))?;

    match token_name(token) {
        "RECT" => Ok((strip_wrapped(&token.value, 1, 1), DiagramShape::Rect)),
        "ROUND" => Ok((strip_wrapped(&token.value, 1, 1), DiagramShape::RoundedRect)),
        "CIRCLE" => Ok((strip_wrapped(&token.value, 2, 2), DiagramShape::Ellipse)),
        "DIAMOND" => Ok((strip_wrapped(&token.value, 1, 1), DiagramShape::Diamond)),
        other => Err(ParseError {
            message: format!("unsupported Mermaid node shape: {other}"),
            line: token.line,
            col: token.column,
        }),
    }
}

fn direction_from_value(value: &str) -> Result<DiagramDirection, ParseError> {
    match value {
        "TB" | "TD" => Ok(DiagramDirection::Tb),
        "BT" => Ok(DiagramDirection::Bt),
        "LR" => Ok(DiagramDirection::Lr),
        "RL" => Ok(DiagramDirection::Rl),
        other => Err(ParseError {
            message: format!("unsupported Mermaid direction: {other}"),
            line: 1,
            col: 1,
        }),
    }
}

fn strip_wrapped(raw: &str, prefix: usize, suffix: usize) -> String {
    raw[prefix..raw.len() - suffix].trim().to_string()
}

fn strip_edge_label(raw: &str) -> String {
    strip_wrapped(raw, 1, 1)
}

fn node_error(node: &GrammarASTNode, message: impl Into<String>) -> ParseError {
    ParseError {
        message: message.into(),
        line: node.start_line.unwrap_or(1),
        col: node.start_column.unwrap_or(1),
    }
}

fn child_nodes<'a>(node: &'a GrammarASTNode) -> Vec<&'a GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|child| match child {
            ASTNodeOrToken::Node(child_node) => Some(child_node),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

fn child_nodes_named<'a>(node: &'a GrammarASTNode, name: &str) -> Vec<&'a GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|child| match child {
            ASTNodeOrToken::Node(child_node) if child_node.rule_name == name => Some(child_node),
            _ => None,
        })
        .collect()
}

fn descendant_nodes_named<'a>(node: &'a GrammarASTNode, name: &str) -> Vec<&'a GrammarASTNode> {
    let mut matches = Vec::new();
    for child in &node.children {
        if let ASTNodeOrToken::Node(child_node) = child {
            if child_node.rule_name == name {
                matches.push(child_node);
            }
            matches.extend(descendant_nodes_named(child_node, name));
        }
    }
    matches
}

fn descendant_tokens(node: &GrammarASTNode) -> Vec<&Token> {
    let mut tokens = Vec::new();
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(token) => tokens.push(token),
            ASTNodeOrToken::Node(child_node) => tokens.extend(descendant_tokens(child_node)),
        }
    }
    tokens
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
