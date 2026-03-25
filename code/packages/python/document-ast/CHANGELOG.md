# Changelog — coding-adventures-document-ast

## [0.1.0] — 2026-03-24

### Added

- Initial Python port of the TypeScript `@coding-adventures/document-ast` package.
- `DocumentNode`, `HeadingNode`, `ParagraphNode`, `CodeBlockNode`, `BlockquoteNode`,
  `ListNode`, `ListItemNode`, `ThematicBreakNode`, `RawBlockNode` block node types.
- `TextNode`, `EmphasisNode`, `StrongNode`, `CodeSpanNode`, `LinkNode`, `ImageNode`,
  `AutolinkNode`, `RawInlineNode`, `HardBreakNode`, `SoftBreakNode` inline node types.
- `BlockNode`, `InlineNode`, `Node` union type aliases.
- All types implemented as `TypedDict` for JSON-compatible, typed data structures.
- Full `py.typed` marker for PEP 561 inline type declarations.
- Comprehensive test suite with >95% coverage.
