# Changelog — document-ast (Swift)

All notable changes to this package are documented here.

## 0.1.0 — Initial release

- All Document AST block node types: `DocumentNode`, `HeadingNode`, `ParagraphNode`, `CodeBlockNode`, `BlockquoteNode`, `ListNode`, `ListItemNode`, `TaskItemNode`, `ThematicBreakNode`, `RawBlockNode`, `TableNode`, `TableRowNode`, `TableCellNode`
- All Document AST inline node types: `TextNode`, `EmphasisNode`, `StrongNode`, `CodeSpanNode`, `LinkNode`, `ImageNode`, `AutolinkNode`, `RawInlineNode`, `HardBreakNode`, `SoftBreakNode`, `StrikethroughNode`
- `BlockNode` and `InlineNode` discriminated union enums with `Equatable` and `Sendable` conformance
- `TableAlignment` enum (`.left`, `.center`, `.right`)
- Comprehensive test suite covering all node types (95%+ coverage)
