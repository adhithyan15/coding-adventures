# Changelog — document-ast (Go)

## [0.1.0] — 2026-03-24

### Added
- Initial Go port of the Document AST package (TypeScript → Go).
- All block node types: `DocumentNode`, `HeadingNode`, `ParagraphNode`,
  `CodeBlockNode`, `BlockquoteNode`, `ListNode`, `ListItemNode`,
  `ThematicBreakNode`, `RawBlockNode`.
- All inline node types: `TextNode`, `EmphasisNode`, `StrongNode`,
  `CodeSpanNode`, `LinkNode`, `ImageNode`, `AutolinkNode`, `RawInlineNode`,
  `HardBreakNode`, `SoftBreakNode`.
- `Node`, `BlockNode`, `InlineNode` interfaces using Go's discriminated-union
  pattern (interface + marker methods + type switch).
- Table-driven tests for all node types.
