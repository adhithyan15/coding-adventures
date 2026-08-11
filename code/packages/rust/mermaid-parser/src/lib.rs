//! Grammar-driven parser and compatibility dispatcher for Mermaid diagrams.

// This file hand-parses many `starts_with(...)` / slice-index prefix strips
// where the prefix and the stripped remainder need slightly different handling;
// rewriting every one as `strip_prefix` hurts readability here, so we opt out
// of the lint file-wide.
#![allow(clippy::manual_strip)]

pub const VERSION: &str = "0.53.0";
pub const MERMAID_COMPATIBILITY_BASELINE: &str = "11.16.1";

use std::collections::HashMap;

use diagram_ir::{
    DiagramDirection, DiagramLabel, DiagramShape, DiagramStyle, EdgeKind, GraphDiagram, GraphEdge,
    GraphNode,
};
use grammar_tools::parser_grammar::parse_parser_grammar;
use lexer::token::{Token, TokenType};
use mermaid_lexer::{
    tokenize_mermaid, tokenize_mermaid_c4, tokenize_mermaid_er, tokenize_mermaid_gitgraph,
    tokenize_mermaid_pie, tokenize_mermaid_sankey, tokenize_mermaid_sequence,
    tokenize_mermaid_state,
};
use parser::grammar_parser::{GrammarASTNode, GrammarParser, DEFAULT_MAX_RULE_DEPTH};

const PARSER_GRAMMAR_SOURCE: &str = include_str!("../../../../grammars/mermaid/mermaid.grammar");
const PIE_PARSER_GRAMMAR_SOURCE: &str = include_str!("../../../../grammars/mermaid/pie.grammar");
const SANKEY_PARSER_GRAMMAR_SOURCE: &str =
    include_str!("../../../../grammars/mermaid/sankey.grammar");
const GITGRAPH_PARSER_GRAMMAR_SOURCE: &str =
    include_str!("../../../../grammars/mermaid/gitgraph.grammar");
const ER_PARSER_GRAMMAR_SOURCE: &str = include_str!("../../../../grammars/mermaid/er.grammar");
const C4_PARSER_GRAMMAR_SOURCE: &str = include_str!("../../../../grammars/mermaid/c4.grammar");
const SEQUENCE_PARSER_GRAMMAR_SOURCE: &str =
    include_str!("../../../../grammars/mermaid/sequence.grammar");
const STATE_PARSER_GRAMMAR_SOURCE: &str =
    include_str!("../../../../grammars/mermaid/state.grammar");

/// Recursion-depth cap for the Mermaid [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] for why this guard exists at all (deep
/// recursion through `parse_rule` can overflow the *native* thread stack —
/// an uncatchable process abort — before this crate's own callers get a
/// chance to report anything). Before this constant was applied,
/// `create_mermaid_parser` never called `with_max_depth` at all.
///
/// Unlike every other crate in this hardening pass, `mermaid.grammar`
/// (read in full: 39 lines) has **no self-referential production at all**
/// — `document -> statement -> (edge_stmt | node_stmt)`, `edge_stmt ->
/// edge_segment { edge_segment }`, `node_ref -> node_shape`: every
/// production either bottoms out in tokens or repeats flatly via EBNF
/// `{ x }`, which costs zero native stack regardless of width (confirmed
/// in `reduce-parser`'s own `MAX_RULE_DEPTH` doc comment via a throwaway
/// probe grammar). There is no adversarial input shape that can drive this
/// specific grammar's own recursion arbitrarily deep. `DEFAULT_MAX_RULE_DEPTH`
/// (128) is used here as pure defense-in-depth — a cheap, harmless backstop
/// consistent with every other `GrammarParser` construction site in this
/// repo now calling `with_max_depth`, not a response to a measured risk.
const MAX_RULE_DEPTH: usize = DEFAULT_MAX_RULE_DEPTH;

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
    GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH)
}

pub fn parse_mermaid_ast(source: &str) -> Result<GrammarASTNode, ParseError> {
    let mut parser = create_mermaid_parser(source);
    parser.parse().map_err(|e| ParseError {
        message: e.message,
        line: e.token.line,
        col: e.token.column,
    })
}

pub fn parse_mermaid_pie_ast(source: &str) -> Result<GrammarASTNode, ParseError> {
    let tokens = tokenize_mermaid_pie(source);
    let grammar = parse_parser_grammar(PIE_PARSER_GRAMMAR_SOURCE)
        .unwrap_or_else(|e| panic!("Failed to parse pie.grammar: {e}"));
    let mut parser = GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH);
    parser.parse().map_err(|e| ParseError {
        message: e.message,
        line: e.token.line,
        col: e.token.column,
    })
}

pub fn parse_mermaid_sankey_ast(source: &str) -> Result<GrammarASTNode, ParseError> {
    let tokens = tokenize_mermaid_sankey(source);
    let grammar = parse_parser_grammar(SANKEY_PARSER_GRAMMAR_SOURCE)
        .unwrap_or_else(|e| panic!("Failed to parse sankey.grammar: {e}"));
    let mut parser = GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH);
    parser.parse().map_err(|e| ParseError {
        message: e.message,
        line: e.token.line,
        col: e.token.column,
    })
}

pub fn parse_mermaid_gitgraph_ast(source: &str) -> Result<GrammarASTNode, ParseError> {
    let tokens = tokenize_mermaid_gitgraph(source);
    let grammar = parse_parser_grammar(GITGRAPH_PARSER_GRAMMAR_SOURCE)
        .unwrap_or_else(|e| panic!("Failed to parse gitgraph.grammar: {e}"));
    let mut parser = GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH);
    parser.parse().map_err(|e| ParseError {
        message: e.message,
        line: e.token.line,
        col: e.token.column,
    })
}

pub fn parse_mermaid_er_ast(source: &str) -> Result<GrammarASTNode, ParseError> {
    let tokens = tokenize_mermaid_er(source);
    let grammar = parse_parser_grammar(ER_PARSER_GRAMMAR_SOURCE)
        .unwrap_or_else(|e| panic!("Failed to parse er.grammar: {e}"));
    let mut parser = GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH);
    parser.parse().map_err(|e| ParseError {
        message: e.message,
        line: e.token.line,
        col: e.token.column,
    })
}

pub fn parse_mermaid_c4_ast(source: &str) -> Result<GrammarASTNode, ParseError> {
    let tokens = tokenize_mermaid_c4(source);
    let grammar = parse_parser_grammar(C4_PARSER_GRAMMAR_SOURCE)
        .unwrap_or_else(|e| panic!("Failed to parse c4.grammar: {e}"));
    let mut parser = GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH);
    parser.parse().map_err(|e| ParseError {
        message: e.message,
        line: e.token.line,
        col: e.token.column,
    })
}

pub fn parse_mermaid_sequence_ast(source: &str) -> Result<GrammarASTNode, ParseError> {
    let preprocessed = preprocess_mermaid_source(source)?;
    let tokens = tokenize_mermaid_sequence(&preprocessed.source);
    let grammar = parse_parser_grammar(SEQUENCE_PARSER_GRAMMAR_SOURCE)
        .unwrap_or_else(|e| panic!("Failed to parse sequence.grammar: {e}"));
    let mut parser = GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH);
    parser.parse().map_err(|e| ParseError {
        message: e.message,
        line: e.token.line,
        col: e.token.column,
    })
}

pub fn parse_mermaid_state_ast(source: &str) -> Result<GrammarASTNode, ParseError> {
    let preprocessed = preprocess_mermaid_source(source)?;
    let tokens = tokenize_mermaid_state(&preprocessed.source);
    let grammar = parse_parser_grammar(STATE_PARSER_GRAMMAR_SOURCE)
        .unwrap_or_else(|e| panic!("Failed to parse state.grammar: {e}"));
    let mut parser = GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH);
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
    token.type_name.as_deref().unwrap_or(match token.type_ {
        TokenType::Name => "NAME",
        TokenType::Number => "NUMBER",
        TokenType::String => "STRING",
        TokenType::Keyword => "KEYWORD",
        TokenType::Plus => "PLUS",
        TokenType::Minus => "MINUS",
        TokenType::Colon => "COLON",
        TokenType::Comma => "COMMA",
        TokenType::Equals => "EQUALS",
        TokenType::LParen => "LPAREN",
        TokenType::RParen => "RPAREN",
        TokenType::LBrace => "LBRACE",
        TokenType::RBrace => "RBRACE",
        TokenType::LBracket => "LBRACKET",
        TokenType::RBracket => "RBRACKET",
        TokenType::Newline => "NEWLINE",
        TokenType::Semicolon => "SEMICOLON",
        TokenType::Eof => "EOF",
        _ => "TOKEN",
    })
}

// ============================================================================
// DG04 — Extended Mermaid parsers for Chart, Structural, and Temporal families
// ============================================================================

use diagram_ir::{
    Axis, AxisKind, ChartDiagram, ChartKind, ChartOrientation, ChartSeries, Compartment,
    CompartmentKind, GanttDiagram, GanttSection, GanttTask, GitBranch, GitCommitType, GitDiagram,
    GitEvent, PieSlice, RelKind, SankeyFlow, SankeyNode, SequenceArrowhead, SequenceBlockKind,
    SequenceCentralConnection, SequenceDiagram, SequenceEvent, SequenceLineStyle, SequenceLink,
    SequenceNotePlacement, SequenceParticipant, SequenceParticipantGroup, SequenceParticipantKind,
    SequenceProperty, SequenceTextWrap, SeriesKind, StructuralDiagram, StructuralGroup,
    StructuralKind, StructuralNode, StructuralNodeKind, StructuralRelationship, TaskStart,
    TaskStatus, TemporalBody, TemporalDiagram, TemporalKind,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MermaidDiagramType {
    Flowchart,
    Sequence,
    Class,
    State,
    Er,
    Journey,
    Gantt,
    Pie,
    Quadrant,
    Requirement,
    GitGraph,
    C4,
    Mindmap,
    Timeline,
    Sankey,
    XyChart,
    Block,
    Packet,
    Kanban,
    Architecture,
    Radar,
    EventModeling,
    Treemap,
    Venn,
    Ishikawa,
    Wardley,
    Cynefin,
    TreeView,
    Swimlane,
    Railroad,
    Info,
    ZenUml,
}

impl MermaidDiagramType {
    pub fn canonical_id(self) -> &'static str {
        match self {
            Self::Flowchart => "flowchart",
            Self::Sequence => "sequence",
            Self::Class => "class",
            Self::State => "state",
            Self::Er => "er",
            Self::Journey => "journey",
            Self::Gantt => "gantt",
            Self::Pie => "pie",
            Self::Quadrant => "quadrant",
            Self::Requirement => "requirement",
            Self::GitGraph => "gitgraph",
            Self::C4 => "c4",
            Self::Mindmap => "mindmap",
            Self::Timeline => "timeline",
            Self::Sankey => "sankey",
            Self::XyChart => "xychart",
            Self::Block => "block",
            Self::Packet => "packet",
            Self::Kanban => "kanban",
            Self::Architecture => "architecture",
            Self::Radar => "radar",
            Self::EventModeling => "eventmodeling",
            Self::Treemap => "treemap",
            Self::Venn => "venn",
            Self::Ishikawa => "ishikawa",
            Self::Wardley => "wardley",
            Self::Cynefin => "cynefin",
            Self::TreeView => "treeview",
            Self::Swimlane => "swimlane",
            Self::Railroad => "railroad",
            Self::Info => "info",
            Self::ZenUml => "zenuml",
        }
    }

    pub fn has_native_pipeline(self) -> bool {
        matches!(
            self,
            Self::Flowchart
                | Self::Class
                | Self::C4
                | Self::Er
                | Self::Gantt
                | Self::GitGraph
                | Self::Pie
                | Self::Sequence
                | Self::State
                | Self::Sankey
                | Self::XyChart
        )
    }
}

/// Union of all Mermaid diagram variants that `parse_any_mermaid` can return.
pub enum MermaidDiagram {
    Graph(GraphDiagram),
    Chart(ChartDiagram),
    Sequence(SequenceDiagram),
    Structural(StructuralDiagram),
    Temporal(TemporalDiagram),
}

/// Detect a Mermaid 11.16.1 diagram family from its header.
///
/// Detection also skips leading YAML front matter, Mermaid directives,
/// comments, and blank lines. A recognized family may still be reported as
/// unsupported by [`parse_any_mermaid`] while its semantic IR is implemented.
pub fn detect_mermaid_type(source: &str) -> Result<MermaidDiagramType, ParseError> {
    let first = first_keyword(source);
    if first.eq_ignore_ascii_case("sequenceDiagram") {
        return Ok(MermaidDiagramType::Sequence);
    }
    let diagram_type = match first.as_str() {
        "flowchart" | "graph" | "flowchart-elk" => MermaidDiagramType::Flowchart,
        "classDiagram" | "classDiagram-v2" => MermaidDiagramType::Class,
        "stateDiagram" | "stateDiagram-v2" => MermaidDiagramType::State,
        "erDiagram" => MermaidDiagramType::Er,
        "journey" => MermaidDiagramType::Journey,
        "gantt" => MermaidDiagramType::Gantt,
        "pie" => MermaidDiagramType::Pie,
        "quadrantChart" => MermaidDiagramType::Quadrant,
        "requirement" | "requirementDiagram" => MermaidDiagramType::Requirement,
        "gitGraph" => MermaidDiagramType::GitGraph,
        "C4Context" | "C4Container" | "C4Component" | "C4Dynamic" | "C4Deployment" => {
            MermaidDiagramType::C4
        }
        "mindmap" => MermaidDiagramType::Mindmap,
        "timeline" => MermaidDiagramType::Timeline,
        "sankey" | "sankey-beta" => MermaidDiagramType::Sankey,
        "xychart" | "xychart-beta" => MermaidDiagramType::XyChart,
        "block" | "block-beta" => MermaidDiagramType::Block,
        "packet" | "packet-beta" => MermaidDiagramType::Packet,
        "kanban" => MermaidDiagramType::Kanban,
        "architecture" | "architecture-beta" => MermaidDiagramType::Architecture,
        "radar-beta" => MermaidDiagramType::Radar,
        "eventmodeling" => MermaidDiagramType::EventModeling,
        "treemap" => MermaidDiagramType::Treemap,
        "venn-beta" => MermaidDiagramType::Venn,
        "ishikawa" | "ishikawa-beta" => MermaidDiagramType::Ishikawa,
        "wardley-beta" => MermaidDiagramType::Wardley,
        "cynefin-beta" => MermaidDiagramType::Cynefin,
        "treeView-beta" => MermaidDiagramType::TreeView,
        "swimlane-beta" => MermaidDiagramType::Swimlane,
        "railroad-beta" | "railroad-ebnf-beta" | "railroad-abnf-beta" | "railroad-peg-beta" => {
            MermaidDiagramType::Railroad
        }
        "info" => MermaidDiagramType::Info,
        "zenuml" => MermaidDiagramType::ZenUml,
        other => {
            return Err(ParseError {
                message: format!("Unknown Mermaid diagram type: {other:?}"),
                line: 1,
                col: 1,
            });
        }
    };

    Ok(diagram_type)
}

/// Dispatch a recognized Mermaid family to its semantic parser.
pub fn parse_any_mermaid(source: &str) -> Result<MermaidDiagram, ParseError> {
    let diagram_type = detect_mermaid_type(source)?;
    match diagram_type {
        MermaidDiagramType::Flowchart => parse_to_diagram(source).map(MermaidDiagram::Graph),
        MermaidDiagramType::Class => parse_class_diagram(source).map(MermaidDiagram::Structural),
        MermaidDiagramType::C4 => parse_c4_diagram(source).map(MermaidDiagram::Structural),
        MermaidDiagramType::Er => parse_er_diagram(source).map(MermaidDiagram::Structural),
        MermaidDiagramType::XyChart => parse_xychart(source).map(MermaidDiagram::Chart),
        MermaidDiagramType::Pie => parse_pie(source).map(MermaidDiagram::Chart),
        MermaidDiagramType::Sequence => {
            parse_sequence_diagram(source).map(MermaidDiagram::Sequence)
        }
        MermaidDiagramType::State => parse_state_diagram(source).map(MermaidDiagram::Graph),
        MermaidDiagramType::Sankey => parse_sankey(source).map(MermaidDiagram::Chart),
        MermaidDiagramType::GitGraph => parse_gitgraph(source).map(|git| {
            MermaidDiagram::Temporal(TemporalDiagram {
                kind: TemporalKind::Git,
                title: None,
                body: TemporalBody::Git(git),
            })
        }),
        MermaidDiagramType::Gantt => parse_gantt(source).map(|g| {
            MermaidDiagram::Temporal(TemporalDiagram {
                kind: TemporalKind::Gantt,
                title: None,
                body: TemporalBody::Gantt(g),
            })
        }),
        unsupported => Err(ParseError {
            message: format!(
                "Mermaid {} diagram family {:?} is recognized but not implemented",
                MERMAID_COMPATIBILITY_BASELINE,
                unsupported.canonical_id()
            ),
            line: 1,
            col: 1,
        }),
    }
}

fn first_keyword(source: &str) -> String {
    let mut in_front_matter = false;
    let mut can_start_front_matter = true;
    let mut in_directive = false;

    for line in source.lines() {
        let trimmed = line.trim();

        if can_start_front_matter && trimmed.is_empty() {
            continue;
        }
        if can_start_front_matter && trimmed == "---" {
            in_front_matter = true;
            can_start_front_matter = false;
            continue;
        }
        if in_front_matter {
            if trimmed == "---" {
                in_front_matter = false;
            }
            continue;
        }

        can_start_front_matter = false;
        if in_directive {
            if trimmed.contains("}%%") {
                in_directive = false;
            }
            continue;
        }
        if trimmed.starts_with("%%{") {
            in_directive = !trimmed.contains("}%%");
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }

        return trimmed
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_end_matches(':')
            .to_string();
    }
    String::new()
}

// ── classDiagram parser ───────────────────────────────────────────────────

