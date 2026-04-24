# Changelog — dot-parser

## 0.1.0

Initial release.

- `parse(source) -> ParseResult` — full AST + diagram + errors
- `parse_to_diagram(source) -> Result<GraphDiagram, ParseError>` — convenience entry point
- `DotDocument`, `DotStatement`, `DotNodeStmt`, `DotEdgeStmt`, `DotAttrStmt`, `DotSubgraph`, `DotAttribute` — raw AST types
- Edge chain expansion: `A -> B -> C` → two `GraphEdge` entries
- Attribute lowering: `shape`, `label`, `rankdir` → diagram-ir types
- Global `node [...]` attribute statements propagated to all nodes
- Subgraph flattening into parent graph
- Lex and parse error collection (non-aborting)
