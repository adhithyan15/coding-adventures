# Changelog

All notable changes to this project will be documented in this file.

## [0.1.1] - 2026-08-07

### Fixed

- **Relocated the misplaced `xml.grammar` source file** from
  `code/grammars/xml.grammar` to `code/grammars/xml/xml.grammar`, matching
  the convention every other grammar directory in the repo follows (one
  `.tokens`/`.grammar` pair per subdirectory). `src/_grammar.rs` regenerates
  byte-identical from the relocated file, confirming it's the same source
  that originally produced it.
- **Consolidated `xml.tokens`/`xml_lua.tokens`/`xml_rust.tokens` into one
  canonical `xml.tokens`.** The three-way fork existed only because the old
  `COMMENT_TEXT`/`CDATA_TEXT`/`PI_TEXT` patterns used regex lookaround,
  which Rust's `regex` crate and Lua's pattern engine can't run — every
  other grammar in the repo instead keeps a single lookaround-free source
  (see `q.tokens`, `dartmouth_basic.tokens`). Rewrote those three patterns
  using the same portable technique already proven in the (now-deleted)
  Lua variant: try the end delimiter first, then a greedy bulk pattern,
  then a single-character fallback aliased back to the bulk token. `CHAR_REF`
  is now two rules (`CHAR_REF_HEX`/`CHAR_REF_DEC`) aliased to `CHAR_REF`
  instead of one rule with a top-level `A|B` alternation, for the same
  portability reason. `xml.grammar`'s `comment`/`cdata`/`pi` rules changed
  from `[ X_TEXT ]` (zero-or-one) to `{ X_TEXT }` (zero-or-more) to match —
  a comment/CDATA/PI body can now surface as several text tokens instead of
  one, and `builder.rs`'s `read_comment`/`read_cdata`/`read_pi` concatenate
  them.
- **Fixed a latent PI-body mis-tokenization bug**, surfaced by the
  consolidation above and present in the source Lua variant before it too:
  in a flat `pi` group holding `PI_TARGET`, `PI_TEXT`, and `PI_QMARK`
  together, a PI body containing a bare `?` followed by more letters (e.g.
  `<?t a?b?>`) had those letters wrongly re-matched as a *second*
  `PI_TARGET` instead of `PI_TEXT`, since `PI_TARGET`'s pattern was still on
  offer for the rest of the body. Fixed by splitting into two groups: `pi`
  (entered by `PI_START`, offering only `PI_END`/`PI_TARGET`) and `pi_body`
  (offering `PI_END`/`PI_TEXT`/`PI_QMARK`), with the on-token callback
  swapping from `pi` to `pi_body` the instant `PI_TARGET` matches — see
  `xml.tokens`' pi/pi_body groups for the full rationale. Every one of the
  nine language ports that consume this grammar (Go, Python, TypeScript,
  Ruby, Swift, Elixir, Perl, Lua, plus this Rust crate) needed the same
  group-swap added to their on-token callback and got a regression test for
  it.

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