/// Parse a Mermaid `classDiagram` block into a `StructuralDiagram`.
///
/// Handles:
/// ```text
/// classDiagram
///   class Animal { +name: String; +speak() void }
///   class Dog
///   Animal <|-- Dog : extends
/// ```
pub fn parse_class_diagram(source: &str) -> Result<StructuralDiagram, ParseError> {
    let mut nodes: Vec<StructuralNode> = Vec::new();
    let mut relationships: Vec<StructuralRelationship> = Vec::new();
    let mut title: Option<String> = None;

    let mut lines = source.lines().peekable();

    // Skip the `classDiagram` header line.
    for line in lines.by_ref() {
        let t = line.trim();
        if t == "classDiagram" {
            break;
        }
        if t.starts_with("%%") || t.is_empty() {
            continue;
        }
        if t.starts_with("title") {
            title = Some(t.trim_start_matches("title").trim().to_string());
        }
    }

    for line in lines {
        let t = line.trim();
        if t.is_empty() || t.starts_with("%%") {
            continue;
        }

        if t.starts_with("class ") {
            let rest = t[6..].trim();
            let (id_str, body_str): (String, Option<String>) = if let Some(pos) = rest.find('{') {
                (
                    rest[..pos].trim().to_string(),
                    Some(rest[pos + 1..].trim_end_matches('}').to_string()),
                )
            } else {
                (rest.to_string(), None)
            };
            let id = id_str.trim().to_string();

            let mut compartments: Vec<Compartment> = Vec::new();
            if let Some(body) = body_str.as_deref() {
                let entries: Vec<String> = body
                    .split(';')
                    .map(|e| e.trim().to_string())
                    .filter(|e| !e.is_empty())
                    .collect();
                if !entries.is_empty() {
                    // Heuristic: entries with `()` are methods, otherwise fields.
                    let fields: Vec<String> = entries
                        .iter()
                        .filter(|e| !e.contains('('))
                        .map(|e| strip_visibility(e))
                        .collect();
                    let methods: Vec<String> = entries
                        .iter()
                        .filter(|e| e.contains('('))
                        .map(|e| strip_visibility(e))
                        .collect();
                    if !fields.is_empty() {
                        compartments.push(Compartment {
                            kind: CompartmentKind::Fields,
                            entries: fields,
                        });
                    }
                    if !methods.is_empty() {
                        compartments.push(Compartment {
                            kind: CompartmentKind::Methods,
                            entries: methods,
                        });
                    }
                }
            }

            // Update existing node or create a new one.
            if let Some(existing) = nodes.iter_mut().find(|n| n.id == id) {
                if !compartments.is_empty() {
                    existing.compartments = compartments;
                }
            } else {
                nodes.push(StructuralNode {
                    id: id.clone(),
                    label: id,
                    stereotype: None,
                    node_kind: StructuralNodeKind::Class,
                    compartments,
                    parent_group: None,
                });
            }
        } else if let Some(rel) = parse_class_relationship(t) {
            // Make sure both nodes exist.
            for id in [&rel.from, &rel.to] {
                if !nodes.iter().any(|n| &n.id == id) {
                    nodes.push(StructuralNode {
                        id: id.clone(),
                        label: id.clone(),
                        stereotype: None,
                        node_kind: StructuralNodeKind::Class,
                        compartments: vec![],
                        parent_group: None,
                    });
                }
            }
            relationships.push(rel);
        }
    }

    Ok(StructuralDiagram {
        kind: StructuralKind::Class,
        title,
        nodes,
        groups: vec![],
        relationships,
    })
}

fn strip_visibility(s: &str) -> String {
    let s = s.trim();
    if s.starts_with(['+', '-', '#', '~']) {
        s[1..].trim().to_string()
    } else {
        s.to_string()
    }
}

#[allow(dead_code)] // retained as API surface / scaffolding
#[allow(clippy::ptr_arg)] // dead code; signature kept as-is
fn strip_visibility_owned(s: &String) -> String {
    strip_visibility(s.as_str())
}

/// Parse a Mermaid class relationship line like `Animal <|-- Dog : extends`.
fn parse_class_relationship(line: &str) -> Option<StructuralRelationship> {
    // Try each arrow pattern.
    let arrows: &[(&str, RelKind)] = &[
        ("<|--", RelKind::Inheritance),
        ("<|..", RelKind::Realization),
        ("*--", RelKind::Composition),
        ("o--", RelKind::Aggregation),
        ("-->", RelKind::Association),
        ("..", RelKind::Dependency),
        ("--", RelKind::Link),
    ];
    for (arrow, kind) in arrows {
        if let Some(pos) = line.find(arrow) {
            let from = line[..pos].trim().to_string();
            let after = line[pos + arrow.len()..].trim();
            let (to, label) = if let Some(colon) = after.find(':') {
                (
                    after[..colon].trim().to_string(),
                    Some(after[colon + 1..].trim().to_string()),
                )
            } else {
                (after.to_string(), None)
            };
            if !from.is_empty() && !to.is_empty() {
                return Some(StructuralRelationship {
                    from,
                    to,
                    kind: kind.clone(),
                    from_mult: None,
                    to_mult: None,
                    label,
                });
            }
        }
    }
    None
}

// ── xychart-beta parser ───────────────────────────────────────────────────

/// Parse a Mermaid `xychart-beta` block into a `ChartDiagram`.
///
/// Handles:
/// ```text
/// xychart-beta
///   title "Q1 Sales"
///   x-axis [Jan, Feb, Mar]
///   y-axis 0 --> 100
///   bar [40, 60, 45]
///   line [35, 55, 48]
/// ```
pub fn parse_xychart(source: &str) -> Result<ChartDiagram, ParseError> {
    let mut title: Option<String> = None;
    let mut x_cats: Vec<String> = Vec::new();
    let mut y_min = 0.0_f64;
    let mut y_max = 100.0_f64;
    let mut series: Vec<ChartSeries> = Vec::new();

    let mut past_header = false;
    for line in source.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("%%") {
            continue;
        }
        if !past_header {
            if t.starts_with("xychart") {
                past_header = true;
            }
            continue;
        }
        if t.starts_with("title") {
            title = Some(t[5..].trim().trim_matches('"').to_string());
        } else if t.starts_with("x-axis") {
            x_cats = parse_bracket_list(&t[6..]);
        } else if t.starts_with("y-axis") {
            let rest = t[6..].trim();
            // Strip optional quoted label before numbers.
            let rest = if rest.starts_with('"') {
                if let Some(end) = rest[1..].find('"') {
                    rest[end + 2..].trim()
                } else {
                    rest
                }
            } else {
                rest
            };
            let nums: Vec<f64> = rest
                .split_whitespace()
                .filter(|s| {
                    s.chars()
                        .all(|c| c.is_ascii_digit() || c == '-' || c == '.')
                })
                .filter_map(|s| s.parse().ok())
                .collect();
            if nums.len() >= 2 {
                y_min = nums[0];
                y_max = nums[nums.len() - 1];
            }
        } else if t.starts_with("bar") {
            let data = parse_data_list(&t[3..]);
            series.push(ChartSeries {
                kind: SeriesKind::Bar,
                label: Some("bar".into()),
                data,
            });
        } else if t.starts_with("line") {
            let data = parse_data_list(&t[4..]);
            series.push(ChartSeries {
                kind: SeriesKind::Line,
                label: Some("line".into()),
                data,
            });
        }
    }

    let x_axis = if !x_cats.is_empty() {
        Some(Axis {
            kind: AxisKind::Categorical,
            title: None,
            categories: x_cats,
            min: 0.0,
            max: 0.0,
        })
    } else {
        None
    };
    let y_axis = Some(Axis {
        kind: AxisKind::Numeric,
        title: None,
        categories: vec![],
        min: y_min,
        max: y_max,
    });

    Ok(ChartDiagram {
        title,
        kind: ChartKind::Xy,
        x_axis,
        y_axis,
        series,
        slices: vec![],
        sankey_nodes: vec![],
        flows: vec![],
        orientation: ChartOrientation::Vertical,
    })
}

fn parse_bracket_list(s: &str) -> Vec<String> {
    let s = s.trim();
    let inner = if let (Some(l), Some(r)) = (s.find('['), s.rfind(']')) {
        &s[l + 1..r]
    } else {
        s
    };
    inner
        .split(',')
        .map(|x| x.trim().trim_matches('"').to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

fn parse_data_list(s: &str) -> Vec<f64> {
    let s = s.trim();
    let inner = if let (Some(l), Some(r)) = (s.find('['), s.rfind(']')) {
        &s[l + 1..r]
    } else {
        s
    };
    inner
        .split(',')
        .filter_map(|x| x.trim().parse().ok())
        .collect()
}

// ── state parser ─────────────────────────────────────────────────────────

/// Parse the graph-compatible core of Mermaid state diagrams.
///
/// The supported state slice lowers flat declarations, transitions,
/// pseudostates, and styles into graph IR. Composite states and notes remain
/// explicit compatibility gaps.
pub fn parse_state_diagram(source: &str) -> Result<GraphDiagram, ParseError> {
    let preprocessed = preprocess_mermaid_source(source)?;
    parse_mermaid_state_ast(&preprocessed.source)?;
    let mut cursor = TokenCursor::new(tokenize_mermaid_state(&preprocessed.source));
    cursor.skip_terminators();
    cursor
        .consume_if("HEADER")
        .ok_or_else(|| token_error(cursor.current(), "expected stateDiagram header"))?;
    cursor.skip_terminators();

    let mut direction = DiagramDirection::Tb;
    let mut nodes = Vec::new();
    let mut node_indices = HashMap::new();
    let mut edges = Vec::new();
    let mut pseudo_index = 0;
    let mut note_index = 0;
    let mut class_styles: HashMap<String, DiagramStyle> = HashMap::new();
    let mut pending_classes: Vec<(Vec<String>, String)> = Vec::new();

    while !cursor.at_eof() {
        if cursor.current().value.eq_ignore_ascii_case("direction") {
            cursor.advance();
            let token = cursor
                .consume_if("DIRECTION")
                .ok_or_else(|| token_error(cursor.current(), "expected state direction"))?;
            direction = match token.value.to_ascii_uppercase().as_str() {
                "TB" => DiagramDirection::Tb,
                "BT" => DiagramDirection::Bt,
                "LR" => DiagramDirection::Lr,
                "RL" => DiagramDirection::Rl,
                _ => unreachable!("state.tokens restricts direction values"),
            };
        } else if cursor.current().value.eq_ignore_ascii_case("classDef") {
            cursor.advance();
            let class_name = take_state_ref(&mut cursor)?;
            let mut style = DiagramStyle::default();
            parse_state_style_assignments(&mut cursor, &mut style)?;
            class_styles.insert(class_name, style);
        } else if cursor.current().value.eq_ignore_ascii_case("class") {
            cursor.advance();
            let mut ids = vec![take_state_ref(&mut cursor)?];
            while cursor.consume_if("COMMA").is_some() {
                ids.push(take_state_ref(&mut cursor)?);
            }
            let class_name = take_state_ref(&mut cursor)?;
            for id in &ids {
                if !node_indices.contains_key(id) {
                    upsert_state_node(&mut nodes, &mut node_indices, id.clone(), id.clone());
                }
            }
            for id in ids {
                apply_or_defer_state_class(
                    &id,
                    class_name.clone(),
                    &mut nodes,
                    &node_indices,
                    &class_styles,
                    &mut pending_classes,
                );
            }
        } else if cursor.current().value.eq_ignore_ascii_case("note") {
            cursor.advance();
            if token_name(cursor.current()) == "STRING" {
                let text = strip_state_string(&cursor.advance().value);
                if !cursor.current().value.eq_ignore_ascii_case("as") {
                    return Err(token_error(
                        cursor.current(),
                        "expected as before floating state note identifier",
                    ));
                }
                cursor.advance();
                let note_id = take_state_ref(&mut cursor)?;
                upsert_state_note_node(&mut nodes, &mut node_indices, note_id, text);
                cursor.skip_terminators();
                continue;
            }
            let note_is_left = if cursor.current().value.eq_ignore_ascii_case("left") {
                cursor.advance();
                true
            } else if cursor.current().value.eq_ignore_ascii_case("right") {
                cursor.advance();
                false
            } else {
                return Err(token_error(
                    cursor.current(),
                    "expected left or right state note placement",
                ));
            };
            if !cursor.current().value.eq_ignore_ascii_case("of") {
                return Err(token_error(
                    cursor.current(),
                    "expected of after note placement",
                ));
            }
            cursor.advance();
            let state_id = take_state_ref(&mut cursor)?;
            let text = if cursor.consume_if("COLON").is_some() {
                take_state_text(&mut cursor)
            } else {
                cursor.consume_if("NEWLINE").ok_or_else(|| {
                    token_error(
                        cursor.current(),
                        "expected ':' or newline before state note text",
                    )
                })?;
                take_state_multiline_note_text(&mut cursor)?
            };
            if !node_indices.contains_key(&state_id) {
                upsert_state_node(
                    &mut nodes,
                    &mut node_indices,
                    state_id.clone(),
                    state_id.clone(),
                );
            }
            let note_id = format!("__state_note_{note_index}");
            note_index += 1;
            upsert_state_note_node(&mut nodes, &mut node_indices, note_id.clone(), text);
            let (from, to) = if note_is_left {
                (note_id, state_id)
            } else {
                (state_id, note_id)
            };
            edges.push(GraphEdge {
                id: None,
                from,
                to,
                label: None,
                kind: EdgeKind::NoteAssociation,
                style: Some(DiagramStyle {
                    stroke: Some("#a16207".into()),
                    stroke_width: Some(1.5),
                    ..Default::default()
                }),
            });
        } else if cursor.current().value.eq_ignore_ascii_case("style") {
            cursor.advance();
            let id = take_state_ref(&mut cursor)?;
            if !node_indices.contains_key(&id) {
                upsert_state_node(&mut nodes, &mut node_indices, id.clone(), id.clone());
            }
            parse_state_style_assignments(
                &mut cursor,
                nodes[node_indices[&id]].style.get_or_insert_default(),
            )?;
        } else if cursor.current().value.eq_ignore_ascii_case("state") {
            cursor.advance();
            let (id, label) = if token_name(cursor.current()) == "STRING" {
                let label = strip_state_string(&cursor.advance().value);
                if !cursor.current().value.eq_ignore_ascii_case("as") {
                    return Err(token_error(
                        cursor.current(),
                        "expected state alias keyword as",
                    ));
                }
                cursor.advance();
                (take_state_ref(&mut cursor)?, label)
            } else {
                let id = take_state_ref(&mut cursor)?;
                if cursor.consume_if("CHOICE").is_some() {
                    upsert_state_node(&mut nodes, &mut node_indices, id.clone(), String::new());
                    let node = &mut nodes[node_indices[&id]];
                    node.label = DiagramLabel::new("");
                    node.shape = Some(DiagramShape::Diamond);
                    cursor.skip_terminators();
                    continue;
                }
                if cursor.consume_if("FORK_JOIN").is_some() {
                    upsert_state_node(&mut nodes, &mut node_indices, id.clone(), String::new());
                    let node = &mut nodes[node_indices[&id]];
                    node.label = DiagramLabel::new("");
                    node.shape = Some(DiagramShape::Bar);
                    node.style = Some(DiagramStyle {
                        fill: Some("#111827".into()),
                        stroke: Some("#111827".into()),
                        ..Default::default()
                    });
                    cursor.skip_terminators();
                    continue;
                }
                let label = if cursor.consume_if("COLON").is_some() {
                    take_state_text(&mut cursor)
                } else {
                    id.clone()
                };
                (id, label)
            };
            upsert_state_node(&mut nodes, &mut node_indices, id, label);
        } else {
            let from_is_edge_state = token_name(cursor.current()) == "EDGE_STATE";
            let from = take_state_endpoint(
                &mut cursor,
                true,
                &mut pseudo_index,
                &mut nodes,
                &mut node_indices,
            )?;
            let from_class = take_state_class_suffix(&mut cursor)?;
            if !from_is_edge_state && from_class.is_none() && cursor.consume_if("COLON").is_some() {
                let label = take_state_text(&mut cursor);
                upsert_state_node(&mut nodes, &mut node_indices, from, label);
                cursor.skip_terminators();
                continue;
            }
            if let Some(class_name) = from_class {
                apply_or_defer_state_class(
                    &from,
                    class_name,
                    &mut nodes,
                    &node_indices,
                    &class_styles,
                    &mut pending_classes,
                );
                if matches!(
                    token_name(cursor.current()),
                    "NEWLINE" | "SEMICOLON" | "EOF"
                ) {
                    cursor.skip_terminators();
                    continue;
                }
            }
            cursor
                .consume_if("ARROW")
                .ok_or_else(|| token_error(cursor.current(), "expected state transition arrow"))?;
            let to = take_state_endpoint(
                &mut cursor,
                false,
                &mut pseudo_index,
                &mut nodes,
                &mut node_indices,
            )?;
            if let Some(class_name) = take_state_class_suffix(&mut cursor)? {
                apply_or_defer_state_class(
                    &to,
                    class_name,
                    &mut nodes,
                    &node_indices,
                    &class_styles,
                    &mut pending_classes,
                );
            }
            let label = cursor
                .consume_if("COLON")
                .map(|_| DiagramLabel::new(take_state_text(&mut cursor)));
            edges.push(GraphEdge {
                id: None,
                from,
                to,
                label,
                kind: EdgeKind::Directed,
                style: None,
            });
        }
        cursor.skip_terminators();
    }

    for (ids, class_name) in pending_classes {
        let class_style = class_styles.get(&class_name).ok_or_else(|| ParseError {
            message: format!("unknown state style class {class_name:?}"),
            line: 1,
            col: 1,
        })?;
        for id in ids {
            merge_state_style(
                nodes[node_indices[&id]].style.get_or_insert_default(),
                class_style,
            );
        }
    }

    Ok(GraphDiagram {
        direction,
        title: None,
        nodes,
        edges,
    })
}

fn take_state_ref(cursor: &mut TokenCursor) -> Result<String, ParseError> {
    if matches!(token_name(cursor.current()), "ID" | "WORD") {
        Ok(cursor.advance().value.clone())
    } else {
        Err(token_error(cursor.current(), "expected state identifier"))
    }
}

fn take_state_class_suffix(cursor: &mut TokenCursor) -> Result<Option<String>, ParseError> {
    if cursor.consume_if("STYLE_SEPARATOR").is_some() {
        take_state_ref(cursor).map(Some)
    } else {
        Ok(None)
    }
}

fn take_state_endpoint(
    cursor: &mut TokenCursor,
    source: bool,
    pseudo_index: &mut usize,
    nodes: &mut Vec<GraphNode>,
    node_indices: &mut HashMap<String, usize>,
) -> Result<String, ParseError> {
    if cursor.consume_if("EDGE_STATE").is_some() {
        let role = if source { "start" } else { "end" };
        let id = format!("__state_{role}_{}", *pseudo_index);
        *pseudo_index += 1;
        upsert_state_node(nodes, node_indices, id.clone(), String::new());
        nodes[node_indices[&id]].shape = Some(DiagramShape::Ellipse);
        Ok(id)
    } else {
        let id = take_state_ref(cursor)?;
        if !node_indices.contains_key(&id) {
            upsert_state_node(nodes, node_indices, id.clone(), id.clone());
        }
        Ok(id)
    }
}

fn upsert_state_node(
    nodes: &mut Vec<GraphNode>,
    node_indices: &mut HashMap<String, usize>,
    id: String,
    label: String,
) {
    if let Some(&index) = node_indices.get(&id) {
        if !label.is_empty() {
            nodes[index].label = DiagramLabel::new(label);
        }
        return;
    }
    node_indices.insert(id.clone(), nodes.len());
    nodes.push(GraphNode {
        id,
        label: DiagramLabel::new(label),
        shape: Some(DiagramShape::RoundedRect),
        style: None,
    });
}

fn upsert_state_note_node(
    nodes: &mut Vec<GraphNode>,
    node_indices: &mut HashMap<String, usize>,
    id: String,
    text: String,
) {
    upsert_state_node(nodes, node_indices, id.clone(), text);
    let note = &mut nodes[node_indices[&id]];
    note.shape = Some(DiagramShape::Note);
    note.style = Some(DiagramStyle {
        fill: Some("#fff7cc".into()),
        stroke: Some("#a16207".into()),
        text_color: Some("#713f12".into()),
        corner_radius: Some(0.0),
        ..Default::default()
    });
}

fn apply_state_style(
    style: &mut DiagramStyle,
    property: &str,
    value: &Token,
) -> Result<(), ParseError> {
    match property.to_ascii_lowercase().as_str() {
        "fill" => style.fill = Some(value.value.clone()),
        "stroke" => style.stroke = Some(value.value.clone()),
        "color" => style.text_color = Some(value.value.clone()),
        "stroke-width" => {
            let width = value
                .value
                .strip_suffix("px")
                .unwrap_or(&value.value)
                .parse::<f64>()
                .map_err(|_| token_error(value, "invalid state stroke width"))?;
            style.stroke_width = Some(width);
        }
        _ => {
            return Err(token_error(
                value,
                format!("unsupported state style property {property:?}"),
            ))
        }
    }
    Ok(())
}

fn parse_state_style_assignments(
    cursor: &mut TokenCursor,
    style: &mut DiagramStyle,
) -> Result<(), ParseError> {
    loop {
        let property = take_state_ref(cursor)?;
        cursor.consume_if("COLON").ok_or_else(|| {
            token_error(cursor.current(), "expected ':' in state style assignment")
        })?;
        let value = cursor.advance().clone();
        if !matches!(token_name(&value), "HASH_COLOR" | "ID" | "WORD") {
            return Err(token_error(&value, "expected state style value"));
        }
        apply_state_style(style, &property, &value)?;
        if cursor.consume_if("COMMA").is_none() {
            return Ok(());
        }
    }
}

fn merge_state_style(target: &mut DiagramStyle, source: &DiagramStyle) {
    if source.fill.is_some() {
        target.fill.clone_from(&source.fill);
    }
    if source.stroke.is_some() {
        target.stroke.clone_from(&source.stroke);
    }
    if source.stroke_width.is_some() {
        target.stroke_width = source.stroke_width;
    }
    if source.text_color.is_some() {
        target.text_color.clone_from(&source.text_color);
    }
}

fn apply_or_defer_state_class(
    id: &str,
    class_name: String,
    nodes: &mut [GraphNode],
    node_indices: &HashMap<String, usize>,
    class_styles: &HashMap<String, DiagramStyle>,
    pending_classes: &mut Vec<(Vec<String>, String)>,
) {
    if let Some(class_style) = class_styles.get(&class_name) {
        merge_state_style(
            nodes[node_indices[id]].style.get_or_insert_default(),
            class_style,
        );
    } else {
        pending_classes.push((vec![id.to_string()], class_name));
    }
}

fn take_state_text(cursor: &mut TokenCursor) -> String {
    let mut text = String::new();
    while !cursor.at_eof() && !matches!(token_name(cursor.current()), "NEWLINE" | "SEMICOLON") {
        let token = cursor.advance();
        let value = if token_name(token) == "STRING" {
            strip_state_string(&token.value)
        } else {
            token.value.clone()
        };
        if token_name(token) == "COMMA" {
            text.push(',');
        } else {
            if !text.is_empty() && !text.ends_with(',') {
                text.push(' ');
            }
            text.push_str(&value);
        }
    }
    text
}

fn take_state_multiline_note_text(cursor: &mut TokenCursor) -> Result<String, ParseError> {
    let mut lines = Vec::new();
    while !cursor.at_eof() && token_name(cursor.current()) != "END_NOTE" {
        if cursor.consume_if("NEWLINE").is_some() {
            lines.push(String::new());
            continue;
        }
        let mut line = String::new();
        while !cursor.at_eof() && !matches!(token_name(cursor.current()), "NEWLINE" | "END_NOTE") {
            let token = cursor.advance();
            let value = if token_name(token) == "STRING" {
                strip_state_string(&token.value)
            } else {
                token.value.clone()
            };
            if token_name(token) == "COMMA" {
                line.push(',');
            } else {
                if !line.is_empty() && !line.ends_with(',') {
                    line.push(' ');
                }
                line.push_str(&value);
            }
        }
        lines.push(line);
        cursor.consume_if("NEWLINE");
    }
    cursor
        .consume_if("END_NOTE")
        .ok_or_else(|| token_error(cursor.current(), "expected end note terminator"))?;
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    Ok(lines.join("\n"))
}

fn strip_state_string(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value)
        .to_string()
}

// ── sequence parser ──────────────────────────────────────────────────────

/// Parse the grammar-backed core of Mermaid sequence diagrams into the
/// shared sequence IR. Unsupported control blocks fail grammar validation
/// instead of being silently discarded.
pub fn parse_sequence_diagram(source: &str) -> Result<SequenceDiagram, ParseError> {
    let preprocessed = preprocess_mermaid_source(source)?;
    parse_mermaid_sequence_ast(&preprocessed.source)?;

    let mut cursor = TokenCursor::new(tokenize_mermaid_sequence(&preprocessed.source));
    cursor.skip_terminators();
    cursor
        .consume_if("HEADER")
        .ok_or_else(|| token_error(cursor.current(), "expected sequenceDiagram header"))?;
    cursor.skip_terminators();

    let mut diagram = SequenceDiagram {
        title: None,
        accessibility_title: None,
        accessibility_description: None,
        auto_number: false,
        auto_number_start: 1.0,
        auto_number_step: 1.0,
        participants: Vec::new(),
        participant_groups: Vec::new(),
        events: Vec::new(),
    };
    let mut participant_indices: HashMap<String, usize> = HashMap::new();

    parse_sequence_body(&mut cursor, &mut diagram, &mut participant_indices, &[])?;
    bind_sequence_lifecycle_events(&mut diagram, cursor.current())?;
    validate_sequence_activation_balance(&diagram, cursor.current())?;
    if preprocessed.wrap == Some(true) {
        apply_sequence_default_wrap(&mut diagram);
    }

    Ok(diagram)
}

struct PreprocessedMermaid {
    source: String,
    wrap: Option<bool>,
}

fn preprocess_mermaid_source(source: &str) -> Result<PreprocessedMermaid, ParseError> {
    let mut cleaned = blank_mermaid_front_matter(source)?;
    let cleaned_source =
        String::from_utf8(cleaned.clone()).expect("front matter blanking preserves UTF-8");
    let mut search_from = 0;
    let mut wrap = None;
    while let Some(relative_start) = cleaned_source[search_from..].find("%%{") {
        let start = search_from + relative_start;
        let content_start = start + 3;
        let Some(relative_end) = cleaned_source[content_start..].find("}%%") else {
            let line = cleaned_source[..start]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            return Err(ParseError {
                message: "unterminated Mermaid directive".into(),
                line,
                col: 1,
            });
        };
        let end = content_start + relative_end;
        let directive = cleaned_source[content_start..end].trim();
        if directive.eq_ignore_ascii_case("wrap") {
            wrap = Some(true);
        } else if directive.eq_ignore_ascii_case("nowrap") {
            wrap = Some(false);
        }
        for byte in &mut cleaned[start..end + 3] {
            if !matches!(*byte, b'\r' | b'\n') {
                *byte = b' ';
            }
        }
        search_from = end + 3;
    }
    Ok(PreprocessedMermaid {
        source: String::from_utf8(cleaned).expect("directive blanking preserves UTF-8"),
        wrap,
    })
}

fn blank_mermaid_front_matter(source: &str) -> Result<Vec<u8>, ParseError> {
    let mut cleaned = source.as_bytes().to_vec();
    let mut offset = 0;
    let mut opening = None;

    for (line_index, line) in source.split_inclusive('\n').enumerate() {
        let trimmed = line.trim();
        if let Some((start, _)) = opening {
            if trimmed == "---" {
                for byte in &mut cleaned[start..offset + line.len()] {
                    if !matches!(*byte, b'\r' | b'\n') {
                        *byte = b' ';
                    }
                }
                return Ok(cleaned);
            }
        } else {
            if trimmed.is_empty() {
                offset += line.len();
                continue;
            }
            if trimmed != "---" {
                return Ok(cleaned);
            }
            opening = Some((offset, line_index + 1));
        }
        offset += line.len();
    }

    if let Some((_, line)) = opening {
        return Err(ParseError {
            message: "unterminated Mermaid YAML front matter".into(),
            line,
            col: 1,
        });
    }
    Ok(cleaned)
}

fn apply_sequence_default_wrap(diagram: &mut SequenceDiagram) {
    for participant in &mut diagram.participants {
        if participant.label_wrap == SequenceTextWrap::Default {
            participant.label_wrap = SequenceTextWrap::Wrap;
        }
    }
    for group in &mut diagram.participant_groups {
        if group.label_wrap == SequenceTextWrap::Default {
            group.label_wrap = SequenceTextWrap::Wrap;
        }
    }
    for event in &mut diagram.events {
        let wrap = match event {
            SequenceEvent::Message { wrap, .. }
            | SequenceEvent::Note { wrap, .. }
            | SequenceEvent::BlockStart { wrap, .. }
            | SequenceEvent::BlockBranch { wrap, .. } => Some(wrap),
            _ => None,
        };
        if let Some(wrap) = wrap {
            if *wrap == SequenceTextWrap::Default {
                *wrap = SequenceTextWrap::Wrap;
            }
        }
    }
}

fn validate_sequence_activation_balance(
    diagram: &SequenceDiagram,
    eof: &Token,
) -> Result<(), ParseError> {
    let mut active: HashMap<&str, usize> = HashMap::new();
    for event in &diagram.events {
        match event {
            SequenceEvent::Message {
                from,
                to,
                activate,
                deactivate,
                central_connection,
                ..
            } => {
                if *deactivate {
                    deactivate_sequence_participant(&mut active, from, eof)?;
                }
                match central_connection {
                    SequenceCentralConnection::Source => {
                        *active.entry(from).or_default() += 1;
                    }
                    SequenceCentralConnection::Destination => {
                        *active.entry(to).or_default() += 1;
                    }
                    SequenceCentralConnection::Both => {
                        *active.entry(from).or_default() += 1;
                        *active.entry(to).or_default() += 1;
                    }
                    SequenceCentralConnection::None => {}
                }
                if *activate {
                    *active.entry(to).or_default() += 1;
                }
            }
            SequenceEvent::Activation {
                participant,
                active: true,
            } => *active.entry(participant).or_default() += 1,
            SequenceEvent::Activation {
                participant,
                active: false,
            } => deactivate_sequence_participant(&mut active, participant, eof)?,
            _ => {}
        }
    }
    Ok(())
}

fn deactivate_sequence_participant<'a>(
    active: &mut HashMap<&'a str, usize>,
    participant: &'a str,
    token: &Token,
) -> Result<(), ParseError> {
    let count = active.entry(participant).or_default();
    if *count == 0 {
        return Err(token_error(
            token,
            format!("trying to deactivate inactive sequence participant {participant:?}"),
        ));
    }
    *count -= 1;
    Ok(())
}

