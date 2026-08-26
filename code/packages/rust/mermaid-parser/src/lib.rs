//! Grammar-driven parser and compatibility dispatcher for Mermaid diagrams.

// This file hand-parses many `starts_with(...)` / slice-index prefix strips
// where the prefix and the stripped remainder need slightly different handling;
// rewriting every one as `strip_prefix` hurts readability here, so we opt out
// of the lint file-wide.
#![allow(clippy::manual_strip)]

pub const VERSION: &str = "0.126.0";
pub const MERMAID_COMPATIBILITY_BASELINE: &str = "11.16.1";

use std::collections::{HashMap, HashSet};

use diagram_ir::{
    DiagramDirection, DiagramLabel, DiagramShape, DiagramStyle, EdgeKind, GraphDiagram, GraphEdge,
    GraphGroup, GraphLink, GraphNode,
};
use grammar_tools::parser_grammar::parse_parser_grammar;
use lexer::token::{Token, TokenType};
use mermaid_lexer::{
    tokenize_mermaid, tokenize_mermaid_c4, tokenize_mermaid_er, tokenize_mermaid_gitgraph,
    tokenize_mermaid_pie, tokenize_mermaid_sankey, tokenize_mermaid_sequence,
    tokenize_mermaid_state, try_tokenize_mermaid_gantt, try_tokenize_mermaid_journey,
    try_tokenize_mermaid_quadrant, try_tokenize_mermaid_requirement, try_tokenize_mermaid_xychart,
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
const QUADRANT_PARSER_GRAMMAR_SOURCE: &str =
    include_str!("../../../../grammars/mermaid/quadrant.grammar");
const JOURNEY_PARSER_GRAMMAR_SOURCE: &str =
    include_str!("../../../../grammars/mermaid/journey.grammar");
const REQUIREMENT_PARSER_GRAMMAR_SOURCE: &str =
    include_str!("../../../../grammars/mermaid/requirement.grammar");
const XYCHART_PARSER_GRAMMAR_SOURCE: &str =
    include_str!("../../../../grammars/mermaid/xychart.grammar");
const GANTT_PARSER_GRAMMAR_SOURCE: &str = include_str!("../../../../grammars/mermaid/gantt.grammar");

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

pub fn parse_mermaid_quadrant_ast(source: &str) -> Result<GrammarASTNode, ParseError> {
    let tokens = try_tokenize_mermaid_quadrant(source).map_err(|message| ParseError {
        message,
        line: 1,
        col: 1,
    })?;
    let grammar = parse_parser_grammar(QUADRANT_PARSER_GRAMMAR_SOURCE)
        .unwrap_or_else(|e| panic!("Failed to parse quadrant.grammar: {e}"));
    let mut parser = GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH);
    parser.parse().map_err(|e| ParseError {
        message: e.message,
        line: e.token.line,
        col: e.token.column,
    })
}

pub fn parse_mermaid_xychart_ast(source: &str) -> Result<GrammarASTNode, ParseError> {
    let tokens = try_tokenize_mermaid_xychart(source).map_err(|message| ParseError {
        message,
        line: 1,
        col: 1,
    })?;
    let grammar = parse_parser_grammar(XYCHART_PARSER_GRAMMAR_SOURCE)
        .unwrap_or_else(|e| panic!("Failed to parse xychart.grammar: {e}"));
    let mut parser = GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH);
    parser.parse().map_err(|e| ParseError {
        message: e.message,
        line: e.token.line,
        col: e.token.column,
    })
}

pub fn parse_mermaid_gantt_ast(source: &str) -> Result<GrammarASTNode, ParseError> {
    let preprocessed = preprocess_mermaid_source(source)?;
    let tokens = try_tokenize_mermaid_gantt(&preprocessed.source).map_err(|message| ParseError {
        message,
        line: 1,
        col: 1,
    })?;
    let grammar = parse_parser_grammar(GANTT_PARSER_GRAMMAR_SOURCE)
        .unwrap_or_else(|error| panic!("Failed to parse gantt.grammar: {error}"));
    let mut parser = GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH);
    parser.parse().map_err(|error| ParseError {
        message: error.message,
        line: error.token.line,
        col: error.token.column,
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
        requested_width: None,
        hide_empty_descriptions: false,
        title: None,
        accessibility_title: None,
        accessibility_description: None,
        links: Vec::new(),
        groups: Vec::new(),
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
    Axis, AxisKind, ChartDataPoint, ChartDiagram, ChartKind, ChartOrientation, ChartSeries,
    Compartment, CompartmentKind, GanttConfig, GanttDateFormat, GanttDateFormatPart, GanttDiagram, GanttDuration, GanttDurationUnit, GanttSection, GanttTask, GitBranch, GitCommitType,
    GitDiagram, GitEvent, JourneyConfig, JourneyDiagram, JourneySection, JourneyTask, PieSlice,
    QuadrantConfig, QuadrantPoint, RelKind, RequirementElementMetadata, RequirementKind,
    RequirementMetadata, RequirementRisk, RequirementVerifyMethod, SankeyFlow, SankeyNode,
    SequenceArrowhead, SequenceBlockKind, SequenceCentralConnection, SequenceDiagram,
    SequenceEvent, SequenceLineStyle, SequenceLink, SequenceNotePlacement, SequenceParticipant,
    SequenceParticipantGroup, SequenceParticipantKind, SequenceProperty, SequenceTextWrap,
    SeriesKind, StructuralDiagram, StructuralGroup, StructuralKind, StructuralNode,
    GanttTaskTags, StructuralNodeKind, StructuralNodeMetadata, StructuralRelationship, TaskEnd,
    TaskStart,
    TemporalBody, TemporalDiagram, TemporalKind, XyAxisConfig, XyChartConfig,
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
                | Self::Journey
                | Self::Requirement
                | Self::Pie
                | Self::Quadrant
                | Self::Sequence
                | Self::State
                | Self::Sankey
                | Self::XyChart
        )
    }
}

