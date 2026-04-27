//! Grammar-driven parser for a focused Mermaid flowchart subset.

pub const VERSION: &str = "0.2.0";

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


// ============================================================================
// DG04 — Extended Mermaid parsers for Chart, Structural, and Temporal families
// ============================================================================

use diagram_ir::{
    Axis, AxisKind, ChartDiagram, ChartKind, ChartOrientation, ChartSeries,
    Compartment, CompartmentKind, GanttDiagram, GanttSection, GanttTask,
    PieSlice, RelKind, SankeyFlow, SankeyNode,
    SeriesKind, StructuralDiagram, StructuralKind, StructuralNode,
    StructuralNodeKind, StructuralRelationship, TaskStart, TaskStatus,
    TemporalBody, TemporalDiagram, TemporalKind,
};

/// Union of all Mermaid diagram variants that `parse_any_mermaid` can return.
pub enum MermaidDiagram {
    Graph(GraphDiagram),
    Chart(ChartDiagram),
    Structural(StructuralDiagram),
    Temporal(TemporalDiagram),
}

/// Dispatch to the correct sub-parser based on the first keyword line.
///
/// Supported diagram types:
/// - `flowchart`/`graph` → `GraphDiagram`
/// - `classDiagram` → `StructuralDiagram`
/// - `xychart-beta`/`xychart` → `ChartDiagram`
/// - `gantt` → `TemporalDiagram` (Gantt body)
pub fn parse_any_mermaid(source: &str) -> Result<MermaidDiagram, ParseError> {
    let first = first_keyword(source);
    match first.as_str() {
        "flowchart" | "graph" => parse_to_diagram(source).map(MermaidDiagram::Graph),
        "classDiagram" => parse_class_diagram(source).map(MermaidDiagram::Structural),
        "xychart-beta" | "xychart" => parse_xychart(source).map(MermaidDiagram::Chart),
        "gantt" => parse_gantt(source).map(|g| MermaidDiagram::Temporal(TemporalDiagram {
            kind: TemporalKind::Gantt,
            title: None,
            body: TemporalBody::Gantt(g),
        })),
        other => Err(ParseError { message: format!("Unknown diagram type: {other:?}"), line: 0, col: 0 }),
    }
}

fn first_keyword(source: &str) -> String {
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") { continue; }
        return trimmed.split_whitespace().next().unwrap_or("").to_string();
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
        if t == "classDiagram" { break; }
        if t.starts_with("%%") || t.is_empty() { continue; }
        if t.starts_with("title") {
            title = Some(t.trim_start_matches("title").trim().to_string());
        }
    }

    for line in lines {
        let t = line.trim();
        if t.is_empty() || t.starts_with("%%") { continue; }

        if t.starts_with("class ") {
            let rest = t[6..].trim();
            let (id_str, body_str): (String, Option<String>) = if let Some(pos) = rest.find('{') {
                (rest[..pos].trim().to_string(), Some(rest[pos+1..].trim_end_matches('}').to_string()))
            } else {
                (rest.to_string(), None)
            };
            let id = id_str.trim().to_string();

            let mut compartments: Vec<Compartment> = Vec::new();
            if let Some(body) = body_str.as_deref() {
                let entries: Vec<String> = body.split(';')
                    .map(|e| e.trim().to_string())
                    .filter(|e| !e.is_empty())
                    .collect();
                if !entries.is_empty() {
                    // Heuristic: entries with `()` are methods, otherwise fields.
                    let fields: Vec<String> = entries.iter()
                        .filter(|e| !e.contains('('))
                        .map(|e| strip_visibility(e))
                        .collect();
                    let methods: Vec<String> = entries.iter()
                        .filter(|e| e.contains('('))
                        .map(|e| strip_visibility(e))
                        .collect();
                    if !fields.is_empty() {
                        compartments.push(Compartment { kind: CompartmentKind::Fields, entries: fields });
                    }
                    if !methods.is_empty() {
                        compartments.push(Compartment { kind: CompartmentKind::Methods, entries: methods });
                    }
                }
            }

            // Update existing node or create a new one.
            if let Some(existing) = nodes.iter_mut().find(|n| n.id == id) {
                if !compartments.is_empty() { existing.compartments = compartments; }
            } else {
                nodes.push(StructuralNode {
                    id: id.clone(), label: id,
                    stereotype: None,
                    node_kind: StructuralNodeKind::Class,
                    compartments,
                });
            }
        } else if let Some(rel) = parse_class_relationship(t) {
            // Make sure both nodes exist.
            for id in [&rel.from, &rel.to] {
                if !nodes.iter().any(|n| &n.id == id) {
                    nodes.push(StructuralNode {
                        id: id.clone(), label: id.clone(),
                        stereotype: None,
                        node_kind: StructuralNodeKind::Class,
                        compartments: vec![],
                    });
                }
            }
            relationships.push(rel);
        }
    }

    Ok(StructuralDiagram { kind: StructuralKind::Class, title, nodes, relationships })
}