fn bind_sequence_lifecycle_events(
    diagram: &mut SequenceDiagram,
    eof: &Token,
) -> Result<(), ParseError> {
    let mut bound = Vec::with_capacity(diagram.events.len());
    let mut pending: Option<SequenceEvent> = None;
    for event in diagram.events.drain(..) {
        match &event {
            SequenceEvent::ParticipantCreated { .. }
            | SequenceEvent::ParticipantDestroyed { .. } => {
                if pending.is_some() {
                    return Err(token_error(
                        eof,
                        "sequence lifecycle declaration requires an associated message",
                    ));
                }
                pending = Some(event);
            }
            SequenceEvent::Message { from, to, .. } => match pending.take() {
                Some(SequenceEvent::ParticipantCreated { participant }) => {
                    if to != &participant {
                        return Err(token_error(
                            eof,
                            format!(
                                "created participant {participant:?} must receive the next message"
                            ),
                        ));
                    }
                    bound.push(SequenceEvent::ParticipantCreated { participant });
                    bound.push(event);
                }
                Some(SequenceEvent::ParticipantDestroyed { participant }) => {
                    if from != &participant && to != &participant {
                        return Err(token_error(
                            eof,
                            format!(
                                "destroyed participant {participant:?} must be part of the next message"
                            ),
                        ));
                    }
                    bound.push(event);
                    bound.push(SequenceEvent::ParticipantDestroyed { participant });
                }
                None => bound.push(event),
                Some(_) => unreachable!("pending lifecycle event has a constrained variant"),
            },
            _ => bound.push(event),
        }
    }
    if pending.is_some() {
        return Err(token_error(
            eof,
            "sequence lifecycle declaration requires an associated message",
        ));
    }
    diagram.events = bound;
    Ok(())
}

fn parse_sequence_body(
    cursor: &mut TokenCursor,
    diagram: &mut SequenceDiagram,
    participant_indices: &mut HashMap<String, usize>,
    terminators: &[&str],
) -> Result<(), ParseError> {
    cursor.skip_terminators();
    while !cursor.at_eof() && !terminators.contains(&cursor.current().value.as_str()) {
        if token_name(cursor.current()) == "ACC_DESCR_BLOCK" {
            let token = cursor.advance().clone();
            let open = token.value.find('{').expect("token grammar requires '{'");
            let close = token.value.rfind('}').expect("token grammar requires '}'");
            diagram.accessibility_description = Some(token.value[open + 1..close].trim().into());
            cursor.skip_terminators();
            continue;
        }
        match cursor.current().value.as_str() {
            "participant" | "actor" => {
                parse_sequence_participant(cursor, diagram, participant_indices, false)?;
            }
            "create" => {
                cursor.advance();
                parse_sequence_participant(cursor, diagram, participant_indices, true)?;
            }
            "box" => parse_sequence_participant_box(cursor, diagram, participant_indices)?,
            "destroy" => {
                cursor.advance();
                let participant = take_sequence_actor_ref(cursor)?;
                ensure_sequence_participant(diagram, participant_indices, &participant);
                diagram
                    .events
                    .push(SequenceEvent::ParticipantDestroyed { participant });
            }
            "activate" | "deactivate" => {
                let active = cursor.advance().value == "activate";
                let participant = take_sequence_actor_ref(cursor)?;
                ensure_sequence_participant(diagram, participant_indices, &participant);
                diagram.events.push(SequenceEvent::Activation {
                    participant,
                    active,
                });
            }
            "note" => parse_sequence_note(cursor, diagram, participant_indices)?,
            "link" | "links" => parse_sequence_links(cursor, diagram, participant_indices)?,
            "properties" => parse_sequence_properties(cursor, diagram, participant_indices)?,
            "details" => parse_sequence_details(cursor, diagram, participant_indices)?,
            "accTitle" | "accDescr" => {
                let kind = cursor.advance().value.clone();
                cursor.consume_if("COLON").ok_or_else(|| {
                    token_error(cursor.current(), "expected ':' before accessibility text")
                })?;
                let text = take_sequence_line_text(cursor);
                if kind == "accTitle" {
                    diagram.accessibility_title = Some(text);
                } else {
                    diagram.accessibility_description = Some(text);
                }
            }
            "title" => {
                cursor.advance();
                cursor.consume_if("COLON");
                diagram.title = Some(take_sequence_line_text(cursor));
            }
            "autonumber" => {
                cursor.advance();
                if cursor.current().value == "off" {
                    diagram.auto_number = false;
                    cursor.advance();
                    diagram.events.push(SequenceEvent::AutoNumber {
                        visible: false,
                        start: None,
                        step: None,
                    });
                } else {
                    diagram.auto_number = true;
                    let start = take_sequence_number(cursor)?;
                    let step = take_sequence_number(cursor)?.or(start.map(|_| 1.0));
                    diagram.auto_number_start = start.unwrap_or(1.0);
                    diagram.auto_number_step = step.unwrap_or(1.0);
                    diagram.events.push(SequenceEvent::AutoNumber {
                        visible: true,
                        start,
                        step,
                    });
                }
            }
            "loop" | "rect" | "opt" | "alt" | "par" | "par_over" | "critical" | "break" => {
                parse_sequence_control_block(cursor, diagram, participant_indices)?
            }
            _ => parse_sequence_message(cursor, diagram, participant_indices)?,
        }
        cursor.skip_terminators();
    }
    Ok(())
}

fn take_sequence_number(cursor: &mut TokenCursor) -> Result<Option<f64>, ParseError> {
    if cursor.at_eof() || matches!(token_name(cursor.current()), "NEWLINE" | "SEMICOLON") {
        return Ok(None);
    }
    let token = cursor.advance().clone();
    if let Some(next) = cursor.tokens.get(cursor.index) {
        let token_end_column = token.column + token.value.chars().count();
        if token.line == next.line
            && token_end_column == next.column
            && token_name(next) == "NUMBER"
        {
            return Err(token_error(
                &token,
                "autonumber decimals support at most two fractional digits and values require whitespace separation",
            ));
        }
    }
    let value = token
        .value
        .parse::<f64>()
        .map_err(|_| token_error(&token, "expected an autonumber decimal value"))?;
    if !value.is_finite() || value < 0.0 {
        return Err(token_error(
            &token,
            "autonumber values must be finite and non-negative",
        ));
    }
    Ok(Some(value))
}

fn parse_sequence_links(
    cursor: &mut TokenCursor,
    diagram: &mut SequenceDiagram,
    participant_indices: &mut HashMap<String, usize>,
) -> Result<(), ParseError> {
    let plural = cursor.advance().value == "links";
    let participant = take_sequence_actor_ref(cursor)?;
    ensure_sequence_participant(diagram, participant_indices, &participant);
    cursor
        .consume_if("COLON")
        .ok_or_else(|| token_error(cursor.current(), "expected ':' before actor links"))?;
    let links = if plural {
        let token = cursor.advance().clone();
        if token_name(&token) != "JSON_OBJECT" {
            return Err(token_error(&token, "expected a JSON object of actor links"));
        }
        serde_json::from_str::<HashMap<String, String>>(&token.value)
            .map_err(|error| token_error(&token, format!("invalid actor links JSON: {error}")))?
            .into_iter()
            .map(|(label, url)| SequenceLink { label, url })
            .collect()
    } else {
        let mut label = Vec::new();
        while !cursor.at_eof() && token_name(cursor.current()) != "AT" {
            label.push(cursor.advance().value.clone());
        }
        cursor
            .consume_if("AT")
            .ok_or_else(|| token_error(cursor.current(), "expected '@' before actor link URL"))?;
        let url = cursor
            .consume_if("URL")
            .ok_or_else(|| token_error(cursor.current(), "expected an http(s) actor link URL"))?;
        vec![SequenceLink {
            label: label.join(" "),
            url: url.value,
        }]
    };
    let index = participant_indices[&participant];
    diagram.participants[index].links.extend(links);
    Ok(())
}

fn parse_sequence_properties(
    cursor: &mut TokenCursor,
    diagram: &mut SequenceDiagram,
    participant_indices: &mut HashMap<String, usize>,
) -> Result<(), ParseError> {
    cursor.advance();
    let participant = take_sequence_actor_ref(cursor)?;
    ensure_sequence_participant(diagram, participant_indices, &participant);
    cursor
        .consume_if("COLON")
        .ok_or_else(|| token_error(cursor.current(), "expected ':' before actor properties"))?;
    let token = cursor.advance().clone();
    if token_name(&token) != "JSON_OBJECT" {
        return Err(token_error(
            &token,
            "expected a JSON object of actor properties",
        ));
    }
    let properties = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(
        &token.value,
    )
    .map_err(|error| token_error(&token, format!("invalid actor properties JSON: {error}")))?;
    let target = &mut diagram.participants[participant_indices[&participant]].properties;
    for (name, value) in properties {
        let property = SequenceProperty {
            name,
            value_json: value.to_string(),
        };
        if let Some(existing) = target
            .iter_mut()
            .find(|existing| existing.name == property.name)
        {
            *existing = property;
        } else {
            target.push(property);
        }
    }
    Ok(())
}

fn parse_sequence_details(
    cursor: &mut TokenCursor,
    diagram: &mut SequenceDiagram,
    participant_indices: &mut HashMap<String, usize>,
) -> Result<(), ParseError> {
    cursor.advance();
    let participant = take_sequence_actor_ref(cursor)?;
    ensure_sequence_participant(diagram, participant_indices, &participant);
    cursor
        .consume_if("COLON")
        .ok_or_else(|| token_error(cursor.current(), "expected ':' before actor details"))?;
    let mut reference = String::new();
    while !cursor.at_eof() && !matches!(token_name(cursor.current()), "NEWLINE" | "SEMICOLON") {
        reference.push_str(&cursor.advance().value);
    }
    if reference.is_empty() {
        return Err(token_error(
            cursor.current(),
            "expected a host document element ID for actor details",
        ));
    }
    diagram.participants[participant_indices[&participant]].details_reference = Some(reference);
    Ok(())
}

