# DG01 — Rust DOT Diagram Pipeline

## Overview

This spec defines the Rust port of the DOT diagram pipeline established by DG00.
The TypeScript packages (`dot-parser`, `diagram-ir`, `diagram-layout-graph`,
`diagram-to-paint`) proved the architecture correct. This spec brings an identical
pipeline to Rust so that `paint-metal` — a Rust crate — can render DOT graphs
natively on macOS without serialisation round-trips.

```text
DOT source string
  → dot-lexer          tokenise into Token stream
  → dot-parser         parse into DotDocument AST
                       lower into GraphDiagram (diagram-ir)
  → diagram-layout-graph  topological rank + geometry
  → diagram-to-paint   lower LayoutedGraphDiagram → PaintScene
  → paint-metal        GPU render on Apple Metal
```

Every crate in this pipeline is a pure Rust library with zero unsafe code except
where explicitly required by FFI. All crates compile on all Tier-1 targets;
`diagram-to-paint` depends on `paint-instructions` which is platform-neutral.

---

## Package Plan

### 1. `diagram-ir`

**Path:** `code/packages/rust/diagram-ir/`

Pure type definitions. No rendering, no layout. This is the Rust counterpart of
the TypeScript `@coding-adventures/diagram-ir` package.

#### Types

```rust
pub enum DiagramDirection { Lr, Rl, Tb, Bt }
pub enum DiagramShape { Rect, RoundedRect, Ellipse, Diamond }

pub struct DiagramLabel { pub text: String }

pub struct DiagramStyle {
    pub fill:         Option<String>,
    pub stroke:       Option<String>,
    pub stroke_width: Option<f64>,
    pub text_color:   Option<String>,
    pub font_size:    Option<f64>,
    pub corner_radius: Option<f64>,
}

pub struct ResolvedDiagramStyle {
    pub fill:         String,
    pub stroke:       String,
    pub stroke_width: f64,
    pub text_color:   String,
    pub font_size:    f64,
    pub corner_radius: f64,
}

pub struct GraphNode {
    pub id:    String,
    pub label: DiagramLabel,
    pub shape: Option<DiagramShape>,
    pub style: Option<DiagramStyle>,
}

pub struct GraphEdge {
    pub id:    Option<String>,
    pub from:  String,      // node id
    pub to:    String,      // node id
    pub label: Option<DiagramLabel>,
    pub kind:  EdgeKind,
    pub style: Option<DiagramStyle>,
}

pub enum EdgeKind { Directed, Undirected }

pub struct GraphDiagram {
    pub direction: DiagramDirection,
    pub title:     Option<String>,
    pub nodes:     Vec<GraphNode>,
    pub edges:     Vec<GraphEdge>,
}

pub struct Point { pub x: f64, pub y: f64 }

pub struct LayoutedGraphNode {
    pub id:     String,
    pub label:  DiagramLabel,
    pub shape:  DiagramShape,
    pub x: f64, pub y: f64,
    pub width: f64, pub height: f64,
    pub style:  ResolvedDiagramStyle,
}

pub struct LayoutedGraphEdge {
    pub id:          Option<String>,
    pub from_node_id: String,
    pub to_node_id:   String,
    pub kind:         EdgeKind,
    pub points:       Vec<Point>,
    pub label:        Option<DiagramLabel>,
    pub label_position: Option<Point>,
    pub style:        ResolvedDiagramStyle,
}

pub struct LayoutedGraphDiagram {
    pub direction: DiagramDirection,
    pub title:     Option<String>,
    pub width:  f64,
    pub height: f64,
    pub nodes:  Vec<LayoutedGraphNode>,
    pub edges:  Vec<LayoutedGraphEdge>,
}
```

#### `resolve_style`

```rust
pub fn resolve_style(style: Option<&DiagramStyle>) -> ResolvedDiagramStyle;
pub fn resolve_style_with_base(
    style: Option<&DiagramStyle>,
    base: ResolvedDiagramStyle,
) -> ResolvedDiagramStyle;
```

Defaults (when `style` is `None` or a field is `None`):

| Field          | Default   |
|----------------|-----------|
| `fill`         | `"#eff6ff"` |
| `stroke`       | `"#2563eb"` |
| `stroke_width` | `2.0`     |
| `text_color`   | `"#1e40af"` |
| `font_size`    | `14.0`    |
| `corner_radius`| `8.0`     |

---

### 2. `dot-lexer`

**Path:** `code/packages/rust/dot-lexer/`

Transforms a DOT source string into a flat `Vec<Token>`. Whitespace and
comments are skipped. Keywords are case-insensitive.

#### Token kinds

