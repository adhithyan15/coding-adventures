//! # dot-parser
//!
//! Recursive-descent parser for the DOT graph description language.
//!
//! Takes the token stream from `dot-lexer` and produces two things:
//!
//! 1. A **raw AST** (`DotDocument`) that faithfully mirrors the DOT source
//!    structure: statements, attributes, subgraphs, and edge chains.
//!
//! 2. A **semantic `GraphDiagram`** (from `diagram-ir`) that is the canonical
//!    input for `diagram-layout-graph`. The lowering step merges attributes,
//!    expands edge chains, and maps DOT concepts to the shared IR.
//!
//! ## Grammar summary
//!
//! ```text
//! graph      := strict? (graph | digraph) id? '{' stmt_list '}'
//! stmt_list  := (stmt ';'?)*
//! stmt       := node_stmt | edge_stmt | attr_stmt | assignment | subgraph
//! node_stmt  := node_id attr_list?
//! edge_stmt  := (node_id | subgraph) edgeRHS attr_list?
//! edgeRHS    := edgeop (node_id | subgraph) edgeRHS?
//! edgeop     := '->' | '--'
//! attr_stmt  := (graph | node | edge) attr_list
//! attr_list  := '[' a_list? ']' attr_list?
//! a_list     := id ('=' id)? (';' | ',')? a_list?
//! node_id    := id (':' id)?
//! assignment := id '=' id
//! subgraph   := subgraph id? '{' stmt_list '}'
//! id         := ID | NUMERAL | QUOTED_STRING | HTML_STRING
//! ```
//!
//! ## Lowering rules (DOT AST → GraphDiagram)
//!
//! - Global `rankdir` attribute → `DiagramDirection`
//! - Global `label` attribute → diagram title
//! - Node `shape` attribute → `DiagramShape`
//! - Node `label` attribute → `DiagramLabel` (falls back to node id)
//! - Edge chain `A -> B -> C` → two edges: `A→B`, `B→C`
//! - Nodes referenced only in edges are auto-created with defaults

pub const VERSION: &str = "0.1.0";

use dot_lexer::{tokenise, LexError, Token, TokenKind};
use diagram_ir::{
    DiagramDirection, DiagramLabel, DiagramShape, EdgeKind, GraphDiagram, GraphEdge, GraphNode,
};

// ============================================================================
// DOT AST types
// ============================================================================

/// An attribute key-value pair from a DOT `[...]` attribute list.
///
/// In DOT, a value is optional: `[bold]` is valid (key only).
#[derive(Clone, Debug, PartialEq)]
pub struct DotAttribute {
    pub key: String,
    pub value: Option<String>,
}

/// A `node_id attr_list?` statement.
#[derive(Clone, Debug, PartialEq)]
pub struct DotNodeStmt {
    pub id: String,
    pub attributes: Vec<DotAttribute>,
}

/// An edge chain statement: one or more source→target pairs sharing attributes.
///
/// `A -> B -> C [color=red]` becomes a single `DotEdgeStmt` with
/// `chain = ["A", "B", "C"]` and the `color` attribute.
#[derive(Clone, Debug, PartialEq)]
pub struct DotEdgeStmt {
    /// Ordered list of node ids in the chain. Length ≥ 2.
    pub chain: Vec<String>,
    /// `true` for `->` (directed); `false` for `--` (undirected).
    pub directed: bool,
    pub attributes: Vec<DotAttribute>,
}

/// `(graph | node | edge) attr_list` — applies attributes to a class of elements.
#[derive(Clone, Debug, PartialEq)]
pub struct DotAttrStmt {
    pub target: AttrTarget,
    pub attributes: Vec<DotAttribute>,
}

/// Which element class an `attr_stmt` applies to.
#[derive(Clone, Debug, PartialEq)]
pub enum AttrTarget {
    Graph,
    Node,
    Edge,
}

/// A named or anonymous subgraph `subgraph id? { stmt_list }`.
///
/// In v1 the parser records subgraph ids but the lowerer flattens them into
/// the parent graph (clusters are not laid out separately yet).
#[derive(Clone, Debug, PartialEq)]
pub struct DotSubgraph {
    pub id: Option<String>,
    pub statements: Vec<DotStatement>,
}

