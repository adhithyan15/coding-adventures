# dot-parser

Recursive-descent parser for the [DOT graph description language](https://graphviz.org/doc/info/lang.html).

Parses DOT source into two outputs:

1. A raw **`DotDocument` AST** — faithful mirror of the DOT syntax
2. A semantic **`GraphDiagram`** — the shared diagram IR consumed by `diagram-layout-graph`

## Usage

```rust
use dot_parser::parse_to_diagram;

let diagram = parse_to_diagram(r#"
    digraph Pipeline {
        rankdir = LR
        Fetch -> Parse -> Layout -> Paint
    }
"#).unwrap();

println!("{} nodes, {} edges", diagram.nodes.len(), diagram.edges.len());
// → "4 nodes, 3 edges"
```

## Lowering rules

| DOT construct             | GraphDiagram result                  |
|---------------------------|--------------------------------------|
| `rankdir = LR`            | `DiagramDirection::Lr`               |
| `label = "My Graph"`      | `diagram.title = Some("My Graph")`   |
| `A [shape=ellipse]`       | `GraphNode { shape: Some(Ellipse) }` |
| `A [label="My Node"]`     | `GraphNode { label: DiagramLabel }` |
| `A -> B -> C`             | Two edges: `A→B`, `B→C`             |
| Node referenced in edge   | Auto-created with default style      |

## Spec

[DG01 — Rust DOT Diagram Pipeline](../../../specs/DG01-dot-pipeline-rust.md)