```rust
pub enum TokenKind {
    // Keywords
    Strict, Graph, Digraph, Node, Edge, Subgraph,
    // Punctuation
    LBrace, RBrace, LBracket, RBracket,
    Equals, Semicolon, Comma, Colon,
    // Operators
    Arrow,    // ->
    DashDash, // --
    // Identifiers (all flavours stored as plain String)
    Id,
    // End of input
    Eof,
}

pub struct Token {
    pub kind:  TokenKind,
    pub value: String, // raw text; empty for punctuation and keywords
    pub line:  u32,
    pub col:   u32,
}
```

#### ID flavours

The DOT language has four ID syntaxes, all normalised to `TokenKind::Id`:

| Flavour         | Example              | Rule |
|-----------------|----------------------|------|
| Unquoted        | `foo`, `_bar`, `A1`  | `[A-Za-z_\u0080-\u00ff][A-Za-z0-9_\u0080-\u00ff]*` |
| Numeral         | `3.14`, `-42`, `.5`  | `-?(\.[0-9]+\|[0-9]+(\.[0-9]*)?)` |
| Double-quoted   | `"hello world"`      | `"..."` with `\"` and `\\` escapes |
| HTML            | `<b>foo</b>`         | Angle-bracket balanced nesting |

For quoted strings, the lexer strips the surrounding quotes and unescapes `\\"`.
For HTML strings, the lexer strips the outer `<` `>` and returns the inner HTML.

#### Comments

`// …` (line) and `/* … */` (block) are skipped entirely.

#### Error handling

Unknown characters produce a `LexError { message, line, col }`. The lexer
collects all errors and returns them alongside the token stream.

```rust
pub struct LexResult {
    pub tokens: Vec<Token>,
    pub errors: Vec<LexError>,
}

pub fn tokenise(source: &str) -> LexResult;
```

---

### 3. `dot-parser`

**Path:** `code/packages/rust/dot-parser/`

Recursive-descent parser over the `dot-lexer` token stream. Produces two
outputs: the raw DOT AST and the derived `GraphDiagram` (from `diagram-ir`).

#### DOT AST

```rust
pub struct DotDocument {
    pub strict: bool,
    pub id:     Option<String>,
    pub statements: Vec<DotStatement>,
}

pub enum DotStatement {
    Node(DotNodeStmt),
    Edge(DotEdgeStmt),
    Attr(DotAttrStmt),
    Assign { key: String, value: String },
    Subgraph(DotSubgraph),
}

pub struct DotNodeStmt {
    pub id:         String,
    pub attributes: Vec<DotAttribute>,
}

pub struct DotEdgeStmt {
    pub chain:      Vec<String>,      // node IDs in order
    pub directed:   bool,
    pub attributes: Vec<DotAttribute>,
}

pub struct DotAttrStmt {
    pub target:     AttrTarget,
    pub attributes: Vec<DotAttribute>,
}

pub enum AttrTarget { Graph, Node, Edge }

pub struct DotSubgraph {
    pub id:         Option<String>,
    pub statements: Vec<DotStatement>,
}

pub struct DotAttribute {
    pub key:   String,
    pub value: Option<String>,
}
```

#### Public API

```rust
pub struct ParseResult {
    pub document: Option<DotDocument>,
    pub diagram:  Option<GraphDiagram>,
    pub errors:   Vec<ParseError>,
}

/// Parse DOT source into both the raw AST and the semantic GraphDiagram.
pub fn parse(source: &str) -> ParseResult;

/// Parse DOT source directly to a GraphDiagram (most callers use this).
pub fn parse_to_diagram(source: &str) -> Result<GraphDiagram, ParseError>;
```

#### AST → GraphDiagram lowering rules

The lowerer reads `DotAttrStmt` to set global defaults, then applies them
per-node and per-edge from individual attribute lists.

**Direction:** top-level `rankdir` attribute → `DiagramDirection`.
- `LR` → `Lr`, `RL` → `Rl`, `TB` / `TD` → `Tb`, `BT` → `Bt`.
- Default: `Tb`.

**Node shape:** `shape` attribute:
- `box` / `rectangle` / `rect` → `Rect`
- `ellipse` / `circle` / `oval` → `Ellipse`
- `diamond` / `rhombus` → `Diamond`
- absent / `rounded` / `roundbox` → `RoundedRect`

**Node label:** `label` attribute value; falls back to the node id.

**Edge chain expansion:** `A -> B -> C` expands to two edges `A→B` and `B→C`,
each inheriting the same attribute list.

**Edge direction:** `->` edges → `Directed`; `--` edges → `Undirected`.

**De-duplication:** if the same node id appears in multiple `node_stmt` or
as part of `edge_stmt` chains, merge attributes (later declarations win).

---

### 4. `diagram-layout-graph`

**Path:** `code/packages/rust/diagram-layout-graph/`

Ports the TypeScript `layoutGraphDiagram` algorithm to Rust. Depends on
`diagram-ir` and `directed-graph`.

#### Algorithm