fn parse_sequence_participant(
    cursor: &mut TokenCursor,
    diagram: &mut SequenceDiagram,
    participant_indices: &mut HashMap<String, usize>,
    created: bool,
) -> Result<String, ParseError> {
    let declaration = cursor.advance().clone();
    let mut kind = match declaration.value.as_str() {
        "actor" => SequenceParticipantKind::Actor,
        "participant" => SequenceParticipantKind::Participant,
        other => {
            return Err(token_error(
                &declaration,
                format!("expected participant or actor after create, got {other:?}"),
            ))
        }
    };
    let id = take_sequence_actor_ref(cursor)?;
    if created && participant_indices.contains_key(&id) {
        return Err(token_error(
            &declaration,
            format!("cannot create duplicate sequence participant {id:?}"),
        ));
    }
    let mut inline_alias = None;
    if token_name(cursor.current()) == "CONFIG" {
        let config = cursor.advance().clone();
        let parsed = parse_sequence_participant_config(&config)?;
        if let Some(config_kind) = parsed.0 {
            kind = config_kind;
        }
        inline_alias = parsed.1;
    }
    let (label, label_wrap) = if cursor.current().value == "as" {
        cursor.advance();
        take_sequence_wrapped_text(cursor)
    } else {
        (
            inline_alias.unwrap_or_else(|| id.clone()),
            SequenceTextWrap::Default,
        )
    };
    upsert_sequence_participant(
        diagram,
        participant_indices,
        id.clone(),
        label,
        label_wrap,
        kind,
    );
    if created {
        diagram.events.push(SequenceEvent::ParticipantCreated {
            participant: id.clone(),
        });
    }
    Ok(id)
}

fn parse_sequence_participant_config(
    token: &Token,
) -> Result<(Option<SequenceParticipantKind>, Option<String>), ParseError> {
    let inner = token
        .value
        .strip_prefix("@{")
        .and_then(|value| value.strip_suffix('}'))
        .ok_or_else(|| token_error(token, "invalid sequence participant configuration"))?;
    let mut kind = None;
    let mut alias = None;
    for field in split_sequence_config_fields(inner) {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = field.split_once(':').ok_or_else(|| {
            token_error(token, "participant configuration fields require key: value")
        })?;
        let key = parse_sequence_config_scalar(token, key)?;
        let value = parse_sequence_config_scalar(token, value)?;
        match key.as_str() {
            "type" => {
                kind = Some(match value.to_ascii_lowercase().as_str() {
                    "participant" => SequenceParticipantKind::Participant,
                    "actor" => SequenceParticipantKind::Actor,
                    "boundary" => SequenceParticipantKind::Boundary,
                    "control" => SequenceParticipantKind::Control,
                    "entity" => SequenceParticipantKind::Entity,
                    "database" => SequenceParticipantKind::Database,
                    "collections" => SequenceParticipantKind::Collections,
                    "queue" => SequenceParticipantKind::Queue,
                    other => {
                        return Err(token_error(
                            token,
                            format!("unsupported sequence participant type {other:?}"),
                        ))
                    }
                });
            }
            "alias" => alias = Some(value),
            _ => {}
        }
    }
    Ok((kind, alias))
}

fn split_sequence_config_fields(input: &str) -> Vec<&str> {
    let mut fields = Vec::new();
    let mut start = 0;
    let mut quote = None;
    let mut escaped = false;
    let mut depth = 0_u32;
    for (index, character) in input.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if quote.is_some() && character == '\\' {
            escaped = true;
            continue;
        }
        if let Some(delimiter) = quote {
            if character == delimiter {
                quote = None;
            }
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '{' | '[' | '(' => depth += 1,
            '}' | ']' | ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                fields.push(&input[start..index]);
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    fields.push(&input[start..]);
    fields
}

fn parse_sequence_config_scalar(token: &Token, value: &str) -> Result<String, ParseError> {
    let value = value.trim();
    if value.starts_with('"') {
        return serde_json::from_str(value).map_err(|error| {
            token_error(
                token,
                format!("invalid double-quoted participant configuration value: {error}"),
            )
        });
    }
    if let Some(inner) = value
        .strip_prefix('\'')
        .and_then(|value| value.strip_suffix('\''))
    {
        return Ok(inner.replace("''", "'"));
    }
    if value.starts_with('\'') || value.ends_with('\'') {
        return Err(token_error(
            token,
            "invalid single-quoted participant configuration value",
        ));
    }
    Ok(value.to_string())
}

fn parse_sequence_participant_box(
    cursor: &mut TokenCursor,
    diagram: &mut SequenceDiagram,
    participant_indices: &mut HashMap<String, usize>,
) -> Result<(), ParseError> {
    cursor.advance();
    let (fill, label, label_wrap) = parse_sequence_box_header(&take_sequence_line_text(cursor));
    let group_id = format!("box-{}", diagram.participant_groups.len() + 1);
    diagram.participant_groups.push(SequenceParticipantGroup {
        id: group_id.clone(),
        label,
        label_wrap,
        fill,
    });
    cursor.skip_terminators();
    while !cursor.at_eof() && cursor.current().value != "end" {
        if !matches!(cursor.current().value.as_str(), "participant" | "actor") {
            return Err(token_error(
                cursor.current(),
                "sequence box may only contain participant declarations",
            ));
        }
        let id = parse_sequence_participant(cursor, diagram, participant_indices, false)?;
        let index = participant_indices[&id];
        if let Some(existing_group) = diagram.participants[index].group_id.as_deref() {
            if existing_group != group_id {
                return Err(token_error(
                    cursor.current(),
                    format!("sequence participant {id:?} cannot belong to multiple boxes"),
                ));
            }
        }
        diagram.participants[index].group_id = Some(group_id.clone());
        cursor.skip_terminators();
    }
    if cursor.at_eof() {
        return Err(token_error(cursor.current(), "unterminated sequence box"));
    }
    cursor.advance();
    Ok(())
}

fn parse_sequence_box_header(raw: &str) -> (Option<String>, Option<String>, SequenceTextWrap) {
    let raw = raw.trim();
    if raw.is_empty() {
        return (None, None, SequenceTextWrap::Default);
    }
    let lower = raw.to_ascii_lowercase();
    let function_color = ["rgb(", "rgba(", "hsl(", "hsla("]
        .iter()
        .any(|prefix| lower.starts_with(prefix));
    let split = if function_color {
        raw.find(')').map_or(raw.len(), |index| index + 1)
    } else {
        raw.find(char::is_whitespace).unwrap_or(raw.len())
    };
    let first = &raw[..split];
    let is_named_color = matches!(
        first.to_ascii_lowercase().as_str(),
        "aqua"
            | "black"
            | "blue"
            | "fuchsia"
            | "gray"
            | "grey"
            | "green"
            | "lime"
            | "maroon"
            | "navy"
            | "olive"
            | "orange"
            | "purple"
            | "red"
            | "silver"
            | "teal"
            | "transparent"
            | "white"
            | "yellow"
    );
    if function_color || is_named_color {
        let (label, wrap) = split_sequence_wrap_directive(raw[split..].trim());
        let fill =
            (!first.eq_ignore_ascii_case("transparent")).then(|| normalize_sequence_color(first));
        (fill, (!label.is_empty()).then(|| label.to_string()), wrap)
    } else {
        let (label, wrap) = split_sequence_wrap_directive(raw);
        (None, (!label.is_empty()).then(|| label.to_string()), wrap)
    }
}

fn split_sequence_wrap_directive(raw: &str) -> (&str, SequenceTextWrap) {
    if let Some(label) = raw.strip_prefix("nowrap:") {
        (label.trim(), SequenceTextWrap::NoWrap)
    } else if let Some(label) = raw.strip_prefix("wrap:") {
        (label.trim(), SequenceTextWrap::Wrap)
    } else {
        (raw, SequenceTextWrap::Default)
    }
}

fn normalize_sequence_color(color: &str) -> String {
    let lower = color.to_ascii_lowercase();
    let Some((has_alpha, inner)) = lower
        .strip_prefix("hsla(")
        .and_then(|value| value.strip_suffix(')'))
        .map(|inner| (true, inner))
        .or_else(|| {
            lower
                .strip_prefix("hsl(")
                .and_then(|value| value.strip_suffix(')'))
                .map(|inner| (false, inner))
        })
    else {
        return color.to_string();
    };
    let parts: Vec<_> = inner.split(',').map(str::trim).collect();
    if parts.len() != if has_alpha { 4 } else { 3 } {
        return color.to_string();
    }
    let Some(hue) = parts[0].parse::<f64>().ok() else {
        return color.to_string();
    };
    let Some(saturation) = parts[1]
        .strip_suffix('%')
        .and_then(|value| value.parse::<f64>().ok())
    else {
        return color.to_string();
    };
    let Some(lightness) = parts[2]
        .strip_suffix('%')
        .and_then(|value| value.parse::<f64>().ok())
    else {
        return color.to_string();
    };
    if !hue.is_finite() || !saturation.is_finite() || !lightness.is_finite() {
        return color.to_string();
    }
    let saturation = (saturation / 100.0).clamp(0.0, 1.0);
    let lightness = (lightness / 100.0).clamp(0.0, 1.0);
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let sector = hue.rem_euclid(360.0) / 60.0;
    let x = chroma * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match sector as u8 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = lightness - chroma / 2.0;
    let channel = |value: f64| ((value + m) * 255.0).round() as u8;
    if has_alpha {
        let Ok(alpha) = parts[3].parse::<f64>() else {
            return color.to_string();
        };
        if !alpha.is_finite() {
            return color.to_string();
        }
        let alpha = alpha.clamp(0.0, 1.0);
        format!(
            "rgba({}, {}, {}, {alpha})",
            channel(r),
            channel(g),
            channel(b)
        )
    } else {
        format!("rgb({}, {}, {})", channel(r), channel(g), channel(b))
    }
}

fn parse_sequence_control_block(
    cursor: &mut TokenCursor,
    diagram: &mut SequenceDiagram,
    participant_indices: &mut HashMap<String, usize>,
) -> Result<(), ParseError> {
    let start = cursor.advance().clone();
    let (kind, branch_keyword) = match start.value.as_str() {
        "loop" => (SequenceBlockKind::Loop, None),
        "rect" => (SequenceBlockKind::Rect, None),
        "opt" => (SequenceBlockKind::Opt, None),
        "alt" => (SequenceBlockKind::Alt, Some("else")),
        "par" => (SequenceBlockKind::Par, Some("and")),
        "par_over" => (SequenceBlockKind::ParOver, Some("and")),
        "critical" => (SequenceBlockKind::Critical, Some("option")),
        "break" => (SequenceBlockKind::Break, None),
        other => {
            return Err(token_error(
                &start,
                format!("unsupported sequence control block {other:?}"),
            ))
        }
    };
    let (label, wrap, fill) = if kind == SequenceBlockKind::Rect {
        let color = take_sequence_line_text(cursor);
        (
            String::new(),
            SequenceTextWrap::Default,
            (!color.is_empty()).then(|| normalize_sequence_color(&color)),
        )
    } else {
        let (label, wrap) = take_sequence_wrapped_text(cursor);
        (label, wrap, None)
    };
    diagram.events.push(SequenceEvent::BlockStart {
        kind: kind.clone(),
        label,
        wrap,
        fill,
    });
    cursor.skip_terminators();

    loop {
        let terminators = match branch_keyword {
            Some(branch) => vec![branch, "end"],
            None => vec!["end"],
        };
        parse_sequence_body(cursor, diagram, participant_indices, &terminators)?;
        if cursor.at_eof() {
            return Err(token_error(
                cursor.current(),
                format!("unterminated {:?} sequence block", kind),
            ));
        }
        if cursor.current().value == "end" {
            cursor.advance();
            diagram.events.push(SequenceEvent::BlockEnd { kind });
            return Ok(());
        }

        let branch = cursor.advance().clone();
        if Some(branch.value.as_str()) != branch_keyword {
            return Err(token_error(
                &branch,
                format!("unexpected sequence block branch {:?}", branch.value),
            ));
        }
        let (label, wrap) = take_sequence_wrapped_text(cursor);
        diagram
            .events
            .push(SequenceEvent::BlockBranch { label, wrap });
        cursor.skip_terminators();
    }
}

fn parse_sequence_message(
    cursor: &mut TokenCursor,
    diagram: &mut SequenceDiagram,
    participant_indices: &mut HashMap<String, usize>,
) -> Result<(), ParseError> {
    let from = take_sequence_actor_ref(cursor)?;
    let central_source = cursor.consume_if("CENTRAL").is_some();
    let arrow = cursor.advance().clone();
    let (line_style, arrowhead, bidirectional) = match token_name(&arrow) {
        "SOLID_OPEN_ARROW" => (SequenceLineStyle::Solid, SequenceArrowhead::Open, false),
        "DOTTED_OPEN_ARROW" => (SequenceLineStyle::Dotted, SequenceArrowhead::Open, false),
        "SOLID_FILLED_ARROW" => (SequenceLineStyle::Solid, SequenceArrowhead::Filled, false),
        "DOTTED_FILLED_ARROW" => (SequenceLineStyle::Dotted, SequenceArrowhead::Filled, false),
        "BIDIRECTIONAL_SOLID" => (SequenceLineStyle::Solid, SequenceArrowhead::Filled, true),
        "BIDIRECTIONAL_DOTTED" => (SequenceLineStyle::Dotted, SequenceArrowhead::Filled, true),
        "SOLID_CROSS_ARROW" => (SequenceLineStyle::Solid, SequenceArrowhead::Cross, false),
        "DOTTED_CROSS_ARROW" => (SequenceLineStyle::Dotted, SequenceArrowhead::Cross, false),
        "SOLID_POINT_ARROW" => (SequenceLineStyle::Solid, SequenceArrowhead::Point, false),
        "DOTTED_POINT_ARROW" => (SequenceLineStyle::Dotted, SequenceArrowhead::Point, false),
        "SOLID_FILLED_TOP" => (
            SequenceLineStyle::Solid,
            SequenceArrowhead::FilledTop,
            false,
        ),
        "SOLID_FILLED_BOTTOM" => (
            SequenceLineStyle::Solid,
            SequenceArrowhead::FilledBottom,
            false,
        ),
        "SOLID_STICK_TOP" => (SequenceLineStyle::Solid, SequenceArrowhead::StickTop, false),
        "SOLID_STICK_BOTTOM" => (
            SequenceLineStyle::Solid,
            SequenceArrowhead::StickBottom,
            false,
        ),
        "DOTTED_FILLED_TOP" => (
            SequenceLineStyle::Dotted,
            SequenceArrowhead::FilledTop,
            false,
        ),
        "DOTTED_FILLED_BOTTOM" => (
            SequenceLineStyle::Dotted,
            SequenceArrowhead::FilledBottom,
            false,
        ),
        "DOTTED_STICK_TOP" => (
            SequenceLineStyle::Dotted,
            SequenceArrowhead::StickTop,
            false,
        ),
        "DOTTED_STICK_BOTTOM" => (
            SequenceLineStyle::Dotted,
            SequenceArrowhead::StickBottom,
            false,
        ),
        "SOLID_REVERSE_FILLED_TOP" => (
            SequenceLineStyle::Solid,
            SequenceArrowhead::ReverseFilledTop,
            false,
        ),
        "SOLID_REVERSE_FILLED_BOTTOM" => (
            SequenceLineStyle::Solid,
            SequenceArrowhead::ReverseFilledBottom,
            false,
        ),
        "SOLID_REVERSE_STICK_TOP" => (
            SequenceLineStyle::Solid,
            SequenceArrowhead::ReverseStickTop,
            false,
        ),
        "SOLID_REVERSE_STICK_BOTTOM" => (
            SequenceLineStyle::Solid,
            SequenceArrowhead::ReverseStickBottom,
            false,
        ),
        "DOTTED_REVERSE_FILLED_TOP" => (
            SequenceLineStyle::Dotted,
            SequenceArrowhead::ReverseFilledTop,
            false,
        ),
        "DOTTED_REVERSE_FILLED_BOTTOM" => (
            SequenceLineStyle::Dotted,
            SequenceArrowhead::ReverseFilledBottom,
            false,
        ),
        "DOTTED_REVERSE_STICK_TOP" => (
            SequenceLineStyle::Dotted,
            SequenceArrowhead::ReverseStickTop,
            false,
        ),
        "DOTTED_REVERSE_STICK_BOTTOM" => (
            SequenceLineStyle::Dotted,
            SequenceArrowhead::ReverseStickBottom,
            false,
        ),
        other => {
            return Err(token_error(
                &arrow,
                format!("unsupported sequence arrow {other}"),
            ))
        }
    };
    let central_destination = cursor.consume_if("CENTRAL").is_some();
    let central_connection = match (central_source, central_destination) {
        (false, false) => SequenceCentralConnection::None,
        (true, false) => SequenceCentralConnection::Source,
        (false, true) => SequenceCentralConnection::Destination,
        (true, true) => SequenceCentralConnection::Both,
    };
    let activate = cursor.consume_if("PLUS").is_some();
    let deactivate = if activate {
        false
    } else {
        cursor.consume_if("MINUS").is_some()
    };
    let to = take_sequence_actor_ref(cursor)?;
    cursor
        .consume_if("COLON")
        .ok_or_else(|| token_error(cursor.current(), "expected ':' before sequence message"))?;
    let (label, wrap) = take_sequence_wrapped_text(cursor);
    ensure_sequence_participant(diagram, participant_indices, &from);
    ensure_sequence_participant(diagram, participant_indices, &to);
    diagram.events.push(SequenceEvent::Message {
        from,
        to,
        label,
        wrap,
        line_style,
        arrowhead,
        bidirectional,
        central_connection,
        activate,
        deactivate,
    });
    Ok(())
}

fn parse_sequence_note(
    cursor: &mut TokenCursor,
    diagram: &mut SequenceDiagram,
    participant_indices: &mut HashMap<String, usize>,
) -> Result<(), ParseError> {
    cursor.advance();
    let placement_token = cursor.advance().clone();
    let placement = match placement_token.value.as_str() {
        "left" => {
            consume_sequence_word(cursor, "of")?;
            SequenceNotePlacement::LeftOf
        }
        "right" => {
            consume_sequence_word(cursor, "of")?;
            SequenceNotePlacement::RightOf
        }
        "over" => SequenceNotePlacement::Over,
        other => {
            return Err(token_error(
                &placement_token,
                format!("invalid note placement {other:?}"),
            ))
        }
    };
    let mut participants = vec![take_sequence_actor_ref(cursor)?];
    if cursor.consume_if("COMMA").is_some() {
        participants.push(take_sequence_actor_ref(cursor)?);
    }
    cursor
        .consume_if("COLON")
        .ok_or_else(|| token_error(cursor.current(), "expected ':' before note text"))?;
    let (text, wrap) = take_sequence_wrapped_text(cursor);
    for participant in &participants {
        ensure_sequence_participant(diagram, participant_indices, participant);
    }
    diagram.events.push(SequenceEvent::Note {
        participants,
        placement,
        text,
        wrap,
    });
    Ok(())
}

fn consume_sequence_word(cursor: &mut TokenCursor, expected: &str) -> Result<(), ParseError> {
    if cursor.current().value == expected {
        cursor.advance();
        Ok(())
    } else {
        Err(token_error(
            cursor.current(),
            format!("expected sequence word {expected:?}"),
        ))
    }
}

fn take_sequence_actor_ref(cursor: &mut TokenCursor) -> Result<String, ParseError> {
    let start = cursor.current().clone();
    let mut actor = String::new();
    loop {
        if matches!(
            token_name(cursor.current()),
            "IDENTIFIER" | "WORD" | "NUMBER"
        ) && cursor.current().value != "as"
        {
            if !actor.is_empty() && !actor.ends_with('-') {
                actor.push(' ');
            }
            actor.push_str(&cursor.advance().value);
            continue;
        }
        let hyphen_continues_actor = token_name(cursor.current()) == "MINUS"
            && cursor
                .tokens
                .get(cursor.index + 1)
                .is_some_and(|next| matches!(token_name(next), "IDENTIFIER" | "WORD" | "NUMBER"));
        if !actor.is_empty() && hyphen_continues_actor {
            actor.push('-');
            cursor.advance();
            continue;
        }
        break;
    }
    if actor.is_empty() {
        return Err(token_error(
            &start,
            "expected sequence participant identifier",
        ));
    }
    Ok(actor)
}

fn take_sequence_line_text(cursor: &mut TokenCursor) -> String {
    let mut text = String::new();
    let mut previous_end_column = None;
    while !cursor.at_eof() && !matches!(token_name(cursor.current()), "NEWLINE" | "SEMICOLON") {
        let token = cursor.advance();
        if let Some(previous_end) = previous_end_column {
            let gap = token.column.saturating_sub(previous_end);
            text.extend(std::iter::repeat_n(' ', gap));
        }
        let value = if token_name(token) == "ENTITY" {
            let inner = token.value.trim_start_matches('#').trim_end_matches(';');
            let html_entity = if inner.chars().all(|character| character.is_ascii_digit()) {
                format!("&#{inner};")
            } else {
                format!("&{inner};")
            };
            commonmark_parser::entities::decode_entity(&html_entity)
        } else {
            token.value.clone()
        };
        text.push_str(&value);
        previous_end_column = Some(token.column + token.value.chars().count());
    }
    let text = text
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<br>", "\n");
    text.split('\n')
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
}

fn take_sequence_wrapped_text(cursor: &mut TokenCursor) -> (String, SequenceTextWrap) {
    let wrap = if token_name(cursor.current()) == "WRAP_DIRECTIVE" {
        let directive = cursor.advance().value.trim_start_matches(':');
        if directive.starts_with("nowrap:") {
            SequenceTextWrap::NoWrap
        } else {
            SequenceTextWrap::Wrap
        }
    } else {
        SequenceTextWrap::Default
    };
    (take_sequence_line_text(cursor), wrap)
}

fn ensure_sequence_participant(
    diagram: &mut SequenceDiagram,
    participant_indices: &mut HashMap<String, usize>,
    id: &str,
) {
    if !participant_indices.contains_key(id) {
        upsert_sequence_participant(
            diagram,
            participant_indices,
            id.to_string(),
            id.to_string(),
            SequenceTextWrap::Default,
            SequenceParticipantKind::Participant,
        );
    }
}

fn upsert_sequence_participant(
    diagram: &mut SequenceDiagram,
    participant_indices: &mut HashMap<String, usize>,
    id: String,
    label: String,
    label_wrap: SequenceTextWrap,
    kind: SequenceParticipantKind,
) {
    if let Some(&index) = participant_indices.get(&id) {
        diagram.participants[index].label = DiagramLabel::new(label);
        diagram.participants[index].label_wrap = label_wrap;
        diagram.participants[index].kind = kind;
        return;
    }
    participant_indices.insert(id.clone(), diagram.participants.len());
    diagram.participants.push(SequenceParticipant {
        id,
        label: DiagramLabel::new(label),
        label_wrap,
        kind,
        style: None,
        group_id: None,
        links: Vec::new(),
        properties: Vec::new(),
        details_reference: None,
    });
}

// ── pie parser ───────────────────────────────────────────────────────────

/// Parse the grammar-backed Mermaid `pie` family into a `ChartDiagram`.
///
/// The first compatibility slice supports `showData` and quoted numeric
/// sections, which are the semantic inputs needed by `diagram-layout-chart`.
pub fn parse_pie(source: &str) -> Result<ChartDiagram, ParseError> {
    parse_mermaid_pie_ast(source)?;

    let mut cursor = TokenCursor::new(tokenize_mermaid_pie(source));
    cursor.skip_terminators();
    cursor.expect_keyword("pie")?;

    if cursor.current().type_ == TokenType::Keyword && cursor.current().value == "showData" {
        cursor.advance();
    }
    cursor.skip_terminators();

    let mut slices = Vec::new();
    while !cursor.at_eof() {
        let label_token = cursor
            .consume_if("STRING")
            .ok_or_else(|| token_error(cursor.current(), "expected quoted pie slice label"))?;
        cursor
            .consume_if("COLON")
            .ok_or_else(|| token_error(cursor.current(), "expected ':' after pie slice label"))?;
        let value_token = cursor
            .consume_if("NUMBER")
            .ok_or_else(|| token_error(cursor.current(), "expected numeric pie slice value"))?;
        let value = value_token.value.parse::<f64>().map_err(|_| {
            token_error(
                &value_token,
                format!("invalid pie slice value {:?}", value_token.value),
            )
        })?;

        slices.push(PieSlice {
            label: unquote_mermaid_string(&label_token.value),
            value,
        });
        cursor.skip_terminators();
    }

    Ok(ChartDiagram {
        title: None,
        kind: ChartKind::Pie,
        x_axis: None,
        y_axis: None,
        series: vec![],
        slices,
        sankey_nodes: vec![],
        flows: vec![],
        orientation: ChartOrientation::Vertical,
    })
}

fn unquote_mermaid_string(raw: &str) -> String {
    let inner = raw
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(raw);
    let mut result = String::with_capacity(inner.len());
    let mut chars = inner.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }

        match chars.next() {
            Some('n') => result.push('\n'),
            Some('r') => result.push('\r'),
            Some('t') => result.push('\t'),
            Some('"') => result.push('"'),
            Some('\\') => result.push('\\'),
            Some(other) => result.push(other),
            None => result.push('\\'),
        }
    }

    result
}

