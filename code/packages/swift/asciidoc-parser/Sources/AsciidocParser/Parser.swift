// ============================================================================
// Parser.swift — AsciiDoc Top-Level Entry Point
// ============================================================================
//
// This is the public-facing API for the AsciiDoc parser. It chains two phases:
//
//   Phase 1 — BlockParser: processes AsciiDoc text line-by-line, producing an
//             intermediate list of block structures (headings, paragraphs, code
//             blocks, lists, quote blocks, thematic breaks, raw passthrough, …).
//
//   Phase 2 — InlineParser: walks each block's raw text content, recognizing
//             character-level markup (*bold*, _italic_, `code`, link macros, …)
//             and converting raw strings into InlineNode arrays.
//
// # AsciiDoc vs. CommonMark Differences
//
// The main semantic difference from CommonMark that affects this parser is:
//
//   | Syntax   | CommonMark           | AsciiDoc            |
//   |----------|----------------------|---------------------|
//   | `*text*` | EmphasisNode (italic)| StrongNode (bold)   |
//   | `_text_` | EmphasisNode (italic)| EmphasisNode (italic)|
//   | `**t**`  | StrongNode (bold)    | StrongNode (bold)   |
//   | `__t__`  | StrongNode (bold)    | EmphasisNode (italic)|
//
// # Example
//
//     let doc = parse("= Hello\n\nWorld\n")
//     // .document(DocumentNode(children: [
//     //   .heading(HeadingNode(level: 1, children: [.text(TextNode(value: "Hello"))])),
//     //   .paragraph(ParagraphNode(children: [.text(TextNode(value: "World"))]))
//     // ]))
//

import DocumentAst

/// Parse AsciiDoc text and return a Document AST node.
///
/// This is the top-level entry point for the AsciiDoc parser. It chains
/// Phase 1 (block parsing via `BlockParser`) with Phase 2 (inline parsing
/// via `InlineParser`), producing a complete `BlockNode.document(...)` tree.
///
///     let doc = parse("= Title\n\nHello *world*\n")
///     // .document(DocumentNode(children: [
///     //   .heading(HeadingNode(level: 1, children: [.text("Title")])),
///     //   .paragraph(ParagraphNode(children: [
///     //     .text("Hello "),
///     //     .strong(StrongNode(children: [.text("world")]))
///     //   ]))
///     // ]))
///
/// - Parameter text: AsciiDoc source text (any line endings: \n, \r\n, \r).
/// - Returns: A `.document(DocumentNode)` BlockNode containing the full AST.
public func parse(_ text: String) -> BlockNode {
    // Phase 1: Parse block structure
    let blocks = BlockParser.parseBlocks(text)
    // Phase 2: Inline parsing is done inside BlockParser during flush
    return .document(DocumentNode(children: blocks))
}