/// Any single statement inside a DOT `{ }` block.
#[derive(Clone, Debug, PartialEq)]
pub enum DotStatement {
    Node(DotNodeStmt),
    Edge(DotEdgeStmt),
    Attr(DotAttrStmt),
    /// Top-level `key = value` assignment (e.g., `rankdir = LR`).
    Assign { key: String, value: String },
    Subgraph(DotSubgraph),
}

/// The root of the parsed DOT AST.
#[derive(Clone, Debug, PartialEq)]
pub struct DotDocument {
    pub strict: bool,
    /// Optional graph name: `digraph MyGraph { … }`.
    pub id: Option<String>,
    pub statements: Vec<DotStatement>,
}

// ============================================================================
// ParseError
// ============================================================================

/// A parse-time error with location information.
#[derive(Clone, Debug, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub line: u32,
    pub col: u32,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.col, self.message)
    }
}

impl std::error::Error for ParseError {}

// ============================================================================
// ParseResult
// ============================================================================

/// The complete result of parsing a DOT source string.
pub struct ParseResult {
    /// The raw DOT AST. `None` if the input was so broken the parser could
    /// not recover a top-level graph structure.
    pub document: Option<DotDocument>,
    /// The semantic `GraphDiagram` derived from the AST by the lowerer.
    /// `None` if lowering failed.
    pub diagram: Option<GraphDiagram>,
    /// All parse errors encountered. Non-empty does not necessarily mean
    /// `document` or `diagram` are absent — the parser recovers where possible.
    pub errors: Vec<ParseError>,
}

// ============================================================================
// Parser — recursive descent
// ============================================================================

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    errors: Vec<ParseError>,
    // Lex errors are promoted to parse errors.
    lex_errors: Vec<LexError>,
}

impl Parser {
    fn new(tokens: Vec<Token>, lex_errors: Vec<LexError>) -> Self {
        Parser { tokens, pos: 0, errors: Vec::new(), lex_errors }
    }

    // ── Token navigation ──────────────────────────────────────────────────────

