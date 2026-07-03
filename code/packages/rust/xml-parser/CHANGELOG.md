# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-07-02

### Added

- Initial release of the XML parser crate (OOXML effort, milestone **M1**).
- New parser grammar `code/grammars/xml.grammar` (structural nesting rules for
  documents, elements, attributes, content, comments, CDATA, and processing
  instructions), compiled to `src/_grammar.rs` via the `grammar-tools` CLI.
- `parse_xml(source) -> Result<XmlDocument, ParseError>` — the primary entry
  point, producing a namespace-aware, entity-decoded AST.
- `create_xml_parser(source) -> GrammarParser` — factory returning the raw
  generic parser, mirroring the repo's other `*-parser` crates.
- Typed AST (`XmlDocument`, `XmlElement`, `XmlAttribute`, `XmlNode`,
  `ParseError`) with navigation helpers `get_child`, `get_children`,
  `get_attr`, and `text_content`.
- Namespace resolution via a scope stack: default namespace applies to
  elements (not unprefixed attributes), inner declarations shadow outer ones,
  URIs compared case-sensitively, reserved `xml` / `xmlns` prefixes, and an
  error on unbound prefixes.
- Entity / character-reference decoding for text and attribute values
  (`&amp;`, `&lt;`, `&gt;`, `&apos;`, `&quot;`, `&#N;`, `&#xH;`); CDATA left
  verbatim.
- XML-declaration handling: `version` / `encoding` are lifted onto the
  document rather than stored as a processing-instruction node.
- End-tag / start-tag name matching enforced by the AST builder (a check the
  context-free grammar cannot express).
- **Recursion-depth cap (DoS protection).** Both entry points enable the
  parser's `with_max_depth(DEFAULT_MAX_RULE_DEPTH)` (128). XML nests without
  bound, so a small crafted document (e.g. thousands of nested tags) would
  otherwise recurse past the native stack and *abort the process* — an
  uncatchable stack-overflow DoS directly reachable from `parse_xml` on
  untrusted OOXML. Over-deep input now returns a normal `ParseError`. Covered
  by `test_deeply_nested_input_errors_instead_of_overflowing` (20000 levels).
- 73 unit tests + 1 doc-test covering structure, attributes, namespaces,
  entities, CDATA, comments, PIs, the XML declaration, whitespace handling,
  well-formedness errors, the recursion-depth guard, and two OOXML-flavoured
  integration tests (`[Content_Types].xml` and `.rels`).

### Known limitations

- No DTD support (only the five predefined named entities) — matches the XML
  subset OOXML/OPC uses.
- Whitespace immediately following an entity reference in mixed text is
  dropped by the underlying `xml-lexer` (a lexer-layer behavior, documented in
  the spec and README); it does not affect element-structured OPC part files.