// ── Sankey parser ─────────────────────────────────────────────────────────

/// Parse Mermaid's three-column Sankey CSV dialect into the shared chart IR.
pub fn parse_sankey(source: &str) -> Result<ChartDiagram, ParseError> {
    parse_mermaid_sankey_ast(source)?;

    let mut cursor = TokenCursor::new(tokenize_mermaid_sankey(source));
    cursor.skip_terminators();
    cursor
        .consume_if("HEADER")
        .ok_or_else(|| token_error(cursor.current(), "expected Sankey header"))?;
    cursor.skip_terminators();

    let mut nodes: Vec<SankeyNode> = Vec::new();
    let mut flows: Vec<SankeyFlow> = Vec::new();

    while !cursor.at_eof() {
        let source_id = parse_sankey_field(&mut cursor)?;
        cursor
            .consume_if("COMMA")
            .ok_or_else(|| token_error(cursor.current(), "expected ',' after Sankey source"))?;
        let target_id = parse_sankey_field(&mut cursor)?;
        cursor
            .consume_if("COMMA")
            .ok_or_else(|| token_error(cursor.current(), "expected ',' after Sankey target"))?;
        let weight_token = cursor
            .consume_if("NUMBER")
            .ok_or_else(|| token_error(cursor.current(), "expected Sankey flow weight"))?;
        let weight = weight_token.value.parse::<f64>().map_err(|_| {
            token_error(
                &weight_token,
                format!("invalid Sankey flow weight {:?}", weight_token.value),
            )
        })?;

        for id in [&source_id, &target_id] {
            if !nodes.iter().any(|node| node.id == *id) {
                nodes.push(SankeyNode {
                    id: id.clone(),
                    label: Some(id.clone()),
                });
            }
        }
        flows.push(SankeyFlow {
            source: source_id,
            target: target_id,
            weight,
        });
        cursor.skip_terminators();
    }

    Ok(ChartDiagram {
        title: None,
        kind: ChartKind::Sankey,
        x_axis: None,
        y_axis: None,
        series: vec![],
        slices: vec![],
        sankey_nodes: nodes,
        flows,
        orientation: ChartOrientation::Horizontal,
    })
}

fn parse_sankey_field(cursor: &mut TokenCursor) -> Result<String, ParseError> {
    let token = cursor.current().clone();
    if !matches!(token_name(&token), "STRING" | "BARE_FIELD" | "NUMBER") {
        return Err(token_error(&token, "expected Sankey CSV field"));
    }

    let value = cursor.advance().value.trim().replace("\"\"", "\"");
    if value.is_empty() {
        return Err(token_error(&token, "Sankey CSV fields cannot be empty"));
    }
    Ok(value)
}

// ── GitGraph parser ───────────────────────────────────────────────────────

/// Parse Mermaid GitGraph syntax into the shared temporal IR.
///
/// The grammar and temporal IR preserve Mermaid's complete command surface,
/// including branch order, commit types, and cherry-pick parent metadata.
pub fn parse_gitgraph(source: &str) -> Result<GitDiagram, ParseError> {
    parse_mermaid_gitgraph_ast(source)?;

    let mut cursor = TokenCursor::new(tokenize_mermaid_gitgraph(source));
    cursor.skip_terminators();
    cursor
        .consume_if("HEADER")
        .ok_or_else(|| token_error(cursor.current(), "expected GitGraph header"))?;

    let direction = cursor
        .consume_if("DIRECTION")
        .map(|token| direction_from_token(&token))
        .transpose()?
        .unwrap_or(DiagramDirection::Lr);
    cursor.consume_if("COLON");
    cursor.skip_terminators();

    let mut branches = vec![GitBranch {
        name: "main".to_string(),
        order: None,
    }];
    let mut events = Vec::new();
    let mut current_branch = "main".to_string();

    while !cursor.at_eof() {
        let command = cursor.current().clone();
        match command.value.as_str() {
            "commit" => {
                cursor.advance();
                let mut id = None;
                let mut message = None;
                let mut tag = None;
                let mut type_ = GitCommitType::Normal;
                while !gitgraph_statement_ended(&cursor) {
                    match token_name(cursor.current()) {
                        "ID_ATTR" => {
                            cursor.advance();
                            id = Some(parse_gitgraph_string(&mut cursor, "commit id")?);
                        }
                        "MSG_ATTR" => {
                            cursor.advance();
                            message = Some(parse_gitgraph_string(&mut cursor, "commit message")?);
                        }
                        "TAG_ATTR" => {
                            cursor.advance();
                            tag = Some(parse_gitgraph_string(&mut cursor, "commit tag")?);
                        }
                        "TYPE_ATTR" => {
                            cursor.advance();
                            type_ = parse_gitgraph_commit_type(&mut cursor)?;
                        }
                        "STRING" => {
                            message = Some(cursor.advance().value.clone());
                        }
                        _ => return Err(token_error(cursor.current(), "invalid commit attribute")),
                    }
                }
                events.push(GitEvent::Commit {
                    id,
                    message,
                    tag,
                    branch: current_branch.clone(),
                    type_,
                });
            }
            "branch" => {
                cursor.advance();
                let branch = parse_gitgraph_reference(&mut cursor)?;
                let mut order = None;
                if cursor.consume_if("ORDER_ATTR").is_some() {
                    let token = expect_gitgraph_token(&mut cursor, "INT", "branch order")?;
                    order = Some(token.value.parse::<i64>().map_err(|_| {
                        token_error(&token, format!("invalid branch order {:?}", token.value))
                    })?);
                }
                if !branches.iter().any(|candidate| candidate.name == branch) {
                    branches.push(GitBranch {
                        name: branch,
                        order,
                    });
                }
            }
            "checkout" | "switch" => {
                cursor.advance();
                let branch = parse_gitgraph_reference(&mut cursor)?;
                if !branches.iter().any(|candidate| candidate.name == branch) {
                    return Err(token_error(
                        &command,
                        format!("cannot checkout unknown GitGraph branch {branch:?}"),
                    ));
                }
                current_branch = branch.clone();
                events.push(GitEvent::Checkout { branch });
            }
            "merge" => {
                cursor.advance();
                let from = parse_gitgraph_reference(&mut cursor)?;
                let mut id = None;
                let mut tag = None;
                let mut type_ = GitCommitType::Normal;
                while !gitgraph_statement_ended(&cursor) {
                    match token_name(cursor.current()) {
                        "ID_ATTR" => {
                            cursor.advance();
                            id = Some(parse_gitgraph_string(&mut cursor, "merge id")?);
                        }
                        "TAG_ATTR" => {
                            cursor.advance();
                            tag = Some(parse_gitgraph_string(&mut cursor, "merge tag")?);
                        }
                        "TYPE_ATTR" => {
                            cursor.advance();
                            type_ = parse_gitgraph_commit_type(&mut cursor)?;
                        }
                        _ => return Err(token_error(cursor.current(), "invalid merge attribute")),
                    }
                }
                events.push(GitEvent::Merge {
                    from,
                    id,
                    tag,
                    type_,
                });
            }
            "cherry-pick" => {
                cursor.advance();
                let mut id = None;
                let mut tag = None;
                let mut parent = None;
                while !gitgraph_statement_ended(&cursor) {
                    match token_name(cursor.current()) {
                        "ID_ATTR" => {
                            cursor.advance();
                            id = Some(parse_gitgraph_string(&mut cursor, "cherry-pick id")?);
                        }
                        "TAG_ATTR" => {
                            cursor.advance();
                            tag = Some(parse_gitgraph_string(&mut cursor, "cherry-pick tag")?);
                        }
                        "PARENT_ATTR" => {
                            cursor.advance();
                            parent =
                                Some(parse_gitgraph_string(&mut cursor, "cherry-pick parent")?);
                        }
                        _ => {
                            return Err(token_error(
                                cursor.current(),
                                "invalid cherry-pick attribute",
                            ));
                        }
                    }
                }
                let id = id.ok_or_else(|| {
                    token_error(&command, "GitGraph cherry-pick requires an id attribute")
                })?;
                events.push(GitEvent::CherryPick {
                    id,
                    tag,
                    parent,
                    branch: current_branch.clone(),
                });
            }
            _ => {
                return Err(token_error(
                    &command,
                    format!("unsupported GitGraph command {:?}", command.value),
                ));
            }
        }
        cursor.skip_terminators();
    }

    Ok(GitDiagram {
        direction,
        branches,
        events,
    })
}

fn gitgraph_statement_ended(cursor: &TokenCursor) -> bool {
    cursor.at_eof() || token_name(cursor.current()) == "NEWLINE"
}

fn parse_gitgraph_reference(cursor: &mut TokenCursor) -> Result<String, ParseError> {
    let token = cursor.current().clone();
    if !matches!(token_name(&token), "REFERENCE" | "STRING") {
        return Err(token_error(&token, "expected GitGraph reference"));
    }
    Ok(cursor.advance().value.clone())
}

fn parse_gitgraph_string(
    cursor: &mut TokenCursor,
    description: &str,
) -> Result<String, ParseError> {
    Ok(expect_gitgraph_token(cursor, "STRING", description)?.value)
}

fn parse_gitgraph_commit_type(cursor: &mut TokenCursor) -> Result<GitCommitType, ParseError> {
    let token = expect_gitgraph_token(cursor, "COMMIT_TYPE", "commit type")?;
    match token.value.as_str() {
        "NORMAL" => Ok(GitCommitType::Normal),
        "REVERSE" => Ok(GitCommitType::Reverse),
        "HIGHLIGHT" => Ok(GitCommitType::Highlight),
        _ => Err(token_error(
            &token,
            format!("invalid GitGraph commit type {:?}", token.value),
        )),
    }
}

fn expect_gitgraph_token(
    cursor: &mut TokenCursor,
    name: &str,
    description: &str,
) -> Result<Token, ParseError> {
    cursor
        .consume_if(name)
        .ok_or_else(|| token_error(cursor.current(), format!("expected GitGraph {description}")))
}

// ── Entity-relationship parser ───────────────────────────────────────────

/// Parse Mermaid ER syntax into the shared structural IR.
pub fn parse_er_diagram(source: &str) -> Result<StructuralDiagram, ParseError> {
    parse_mermaid_er_ast(source)?;

    let mut cursor = TokenCursor::new(tokenize_mermaid_er(source));
    cursor.skip_terminators();
    cursor
        .consume_if("HEADER")
        .ok_or_else(|| token_error(cursor.current(), "expected erDiagram header"))?;
    cursor.skip_terminators();

    let mut title = None;
    let mut nodes: Vec<StructuralNode> = Vec::new();
    let mut node_indices: HashMap<String, usize> = HashMap::new();
    let mut relationships = Vec::new();

    while !cursor.at_eof() {
        if cursor.current().type_ == TokenType::Keyword {
            match cursor.current().value.as_str() {
                "title" => {
                    cursor.advance();
                    title = Some(parse_er_line_text(&mut cursor));
                }
                "direction" | "classDef" | "class" | "style" => {
                    while !cursor.at_eof() && token_name(cursor.current()) != "NEWLINE" {
                        cursor.advance();
                    }
                }
                _ => return Err(token_error(cursor.current(), "unsupported ER statement")),
            }
            cursor.skip_terminators();
            continue;
        }

        let entity_id = parse_er_name(&mut cursor)?;
        let mut entity_label = entity_id.clone();
        if cursor.consume_if("LBRACKET").is_some() {
            entity_label = parse_er_name(&mut cursor)?;
            cursor
                .consume_if("RBRACKET")
                .ok_or_else(|| token_error(cursor.current(), "expected ']' after ER alias"))?;
        }
        consume_er_class_suffix(&mut cursor)?;

        if is_er_cardinality(cursor.current()) {
            let from_mult = parse_er_cardinality(&mut cursor)?;
            let relation_token = cursor.current().clone();
            if !matches!(
                token_name(&relation_token),
                "IDENTIFYING" | "NON_IDENTIFYING"
            ) {
                return Err(token_error(
                    &relation_token,
                    "expected ER relationship type",
                ));
            }
            cursor.advance();
            let to_mult = parse_er_cardinality(&mut cursor)?;
            let target_id = parse_er_name(&mut cursor)?;
            consume_er_class_suffix(&mut cursor)?;
            cursor.consume_if("COLON").ok_or_else(|| {
                token_error(cursor.current(), "expected ':' before ER relationship role")
            })?;
            let label = parse_er_line_text(&mut cursor);

            upsert_er_node(
                &mut nodes,
                &mut node_indices,
                entity_id.clone(),
                entity_label,
            );
            upsert_er_node(
                &mut nodes,
                &mut node_indices,
                target_id.clone(),
                target_id.clone(),
            );
            relationships.push(StructuralRelationship {
                from: entity_id,
                to: target_id,
                kind: if token_name(&relation_token) == "IDENTIFYING" {
                    RelKind::Association
                } else {
                    RelKind::Dependency
                },
                from_mult: Some(from_mult),
                to_mult: Some(to_mult),
                label: (!label.is_empty()).then_some(label),
            });
        } else {
            let mut fields = Vec::new();
            if cursor.consume_if("LBRACE").is_some() {
                cursor.skip_terminators();
                while cursor.consume_if("RBRACE").is_none() {
                    if cursor.at_eof() {
                        return Err(token_error(
                            cursor.current(),
                            "unterminated ER attribute block",
                        ));
                    }
                    let mut attribute_type = parse_er_name(&mut cursor)?;
                    if cursor.consume_if("LBRACKET").is_some() {
                        cursor.consume_if("RBRACKET").ok_or_else(|| {
                            token_error(cursor.current(), "expected ']' in ER attribute type")
                        })?;
                        attribute_type.push_str("[]");
                    }
                    if cursor.consume_if("QUESTION").is_some() {
                        attribute_type.push('?');
                    }
                    let attribute_name = parse_er_name(&mut cursor)?;
                    let mut keys = Vec::new();
                    while token_name(cursor.current()) == "ATTRIBUTE_KEY" {
                        keys.push(cursor.advance().value.clone());
                        if cursor.consume_if("COMMA").is_none() {
                            break;
                        }
                    }
                    let comment = cursor.consume_if("STRING").map(|token| token.value);
                    let mut field = format!("{attribute_name}: {attribute_type}");
                    if !keys.is_empty() {
                        field.push_str(&format!(" [{}]", keys.join(", ")));
                    }
                    if let Some(comment) = comment {
                        field.push_str(&format!(" - {comment}"));
                    }
                    fields.push(field);
                    cursor.skip_terminators();
                }
            }
            let index = upsert_er_node(&mut nodes, &mut node_indices, entity_id, entity_label);
            if !fields.is_empty() {
                nodes[index].compartments.push(Compartment {
                    kind: CompartmentKind::Fields,
                    entries: fields,
                });
            }
        }
        cursor.skip_terminators();
    }

    Ok(StructuralDiagram {
        kind: StructuralKind::Er,
        title,
        nodes,
        groups: vec![],
        relationships,
    })
}