    fn peek(&self) -> &Token {
        // The lexer always produces a trailing Eof, so this is safe.
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn at(&self, kind: &TokenKind) -> bool {
        &self.peek().kind == kind
    }

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos.min(self.tokens.len() - 1)];
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, kind: &TokenKind) -> Option<String> {
        if &self.peek().kind == kind {
            Some(self.advance().value.clone())
        } else {
            let tok = self.peek();
            self.errors.push(ParseError {
                message: format!(
                    "expected {:?} but found {:?} ({:?})",
                    kind, tok.kind, tok.value
                ),
                line: tok.line,
                col:  tok.col,
            });
            None
        }
    }

    // Skip an optional semicolon between statements.
    fn skip_semicolon(&mut self) {
        if self.at(&TokenKind::Semicolon) {
            self.advance();
        }
    }

    // ── id — any of the four DOT id flavours ──────────────────────────────────

    fn parse_id(&mut self) -> Option<String> {
        if self.at(&TokenKind::Id) {
            Some(self.advance().value.clone())
        } else {
            None
        }
    }

    // ── attr_list: '[' a_list? ']' ────────────────────────────────────────────

    fn parse_attr_list(&mut self) -> Vec<DotAttribute> {
        let mut attrs = Vec::new();
        while self.at(&TokenKind::LBracket) {
            self.advance(); // consume '['
            attrs.extend(self.parse_a_list());
            self.expect(&TokenKind::RBracket);
        }
        attrs
    }

    fn parse_a_list(&mut self) -> Vec<DotAttribute> {
        let mut attrs = Vec::new();
        loop {
            if !self.at(&TokenKind::Id) {
                break;
            }
            let key = self.advance().value.clone();
            let value = if self.at(&TokenKind::Equals) {
                self.advance();
                self.parse_id()
            } else {
                None
            };
            attrs.push(DotAttribute { key, value });
            // Optional separator
            if self.at(&TokenKind::Comma) || self.at(&TokenKind::Semicolon) {
                self.advance();
            }
        }
        attrs
    }

    // ── node_id: id (':' id)? ─────────────────────────────────────────────────
    // Returns the node id (port and compass point are discarded in v1).

    fn parse_node_id(&mut self) -> Option<String> {
        let id = self.parse_id()?;
        // Consume optional port `:port` and/or `:compass_pt`.
        if self.at(&TokenKind::Colon) {
            self.advance();
            self.parse_id(); // port id — discarded
            if self.at(&TokenKind::Colon) {
                self.advance();
                self.parse_id(); // compass point — discarded
            }
        }
        Some(id)
    }

    // ── subgraph: subgraph id? '{' stmt_list '}' ──────────────────────────────

    fn parse_subgraph(&mut self) -> DotSubgraph {
        // `subgraph` keyword already consumed.
        let id = self.parse_id();
        self.expect(&TokenKind::LBrace);
        let stmts = self.parse_stmt_list();
        self.expect(&TokenKind::RBrace);
        DotSubgraph { id, statements: stmts }
    }

    // ── stmt_list ─────────────────────────────────────────────────────────────

    fn parse_stmt_list(&mut self) -> Vec<DotStatement> {
        let mut stmts = Vec::new();
        loop {
            if self.at(&TokenKind::RBrace) || self.at(&TokenKind::Eof) {
                break;
            }
            if let Some(stmt) = self.parse_stmt() {
                stmts.push(stmt);
            }
            self.skip_semicolon();
        }
        stmts
    }

    // ── stmt ─────────────────────────────────────────────────────────────────

    fn parse_stmt(&mut self) -> Option<DotStatement> {
        let tok = self.peek();

        // attr_stmt: `graph`, `node`, or `edge` followed by `[`
        if tok.kind == TokenKind::Graph || tok.kind == TokenKind::Node || tok.kind == TokenKind::Edge {
            let target = match tok.kind {
                TokenKind::Graph => AttrTarget::Graph,
                TokenKind::Node  => AttrTarget::Node,
                TokenKind::Edge  => AttrTarget::Edge,
                _ => unreachable!(),
            };
            self.advance();
            // Only treat as attr_stmt if immediately followed by `[`.
            if self.at(&TokenKind::LBracket) {
                let attributes = self.parse_attr_list();
                return Some(DotStatement::Attr(DotAttrStmt { target, attributes }));
            } else {
                // It's an ordinary id that happened to be a keyword.
                // Fall through by treating the keyword value as an Id.
                // We need to produce an Id token from the consumed keyword.
                let kw_name = match target {
                    AttrTarget::Graph => "graph",
                    AttrTarget::Node  => "node",
                    AttrTarget::Edge  => "edge",
                };
                return self.parse_stmt_starting_with_id(kw_name.to_string());
            }
        }

        // subgraph
        if tok.kind == TokenKind::Subgraph {
            self.advance();
            let sg = self.parse_subgraph();
            return Some(DotStatement::Subgraph(sg));
        }

        // Everything else starts with an id.
        if let Some(id) = self.parse_id() {
            return self.parse_stmt_starting_with_id(id);
        }

        // Unknown token — report and skip.
        let tok = self.peek().clone();
        self.errors.push(ParseError {
            message: format!("unexpected token {:?} ({:?})", tok.kind, tok.value),
            line: tok.line,
            col:  tok.col,
        });
        self.advance();
        None
    }

    fn parse_stmt_starting_with_id(&mut self, first_id: String) -> Option<DotStatement> {
        // assignment: id '=' id
        if self.at(&TokenKind::Equals) {
            self.advance();
            let value = self.parse_id().unwrap_or_default();
            return Some(DotStatement::Assign { key: first_id, value });
        }

        // edge_stmt: id (edgeop …)+
        if self.at(&TokenKind::Arrow) || self.at(&TokenKind::DashDash) {
            let mut chain = vec![first_id];
            let mut directed = false;
            loop {
                if self.at(&TokenKind::Arrow) {
                    directed = true;
                    self.advance();
                } else if self.at(&TokenKind::DashDash) {
                    self.advance();
                } else {
                    break;
                }
                // The right-hand side of an edge op can be a subgraph.
                if self.at(&TokenKind::Subgraph) || self.at(&TokenKind::LBrace) {
                    if self.at(&TokenKind::Subgraph) {
                        self.advance();
                    }
                    let sg = self.parse_subgraph();
                    // Collect node ids from the subgraph for the chain.
                    let ids = collect_node_ids_from_subgraph(&sg);
                    chain.extend(ids);
                } else if let Some(id) = self.parse_node_id() {
                    chain.push(id);
                } else {
                    break;
                }
            }
            let attributes = self.parse_attr_list();
            return Some(DotStatement::Edge(DotEdgeStmt { chain, directed, attributes }));
        }

        // node_stmt: id attr_list?
        let attributes = self.parse_attr_list();
        Some(DotStatement::Node(DotNodeStmt { id: first_id, attributes }))
    }

    // ── graph top-level ───────────────────────────────────────────────────────

    fn parse_graph(&mut self) -> Option<DotDocument> {
        let strict = if self.at(&TokenKind::Strict) {
            self.advance();
            true
        } else {
            false
        };

        // Require graph or digraph.
        match self.peek().kind {
            TokenKind::Graph | TokenKind::Digraph => { self.advance(); }
            _ => {
                let tok = self.peek().clone();
                self.errors.push(ParseError {
                    message: format!("expected 'graph' or 'digraph', found {:?}", tok.kind),
                    line: tok.line,
                    col: tok.col,
                });
                return None;
            }
        }

        let id = self.parse_id();
        self.expect(&TokenKind::LBrace);
        let statements = self.parse_stmt_list();
        self.expect(&TokenKind::RBrace);

        Some(DotDocument { strict, id, statements })
    }
}

