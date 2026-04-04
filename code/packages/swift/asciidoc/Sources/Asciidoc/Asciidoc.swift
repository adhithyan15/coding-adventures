// ============================================================================
// Asciidoc.swift — AsciiDoc → HTML Convenience Pipeline
// ============================================================================
//
// This package is a thin wrapper that chains two independent packages:
//
//   1. AsciidocParser    — converts AsciiDoc text to a Document AST
//   2. DocumentAstToHtml — converts a Document AST to HTML
//
// Having a shared Document AST IR means the parser and renderer are
// independently testable and replaceable. This package simply connects them.
//
// # Architecture
//
//   AsciiDoc text
//        │
//        ▼  parse(_:) → DocumentNode via BlockParser + InlineParser
//   BlockNode.document(DocumentNode(...))
//        │
//        ▼  render(_:) → HTML string via DocumentAstToHtml
//   "<h1>Hello</h1>\n<p>World</p>\n"
//
// # Why this package exists
//
// Most users just want `toHtml(text)`. They shouldn't need to know about the
// Document AST, or that the parser and renderer are separate packages.
// This package provides that one-line API while keeping the internals clean.
//
// # AsciiDoc Specifics
//
// AsciiDoc and CommonMark share the same Document AST and HTML renderer.
// The only difference is the parser: AsciiDoc uses `=` for headings,
// `*text*` for bold (not italic!), and additional block delimiters.
//

import AsciidocParser
import DocumentAstToHtml

/// Convert an AsciiDoc string to an HTML string.
///
/// This function chains the full AsciiDoc parser (supporting section headings,
/// thematic breaks, fenced code blocks, literal blocks, passthrough blocks,
/// quote blocks, ordered and unordered lists, bold, italic, code spans, link
/// macros, image macros, cross-references, and bare URLs) with the Document
/// AST HTML renderer.
///
///     toHtml("= Hello")
///     // → "<h1>Hello</h1>\n"
///
///     toHtml("*bold* and _italic_")
///     // → "<p><strong>bold</strong> and <em>italic</em></p>\n"
///
///     toHtml("> blockquote")
///     // Note: AsciiDoc uses ____ for quote blocks, not >
///     // → "<p>&gt; blockquote</p>\n"
///
///     toHtml("____\nquoted\n____")
///     // → "<blockquote>\n<p>quoted</p>\n</blockquote>\n"
///
/// - Parameter text: An AsciiDoc-formatted string.
/// - Returns: An HTML string representing the rendered document.
public func toHtml(_ text: String) -> String {
    // Phase 1 + 2: Parse AsciiDoc to Document AST
    let doc = parse(text)
    // Phase 3: Render Document AST to HTML
    return render(doc)
}