/// Union of all Mermaid diagram variants that `parse_any_mermaid` can return.
#[allow(clippy::large_enum_variant)]
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
    if first.eq_ignore_ascii_case("quadrantChart") {
        return Ok(MermaidDiagramType::Quadrant);
    }
    if first.eq_ignore_ascii_case("journey") {
        return Ok(MermaidDiagramType::Journey);
    }
    if first.eq_ignore_ascii_case("requirement") || first.eq_ignore_ascii_case("requirementDiagram")
    {
        return Ok(MermaidDiagramType::Requirement);
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
        MermaidDiagramType::Requirement => {
            parse_requirement_diagram(source).map(MermaidDiagram::Structural)
        }
        MermaidDiagramType::XyChart => parse_xychart(source).map(MermaidDiagram::Chart),
        MermaidDiagramType::Pie => parse_pie(source).map(MermaidDiagram::Chart),
        MermaidDiagramType::Quadrant => parse_quadrant_chart(source).map(MermaidDiagram::Chart),
        MermaidDiagramType::Sequence => {
            parse_sequence_diagram(source).map(MermaidDiagram::Sequence)
        }
        MermaidDiagramType::State => parse_state_diagram(source).map(MermaidDiagram::Graph),
        MermaidDiagramType::Sankey => parse_sankey(source).map(MermaidDiagram::Chart),
        MermaidDiagramType::GitGraph => parse_gitgraph(source).map(|git| {
            let title = git.title.clone();
            MermaidDiagram::Temporal(TemporalDiagram {
                kind: TemporalKind::Git,
                title,
                body: TemporalBody::Git(git),
            })
        }),
        MermaidDiagramType::Gantt => parse_gantt(source).map(|g| {
            let title = g.title.clone();
            MermaidDiagram::Temporal(TemporalDiagram {
                kind: TemporalKind::Gantt,
                title,
                body: TemporalBody::Gantt(g),
            })
        }),
        MermaidDiagramType::Journey => parse_journey(source).map(|(title, journey)| {
            MermaidDiagram::Temporal(TemporalDiagram {
                kind: TemporalKind::Journey,
                title,
                body: TemporalBody::Journey(Box::new(journey)),
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

/// Parse Mermaid requirements and elements into the shared structural IR.
pub fn parse_requirement_diagram(source: &str) -> Result<StructuralDiagram, ParseError> {
    let tokens = try_tokenize_mermaid_requirement(source).map_err(|message| ParseError {
        message,
        line: 1,
        col: 1,
    })?;
    let grammar = parse_parser_grammar(REQUIREMENT_PARSER_GRAMMAR_SOURCE)
        .unwrap_or_else(|error| panic!("Failed to parse requirement.grammar: {error}"));
    let mut parser = GrammarParser::new(tokens.clone(), grammar).with_max_depth(MAX_RULE_DEPTH);
    parser.parse().map_err(|error| ParseError {
        message: error.message,
        line: error.token.line,
        col: error.token.column,
    })?;

    let mut title = None;
    let mut accessibility_title = None;
    let mut accessibility_description = None;
    let mut direction = None;
    let mut nodes = Vec::new();
    let mut relationships = Vec::new();
    let mut style_events = Vec::new();
    let mut cursor = TokenCursor::new(tokens);
    cursor.skip_terminators();
    cursor.consume_if("HEADER");
    cursor.skip_terminators();
    while !cursor.at_eof() {
        match token_name(cursor.current()) {
            "TITLE" => {
                let value = cursor.advance().value.clone();
                title = value
                    .split_once(char::is_whitespace)
                    .map(|(_, title)| title.trim().to_string());
            }
            "ACC_TITLE" => {
                accessibility_title = cursor
                    .advance()
                    .value
                    .split_once(':')
                    .map(|(_, value)| value.trim().to_string());
            }
            "ACC_DESCR" => {
                accessibility_description = cursor
                    .advance()
                    .value
                    .split_once(':')
                    .map(|(_, value)| value.trim().to_string());
            }
            "ACC_DESCR_BLOCK" => {
                let value = cursor.advance().value.clone();
                let open = value.find('{').expect("accessibility block requires '{'");
                let close = value.rfind('}').expect("accessibility block requires '}'");
                accessibility_description = Some(
                    value[open + 1..close]
                        .lines()
                        .map(str::trim)
                        .filter(|line| !line.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
            }
            "DIRECTION" => {
                let value = cursor.advance().value.clone();
                direction = Some(
                    match value
                        .split_whitespace()
                        .nth(1)
                        .map(|value| value.to_ascii_uppercase())
                        .as_deref()
                    {
                        Some("TB") => DiagramDirection::Tb,
                        Some("BT") => DiagramDirection::Bt,
                        Some("LR") => DiagramDirection::Lr,
                        Some("RL") => DiagramDirection::Rl,
                        _ => {
                            return Err(token_error(
                                cursor.current(),
                                "invalid requirement direction",
                            ))
                        }
                    },
                );
            }
            "STYLE" => {
                let (node_ids, style) = parse_requirement_style(cursor.advance())?;
                style_events.push(RequirementStyleEvent::Direct { node_ids, style });
            }
            "CLASSDEF" => {
                let (class_names, style) = parse_requirement_class_def(cursor.advance())?;
                style_events.push(RequirementStyleEvent::DefineClass { class_names, style });
            }
            "CLASS" => {
                let (node_ids, class_names) = parse_requirement_class_assignment(cursor.advance())?;
                style_events.push(RequirementStyleEvent::AssignClass {
                    node_ids,
                    class_names,
                });
            }
            "INLINE_CLASS" => {
                let (node_id, class_names) = parse_requirement_inline_class(cursor.advance())?;
                style_events.push(RequirementStyleEvent::AssignClass {
                    node_ids: vec![node_id],
                    class_names,
                });
            }
            "REQUIREMENT_START" | "ELEMENT_START" => {
                let is_element = token_name(cursor.current()) == "ELEMENT_START";
                let declaration = cursor
                    .advance()
                    .value
                    .trim_end_matches('{')
                    .trim()
                    .to_string();
                let (kind, node_ref) =
                    declaration.split_once(char::is_whitespace).ok_or_else(|| {
                        token_error(cursor.current(), "invalid requirement definition")
                    })?;
                let (name, inline_classes) = parse_requirement_node_ref(node_ref);
                let mut fields = Vec::new();
                let mut requirement_metadata = RequirementMetadata::default();
                if !is_element {
                    requirement_metadata.kind = parse_requirement_kind(kind);
                }
                let mut element_metadata = RequirementElementMetadata::default();
                cursor.skip_terminators();
                while token_name(cursor.current()) != "RBRACE" {
                    if cursor.at_eof() {
                        return Err(token_error(
                            cursor.current(),
                            "unterminated requirement definition",
                        ));
                    }
                    if matches!(
                        token_name(cursor.current()),
                        "ID_FIELD"
                            | "TEXT_FIELD"
                            | "RISK_FIELD"
                            | "VERIFY_FIELD"
                            | "TYPE_FIELD"
                            | "DOCREF_FIELD"
                    ) {
                        let value = cursor.advance().value.clone();
                        if let Some((key, value)) = value.split_once(':') {
                            let field_value = unquote_requirement_value(value);
                            match key.trim().to_ascii_lowercase().as_str() {
                                "id" => {
                                    requirement_metadata.external_id = Some(field_value.clone())
                                }
                                "text" => requirement_metadata.text = Some(field_value.clone()),
                                "risk" => {
                                    requirement_metadata.risk =
                                        Some(parse_requirement_risk(&field_value));
                                }
                                "verifymethod" => {
                                    requirement_metadata.verify_method =
                                        Some(parse_requirement_verify_method(&field_value));
                                }
                                "type" => element_metadata.element_type = Some(field_value.clone()),
                                "docref" => {
                                    element_metadata.document_reference = Some(field_value.clone());
                                }
                                _ => unreachable!("requirement grammar emitted an unknown field"),
                            }
                            fields.push(format!("{}: {}", key.trim(), field_value));
                        }
                    } else {
                        cursor.advance();
                    }
                    cursor.skip_terminators();
                }
                cursor.advance();
                nodes.push(StructuralNode {
                    id: name.clone(),
                    label: name.clone(),
                    stereotype: Some(kind.to_string()),
                    node_kind: if is_element {
                        StructuralNodeKind::Element
                    } else {
                        StructuralNodeKind::Requirement
                    },
                    metadata: Some(if is_element {
                        StructuralNodeMetadata::RequirementElement(element_metadata)
                    } else {
                        StructuralNodeMetadata::Requirement(requirement_metadata)
                    }),
                    style: None,
                    compartments: vec![Compartment {
                        kind: CompartmentKind::Values,
                        entries: fields,
                    }],
                    parent_group: None,
                });
                if !inline_classes.is_empty() {
                    style_events.push(RequirementStyleEvent::AssignClass {
                        node_ids: vec![name],
                        class_names: inline_classes,
                    });
                }
            }
            "RELATIONSHIP" => {
                let token = cursor.advance().clone();
                relationships.push(parse_requirement_relationship(&token)?);
            }
            _ => {
                cursor.advance();
            }
        }
        cursor.skip_terminators();
    }
    resolve_requirement_styles(&mut nodes, style_events, cursor.current())?;
    Ok(StructuralDiagram {
        kind: StructuralKind::Requirement,
        title,
        accessibility_title,
        accessibility_description,
        direction,
        nodes,
        groups: Vec::new(),
        relationships,
    })
}

fn unquote_requirement_value(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}

fn parse_requirement_risk(value: &str) -> RequirementRisk {
    match value.trim().to_ascii_lowercase().as_str() {
        "low" => RequirementRisk::Low,
        "medium" => RequirementRisk::Medium,
        "high" => RequirementRisk::High,
        _ => unreachable!("requirement grammar accepted an unknown risk"),
    }
}

fn parse_requirement_style(token: &Token) -> Result<(Vec<String>, DiagramStyle), ParseError> {
    parse_requirement_target_style(token, "style")
}

fn parse_requirement_class_def(token: &Token) -> Result<(Vec<String>, DiagramStyle), ParseError> {
    parse_requirement_target_style(token, "classDef")
}

fn parse_requirement_target_style(
    token: &Token,
    keyword: &str,
) -> Result<(Vec<String>, DiagramStyle), ParseError> {
    let value = token
        .value
        .trim_end_matches(';')
        .get(keyword.len()..)
        .expect("requirement grammar emitted the expected style keyword")
        .trim();
    let (targets, declarations) = split_requirement_head(value)
        .ok_or_else(|| token_error(token, "invalid requirement style"))?;
    let mut style = DiagramStyle::default();
    for declaration in split_requirement_segments(declarations, ',') {
        let (property, value) = declaration
            .split_once(':')
            .ok_or_else(|| token_error(token, "invalid requirement style declaration"))?;
        let value = value.trim();
        match property.trim().to_ascii_lowercase().as_str() {
            "fill" => style.fill = Some(value.to_string()),
            "stroke" => style.stroke = Some(value.to_string()),
            "color" => style.text_color = Some(value.to_string()),
            "stroke-width" => {
                let width = value
                    .strip_suffix("px")
                    .unwrap_or(value)
                    .parse::<f64>()
                    .map_err(|_| token_error(token, "invalid requirement stroke width"))?;
                if width <= 0.0 {
                    return Err(token_error(
                        token,
                        "requirement stroke width must be positive",
                    ));
                }
                style.stroke_width = Some(width);
            }
            "font-size" => {
                let size = value
                    .strip_suffix("px")
                    .unwrap_or(value)
                    .parse::<f64>()
                    .map_err(|_| token_error(token, "invalid requirement font size"))?;
                if size <= 0.0 {
                    return Err(token_error(token, "requirement font size must be positive"));
                }
                style.font_size = Some(size);
            }
            "font-weight" => {
                let weight = match value.to_ascii_lowercase().as_str() {
                    "normal" => 400,
                    "bold" => 700,
                    numeric => numeric
                        .parse::<u16>()
                        .map_err(|_| token_error(token, "invalid requirement font weight"))?,
                };
                if !(100..=900).contains(&weight) || weight % 100 != 0 {
                    return Err(token_error(
                        token,
                        "requirement font weight must be normal, bold, or 100 through 900",
                    ));
                }
                style.font_weight = Some(weight);
            }
            "font-style" => {
                style.font_italic = Some(match value.to_ascii_lowercase().as_str() {
                    "normal" => false,
                    "italic" => true,
                    _ => {
                        return Err(token_error(
                            token,
                            "requirement font style must be normal or italic",
                        ))
                    }
                });
            }
            "font-family" => {
                let family = value.trim_matches('"').trim();
                if family.is_empty() {
                    return Err(token_error(
                        token,
                        "requirement font family cannot be empty",
                    ));
                }
                style.font_family = Some(family.to_string());
            }
            property => {
                return Err(token_error(
                    token,
                    format!("unsupported requirement style property {property:?}"),
                ));
            }
        }
    }
    Ok((split_requirement_list(targets), style))
}

fn parse_requirement_class_assignment(
    token: &Token,
) -> Result<(Vec<String>, Vec<String>), ParseError> {
    let value = token
        .value
        .trim_end_matches(';')
        .get("class".len()..)
        .expect("requirement grammar emitted the class keyword")
        .trim();
    let (node_ids, class_names) = split_requirement_head(value)
        .ok_or_else(|| token_error(token, "invalid requirement class assignment"))?;
    Ok((
        split_requirement_list(node_ids),
        split_requirement_list(class_names),
    ))
}

fn parse_requirement_node_ref(value: &str) -> (String, Vec<String>) {
    let (name, classes) = value
        .split_once(":::")
        .map(|(name, classes)| (name, split_requirement_list(classes)))
        .unwrap_or((value, Vec::new()));
    (unquote_requirement_value(name), classes)
}

fn parse_requirement_inline_class(token: &Token) -> Result<(String, Vec<String>), ParseError> {
    let value = token.value.trim_end_matches(';');
    let (node_id, class_names) = value
        .split_once(":::")
        .ok_or_else(|| token_error(token, "invalid requirement inline class assignment"))?;
    Ok((
        unquote_requirement_value(node_id),
        split_requirement_list(class_names),
    ))
}

fn split_requirement_list(value: &str) -> Vec<String> {
    split_requirement_segments(value, ',')
        .into_iter()
        .map(unquote_requirement_value)
        .filter(|value| !value.is_empty())
        .collect()
}

fn split_requirement_head(value: &str) -> Option<(&str, &str)> {
    let mut quoted = false;
    for (index, character) in value.char_indices() {
        match character {
            '"' => quoted = !quoted,
            character if character.is_whitespace() && !quoted => {
                let tail = value[index..].trim_start();
                if !tail.is_empty() {
                    return Some((&value[..index], tail));
                }
            }
            _ => {}
        }
    }
    None
}

fn split_requirement_segments(value: &str, separator: char) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    for (index, character) in value.char_indices() {
        match character {
            '"' => quoted = !quoted,
            character if character == separator && !quoted => {
                segments.push(value[start..index].trim());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    segments.push(value[start..].trim());
    segments
}

enum RequirementStyleEvent {
    DefineClass {
        class_names: Vec<String>,
        style: DiagramStyle,
    },
    AssignClass {
        node_ids: Vec<String>,
        class_names: Vec<String>,
    },
    Direct {
        node_ids: Vec<String>,
        style: DiagramStyle,
    },
}

fn resolve_requirement_styles(
    nodes: &mut [StructuralNode],
    events: Vec<RequirementStyleEvent>,
    error_token: &Token,
) -> Result<(), ParseError> {
    let mut classes = HashMap::<String, DiagramStyle>::new();
    let mut memberships = nodes
        .iter()
        .map(|node| (node.id.clone(), vec!["default".to_string()]))
        .collect::<HashMap<_, _>>();

    for event in events {
        match event {
            RequirementStyleEvent::DefineClass { class_names, style } => {
                for class_name in class_names {
                    merge_requirement_style(classes.entry(class_name.clone()).or_default(), &style);
                    for node in nodes
                        .iter_mut()
                        .filter(|node| memberships[&node.id].iter().any(|name| name == &class_name))
                    {
                        merge_requirement_style(node.style.get_or_insert_default(), &style);
                    }
                }
            }
            RequirementStyleEvent::AssignClass {
                node_ids,
                class_names,
            } => {
                for node_id in node_ids {
                    let node = requirement_style_node(nodes, &node_id, error_token)?;
                    let node_memberships = memberships
                        .get_mut(&node_id)
                        .expect("styled requirement node has memberships");
                    for class_name in &class_names {
                        node_memberships.push(class_name.clone());
                        if let Some(style) = classes.get(class_name) {
                            merge_requirement_style(node.style.get_or_insert_default(), style);
                        }
                    }
                }
            }
            RequirementStyleEvent::Direct { node_ids, style } => {
                for node_id in node_ids {
                    let node = requirement_style_node(nodes, &node_id, error_token)?;
                    merge_requirement_style(node.style.get_or_insert_default(), &style);
                }
            }
        }
    }
    Ok(())
}

fn requirement_style_node<'a>(
    nodes: &'a mut [StructuralNode],
    node_id: &str,
    token: &Token,
) -> Result<&'a mut StructuralNode, ParseError> {
    nodes
        .iter_mut()
        .find(|node| node.id == node_id)
        .ok_or_else(|| token_error(token, format!("unknown styled node {node_id:?}")))
}

fn merge_requirement_style(target: &mut DiagramStyle, source: &DiagramStyle) {
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
    if source.font_size.is_some() {
        target.font_size = source.font_size;
    }
    if source.font_weight.is_some() {
        target.font_weight = source.font_weight;
    }
    if source.font_italic.is_some() {
        target.font_italic = source.font_italic;
    }
    if source.font_family.is_some() {
        target.font_family.clone_from(&source.font_family);
    }
}

fn parse_requirement_kind(value: &str) -> RequirementKind {
    match value.to_ascii_lowercase().as_str() {
        "requirement" => RequirementKind::Requirement,
        "functionalrequirement" => RequirementKind::Functional,
        "interfacerequirement" => RequirementKind::Interface,
        "performancerequirement" => RequirementKind::Performance,
        "physicalrequirement" => RequirementKind::Physical,
        "designconstraint" => RequirementKind::DesignConstraint,
        _ => unreachable!("requirement grammar accepted an unknown definition kind"),
    }
}

fn parse_requirement_verify_method(value: &str) -> RequirementVerifyMethod {
    match value.trim().to_ascii_lowercase().as_str() {
        "analysis" => RequirementVerifyMethod::Analysis,
        "inspection" => RequirementVerifyMethod::Inspection,
        "test" => RequirementVerifyMethod::Test,
        "demonstration" => RequirementVerifyMethod::Demonstration,
        _ => unreachable!("requirement grammar accepted an unknown verification method"),
    }
}

fn parse_requirement_relationship(token: &Token) -> Result<StructuralRelationship, ParseError> {
    let value = token.value.trim();
    let (from, kind, to) = if let Some((left, to)) = value.split_once("->") {
        let (from, kind) = left
            .rsplit_once('-')
            .ok_or_else(|| token_error(token, "invalid relationship"))?;
        (from, kind, to)
    } else if let Some((to, right)) = value.split_once("<-") {
        let (kind, from) = right
            .split_once('-')
            .ok_or_else(|| token_error(token, "invalid relationship"))?;
        (from, kind, to)
    } else {
        return Err(token_error(token, "invalid requirement relationship"));
    };
    let label = kind.trim().to_ascii_lowercase();
    let relationship_kind = match label.as_str() {
        "contains" => RelKind::Composition,
        "copies" => RelKind::Association,
        "derives" | "verifies" => RelKind::Dependency,
        "satisfies" => RelKind::Realization,
        "refines" => RelKind::Inheritance,
        "traces" => RelKind::Link,
        _ => return Err(token_error(token, "unknown requirement relationship")),
    };
    Ok(StructuralRelationship {
        from: unquote_requirement_value(from),
        to: unquote_requirement_value(to),
        kind: relationship_kind,
        from_mult: None,
        to_mult: None,
        label: Some(label),
    })
}

pub fn parse_journey(source: &str) -> Result<(Option<String>, JourneyDiagram), ParseError> {
    let number = |key| quadrant_directive_value(source, key).and_then(|value| value.parse().ok());
    let font_size = |key| quadrant_directive_value(source, key).and_then(parse_mermaid_font_size);
    let config = JourneyConfig {
        diagram_margin_x: number("diagramMarginX"),
        diagram_margin_y: number("diagramMarginY"),
        task_width: number("width"),
        task_height: number("height"),
        task_margin: number("taskMargin"),
        task_font_size: font_size("taskFontSize"),
        task_font_family: quadrant_directive_value(source, "taskFontFamily"),
        title_font_size: font_size("titleFontSize"),
        title_font_family: quadrant_directive_value(source, "titleFontFamily"),
        title_color: quadrant_directive_value(source, "titleColor"),
        actor_colors: mermaid_directive_string_array(source, "actorColours"),
        section_fills: mermaid_directive_string_array(source, "sectionFills"),
        section_colors: mermaid_directive_string_array(source, "sectionColours"),
        left_margin: number("leftMargin"),
        max_label_width: number("maxLabelWidth"),
    };
    let preprocessed = preprocess_mermaid_source(source)?;
    let tokens =
        try_tokenize_mermaid_journey(&preprocessed.source).map_err(|message| ParseError {
            message,
            line: 1,
            col: 1,
        })?;
    let grammar = parse_parser_grammar(JOURNEY_PARSER_GRAMMAR_SOURCE)
        .unwrap_or_else(|error| panic!("Failed to parse journey.grammar: {error}"));
    let mut grammar_parser =
        GrammarParser::new(tokens.clone(), grammar).with_max_depth(MAX_RULE_DEPTH);
    grammar_parser.parse().map_err(|error| ParseError {
        message: error.message,
        line: error.token.line,
        col: error.token.column,
    })?;

    let mut title = None;
    let mut accessibility_title = None;
    let mut accessibility_description = None;
    let mut sections = Vec::<JourneySection>::new();
    for token in tokens {
        match token.type_name.as_deref() {
            Some("TITLE_STATEMENT") => {
                title = Some(normalize_mermaid_line_breaks(token.value["title".len()..].trim()));
            }
            Some("ACC_TITLE_STATEMENT") => {
                accessibility_title = token
                    .value
                    .split_once(':')
                    .map(|(_, value)| value.trim().to_string());
            }
            Some("ACC_DESCR_STATEMENT") => {
                accessibility_description = token
                    .value
                    .split_once(':')
                    .map(|(_, value)| value.trim().to_string());
            }
            Some("ACC_DESCR_BLOCK") => {
                let open = token.value.find('{').expect("journey token requires '{'");
                let close = token.value.rfind('}').expect("journey token requires '}'");
                accessibility_description = Some(
                    token.value[open + 1..close]
                        .lines()
                        .map(str::trim)
                        .filter(|line| !line.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
            }
            Some("SECTION_STATEMENT") => sections.push(JourneySection {
                label: normalize_mermaid_line_breaks(token.value["section".len()..].trim()),
                tasks: Vec::new(),
            }),
            Some("TASK_STATEMENT") => {
                let mut parts = token.value.split(':');
                let label = normalize_mermaid_line_breaks(parts.next().unwrap_or_default().trim());
                let score = parts
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .parse::<u8>()
                    .map_err(|_| token_error(&token, "invalid journey task score"))?;
                let people = parts
                    .next()
                    .map(|value| {
                        value
                            .split(',')
                            .map(str::trim)
                            .filter(|person| !person.is_empty())
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default();
                let section = sections
                    .last_mut()
                    .ok_or_else(|| token_error(&token, "journey task must follow a section"))?;
                section.tasks.push(JourneyTask {
                    label,
                    score,
                    people,
                });
            }
            _ => {}
        }
    }
    Ok((
        title,
        JourneyDiagram {
            accessibility_title,
            accessibility_description,
            config,
            sections,
        },
    ))
}

fn parse_mermaid_font_size(value: String) -> Option<f64> {
    let value = value.trim();
    for (suffix, scale) in [("rem", 16.0), ("px", 1.0), ("em", 16.0), ("ex", 8.0)] {
        if let Some(number) = value.strip_suffix(suffix) {
            return number.trim().parse::<f64>().ok().map(|size| size * scale);
        }
    }
    value.parse().ok()
}

fn mermaid_directive_string_array(source: &str, key: &str) -> Vec<String> {
    for quote in ['"', '\''] {
        let needle = format!("{quote}{key}{quote}");
        let Some(after_key) = source
            .find(&needle)
            .map(|index| &source[index + needle.len()..])
        else {
            continue;
        };
        let Some(after_open) = after_key.split_once('[').map(|(_, value)| value) else {
            continue;
        };
        let Some((values, _)) = after_open.split_once(']') else {
            continue;
        };
        return values
            .split(',')
            .map(str::trim)
            .map(|value| value.trim_matches(['"', '\'']))
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect();
    }
    Vec::new()
}

fn normalize_mermaid_line_breaks(source: &str) -> String {
    let mut output = String::new();
    let mut rest = source;
    while let Some(open) = rest.find('<') {
        output.push_str(&rest[..open]);
        let Some(relative_close) = rest[open + 1..].find('>') else {
            output.push_str(&rest[open..]);
            return output;
        };
        let close = open + 1 + relative_close;
        let tag = rest[open + 1..close]
            .chars()
            .filter(|character| !character.is_whitespace())
            .flat_map(char::to_lowercase)
            .collect::<String>();
        if matches!(tag.as_str(), "br" | "br/" | "/br" | "/br/") {
            output.push('\n');
        } else {
            output.push_str(&rest[open..=close]);
        }
        rest = &rest[close + 1..];
    }
    output.push_str(rest);
    output
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
                    metadata: None,
                    style: None,
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
                        metadata: None,
                        style: None,
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
        accessibility_title: None,
        accessibility_description: None,
        direction: None,
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
    parse_mermaid_xychart_ast(source)?;
    let xy_config = parse_xychart_config(source);
    let tokens = try_tokenize_mermaid_xychart(source).map_err(|message| ParseError {
        message,
        line: 1,
        col: 1,
    })?;
    let mut cursor = TokenCursor::new(tokens);
    cursor.skip_terminators();
    cursor
        .consume_if("HEADER")
        .ok_or_else(|| token_error(cursor.current(), "expected xychart header"))?;

    let mut title: Option<String> = None;
    let mut accessibility_title = None;
    let mut accessibility_description = None;
    let mut orientation = xy_config
        .chart_orientation
        .clone()
        .unwrap_or(ChartOrientation::Vertical);
    if let Some(token) = cursor.consume_if("ORIENTATION") {
        orientation = if token.value.eq_ignore_ascii_case("horizontal") {
            ChartOrientation::Horizontal
        } else {
            ChartOrientation::Vertical
        };
    }
    cursor.skip_terminators();

    let mut x_axis = None;
    let mut y_axis = None;
    let mut series: Vec<ChartSeries> = Vec::new();

    while !cursor.at_eof() {
        let token = cursor.advance().clone();
        match token_name(&token) {
            "ACC_TITLE_STATEMENT" => {
                accessibility_title = Some(xychart_metadata_value(&token));
            }
            "ACC_DESCR_STATEMENT" => {
                accessibility_description = Some(xychart_metadata_value(&token));
            }
            "ACC_DESCR_BLOCK" => {
                let open = token.value.find('{').expect("grammar requires '{'");
                let close = token.value.rfind('}').expect("grammar requires '}'");
                accessibility_description = Some(token.value[open + 1..close].trim().to_string());
            }
            "TITLE_STATEMENT" => {
                title = Some(unquote_mermaid_string(token.value["title".len()..].trim()));
            }
            "X_AXIS_STATEMENT" => {
                x_axis = Some(parse_xychart_axis(&token, true)?);
            }
            "Y_AXIS_STATEMENT" => {
                y_axis = Some(parse_xychart_axis(&token, false)?);
            }
            "BAR_STATEMENT" => series.push(parse_xychart_series(&token, SeriesKind::Bar)?),
            "LINE_STATEMENT" => series.push(parse_xychart_series(&token, SeriesKind::Line)?),
            "NEWLINE" | "SEMICOLON" => {}
            other => {
                return Err(token_error(
                    &token,
                    format!("unexpected XY-chart token {other}"),
                ))
            }
        }
    }

    let point_count = series
        .iter()
        .map(|series| series.data.len())
        .max()
        .unwrap_or(0);
    match x_axis.as_mut() {
        Some(axis) if axis.kind == AxisKind::Categorical && !axis.categories.is_empty() => {
            for plot in &mut series {
                plot.data.truncate(axis.categories.len());
            }
        }
        Some(axis) if axis.kind == AxisKind::Categorical && point_count > 0 => {
            axis.kind = AxisKind::Numeric;
            axis.min = 1.0;
            axis.max = point_count as f64;
        }
        None if point_count > 0 => {
            x_axis = Some(Axis {
                kind: AxisKind::Numeric,
                title: None,
                categories: vec![],
                min: 1.0,
                max: point_count as f64,
            });
        }
        _ => {}
    }

    if y_axis.is_none() {
        y_axis = Some(Axis {
            kind: AxisKind::Numeric,
            title: None,
            categories: vec![],
            min: 0.0,
            max: 0.0,
        });
    }

    Ok(ChartDiagram {
        title,
        accessibility_title,
        accessibility_description,
        kind: ChartKind::Xy,
        show_data: false,
        x_axis,
        y_axis,
        series,
        slices: vec![],
        sankey_nodes: vec![],
        flows: vec![],
        quadrant_labels: [None, None, None, None],
        quadrant_points: vec![],
        quadrant_config: QuadrantConfig::default(),
        xy_config,
        orientation,
    })
}

fn xychart_metadata_value(token: &Token) -> String {
    token
        .value
        .split_once(':')
        .expect("metadata token requires ':'")
        .1
        .trim()
        .to_string()
}

fn parse_xychart_config(source: &str) -> XyChartConfig {
    let chart_source = mermaid_directive_object(source, "xyChart").unwrap_or(source);
    let theme_source = mermaid_directive_object(source, "themeVariables")
        .and_then(|theme| mermaid_directive_object(theme, "xyChart"))
        .unwrap_or(source);
    let positive_number = |key| {
        quadrant_directive_value(chart_source, key)
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| *value > 0.0)
    };
    let non_negative_number = |key| {
        quadrant_directive_value(chart_source, key)
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| *value >= 0.0)
    };
    let boolean = |key| {
        quadrant_directive_value(chart_source, key).and_then(|value| {
            match value.to_ascii_lowercase().as_str() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            }
        })
    };

    let mut config = XyChartConfig {
        background_color: quadrant_directive_value(theme_source, "backgroundColor"),
        title_color: quadrant_directive_value(theme_source, "titleColor"),
        plot_color_palette: quadrant_directive_value(theme_source, "plotColorPalette").and_then(
            |palette| {
                let colors: Vec<String> = palette
                    .split(',')
                    .map(str::trim)
                    .filter(|color| !color.is_empty())
                    .map(str::to_string)
                    .collect();
                (!colors.is_empty()).then_some(colors)
            },
        ),
        width: positive_number("width"),
        height: positive_number("height"),
        chart_orientation: quadrant_directive_value(chart_source, "chartOrientation").and_then(
            |value| match value.to_ascii_lowercase().as_str() {
                "horizontal" => Some(ChartOrientation::Horizontal),
                "vertical" => Some(ChartOrientation::Vertical),
                _ => None,
            },
        ),
        plot_reserved_space_percent: positive_number("plotReservedSpacePercent")
            .filter(|value| *value >= 30.0),
        title_font_size: positive_number("titleFontSize"),
        title_padding: non_negative_number("titlePadding"),
        show_title: boolean("showTitle"),
        show_legend: boolean("showLegend"),
        legend_font_size: positive_number("legendFontSize"),
        legend_padding: non_negative_number("legendPadding"),
        show_data_label: boolean("showDataLabel"),
        show_data_label_outside_bar: boolean("showDataLabelOutsideBar"),
        data_label_color: quadrant_directive_value(theme_source, "dataLabelColor"),
        x_axis: parse_xychart_axis_config(chart_source, "xAxis"),
        y_axis: parse_xychart_axis_config(chart_source, "yAxis"),
    };
    config.x_axis.label_color = quadrant_directive_value(theme_source, "xAxisLabelColor");
    config.x_axis.title_color = quadrant_directive_value(theme_source, "xAxisTitleColor");
    config.x_axis.tick_color = quadrant_directive_value(theme_source, "xAxisTickColor");
    config.x_axis.axis_line_color = quadrant_directive_value(theme_source, "xAxisLineColor");
    config.y_axis.label_color = quadrant_directive_value(theme_source, "yAxisLabelColor");
    config.y_axis.title_color = quadrant_directive_value(theme_source, "yAxisTitleColor");
    config.y_axis.tick_color = quadrant_directive_value(theme_source, "yAxisTickColor");
    config.y_axis.axis_line_color = quadrant_directive_value(theme_source, "yAxisLineColor");
    config
}

fn parse_xychart_axis_config(source: &str, key: &str) -> XyAxisConfig {
    let Some(axis_source) = mermaid_directive_object(source, key) else {
        return XyAxisConfig::default();
    };
    let number = |key| {
        quadrant_directive_value(axis_source, key)
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| *value >= 0.0)
    };
    let boolean = |key| {
        quadrant_directive_value(axis_source, key).and_then(|value| {
            match value.to_ascii_lowercase().as_str() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            }
        })
    };
    XyAxisConfig {
        show_label: boolean("showLabel"),
        label_font_size: number("labelFontSize").filter(|value| *value > 0.0),
        label_padding: number("labelPadding"),
        label_rotation: quadrant_directive_value(axis_source, "labelRotation")
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| (-90.0..=90.0).contains(value)),
        label_color: None,
        show_title: boolean("showTitle"),
        title_font_size: number("titleFontSize").filter(|value| *value > 0.0),
        title_padding: number("titlePadding"),
        title_color: None,
        show_tick: boolean("showTick"),
        tick_length: number("tickLength"),
        tick_width: number("tickWidth").filter(|value| *value > 0.0),
        tick_color: None,
        show_axis_line: boolean("showAxisLine"),
        axis_line_width: number("axisLineWidth").filter(|value| *value > 0.0),
        axis_line_color: None,
    }
}

fn parse_xychart_axis(token: &Token, is_x: bool) -> Result<Axis, ParseError> {
    let keyword_len = "x-axis".len();
    let rest = token.value[keyword_len..].trim();
    if let Some((open, close)) = xychart_bracket_bounds(token, rest)? {
        if !is_x {
            return Err(token_error(
                token,
                "Mermaid XY y-axis cannot be categorical",
            ));
        }
        if !rest[close + 1..].trim().is_empty() {
            return Err(token_error(
                token,
                "unexpected content after axis categories",
            ));
        }
        let categories = parse_bracket_list(&rest[open..=close]);
        if categories.is_empty() {
            return Err(token_error(token, "XY-chart categories cannot be empty"));
        }
        return Ok(Axis {
            kind: AxisKind::Categorical,
            title: xychart_optional_text(&rest[..open]),
            categories,
            min: 0.0,
            max: 0.0,
        });
    }

    if let Some((left, right)) = rest.split_once("-->") {
        let (title, min_text) = xychart_title_and_number(left);
        let max_text = right.trim();
        let min = min_text
            .parse::<f64>()
            .map_err(|_| token_error(token, "invalid XY-chart axis minimum"))?;
        let max = max_text
            .parse::<f64>()
            .map_err(|_| token_error(token, "invalid XY-chart axis maximum"))?;
        return Ok(Axis {
            kind: AxisKind::Numeric,
            title,
            categories: vec![],
            min,
            max,
        });
    }

    Ok(Axis {
        kind: if is_x {
            AxisKind::Categorical
        } else {
            AxisKind::Numeric
        },
        title: xychart_optional_text(rest),
        categories: vec![],
        min: 0.0,
        max: 0.0,
    })
}

fn xychart_title_and_number(value: &str) -> (Option<String>, &str) {
    let value = value.trim();
    let Some(split) = value.rfind(char::is_whitespace) else {
        return (None, value);
    };
    let title = xychart_optional_text(&value[..split]);
    (title, value[split..].trim())
}

fn xychart_optional_text(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| unquote_mermaid_string(value))
}

fn parse_xychart_series(token: &Token, kind: SeriesKind) -> Result<ChartSeries, ParseError> {
    let keyword_len = match kind {
        SeriesKind::Bar => "bar".len(),
        SeriesKind::Line => "line".len(),
    };
    let rest = token.value[keyword_len..].trim();
    let (open, close) =
        xychart_bracket_bounds(token, rest)?.ok_or_else(|| token_error(token, "expected '['"))?;
    if !rest[close + 1..].trim().is_empty() {
        return Err(token_error(token, "unexpected content after XY-chart data"));
    }
    let data = parse_xychart_data_points(token, &rest[open + 1..close])?;
    if data.is_empty() {
        return Err(token_error(token, "XY-chart series cannot be empty"));
    }
    Ok(ChartSeries {
        kind,
        label: xychart_optional_text(&rest[..open]),
        data,
    })
}

fn xychart_bracket_bounds(
    token: &Token,
    source: &str,
) -> Result<Option<(usize, usize)>, ParseError> {
    let mut quoted = false;
    let mut open = None;
    let mut close = None;
    for (index, ch) in source.char_indices() {
        match ch {
            '"' => quoted = !quoted,
            '[' if !quoted && open.is_some() => {
                return Err(token_error(token, "unexpected '['"));
            }
            '[' if !quoted => open = Some(index),
            ']' if !quoted && close.is_some() => {
                return Err(token_error(token, "unexpected ']'"));
            }
            ']' if !quoted => close = Some(index),
            _ => {}
        }
    }
    if quoted {
        return Err(token_error(token, "unterminated XY-chart string"));
    }
    match (open, close) {
        (None, None) => Ok(None),
        (Some(_), None) => Err(token_error(token, "expected ']'")),
        (None, Some(_)) => Err(token_error(token, "unexpected ']'")),
        (Some(open), Some(close)) if open < close => Ok(Some((open, close))),
        _ => Err(token_error(token, "unexpected ']' before '['")),
    }
}

fn parse_xychart_data_points(
    token: &Token,
    source: &str,
) -> Result<Vec<ChartDataPoint>, ParseError> {
    let mut points = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    for (index, ch) in source.char_indices() {
        match ch {
            '"' => quoted = !quoted,
            ',' if !quoted => {
                points.push(parse_xychart_data_point(token, &source[start..index])?);
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    if quoted {
        return Err(token_error(token, "unterminated XY-chart point label"));
    }
    points.push(parse_xychart_data_point(token, &source[start..])?);
    Ok(points)
}

fn parse_xychart_data_point(token: &Token, source: &str) -> Result<ChartDataPoint, ParseError> {
    let source = source.trim();
    if source.is_empty() {
        return Err(token_error(token, "XY-chart data point cannot be empty"));
    }
    let number_end = source
        .find(|ch: char| ch.is_whitespace() || ch == '"')
        .unwrap_or(source.len());
    let number = &source[..number_end];
    let value = number
        .parse::<f64>()
        .map_err(|_| token_error(token, format!("invalid XY-chart data point {number:?}")))?;
    let remainder = source[number_end..].trim();
    let label = if remainder.is_empty() {
        None
    } else if let Some(quoted_label) = remainder.strip_prefix('"') {
        let close = quoted_label
            .find('"')
            .ok_or_else(|| token_error(token, "unterminated XY-chart point label"))?;
        if !quoted_label[close + 1..].trim().is_empty() {
            return Err(token_error(
                token,
                "unexpected content after XY-chart point label",
            ));
        }
        Some(quoted_label[..close].to_string())
    } else {
        return Err(token_error(token, "invalid XY-chart point label"));
    };
    Ok(ChartDataPoint { value, label })
}

/// Parse the grammar-backed native subset of Mermaid `quadrantChart`.
#[derive(Clone, Debug, Default)]
struct QuadrantPointStyle {
    radius: Option<f64>,
    color: Option<String>,
    stroke_color: Option<String>,
    stroke_width: Option<f64>,
}

impl QuadrantPointStyle {
    fn overlay(&mut self, other: &Self) {
        if other.radius.is_some() {
            self.radius = other.radius;
        }
        if other.color.is_some() {
            self.color.clone_from(&other.color);
        }
        if other.stroke_color.is_some() {
            self.stroke_color.clone_from(&other.stroke_color);
        }
        if other.stroke_width.is_some() {
            self.stroke_width = other.stroke_width;
        }
    }
}

fn parse_quadrant_point_style(raw: &str, token: &Token) -> Result<QuadrantPointStyle, ParseError> {
    let mut style = QuadrantPointStyle::default();
    for declaration in raw
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let (property, value) = declaration.split_once(':').ok_or_else(|| {
            token_error(
                token,
                format!("invalid quadrant point style {declaration:?}"),
            )
        })?;
        let property = property.trim();
        let value = value.trim();
        let digits = |raw: &str| !raw.is_empty() && raw.bytes().all(|byte| byte.is_ascii_digit());
        let hex_color = |raw: &str| {
            let digits = raw.strip_prefix('#').unwrap_or(raw);
            matches!(digits.len(), 3 | 6) && digits.bytes().all(|byte| byte.is_ascii_hexdigit())
        };
        match property {
            "radius" => {
                if !digits(value) {
                    return Err(token_error(token, "invalid quadrant point radius"));
                }
                style.radius = Some(value.parse().expect("validated decimal radius"));
            }
            "color" => {
                if !hex_color(value) {
                    return Err(token_error(token, "invalid quadrant point color"));
                }
                style.color = Some(value.to_string());
            }
            "stroke-color" => {
                if !hex_color(value) {
                    return Err(token_error(token, "invalid quadrant point stroke color"));
                }
                style.stroke_color = Some(value.to_string());
            }
            "stroke-width" => {
                let Some(pixels) = value.strip_suffix("px") else {
                    return Err(token_error(token, "invalid quadrant point stroke width"));
                };
                if !digits(pixels) {
                    return Err(token_error(token, "invalid quadrant point stroke width"));
                }
                style.stroke_width = Some(pixels.parse().expect("validated pixel width"));
            }
            _ => {
                return Err(token_error(
                    token,
                    format!("unsupported quadrant point style {property:?}"),
                ))
            }
        }
    }
    Ok(style)
}

pub fn parse_quadrant_chart(source: &str) -> Result<ChartDiagram, ParseError> {
    let quadrant_config = parse_quadrant_config(source);
    let preprocessed = preprocess_mermaid_source(source)?;
    let source = preprocessed.source.as_str();
    parse_mermaid_quadrant_ast(source)?;

    let tokens = try_tokenize_mermaid_quadrant(source).map_err(|message| ParseError {
        message,
        line: 1,
        col: 1,
    })?;
    let mut cursor = TokenCursor::new(tokens);
    cursor.skip_terminators();
    cursor
        .consume_if("HEADER")
        .ok_or_else(|| token_error(cursor.current(), "expected quadrantChart header"))?;
    cursor.skip_terminators();

    let mut title = None;
    let mut accessibility_title = None;
    let mut accessibility_description = None;
    let mut x_labels = Vec::new();
    let mut y_labels = Vec::new();
    let mut quadrant_labels: [Option<String>; 4] = [None, None, None, None];
    let mut point_classes: HashMap<String, QuadrantPointStyle> = HashMap::new();
    let mut pending_points = Vec::new();

    while !cursor.at_eof() {
        let token = cursor.advance().clone();
        match token_name(&token) {
            "ACC_TITLE_STATEMENT" => {
                accessibility_title = Some(
                    token
                        .value
                        .split_once(':')
                        .expect("token grammar requires ':'")
                        .1
                        .trim()
                        .to_string(),
                );
            }
            "ACC_DESCR_STATEMENT" => {
                accessibility_description = Some(
                    token
                        .value
                        .split_once(':')
                        .expect("token grammar requires ':'")
                        .1
                        .trim()
                        .to_string(),
                );
            }
            "ACC_DESCR_BLOCK" => {
                let open = token.value.find('{').expect("token grammar requires '{'");
                let close = token.value.rfind('}').expect("token grammar requires '}'");
                accessibility_description = Some(token.value[open + 1..close].trim().to_string());
            }
            "TITLE_STATEMENT" => {
                title = Some(token.value["title".len()..].trim().to_string());
            }
            "AXIS_STATEMENT" => {
                let is_x = token.value[..6].eq_ignore_ascii_case("x-axis");
                let value = token.value[6..].trim();
                let has_dangling_arrow = quadrant_axis_has_dangling_arrow(value);
                let mut labels = split_quadrant_axis_labels(value)
                    .into_iter()
                    .map(|part| unquote_mermaid_string(part.trim()))
                    .collect::<Vec<_>>();
                labels.retain(|label| !label.is_empty());
                if has_dangling_arrow {
                    if let Some(label) = labels.first_mut() {
                        label.push_str(" ⟶ ");
                    }
                }
                if is_x {
                    x_labels = labels;
                } else {
                    y_labels = labels;
                }
            }
            "QUADRANT_STATEMENT" => {
                let index = token.value.as_bytes()[9] as usize - b'1' as usize;
                let label = token.value[10..].trim();
                quadrant_labels[index] = Some(unquote_mermaid_string(label));
            }
            "CLASSDEF_STATEMENT" => {
                let rest = token.value["classDef".len()..].trim();
                let (name, declarations) =
                    rest.split_once(char::is_whitespace).ok_or_else(|| {
                        token_error(&token, "expected class name and quadrant point styles")
                    })?;
                point_classes.insert(
                    name.to_string(),
                    parse_quadrant_point_style(declarations, &token)?,
                );
            }
            "POINT_STATEMENT" => {
                let open = token.value.rfind('[').ok_or_else(|| {
                    token_error(&token, "expected '[' before quadrant point coordinates")
                })?;
                let close = token.value.rfind(']').ok_or_else(|| {
                    token_error(&token, "expected ']' after quadrant point coordinates")
                })?;
                let point_ref = token.value[..open]
                    .trim()
                    .strip_suffix(':')
                    .map(str::trim)
                    .ok_or_else(|| token_error(&token, "expected ':' before quadrant point"))?;
                let (label, class_name) = point_ref
                    .split_once(":::")
                    .map(|(label, class_name)| (label.trim(), Some(class_name.trim().to_string())))
                    .unwrap_or((point_ref, None));
                let coordinates = token.value[open + 1..close]
                    .split(',')
                    .map(str::trim)
                    .collect::<Vec<_>>();
                let [x, y] = coordinates.as_slice() else {
                    return Err(token_error(
                        &token,
                        "expected two quadrant point coordinates",
                    ));
                };
                let inline_style = parse_quadrant_point_style(&token.value[close + 1..], &token)?;
                pending_points.push((
                    unquote_mermaid_string(label),
                    class_name,
                    x.parse()
                        .map_err(|_| token_error(&token, "invalid quadrant x value"))?,
                    y.parse()
                        .map_err(|_| token_error(&token, "invalid quadrant y value"))?,
                    inline_style,
                ));
            }
            _ => return Err(token_error(&token, "unsupported quadrant-chart statement")),
        }
        cursor.skip_terminators();
    }

    let axis = |categories: Vec<String>| {
        (!categories.is_empty()).then_some(Axis {
            kind: AxisKind::Numeric,
            title: None,
            categories,
            min: 0.0,
            max: 1.0,
        })
    };
    let quadrant_points = pending_points
        .into_iter()
        .map(|(label, class_name, x, y, inline_style)| {
            let mut style = class_name
                .as_ref()
                .and_then(|name| point_classes.get(name))
                .cloned()
                .unwrap_or_default();
            style.overlay(&inline_style);
            QuadrantPoint {
                label,
                x,
                y,
                radius: style.radius,
                color: style.color,
                stroke_color: style.stroke_color,
                stroke_width: style.stroke_width,
            }
        })
        .collect();

    Ok(ChartDiagram {
        title,
        accessibility_title,
        accessibility_description,
        kind: ChartKind::Quadrant,
        show_data: false,
        x_axis: axis(x_labels),
        y_axis: axis(y_labels),
        series: vec![],
        slices: vec![],
        sankey_nodes: vec![],
        flows: vec![],
        quadrant_labels,
        quadrant_points,
        quadrant_config,
        xy_config: XyChartConfig::default(),
        orientation: ChartOrientation::Vertical,
    })
}

fn parse_quadrant_config(source: &str) -> QuadrantConfig {
    let number = |key| quadrant_directive_value(source, key).and_then(|value| value.parse().ok());
    QuadrantConfig {
        chart_width: number("chartWidth"),
        chart_height: number("chartHeight"),
        x_axis_position: quadrant_directive_value(source, "xAxisPosition"),
        y_axis_position: quadrant_directive_value(source, "yAxisPosition"),
        point_radius: number("pointRadius"),
        quadrant_padding: number("quadrantPadding"),
        internal_border_width: number("quadrantInternalBorderStrokeWidth"),
        external_border_width: number("quadrantExternalBorderStrokeWidth"),
        title_font_size: number("titleFontSize"),
        title_padding: number("titlePadding"),
        x_axis_label_font_size: number("xAxisLabelFontSize"),
        x_axis_label_padding: number("xAxisLabelPadding"),
        y_axis_label_font_size: number("yAxisLabelFontSize"),
        y_axis_label_padding: number("yAxisLabelPadding"),
        quadrant_label_font_size: number("quadrantLabelFontSize"),
        quadrant_text_top_padding: number("quadrantTextTopPadding"),
        point_label_font_size: number("pointLabelFontSize"),
        point_text_padding: number("pointTextPadding"),
        quadrant_fills: std::array::from_fn(|index| {
            quadrant_directive_value(source, &format!("quadrant{}Fill", index + 1))
        }),
        quadrant_text_fills: std::array::from_fn(|index| {
            quadrant_directive_value(source, &format!("quadrant{}TextFill", index + 1))
        }),
        point_fill: quadrant_directive_value(source, "quadrantPointFill"),
        point_text_fill: quadrant_directive_value(source, "quadrantPointTextFill"),
        x_axis_text_fill: quadrant_directive_value(source, "quadrantXAxisTextFill"),
        y_axis_text_fill: quadrant_directive_value(source, "quadrantYAxisTextFill"),
        internal_border_stroke_fill: quadrant_directive_value(
            source,
            "quadrantInternalBorderStrokeFill",
        ),
        external_border_stroke_fill: quadrant_directive_value(
            source,
            "quadrantExternalBorderStrokeFill",
        ),
        title_fill: quadrant_directive_value(source, "quadrantTitleFill"),
    }
}

fn quadrant_directive_value(source: &str, key: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let needle = format!("{quote}{key}{quote}");
        let Some(after_key) = source
            .find(&needle)
            .map(|index| &source[index + needle.len()..])
        else {
            continue;
        };
        let after_colon = after_key.split_once(':')?.1.trim_start();
        if let Some(value) = after_colon.strip_prefix(['"', '\'']) {
            return value.find(['"', '\'']).map(|end| value[..end].to_string());
        }
        let end = after_colon
            .find([',', '}', ' '])
            .unwrap_or(after_colon.len());
        return Some(after_colon[..end].trim().to_string());
    }
    None
}

fn mermaid_directive_object<'a>(source: &'a str, key: &str) -> Option<&'a str> {
    let key_start = ['"', '\'']
        .into_iter()
        .find_map(|quote| source.find(&format!("{quote}{key}{quote}")))?;
    let after_key = &source[key_start + key.len() + 2..];
    let object_start = after_key.find('{')?;
    let object = &after_key[object_start..];
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in object.char_indices() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == active_quote {
                quote = None;
            }
            continue;
        }
        match character {
            '"' | '\'' => quote = Some(character),
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&object[1..index]);
                }
            }
            _ => {}
        }
    }
    None
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