fn parse_er_name(cursor: &mut TokenCursor) -> Result<String, ParseError> {
    let token = cursor.current().clone();
    if !matches!(
        token_name(&token),
        "IDENTIFIER" | "NUMBER" | "ATTRIBUTE_KEY" | "MD_PARENT" | "STRING"
    ) {
        return Err(token_error(&token, "expected ER name"));
    }
    Ok(cursor.advance().value.clone())
}

fn consume_er_class_suffix(cursor: &mut TokenCursor) -> Result<(), ParseError> {
    if cursor.consume_if("STYLE_SEPARATOR").is_none() {
        return Ok(());
    }
    parse_er_name(cursor)?;
    while cursor.consume_if("COMMA").is_some() {
        parse_er_name(cursor)?;
    }
    Ok(())
}

fn is_er_cardinality(token: &Token) -> bool {
    matches!(
        token_name(token),
        "ZERO_OR_ONE" | "ZERO_OR_MORE" | "ONE_OR_MORE" | "ONLY_ONE" | "MD_PARENT"
    )
}

fn parse_er_cardinality(cursor: &mut TokenCursor) -> Result<String, ParseError> {
    let token = cursor.current().clone();
    let normalized = match token_name(&token) {
        "ZERO_OR_ONE" => "0..1",
        "ZERO_OR_MORE" => "0..*",
        "ONE_OR_MORE" => "1..*",
        "ONLY_ONE" => "1",
        "MD_PARENT" => "parent",
        _ => return Err(token_error(&token, "expected ER cardinality")),
    };
    cursor.advance();
    Ok(normalized.to_string())
}

fn parse_er_line_text(cursor: &mut TokenCursor) -> String {
    let mut words = Vec::new();
    while !cursor.at_eof() && token_name(cursor.current()) != "NEWLINE" {
        words.push(cursor.advance().value.clone());
    }
    words.join(" ")
}

fn upsert_er_node(
    nodes: &mut Vec<StructuralNode>,
    indices: &mut HashMap<String, usize>,
    id: String,
    label: String,
) -> usize {
    if let Some(index) = indices.get(&id).copied() {
        if label != id {
            nodes[index].label = label;
        }
        return index;
    }
    let index = nodes.len();
    indices.insert(id.clone(), index);
    nodes.push(StructuralNode {
        id,
        label,
        stereotype: Some("entity".to_string()),
        node_kind: StructuralNodeKind::Entity,
        compartments: Vec::new(),
        parent_group: None,
    });
    index
}

// ── C4 parser ─────────────────────────────────────────────────────────────

/// Parse Mermaid C4 macros into the shared structural IR.
pub fn parse_c4_diagram(source: &str) -> Result<StructuralDiagram, ParseError> {
    parse_mermaid_c4_ast(source)?;

    let mut cursor = TokenCursor::new(tokenize_mermaid_c4(source));
    cursor.skip_terminators();
    cursor
        .consume_if("HEADER")
        .ok_or_else(|| token_error(cursor.current(), "expected C4 diagram header"))?;
    cursor.skip_terminators();

    let mut title = None;
    let mut nodes = Vec::new();
    let mut node_indices = HashMap::new();
    let mut groups = Vec::new();
    let mut group_stack: Vec<String> = Vec::new();
    let mut relationships = Vec::new();

    while !cursor.at_eof() {
        match token_name(cursor.current()) {
            "TITLE" => {
                let value = cursor.advance().value.clone();
                title = Some(value.trim_start_matches("title").trim().to_string());
            }
            "DIRECTION" | "CONFIG_MACRO" => {
                cursor.advance();
                if token_name(cursor.current()) == "LPAREN" {
                    parse_c4_arguments(&mut cursor)?;
                }
            }
            "RBRACE" => {
                cursor.advance();
                group_stack.pop();
            }
            "BOUNDARY_MACRO" => {
                let macro_token = cursor.advance().clone();
                let args = parse_c4_arguments(&mut cursor)?;
                let id = args.first().cloned().unwrap_or_default();
                if id.is_empty() {
                    return Err(token_error(&macro_token, "C4 boundary requires an alias"));
                }
                let label = args
                    .get(1)
                    .filter(|value| !value.is_empty())
                    .cloned()
                    .unwrap_or_else(|| id.clone());
                groups.push(StructuralGroup {
                    id: id.clone(),
                    label,
                    stereotype: Some(macro_token.value),
                    parent_group: group_stack.last().cloned(),
                });
                cursor.skip_terminators();
                cursor.consume_if("LBRACE").ok_or_else(|| {
                    token_error(cursor.current(), "expected '{' after C4 boundary")
                })?;
                group_stack.push(id);
            }
            "ELEMENT_MACRO" => {
                let macro_token = cursor.advance().clone();
                let args = parse_c4_arguments(&mut cursor)?;
                let id = args.first().cloned().unwrap_or_default();
                if id.is_empty() {
                    return Err(token_error(&macro_token, "C4 element requires an alias"));
                }
                let label = args
                    .get(1)
                    .filter(|value| !value.is_empty())
                    .cloned()
                    .unwrap_or_else(|| id.clone());
                let index = upsert_c4_node(
                    &mut nodes,
                    &mut node_indices,
                    id,
                    label,
                    macro_token.value,
                    group_stack.last().cloned(),
                );
                let details: Vec<String> = args
                    .iter()
                    .skip(2)
                    .filter(|value| !value.is_empty() && !value.starts_with('$'))
                    .cloned()
                    .collect();
                if !details.is_empty() {
                    nodes[index].compartments.push(Compartment {
                        kind: CompartmentKind::Fields,
                        entries: details,
                    });
                }
            }
            "RELATION_MACRO" => {
                let macro_token = cursor.advance().clone();
                let args = parse_c4_arguments(&mut cursor)?;
                if args.len() < 3 {
                    return Err(token_error(
                        &macro_token,
                        "C4 relationship requires source, target, and label",
                    ));
                }
                relationships.push(StructuralRelationship {
                    from: args[0].clone(),
                    to: args[1].clone(),
                    kind: RelKind::Association,
                    from_mult: None,
                    to_mult: None,
                    label: Some(args[2].clone()),
                });
            }
            _ => return Err(token_error(cursor.current(), "unsupported C4 statement")),
        }
        cursor.skip_terminators();
    }

    Ok(StructuralDiagram {
        kind: StructuralKind::C4,
        title,
        nodes,
        groups,
        relationships,
    })
}

fn parse_c4_arguments(cursor: &mut TokenCursor) -> Result<Vec<String>, ParseError> {
    cursor
        .consume_if("LPAREN")
        .ok_or_else(|| token_error(cursor.current(), "expected '(' after C4 macro"))?;
    let mut args = Vec::new();
    let mut needs_value = true;

    while token_name(cursor.current()) != "RPAREN" {
        if cursor.at_eof() || token_name(cursor.current()) == "NEWLINE" {
            return Err(token_error(cursor.current(), "unterminated C4 arguments"));
        }
        if cursor.consume_if("COMMA").is_some() {
            if needs_value {
                args.push(String::new());
            }
            needs_value = true;
            continue;
        }

        let token = cursor.current().clone();
        let value = match token_name(&token) {
            "KV_KEY" => {
                cursor.advance();
                cursor.consume_if("EQUALS").ok_or_else(|| {
                    token_error(cursor.current(), "expected '=' in C4 keyed argument")
                })?;
                let value = parse_c4_argument_value(cursor)?;
                format!("{}={value}", token.value)
            }
            "STRING" | "IDENTIFIER" | "NUMBER" => parse_c4_argument_value(cursor)?,
            _ => return Err(token_error(&token, "expected C4 argument")),
        };
        args.push(value);
        needs_value = false;
    }
    cursor.advance();
    Ok(args)
}

fn parse_c4_argument_value(cursor: &mut TokenCursor) -> Result<String, ParseError> {
    let token = cursor.current().clone();
    if !matches!(token_name(&token), "STRING" | "IDENTIFIER" | "NUMBER") {
        return Err(token_error(&token, "expected C4 argument value"));
    }
    Ok(cursor.advance().value.clone())
}

fn upsert_c4_node(
    nodes: &mut Vec<StructuralNode>,
    indices: &mut HashMap<String, usize>,
    id: String,
    label: String,
    stereotype: String,
    parent_group: Option<String>,
) -> usize {
    if let Some(index) = indices.get(&id).copied() {
        return index;
    }
    let index = nodes.len();
    indices.insert(id.clone(), index);
    nodes.push(StructuralNode {
        id,
        label,
        stereotype: Some(stereotype),
        node_kind: StructuralNodeKind::Class,
        compartments: Vec::new(),
        parent_group,
    });
    index
}

// ── gantt parser ──────────────────────────────────────────────────────────

/// Parse a Mermaid `gantt` block into a `GanttDiagram`.
///
/// Handles:
/// ```text
/// gantt
///   title Project Timeline
///   dateFormat YYYY-MM-DD
///   section Phase 1
///     Task A :done, t1, 2026-01-01, 5d
///     Task B :t2, after t1, 3d
/// ```
pub fn parse_gantt(source: &str) -> Result<GanttDiagram, ParseError> {
    let mut date_format = "YYYY-MM-DD".to_string();
    let mut sections: Vec<GanttSection> = Vec::new();
    let mut current_section: Option<GanttSection> = None;

    let mut past_header = false;
    for line in source.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("%%") {
            continue;
        }
        if !past_header {
            if t == "gantt" {
                past_header = true;
            }
            continue;
        }
        if t.starts_with("title") {
            // title is ignored at GanttDiagram level (held at TemporalDiagram)
            continue;
        } else if t.starts_with("dateFormat") {
            date_format = t[10..].trim().to_string();
        } else if t.starts_with("section") {
            if let Some(sec) = current_section.take() {
                sections.push(sec);
            }
            current_section = Some(GanttSection {
                label: Some(t[7..].trim().to_string()),
                tasks: vec![],
            });
        } else if t.contains(':') {
            if let Some(task) = parse_gantt_task(t) {
                let sec = current_section.get_or_insert_with(|| GanttSection {
                    label: None,
                    tasks: vec![],
                });
                sec.tasks.push(task);
            }
        }
    }
    if let Some(sec) = current_section {
        sections.push(sec);
    }

    Ok(GanttDiagram {
        date_format,
        sections,
    })
}

/// Parse a single Gantt task line.
///
/// Format: `label :status, id, start, duration`
///    or   `label :id, start, duration`
fn parse_gantt_task(line: &str) -> Option<GanttTask> {
    let colon = line.find(':')?;
    let label = line[..colon].trim().to_string();
    let rest = line[colon + 1..].trim();

    let parts: Vec<&str> = rest.splitn(4, ',').map(str::trim).collect();
    if parts.is_empty() {
        return None;
    }

    // Detect status keywords in the first part.
    let status_keywords = ["done", "active", "crit", "milestone"];
    let first = parts[0];
    let (status, remaining) = if status_keywords.contains(&first) {
        (parse_task_status(first), &parts[1..])
    } else {
        (TaskStatus::Normal, &parts[..])
    };

    if remaining.is_empty() {
        return None;
    }
    let id = remaining[0].to_string();
    let start = if remaining.len() > 1 {
        let s = remaining[1];
        if s.starts_with("after ") {
            TaskStart::After(s[6..].trim().to_string())
        } else {
            TaskStart::Date(s.to_string())
        }
    } else {
        TaskStart::Date("2026-01-01".to_string())
    };
    let duration_days = if remaining.len() > 2 {
        parse_duration(remaining[2]).unwrap_or(1.0)
    } else {
        1.0
    };

    Some(GanttTask {
        id,
        label,
        start,
        duration_days,
        status,
        dependencies: vec![],
    })
}

fn parse_task_status(s: &str) -> TaskStatus {
    match s {
        "done" => TaskStatus::Done,
        "active" => TaskStatus::Active,
        "crit" => TaskStatus::Crit,
        "milestone" => TaskStatus::Milestone,
        _ => TaskStatus::Normal,
    }
}