fn strip_visibility(s: &str) -> String {
    let s = s.trim();
    if s.starts_with(['+', '-', '#', '~']) {
        s[1..].trim().to_string()
    } else {
        s.to_string()
    }
}

fn strip_visibility_owned(s: &String) -> String {
    strip_visibility(s.as_str())
}

/// Parse a Mermaid class relationship line like `Animal <|-- Dog : extends`.
fn parse_class_relationship(line: &str) -> Option<StructuralRelationship> {
    // Try each arrow pattern.
    let arrows: &[(&str, RelKind)] = &[
        ("<|--", RelKind::Inheritance),
        ("<|..", RelKind::Realization),
        ("*--",  RelKind::Composition),
        ("o--",  RelKind::Aggregation),
        ("-->",  RelKind::Association),
        ("..",   RelKind::Dependency),
        ("--",   RelKind::Link),
    ];
    for (arrow, kind) in arrows {
        if let Some(pos) = line.find(arrow) {
            let from  = line[..pos].trim().to_string();
            let after = line[pos + arrow.len()..].trim();
            let (to, label) = if let Some(colon) = after.find(':') {
                (after[..colon].trim().to_string(), Some(after[colon+1..].trim().to_string()))
            } else {
                (after.to_string(), None)
            };
            if !from.is_empty() && !to.is_empty() {
                return Some(StructuralRelationship {
                    from, to, kind: kind.clone(),
                    from_mult: None, to_mult: None, label,
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
        if t.is_empty() || t.starts_with("%%") { continue; }
        if !past_header {
            if t.starts_with("xychart") { past_header = true; }
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
                    rest[end+2..].trim()
                } else { rest }
            } else { rest };
            let nums: Vec<f64> = rest.split_whitespace()
                .filter(|s| s.chars().all(|c| c.is_ascii_digit() || c == '-' || c == '.'))
                .filter_map(|s| s.parse().ok())
                .collect();
            if nums.len() >= 2 { y_min = nums[0]; y_max = nums[nums.len()-1]; }
        } else if t.starts_with("bar") {
            let data = parse_data_list(&t[3..]);
            series.push(ChartSeries { kind: SeriesKind::Bar, label: Some("bar".into()), data });
        } else if t.starts_with("line") {
            let data = parse_data_list(&t[4..]);
            series.push(ChartSeries { kind: SeriesKind::Line, label: Some("line".into()), data });
        }
    }

    let x_axis = if !x_cats.is_empty() {
        Some(Axis { kind: AxisKind::Categorical, title: None, categories: x_cats, min: 0.0, max: 0.0 })
    } else { None };
    let y_axis = Some(Axis {
        kind: AxisKind::Numeric, title: None, categories: vec![], min: y_min, max: y_max,
    });

    Ok(ChartDiagram {
        title, kind: ChartKind::Xy,
        x_axis, y_axis, series,
        slices: vec![], sankey_nodes: vec![], flows: vec![],
        orientation: ChartOrientation::Vertical,
    })
}

fn parse_bracket_list(s: &str) -> Vec<String> {
    let s = s.trim();
    let inner = if let (Some(l), Some(r)) = (s.find('['), s.rfind(']')) {
        &s[l+1..r]
    } else { s };
    inner.split(',').map(|x| x.trim().trim_matches('"').to_string())
        .filter(|x| !x.is_empty()).collect()
}

fn parse_data_list(s: &str) -> Vec<f64> {
    let s = s.trim();
    let inner = if let (Some(l), Some(r)) = (s.find('['), s.rfind(']')) {
        &s[l+1..r]
    } else { s };
    inner.split(',').filter_map(|x| x.trim().parse().ok()).collect()
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
        if t.is_empty() || t.starts_with("%%") { continue; }
        if !past_header {
            if t == "gantt" { past_header = true; }
            continue;
        }
        if t.starts_with("title") {
            // title is ignored at GanttDiagram level (held at TemporalDiagram)
            continue;
        } else if t.starts_with("dateFormat") {
            date_format = t[10..].trim().to_string();
        } else if t.starts_with("section") {
            if let Some(sec) = current_section.take() { sections.push(sec); }
            current_section = Some(GanttSection {
                label: Some(t[7..].trim().to_string()),
                tasks: vec![],
            });
        } else if t.contains(':') {
            if let Some(task) = parse_gantt_task(t) {
                let sec = current_section.get_or_insert_with(|| GanttSection {
                    label: None, tasks: vec![],
                });
                sec.tasks.push(task);
            }
        }
    }
    if let Some(sec) = current_section { sections.push(sec); }

    Ok(GanttDiagram { date_format, sections })
}

/// Parse a single Gantt task line.
///
/// Format: `label :status, id, start, duration`
///    or   `label :id, start, duration`
fn parse_gantt_task(line: &str) -> Option<GanttTask> {
    let colon = line.find(':')?;
    let label = line[..colon].trim().to_string();
    let rest  = line[colon+1..].trim();

    let parts: Vec<&str> = rest.splitn(4, ',').map(str::trim).collect();
    if parts.is_empty() { return None; }

    // Detect status keywords in the first part.
    let status_keywords = ["done", "active", "crit", "milestone"];
    let first = parts[0];
    let (status, remaining) = if status_keywords.iter().any(|&kw| first == kw) {
        (parse_task_status(first), &parts[1..])
    } else {
        (TaskStatus::Normal, &parts[..])
    };

    if remaining.is_empty() { return None; }
    let id    = remaining[0].to_string();
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
    } else { 1.0 };

    Some(GanttTask { id, label, start, duration_days, status, dependencies: vec![] })
}

