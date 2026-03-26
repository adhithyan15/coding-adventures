# Changelog — @coding-adventures/document-ast

## [0.1.0] — 2026-03-24

### Added

- Initial release of the Document AST package (spec TE00).
- 19 immutable, discriminated-union node types for structured documents:
  - **Block nodes (9):** `DocumentNode`, `HeadingNode`, `ParagraphNode`,
    `CodeBlockNode`, `BlockquoteNode`, `ListNode`, `ListItemNode`,
    `ThematicBreakNode`, `RawBlockNode`
  - **Inline nodes (10):** `TextNode`, `EmphasisNode`, `StrongNode`,
    `CodeSpanNode`, `LinkNode`, `ImageNode`, `AutolinkNode`, `RawInlineNode`,
    `HardBreakNode`, `SoftBreakNode`
- `BlockNode` and `InlineNode` union types for exhaustive `switch` dispatch.
- `Node` top-level union (BlockNode | InlineNode).
- Types-only package — zero runtime code, zero dependencies.
- Full test suite covering every node type and discriminated-union dispatch.

### Design decisions

- **`RawBlockNode` / `RawInlineNode`** replace the CommonMark-specific
  `HtmlBlockNode` / `HtmlInlineNode`. The `format` field (`"html"`, `"latex"`,
  …) identifies the target renderer, generalising the concept to any back-end.
- **No `LinkDefinitionNode`** — link references are Markdown parse artifacts.
  The IR always contains fully resolved `LinkNode` values; the front-end parser
  is responsible for resolving `[text][label]` before emitting the IR.
- All fields are `readonly` — the IR is immutable after construction.