fn parse_duration(s: &str) -> Option<f64> {
    let s = s.trim();
    if let Some(rest) = s.strip_suffix('d') {
        rest.parse().ok()
    } else if let Some(rest) = s.strip_suffix('w') {
        rest.parse::<f64>().ok().map(|w| w * 7.0)
    } else if let Some(rest) = s.strip_suffix('h') {
        rest.parse::<f64>().ok().map(|h| h / 24.0)
    } else {
        s.parse().ok()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests_dg04 {
    use super::*;

    const CLASS_SRC: &str = "classDiagram
  class Animal { +name: String; +speak() void }
  class Dog
  Animal <|-- Dog : extends";

    const XYCHART_SRC: &str = "xychart-beta
  title \"Q1 Sales\"
  x-axis [Jan, Feb, Mar]
  y-axis 0 --> 100
  bar [40, 60, 45]
  line [35, 55, 48]";

    const GANTT_SRC: &str = "gantt
  title Project
  dateFormat YYYY-MM-DD
  section Phase 1
    Design :done, t1, 2026-01-01, 5d
    Build :t2, after t1, 3d";

    const PIE_SRC: &str = "pie showData
  \"Dogs\" : 60
  \"Cats\" : 40";

    const SANKEY_SRC: &str = "sankey
Grid,\"Heating, homes\",113.726
Grid,Losses,56";

    const GITGRAPH_SRC: &str = "gitGraph LR:
commit id: \"root\" msg: \"Initial commit\"
branch develop order: 1
checkout develop
commit id: \"feature\" tag: \"v1\"
checkout main
merge develop id: \"merge-1\"";

    const ER_SRC: &str = "erDiagram
CUSTOMER ||--o{ ORDER : places
CUSTOMER {
string name PK \"display name\"
string email UK
}
ORDER[Purchase] {
int id PK
}";

    const C4_SRC: &str = "C4Context
title Banking System
Person(customer, \"Customer\", \"Uses online banking\")
System_Boundary(bank, \"Bank\") {
System(web, \"Internet Banking\", \"Handles accounts\")
}
Rel(customer, web, \"Uses\", \"HTTPS\")";

    #[test]
    fn class_diagram_parses_nodes() {
        let d = parse_class_diagram(CLASS_SRC).unwrap();
        assert_eq!(d.nodes.len(), 2);
        assert!(d.nodes.iter().any(|n| n.id == "Animal"));
        assert!(d.nodes.iter().any(|n| n.id == "Dog"));
    }

    #[test]
    fn class_diagram_parses_relationship() {
        let d = parse_class_diagram(CLASS_SRC).unwrap();
        assert_eq!(d.relationships.len(), 1);
        assert_eq!(d.relationships[0].kind, RelKind::Inheritance);
    }

    #[test]
    fn class_diagram_compartments() {
        let d = parse_class_diagram(CLASS_SRC).unwrap();
        let animal = d.nodes.iter().find(|n| n.id == "Animal").unwrap();
        assert!(!animal.compartments.is_empty());
    }

    #[test]
    fn xychart_parses_title() {
        let d = parse_xychart(XYCHART_SRC).unwrap();
        assert_eq!(d.title.as_deref(), Some("Q1 Sales"));
    }

    #[test]
    fn xychart_parses_categories() {
        let d = parse_xychart(XYCHART_SRC).unwrap();
        let cats = d.x_axis.as_ref().unwrap().categories.clone();
        assert_eq!(cats, vec!["Jan", "Feb", "Mar"]);
    }

    #[test]
    fn xychart_parses_series() {
        let d = parse_xychart(XYCHART_SRC).unwrap();
        assert_eq!(d.series.len(), 2);
        let bar = d.series.iter().find(|s| s.kind == SeriesKind::Bar).unwrap();
        assert_eq!(bar.data, vec![40.0, 60.0, 45.0]);
    }

    #[test]
    fn gantt_parses_sections() {
        let d = parse_gantt(GANTT_SRC).unwrap();
        assert_eq!(d.sections.len(), 1);
        assert_eq!(d.sections[0].label.as_deref(), Some("Phase 1"));
    }

    #[test]
    fn gantt_parses_tasks() {
        let d = parse_gantt(GANTT_SRC).unwrap();
        assert_eq!(d.sections[0].tasks.len(), 2);
    }

    #[test]
    fn gantt_resolves_after_dependency() {
        let d = parse_gantt(GANTT_SRC).unwrap();
        let t2 = &d.sections[0].tasks[1];
        assert_eq!(t2.id, "t2");
        assert!(matches!(t2.start, TaskStart::After(_)));
    }

    #[test]
    fn gantt_parses_status() {
        let d = parse_gantt(GANTT_SRC).unwrap();
        assert_eq!(d.sections[0].tasks[0].status, TaskStatus::Done);
    }

    #[test]
    fn pie_parses_slices() {
        let d = parse_pie(PIE_SRC).unwrap();
        assert_eq!(d.kind, ChartKind::Pie);
        assert_eq!(d.slices.len(), 2);
        assert_eq!(d.slices[0].label, "Dogs");
        assert_eq!(d.slices[0].value, 60.0);
    }

    #[test]
    fn sankey_parses_csv_flows_and_nodes() {
        let d = parse_sankey(SANKEY_SRC).unwrap();
        assert_eq!(d.kind, ChartKind::Sankey);
        assert_eq!(d.flows.len(), 2);
        assert_eq!(d.sankey_nodes.len(), 3);
        assert_eq!(d.flows[0].target, "Heating, homes");
        assert_eq!(d.flows[0].weight, 113.726);
    }

    #[test]
    fn gitgraph_parses_branch_history() {
        let d = parse_gitgraph(GITGRAPH_SRC).unwrap();
        assert_eq!(d.direction, DiagramDirection::Lr);
        assert_eq!(d.branches.len(), 2);
        assert_eq!(d.events.len(), 5);
        assert!(matches!(
            &d.events[1],
            GitEvent::Checkout { branch } if branch == "develop"
        ));
        assert!(matches!(
            &d.events[4],
            GitEvent::Merge { from, id, .. }
                if from == "develop" && id.as_deref() == Some("merge-1")
        ));
    }

    #[test]
    fn gitgraph_parses_cherry_pick_metadata() {
        let d = parse_gitgraph(
            "gitGraph\ncommit id: \"abc123\"\ncherry-pick id: \"abc123\" parent: \"root\"",
        )
        .unwrap();
        assert!(matches!(
            &d.events[1],
            GitEvent::CherryPick { id, parent, branch, .. }
                if id == "abc123"
                    && parent.as_deref() == Some("root")
                    && branch == "main"
        ));
    }

    #[test]
    fn er_parses_entities_attributes_and_cardinalities() {
        let d = parse_er_diagram(ER_SRC).unwrap();
        assert_eq!(d.kind, StructuralKind::Er);
        assert_eq!(
            d.nodes
                .iter()
                .map(|node| node.id.as_str())
                .collect::<Vec<_>>(),
            vec!["CUSTOMER", "ORDER"]
        );
        assert_eq!(d.relationships.len(), 1);
        assert_eq!(d.relationships[0].from_mult.as_deref(), Some("1"));
        assert_eq!(d.relationships[0].to_mult.as_deref(), Some("0..*"));
        let customer = d.nodes.iter().find(|node| node.id == "CUSTOMER").unwrap();
        assert_eq!(customer.compartments[0].entries.len(), 2);
        let order = d.nodes.iter().find(|node| node.id == "ORDER").unwrap();
        assert_eq!(order.label, "Purchase");
    }

    #[test]
    fn c4_parses_elements_boundaries_and_relationships() {
        let d = parse_c4_diagram(C4_SRC).unwrap();
        assert_eq!(d.kind, StructuralKind::C4);
        assert_eq!(d.title.as_deref(), Some("Banking System"));
        assert_eq!(d.nodes.len(), 2);
        assert_eq!(d.groups.len(), 1);
        assert_eq!(d.groups[0].id, "bank");
        assert_eq!(
            d.nodes
                .iter()
                .find(|node| node.id == "web")
                .and_then(|node| node.parent_group.as_deref()),
            Some("bank")
        );
        assert_eq!(d.relationships.len(), 1);
        assert_eq!(d.relationships[0].from, "customer");
        assert_eq!(d.relationships[0].to, "web");
    }

    #[test]
    fn c4_preserves_nested_boundary_membership() {
        let d = parse_c4_diagram(
            "C4Deployment\nDeployment_Node(cloud, \"Cloud\") {\nContainer_Boundary(apps, \"Apps\") {\nContainer(api, \"API\", \"Rust\")\n}\n}",
        )
        .unwrap();
        assert_eq!(d.groups.len(), 2);
        assert_eq!(d.groups[1].parent_group.as_deref(), Some("cloud"));
        assert_eq!(d.nodes[0].parent_group.as_deref(), Some("apps"));
    }

    #[test]
    fn dispatch_flowchart() {
        let src = "flowchart LR\n  A --> B";
        match parse_any_mermaid(src).unwrap() {
            MermaidDiagram::Graph(_) => {}
            _ => panic!("expected Graph"),
        }
    }

    #[test]
    fn dispatch_class_diagram() {
        let src = "classDiagram\n  class Foo";
        match parse_any_mermaid(src).unwrap() {
            MermaidDiagram::Structural(_) => {}
            _ => panic!("expected Structural"),
        }
    }

    #[test]
    fn dispatch_xychart() {
        let src = "xychart-beta\n  bar [1,2,3]";
        match parse_any_mermaid(src).unwrap() {
            MermaidDiagram::Chart(_) => {}
            _ => panic!("expected Chart"),
        }
    }

    #[test]
    fn dispatch_gantt() {
        let src = "gantt\n  dateFormat YYYY-MM-DD";
        match parse_any_mermaid(src).unwrap() {
            MermaidDiagram::Temporal(_) => {}
            _ => panic!("expected Temporal"),
        }
    }

    #[test]
    fn dispatch_pie() {
        match parse_any_mermaid(PIE_SRC).unwrap() {
            MermaidDiagram::Chart(chart) => assert_eq!(chart.kind, ChartKind::Pie),
            _ => panic!("expected Chart"),
        }
    }

    #[test]
    fn dispatch_sankey() {
        match parse_any_mermaid(SANKEY_SRC).unwrap() {
            MermaidDiagram::Chart(chart) => assert_eq!(chart.kind, ChartKind::Sankey),
            _ => panic!("expected Chart"),
        }
    }

    #[test]
    fn dispatch_gitgraph() {
        match parse_any_mermaid(GITGRAPH_SRC).unwrap() {
            MermaidDiagram::Temporal(diagram) => assert_eq!(diagram.kind, TemporalKind::Git),
            _ => panic!("expected Temporal"),
        }
    }

    #[test]
    fn dispatch_er() {
        match parse_any_mermaid(ER_SRC).unwrap() {
            MermaidDiagram::Structural(diagram) => assert_eq!(diagram.kind, StructuralKind::Er),
            _ => panic!("expected Structural"),
        }
    }

    #[test]
    fn dispatch_c4() {
        match parse_any_mermaid(C4_SRC).unwrap() {
            MermaidDiagram::Structural(diagram) => assert_eq!(diagram.kind, StructuralKind::C4),
            _ => panic!("expected Structural"),
        }
    }

    #[test]
    fn sequence_parses_core_events_and_implicit_participants() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nautonumber\nparticipant A as Alice\nA->>+Bob: Hello Bob\nnote right of Bob: Ready\ndeactivate Bob\n",
        )
        .unwrap();
        assert!(diagram.auto_number);
        assert_eq!(diagram.participants.len(), 2);
        assert_eq!(diagram.participants[0].label.text, "Alice");
        assert!(matches!(
            &diagram.events[0],
            SequenceEvent::AutoNumber { visible: true, .. }
        ));
        assert!(matches!(
            &diagram.events[1],
            SequenceEvent::Message { from, to, activate: true, .. }
                if from == "A" && to == "Bob"
        ));
        assert!(matches!(
            &diagram.events[2],
            SequenceEvent::Note {
                placement: SequenceNotePlacement::RightOf,
                ..
            }
        ));
        assert!(matches!(
            &diagram.events[3],
            SequenceEvent::Activation { participant, active: false } if participant == "Bob"
        ));
    }

    #[test]
    fn dispatch_sequence() {
        match parse_any_mermaid("sequenceDiagram\nAlice-->>Bob: Hello").unwrap() {
            MermaidDiagram::Sequence(diagram) => assert_eq!(diagram.events.len(), 1),
            _ => panic!("expected Sequence"),
        }
    }

    #[test]
    fn state_parses_graph_compatible_core() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\ndirection LR\nstate \"Still waiting\" as Still\n[*] --> Still\nStill --> Moving: begin motion\nMoving: In motion\nMoving --> [*]: stop\n",
        )
        .expect("state core should parse");

        assert_eq!(diagram.direction, DiagramDirection::Lr);
        assert_eq!(diagram.nodes.len(), 4);
        assert_eq!(diagram.edges.len(), 3);
        assert_eq!(
            diagram
                .nodes
                .iter()
                .find(|node| node.id == "Still")
                .unwrap()
                .label
                .text,
            "Still waiting"
        );
        assert_eq!(
            diagram.edges[1].label.as_ref().unwrap().text,
            "begin motion"
        );
        assert_eq!(
            diagram
                .nodes
                .iter()
                .find(|node| node.id == "Moving")
                .unwrap()
                .label
                .text,
            "In motion"
        );
        assert_eq!(
            diagram
                .nodes
                .iter()
                .filter(|node| node.shape == Some(DiagramShape::Ellipse))
                .count(),
            2
        );
    }

    #[test]
    fn dispatch_state_to_graph_ir() {
        match parse_any_mermaid("stateDiagram\nReady --> Running").unwrap() {
            MermaidDiagram::Graph(diagram) => assert_eq!(diagram.edges.len(), 1),
            _ => panic!("expected graph-compatible state diagram"),
        }
    }

    #[test]
    fn state_parses_choice_pseudostates() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\nstate First <<choice>>\nstate Second [[choice]]\nReady --> First\nFirst --> Second: continue\n",
        )
        .expect("choice pseudostates should parse");

        for id in ["First", "Second"] {
            let node = diagram.nodes.iter().find(|node| node.id == id).unwrap();
            assert_eq!(node.shape, Some(DiagramShape::Diamond));
            assert_eq!(node.label.text, "");
        }
    }

    #[test]
    fn state_parses_fork_and_join_pseudostates() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\nstate WorkFork <<fork>>\nstate WorkJoin [[join]]\nReady --> WorkFork\nWorkFork --> Running\nRunning --> WorkJoin\n",
        )
        .expect("fork and join pseudostates should parse");

        for id in ["WorkFork", "WorkJoin"] {
            let node = diagram.nodes.iter().find(|node| node.id == id).unwrap();
            assert_eq!(node.shape, Some(DiagramShape::Bar));
            assert_eq!(
                node.style.as_ref().unwrap().fill.as_deref(),
                Some("#111827")
            );
        }
    }

    #[test]
    fn state_parses_inline_styles() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\nReady --> Running\nstyle Ready fill:#fee2e2,stroke:#991b1b,color:#111827,stroke-width:3px\n",
        )
        .expect("state inline styles should parse");
        let style = diagram
            .nodes
            .iter()
            .find(|node| node.id == "Ready")
            .unwrap()
            .style
            .as_ref()
            .unwrap();

        assert_eq!(style.fill.as_deref(), Some("#fee2e2"));
        assert_eq!(style.stroke.as_deref(), Some("#991b1b"));
        assert_eq!(style.text_color.as_deref(), Some("#111827"));
        assert_eq!(style.stroke_width, Some(3.0));
    }

    #[test]
    fn state_resolves_reusable_style_classes() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\nclass Ready,Waiting warning\nclassDef warning fill:#fef3c7,stroke:#92400e,color:#451a03,stroke-width:2px\nReady --> Waiting\n",
        )
        .expect("state style classes should parse");

        for id in ["Ready", "Waiting"] {
            let style = diagram
                .nodes
                .iter()
                .find(|node| node.id == id)
                .unwrap()
                .style
                .as_ref()
                .unwrap();
            assert_eq!(style.fill.as_deref(), Some("#fef3c7"));
            assert_eq!(style.stroke.as_deref(), Some("#92400e"));
            assert_eq!(style.text_color.as_deref(), Some("#451a03"));
            assert_eq!(style.stroke_width, Some(2.0));
        }
    }

    #[test]
    fn state_rejects_unknown_style_classes() {
        let error = parse_state_diagram("stateDiagram-v2\nclass Ready missing\n")
            .expect_err("unknown state classes should fail");

        assert!(error.message.contains("unknown state style class"));
    }

    #[test]
    fn state_resolves_inline_class_shorthand() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\nclassDef quiet fill:#f8fafc,stroke:#64748b\nclassDef active fill:#dcfce7,color:#14532d\n[*]:::quiet --> Still:::quiet\nStill --> Moving:::active\nCrash:::active\n",
        )
        .expect("state inline class shorthand should parse");

        let still = diagram
            .nodes
            .iter()
            .find(|node| node.id == "Still")
            .unwrap();
        assert_eq!(
            still.style.as_ref().unwrap().fill.as_deref(),
            Some("#f8fafc")
        );
        for id in ["Moving", "Crash"] {
            let style = diagram
                .nodes
                .iter()
                .find(|node| node.id == id)
                .unwrap()
                .style
                .as_ref()
                .unwrap();
            assert_eq!(style.fill.as_deref(), Some("#dcfce7"));
            assert_eq!(style.text_color.as_deref(), Some("#14532d"));
        }
        assert!(diagram.nodes.iter().any(|node| {
            node.shape == Some(DiagramShape::Ellipse)
                && node
                    .style
                    .as_ref()
                    .and_then(|style| style.stroke.as_deref())
                    == Some("#64748b")
        }));
    }

    #[test]
    fn state_parses_attached_notes() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\nReady --> Running\nnote left of Ready: Waiting for work\nnote right of Running: Work is active\n",
        )
        .expect("attached state notes should parse");

        let notes: Vec<_> = diagram
            .nodes
            .iter()
            .filter(|node| node.shape == Some(DiagramShape::Note))
            .collect();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].label.text, "Waiting for work");
        assert_eq!(notes[1].label.text, "Work is active");
        let note_edges: Vec<_> = diagram
            .edges
            .iter()
            .filter(|edge| edge.kind == EdgeKind::NoteAssociation)
            .collect();
        assert_eq!(note_edges.len(), 2);
        assert_eq!(note_edges[0].to, "Ready");
        assert_eq!(note_edges[1].from, "Running");
    }

    #[test]
    fn state_parses_multiline_and_floating_notes() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\nReady --> Running\nnote right of Running\nFirst line\nSecond line\nend note\nnote \"Detached reminder\" as Reminder\n",
        )
        .expect("multiline and floating state notes should parse");

        let attached = diagram
            .nodes
            .iter()
            .find(|node| node.id.starts_with("__state_note_"))
            .unwrap();
        assert_eq!(attached.label.text, "First line\nSecond line");
        let floating = diagram
            .nodes
            .iter()
            .find(|node| node.id == "Reminder")
            .unwrap();
        assert_eq!(floating.shape, Some(DiagramShape::Note));
        assert_eq!(floating.label.text, "Detached reminder");
        assert_eq!(
            diagram
                .edges
                .iter()
                .filter(|edge| edge.kind == EdgeKind::NoteAssociation)
                .count(),
            1
        );
    }

    #[test]
    fn sequence_parses_case_insensitive_keywords() {
        let diagram = parse_any_mermaid(
            "SeQuEnCeDiAgRaM\nPaRtIcIpAnT A As Alice\nA->>B: Hello\nAcTiVaTe B\nNoTe RiGhT Of B: WRAP: Ready\nDeAcTiVaTe B\n",
        )
        .expect("mixed-case sequence syntax should parse");
        let MermaidDiagram::Sequence(diagram) = diagram else {
            panic!("expected sequence diagram");
        };

        assert_eq!(diagram.participants[0].label.text, "Alice");
        assert!(matches!(
            diagram.events[1],
            SequenceEvent::Activation { active: true, .. }
        ));
        assert!(matches!(
            diagram.events[2],
            SequenceEvent::Note {
                wrap: SequenceTextWrap::Wrap,
                ..
            }
        ));
        assert!(matches!(
            diagram.events[3],
            SequenceEvent::Activation { active: false, .. }
        ));
    }

    #[test]
    fn sequence_preprocesses_init_and_wrap_directives() {
        let diagram = parse_sequence_diagram(
            "%%{init: {'logLevel': 0}}%%\nsequenceDiagram\n%%{wrap}%%\nparticipant Alice as Primary client\nAlice->>Bob: A deliberately long request\nnote right of Bob: A deliberately long note\n",
        )
        .expect("preprocessor directives should not enter the sequence grammar");

        assert_eq!(diagram.participants[0].label_wrap, SequenceTextWrap::Wrap);
        assert!(matches!(
            diagram.events[0],
            SequenceEvent::Message {
                wrap: SequenceTextWrap::Wrap,
                ..
            }
        ));
        assert!(matches!(
            diagram.events[1],
            SequenceEvent::Note {
                wrap: SequenceTextWrap::Wrap,
                ..
            }
        ));
    }

    #[test]
    fn sequence_preprocesses_leading_yaml_front_matter() {
        let diagram = parse_sequence_diagram(
            "\n---\ntitle: Front matter title\nconfig:\n  theme: neutral\n---\n%%{wrap}%%\nsequenceDiagram\nAlice->>Bob: Request\n",
        )
        .expect("front matter should not enter the sequence grammar");

        assert_eq!(diagram.title, None);
        assert!(matches!(
            diagram.events[0],
            SequenceEvent::Message {
                wrap: SequenceTextWrap::Wrap,
                ..
            }
        ));
    }

    #[test]
    fn sequence_rejects_unterminated_yaml_front_matter() {
        let error = parse_sequence_diagram(
            "\n---\ntitle: Missing closing delimiter\nsequenceDiagram\nAlice->>Bob: Request\n",
        )
        .expect_err("unterminated front matter must not consume the diagram");

        assert!(error
            .message
            .contains("unterminated Mermaid YAML front matter"));
        assert_eq!(error.line, 2);
    }

    #[test]
    fn sequence_rejects_unterminated_directives() {
        let error = parse_sequence_diagram(
            "%%{init: {'logLevel': 0}\nsequenceDiagram\nAlice->>Bob: Request\n",
        )
        .expect_err("unterminated directives must not be silently discarded");
        assert!(error.message.contains("unterminated Mermaid directive"));
        assert_eq!(error.line, 1);
    }

    #[test]
    fn sequence_parses_nested_control_blocks_and_branches() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nalt Authorized\nAlice->>Bob: Submit\nloop Retry\nBob-->>Alice: Pending\nend\nelse Rejected\nBob-->>Alice: Denied\nend\n",
        )
        .unwrap();
        assert!(matches!(
            &diagram.events[0],
            SequenceEvent::BlockStart { kind: SequenceBlockKind::Alt, label, .. } if label == "Authorized"
        ));
        assert!(matches!(
            &diagram.events[2],
            SequenceEvent::BlockStart { kind: SequenceBlockKind::Loop, label, .. } if label == "Retry"
        ));
        assert!(matches!(
            &diagram.events[5],
            SequenceEvent::BlockBranch { label, .. } if label == "Rejected"
        ));
        assert!(matches!(
            diagram.events.last(),
            Some(SequenceEvent::BlockEnd {
                kind: SequenceBlockKind::Alt
            })
        ));
    }

    #[test]
    fn sequence_rejects_unterminated_control_block() {
        let error = parse_sequence_diagram("sequenceDiagram\nopt Available\nA->>B: Ping\n")
            .expect_err("unterminated opt must fail");
        assert!(!error.message.is_empty());
        assert!(error.line >= 2);
    }

    #[test]
    fn sequence_parses_participant_lifecycle_events() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nparticipant A as Alice\nA->>B: Start\ncreate actor Worker as Background Worker\nB->>Worker: Run\ndestroy Worker\nWorker-->>B: Stop\n",
        )
        .unwrap();
        let worker = diagram
            .participants
            .iter()
            .find(|p| p.id == "Worker")
            .unwrap();
        assert_eq!(worker.kind, SequenceParticipantKind::Actor);
        assert_eq!(worker.label.text, "Background Worker");
        assert!(diagram.events.iter().any(|event| matches!(
            event,
            SequenceEvent::ParticipantCreated { participant } if participant == "Worker"
        )));
        assert!(diagram.events.iter().any(|event| matches!(
            event,
            SequenceEvent::ParticipantDestroyed { participant } if participant == "Worker"
        )));
        let destroy_index = diagram
            .events
            .iter()
            .position(|event| matches!(event, SequenceEvent::ParticipantDestroyed { .. }))
            .unwrap();
        assert!(matches!(
            diagram.events[destroy_index - 1],
            SequenceEvent::Message { ref from, .. } if from == "Worker"
        ));
    }

    #[test]
    fn sequence_rejects_unassociated_lifecycle_declarations() {
        let create_error = parse_sequence_diagram(
            "sequenceDiagram\ncreate participant Worker\nA->>B: Wrong target\n",
        )
        .expect_err("created participant must receive the associated message");
        assert!(create_error.message.contains("must receive"));

        let destroy_error =
            parse_sequence_diagram("sequenceDiagram\ndestroy Worker\nA->>B: Wrong participants\n")
                .expect_err("destroyed participant must join the associated message");
        assert!(destroy_error.message.contains("must be part"));
    }

    #[test]
    fn sequence_rejects_duplicate_created_participants() {
        let error = parse_sequence_diagram(
            "sequenceDiagram\nparticipant Worker\ncreate actor Worker\nA->>Worker: Start\n",
        )
        .expect_err("create cannot reuse an existing participant ID");

        assert!(error.message.contains("duplicate sequence participant"));
    }

    #[test]
    fn sequence_rejects_activation_underflow() {
        let statement_error =
            parse_sequence_diagram("sequenceDiagram\nparticipant Worker\ndeactivate Worker\n")
                .expect_err("explicit deactivation requires an active participant");
        assert!(statement_error.message.contains("deactivate inactive"));

        let message_error =
            parse_sequence_diagram("sequenceDiagram\nA->>-B: Invalid sender deactivation\n")
                .expect_err("message deactivation requires an active sender");
        assert!(message_error.message.contains("deactivate inactive"));
    }

    #[test]
    fn sequence_parses_participant_boxes() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nbox hsl(270, 100%, 50%) Client tier\nactor User\nparticipant API as Banking API\nend\nbox Services\nparticipant DB\nend\n",
        )
        .unwrap();
        assert_eq!(diagram.participant_groups.len(), 2);
        assert_eq!(
            diagram.participant_groups[0].fill.as_deref(),
            Some("rgb(128, 0, 255)")
        );
        assert_eq!(
            diagram.participant_groups[0].label.as_deref(),
            Some("Client tier")
        );
        assert_eq!(diagram.participant_groups[1].fill, None);
        assert_eq!(
            diagram.participant_groups[1].label.as_deref(),
            Some("Services")
        );
        assert_eq!(diagram.participants[0].group_id.as_deref(), Some("box-1"));
        assert_eq!(diagram.participants[2].group_id.as_deref(), Some("box-2"));
    }

    #[test]
    fn sequence_rejects_participants_in_multiple_boxes() {
        let error = parse_sequence_diagram(
            "sequenceDiagram\nbox First\nparticipant API\nend\nbox Second\nparticipant API\nend\n",
        )
        .expect_err("a participant cannot move between boxes");

        assert!(error.message.contains("cannot belong to multiple boxes"));
    }

    #[test]
    fn sequence_preserves_participant_box_wrap_directives() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nbox hsl(180, 100%, 50%) wrap: A deliberately detailed client application tier\nparticipant API\nend\nbox nowrap: Core services\nparticipant DB\nend\n",
        )
        .unwrap();
        assert_eq!(
            diagram.participant_groups[0].label.as_deref(),
            Some("A deliberately detailed client application tier")
        );
        assert_eq!(
            diagram.participant_groups[0].label_wrap,
            SequenceTextWrap::Wrap
        );
        assert_eq!(
            diagram.participant_groups[1].label.as_deref(),
            Some("Core services")
        );
        assert_eq!(
            diagram.participant_groups[1].label_wrap,
            SequenceTextWrap::NoWrap
        );
    }

    #[test]
    fn sequence_rejects_messages_inside_participant_boxes() {
        let error = parse_sequence_diagram(
            "sequenceDiagram\nbox Services\nparticipant API\nAPI->>DB: Query\nend\n",
        )
        .expect_err("box bodies only allow participant declarations");
        assert!(!error.message.is_empty());
    }

    #[test]
    fn sequence_parses_participant_stereotypes_and_alias_precedence() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nparticipant API@{ \"type\": \"boundary\", \"alias\": \"Internal\" } as Public API\nparticipant C@{ type: control }\nparticipant E@{ type: entity }\nparticipant DB@{ type: 'database', alias: \"Ledger, \\\"primary\\\"\" }\nparticipant L@{ type: collections, alias: 'Collector''s lane' }\nparticipant Q@{ type: queue }\n",
        )
        .unwrap();
        assert_eq!(
            diagram.participants[0].kind,
            SequenceParticipantKind::Boundary
        );
        assert_eq!(diagram.participants[0].label.text, "Public API");
        assert_eq!(
            diagram.participants[1].kind,
            SequenceParticipantKind::Control
        );
        assert_eq!(
            diagram.participants[2].kind,
            SequenceParticipantKind::Entity
        );
        assert_eq!(
            diagram.participants[3].kind,
            SequenceParticipantKind::Database
        );
        assert_eq!(diagram.participants[3].label.text, "Ledger, \"primary\"");
        assert_eq!(
            diagram.participants[4].kind,
            SequenceParticipantKind::Collections
        );
        assert_eq!(diagram.participants[4].label.text, "Collector's lane");
        assert_eq!(diagram.participants[5].kind, SequenceParticipantKind::Queue);
    }

    #[test]
    fn sequence_parses_half_arrow_families() {
        let diagram = parse_sequence_diagram(
            r#"sequenceDiagram
A-|\B: filled top
B--|/A: dotted filled bottom
A-\\B: stick top
B//-A: reverse stick top
"#,
        )
        .unwrap();
        let arrows: Vec<_> = diagram
            .events
            .iter()
            .filter_map(|event| match event {
                SequenceEvent::Message {
                    arrowhead,
                    line_style,
                    ..
                } => Some((arrowhead, line_style)),
                _ => None,
            })
            .collect();
        assert_eq!(arrows[0].0, &SequenceArrowhead::FilledTop);
        assert_eq!(
            arrows[1],
            (&SequenceArrowhead::FilledBottom, &SequenceLineStyle::Dotted)
        );
        assert_eq!(arrows[2].0, &SequenceArrowhead::StickTop);
        assert_eq!(arrows[3].0, &SequenceArrowhead::ReverseStickTop);
    }

    #[test]
    fn sequence_parses_central_connection_endpoints() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nAlice->>()John: destination\nAlice()->>John: source\nJohn()->>()Alice: both\n",
        )
        .unwrap();
        let connections: Vec<_> = diagram
            .events
            .iter()
            .filter_map(|event| match event {
                SequenceEvent::Message {
                    central_connection, ..
                } => Some(central_connection),
                _ => None,
            })
            .collect();
        assert_eq!(
            connections,
            vec![
                &SequenceCentralConnection::Destination,
                &SequenceCentralConnection::Source,
                &SequenceCentralConnection::Both,
            ]
        );
    }

    #[test]
    fn sequence_central_connections_open_endpoint_activations() {
        parse_sequence_diagram(
            "sequenceDiagram\nAlice()->>Bob: source\ndeactivate Alice\nAlice->>()Bob: destination\ndeactivate Bob\nAlice()->>()Bob: both\ndeactivate Alice\ndeactivate Bob\n",
        )
        .expect("central endpoint activations should balance explicit deactivations");
    }

    #[test]
    fn sequence_rejects_activation_suffixes_on_central_connections() {
        for source in [
            "sequenceDiagram\nAlice()->>+Bob: invalid source suffix\n",
            "sequenceDiagram\nAlice->>()-Bob: invalid destination suffix\n",
            "sequenceDiagram\nAlice()->>()+Bob: invalid dual suffix\n",
        ] {
            parse_sequence_diagram(source)
                .expect_err("central connections have their own activation semantics");
        }
    }

    #[test]
    fn sequence_parses_autonumber_start_and_increment() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nautonumber 10.5 2.25\nAlice->>Bob: First\nBob->>Alice: Second\n",
        )
        .unwrap();
        assert!(diagram.auto_number);
        assert_eq!(diagram.auto_number_start, 10.5);
        assert_eq!(diagram.auto_number_step, 2.25);
        assert!(matches!(
            diagram.events.first(),
            Some(SequenceEvent::AutoNumber {
                visible: true,
                start: Some(10.5),
                step: Some(2.25),
            })
        ));
    }

    #[test]
    fn sequence_autonumber_start_defaults_increment_to_one() {
        let diagram =
            parse_sequence_diagram("sequenceDiagram\nautonumber 20\nAlice->>Bob: First\n").unwrap();

        assert!(matches!(
            diagram.events[0],
            SequenceEvent::AutoNumber {
                visible: true,
                start: Some(20.0),
                step: Some(1.0),
            }
        ));
    }

    #[test]
    fn sequence_rejects_autonumber_thousandths_and_unseparated_values() {
        let precision_error =
            parse_sequence_diagram("sequenceDiagram\nautonumber 10.001\nAlice->>Bob: First\n")
                .expect_err("thousandths must not split into start and step values");
        assert!(precision_error.message.contains("at most two"));

        let separation_error =
            parse_sequence_diagram("sequenceDiagram\nautonumber 10.1.01\nAlice->>Bob: First\n")
                .expect_err("start and step values require whitespace");
        assert!(separation_error.message.contains("whitespace"));
    }

    #[test]
    fn sequence_preserves_ordered_autonumber_toggles() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nautonumber\nA->>B: One\nautonumber off\nA->>B: Hidden\nautonumber 20 5\nA->>B: Twenty\n",
        )
        .unwrap();
        let controls: Vec<_> = diagram
            .events
            .iter()
            .filter(|event| matches!(event, SequenceEvent::AutoNumber { .. }))
            .collect();
        assert_eq!(controls.len(), 3);
        assert!(matches!(
            controls[1],
            SequenceEvent::AutoNumber { visible: false, .. }
        ));
        assert!(matches!(
            controls[2],
            SequenceEvent::AutoNumber {
                visible: true,
                start: Some(20.0),
                step: Some(5.0)
            }
        ));
    }

    #[test]
    fn sequence_parses_nested_rect_background_colors() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nrect rgba(0, 0, 255, .1)\nAlice->>Bob: Outer\nrect hsla(30, 100%, 50%, .25)\nBob->>Alice: Inner\nend\nend\n",
        )
        .unwrap();
        let fills: Vec<_> = diagram
            .events
            .iter()
            .filter_map(|event| match event {
                SequenceEvent::BlockStart {
                    kind: SequenceBlockKind::Rect,
                    fill,
                    ..
                } => fill.as_deref(),
                _ => None,
            })
            .collect();
        assert_eq!(
            fills,
            vec!["rgba(0, 0, 255, .1)", "rgba(255, 128, 0, 0.25)"]
        );
    }

    #[test]
    fn sequence_parses_default_and_named_rect_backgrounds() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nrect\nAlice->>Bob: Default\nend\nrect green\nBob->>Alice: Named\nend\n",
        )
        .unwrap();
        let fills: Vec<_> = diagram
            .events
            .iter()
            .filter_map(|event| match event {
                SequenceEvent::BlockStart {
                    kind: SequenceBlockKind::Rect,
                    fill,
                    ..
                } => Some(fill.as_deref()),
                _ => None,
            })
            .collect();

        assert_eq!(fills, vec![None, Some("green")]);
    }

    #[test]
    fn sequence_parses_simple_and_json_actor_links() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nparticipant Alice\nlink Alice: Health Dashboard @ https://example.com/health\nlinks Alice: {\"Wiki\": \"https://example.com/wiki\", \"Repo\": \"https://example.com/repo\"}\n",
        )
        .unwrap();
        let alice = &diagram.participants[0];
        assert_eq!(alice.links.len(), 3);
        assert!(alice.links.iter().any(
            |link| link.label == "Health Dashboard" && link.url == "https://example.com/health"
        ));
        assert!(alice.links.iter().any(|link| link.label == "Wiki"));
        assert!(alice.links.iter().any(|link| link.label == "Repo"));
    }

    #[test]
    fn sequence_parses_and_merges_actor_properties() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nparticipant Alice\nproperties Alice: {\"role\": \"admin\", \"active\": true, \"limits\": {\"daily\": 5}}\nproperties Alice: {\"role\": \"owner\"}\n",
        )
        .unwrap();
        let properties = &diagram.participants[0].properties;
        assert_eq!(properties.len(), 3);
        assert!(properties
            .iter()
            .any(|property| property.name == "role" && property.value_json == "\"owner\""));
        assert!(properties
            .iter()
            .any(|property| property.name == "active" && property.value_json == "true"));
        assert!(properties.iter().any(|property| {
            property.name == "limits" && property.value_json == "{\"daily\":5}"
        }));
    }

    #[test]
    fn sequence_parses_actor_details_reference() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nparticipant Alice\ndetails Alice: alice-info\n",
        )
        .unwrap();
        assert_eq!(
            diagram.participants[0].details_reference.as_deref(),
            Some("alice-info")
        );
    }

    #[test]
    fn sequence_parses_accessibility_title_and_description() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\naccTitle: Transfer flow\naccDescr: Banking interaction\nAlice->>Bob: Hello\n",
        )
        .unwrap();
        assert_eq!(
            diagram.accessibility_title.as_deref(),
            Some("Transfer flow")
        );
        assert_eq!(
            diagram.accessibility_description.as_deref(),
            Some("Banking interaction")
        );
    }

    #[test]
    fn sequence_parses_legacy_colon_title() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\ntitle: Native transfer sequence\nAlice->>Bob: Hello\n",
        )
        .unwrap();
        assert_eq!(diagram.title.as_deref(), Some("Native transfer sequence"));
    }

    #[test]
    fn sequence_decodes_numeric_and_named_entities_in_text() {
        let diagram =
            parse_sequence_diagram("sequenceDiagram\nAlice->>Bob: I #9829; you #infin; times\n")
                .unwrap();
        assert!(matches!(
            diagram.events.first(),
            Some(SequenceEvent::Message { label, .. }) if label == "I ♥ you ∞ times"
        ));
    }

    #[test]
    fn sequence_ignores_hash_comments_around_semantic_text() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\n# participant setup\nparticipant A # declaration comment\nA->>B: Hello # message comment\n",
        )
        .unwrap();
        assert_eq!(diagram.participants.len(), 2);
        assert!(matches!(
            &diagram.events[0],
            SequenceEvent::Message { label, .. } if label == "Hello"
        ));
    }

    #[test]
    fn sequence_preserves_punctuation_and_keywords_in_semantic_text() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nAlice->Bob: -:<>, end + @value\nnote right of Bob: -:<>, end\nloop -:<>, end\nBob-->Alice: retry->now\nend\n",
        )
        .unwrap();
        let labels: Vec<_> = diagram
            .events
            .iter()
            .filter_map(|event| match event {
                SequenceEvent::Message { label, .. }
                | SequenceEvent::Note { text: label, .. }
                | SequenceEvent::BlockStart { label, .. } => Some(label.as_str()),
                _ => None,
            })
            .collect();

        assert_eq!(
            labels,
            vec!["-:<>, end + @value", "-:<>, end", "-:<>, end", "retry->now"]
        );
    }

    #[test]
    fn sequence_converts_html_breaks_to_semantic_newlines() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nAlice->>Bob: First line<br/>Second line\nnote over Alice,Bob: Note one<br />Note two\n",
        )
        .unwrap();
        assert!(matches!(
            &diagram.events[0],
            SequenceEvent::Message { label, .. } if label == "First line\nSecond line"
        ));
        assert!(matches!(
            &diagram.events[1],
            SequenceEvent::Note { text, .. } if text == "Note one\nNote two"
        ));
    }

    #[test]
    fn sequence_preserves_message_and_note_wrap_directives() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nAlice->>Bob: wrap: A deliberately long message\nnote over Alice,Bob: nowrap: A deliberately long note\n",
        )
        .unwrap();
        assert!(matches!(
            &diagram.events[0],
            SequenceEvent::Message { label, wrap: SequenceTextWrap::Wrap, .. }
                if label == "A deliberately long message"
        ));
        assert!(matches!(
            &diagram.events[1],
            SequenceEvent::Note { text, wrap: SequenceTextWrap::NoWrap, .. }
                if text == "A deliberately long note"
        ));
    }

    #[test]
    fn sequence_preserves_participant_alias_wrap_directives() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nparticipant API as wrap: A deliberately detailed public application programming interface\nactor User as nowrap: Banking User\n",
        )
        .unwrap();
        assert_eq!(
            diagram.participants[0].label.text,
            "A deliberately detailed public application programming interface"
        );
        assert_eq!(diagram.participants[0].label_wrap, SequenceTextWrap::Wrap);
        assert_eq!(diagram.participants[1].label.text, "Banking User");
        assert_eq!(diagram.participants[1].label_wrap, SequenceTextWrap::NoWrap);
    }

    #[test]
    fn sequence_preserves_control_block_wrap_directives() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nalt wrap: A deliberately detailed acceptance path\nA->>B: Yes\nelse nowrap: A deliberately detailed rejection path\nB-->>A: No\nend\n",
        )
        .unwrap();
        assert!(matches!(
            &diagram.events[0],
            SequenceEvent::BlockStart {
                label,
                wrap: SequenceTextWrap::Wrap,
                ..
            } if label == "A deliberately detailed acceptance path"
        ));
        assert!(matches!(
            &diagram.events[2],
            SequenceEvent::BlockBranch {
                label,
                wrap: SequenceTextWrap::NoWrap,
            } if label == "A deliberately detailed rejection path"
        ));
    }

    #[test]
    fn sequence_preserves_multiword_actor_identifiers() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nparticipant Customer Portal as Customer\nparticipant Order Service\nCustomer Portal->>Order Service: Submit\nnote over Customer Portal,Order Service: Accepted\nactivate Order Service\ndeactivate Order Service\n",
        )
        .unwrap();
        assert_eq!(diagram.participants[0].id, "Customer Portal");
        assert_eq!(diagram.participants[0].label.text, "Customer");
        assert_eq!(diagram.participants[1].id, "Order Service");
        assert!(matches!(
            &diagram.events[0],
            SequenceEvent::Message { from, to, .. }
                if from == "Customer Portal" && to == "Order Service"
        ));
        assert!(matches!(
            &diagram.events[1],
            SequenceEvent::Note { participants, .. }
                if participants == &["Customer Portal", "Order Service"]
        ));
    }

    #[test]
    fn sequence_preserves_hyphenated_actor_identifiers() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\nparticipant Customer-Portal as Customer\nparticipant Order-Service\nCustomer-Portal->>+Order-Service: Submit\nnote over Customer-Portal,Order-Service: Accepted\nOrder-Service-->>-Customer-Portal: Done\nactivate Order-Service\ndeactivate Order-Service\n",
        )
        .unwrap();
        assert_eq!(diagram.participants[0].id, "Customer-Portal");
        assert_eq!(diagram.participants[1].id, "Order-Service");
        assert!(matches!(
            &diagram.events[0],
            SequenceEvent::Message { from, to, .. }
                if from == "Customer-Portal" && to == "Order-Service"
        ));
        assert!(matches!(
            &diagram.events[1],
            SequenceEvent::Note { participants, .. }
                if participants == &["Customer-Portal", "Order-Service"]
        ));
        assert!(matches!(
            &diagram.events[2],
            SequenceEvent::Message { from, to, deactivate: true, .. }
                if from == "Order-Service" && to == "Customer-Portal"
        ));
    }

    #[test]
    fn sequence_parses_multiline_accessibility_description() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram\naccDescr {\n  Transfers funds\n  between accounts\n}\nAlice->>Bob: Hello\n",
        )
        .unwrap();
        assert_eq!(
            diagram.accessibility_description.as_deref(),
            Some("Transfers funds\n  between accounts")
        );
    }

    #[test]
    fn sequence_parses_semicolon_terminated_statements_and_blocks() {
        let diagram = parse_sequence_diagram(
            "sequenceDiagram;participant Alice;participant Bob;loop Retry;Alice->>Bob: Ping;end;",
        )
        .unwrap();
        assert_eq!(diagram.participants.len(), 2);
        assert!(matches!(
            diagram.events.as_slice(),
            [
                SequenceEvent::BlockStart { .. },
                SequenceEvent::Message { label, .. },
                SequenceEvent::BlockEnd { .. }
            ] if label == "Ping"
        ));
    }
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
        assert_eq!(crate::VERSION, "0.53.0");
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

    // -------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening) -- see MAX_RULE_DEPTH's own
    // doc comment: this grammar has no self-referential production at
    // all, so there is no adversarial *nesting* shape to probe. The only
    // "chain" shape (`edge_stmt`'s own `edge_segment { edge_segment }`
    // repetition) is iterative, not recursive -- this test confirms a
    // very long edge chain still parses cleanly even with the cap
    // applied, i.e. that DEFAULT_MAX_RULE_DEPTH does not falsely reject
    // wide (as opposed to deep) input.
    // -------------------------------------------------------------------

    #[test]
    fn a_very_long_edge_chain_still_parses_cleanly() {
        let mut src = String::from("flowchart LR\nA");
        for _ in 0..5000 {
            src.push_str(" --> A");
        }
        src.push('\n');
        let diagram = parse_to_diagram(&src).expect("wide edge chain must not trip the cap");
        assert_eq!(diagram.edges.len(), 5000);
    }
}
