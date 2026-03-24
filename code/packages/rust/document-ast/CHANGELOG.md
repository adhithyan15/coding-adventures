# Changelog — document-ast

## 0.1.0 — 2026-03-24

Initial release.

### Added

- `DocumentNode`, `BlockNode`, `InlineNode` types and all their variants
- All block node types: `HeadingNode`, `ParagraphNode`, `CodeBlockNode`, `BlockquoteNode`, `ListNode`, `ListItemNode`, `ThematicBreakNode`, `RawBlockNode`
- All inline node types: `TextNode`, `EmphasisNode`, `StrongNode`, `CodeSpanNode`, `LinkNode`, `ImageNode`, `AutolinkNode`, `RawInlineNode`, `HardBreakNode`, `SoftBreakNode`
- `#[derive(Debug, Clone, PartialEq)]` on all types
- Unit tests and doctests