fn parse_task_status(s: &str) -> TaskStatus {
    match s {
        "done"      => TaskStatus::Done,
        "active"    => TaskStatus::Active,
        "crit"      => TaskStatus::Crit,
        "milestone" => TaskStatus::Milestone,
        _           => TaskStatus::Normal,
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
    use diagram_ir::*;

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
    fn dispatch_flowchart() {
        let src = "flowchart LR\n  A --> B";
        match parse_any_mermaid(src).unwrap() {
            MermaidDiagram::Graph(_) => {},
            _ => panic!("expected Graph"),
        }
    }

    #[test]
    fn dispatch_class_diagram() {
        let src = "classDiagram\n  class Foo";
        match parse_any_mermaid(src).unwrap() {
            MermaidDiagram::Structural(_) => {},
            _ => panic!("expected Structural"),
        }
    }

    #[test]
    fn dispatch_xychart() {
        let src = "xychart-beta\n  bar [1,2,3]";
        match parse_any_mermaid(src).unwrap() {
            MermaidDiagram::Chart(_) => {},
            _ => panic!("expected Chart"),
        }
    }

    #[test]
    fn dispatch_gantt() {
        let src = "gantt\n  dateFormat YYYY-MM-DD";
        match parse_any_mermaid(src).unwrap() {
            MermaidDiagram::Temporal(_) => {},
            _ => panic!("expected Temporal"),
        }
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
        assert_eq!(crate::VERSION, "0.2.0");
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