// ── Helper: collect all node ids from a subgraph (for edge chains) ────────────

fn collect_node_ids_from_subgraph(sg: &DotSubgraph) -> Vec<String> {
    let mut ids = Vec::new();
    for stmt in &sg.statements {
        match stmt {
            DotStatement::Node(n) => ids.push(n.id.clone()),
            DotStatement::Edge(e) => {
                if !e.chain.is_empty() {
                    ids.push(e.chain[0].clone());
                }
            }
            DotStatement::Subgraph(sub) => ids.extend(collect_node_ids_from_subgraph(sub)),
            _ => {}
        }
    }
    ids
}

// ============================================================================
// Lowerer — DotDocument → GraphDiagram
// ============================================================================

fn map_direction(s: &str) -> DiagramDirection {
    match s.to_ascii_uppercase().as_str() {
        "LR"          => DiagramDirection::Lr,
        "RL"          => DiagramDirection::Rl,
        "BT"          => DiagramDirection::Bt,
        "TB" | "TD"   => DiagramDirection::Tb,
        _             => DiagramDirection::Tb,
    }
}

fn map_shape(s: &str) -> DiagramShape {
    match s.to_ascii_lowercase().as_str() {
        "box" | "rectangle" | "rect" | "square" => DiagramShape::Rect,
        "ellipse" | "circle" | "oval"           => DiagramShape::Ellipse,
        "diamond" | "rhombus"                   => DiagramShape::Diamond,
        _                                        => DiagramShape::RoundedRect,
    }
}

fn attr_value<'a>(attrs: &'a [DotAttribute], key: &str) -> Option<&'a str> {
    attrs.iter()
        .find(|a| a.key.eq_ignore_ascii_case(key))
        .and_then(|a| a.value.as_deref())
}

/// Lower a `DotDocument` into a `GraphDiagram`.
///
/// The lowering:
/// 1. Reads top-level assignments for `rankdir` (direction) and `label` (title).
/// 2. Reads `node [...]` and `edge [...]` attribute statements for global defaults.
/// 3. Processes each `node_stmt` and `edge_stmt` in order.
/// 4. Nodes referenced only in edge chains are auto-created with default style.
/// 5. Edge chains `A -> B -> C` expand to individual edges `A→B`, `B→C`.
fn lower(doc: &DotDocument) -> GraphDiagram {
    let mut direction = DiagramDirection::Tb;
    let mut title: Option<String> = None;

    // Global node/edge defaults collected from attr_stmt declarations.
    let mut global_node_attrs: Vec<DotAttribute> = Vec::new();
    let mut _global_edge_attrs: Vec<DotAttribute> = Vec::new();

    // We'll collect nodes and edges into maps keyed by id to handle merging.
    let mut node_order: Vec<String> = Vec::new();
    let mut node_attrs_map: std::collections::HashMap<String, Vec<DotAttribute>> =
        std::collections::HashMap::new();
    let mut edges: Vec<GraphEdge> = Vec::new();

    // Process top-level statements in a single pass.
    process_statements(
        &doc.statements,
        &mut direction,
        &mut title,
        &mut global_node_attrs,
        &mut _global_edge_attrs,
        &mut node_order,
        &mut node_attrs_map,
        &mut edges,
    );

    // Build the final node list from the ordered ids.
    let nodes: Vec<GraphNode> = node_order
        .iter()
        .map(|id| {
            let attrs = node_attrs_map.get(id).cloned().unwrap_or_default();
            let merged = merge_attrs(&global_node_attrs, &attrs);

            let label_text = attr_value(&merged, "label")
                .map(|s| s.to_string())
                .unwrap_or_else(|| id.clone());

            let shape = attr_value(&merged, "shape").map(map_shape);

            GraphNode {
                id: id.clone(),
                label: DiagramLabel::new(label_text),
                shape,
                style: None, // v1: style attributes are not yet mapped to DiagramStyle
            }
        })
        .collect();

    GraphDiagram { direction, title, nodes, edges }
}