```text
1. Build a directed-graph from the GraphDiagram's edges.
2. Topological sort to get an ordered node list.
3. Walk sorted list: rank(n) = max(rank(predecessor)) + 1, or 0 for roots.
4. Assign x, y based on direction (LR/RL use rank as x-axis; TB/BT use y-axis).
   Within a rank, nodes are stacked perpendicular to the rank axis.
5. Compute edge routes: connect node boundary midpoints via a two-point
   polyline (or five-point self-loop route).
6. Compute arrowhead label midpoints.
```

#### Layout constants (defaults)

| Constant          | Default |
|-------------------|---------|
| `margin`          | `24.0`  |
| `rank_gap`        | `96.0`  |
| `node_gap`        | `56.0`  |
| `title_gap`       | `48.0`  |
| `min_node_width`  | `96.0`  |
| `node_height`     | `52.0`  |
| `h_padding`       | `24.0`  |
| `char_width`      | `8.0`   |

#### Public API

```rust
pub struct GraphLayoutOptions {
    pub margin:         Option<f64>,
    pub rank_gap:       Option<f64>,
    pub node_gap:       Option<f64>,
    pub title_gap:      Option<f64>,
    pub min_node_width: Option<f64>,
    pub node_height:    Option<f64>,
    pub h_padding:      Option<f64>,
    pub char_width:     Option<f64>,
}

pub fn layout_graph_diagram(
    diagram: &GraphDiagram,
    options: Option<&GraphLayoutOptions>,
) -> LayoutedGraphDiagram;
```

#### Cycle handling

If `topological_sort()` returns `GraphError::CycleDetected`, fall back to
treating each node as its own rank (flat layout, one node per row/column).

---

### 5. `diagram-to-paint`

**Path:** `code/packages/rust/diagram-to-paint/`

Lowers a `LayoutedGraphDiagram` into a `PaintScene`. Depends on `diagram-ir`
and `paint-instructions`.

#### Node shapes

| Shape         | Instruction         |
|---------------|---------------------|
| `Rect`        | `PaintRect` with `corner_radius = 0` |
| `RoundedRect` | `PaintRect` with `corner_radius` from style |
| `Ellipse`     | `PaintEllipse`      |
| `Diamond`     | `PaintPath` (4-point diamond polygon) |

#### Edge rendering

Each edge becomes:
1. A `PaintPath` polyline (stroke only, `stroke_cap = round`, `stroke_join = round`).
2. For directed edges: a filled triangle `PaintPath` arrowhead computed from
   the final two points of the route.

#### Labels

Node labels and edge labels both become `PaintGlyphRun` instructions using a
`coretext:` font reference scheme so paint-metal can render them via CoreText.

Font reference format: `coretext:<PostScript-name>@<size>` where the
PostScript name is `Helvetica` for the default system sans-serif. This matches
the scheme already implemented in `paint-metal`'s `glyph_run_overlay`.

#### Title

An optional diagram title is rendered as a `PaintGlyphRun` centred at the
top of the scene.

#### Public API

```rust
pub struct DiagramToPaintOptions {
    pub background:       Option<String>,  // default "#ffffff"
    pub ps_font_name:     Option<String>,  // default "Helvetica"
    pub title_font_size:  Option<f64>,     // default 18.0
}

pub fn diagram_to_paint(
    diagram: &LayoutedGraphDiagram,
    options: Option<&DiagramToPaintOptions>,
) -> PaintScene;
```

---

## Implementation Order

1. `diagram-ir` (no deps inside this repo)
2. `dot-lexer` (no deps)
3. `dot-parser` (depends on `dot-lexer`, `diagram-ir`)
4. `diagram-layout-graph` (depends on `diagram-ir`, `directed-graph`)
5. `diagram-to-paint` (depends on `diagram-ir`, `paint-instructions`)

Each package follows the standard repo pattern:
- `Cargo.toml`, `src/lib.rs`, `BUILD`, `README.md`, `CHANGELOG.md`
- ≥95% test coverage via in-module `#[cfg(test)]` blocks
- Literate programming: explain every algorithm choice inline

---

## Divergences from the TypeScript Port

| Area                  | TypeScript                  | Rust                                       |
|-----------------------|-----------------------------|--------------------------------------------|
| Font references       | `canvas:system-ui@14:400`   | `coretext:Helvetica@14` (Metal-native)     |
| Text instruction      | `PaintText`                 | `PaintGlyphRun` (Rust IR has no PaintText) |
| Glyph IDs             | Absent (canvas handles)     | Provided as indices 0..N per character — CoreText resolves them |
| Error handling        | Throws exceptions           | Returns `Result<_, ParseError>`            |
| Cycle in graph        | `CycleError` caught         | `GraphError::CycleDetected` caught         |

The font reference difference is the most important: the Rust pipeline targets
`paint-metal` which uses CoreText directly. The canvas font-ref scheme used in
TypeScript would be meaningless to the Metal backend.
