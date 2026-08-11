# Changelog

## 0.44.0

- Blank leading YAML front matter before sequence grammar parsing while preserving source line positions.

## 0.43.0

- Preprocess Mermaid directives before sequence grammar parsing and apply the global `wrap` directive to default-wrapped semantic labels.

## 0.42.0

- Reject `+` and `-` activation suffixes on central sequence connections, matching the pinned Mermaid grammar alternatives.

## 0.37.0

- Reject explicit and message-suffix deactivation of inactive sequence participants.

## 0.36.0

- Reject duplicate participant IDs in `create` declarations and participants assigned to multiple boxes.

## 0.35.0

- Bind `create` and `destroy` declarations to their required following messages and reject invalid lifecycle sequences.

## 0.34.0

- Give start-only sequence `autonumber` statements their Mermaid 11.16.1 default increment of one.

## 0.33.0

- Parse Mermaid 11.16.1 sequence headers and keywords case-insensitively.

## 0.32.0

- Decode Mermaid 11.16.1 YAML/JSON-schema escapes in quoted sequence participant configuration values.

## 0.31.0

- Preserve commas inside quoted sequence participant configuration aliases.

## 0.30.0

- Accept Mermaid 11.16.1 sequence `#` comments without losing entity-coded text.

## 0.29.0

- Preserve ordered sequence `autonumber`, `autonumber off`, and counter-reset statements.

## 0.28.0

- Preserve `wrap:` and `nowrap:` directives on sequence participant-box labels.

## 0.27.0

- Preserve `wrap:` and `nowrap:` directives on sequence participant aliases.

## 0.26.0

- Preserve `wrap:` and `nowrap:` directives on sequence control-block and branch labels.

## 0.25.0

- Preserve hyphens inside sequence actor identifiers without confusing them with message deactivation markers.

## 0.24.0

- Parse multiword sequence actor identifiers across declarations, messages, notes, lifecycle statements, and metadata commands.

## 0.23.0

- Preserve explicit message and note `wrap:` and `nowrap:` semantics.

## 0.22.0

- Convert sequence `<br>`, `<br/>`, and `<br />` tags to semantic newlines.

## 0.21.0

- Decode Mermaid numeric and HTML named entity codes in sequence text.

## 0.20.0

- Parse legacy colon-prefixed sequence titles.

## 0.19.0

- Parse HSL sequence box and rect colors and normalize them to backend-safe RGB.
- Preserve complete functional colors containing internal whitespace.

## 0.18.0

- Accept semicolon terminators between sequence statements and block contents.

## 0.17.0

- Parse multiline sequence accessibility descriptions.

## 0.16.0

- Parse single-line sequence accessibility titles and descriptions.

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