/// Recursively process a statement list, modifying the accumulators.
#[allow(clippy::too_many_arguments)]
fn process_statements(
    stmts: &[DotStatement],
    direction: &mut DiagramDirection,
    title: &mut Option<String>,
    global_node_attrs: &mut Vec<DotAttribute>,
    global_edge_attrs: &mut Vec<DotAttribute>,
    node_order: &mut Vec<String>,
    node_attrs_map: &mut std::collections::HashMap<String, Vec<DotAttribute>>,
    edges: &mut Vec<GraphEdge>,
) {
    for stmt in stmts {
        match stmt {
            DotStatement::Assign { key, value } => {
                match key.to_ascii_lowercase().as_str() {
                    "rankdir" => *direction = map_direction(value),
                    "label"   => *title = Some(value.clone()),
                    _         => {}
                }
            }
            DotStatement::Attr(attr_stmt) => match attr_stmt.target {
                AttrTarget::Graph => {
                    if let Some(rd) = attr_value(&attr_stmt.attributes, "rankdir") {
                        *direction = map_direction(rd);
                    }
                    if let Some(lbl) = attr_value(&attr_stmt.attributes, "label") {
                        *title = Some(lbl.to_string());
                    }
                }
                AttrTarget::Node => {
                    global_node_attrs.extend(attr_stmt.attributes.clone());
                }
                AttrTarget::Edge => {
                    global_edge_attrs.extend(attr_stmt.attributes.clone());
                }
            },
            DotStatement::Node(n) => {
                ensure_node(n.id.clone(), node_order, node_attrs_map);
                // Later declarations win: extend (not replace) so callers can
                // override individual keys.
                node_attrs_map
                    .entry(n.id.clone())
                    .or_default()
                    .extend(n.attributes.clone());
            }
            DotStatement::Edge(e) => {
                // Auto-create any nodes referenced only in edge chains.
                for id in &e.chain {
                    ensure_node(id.clone(), node_order, node_attrs_map);
                }
                // Expand chain into individual edges.
                for pair in e.chain.windows(2) {
                    let kind = if e.directed {
                        EdgeKind::Directed
                    } else {
                        EdgeKind::Undirected
                    };
                    let label = attr_value(&e.attributes, "label")
                        .map(|s| DiagramLabel::new(s.to_string()));
                    edges.push(GraphEdge {
                        id: None,
                        from: pair[0].clone(),
                        to: pair[1].clone(),
                        label,
                        kind,
                        style: None,
                    });
                }
            }
            DotStatement::Subgraph(sg) => {
                // Flatten subgraphs into the parent graph for v1.
                process_statements(
                    &sg.statements,
                    direction,
                    title,
                    global_node_attrs,
                    global_edge_attrs,
                    node_order,
                    node_attrs_map,
                    edges,
                );
            }
        }
    }
}

/// Register a node id if it hasn't been seen yet.
fn ensure_node(
    id: String,
    node_order: &mut Vec<String>,
    node_attrs_map: &mut std::collections::HashMap<String, Vec<DotAttribute>>,
) {
    if !node_attrs_map.contains_key(&id) {
        node_order.push(id.clone());
        node_attrs_map.insert(id, Vec::new());
    }
}

