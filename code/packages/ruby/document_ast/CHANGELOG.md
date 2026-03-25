# Changelog

All notable changes to `coding_adventures_document_ast` are documented here.

## [0.1.0] - 2026-03-24

### Added

- Initial release.
- All 18 Document AST node types as `Data.define` immutable value objects:
  - Block nodes: `DocumentNode`, `HeadingNode`, `ParagraphNode`, `CodeBlockNode`,
    `BlockquoteNode`, `ListNode`, `ListItemNode`, `ThematicBreakNode`, `RawBlockNode`
  - Inline nodes: `TextNode`, `EmphasisNode`, `StrongNode`, `CodeSpanNode`,
    `LinkNode`, `ImageNode`, `AutolinkNode`, `RawInlineNode`, `HardBreakNode`, `SoftBreakNode`
- Ruby 3.2+ `Data.define` for fully immutable frozen node objects.
- `type` method on all nodes returning the snake_case node type string.
- `children` method on container nodes (always returns a frozen array).
- 51 unit tests; 99%+ line and branch coverage.
- Spec: TE00 — Document AST.
