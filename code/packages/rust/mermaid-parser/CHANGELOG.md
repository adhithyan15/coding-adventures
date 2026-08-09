# Changelog

## 0.5.0

- Added recursive grammar and semantic lowering for nested sequence control blocks.
- Rejects unterminated blocks instead of silently degrading their contents.

## 0.4.0

- Added grammar-backed sequence parsing for participants, actors, aliases,
  messages, notes, activations, titles, and automatic numbering.
- Added sequence dispatch into the shared semantic IR and marked the family partial.

## 0.3.0

- Pinned the compatibility target to Mermaid 11.16.1.
- Added detection for every documented core diagram family and the external
  ZenUML family.
- Added grammar-backed Pie parsing into the shared chart IR.
- Added a machine-readable compatibility manifest and conformance tests.

## 0.1.0

- Added a grammar-driven Mermaid flowchart parser that lowers into `diagram-ir::GraphDiagram`.
