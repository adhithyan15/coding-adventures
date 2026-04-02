# Changelog — document-ast (Go)

## [0.2.0] — 2026-04-02

### Changed
- Wrapped all `NodeType()` methods on every node type with the Operations
  system (`StartNew[string]`): `DocumentNode`, `HeadingNode`, `ParagraphNode`,
  `CodeBlockNode`, `BlockquoteNode`, `ListNode`, `ListItemNode`, `TaskItemNode`,
  `ThematicBreakNode`, `RawBlockNode`, `TableNode`, `TableRowNode`,
  `TableCellNode`, `TextNode`, `EmphasisNode`, `StrongNode`,
  `StrikethroughNode`, `CodeSpanNode`, `LinkNode`, `ImageNode`, `AutolinkNode`,
  `RawInlineNode`, `HardBreakNode`, `SoftBreakNode`.
- Every public method now has automatic timing, structured logging, and panic
  recovery via the capability-cage Operations infrastructure.
- Public API signatures are unchanged.

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
