# Changelog — @coding-adventures/document-ast-sanitizer

## 0.1.0 — 2026-03-24

Initial release.

### Added

- `sanitize(document, policy)` — pure, immutable AST transform
- `SanitizationPolicy` interface with all optional fields (defaults to PASSTHROUGH)
- Three named presets: `STRICT`, `RELAXED`, `PASSTHROUGH`
- Full truth table implementation covering all TE00 node types:
  - `DocumentNode`, `HeadingNode`, `ParagraphNode`, `CodeBlockNode`
  - `BlockquoteNode`, `ListNode`, `ListItemNode`, `ThematicBreakNode`
  - `RawBlockNode` (allowlist-based filtering)
  - `TextNode`, `EmphasisNode`, `StrongNode`, `CodeSpanNode`
  - `LinkNode` (with child promotion when `dropLinks: true`)
  - `ImageNode` (drop / convert to TextNode / URL sanitize)
  - `AutolinkNode` (drop if URL scheme not allowed)
  - `RawInlineNode` (allowlist-based filtering)
  - `HardBreakNode`, `SoftBreakNode`
- URL scheme sanitization:
  - Strips C0 control characters before scheme detection (blocks `java\x00script:`)
  - Strips zero-width Unicode characters (blocks `\u200bjavascript:`)
  - Relative URLs always pass through (no scheme = no risk)
  - Scheme allowlist checked case-insensitively
- Empty children cleanup (drops parent when all children are dropped, except DocumentNode)
- Link promotion when `dropLinks: true` (children promoted to parent, not dropped)
- URL utilities exported: `stripControlChars`, `extractScheme`, `isSchemeAllowed`
- 123 unit tests, 96.78% coverage

### Notes

- TypeScript exhaustiveness checks in all `switch` statements — unknown node types
  cause a compile error, preventing silent pass-through of future node types
- All `_never: never` exhaustiveness branches are unreachable by design
