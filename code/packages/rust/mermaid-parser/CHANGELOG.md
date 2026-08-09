# Changelog

## 0.15.0

- Parse sequence actor `details` element IDs into semantic IR.

## 0.14.0

- Parse and merge arbitrary JSON-valued sequence actor properties.

## 0.13.0

- Parse singular and JSON-map sequence actor links into semantic IR.

## 0.12.0

- Parse nested `rect` blocks with required `rgb` or `rgba` fills.

## 0.11.0

- Parse Mermaid 11.15+ autonumber start and increment values.

## 0.10.0

- Parse source, destination, and dual sequence central connections.

## 0.9.0

- Parse normal and reverse filled/stick half arrows into sequence IR.

## 0.8.0

- Parse participant `type` and `alias` configuration with external-alias precedence.

## 0.7.0

- Parse Mermaid sequence `box` declarations into participant-group IR.

## 0.6.0

- Added grammar-backed `create participant`, `create actor`, and `destroy` lowering.

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
