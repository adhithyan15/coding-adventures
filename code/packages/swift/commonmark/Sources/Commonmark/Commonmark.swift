// ============================================================================
// Commonmark.swift — CommonMark → HTML Convenience Pipeline
// ============================================================================
//
// This package is a thin wrapper that chains two independent packages:
//
//   1. CommonmarkParser  — converts Markdown text to a Document AST
//   2. DocumentAstToHtml — converts a Document AST to HTML
//
// Having a shared Document AST IR means the parser and renderer are
// independently testable and replaceable. This package simply connects them.
//
// # Architecture
//
//   CommonMark text
//        │
//        ▼  parse(_:) → DocumentNode via BlockParser + InlineParser
//   BlockNode.document(DocumentNode(...))
//        │
//        ▼  render(_:) → HTML string via DocumentAstToHtml
//   "<h1>Hello</h1>\n<p>World</p>\n"
//
// # Why this package exists
//
// Most users just want `markdownToHtml(text)`. They shouldn't need to know
// about the Document AST, or that the parser and renderer are separate.
// This package provides that one-line API while keeping the internals clean.
//

import CommonmarkParser
import DocumentAstToHtml

/// Convert a CommonMark Markdown string to an HTML string.
///
/// This function chains the full CommonMark parser (supporting ATX headings,
/// thematic breaks, fenced code blocks, blockquotes, lists, emphasis, strong,
/// code spans, links, images, autolinks, hard and soft breaks) with the
/// Document AST HTML renderer.
///
///     toHtml("# Hello")
///     // → "<h1>Hello</h1>\n"
///
///     toHtml("**bold** and *italic*")
///     // → "<p><strong>bold</strong> and <em>italic</em></p>\n"
///
///     toHtml("> blockquote")
///     // → "<blockquote>\n<p>blockquote</p>\n</blockquote>\n"
///
/// - Parameter markdown: A CommonMark-formatted string.
/// - Returns: An HTML string representing the rendered document.
public func toHtml(_ markdown: String) -> String {
    // Phase 1 + 2: Parse Markdown to Document AST
    let doc = parse(markdown)
    // Phase 3: Render Document AST to HTML
    return render(doc)
}