// ── state parser ─────────────────────────────────────────────────────────

/// Parse the graph-compatible core of Mermaid state diagrams.
///
/// The supported state slice lowers flat declarations, transitions,
/// pseudostates, composite groups, notes, metadata, and styles into graph IR.
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
    let mut requested_width = None;
    let mut hide_empty_descriptions = false;
    let mut title = None;
    let mut accessibility_title = None;
    let mut accessibility_description = None;
    let mut nodes = Vec::new();
    let mut node_indices = HashMap::new();
    let mut edges = Vec::new();
    let mut links = Vec::new();
    let mut groups = Vec::new();
    let mut group_stack: Vec<String> = Vec::new();
    let mut pseudo_index = 0;
    let mut note_index = 0;
    let mut class_styles: HashMap<String, DiagramStyle> = HashMap::new();
    let mut pending_classes: Vec<(Vec<String>, String)> = Vec::new();
    let mut membership_cursor = 0;

    while !cursor.at_eof() {
        record_new_state_group_members(&group_stack, &mut groups, &nodes, membership_cursor);
        membership_cursor = nodes.len();
        if cursor.consume_if("RBRACE").is_some() {
            let group_id = group_stack.pop().ok_or_else(|| {
                token_error(cursor.current(), "unexpected composite state closing brace")
            })?;
            let group = groups
                .iter()
                .find(|group| group.id == group_id)
                .expect("open composite group must exist");
            if group.regions.len() > 1 && group.regions.last().is_some_and(Vec::is_empty) {
                return Err(token_error(
                    cursor.current(),
                    "concurrent state region cannot be empty",
                ));
            }
            cursor.skip_terminators();
            continue;
        }
        if cursor.consume_if("CONCURRENT").is_some() {
            let group_id = group_stack.last().ok_or_else(|| {
                token_error(
                    cursor.current(),
                    "concurrent state divider requires a composite state",
                )
            })?;
            let group = groups
                .iter_mut()
                .find(|group| &group.id == group_id)
                .expect("open composite group must exist");
            if group.regions.last().is_none_or(Vec::is_empty) {
                return Err(token_error(
                    cursor.current(),
                    "concurrent state region cannot be empty",
                ));
            }
            group.regions.push(Vec::new());
            cursor.skip_terminators();
            continue;
        } else if cursor.consume_if("HIDE_EMPTY").is_some() {
            hide_empty_descriptions = true;
        } else if cursor.current().value.eq_ignore_ascii_case("scale") {
            cursor.advance();
            let width =
                cursor.advance().value.parse::<f64>().map_err(|_| {
                    token_error(cursor.current(), "expected numeric state scale width")
                })?;
            if width <= 0.0 {
                return Err(token_error(
                    cursor.current(),
                    "state scale width must be positive",
                ));
            }
            if !cursor.current().value.eq_ignore_ascii_case("width") {
                return Err(token_error(
                    cursor.current(),
                    "expected width after state scale value",
                ));
            }
            cursor.advance();
            requested_width = Some(width);
        } else if cursor.current().value.eq_ignore_ascii_case("title") {
            cursor.advance();
            cursor.consume_if("COLON");
            title = Some(take_state_text(&mut cursor));
        } else if token_name(cursor.current()) == "ACC_TITLE" {
            cursor.advance();
            accessibility_title = Some(take_state_text(&mut cursor));
        } else if token_name(cursor.current()) == "ACC_DESCR" {
            cursor.advance();
            accessibility_description = Some(take_state_text(&mut cursor));
        } else if token_name(cursor.current()) == "ACC_DESCR_START" {
            cursor.advance();
            cursor.consume_if("NEWLINE").ok_or_else(|| {
                token_error(
                    cursor.current(),
                    "expected newline before multiline accessibility description",
                )
            })?;
            accessibility_description = Some(take_state_multiline_accessibility_text(&mut cursor)?);
        } else if cursor.current().value.eq_ignore_ascii_case("direction") {
            cursor.advance();
            let token = cursor
                .consume_if("DIRECTION")
                .ok_or_else(|| token_error(cursor.current(), "expected state direction"))?;
            let parsed_direction = match token.value.to_ascii_uppercase().as_str() {
                "TB" => DiagramDirection::Tb,
                "BT" => DiagramDirection::Bt,
                "LR" => DiagramDirection::Lr,
                "RL" => DiagramDirection::Rl,
                _ => unreachable!("state.tokens restricts direction values"),
            };
            if let Some(group_id) = group_stack.last() {
                groups
                    .iter_mut()
                    .find(|group| &group.id == group_id)
                    .expect("open composite group must exist")
                    .direction = Some(parsed_direction);
            } else {
                direction = parsed_direction;
            }
        } else if cursor.current().value.eq_ignore_ascii_case("click") {
            cursor.advance();
            let node_id = take_state_ref(&mut cursor)?;
            if cursor.current().value.eq_ignore_ascii_case("href") {
                cursor.advance();
            }
            if token_name(cursor.current()) != "STRING" {
                return Err(token_error(cursor.current(), "expected state click URL"));
            }
            let url = strip_state_string(&cursor.advance().value);
            let tooltip = if token_name(cursor.current()) == "STRING" {
                Some(strip_state_string(&cursor.advance().value))
            } else {
                None
            };
            if !node_indices.contains_key(&node_id) {
                upsert_state_node(
                    &mut nodes,
                    &mut node_indices,
                    node_id.clone(),
                    node_id.clone(),
                );
            }
            links.retain(|link: &GraphLink| link.node_id != node_id);
            links.push(GraphLink {
                node_id,
                url,
                tooltip,
            });
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
            let mut class_names = vec![take_state_ref(&mut cursor)?];
            while matches!(token_name(cursor.current()), "ID" | "WORD") {
                class_names.push(take_state_ref(&mut cursor)?);
            }
            for id in &ids {
                if !node_indices.contains_key(id) && !groups.iter().any(|group| &group.id == id) {
                    upsert_state_node(&mut nodes, &mut node_indices, id.clone(), id.clone());
                }
            }
            for id in ids {
                for class_name in &class_names {
                    apply_or_defer_state_class(
                        &id,
                        class_name.clone(),
                        &mut nodes,
                        &node_indices,
                        &mut groups,
                        &class_styles,
                        &mut pending_classes,
                    );
                }
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
            if !node_indices.contains_key(&state_id)
                && !groups.iter().any(|group| group.id == state_id)
            {
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
            let mut ids = vec![take_state_ref(&mut cursor)?];
            while token_name(cursor.current()) == "COMMA"
                && !state_comma_starts_style_assignment(&cursor)
            {
                cursor.advance();
                ids.push(take_state_ref(&mut cursor)?);
            }
            let mut style = DiagramStyle::default();
            parse_state_style_assignments(&mut cursor, &mut style)?;
            for id in ids {
                if let Some(group) = groups.iter_mut().find(|group| group.id == id) {
                    merge_state_style(group.style.get_or_insert_default(), &style);
                    continue;
                }
                if !node_indices.contains_key(&id) {
                    upsert_state_node(&mut nodes, &mut node_indices, id.clone(), id.clone());
                }
                merge_state_style(
                    nodes[node_indices[&id]].style.get_or_insert_default(),
                    &style,
                );
            }
        } else if cursor.current().value.eq_ignore_ascii_case("state") {
            cursor.advance();
            let (id, label) = if token_name(cursor.current()) == "STRING" {
                let mut label = strip_state_string(&cursor.advance().value);
                if !cursor.current().value.eq_ignore_ascii_case("as") {
                    return Err(token_error(
                        cursor.current(),
                        "expected state alias keyword as",
                    ));
                }
                cursor.advance();
                let id = take_state_ref(&mut cursor)?;
                if cursor.consume_if("COLON").is_some() {
                    label.push('\n');
                    label.push_str(&take_state_text(&mut cursor));
                }
                (id, label)
            } else {
                let id = take_state_ref(&mut cursor)?;
                if cursor.consume_if("LBRACE").is_some() {
                    groups.push(GraphGroup {
                        id: id.clone(),
                        label: DiagramLabel::new(id.clone()),
                        parent_id: group_stack.last().cloned(),
                        node_ids: Vec::new(),
                        regions: vec![Vec::new()],
                        direction: None,
                        style: None,
                    });
                    group_stack.push(id);
                    cursor.skip_terminators();
                    continue;
                }
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
            if cursor.consume_if("LBRACE").is_some() {
                groups.push(GraphGroup {
                    id: id.clone(),
                    label: DiagramLabel::new(label),
                    parent_id: group_stack.last().cloned(),
                    node_ids: Vec::new(),
                    regions: vec![Vec::new()],
                    direction: None,
                    style: None,
                });
                group_stack.push(id);
                cursor.skip_terminators();
                continue;
            }
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
                append_state_description(&mut nodes, &mut node_indices, from, label);
                cursor.skip_terminators();
                continue;
            }
            if let Some(class_name) = from_class {
                apply_or_defer_state_class(
                    &from,
                    class_name,
                    &mut nodes,
                    &node_indices,
                    &mut groups,
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
            if !from_is_edge_state
                && matches!(
                    token_name(cursor.current()),
                    "NEWLINE" | "SEMICOLON" | "EOF" | "RBRACE"
                )
            {
                cursor.skip_terminators();
                continue;
            }
            if !from_is_edge_state && matches!(token_name(cursor.current()), "ID" | "WORD") {
                continue;
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
                    &mut groups,
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

    record_new_state_group_members(&group_stack, &mut groups, &nodes, membership_cursor);
    if !group_stack.is_empty() {
        return Err(token_error(
            cursor.current(),
            "unterminated composite state group",
        ));
    }

    let group_ids: std::collections::HashSet<_> =
        groups.iter().map(|group| group.id.as_str()).collect();
    nodes.retain(|node| !group_ids.contains(node.id.as_str()));
    node_indices = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), index))
        .collect();

    for (ids, class_name) in pending_classes {
        let class_style = class_styles.get(&class_name).ok_or_else(|| ParseError {
            message: format!("unknown state style class {class_name:?}"),
            line: 1,
            col: 1,
        })?;
        for id in ids {
            if let Some(group) = groups.iter_mut().find(|group| group.id == id) {
                merge_state_style(group.style.get_or_insert_default(), class_style);
            } else {
                merge_state_style(
                    nodes[node_indices[&id]].style.get_or_insert_default(),
                    class_style,
                );
            }
        }
    }

    Ok(GraphDiagram {
        direction,
        requested_width,
        hide_empty_descriptions,
        title,
        accessibility_title,
        accessibility_description,
        links,
        groups,
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

fn append_state_description(
    nodes: &mut Vec<GraphNode>,
    node_indices: &mut HashMap<String, usize>,
    id: String,
    description: String,
) {
    if let Some(&index) = node_indices.get(&id) {
        let label = &mut nodes[index].label.text;
        if label == &id {
            label.clear();
        }
        if !label.is_empty() {
            label.push('\n');
        }
        label.push_str(&description);
    } else {
        upsert_state_node(nodes, node_indices, id, description);
    }
}

fn record_new_state_group_members(
    group_stack: &[String],
    groups: &mut [GraphGroup],
    nodes: &[GraphNode],
    node_count_before: usize,
) {
    let Some(group_id) = group_stack.last() else {
        return;
    };
    let Some(group) = groups.iter_mut().find(|group| &group.id == group_id) else {
        return;
    };
    for node in &nodes[node_count_before..] {
        if !group.node_ids.contains(&node.id) {
            group.node_ids.push(node.id.clone());
        }
        let region = group
            .regions
            .last_mut()
            .expect("composite groups always have a current region");
        if !region.contains(&node.id) {
            region.push(node.id.clone());
        }
    }
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
    values: &[Token],
) -> Result<(), ParseError> {
    let value = &values[0];
    let property = property.to_ascii_lowercase();
    if !matches!(property.as_str(), "border" | "font-family") && values.len() != 1 {
        return Err(token_error(
            value,
            format!("state style property {property:?} requires one value"),
        ));
    }
    match property.as_str() {
        "fill" | "background" => style.fill = Some(value.value.clone()),
        "stroke" => style.stroke = Some(value.value.clone()),
        "border" => {
            if values.len() != 3 || !values[1].value.eq_ignore_ascii_case("solid") {
                return Err(token_error(
                    value,
                    "state border must be '<width> solid <color>'",
                ));
            }
            let width = value
                .value
                .strip_suffix("px")
                .unwrap_or(&value.value)
                .parse::<f64>()
                .map_err(|_| token_error(value, "invalid state border width"))?;
            if width <= 0.0 {
                return Err(token_error(value, "state border width must be positive"));
            }
            style.stroke_width = Some(width);
            style.stroke = Some(values[2].value.clone());
        }
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
        "font-size" => {
            let size = value
                .value
                .strip_suffix("px")
                .unwrap_or(&value.value)
                .parse::<f64>()
                .map_err(|_| token_error(value, "invalid state font size"))?;
            if size <= 0.0 {
                return Err(token_error(value, "state font size must be positive"));
            }
            style.font_size = Some(size);
        }
        "font-weight" => {
            let weight = match value.value.to_ascii_lowercase().as_str() {
                "normal" => 400,
                "bold" => 700,
                numeric => numeric
                    .parse::<u16>()
                    .map_err(|_| token_error(value, "invalid state font weight"))?,
            };
            if !(100..=900).contains(&weight) || weight % 100 != 0 {
                return Err(token_error(
                    value,
                    "state font weight must be normal, bold, or 100 through 900",
                ));
            }
            style.font_weight = Some(weight);
        }
        "font-style" => {
            style.font_italic = Some(match value.value.to_ascii_lowercase().as_str() {
                "normal" => false,
                "italic" => true,
                _ => {
                    return Err(token_error(
                        value,
                        "state font style must be normal or italic",
                    ))
                }
            });
        }
        "font-family" => {
            let family = values
                .iter()
                .map(|token| strip_state_string(&token.value))
                .collect::<Vec<_>>()
                .join(" ");
            if family.trim().is_empty() {
                return Err(token_error(value, "state font family cannot be empty"));
            }
            style.font_family = Some(family);
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
        let mut values = Vec::new();
        while !cursor.at_eof()
            && !matches!(
                token_name(cursor.current()),
                "COMMA" | "NEWLINE" | "SEMICOLON"
            )
        {
            let value = cursor.advance().clone();
            if !matches!(token_name(&value), "HASH_COLOR" | "ID" | "WORD" | "STRING") {
                return Err(token_error(&value, "expected state style value"));
            }
            values.push(value);
        }
        if values.is_empty() {
            return Err(token_error(cursor.current(), "expected state style value"));
        }
        apply_state_style(style, &property, &values)?;
        if cursor.consume_if("COMMA").is_none() {
            return Ok(());
        }
    }
}

fn state_comma_starts_style_assignment(cursor: &TokenCursor) -> bool {
    cursor
        .tokens
        .get(cursor.index + 2)
        .is_some_and(|token| token_name(token) == "COLON")
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
    if source.font_size.is_some() {
        target.font_size = source.font_size;
    }
    if source.font_weight.is_some() {
        target.font_weight = source.font_weight;
    }
    if source.font_italic.is_some() {
        target.font_italic = source.font_italic;
    }
    if source.font_family.is_some() {
        target.font_family.clone_from(&source.font_family);
    }
}

fn apply_or_defer_state_class(
    id: &str,
    class_name: String,
    nodes: &mut [GraphNode],
    node_indices: &HashMap<String, usize>,
    groups: &mut [GraphGroup],
    class_styles: &HashMap<String, DiagramStyle>,
    pending_classes: &mut Vec<(Vec<String>, String)>,
) {
    if let Some(class_style) = class_styles.get(&class_name) {
        if let Some(group) = groups.iter_mut().find(|group| group.id == id) {
            merge_state_style(group.style.get_or_insert_default(), class_style);
        } else {
            merge_state_style(
                nodes[node_indices[id]].style.get_or_insert_default(),
                class_style,
            );
        }
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
        } else if token_name(token) == "ENTITY" {
            decode_mermaid_entity(&token.value)
        } else if token_name(token) == "LINE_BREAK" {
            "\n".to_string()
        } else {
            token.value.clone()
        };
        if token_name(token) == "COMMA" {
            text.push(',');
        } else if token_name(token) == "COLON" {
            text.push(':');
        } else {
            if !text.is_empty() && !text.ends_with(',') {
                text.push(' ');
            }
            text.push_str(&value);
        }
    }
    decode_state_line_breaks(text)
}

fn decode_mermaid_entity(value: &str) -> String {
    let inner = value.trim_start_matches('#').trim_end_matches(';');
    let html_entity = if inner.chars().all(|character| character.is_ascii_digit()) {
        format!("&#{inner};")
    } else {
        format!("&{inner};")
    };
    commonmark_parser::entities::decode_entity(&html_entity)
}

fn decode_state_line_breaks(text: String) -> String {
    let mut decoded = String::with_capacity(text.len());
    let mut remaining = text.as_str();
    while let Some(start) = remaining.find('<') {
        decoded.push_str(&remaining[..start]);
        let candidate = &remaining[start..];
        let Some(end) = candidate.find('>') else {
            decoded.push_str(candidate);
            remaining = "";
            break;
        };
        let inner = candidate[1..end].trim();
        let tag_name = inner.strip_suffix('/').unwrap_or(inner).trim();
        if tag_name.eq_ignore_ascii_case("br") {
            decoded.push('\n');
            remaining = &candidate[end + 1..];
        } else {
            decoded.push('<');
            remaining = &candidate[1..];
        }
    }
    decoded.push_str(remaining);
    decoded
        .split('\n')
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
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
            } else if token_name(token) == "ENTITY" {
                decode_mermaid_entity(&token.value)
            } else if token_name(token) == "LINE_BREAK" {
                "\n".to_string()
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
        lines.push(decode_state_line_breaks(line));
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

fn take_state_multiline_accessibility_text(cursor: &mut TokenCursor) -> Result<String, ParseError> {
    let mut lines = Vec::new();
    while !cursor.at_eof() && token_name(cursor.current()) != "RBRACE" {
        if cursor.consume_if("NEWLINE").is_some() {
            lines.push(String::new());
            continue;
        }
        let line = take_state_text(cursor);
        lines.push(line);
        cursor.consume_if("NEWLINE");
    }
    cursor.consume_if("RBRACE").ok_or_else(|| {
        token_error(
            cursor.current(),
            "expected '}' after accessibility description",
        )
    })?;
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
/// Supports metadata, `showData`, and quoted non-negative numeric sections.
pub fn parse_pie(source: &str) -> Result<ChartDiagram, ParseError> {
    parse_mermaid_pie_ast(source)?;

    let mut cursor = TokenCursor::new(tokenize_mermaid_pie(source));
    cursor.skip_terminators();
    cursor.expect_keyword("pie")?;

    let show_data =
        cursor.current().type_ == TokenType::Keyword && cursor.current().value == "showData";
    if show_data {
        cursor.advance();
    }
    cursor.skip_terminators();

    let mut title = None;
    let mut accessibility_title = None;
    let mut accessibility_description = None;
    let mut slices = Vec::new();
    while !cursor.at_eof() {
        match token_name(cursor.current()) {
            "TITLE" => {
                title = Some(
                    cursor
                        .advance()
                        .value
                        .strip_prefix("title")
                        .expect("Pie grammar emitted a title token")
                        .trim()
                        .to_string(),
                );
                cursor.skip_terminators();
                continue;
            }
            "ACC_TITLE" => {
                accessibility_title = cursor
                    .advance()
                    .value
                    .split_once(':')
                    .map(|(_, value)| value.trim().to_string());
                cursor.skip_terminators();
                continue;
            }
            "ACC_DESCR" => {
                accessibility_description = cursor
                    .advance()
                    .value
                    .split_once(':')
                    .map(|(_, value)| value.trim().to_string());
                cursor.skip_terminators();
                continue;
            }
            "ACC_DESCR_BLOCK" => {
                let value = cursor.advance().value.clone();
                let open = value.find('{').expect("accessibility block requires '{'");
                let close = value.rfind('}').expect("accessibility block requires '}'");
                accessibility_description = Some(
                    value[open + 1..close]
                        .lines()
                        .map(str::trim)
                        .filter(|line| !line.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
                cursor.skip_terminators();
                continue;
            }
            _ => {}
        }
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
        if value < 0.0 {
            return Err(token_error(
                &value_token,
                format!(
                    "pie slice {:?} has negative value {value}; values must be non-negative",
                    unquote_mermaid_string(&label_token.value)
                ),
            ));
        }

        slices.push(PieSlice {
            label: unquote_mermaid_string(&label_token.value),
            value,
        });
        cursor.skip_terminators();
    }

    Ok(ChartDiagram {
        title,
        accessibility_title,
        accessibility_description,
        kind: ChartKind::Pie,
        show_data,
        x_axis: None,
        y_axis: None,
        series: vec![],
        slices,
        sankey_nodes: vec![],
        flows: vec![],
        quadrant_labels: [None, None, None, None],
        quadrant_points: vec![],
        quadrant_config: QuadrantConfig::default(),
        xy_config: XyChartConfig::default(),
        orientation: ChartOrientation::Vertical,
    })
}

fn unquote_mermaid_string(raw: &str) -> String {
    let quoted_inner = raw
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(raw);
    let inner = quoted_inner
        .strip_prefix('`')
        .and_then(|value| value.strip_suffix('`'))
        .unwrap_or(quoted_inner);
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

fn split_quadrant_axis_labels(value: &str) -> Vec<&str> {
    let bytes = value.as_bytes();
    for start in 0..bytes.len() {
        if bytes[start] != b'-' {
            continue;
        }
        let mut end = start;
        while end < bytes.len() && bytes[end] == b'-' {
            end += 1;
        }
        if end - start >= 2 && bytes.get(end) == Some(&b'>') {
            return vec![&value[..start], &value[end + 1..]];
        }
    }
    vec![value]
}

fn quadrant_axis_has_dangling_arrow(value: &str) -> bool {
    let trimmed = value.trim_end();
    let Some(arrow) = trimmed.strip_suffix('>') else {
        return false;
    };
    arrow
        .as_bytes()
        .iter()
        .rev()
        .take_while(|byte| **byte == b'-')
        .count()
        >= 2
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
        let source_id = parse_sankey_node_field(&mut cursor)?;
        cursor
            .consume_if("COMMA")
            .ok_or_else(|| token_error(cursor.current(), "expected ',' after Sankey source"))?;
        let target_id = parse_sankey_node_field(&mut cursor)?;
        cursor
            .consume_if("COMMA")
            .ok_or_else(|| token_error(cursor.current(), "expected ',' after Sankey target"))?;
        let weight_token = cursor.current().clone();
        let weight_value = parse_sankey_field(&mut cursor)?;
        let weight = weight_value.parse::<f64>().map_err(|_| {
            token_error(
                &weight_token,
                format!("invalid Sankey flow weight {weight_value:?}"),
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
        accessibility_title: None,
        accessibility_description: None,
        kind: ChartKind::Sankey,
        show_data: false,
        x_axis: None,
        y_axis: None,
        series: vec![],
        slices: vec![],
        sankey_nodes: nodes,
        flows,
        quadrant_labels: [None, None, None, None],
        quadrant_points: vec![],
        quadrant_config: QuadrantConfig::default(),
        xy_config: XyChartConfig::default(),
        orientation: ChartOrientation::Horizontal,
    })
}

fn parse_sankey_field(cursor: &mut TokenCursor) -> Result<String, ParseError> {
    let token = cursor.current().clone();
    if !matches!(token_name(&token), "STRING" | "BARE_FIELD" | "NUMBER") {
        return Err(token_error(&token, "expected Sankey CSV field"));
    }

    Ok(cursor.advance().value.trim().replace("\"\"", "\""))
}

fn parse_sankey_node_field(cursor: &mut TokenCursor) -> Result<String, ParseError> {
    if token_name(cursor.current()) == "COMMA" {
        Ok(String::new())
    } else {
        parse_sankey_field(cursor)
    }
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
    let mut branch_heads = HashMap::from([("main".to_string(), None::<String>)]);
    let mut commit_parents: HashMap<String, Vec<String>> = HashMap::new();
    let mut commit_branches: HashMap<String, String> = HashMap::new();
    let mut merge_commits: HashMap<String, bool> = HashMap::new();
    let mut sequence = 0_usize;
    let mut title = None;
    let mut accessibility_title = None;
    let mut accessibility_description = None;

    while !cursor.at_eof() {
        let command = cursor.current().clone();
        match token_name(&command) {
            "TITLE" => {
                title = Some(
                    cursor
                        .advance()
                        .value
                        .strip_prefix("title")
                        .expect("GitGraph grammar emitted a title token")
                        .trim()
                        .to_string(),
                );
                cursor.skip_terminators();
                continue;
            }
            "ACC_TITLE" => {
                accessibility_title = cursor
                    .advance()
                    .value
                    .split_once(':')
                    .map(|(_, value)| value.trim().to_string());
                cursor.skip_terminators();
                continue;
            }
            "ACC_DESCR" => {
                accessibility_description = cursor
                    .advance()
                    .value
                    .split_once(':')
                    .map(|(_, value)| value.trim().to_string());
                cursor.skip_terminators();
                continue;
            }
            "ACC_DESCR_BLOCK" => {
                let value = cursor.advance().value.clone();
                let open = value.find('{').expect("accessibility block requires '{'");
                let close = value.rfind('}').expect("accessibility block requires '}'");
                accessibility_description = Some(
                    value[open + 1..close]
                        .lines()
                        .map(str::trim)
                        .filter(|line| !line.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
                cursor.skip_terminators();
                continue;
            }
            _ => {}
        }
        match command.value.as_str() {
            "commit" => {
                cursor.advance();
                let mut id = None;
                let mut message = None;
                let mut tags = Vec::new();
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
                            tags.push(parse_gitgraph_string(&mut cursor, "commit tag")?);
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
                let resolved_id = id
                    .clone()
                    .unwrap_or_else(|| format!("{sequence}-generated"));
                sequence += 1;
                let parents = branch_heads
                    .get(&current_branch)
                    .and_then(Clone::clone)
                    .into_iter()
                    .collect::<Vec<_>>();
                commit_parents.insert(resolved_id.clone(), parents.clone());
                commit_branches.insert(resolved_id.clone(), current_branch.clone());
                merge_commits.insert(resolved_id.clone(), false);
                branch_heads.insert(current_branch.clone(), Some(resolved_id.clone()));
                events.push(GitEvent::Commit {
                    id,
                    resolved_id,
                    parents,
                    message,
                    tags,
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
                if branches.iter().any(|candidate| candidate.name == branch) {
                    return Err(token_error(
                        &command,
                        format!("cannot create existing GitGraph branch {branch:?}"),
                    ));
                }
                branches.push(GitBranch {
                    name: branch.clone(),
                    order,
                });
                let head = branch_heads.get(&current_branch).and_then(Clone::clone);
                branch_heads.insert(branch.clone(), head);
                current_branch = branch.clone();
                events.push(GitEvent::Checkout { branch });
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
                let mut tags = Vec::new();
                let mut type_ = GitCommitType::Normal;
                while !gitgraph_statement_ended(&cursor) {
                    match token_name(cursor.current()) {
                        "ID_ATTR" => {
                            cursor.advance();
                            id = Some(parse_gitgraph_string(&mut cursor, "merge id")?);
                        }
                        "TAG_ATTR" => {
                            cursor.advance();
                            tags.push(parse_gitgraph_string(&mut cursor, "merge tag")?);
                        }
                        "TYPE_ATTR" => {
                            cursor.advance();
                            type_ = parse_gitgraph_commit_type(&mut cursor)?;
                        }
                        _ => return Err(token_error(cursor.current(), "invalid merge attribute")),
                    }
                }
                if from == current_branch {
                    return Err(token_error(
                        &command,
                        "cannot merge a GitGraph branch into itself",
                    ));
                }
                let current_head = branch_heads
                    .get(&current_branch)
                    .and_then(Clone::clone)
                    .ok_or_else(|| {
                        token_error(&command, "current GitGraph branch has no commits")
                    })?;
                let from_head = branch_heads
                    .get(&from)
                    .ok_or_else(|| {
                        token_error(&command, format!("unknown GitGraph branch {from:?}"))
                    })?
                    .clone()
                    .ok_or_else(|| {
                        token_error(&command, "merged GitGraph branch has no commits")
                    })?;
                if current_head == from_head {
                    return Err(token_error(
                        &command,
                        "GitGraph branches have the same head",
                    ));
                }
                if id
                    .as_ref()
                    .is_some_and(|id| commit_parents.contains_key(id))
                {
                    return Err(token_error(
                        &command,
                        "GitGraph merge commit id already exists",
                    ));
                }
                let resolved_id = id
                    .clone()
                    .unwrap_or_else(|| format!("{sequence}-generated"));
                sequence += 1;
                let parents = vec![current_head, from_head];
                commit_parents.insert(resolved_id.clone(), parents.clone());
                commit_branches.insert(resolved_id.clone(), current_branch.clone());
                merge_commits.insert(resolved_id.clone(), true);
                branch_heads.insert(current_branch.clone(), Some(resolved_id.clone()));
                events.push(GitEvent::Merge {
                    from,
                    id,
                    resolved_id,
                    parents,
                    tags,
                    type_,
                });
            }
            "cherry-pick" => {
                cursor.advance();
                let mut id = None;
                let mut tags = Vec::new();
                let mut parent = None;
                while !gitgraph_statement_ended(&cursor) {
                    match token_name(cursor.current()) {
                        "ID_ATTR" => {
                            cursor.advance();
                            id = Some(parse_gitgraph_string(&mut cursor, "cherry-pick id")?);
                        }
                        "TAG_ATTR" => {
                            cursor.advance();
                            tags.push(parse_gitgraph_string(&mut cursor, "cherry-pick tag")?);
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
                let source_parents = commit_parents.get(&id).ok_or_else(|| {
                    token_error(
                        &command,
                        "GitGraph cherry-pick source commit does not exist",
                    )
                })?;
                if let Some(parent) = &parent {
                    if !source_parents.contains(parent) {
                        return Err(token_error(
                            &command,
                            "GitGraph cherry-pick parent is not an immediate parent",
                        ));
                    }
                } else if merge_commits.get(&id).copied().unwrap_or(false) {
                    return Err(token_error(
                        &command,
                        "GitGraph merge cherry-pick requires a parent",
                    ));
                }
                if commit_branches.get(&id) == Some(&current_branch) {
                    return Err(token_error(
                        &command,
                        "GitGraph cherry-pick source is already on the current branch",
                    ));
                }
                let current_head = branch_heads
                    .get(&current_branch)
                    .and_then(Clone::clone)
                    .ok_or_else(|| {
                        token_error(&command, "current GitGraph branch has no commits")
                    })?;
                let resolved_id = format!("{sequence}-generated");
                sequence += 1;
                let parents = vec![current_head, id.clone()];
                if tags.is_empty() {
                    tags.push(format!(
                        "cherry-pick:{id}{}",
                        parent
                            .as_ref()
                            .map(|parent| format!("|parent:{parent}"))
                            .unwrap_or_default()
                    ));
                }
                commit_parents.insert(resolved_id.clone(), parents.clone());
                commit_branches.insert(resolved_id.clone(), current_branch.clone());
                merge_commits.insert(resolved_id.clone(), false);
                branch_heads.insert(current_branch.clone(), Some(resolved_id.clone()));
                events.push(GitEvent::CherryPick {
                    id,
                    resolved_id,
                    parents,
                    tags,
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
        title,
        accessibility_title,
        accessibility_description,
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
    if !matches!(
        token_name(&token),
        "REFERENCE" | "PREFIXED_REFERENCE" | "NUMERIC_REFERENCE" | "INT" | "STRING"
    ) {
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
        accessibility_title: None,
        accessibility_description: None,
        direction: None,
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
        metadata: None,
        style: None,
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
        accessibility_title: None,
        accessibility_description: None,
        direction: None,
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
        metadata: None,
        style: None,
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
    parse_mermaid_gantt_ast(source)?;
    let preprocessed = preprocess_mermaid_source(source)?;
    let tokens = try_tokenize_mermaid_gantt(&preprocessed.source).map_err(|message| ParseError {
        message,
        line: 1,
        col: 1,
    })?;
    let mut date_format = GanttDateFormat::default();
    let mut title = None;
    let mut accessibility_title = None;
    let mut accessibility_description = None;
    let mut config = GanttConfig::default();
    let mut sections: Vec<GanttSection> = Vec::new();
    let mut current_section: Option<GanttSection> = None;
    let mut previous_task_id = None;
    let mut generated_task_count = 0;

    for token in tokens {
        match token.type_name.as_deref() {
            Some("TITLE_STATEMENT") => {
                title = Some(normalize_gantt_prefixed_label(
                    token.value["title".len()..].trim(),
                ));
            }
            Some("ACC_TITLE_STATEMENT") => {
                accessibility_title = gantt_metadata_value(&token);
            }
            Some("ACC_DESCR_STATEMENT") => {
                accessibility_description = gantt_metadata_value(&token);
            }
            Some("ACC_DESCR_BLOCK") => {
                let open = token.value.find('{').expect("grammar requires '{'");
                let close = token.value.rfind('}').expect("grammar requires '}'");
                accessibility_description = Some(token.value[open + 1..close].trim().to_string());
            }
            Some("DATE_FORMAT_STATEMENT") => {
                date_format = parse_gantt_date_format(&token, token.value["dateFormat".len()..].trim())?;
            }
            Some("AXIS_FORMAT_STATEMENT") => {
                config.axis_format = Some(gantt_statement_value(&token, "axisFormat"));
            }
            Some("TICK_INTERVAL_STATEMENT") => {
                config.tick_interval = Some(gantt_statement_value(&token, "tickInterval"));
            }
            Some("INCLUDES_STATEMENT") => {
                merge_gantt_calendar_tokens(&mut config.includes, &gantt_statement_value(&token, "includes"));
            }
            Some("EXCLUDES_STATEMENT") => {
                merge_gantt_calendar_tokens(&mut config.excludes, &gantt_statement_value(&token, "excludes"));
            }
            Some("INCLUSIVE_END_DATES") => config.inclusive_end_dates = true,
            Some("TOP_AXIS") => config.top_axis = true,
            Some("TODAY_MARKER_STATEMENT") => {
                config.today_marker = Some(gantt_statement_value(&token, "todayMarker"));
            }
            Some("WEEKDAY_STATEMENT") => {
                config.weekday = Some(gantt_statement_value(&token, "weekday").to_ascii_lowercase());
            }
            Some("WEEKEND_STATEMENT") => {
                config.weekend = Some(gantt_statement_value(&token, "weekend").to_ascii_lowercase());
            }
            Some("SECTION_STATEMENT") => {
                if let Some(section) = current_section.take() {
                    sections.push(section);
                }
                current_section = Some(GanttSection {
                    label: Some(normalize_gantt_prefixed_label(
                        token.value["section".len()..].trim(),
                    )),
                    tasks: vec![],
                });
            }
            Some("TASK_STATEMENT") => {
                let task = parse_gantt_task(
                    &token,
                    previous_task_id.as_deref(),
                    &mut generated_task_count,
                    &date_format,
                )?;
                previous_task_id = Some(task.id.clone());
                let sec = current_section.get_or_insert_with(|| GanttSection {
                    label: None,
                    tasks: vec![],
                });
                sec.tasks.push(task);
            }
            Some("CLICK_STATEMENT") => {
                apply_gantt_interaction(&token, &mut sections, current_section.as_mut())?;
            }
            _ => {}
        }
    }
    if let Some(sec) = current_section {
        sections.push(sec);
    }
    validate_gantt_dependencies(&sections)?;

    Ok(GanttDiagram {
        title,
        accessibility_title,
        accessibility_description,
        date_format,
        config,
        sections,
    })
}

fn parse_gantt_date_format(token: &Token, source: &str) -> Result<GanttDateFormat, ParseError> {
    if source.is_empty() { return Err(token_error(token, "Gantt dateFormat cannot be empty")); }
    if source == "yyyy-mm-dd" {
        return Ok(GanttDateFormat { source: source.into(), ..GanttDateFormat::default() });
    }
    let mut parts = Vec::new();
    let mut index = 0;
    while index < source.len() {
        let remaining = &source[index..];
        if let Some(literal) = remaining.strip_prefix('[') {
            let close = literal.find(']').ok_or_else(|| token_error(token, "unterminated Gantt dateFormat literal"))?;
            parts.push(GanttDateFormatPart::Literal(literal[..close].to_string()));
            index += close + 2;
            continue;
        }
        let known = [
            ("YYYY", GanttDateFormatPart::Year4), ("MMMM", GanttDateFormatPart::MonthLong),
            ("MMM", GanttDateFormatPart::MonthShort), ("SSS", GanttDateFormatPart::Millisecond),
            ("ZZ", GanttDateFormatPart::TimezoneOffsetCompact),
            ("YY", GanttDateFormatPart::Year2), ("MM", GanttDateFormatPart::Month2),
            ("DD", GanttDateFormatPart::Day2), ("HH", GanttDateFormatPart::Hour24),
            ("mm", GanttDateFormatPart::Minute), ("ss", GanttDateFormatPart::Second),
            ("M", GanttDateFormatPart::Month), ("D", GanttDateFormatPart::Day),
            ("Z", GanttDateFormatPart::TimezoneOffsetColon),
            ("X", GanttDateFormatPart::UnixSeconds), ("x", GanttDateFormatPart::UnixMilliseconds),
        ];
        if let Some((name, part)) = known.into_iter().find(|(name, _)| remaining.starts_with(name)) {
            parts.push(part);
            index += name.len();
            continue;
        }
        let character = remaining.chars().next().expect("index is within source");
        if character.is_ascii_alphabetic() {
            return Err(token_error(token, format!("unsupported Gantt dateFormat token starting at {remaining:?}")));
        }
        match parts.last_mut() {
            Some(GanttDateFormatPart::Literal(literal)) => literal.push(character),
            _ => parts.push(GanttDateFormatPart::Literal(character.to_string())),
        }
        index += character.len_utf8();
    }
    let timestamp_parts = parts.iter().filter(|part| matches!(part,
        GanttDateFormatPart::UnixSeconds | GanttDateFormatPart::UnixMilliseconds)).count();
    if timestamp_parts > 0 && parts.len() != 1 {
        return Err(token_error(token, "Gantt Unix timestamp formats cannot be combined"));
    }
    Ok(GanttDateFormat { source: source.to_string(), parts })
}

fn gantt_date_matches_format(value: &str, format: &GanttDateFormat) -> bool {
    let mut rest = value;
    let seconds_only = format.parts.as_slice() == [GanttDateFormatPart::Second];
    for part in &format.parts {
        let consumed = match part {
            GanttDateFormatPart::Literal(literal) => {
                let Some(next) = rest.strip_prefix(literal) else { return false };
                rest = next;
                continue;
            }
            GanttDateFormatPart::Year4 => consume_digits(rest, 4, 4),
            GanttDateFormatPart::Year2 => consume_digits(rest, 2, 2),
            GanttDateFormatPart::Month | GanttDateFormatPart::Day => consume_digits(rest, 1, 2),
            GanttDateFormatPart::Month2 | GanttDateFormatPart::Day2 | GanttDateFormatPart::Hour24
                | GanttDateFormatPart::Minute => consume_digits(rest, 2, 2),
            GanttDateFormatPart::Second if seconds_only => consume_digits(rest, 1, 2),
            GanttDateFormatPart::Second => consume_digits(rest, 2, 2),
            GanttDateFormatPart::Millisecond => consume_digits(rest, 3, 3),
            GanttDateFormatPart::MonthShort => consume_letters(rest, 3, 3),
            GanttDateFormatPart::MonthLong => consume_letters(rest, 3, 9),
            GanttDateFormatPart::TimezoneOffsetColon => consume_timezone_offset(rest, true),
            GanttDateFormatPart::TimezoneOffsetCompact => consume_timezone_offset(rest, false),
            GanttDateFormatPart::UnixSeconds | GanttDateFormatPart::UnixMilliseconds => consume_signed_digits(rest),
        };
        let Some(length) = consumed else { return false };
        rest = &rest[length..];
    }
    rest.is_empty()
}

fn consume_timezone_offset(value: &str, colon: bool) -> Option<usize> {
    if value.starts_with('Z') { return Some(1); }
    let bytes = value.as_bytes();
    if !matches!(bytes.first(), Some(b'+') | Some(b'-')) { return None; }
    let expected = if colon { 6 } else { 5 };
    if bytes.len() < expected { return None; }
    if colon && bytes.get(3) != Some(&b':') { return None; }
    let hour = &value[1..3];
    let minute = if colon { &value[4..6] } else { &value[3..5] };
    (hour.bytes().all(|byte| byte.is_ascii_digit())
        && minute.bytes().all(|byte| byte.is_ascii_digit()))
        .then_some(expected)
}

fn consume_digits(value: &str, minimum: usize, maximum: usize) -> Option<usize> {
    let count = value.bytes().take(maximum).take_while(u8::is_ascii_digit).count();
    (count >= minimum).then_some(count)
}

fn consume_letters(value: &str, minimum: usize, maximum: usize) -> Option<usize> {
    let count = value.bytes().take(maximum).take_while(u8::is_ascii_alphabetic).count();
    (count >= minimum).then_some(count)
}

fn consume_signed_digits(value: &str) -> Option<usize> {
    let sign = usize::from(value.starts_with(['+', '-']));
    let digits = value[sign..].bytes().take_while(u8::is_ascii_digit).count();
    (digits > 0).then_some(sign + digits)
}

fn validate_gantt_dependencies(sections: &[GanttSection]) -> Result<(), ParseError> {
    let tasks = sections.iter().flat_map(|section| section.tasks.iter()).collect::<Vec<_>>();
    let ids = tasks.iter().map(|task| task.id.as_str()).collect::<HashSet<_>>();
    if ids.len() != tasks.len() {
        return Err(ParseError { message: "duplicate Gantt task id".into(), line: 1, col: 1 });
    }
    for task in &tasks {
        for dependency in &task.dependencies {
            if !ids.contains(dependency.as_str()) {
                return Err(ParseError {
                    message: format!("unknown Gantt task id {dependency:?}"),
                    line: 1,
                    col: 1,
                });
            }
        }
        if let Some(TaskEnd::Until(dependencies)) = &task.end {
            for dependency in dependencies {
                if !ids.contains(dependency.as_str()) {
                    return Err(ParseError {
                        message: format!("unknown Gantt task id {dependency:?}"),
                        line: 1,
                        col: 1,
                    });
                }
            }
        }
    }

    let starts = tasks.iter().map(|task| (task.id.as_str(), &task.start)).collect::<HashMap<_, _>>();
    fn visit<'a>(
        id: &'a str,
        starts: &HashMap<&'a str, &'a TaskStart>,
        visiting: &mut HashSet<&'a str>,
        visited: &mut HashSet<&'a str>,
    ) -> bool {
        if visited.contains(id) { return false; }
        if !visiting.insert(id) { return true; }
        if let Some(TaskStart::After(dependencies)) = starts.get(id) {
            if dependencies.iter().any(|dependency| visit(dependency, starts, visiting, visited)) {
                return true;
            }
        }
        visiting.remove(id);
        visited.insert(id);
        false
    }
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    if starts.keys().any(|id| visit(id, &starts, &mut visiting, &mut visited)) {
        return Err(ParseError { message: "cyclic Gantt after dependency".into(), line: 1, col: 1 });
    }
    Ok(())
}

fn gantt_statement_value(token: &Token, keyword: &str) -> String {
    token.value[keyword.len()..].trim().to_string()
}

fn merge_gantt_calendar_tokens(target: &mut Vec<String>, source: &str) {
    for value in source
        .to_ascii_lowercase()
        .split(|character: char| character.is_whitespace() || character == ',')
        .filter(|value| !value.is_empty())
    {
        if !target.iter().any(|existing| existing == value) {
            target.push(value.to_string());
        }
    }
}

fn gantt_metadata_value(token: &Token) -> Option<String> {
    token
        .value
        .split_once(':')
        .map(|(_, value)| value.trim().to_string())
}

fn normalize_gantt_prefixed_label(value: &str) -> String {
    normalize_mermaid_line_breaks(
        value
            .trim_start_matches([';', '#'])
            .trim_start(),
    )
}

fn apply_gantt_interaction(
    token: &Token,
    sections: &mut [GanttSection],
    current_section: Option<&mut GanttSection>,
) -> Result<(), ParseError> {
    let rest = token.value["click".len()..].trim();
    let (task_id, commands) = rest
        .split_once(char::is_whitespace)
        .ok_or_else(|| token_error(token, "Gantt click requires href or call"))?;
    let lowercase = commands.to_ascii_lowercase();

    let link = find_gantt_command(&lowercase, "href")
        .map(|index| {
            let value = commands[index + "href".len()..].trim_start();
            let quoted = value
                .strip_prefix('"')
                .ok_or_else(|| token_error(token, "Gantt href requires a quoted URL"))?;
            let close = quoted
                .find('"')
                .ok_or_else(|| token_error(token, "unterminated Gantt href URL"))?;
            Ok(quoted[..close].to_string())
        })
        .transpose()?;

    let callback = find_gantt_command(&lowercase, "call")
        .map(|index| {
            let value = commands[index + "call".len()..].trim_start();
            let open = value
                .find('(')
                .ok_or_else(|| token_error(token, "Gantt callback requires parentheses"))?;
            let close = value[open + 1..]
                .find(')')
                .map(|index| open + 1 + index)
                .ok_or_else(|| token_error(token, "unterminated Gantt callback arguments"))?;
            let name = value[..open].trim();
            if name.is_empty() {
                return Err(token_error(token, "Gantt callback name cannot be empty"));
            }
            let args = value[open + 1..close].trim();
            Ok((
                name.to_string(),
                (!args.is_empty()).then(|| args.to_string()),
            ))
        })
        .transpose()?;

    if link.is_none() && callback.is_none() {
        return Err(token_error(token, "Gantt click requires href or call"));
    }
    let task = current_section
        .into_iter()
        .chain(sections.iter_mut())
        .flat_map(|section| section.tasks.iter_mut())
        .find(|task| task.id == task_id)
        .ok_or_else(|| token_error(token, format!("unknown Gantt task id {task_id:?}")))?;
    task.link = link;
    if let Some((name, args)) = callback {
        task.callback = Some(name);
        task.callback_args = args;
    }
    Ok(())
}

fn find_gantt_command(source: &str, command: &str) -> Option<usize> {
    source.match_indices(command).find_map(|(index, _)| {
        let before = source[..index].chars().next_back();
        let after = source[index + command.len()..].chars().next();
        (before.is_none_or(char::is_whitespace) && after.is_some_and(char::is_whitespace))
            .then_some(index)
    })
}

/// Parse a single Gantt task line.
///
/// Formats include explicit IDs (`id, start, end`), generated IDs
/// (`start, end`), and sequential tasks (`end`) that follow the prior task.
fn parse_gantt_task(
    token: &Token,
    previous_task_id: Option<&str>,
    generated_task_count: &mut usize,
    date_format: &GanttDateFormat,
) -> Result<GanttTask, ParseError> {
    let colon = token
        .value
        .find(':')
        .expect("task statement grammar requires ':'");
    let label = normalize_gantt_prefixed_label(token.value[..colon].trim());
    let rest = token.value[colon + 1..].trim();
    if label.is_empty() {
        return Err(token_error(token, "Gantt task label cannot be empty"));
    }

    let parts: Vec<&str> = rest.split(',').map(str::trim).collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(token_error(token, "Gantt task data cannot be empty"));
    }

    let mut tags = GanttTaskTags::default();
    let mut first_data = 0;
    while first_data < parts.len() && apply_gantt_task_tag(parts[first_data], &mut tags) {
        first_data += 1;
    }
    let remaining = &parts[first_data..];

    if remaining.iter().any(|part| part.is_empty()) {
        return Err(token_error(token, "Gantt task data cannot be empty"));
    }
    let generated_id = |counter: &mut usize| {
        *counter += 1;
        format!("task{counter}")
    };
    let (id, start_data, end_data) = match remaining {
        [end] => {
            let previous = previous_task_id.ok_or_else(|| {
                token_error(token, "sequential Gantt task requires a previous task")
            })?;
            (generated_id(generated_task_count), format!("after {previous}"), *end)
        }
        [start, end] => (generated_id(generated_task_count), (*start).to_string(), *end),
        [id, start, end] => ((*id).to_string(), (*start).to_string(), *end),
        _ => return Err(token_error(token, "invalid Gantt task field count")),
    };
    let start = parse_gantt_task_start(token, &start_data, date_format)?;
    let (duration, end) = parse_gantt_task_end(token, end_data, date_format)?;

    Ok(GanttTask {
        id,
        label,
        dependencies: match &start {
            TaskStart::After(ids) => ids.clone(),
            TaskStart::Date(_) => Vec::new(),
        },
        start,
        duration,
        end,
        tags,
        link: None,
        callback: None,
        callback_args: None,
    })
}

fn apply_gantt_task_tag(value: &str, tags: &mut GanttTaskTags) -> bool {
    match value.to_ascii_lowercase().as_str() {
        "active" => tags.active = true,
        "done" => tags.done = true,
        "crit" => tags.critical = true,
        "milestone" => tags.milestone = true,
        "vert" => tags.vertical = true,
        _ => return false,
    }
    true
}

fn parse_gantt_task_start(token: &Token, value: &str, date_format: &GanttDateFormat) -> Result<TaskStart, ParseError> {
    if let Some(ids) = value.strip_prefix("after ") {
        let dependencies = gantt_dependency_ids(ids);
        if dependencies.is_empty() {
            return Err(token_error(token, "Gantt after requires at least one task id"));
        }
        Ok(TaskStart::After(dependencies))
    } else if gantt_date_matches_format(value, date_format) {
        Ok(TaskStart::Date(value.to_string()))
    } else {
        Err(token_error(token, "Gantt task start does not match dateFormat"))
    }
}

fn parse_gantt_task_end(
    token: &Token,
    value: &str,
    date_format: &GanttDateFormat,
) -> Result<(GanttDuration, Option<TaskEnd>), ParseError> {
    if let Some(ids) = value.strip_prefix("until ") {
        let dependencies = gantt_dependency_ids(ids);
        if dependencies.is_empty() {
            return Err(token_error(token, "Gantt until requires at least one task id"));
        }
        Ok((GanttDuration::default(), Some(TaskEnd::Until(dependencies))))
    } else if gantt_date_matches_format(value, date_format) {
        Ok((GanttDuration::default(), Some(TaskEnd::Date(value.to_string()))))
    } else if let Some(duration) = parse_duration(value).filter(|duration| duration.value >= 0.0) {
        Ok((duration, None))
    } else {
        Err(token_error(token, "invalid Gantt task duration or end date"))
    }
}

fn gantt_dependency_ids(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_duration(s: &str) -> Option<GanttDuration> {
    let s = s.trim();
    let units = [("ms", GanttDurationUnit::Milliseconds), ("s", GanttDurationUnit::Seconds),
        ("m", GanttDurationUnit::Minutes), ("h", GanttDurationUnit::Hours),
        ("d", GanttDurationUnit::Days), ("w", GanttDurationUnit::Weeks)];
    let (number, unit) = units.into_iter().find_map(|(suffix, unit)|
        s.strip_suffix(suffix).map(|number| (number, unit)))
        .unwrap_or((s, GanttDurationUnit::Days));
    Some(GanttDuration { value: number.parse().ok()?, unit })
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
        assert_eq!(
            bar.data.iter().map(|point| point.value).collect::<Vec<_>>(),
            vec![40.0, 60.0, 45.0]
        );
    }

    #[test]
    fn xychart_grammar_preserves_orientation_axis_titles_and_series_names() {
        let diagram = parse_xychart(
            "xychart horizontal\n\
             x-axis \"Quarter\" [Q1, Q2]\n\
             y-axis Revenue -10 --> 50\n\
             bar \"Forecast\" [+12, -4]\n",
        )
        .unwrap();
        assert_eq!(diagram.orientation, ChartOrientation::Horizontal);
        assert_eq!(
            diagram.x_axis.as_ref().unwrap().title.as_deref(),
            Some("Quarter")
        );
        let y_axis = diagram.y_axis.as_ref().unwrap();
        assert_eq!(y_axis.title.as_deref(), Some("Revenue"));
        assert_eq!((y_axis.min, y_axis.max), (-10.0, 50.0));
        assert_eq!(diagram.series[0].label.as_deref(), Some("Forecast"));
        assert_eq!(
            diagram.series[0]
                .data
                .iter()
                .map(|point| point.value)
                .collect::<Vec<_>>(),
            [12.0, -4.0]
        );
    }

    #[test]
    fn xychart_grammar_preserves_accessibility_metadata() {
        let diagram = parse_xychart(
            "xychart\naccTitle: Quarterly revenue\naccDescr {\n  Forecast and actuals\n}\nline [1, 2]\n",
        )
        .unwrap();
        assert_eq!(
            diagram.accessibility_title.as_deref(),
            Some("Quarterly revenue")
        );
        assert_eq!(
            diagram.accessibility_description.as_deref(),
            Some("Forecast and actuals")
        );
    }

    #[test]
    fn xychart_preserves_mixed_point_labels() {
        let diagram = parse_xychart(
            "xychart\nx-axis [A, B, C]\nline \"Models\" [3.8 \"Phi-3\", 7, 540 \"PaLM, 2\"]\nbar [1 \"One\", 2, 3 \"Three\"]\n",
        )
        .unwrap();
        assert_eq!(diagram.series[0].data[0].value, 3.8);
        assert_eq!(diagram.series[0].data[0].label.as_deref(), Some("Phi-3"));
        assert_eq!(diagram.series[0].data[1].label, None);
        assert_eq!(diagram.series[0].data[2].label.as_deref(), Some("PaLM, 2"));
        assert_eq!(diagram.series[1].data[2].label.as_deref(), Some("Three"));
    }

    #[test]
    fn xychart_infers_numeric_x_positions_and_truncates_categorical_data() {
        let inferred = parse_xychart("xychart\nx-axis Samples\nline [10, 20, 30]\n").unwrap();
        let axis = inferred.x_axis.unwrap();
        assert_eq!(axis.kind, AxisKind::Numeric);
        assert_eq!((axis.min, axis.max), (1.0, 3.0));
        assert_eq!(axis.title.as_deref(), Some("Samples"));

        let categorical =
            parse_xychart("xychart\nx-axis [A, B]\nbar [1, 2, 99]\nline [3, 4, 88]\n").unwrap();
        assert!(categorical
            .series
            .iter()
            .all(|series| series.data.len() == 2));
    }

    #[test]
    fn xychart_preserves_core_init_configuration() {
        let diagram = parse_xychart(
            "%%{init: {\"xyChart\": {\"width\": 720, \"height\": 440, \"chartOrientation\": \"horizontal\", \"plotReservedSpacePercent\": 65, \"titleFontSize\": 24, \"titlePadding\": 14, \"showTitle\": false, \"showLegend\": false, \"legendFontSize\": 18, \"legendPadding\": 16, \"showDataLabel\": true, \"showDataLabelOutsideBar\": true, \"xAxis\": {\"showLabel\": false, \"labelFontSize\": 13, \"labelPadding\": 7, \"labelRotation\": -45, \"showTitle\": false, \"titleFontSize\": 18, \"titlePadding\": 9, \"showTick\": false, \"tickLength\": 11, \"tickWidth\": 3, \"showAxisLine\": false, \"axisLineWidth\": 4}, \"yAxis\": {\"showLabel\": true, \"labelFontSize\": 15, \"labelPadding\": 8, \"labelRotation\": 30, \"showTitle\": true, \"titleFontSize\": 19, \"titlePadding\": 10, \"showTick\": true, \"tickLength\": 12, \"tickWidth\": 4, \"showAxisLine\": true, \"axisLineWidth\": 5}}, \"themeVariables\": {\"xyChart\": {\"dataLabelColor\": \"#123456\"}}}}%%\nxychart\ntitle Hidden\nbar [10, 20]\n",
        )
        .unwrap();

        assert_eq!(diagram.xy_config.width, Some(720.0));
        assert_eq!(diagram.xy_config.height, Some(440.0));
        assert_eq!(
            diagram.xy_config.chart_orientation,
            Some(ChartOrientation::Horizontal)
        );
        assert_eq!(diagram.orientation, ChartOrientation::Horizontal);
        assert_eq!(diagram.xy_config.plot_reserved_space_percent, Some(65.0));
        assert_eq!(diagram.xy_config.title_font_size, Some(24.0));
        assert_eq!(diagram.xy_config.title_padding, Some(14.0));
        assert_eq!(diagram.xy_config.show_title, Some(false));
        assert_eq!(diagram.xy_config.show_legend, Some(false));
        assert_eq!(diagram.xy_config.legend_font_size, Some(18.0));
        assert_eq!(diagram.xy_config.legend_padding, Some(16.0));
        assert_eq!(diagram.xy_config.show_data_label, Some(true));
        assert_eq!(diagram.xy_config.show_data_label_outside_bar, Some(true));
        assert_eq!(
            diagram.xy_config.data_label_color.as_deref(),
            Some("#123456")
        );
        assert_eq!(diagram.xy_config.x_axis.show_label, Some(false));
        assert_eq!(diagram.xy_config.x_axis.label_font_size, Some(13.0));
        assert_eq!(diagram.xy_config.x_axis.label_padding, Some(7.0));
        assert_eq!(diagram.xy_config.x_axis.label_rotation, Some(-45.0));
        assert_eq!(diagram.xy_config.x_axis.show_title, Some(false));
        assert_eq!(diagram.xy_config.x_axis.title_font_size, Some(18.0));
        assert_eq!(diagram.xy_config.x_axis.title_padding, Some(9.0));
        assert_eq!(diagram.xy_config.x_axis.show_tick, Some(false));
        assert_eq!(diagram.xy_config.x_axis.tick_length, Some(11.0));
        assert_eq!(diagram.xy_config.x_axis.tick_width, Some(3.0));
        assert_eq!(diagram.xy_config.x_axis.show_axis_line, Some(false));
        assert_eq!(diagram.xy_config.x_axis.axis_line_width, Some(4.0));
        assert_eq!(diagram.xy_config.y_axis.show_label, Some(true));
        assert_eq!(diagram.xy_config.y_axis.label_font_size, Some(15.0));
        assert_eq!(diagram.xy_config.y_axis.label_rotation, Some(30.0));
        assert_eq!(diagram.xy_config.y_axis.show_tick, Some(true));
        assert_eq!(diagram.xy_config.y_axis.tick_length, Some(12.0));
        assert_eq!(diagram.xy_config.y_axis.tick_width, Some(4.0));
        assert_eq!(diagram.xy_config.y_axis.axis_line_width, Some(5.0));

        let explicit = parse_xychart(
            "%%{init: {\"xyChart\": {\"chartOrientation\": \"horizontal\"}}}%%\nxychart vertical\nline [1, 2]\n",
        )
        .unwrap();
        assert_eq!(explicit.orientation, ChartOrientation::Vertical);

        let out_of_range = parse_xychart(
            "%%{init: {\"xyChart\": {\"xAxis\": {\"labelRotation\": 91}}}}%%\nxychart\nline [1, 2]\n",
        )
        .unwrap();
        assert_eq!(out_of_range.xy_config.x_axis.label_rotation, None);
    }

    #[test]
    fn xychart_preserves_axis_theme_colors() {
        let diagram = parse_xychart(
            "%%{init: {\"themeVariables\": {\"xyChart\": {\"backgroundColor\": \"#010203\", \"titleColor\": \"#040506\", \"plotColorPalette\": \"#070809, #0a0b0c\", \"xAxisLabelColor\": \"#110001\", \"xAxisTitleColor\": \"#220002\", \"xAxisTickColor\": \"#330003\", \"xAxisLineColor\": \"#440004\", \"yAxisLabelColor\": \"#550005\", \"yAxisTitleColor\": \"#660006\", \"yAxisTickColor\": \"#770007\", \"yAxisLineColor\": \"#880008\"}}}}%%\nxychart\nx-axis Quarter [Q1, Q2]\ny-axis Revenue 0 --> 20\nbar [10, 20]\n",
        )
        .unwrap();

        assert_eq!(
            diagram.xy_config.background_color.as_deref(),
            Some("#010203")
        );
        assert_eq!(diagram.xy_config.title_color.as_deref(), Some("#040506"));
        assert_eq!(
            diagram.xy_config.plot_color_palette,
            Some(vec!["#070809".into(), "#0a0b0c".into()])
        );
        assert_eq!(
            diagram.xy_config.x_axis.label_color.as_deref(),
            Some("#110001")
        );
        assert_eq!(
            diagram.xy_config.x_axis.title_color.as_deref(),
            Some("#220002")
        );
        assert_eq!(
            diagram.xy_config.x_axis.tick_color.as_deref(),
            Some("#330003")
        );
        assert_eq!(
            diagram.xy_config.x_axis.axis_line_color.as_deref(),
            Some("#440004")
        );
        assert_eq!(
            diagram.xy_config.y_axis.label_color.as_deref(),
            Some("#550005")
        );
        assert_eq!(
            diagram.xy_config.y_axis.title_color.as_deref(),
            Some("#660006")
        );
        assert_eq!(
            diagram.xy_config.y_axis.tick_color.as_deref(),
            Some("#770007")
        );
        assert_eq!(
            diagram.xy_config.y_axis.axis_line_color.as_deref(),
            Some("#880008")
        );
    }

    #[test]
    fn xychart_grammar_rejects_categorical_y_axis_and_invalid_data() {
        assert!(parse_xychart("xychart\ny-axis [low, high]\nline [1, 2]\n").is_err());
        assert!(parse_xychart("xychart\nline [1, nope]\n").is_err());
    }

    #[test]
    fn quadrant_parses_labels_and_normalized_points() {
        let diagram = parse_quadrant_chart(
            "quadrantChart\n\
             title Native portfolio\n\
             x-axis Low reach --> High reach\n\
             y-axis Low impact --> High impact\n\
             quadrant-1 Invest\n\
             quadrant-2 Explore\n\
             quadrant-3 Retire\n\
             quadrant-4 Maintain\n\
             Metal: [0.75, 0.80]\n\
             Direct2D: [0.35, 0.45]\n",
        )
        .unwrap();

        assert_eq!(diagram.kind, ChartKind::Quadrant);
        assert_eq!(diagram.title.as_deref(), Some("Native portfolio"));
        assert_eq!(diagram.quadrant_labels[0].as_deref(), Some("Invest"));
        assert_eq!(
            diagram.x_axis.unwrap().categories,
            ["Low reach", "High reach"]
        );
        assert_eq!(diagram.quadrant_points.len(), 2);
        assert_eq!(diagram.quadrant_points[0].label, "Metal");
        assert_eq!(diagram.quadrant_points[0].x, 0.75);
        assert_eq!(diagram.quadrant_points[0].y, 0.8);
    }

    #[test]
    fn quadrant_parses_accessibility_metadata() {
        let diagram = parse_quadrant_chart(
            "quadrantChart\naccTitle: Portfolio matrix\naccDescr {\nNative renderer priorities\nacross backends\n}\nMetal: [0.75, 0.8]\n",
        )
        .unwrap();

        assert_eq!(
            diagram.accessibility_title.as_deref(),
            Some("Portfolio matrix")
        );
        assert_eq!(
            diagram.accessibility_description.as_deref(),
            Some("Native renderer priorities\nacross backends")
        );
    }

    #[test]
    fn quadrant_parses_case_insensitive_keywords_unicode_and_markdown_text() {
        let source = "QuAdRaNtChArT\n\
            TiTlE Native portfolio\n\
            X-AxIs \"Low reach 📉\" ---> \"`High reach Ω`\"\n\
            QuAdRaNt-1 \"`Invest 🚀`\"\n\
            \"`Métal 渲染`\": [0.75, 0.80]\n";
        let diagram_type = detect_mermaid_type(source).unwrap();
        assert_eq!(diagram_type, MermaidDiagramType::Quadrant);

        let diagram = parse_quadrant_chart(source).unwrap();
        assert_eq!(diagram.title.as_deref(), Some("Native portfolio"));
        assert_eq!(
            diagram.x_axis.unwrap().categories,
            ["Low reach 📉", "High reach Ω"]
        );
        assert_eq!(diagram.quadrant_labels[0].as_deref(), Some("Invest 🚀"));
        assert_eq!(diagram.quadrant_points[0].label, "Métal 渲染");
    }

    #[test]
    fn quadrant_parses_comments_empty_charts_and_one_sided_axes() {
        let empty = parse_quadrant_chart("%% comment\nquadrantChart").unwrap();
        assert!(empty.quadrant_points.is_empty());

        let diagram = parse_quadrant_chart(
            "quadrantChart\n\
             x-axis \"Urgent(* +=[❤\" --> %% preserve arrow\n\
             y-axis Engagement %% one label\n\
             quadrant-1 Plan %% label comment\n\
             \"Point1 : (* +=[❤\": [1, 0] %% point comment\n",
        )
        .unwrap();

        assert_eq!(diagram.x_axis.unwrap().categories, ["Urgent(* +=[❤ ⟶ "]);
        assert_eq!(diagram.y_axis.unwrap().categories, ["Engagement"]);
        assert_eq!(diagram.quadrant_labels[0].as_deref(), Some("Plan"));
        assert_eq!(diagram.quadrant_points[0].label, "Point1 : (* +=[❤");
    }

    #[test]
    fn quadrant_parses_layout_init_config() {
        let diagram = parse_quadrant_chart(
            "%%{init: {\"quadrantChart\": {\"chartWidth\": 720, \"chartHeight\": 540, \"xAxisPosition\": \"top\", \"yAxisPosition\": \"right\", \"pointRadius\": 11, \"quadrantPadding\": 18, \"quadrantInternalBorderStrokeWidth\": 3, \"quadrantExternalBorderStrokeWidth\": 5, \"titleFontSize\": 22, \"titlePadding\": 12, \"xAxisLabelFontSize\": 15, \"xAxisLabelPadding\": 21, \"yAxisLabelFontSize\": 16, \"yAxisLabelPadding\": 23, \"quadrantLabelFontSize\": 17, \"quadrantTextTopPadding\": 19, \"pointLabelFontSize\": 14, \"pointTextPadding\": 9}, \"themeVariables\": {\"quadrant1Fill\": \"#111111\", \"quadrant2Fill\": \"#222222\", \"quadrant3Fill\": \"#333333\", \"quadrant4Fill\": \"#444444\", \"quadrant1TextFill\": \"#aaaaaa\", \"quadrant2TextFill\": \"#bbbbbb\", \"quadrant3TextFill\": \"#cccccc\", \"quadrant4TextFill\": \"#dddddd\", \"quadrantPointFill\": \"#123456\", \"quadrantPointTextFill\": \"#234567\", \"quadrantXAxisTextFill\": \"#345678\", \"quadrantYAxisTextFill\": \"#456789\", \"quadrantInternalBorderStrokeFill\": \"#56789a\", \"quadrantExternalBorderStrokeFill\": \"#6789ab\", \"quadrantTitleFill\": \"#789abc\"}}}%%\nquadrantChart\nMetal: [0.75, 0.8]\n",
        )
        .unwrap();

        assert_eq!(diagram.quadrant_config.chart_width, Some(720.0));
        assert_eq!(diagram.quadrant_config.chart_height, Some(540.0));
        assert_eq!(
            diagram.quadrant_config.x_axis_position.as_deref(),
            Some("top")
        );
        assert_eq!(
            diagram.quadrant_config.y_axis_position.as_deref(),
            Some("right")
        );
        assert_eq!(diagram.quadrant_config.point_radius, Some(11.0));
        assert_eq!(diagram.quadrant_config.quadrant_padding, Some(18.0));
        assert_eq!(diagram.quadrant_config.internal_border_width, Some(3.0));
        assert_eq!(diagram.quadrant_config.external_border_width, Some(5.0));
        assert_eq!(diagram.quadrant_config.title_font_size, Some(22.0));
        assert_eq!(diagram.quadrant_config.title_padding, Some(12.0));
        assert_eq!(diagram.quadrant_config.x_axis_label_font_size, Some(15.0));
        assert_eq!(diagram.quadrant_config.x_axis_label_padding, Some(21.0));
        assert_eq!(diagram.quadrant_config.y_axis_label_font_size, Some(16.0));
        assert_eq!(diagram.quadrant_config.y_axis_label_padding, Some(23.0));
        assert_eq!(diagram.quadrant_config.quadrant_label_font_size, Some(17.0));
        assert_eq!(
            diagram.quadrant_config.quadrant_text_top_padding,
            Some(19.0)
        );
        assert_eq!(diagram.quadrant_config.point_label_font_size, Some(14.0));
        assert_eq!(diagram.quadrant_config.point_text_padding, Some(9.0));
        assert_eq!(
            diagram.quadrant_config.quadrant_fills,
            ["#111111", "#222222", "#333333", "#444444"].map(|value| Some(value.into()))
        );
        assert_eq!(
            diagram.quadrant_config.quadrant_text_fills,
            ["#aaaaaa", "#bbbbbb", "#cccccc", "#dddddd"].map(|value| Some(value.into()))
        );
        assert_eq!(
            diagram.quadrant_config.point_fill.as_deref(),
            Some("#123456")
        );
        assert_eq!(
            diagram.quadrant_config.point_text_fill.as_deref(),
            Some("#234567")
        );
        assert_eq!(
            diagram.quadrant_config.x_axis_text_fill.as_deref(),
            Some("#345678")
        );
        assert_eq!(
            diagram.quadrant_config.y_axis_text_fill.as_deref(),
            Some("#456789")
        );
        assert_eq!(
            diagram.quadrant_config.title_fill.as_deref(),
            Some("#789abc")
        );
    }

    #[test]
    fn quadrant_resolves_point_classes_and_inline_style_precedence() {
        let diagram = parse_quadrant_chart(
            "quadrantChart\n\
             Metal:::native: [0.75, 0.80] color: #ff3300\n\
             Direct2D: [0.55, 0.60] radius: 8, stroke-width: 2px\n\
             classDef native color: #109060, radius: 12, stroke-color: #310085, stroke-width: 4px\n",
        )
        .unwrap();

        let metal = &diagram.quadrant_points[0];
        assert_eq!(metal.color.as_deref(), Some("#ff3300"));
        assert_eq!(metal.radius, Some(12.0));
        assert_eq!(metal.stroke_color.as_deref(), Some("#310085"));
        assert_eq!(metal.stroke_width, Some(4.0));
        let direct2d = &diagram.quadrant_points[1];
        assert_eq!(direct2d.radius, Some(8.0));
        assert_eq!(direct2d.stroke_width, Some(2.0));
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
        assert!(d.sections[0].tasks[0].tags.done);
    }

    #[test]
    fn gantt_grammar_preserves_title_and_accessibility_metadata() {
        let diagram = parse_gantt(
            "gantt\n\
             title Native project\n\
             accTitle: Accessible project timeline\n\
             accDescr {\n  Build and ship\n}\n\
             section Delivery\n\
             Release :milestone, release, 2026-03-01, 0d\n",
        )
        .unwrap();
        assert_eq!(diagram.title.as_deref(), Some("Native project"));
        assert_eq!(
            diagram.accessibility_title.as_deref(),
            Some("Accessible project timeline")
        );
        assert_eq!(
            diagram.accessibility_description.as_deref(),
            Some("Build and ship")
        );
    }

    #[test]
    fn gantt_grammar_rejects_missing_header_and_malformed_tasks() {
        assert!(parse_gantt("section Delivery\nRelease :r1, 2026-03-01, 1d").is_err());
        assert!(parse_gantt("gantt\nRelease :done,").is_err());
        assert!(parse_gantt("gantt\nRelease :r1, 2026-03-01, forever").is_err());
    }

    #[test]
    fn gantt_grammar_accepts_prefixed_title_section_and_task_labels() {
        let semicolon = parse_gantt(
            "gantt\ntitle ;Release plan\nsection ;Build\n;Parser :parser, 2026-03-01, 1d",
        )
        .unwrap();
        assert_eq!(semicolon.title.as_deref(), Some("Release plan"));
        assert_eq!(semicolon.sections[0].label.as_deref(), Some("Build"));
        assert_eq!(semicolon.sections[0].tasks[0].label, "Parser");

        let hash = parse_gantt(
            "gantt\ntitle #Release plan\nsection #Build\n#Parser :parser, 2026-03-01, 1d",
        )
        .unwrap();
        assert_eq!(hash.title.as_deref(), Some("Release plan"));
        assert_eq!(hash.sections[0].label.as_deref(), Some("Build"));
        assert_eq!(hash.sections[0].tasks[0].label, "Parser");

        let delimited = parse_gantt(
            "gantt;title ;Release plan;section ;Build;Parser :parser, 2026-03-01, 1d",
        )
        .unwrap();
        assert_eq!(delimited.sections[0].tasks[0].label, "Parser");
    }

    #[test]
    fn gantt_grammar_lowers_html_break_variants_to_semantic_newlines() {
        let diagram = parse_gantt(
            "gantt\ntitle Release<br>Plan\nsection Line1<br>Line2<br/>Line3</br />Line4<br\t/>Line5\nTask<br />One :task, 2026-03-01, 1d",
        )
        .unwrap();
        assert_eq!(diagram.title.as_deref(), Some("Release\nPlan"));
        assert_eq!(
            diagram.sections[0].label.as_deref(),
            Some("Line1\nLine2\nLine3\nLine4\nLine5")
        );
        assert_eq!(diagram.sections[0].tasks[0].label, "Task\nOne");
    }

    #[test]
    fn gantt_preserves_task_links_and_callbacks() {
        let diagram = parse_gantt(
            "gantt\nTask :done, t1, 2026-03-01, 2d\nclick t1 call inspectTask(t1, release) href \"https://example.com/task/call\"\n",
        )
        .unwrap();
        let task = &diagram.sections[0].tasks[0];
        assert_eq!(task.link.as_deref(), Some("https://example.com/task/call"));
        assert_eq!(task.callback.as_deref(), Some("inspectTask"));
        assert_eq!(task.callback_args.as_deref(), Some("t1, release"));

        assert!(parse_gantt("gantt\nclick missing href \"https://example.com\"\n").is_err());
        assert!(parse_gantt("gantt\nTask :t1, 2026-03-01, 1d\nclick t1\n").is_err());
    }

    #[test]
    fn gantt_preserves_calendar_and_axis_controls() {
        let diagram = parse_gantt(
            "gantt\naxisFormat %m/%d\ntickInterval 2weeks\nexcludes weekends, 2026-01-01\nexcludes monday\nincludes 2026-01-03, 2026-01-03\ninclusiveEndDates\ntopAxis\ntodayMarker off\nweekday monday\nweekend friday\nTask :t1, 2026-01-01, 2d\n",
        ).unwrap();
        assert_eq!(diagram.config.axis_format.as_deref(), Some("%m/%d"));
        assert_eq!(diagram.config.tick_interval.as_deref(), Some("2weeks"));
        assert_eq!(diagram.config.excludes, ["weekends", "2026-01-01", "monday"]);
        assert_eq!(diagram.config.includes, ["2026-01-03"]);
        assert!(diagram.config.inclusive_end_dates);
        assert!(diagram.config.top_axis);
        assert_eq!(diagram.config.today_marker.as_deref(), Some("off"));
        assert_eq!(diagram.config.weekday.as_deref(), Some("monday"));
        assert_eq!(diagram.config.weekend.as_deref(), Some("friday"));
    }

    #[test]
    fn gantt_preserves_explicit_end_dates() {
        let diagram = parse_gantt(
            "gantt\ninclusiveEndDates\nRelease :release, 2026-03-01, 2026-03-03\n",
        ).unwrap();
        let task = &diagram.sections[0].tasks[0];
        assert_eq!(task.end, Some(TaskEnd::Date("2026-03-03".into())));
        assert_eq!(task.duration, GanttDuration::default());
    }

    #[test]
    fn gantt_preserves_multi_task_after_and_until_dependencies() {
        let diagram = parse_gantt(
            "gantt\nA :a, 2026-03-01, 2d\nB :b, 2026-03-02, 5d\nC :c, after a b, 1d\nWindow :w, 2026-02-28, until b c\n",
        ).unwrap();
        let tasks = &diagram.sections[0].tasks;
        assert_eq!(tasks[2].start, TaskStart::After(vec!["a".into(), "b".into()]));
        assert_eq!(tasks[2].dependencies, ["a", "b"]);
        assert_eq!(tasks[3].end, Some(TaskEnd::Until(vec!["b".into(), "c".into()])));

        assert!(parse_gantt("gantt\nA :a, after missing, 1d\n").is_err());
        assert!(parse_gantt("gantt\nA :a, after b, 1d\nB :b, after a, 1d\n").is_err());
        assert!(parse_gantt("gantt\nA :a, 2026-01-01, until missing\n").is_err());
    }

    #[test]
    fn gantt_generates_ids_and_preserves_sequential_task_semantics() {
        let diagram = parse_gantt(
            "gantt\nsection First\nAnchor :anchor, 2026-03-01, 2d\nGenerated :after anchor, 1d\nsection Second\nSequential :active, 3d\nDated :2026-03-10, 1d\n",
        ).unwrap();
        let tasks = diagram.sections.iter().flat_map(|section| &section.tasks).collect::<Vec<_>>();
        assert_eq!(tasks[1].id, "task1");
        assert_eq!(tasks[1].start, TaskStart::After(vec!["anchor".into()]));
        assert_eq!(tasks[2].id, "task2");
        assert_eq!(tasks[2].start, TaskStart::After(vec!["task1".into()]));
        assert!(tasks[2].tags.active);
        assert_eq!(tasks[3].id, "task3");
        assert_eq!(tasks[3].start, TaskStart::Date("2026-03-10".into()));
        assert!(parse_gantt("gantt\nFirst :2d\n").is_err());
    }

    #[test]
    fn gantt_preserves_combined_and_vertical_task_tags() {
        let diagram = parse_gantt(
            "gantt\nCombined :crit, done, milestone, combined, 2026-03-01, 0d\nDeadline :vert, active, deadline, 2026-03-05, 0d\n",
        ).unwrap();
        let tasks = &diagram.sections[0].tasks;
        assert_eq!(
            tasks[0].tags,
            GanttTaskTags {
                done: true,
                critical: true,
                milestone: true,
                ..GanttTaskTags::default()
            }
        );
        assert_eq!(
            tasks[1].tags,
            GanttTaskTags {
                active: true,
                vertical: true,
                ..GanttTaskTags::default()
            }
        );
    }

    #[test]
    fn gantt_compiles_and_enforces_non_iso_date_formats() {
        let diagram = parse_gantt(
            "gantt\ndateFormat DD/MM/YYYY HH:mm\nsection Build\nTask :t1, 02/01/2026 06:30, 03/01/2026 18:45",
        ).unwrap();
        assert_eq!(diagram.date_format.source, "DD/MM/YYYY HH:mm");
        assert!(diagram.date_format.parts.contains(&GanttDateFormatPart::Hour24));
        assert!(matches!(diagram.sections[0].tasks[0].end,
            Some(TaskEnd::Date(ref value)) if value == "03/01/2026 18:45"));
        assert!(parse_gantt("gantt\ndateFormat DD/MM/YYYY\nTask :t1, 2026-01-02, 1d")
            .unwrap_err().message.contains("does not match dateFormat"));
    }

    #[test]
    fn gantt_compiles_text_month_literals_and_unix_formats() {
        let named = parse_gantt(
            "gantt\ndateFormat D MMMM YYYY [at] HH:mm:ss.SSS\nTask :t1, 2 January 2026 at 04:05:06.007, 1d",
        ).unwrap();
        assert!(named.date_format.parts.contains(&GanttDateFormatPart::MonthLong));
        let unix = parse_gantt("gantt\ndateFormat X\nTask :t1, 1767225600, 1767312000").unwrap();
        assert!(matches!(unix.sections[0].tasks[0].end, Some(TaskEnd::Date(_))));
        assert!(parse_gantt("gantt\ndateFormat YYYY-QQ\nTask :t1, 2026-01, 1d").is_err());
    }

    #[test]
    fn gantt_compiles_timezone_offset_formats() {
        let colon = parse_gantt(
            "gantt\ndateFormat YYYY-MM-DD[T]HH:mmZ\nTask :t1, 2026-01-02T04:05+02:30, 1h",
        ).unwrap();
        assert!(colon.date_format.parts.contains(&GanttDateFormatPart::TimezoneOffsetColon));

        let compact = parse_gantt(
            "gantt\ndateFormat YYYY-MM-DD[T]HH:mmZZ\nTask :t1, 2026-01-02T04:05-0730, 1h",
        ).unwrap();
        assert!(compact.date_format.parts.contains(&GanttDateFormatPart::TimezoneOffsetCompact));
        assert!(parse_gantt(
            "gantt\ndateFormat YYYY-MM-DD[T]HH:mmZ\nTask :t1, 2026-01-02T04:05+0230, 1h",
        ).is_err());
    }

    #[test]
    fn gantt_accepts_single_component_seconds_format() {
        let diagram = parse_gantt(
            "gantt\ndateFormat ss\nsection Network Request\nRTT :rtt, 0, 20",
        ).unwrap();
        assert_eq!(diagram.date_format.parts, [GanttDateFormatPart::Second]);
        assert_eq!(diagram.sections[0].tasks[0].start, TaskStart::Date("0".into()));
        assert_eq!(diagram.sections[0].tasks[0].end, Some(TaskEnd::Date("20".into())));
    }

    #[test]
    fn gantt_preserves_sub_day_duration_units() {
        let diagram = parse_gantt(
            "gantt\ndateFormat x\nA :a, 0, 20ms\nB :b, after a, 0.1s\nC :c, after b, 2m\nD :d, after c, 3h",
        ).unwrap();
        let tasks = &diagram.sections[0].tasks;
        assert_eq!(tasks[0].duration, GanttDuration { value: 20.0, unit: GanttDurationUnit::Milliseconds });
        assert_eq!(tasks[1].duration, GanttDuration { value: 0.1, unit: GanttDurationUnit::Seconds });
        assert_eq!(tasks[2].duration, GanttDuration { value: 2.0, unit: GanttDurationUnit::Minutes });
        assert_eq!(tasks[3].duration, GanttDuration { value: 3.0, unit: GanttDurationUnit::Hours });
    }

    #[test]
    fn pie_parses_slices() {
        let d = parse_pie(PIE_SRC).unwrap();
        assert_eq!(d.kind, ChartKind::Pie);
        assert!(d.show_data);
        assert_eq!(d.slices.len(), 2);
        assert_eq!(d.slices[0].label, "Dogs");
        assert_eq!(d.slices[0].value, 60.0);
    }

    #[test]
    fn pie_parses_title_accessibility_and_non_negative_values() {
        let d = parse_pie(
            "pie title Adoption\naccTitle: Adoption breakdown\naccDescr {\nDogs and cats\nby share\n}\n\"Dogs\": 0\n\"Cats\": 40.12",
        )
        .unwrap();
        assert_eq!(d.title.as_deref(), Some("Adoption"));
        assert_eq!(d.accessibility_title.as_deref(), Some("Adoption breakdown"));
        assert_eq!(
            d.accessibility_description.as_deref(),
            Some("Dogs and cats\nby share")
        );
        assert_eq!(d.slices[0].value, 0.0);
        assert!(!d.show_data);
    }

    #[test]
    fn pie_rejects_negative_values() {
        let error = parse_pie("pie\n\"Dogs\": -60.67").unwrap_err();
        assert!(error.message.contains("values must be non-negative"));
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
    fn sankey_parses_case_insensitive_headers_and_quoted_weights() {
        let d = parse_sankey("SANKEY-BETA\nGrid,\"Heating, homes\",\"113.726\"").unwrap();
        assert_eq!(d.flows[0].source, "Grid");
        assert_eq!(d.flows[0].target, "Heating, homes");
        assert_eq!(d.flows[0].weight, 113.726);
    }

    #[test]
    fn sankey_requires_csv_rows_and_header_newline() {
        assert!(parse_sankey("sankey").is_err());
        assert!(parse_sankey("sankey A,B,1").is_err());
    }

    #[test]
    fn sankey_preserves_empty_rfc_csv_node_ids() {
        let d = parse_sankey("sankey\n,A,1\nA,,2\n\"\",B,3").unwrap();
        assert_eq!(d.flows[0].source, "");
        assert_eq!(d.flows[1].target, "");
        assert_eq!(d.flows[2].source, "");
    }

    #[test]
    fn gitgraph_parses_branch_history() {
        let d = parse_gitgraph(GITGRAPH_SRC).unwrap();
        assert_eq!(d.direction, DiagramDirection::Lr);
        assert_eq!(d.branches.len(), 2);
        assert_eq!(d.branches[1].order, Some(1));
        assert_eq!(d.events.len(), 5);
        assert!(matches!(
            &d.events[1],
            GitEvent::Checkout { branch } if branch == "develop"
        ));
        assert!(matches!(
            &d.events[4],
            GitEvent::Merge { from, id, resolved_id, parents, .. }
                if from == "develop"
                    && id.as_deref() == Some("merge-1")
                    && resolved_id == "merge-1"
                    && parents == &["root", "feature"]
        ));
    }

    #[test]
    fn gitgraph_parses_vertical_directions() {
        assert_eq!(
            parse_gitgraph("gitGraph TB:\ncommit").unwrap().direction,
            DiagramDirection::Tb
        );
        assert_eq!(
            parse_gitgraph("gitGraph BT:\ncommit").unwrap().direction,
            DiagramDirection::Bt
        );
    }

    #[test]
    fn gitgraph_branch_creation_checks_out_the_new_branch() {
        let d =
            parse_gitgraph("gitGraph\ncommit id: \"root\"\nbranch feature\ncommit id: \"work\"")
                .unwrap();
        assert!(matches!(
            &d.events[1],
            GitEvent::Checkout { branch } if branch == "feature"
        ));
        assert!(matches!(
            &d.events[2],
            GitEvent::Commit { branch, .. } if branch == "feature"
        ));
    }

    #[test]
    fn gitgraph_rejects_duplicate_branch_creation() {
        let error = parse_gitgraph("gitGraph\nbranch feature\nbranch feature").unwrap_err();
        assert!(error.message.contains("existing GitGraph branch"));
    }

    #[test]
    fn gitgraph_validates_merge_and_cherry_pick_history() {
        assert!(parse_gitgraph("gitGraph\ncommit\nmerge main").is_err());
        assert!(parse_gitgraph("gitGraph\ncommit\ncherry-pick id: \"missing\"").is_err());

        let source = "gitGraph\ncommit id: \"root\"\nbranch feature\ncommit id: \"work\"\ncheckout main\nmerge feature id: \"merged\"\nbranch release\ncherry-pick id: \"merged\"";
        let error = parse_gitgraph(source).unwrap_err();
        assert!(error.message.contains("requires a parent"));
    }

    #[test]
    fn gitgraph_parses_cherry_pick_metadata() {
        let d = parse_gitgraph(
            "gitGraph\ncommit id: \"root\"\ncommit id: \"abc123\"\nbranch release\ncherry-pick id: \"abc123\" parent: \"root\"",
        )
        .unwrap();
        assert!(matches!(
            &d.events[3],
            GitEvent::CherryPick { id, parent, branch, .. }
                if id == "abc123"
                    && parent.as_deref() == Some("root")
                    && branch == "release"
        ));
    }

    #[test]
    fn gitgraph_preserves_repeated_tags_in_source_order() {
        let d = parse_gitgraph(
            "gitGraph\ncommit id: \"root\" tag: \"v1\" tag: \"stable\"\nbranch release\ncherry-pick id: \"root\" tag: \"picked\" tag: \"backport\"\ncheckout main\nmerge release tag: \"v2\" tag: \"latest\"",
        )
        .unwrap();
        assert!(matches!(
            &d.events[0],
            GitEvent::Commit { tags, .. } if tags == &["v1", "stable"]
        ));
        assert!(matches!(
            &d.events[2],
            GitEvent::CherryPick { tags, .. } if tags == &["picked", "backport"]
        ));
        assert!(matches!(
            &d.events[4],
            GitEvent::Merge { tags, .. } if tags == &["v2", "latest"]
        ));
    }

    #[test]
    fn gitgraph_parses_all_commit_types() {
        let d = parse_gitgraph(
            "gitGraph\ncommit id: \"normal\" type: NORMAL\ncommit id: \"reverse\" type: REVERSE\ncommit id: \"highlight\" type: HIGHLIGHT",
        )
        .unwrap();
        assert!(matches!(
            &d.events[0],
            GitEvent::Commit {
                type_: GitCommitType::Normal,
                ..
            }
        ));
        assert!(matches!(
            &d.events[1],
            GitEvent::Commit {
                type_: GitCommitType::Reverse,
                ..
            }
        ));
        assert!(matches!(
            &d.events[2],
            GitEvent::Commit {
                type_: GitCommitType::Highlight,
                ..
            }
        ));
    }

    #[test]
    fn gitgraph_parses_title_and_accessibility_metadata() {
        let source = "gitGraph TB:\ntitle Release history\naccTitle: Accessible release history\naccDescr {\nTwo branches\nand one merge\n}\ncommit";
        let d = parse_gitgraph(source).unwrap();
        assert_eq!(d.title.as_deref(), Some("Release history"));
        assert_eq!(
            d.accessibility_title.as_deref(),
            Some("Accessible release history")
        );
        assert_eq!(
            d.accessibility_description.as_deref(),
            Some("Two branches\nand one merge")
        );
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
    fn dispatch_quadrant() {
        let src = "quadrantChart\nquadrant-1 Invest\nMetal: [0.75, 0.8]";
        match parse_any_mermaid(src).unwrap() {
            MermaidDiagram::Chart(chart) => assert_eq!(chart.kind, ChartKind::Quadrant),
            _ => panic!("expected chart diagram"),
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
            "stateDiagram-v2\nReady --> Running\nstyle Ready fill:#fee2e2,stroke:#991b1b,color:#111827,stroke-width:3px,font-size:24px,font-weight:bold,font-style:italic,font-family:\"Avenir Next\"\n",
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
        assert_eq!(style.font_size, Some(24.0));
        assert_eq!(style.font_weight, Some(700));
        assert_eq!(style.font_italic, Some(true));
        assert_eq!(style.font_family.as_deref(), Some("Avenir Next"));
    }

    #[test]
    fn state_lowers_background_and_border_style_aliases() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\nclassDef emphasized background:  #bbb,border:1.5px solid red\nReady:::emphasized --> Done\n",
        )
        .expect("CSS-like state style aliases should parse");

        let ready = diagram
            .nodes
            .iter()
            .find(|node| node.id == "Ready")
            .expect("styled state node");
        let style = ready.style.as_ref().expect("resolved state style");
        assert_eq!(style.fill.as_deref(), Some("#bbb"));
        assert_eq!(style.stroke.as_deref(), Some("red"));
        assert_eq!(style.stroke_width, Some(1.5));
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
    fn state_attaches_notes_to_composite_groups_without_duplicate_nodes() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\nstate \"Not Shooting State\" as NotShooting {\nIdle --> Configuring\n}\nnote right of NotShooting: This is a note on a composite state\n",
        )
        .expect("composite state note should parse");

        assert!(diagram.groups.iter().any(|group| group.id == "NotShooting"));
        assert!(!diagram.nodes.iter().any(|node| node.id == "NotShooting"));
        let association = diagram
            .edges
            .iter()
            .find(|edge| edge.kind == EdgeKind::NoteAssociation)
            .expect("composite note association");
        assert_eq!(association.from, "NotShooting");
        assert!(association.to.starts_with("__state_note_"));
    }

    #[test]
    fn state_preserves_accessibility_metadata() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\naccTitle: State lifecycle\naccDescr {\nReady transitions to running\nAcross two lines\n}\nReady --> Running\n",
        )
        .expect("state accessibility metadata should parse");

        assert_eq!(
            diagram.accessibility_title.as_deref(),
            Some("State lifecycle")
        );
        assert_eq!(
            diagram.accessibility_description.as_deref(),
            Some("Ready transitions to running\nAcross two lines")
        );
    }

    #[test]
    fn state_preserves_click_links_and_tooltips() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\nclick Ready \"https://example.com/ready\" \"Open ready state\"\nclick Running href \"https://example.com/run\"\nReady --> Running\n",
        )
        .expect("state click links should parse");

        assert_eq!(diagram.links.len(), 2);
        assert_eq!(diagram.links[0].node_id, "Ready");
        assert_eq!(diagram.links[0].url, "https://example.com/ready");
        assert_eq!(
            diagram.links[0].tooltip.as_deref(),
            Some("Open ready state")
        );
        assert_eq!(diagram.links[1].node_id, "Running");
        assert_eq!(diagram.links[1].tooltip, None);
    }

    #[test]
    fn state_parses_nested_composite_groups() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\nstate Outer {\nA --> B\nstate Inner {\nC --> D\n}\n}\n",
        )
        .expect("nested composite states should parse");

        assert_eq!(diagram.groups.len(), 2);
        assert_eq!(diagram.groups[0].id, "Outer");
        assert_eq!(diagram.groups[0].node_ids, vec!["A", "B"]);
        assert_eq!(diagram.groups[1].id, "Inner");
        assert_eq!(diagram.groups[1].parent_id.as_deref(), Some("Outer"));
        assert_eq!(diagram.groups[1].node_ids, vec!["C", "D"]);
    }

    #[test]
    fn state_preserves_composite_aliases_and_styles() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\nclassDef phase fill:#ecfccb,stroke:#3f6212,color:#365314\nstate \"Processing Queue\" as Processing {\nA --> B\n}\nclass Processing phase\nstyle Processing stroke-width:3px\n",
        )
        .expect("styled aliased composite state should parse");
        let group = &diagram.groups[0];

        assert_eq!(group.id, "Processing");
        assert_eq!(group.label.text, "Processing Queue");
        assert_eq!(
            group.style.as_ref().and_then(|style| style.fill.as_deref()),
            Some("#ecfccb")
        );
        assert_eq!(
            group.style.as_ref().and_then(|style| style.stroke_width),
            Some(3.0)
        );
        assert!(!diagram.nodes.iter().any(|node| node.id == "Processing"));
    }

    #[test]
    fn state_preserves_concurrent_region_membership() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\nstate Active {\nOff --> On\n--\nIdle --> Busy\n}\n",
        )
        .expect("concurrent state regions should parse");
        let group = &diagram.groups[0];

        assert_eq!(group.regions, vec![vec!["Off", "On"], vec!["Idle", "Busy"]]);
    }

    #[test]
    fn state_transitions_keep_composite_group_endpoints() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\nstate Active {\nIdle --> Busy\n}\nReady --> Active\nActive --> Done\n",
        )
        .expect("composite transition endpoints should parse");

        assert_eq!(diagram.edges[1].to, "Active");
        assert_eq!(diagram.edges[2].from, "Active");
        assert!(!diagram.nodes.iter().any(|node| node.id == "Active"));
    }

    #[test]
    fn state_preserves_modern_and_legacy_titles() {
        let modern = parse_state_diagram("stateDiagram-v2\ntitle Native lifecycle\nA --> B\n")
            .expect("modern state title");
        let legacy = parse_state_diagram("stateDiagram-v2\ntitle: Legacy lifecycle\nA --> B\n")
            .expect("legacy state title");

        assert_eq!(modern.title.as_deref(), Some("Native lifecycle"));
        assert_eq!(legacy.title.as_deref(), Some("Legacy lifecycle"));
    }

    #[test]
    fn state_accumulates_repeated_description_lines() {
        let diagram =
            parse_state_diagram("stateDiagram-v2\nActive: First detail\nActive: Second detail\n")
                .expect("repeated state descriptions");

        assert_eq!(diagram.nodes[0].label.text, "First detail\nSecond detail");
    }

    #[test]
    fn state_preserves_composite_local_direction() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\ndirection TB\nstate Active {\ndirection LR\nIdle --> Busy\n}\n",
        )
        .expect("composite local direction");

        assert_eq!(diagram.direction, DiagramDirection::Tb);
        assert_eq!(diagram.groups[0].direction, Some(DiagramDirection::Lr));
    }

    #[test]
    fn state_preserves_requested_scale_width() {
        let diagram = parse_state_diagram("stateDiagram-v2\nscale 640 width\nA --> B\n")
            .expect("state scale width");

        assert_eq!(diagram.requested_width, Some(640.0));
    }

    #[test]
    fn state_preserves_hide_empty_description_directive() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\nhide empty description\nstate Junction <<choice>>\n",
        )
        .expect("hide empty description directive");

        assert!(diagram.hide_empty_descriptions);
        assert!(diagram.nodes[0].label.text.is_empty());
    }

    #[test]
    fn state_decodes_entities_and_line_breaks() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\nReady: Metal #9829;<br>native<br/>and<br />GPU<br\t/>shaped\nQuoted: \"One<BR \t/>Two\"\n",
        )
        .expect("state text entities and line breaks");

        assert_eq!(
            diagram.nodes[0].label.text,
            "Metal ♥\nnative\nand\nGPU\nshaped"
        );
        assert_eq!(diagram.nodes[1].label.text, "One\nTwo");
    }

    #[test]
    fn state_decodes_line_break_variants_in_multiline_notes() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\nState1\nnote right of State1\nLine1<br>Line2<br/>Line3<br />Line4<br\t/>Line5\nend note\n",
        )
        .expect("state note line breaks");
        let note = diagram
            .nodes
            .iter()
            .find(|node| node.shape == Some(DiagramShape::Note))
            .expect("note node");

        assert_eq!(note.label.text, "Line1\nLine2\nLine3\nLine4\nLine5");
    }

    #[test]
    fn state_treats_single_percent_and_adjacent_words_as_bare_states() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\n% not a comment\nMoving --> Still %inline\nStill%Active\n",
        )
        .expect("single-percent state syntax should parse");
        let ids: Vec<_> = diagram.nodes.iter().map(|node| node.id.as_str()).collect();

        assert_eq!(
            ids,
            [
                "%",
                "not",
                "a",
                "comment",
                "Moving",
                "Still",
                "%inline",
                "Still%Active"
            ]
        );
        assert_eq!(diagram.edges.len(), 1);
        assert_eq!(diagram.edges[0].from, "Moving");
        assert_eq!(diagram.edges[0].to, "Still");
    }

    #[test]
    fn state_applies_inline_style_to_multiple_targets() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\nstate Active {\nA --> B\n}\nstyle A,B,Active fill:#dcfce7,stroke:#166534\n",
        )
        .expect("multi-target state style");

        assert_eq!(
            diagram.nodes[0].style.as_ref().unwrap().fill.as_deref(),
            Some("#dcfce7")
        );
        assert_eq!(
            diagram.nodes[1].style.as_ref().unwrap().stroke.as_deref(),
            Some("#166534")
        );
        assert_eq!(
            diagram.groups[0].style.as_ref().unwrap().fill.as_deref(),
            Some("#dcfce7")
        );
    }

    #[test]
    fn state_skips_hash_comments() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\n# lifecycle comment\n#abc color-looking comment\nReady --> Running # transition comment\nRunning: Metal #9829; native\nstyle Ready fill:#dbeafe\n",
        )
        .expect("hash comments should parse");

        assert_eq!(diagram.edges.len(), 1);
        assert_eq!(diagram.nodes.len(), 2);
        let ready = diagram
            .nodes
            .iter()
            .find(|node| node.id == "Ready")
            .unwrap();
        assert_eq!(
            ready.style.as_ref().unwrap().fill.as_deref(),
            Some("#dbeafe")
        );
        let running = diagram
            .nodes
            .iter()
            .find(|node| node.id == "Running")
            .unwrap();
        assert_eq!(running.label.text, "Metal ♥ native");
    }

    #[test]
    fn state_alias_accepts_a_trailing_description() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\nstate \"Processing queue\" as Processing: Awaiting native work\n",
        )
        .expect("state alias with trailing description");

        assert_eq!(diagram.nodes.len(), 1);
        assert_eq!(diagram.nodes[0].id, "Processing");
        assert_eq!(
            diagram.nodes[0].label.text,
            "Processing queue\nAwaiting native work"
        );
    }

    #[test]
    fn state_composes_multiple_classes_in_source_order() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\nclassDef base fill:#dbeafe,stroke:#1d4ed8\nclassDef emphasized fill:#dcfce7,stroke-width:4px\nclass Ready,Waiting base emphasized\nReady --> Waiting\n",
        )
        .expect("multiple state classes");

        for node in &diagram.nodes {
            let style = node.style.as_ref().unwrap();
            assert_eq!(style.fill.as_deref(), Some("#dcfce7"));
            assert_eq!(style.stroke.as_deref(), Some("#1d4ed8"));
            assert_eq!(style.stroke_width, Some(4.0));
        }
    }

    #[test]
    fn state_preserves_colons_inside_descriptions_and_transition_labels() {
        let diagram = parse_state_diagram(
            "stateDiagram-v2\nReady: Status: awaiting work\nReady --> Running: Trigger: native event\n",
        )
        .expect("colon-bearing state text");

        assert_eq!(diagram.nodes[0].label.text, "Status: awaiting work");
        assert_eq!(
            diagram.edges[0].label.as_ref().unwrap().text,
            "Trigger: native event"
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
        assert_eq!(crate::VERSION, "0.126.0");
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
