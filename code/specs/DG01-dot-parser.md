# DG01 — DOT Parser: Graphviz DOT Subset Parser

## Overview

`dot-parser` converts **Graphviz DOT source text** into a `GraphDiagram`
(DG00). It targets the subset of DOT used by typical diagrams: digraphs with
node and edge declarations, inline attributes, and top-level assignments.

```
DOT source text
  → tokenizer     (hand-written or grammar-driven)
  → AST           (DotDocument — internal, not exported)
  → DotDocument to GraphDiagram lowering
  → GraphDiagram  (DG00)
```

The parser intentionally targets only the features needed for diagram
rendering. Full Graphviz DOT (subgraphs, clusters, ports, compass points,
`graph {}` blocks) is out of scope.

---

## 1. Supported DOT Subset

### 1.1 Document structure

```dot
digraph <id>? { <stmt>* }
strict digraph <id>? { <stmt>* }
```

Only `digraph` is supported. Plain `graph` (undirected) may be parsed as
directed. `strict` is parsed but has no semantic effect.

### 1.2 Statement types

**Node statement** — declares a node with optional attributes:

```dot
A;
A [label="Start", shape=ellipse];
A [label="One"][shape=diamond];   // multiple bracket groups are merged
```

**Edge statement** — declares one or more directed edges in a chain:

```dot
A -> B;
A -> B -> C [label="next"];
```

**Attribute statement** — sets defaults for graph, node, or edge objects:

```dot
node [shape=ellipse, fontcolor=blue];
edge [color=red];
graph [label="My Graph"];
```

**Assignment** — sets a graph-level attribute:

```dot
rankdir=LR;
label="My Graph";
```

### 1.3 Attribute keys (case-insensitive)

| Key | Effect |
|-----|--------|
| `label` | Text label for node or edge |
| `shape` | Node shape: `ellipse`, `circle`, `diamond`, `rect`, `rectangle`, `rounded` |
| `style` | Comma-separated styles; `rounded` maps to `rounded_rect` shape |
| `fillcolor` | Node fill colour (CSS colour string) |
| `color` | Border / edge stroke colour |
| `fontcolor` | Text colour |
| `fontsize` | Font size in points |
| `rankdir` | Graph direction: `TB`, `BT`, `LR`, `RL` |

All other attribute keys are parsed and discarded.

### 1.4 ID tokens

- **Quoted string** — `"hello world"` (double-quoted, `\"` escaping supported)
- **Unquoted identifier** — `[A-Za-z_][A-Za-z0-9_]*`
- **Numeric literal** — `-?[0-9]+(\.[0-9]+)?` treated as a string

### 1.5 Comments

Line comments (`// …` and `# …`) and block comments (`/* … */`) are skipped.

---

## 2. Lowering to GraphDiagram

After parsing, the `DotDocument` AST is lowered to a `GraphDiagram`:

### 2.1 Nodes

- Every node referenced in the document (via a node statement or an edge
  chain) becomes a `GraphNode`.
- If a node is referenced in an edge chain but never explicitly declared, it
  is created with default style.
- A node statement with explicit attributes overrides the defaults established
  by the most recent `node [...]` attribute statement.
- Attributes are resolved in this precedence order (highest wins):
  1. Explicit attributes on the node statement
  2. Node defaults from `node [...]` attribute statements

### 2.2 Edges

- Each `->` in an edge chain produces one `GraphEdge`.
- Edge attributes (from `[...]` after the edge) apply to all edges in that
  chain.

### 2.3 Graph-level attributes

- `rankdir` → `GraphDiagram.direction`
- `label` (from assignment or `graph [label=...]`) → `GraphDiagram.title`

### 2.4 Attribute-to-style mapping

| DOT attribute | DiagramStyle field |
|---------------|--------------------|
| `label` | `GraphNode.label.text` |
| `fillcolor` | `DiagramStyle.fill` |
| `color` | `DiagramStyle.stroke` |
| `fontcolor` | `DiagramStyle.textColor` |
| `fontsize` | `DiagramStyle.fontSize` |
| `shape=ellipse` or `shape=circle` | `DiagramShape::Ellipse` |
| `shape=diamond` | `DiagramShape::Diamond` |
| `shape=rect` or `shape=rectangle` | `DiagramShape::Rect` |
| `style=rounded` or `shape=rounded` | `DiagramShape::RoundedRect` |
| (default) | `DiagramShape::RoundedRect` |

---

## 3. Public API

### `parse_dot(source: &str) -> Result<DotDocument, ParseError>`

Tokenizes and parses DOT source into the internal AST.

### `dot_to_graph_diagram(doc: &DotDocument) -> GraphDiagram`

Lowers a `DotDocument` to a `GraphDiagram` (DG00).

### `parse_dot_to_graph_diagram(source: &str) -> Result<GraphDiagram, ParseError>`

Convenience: parse + lower in one call.

---

## 4. Error Handling

`ParseError` carries a human-readable message and an optional source position
(byte offset). Errors are returned for:

- Invalid token in unexpected position
- Unterminated string literal
- Missing `{` or `}` for the graph body
- `->` with no right-hand side
- Attribute list missing `]`

Warnings (e.g. unrecognised attribute keys, `graph {}` blocks) are silently
ignored.

---

## 5. Parsing Algorithm

The parser is a **hand-written recursive descent** parser. No external
parser-generator or grammar crate is used.

```
parse_document()
  → skip "strict"?
  → expect "digraph"
  → parse_id()?          // optional graph name
  → expect "{"
  → parse_stmt_list()
  → expect "}"

parse_stmt_list()
  → while peek != "}" → parse_stmt() → skip ";"?

parse_stmt()
  → if peek is "node" | "edge" | "graph" → parse_attr_stmt()
  → else if peek is ID and next is "=" → parse_assignment()
  → else if peek is ID and next+1 is "->" → parse_edge_stmt()
  → else → parse_node_stmt()

parse_node_stmt()
  → parse_id()
  → parse_attr_list()?

parse_edge_stmt()
  → parse_id()
  → ("->" parse_id())+
  → parse_attr_list()?

parse_attr_list()
  → ("[" parse_a_list() "]")+   // one or more bracket groups merged

parse_a_list()
  → (parse_id() ("=" parse_id())? ","?)*
```

---

## 6. Tokenizer

The tokenizer is a hand-written character-by-character scanner. Token types:

```
DIGRAPH   — "digraph" (case-insensitive)
STRICT    — "strict"  (case-insensitive)
NODE      — "node"    (case-insensitive)
EDGE      — "edge"    (case-insensitive)
GRAPH     — "graph"   (case-insensitive)
ARROW     — "->"
LBRACE    — "{"
RBRACE    — "}"
LBRACKET  — "["
RBRACKET  — "]"
EQUALS    — "="
COMMA     — ","
SEMICOLON — ";"
ID        — identifier, quoted string, or numeric literal
EOF       — end of input
```

---

## 7. Divergence from Full DOT

Intentionally unsupported:

- Undirected `graph {}` blocks
- Subgraphs and clusters (`subgraph {}`)
- Port notation (`A:n`, `A:se`)
- Compass points in edges
- `HTML-like labels` (`<...>`)
- `\N` and other special escapes in labels
- Node and edge `id` overrides (`node [id=...]`)

If any of these appear in input, the parser silently skips or approximates.
