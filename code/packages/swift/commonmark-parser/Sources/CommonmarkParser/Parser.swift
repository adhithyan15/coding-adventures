// ============================================================================
// Parser.swift — Public Entry Point for the CommonMark Parser
// ============================================================================
//
// The public API is a single function: `parse(_ markdown: String) -> BlockNode`.
// It returns a `.document(...)` block node containing the parsed content.
//
// # Two-Phase Architecture
//
// The CommonMark spec uses a two-pass strategy:
//
//   Phase 1 — Block structure (BlockParser):
//     Process lines top-to-bottom. Identify ATX headings, fenced code blocks,
//     blockquotes, lists, thematic breaks, and paragraphs. Raw inline content
//     is stored as strings.
//
//   Phase 2 — Inline parsing (InlineParser):
//     For each container node, parse the stored raw inline strings into inline
//     nodes: emphasis, strong, code spans, links, images, autolinks, breaks.
//
// This separation is necessary because inline parsing (especially emphasis)
// requires seeing the full run of text before deciding how to parse it.
//

import DocumentAst

/// Parse a CommonMark Markdown string and return a Document AST node.
///
/// The returned value is a `.document(...)` `BlockNode` as defined by
/// the `DocumentAst` package.
///
///     let doc = parse("# Hello\n\nWorld")
///     // → .document(DocumentNode(children: [
///     //     .heading(HeadingNode(level: 1, children: [.text(TextNode(value: "Hello"))])),
///     //     .paragraph(ParagraphNode(children: [.text(TextNode(value: "World"))]))
///     //   ]))
///
/// - Parameter markdown: A UTF-8 CommonMark string.
/// - Returns: A `.document(DocumentNode(...))` block node.
public func parse(_ markdown: String) -> BlockNode {
    // Phase 1: Parse block structure from lines.
    let blocks = BlockParser.parseBlocks(markdown)
    // Phase 2: Parse inline content within each block.
    let children = blocks.map { resolveInlines($0) }
    return .document(DocumentNode(children: children))
}

// ── Internal: resolve inline content ─────────────────────────────────────────

/// Recursively resolve raw inline strings into InlineNode trees.
///
/// BlockParser stores paragraph and heading content as raw strings
/// (in intermediate `RawParagraph` / `RawHeading` nodes). This function
/// walks the tree and calls InlineParser on those raw strings.
///
/// Because this is an internal function, it uses an intermediate type
/// (`IntermediateBlock`) rather than `BlockNode`.
func resolveInlines(_ block: IntermediateBlock) -> BlockNode {
    switch block {
    case .heading(let level, let raw):
        let inlines = InlineParser.parse(raw)
        return .heading(HeadingNode(level: level, children: inlines))

    case .paragraph(let raw):
        let inlines = InlineParser.parse(raw)
        return .paragraph(ParagraphNode(children: inlines))

    case .codeBlock(let lang, let value):
        return .codeBlock(CodeBlockNode(language: lang, value: value))

    case .blockquote(let children):
        return .blockquote(BlockquoteNode(children: children.map { resolveInlines($0) }))

    case .list(let ordered, let start, let tight, let items):
        let resolvedItems = items.map { item -> ListItemNode in
            ListItemNode(children: item.map { resolveInlines($0) })
        }
        return .list(ListNode(ordered: ordered, start: start, tight: tight, children: resolvedItems))

    case .thematicBreak:
        return .thematicBreak

    case .rawBlock(let format, let value):
        return .rawBlock(RawBlockNode(format: format, value: value))
    }
}