/// Merge global defaults with per-element attributes (per-element wins on conflict).
fn merge_attrs(globals: &[DotAttribute], locals: &[DotAttribute]) -> Vec<DotAttribute> {
    let mut merged = globals.to_vec();
    for local in locals {
        if let Some(existing) = merged.iter_mut().find(|a| a.key == local.key) {
            *existing = local.clone();
        } else {
            merged.push(local.clone());
        }
    }
    merged
}

// ============================================================================
// Public API
// ============================================================================

/// Parse a DOT source string into both the raw AST and a semantic `GraphDiagram`.
///
/// Lex and parse errors are collected rather than aborting — partial results
/// are returned alongside any errors.
pub fn parse(source: &str) -> ParseResult {
    let lex = tokenise(source);
    let mut parser = Parser::new(lex.tokens, lex.errors);

    let document = parser.parse_graph();
    let diagram = document.as_ref().map(lower);

    // Promote lex errors into the parse error list.
    let mut errors: Vec<ParseError> = parser
        .lex_errors
        .iter()
        .map(|e| ParseError { message: e.message.clone(), line: e.line, col: e.col })
        .collect();
    errors.extend(parser.errors);

    ParseResult { document, diagram, errors }
}

/// Parse a DOT source string directly to a `GraphDiagram`.
///
/// Returns the first error if parsing fails entirely.
///
/// # Example
///
/// ```rust
/// use dot_parser::parse_to_diagram;
///
/// let diagram = parse_to_diagram("digraph G { A -> B -> C }").unwrap();
/// assert_eq!(diagram.nodes.len(), 3);
/// assert_eq!(diagram.edges.len(), 2);
/// ```
pub fn parse_to_diagram(source: &str) -> Result<GraphDiagram, ParseError> {
    let result = parse(source);
    if let Some(diagram) = result.diagram {
        Ok(diagram)
    } else {
        Err(result.errors.into_iter().next().unwrap_or(ParseError {
            message: "parse failed with no specific error".to_string(),
            line: 1,
            col: 1,
        }))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_ir::{DiagramDirection, DiagramShape, EdgeKind};

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    // ── Simple digraph ────────────────────────────────────────────────────────

    #[test]
    fn parse_minimal_digraph() {
        let d = parse_to_diagram("digraph G { A -> B }").unwrap();
        assert_eq!(d.nodes.len(), 2);
        assert_eq!(d.edges.len(), 1);
        assert_eq!(d.edges[0].from, "A");
        assert_eq!(d.edges[0].to, "B");
        assert_eq!(d.edges[0].kind, EdgeKind::Directed);
    }

    #[test]
    fn parse_edge_chain_expands() {
        let d = parse_to_diagram("digraph G { A -> B -> C }").unwrap();
        assert_eq!(d.nodes.len(), 3);
        assert_eq!(d.edges.len(), 2);
        assert_eq!(d.edges[0].from, "A");
        assert_eq!(d.edges[0].to, "B");
        assert_eq!(d.edges[1].from, "B");
        assert_eq!(d.edges[1].to, "C");
    }

    #[test]
    fn parse_undirected_graph() {
        let d = parse_to_diagram("graph G { A -- B }").unwrap();
        assert_eq!(d.edges[0].kind, EdgeKind::Undirected);
    }

    // ── Attributes ────────────────────────────────────────────────────────────

    #[test]
    fn node_label_attribute() {
        let d = parse_to_diagram(r#"digraph G { A [label="Hello"] }"#).unwrap();
        assert_eq!(d.nodes[0].label.text, "Hello");
    }

    #[test]
    fn node_label_defaults_to_id() {
        let d = parse_to_diagram("digraph G { MyNode }").unwrap();
        assert_eq!(d.nodes[0].label.text, "MyNode");
    }

    #[test]
    fn node_shape_ellipse() {
        let d = parse_to_diagram("digraph G { A [shape=ellipse] }").unwrap();
        assert_eq!(d.nodes[0].shape, Some(DiagramShape::Ellipse));
    }

    #[test]
    fn node_shape_diamond() {
        let d = parse_to_diagram("digraph G { A [shape=diamond] }").unwrap();
        assert_eq!(d.nodes[0].shape, Some(DiagramShape::Diamond));
    }

    #[test]
    fn node_shape_box_maps_to_rect() {
        let d = parse_to_diagram("digraph G { A [shape=box] }").unwrap();
        assert_eq!(d.nodes[0].shape, Some(DiagramShape::Rect));
    }

    // ── Direction ─────────────────────────────────────────────────────────────

    #[test]
    fn rankdir_lr() {
        let d = parse_to_diagram("digraph G { rankdir = LR; A -> B }").unwrap();
        assert_eq!(d.direction, DiagramDirection::Lr);
    }

    #[test]
    fn rankdir_bt() {
        let d = parse_to_diagram("digraph G { rankdir = BT; A -> B }").unwrap();
        assert_eq!(d.direction, DiagramDirection::Bt);
    }

    #[test]
    fn default_direction_is_tb() {
        let d = parse_to_diagram("digraph G { A -> B }").unwrap();
        assert_eq!(d.direction, DiagramDirection::Tb);
    }

    // ── Edge labels ───────────────────────────────────────────────────────────

    #[test]
    fn edge_label() {
        let d = parse_to_diagram(r#"digraph G { A -> B [label="hi"] }"#).unwrap();
        let lbl = d.edges[0].label.as_ref().unwrap();
        assert_eq!(lbl.text, "hi");
    }

    // ── Auto-create nodes from edges ──────────────────────────────────────────

    #[test]
    fn nodes_auto_created_from_edges() {
        let d = parse_to_diagram("digraph G { X -> Y }").unwrap();
        assert_eq!(d.nodes.len(), 2);
        assert!(d.nodes.iter().any(|n| n.id == "X"));
        assert!(d.nodes.iter().any(|n| n.id == "Y"));
    }

    // ── Strict graph ──────────────────────────────────────────────────────────

    #[test]
    fn strict_keyword_parsed() {
        let r = parse("strict digraph G {}");
        let doc = r.document.unwrap();
        assert!(doc.strict);
    }

    // ── Global node attrs ─────────────────────────────────────────────────────

    #[test]
    fn global_node_shape_applies_to_all() {
        let d = parse_to_diagram("digraph G { node [shape=ellipse]; A; B }").unwrap();
        for n in &d.nodes {
            assert_eq!(n.shape, Some(DiagramShape::Ellipse), "node {} should be ellipse", n.id);
        }
    }

    // ── Multiple edges between same nodes ─────────────────────────────────────

    #[test]
    fn multiple_edges_allowed() {
        let d = parse_to_diagram("digraph G { A -> B; A -> B }").unwrap();
        assert_eq!(d.edges.len(), 2);
    }

    // ── Quoted node ids ───────────────────────────────────────────────────────

    #[test]
    fn quoted_node_ids() {
        let d = parse_to_diagram(r#"digraph G { "hello world" -> "foo bar" }"#).unwrap();
        assert_eq!(d.nodes[0].id, "hello world");
        assert_eq!(d.nodes[1].id, "foo bar");
    }

    // ── Subgraph flattening ───────────────────────────────────────────────────

    #[test]
    fn subgraph_nodes_flattened() {
        let d = parse_to_diagram("digraph G { subgraph cluster_0 { A; B } }").unwrap();
        assert_eq!(d.nodes.len(), 2);
    }

    // ── Comments ─────────────────────────────────────────────────────────────

    #[test]
    fn comments_are_ignored() {
        let d = parse_to_diagram("digraph G { // comment\n A -> B /* also comment */ }").unwrap();
        assert_eq!(d.nodes.len(), 2);
    }

    // ── Diagram title ─────────────────────────────────────────────────────────

    #[test]
    fn label_becomes_title() {
        let d = parse_to_diagram(r#"digraph G { label = "My Title"; A -> B }"#).unwrap();
        assert_eq!(d.title.as_deref(), Some("My Title"));
    }

    // ── Raw AST ───────────────────────────────────────────────────────────────

    #[test]
    fn raw_document_accessible() {
        let r = parse("digraph G { A -> B }");
        let doc = r.document.unwrap();
        assert!(!doc.strict);
        assert_eq!(doc.id.as_deref(), Some("G"));
        assert_eq!(doc.statements.len(), 1);
    }
}
