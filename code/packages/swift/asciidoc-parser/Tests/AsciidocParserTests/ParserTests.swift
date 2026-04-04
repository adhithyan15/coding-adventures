// ============================================================================
// ParserTests.swift — AsciidocParser Test Suite
// ============================================================================
//
// Comprehensive tests for the AsciiDoc parser. Covers all block constructs
// (headings, code blocks, lists, quote blocks, thematic breaks, raw passthrough)
// and all inline constructs (*bold*, _italic_, `code`, link macros, etc.).
//
// # Key AsciiDoc vs CommonMark Difference
//
// In AsciiDoc:  *text* → StrongNode (bold)
// In CommonMark: *text* → EmphasisNode (italic)
//
// All tests in this file verify the AsciiDoc semantics.
//

import XCTest
import DocumentAst
@testable import AsciidocParser

final class ParserTests: XCTestCase {

    // ── Headings ──────────────────────────────────────────────────────────────

    func testDocumentTitle() {
        // `= Title` is a level-1 heading (document title in AsciiDoc)
        let result = parse("= Hello World\n")
        guard case .document(let doc) = result else {
            XCTFail("Expected .document"); return
        }
        XCTAssertEqual(doc.children.count, 1)
        guard case .heading(let h) = doc.children[0] else {
            XCTFail("Expected .heading"); return
        }
        XCTAssertEqual(h.level, 1)
        XCTAssertEqual(h.children, [.text(TextNode(value: "Hello World"))])
    }

    func testHeadingLevel2() {
        let result = parse("== Section\n")
        guard case .document(let doc) = result,
              case .heading(let h) = doc.children[0] else {
            XCTFail("Expected heading"); return
        }
        XCTAssertEqual(h.level, 2)
        XCTAssertEqual(h.children, [.text(TextNode(value: "Section"))])
    }

    func testHeadingLevel3() {
        let result = parse("=== Subsection\n")
        guard case .document(let doc) = result,
              case .heading(let h) = doc.children[0] else {
            XCTFail("Expected heading"); return
        }
        XCTAssertEqual(h.level, 3)
    }

    func testHeadingLevel4() {
        let result = parse("==== L4\n")
        guard case .document(let doc) = result,
              case .heading(let h) = doc.children[0] else {
            XCTFail("Expected heading"); return
        }
        XCTAssertEqual(h.level, 4)
    }

    func testHeadingLevel5() {
        let result = parse("===== L5\n")
        guard case .document(let doc) = result,
              case .heading(let h) = doc.children[0] else {
            XCTFail("Expected heading"); return
        }
        XCTAssertEqual(h.level, 5)
    }

    func testHeadingLevel6() {
        let result = parse("====== L6\n")
        guard case .document(let doc) = result,
              case .heading(let h) = doc.children[0] else {
            XCTFail("Expected heading"); return
        }
        XCTAssertEqual(h.level, 6)
    }

    func testHeadingWithInlineMarkup() {
        // Inline markup in headings should be parsed
        let result = parse("== *Bold* Title\n")
        guard case .document(let doc) = result,
              case .heading(let h) = doc.children[0] else {
            XCTFail("Expected heading"); return
        }
        XCTAssertEqual(h.level, 2)
        // First child should be a StrongNode (AsciiDoc: * = bold)
        guard case .strong(let s) = h.children[0] else {
            XCTFail("Expected .strong in heading, got \(h.children[0])"); return
        }
        XCTAssertEqual(s.children, [.text(TextNode(value: "Bold"))])
        // Second child should be " Title"
        XCTAssertEqual(h.children[1], .text(TextNode(value: " Title")))
    }

    // ── Thematic Break ────────────────────────────────────────────────────────

    func testThematicBreak() {
        let result = parse("'''\n")
        guard case .document(let doc) = result else {
            XCTFail("Expected .document"); return
        }
        XCTAssertEqual(doc.children.count, 1)
        XCTAssertEqual(doc.children[0], .thematicBreak)
    }

    func testThematicBreakFourQuotes() {
        let result = parse("''''\n")
        guard case .document(let doc) = result else {
            XCTFail(); return
        }
        XCTAssertEqual(doc.children[0], .thematicBreak)
    }

    // ── Code Blocks ───────────────────────────────────────────────────────────

    func testCodeBlockWithLanguage() {
        // [source,swift] sets the language; ---- delimiter fences the block
        let input = "[source,swift]\n----\nlet x = 1\n----\n"
        let result = parse(input)
        guard case .document(let doc) = result,
              case .codeBlock(let cb) = doc.children[0] else {
            XCTFail("Expected .codeBlock"); return
        }
        XCTAssertEqual(cb.language, "swift")
        XCTAssertEqual(cb.value, "let x = 1\n")
    }

    func testCodeBlockWithPythonLanguage() {
        let input = "[source,python]\n----\nprint('hello')\n----\n"
        let result = parse(input)
        guard case .document(let doc) = result,
              case .codeBlock(let cb) = doc.children[0] else {
            XCTFail("Expected .codeBlock"); return
        }
        XCTAssertEqual(cb.language, "python")
        XCTAssertEqual(cb.value, "print('hello')\n")
    }

    func testCodeBlockWithoutAttributeHasNilLanguage() {
        // No [source,lang] → language is nil
        let input = "----\nsome code\n----\n"
        let result = parse(input)
        guard case .document(let doc) = result,
              case .codeBlock(let cb) = doc.children[0] else {
            XCTFail("Expected .codeBlock"); return
        }
        XCTAssertNil(cb.language)
        XCTAssertEqual(cb.value, "some code\n")
    }

    func testCodeBlockMultipleLines() {
        let input = "[source,elixir]\n----\ndef hello do\n  :world\nend\n----\n"
        let result = parse(input)
        guard case .document(let doc) = result,
              case .codeBlock(let cb) = doc.children[0] else {
            XCTFail("Expected .codeBlock"); return
        }
        XCTAssertEqual(cb.language, "elixir")
        XCTAssertEqual(cb.value, "def hello do\n  :world\nend\n")
    }

    // ── Literal Block ─────────────────────────────────────────────────────────

    func testLiteralBlock() {
        // `....` fenced block → CodeBlockNode with nil language
        let input = "....\nverbatim content\n....\n"
        let result = parse(input)
        guard case .document(let doc) = result,
              case .codeBlock(let cb) = doc.children[0] else {
            XCTFail("Expected .codeBlock for literal block"); return
        }
        XCTAssertNil(cb.language)
        XCTAssertEqual(cb.value, "verbatim content\n")
    }

    // ── Passthrough Block ─────────────────────────────────────────────────────

    func testPassthroughBlock() {
        // `++++` fenced block → RawBlockNode(format: "html")
        let input = "++++\n<div class=\"note\">Hello</div>\n++++\n"
        let result = parse(input)
        guard case .document(let doc) = result,
              case .rawBlock(let rb) = doc.children[0] else {
            XCTFail("Expected .rawBlock"); return
        }
        XCTAssertEqual(rb.format, "html")
        XCTAssertEqual(rb.value, "<div class=\"note\">Hello</div>\n")
    }

    // ── Quote Block ───────────────────────────────────────────────────────────

    func testQuoteBlock() {
        // `____` fenced block → BlockquoteNode with recursively parsed content
        let input = "____\nWise words here.\n____\n"
        let result = parse(input)
        guard case .document(let doc) = result,
              case .blockquote(let bq) = doc.children[0] else {
            XCTFail("Expected .blockquote"); return
        }
        XCTAssertEqual(bq.children.count, 1)
        guard case .paragraph(let p) = bq.children[0] else {
            XCTFail("Expected paragraph inside blockquote"); return
        }
        XCTAssertEqual(p.children, [.text(TextNode(value: "Wise words here."))])
    }

    func testQuoteBlockWithHeading() {
        let input = "____\n== Inner Heading\n____\n"
        let result = parse(input)
        guard case .document(let doc) = result,
              case .blockquote(let bq) = doc.children[0] else {
            XCTFail("Expected .blockquote"); return
        }
        guard case .heading(let h) = bq.children[0] else {
            XCTFail("Expected heading inside blockquote"); return
        }
        XCTAssertEqual(h.level, 2)
    }

    // ── Unordered List ────────────────────────────────────────────────────────

    func testUnorderedListSingleLevel() {
        let input = "* Alpha\n* Beta\n* Gamma\n"
        let result = parse(input)
        guard case .document(let doc) = result,
              case .list(let list) = doc.children[0] else {
            XCTFail("Expected .list"); return
        }
        XCTAssertFalse(list.ordered)
        XCTAssertEqual(list.children.count, 3)

        let texts = list.children.compactMap { item -> String? in
            guard case .paragraph(let p) = item.children.first,
                  case .text(let t) = p.children.first else { return nil }
            return t.value
        }
        XCTAssertEqual(texts, ["Alpha", "Beta", "Gamma"])
    }

    func testUnorderedListNestedTwoLevels() {
        let input = "* Top\n** Nested\n* Bottom\n"
        let result = parse(input)
        guard case .document(let doc) = result,
              case .list(let list) = doc.children[0] else {
            XCTFail("Expected .list"); return
        }
        XCTAssertFalse(list.ordered)
        // Two top-level items: "Top" (with sub-list) and "Bottom"
        XCTAssertEqual(list.children.count, 2)

        // First item: paragraph + nested list
        let firstItem = list.children[0]
        XCTAssertEqual(firstItem.children.count, 2)
        guard case .list(let subList) = firstItem.children[1] else {
            XCTFail("Expected nested list"); return
        }
        XCTAssertEqual(subList.children.count, 1)

        // Last item: just a paragraph
        let lastItem = list.children[1]
        guard case .paragraph(let p) = lastItem.children[0],
              case .text(let t) = p.children[0] else {
            XCTFail("Expected text paragraph"); return
        }
        XCTAssertEqual(t.value, "Bottom")
    }

    // ── Ordered List ──────────────────────────────────────────────────────────

    func testOrderedList() {
        let input = ". First\n. Second\n. Third\n"
        let result = parse(input)
        guard case .document(let doc) = result,
              case .list(let list) = doc.children[0] else {
            XCTFail("Expected .list"); return
        }
        XCTAssertTrue(list.ordered)
        XCTAssertEqual(list.children.count, 3)

        let texts = list.children.compactMap { item -> String? in
            guard case .paragraph(let p) = item.children.first,
                  case .text(let t) = p.children.first else { return nil }
            return t.value
        }
        XCTAssertEqual(texts, ["First", "Second", "Third"])
    }

    func testOrderedListNested() {
        let input = ". Top\n.. Sub\n. Bottom\n"
        let result = parse(input)
        guard case .document(let doc) = result,
              case .list(let list) = doc.children[0] else {
            XCTFail("Expected .list"); return
        }
        XCTAssertTrue(list.ordered)
        XCTAssertEqual(list.children.count, 2)
        // First item has a nested sub-list
        XCTAssertEqual(list.children[0].children.count, 2)
    }

    // ── Paragraph ─────────────────────────────────────────────────────────────

    func testPlainParagraph() {
        let result = parse("Hello, world.\n")
        guard case .document(let doc) = result,
              case .paragraph(let p) = doc.children[0] else {
            XCTFail("Expected .paragraph"); return
        }
        XCTAssertEqual(p.children, [.text(TextNode(value: "Hello, world."))])
    }

    func testMultipleParagraphs() {
        let input = "First paragraph.\n\nSecond paragraph.\n"
        let result = parse(input)
        guard case .document(let doc) = result else {
            XCTFail("Expected .document"); return
        }
        XCTAssertEqual(doc.children.count, 2)
        guard case .paragraph(let p1) = doc.children[0],
              case .paragraph(let p2) = doc.children[1] else {
            XCTFail("Expected two paragraphs"); return
        }
        XCTAssertEqual(p1.children, [.text(TextNode(value: "First paragraph."))])
        XCTAssertEqual(p2.children, [.text(TextNode(value: "Second paragraph."))])
    }

    // ── Comments ──────────────────────────────────────────────────────────────

    func testCommentLinesSkipped() {
        // `//` lines should be silently discarded
        let input = "// This is a comment\nHello.\n"
        let result = parse(input)
        guard case .document(let doc) = result else {
            XCTFail("Expected .document"); return
        }
        XCTAssertEqual(doc.children.count, 1)
        guard case .paragraph(let p) = doc.children[0] else {
            XCTFail("Expected .paragraph"); return
        }
        XCTAssertEqual(p.children, [.text(TextNode(value: "Hello."))])
    }

    func testMultipleCommentLinesSkipped() {
        let input = "// Comment 1\n// Comment 2\nActual content.\n"
        let result = parse(input)
        guard case .document(let doc) = result else {
            XCTFail(); return
        }
        XCTAssertEqual(doc.children.count, 1)
    }

    // ── Inline: Bold (*) in AsciiDoc ──────────────────────────────────────────

    func testSingleStarIsStrong() {
        // KEY AsciiDoc DIFFERENCE: *text* = StrongNode (NOT EmphasisNode!)
        let nodes = InlineParser.parse("*bold*")
        XCTAssertEqual(nodes.count, 1)
        guard case .strong(let s) = nodes[0] else {
            XCTFail("Expected .strong for *bold*, got \(nodes[0])"); return
        }
        XCTAssertEqual(s.children, [.text(TextNode(value: "bold"))])
    }

    func testDoubleStarIsStrong() {
        let nodes = InlineParser.parse("**also bold**")
        XCTAssertEqual(nodes.count, 1)
        guard case .strong(let s) = nodes[0] else {
            XCTFail("Expected .strong for **also bold**"); return
        }
        XCTAssertEqual(s.children, [.text(TextNode(value: "also bold"))])
    }

    // ── Inline: Italic (_) ────────────────────────────────────────────────────

    func testSingleUnderscoreIsEmphasis() {
        let nodes = InlineParser.parse("_italic_")
        XCTAssertEqual(nodes.count, 1)
        guard case .emphasis(let e) = nodes[0] else {
            XCTFail("Expected .emphasis for _italic_"); return
        }
        XCTAssertEqual(e.children, [.text(TextNode(value: "italic"))])
    }

    func testDoubleUnderscoreIsEmphasis() {
        let nodes = InlineParser.parse("__also italic__")
        XCTAssertEqual(nodes.count, 1)
        guard case .emphasis(let e) = nodes[0] else {
            XCTFail("Expected .emphasis for __also italic__"); return
        }
        XCTAssertEqual(e.children, [.text(TextNode(value: "also italic"))])
    }

    // ── Inline: Code Span ─────────────────────────────────────────────────────

    func testCodeSpan() {
        let nodes = InlineParser.parse("`myFunction()`")
        XCTAssertEqual(nodes.count, 1)
        guard case .codeSpan(let cs) = nodes[0] else {
            XCTFail("Expected .codeSpan"); return
        }
        XCTAssertEqual(cs.value, "myFunction()")
    }

    func testCodeSpanIsVerbatim() {
        // Content inside backticks should NOT be parsed for inline markup
        let nodes = InlineParser.parse("`*not bold*`")
        XCTAssertEqual(nodes.count, 1)
        guard case .codeSpan(let cs) = nodes[0] else {
            XCTFail("Expected .codeSpan"); return
        }
        XCTAssertEqual(cs.value, "*not bold*")
    }

    // ── Inline: Link Macro ────────────────────────────────────────────────────

    func testLinkMacro() {
        let nodes = InlineParser.parse("link:https://example.com[Example]")
        XCTAssertEqual(nodes.count, 1)
        guard case .link(let link) = nodes[0] else {
            XCTFail("Expected .link"); return
        }
        XCTAssertEqual(link.destination, "https://example.com")
        XCTAssertNil(link.title)
        XCTAssertEqual(link.children, [.text(TextNode(value: "Example"))])
    }

    func testLinkMacroWithEmptyLabel() {
        // Empty label → URL is used as display text
        let nodes = InlineParser.parse("link:https://example.com[]")
        XCTAssertEqual(nodes.count, 1)
        guard case .link(let link) = nodes[0] else {
            XCTFail("Expected .link"); return
        }
        XCTAssertEqual(link.destination, "https://example.com")
        XCTAssertEqual(link.children, [.text(TextNode(value: "https://example.com"))])
    }

    // ── Inline: Image Macro ───────────────────────────────────────────────────

    func testImageMacro() {
        let nodes = InlineParser.parse("image:cat.png[A fluffy cat]")
        XCTAssertEqual(nodes.count, 1)
        guard case .image(let img) = nodes[0] else {
            XCTFail("Expected .image"); return
        }
        XCTAssertEqual(img.destination, "cat.png")
        XCTAssertEqual(img.alt, "A fluffy cat")
        XCTAssertNil(img.title)
    }

    // ── Inline: Cross-Reference ───────────────────────────────────────────────

    func testXrefWithDisplayText() {
        let nodes = InlineParser.parse("<<section-1,Section 1>>")
        XCTAssertEqual(nodes.count, 1)
        guard case .link(let link) = nodes[0] else {
            XCTFail("Expected .link for xref"); return
        }
        XCTAssertEqual(link.destination, "#section-1")
        XCTAssertEqual(link.children, [.text(TextNode(value: "Section 1"))])
    }

    func testXrefWithoutDisplayText() {
        let nodes = InlineParser.parse("<<section-2>>")
        XCTAssertEqual(nodes.count, 1)
        guard case .link(let link) = nodes[0] else {
            XCTFail("Expected .link for xref"); return
        }
        XCTAssertEqual(link.destination, "#section-2")
        XCTAssertEqual(link.children, [.text(TextNode(value: "section-2"))])
    }

    // ── Inline: Bare URLs ─────────────────────────────────────────────────────

    func testBareHttpsUrl() {
        // Bare URL without [] → AutolinkNode
        let nodes = InlineParser.parse("https://example.com")
        XCTAssertEqual(nodes.count, 1)
        guard case .autolink(let al) = nodes[0] else {
            XCTFail("Expected .autolink for bare URL"); return
        }
        XCTAssertEqual(al.destination, "https://example.com")
        XCTAssertFalse(al.isEmail)
    }

    func testBareHttpsUrlWithBracketedText() {
        // URL followed by [text] → LinkNode
        let nodes = InlineParser.parse("https://example.com[Visit Example]")
        XCTAssertEqual(nodes.count, 1)
        guard case .link(let link) = nodes[0] else {
            XCTFail("Expected .link for URL with brackets"); return
        }
        XCTAssertEqual(link.destination, "https://example.com")
        XCTAssertEqual(link.children, [.text(TextNode(value: "Visit Example"))])
    }

    func testBareHttpUrl() {
        let nodes = InlineParser.parse("http://example.org")
        XCTAssertEqual(nodes.count, 1)
        guard case .autolink(let al) = nodes[0] else {
            XCTFail("Expected .autolink"); return
        }
        XCTAssertEqual(al.destination, "http://example.org")
    }

    // ── Inline: Soft/Hard Breaks ──────────────────────────────────────────────

    func testSoftBreak() {
        // A single newline in paragraph text → SoftBreakNode
        let nodes = InlineParser.parse("line one\nline two")
        XCTAssertEqual(nodes.count, 3)
        XCTAssertEqual(nodes[0], .text(TextNode(value: "line one")))
        XCTAssertEqual(nodes[1], .softBreak)
        XCTAssertEqual(nodes[2], .text(TextNode(value: "line two")))
    }

    func testHardBreakTwoSpaces() {
        // Two trailing spaces before newline → HardBreakNode
        let nodes = InlineParser.parse("line one  \nline two")
        // Should contain: text("line one"), hardBreak, text("line two")
        let hasHardBreak = nodes.contains { node in
            if case .hardBreak = node { return true }
            return false
        }
        XCTAssertTrue(hasHardBreak, "Expected hardBreak node")
    }

    // ── Mixed Content ─────────────────────────────────────────────────────────

    func testMixedInlineInParagraph() {
        let input = "Use *bold* and _italic_ text.\n"
        let result = parse(input)
        guard case .document(let doc) = result,
              case .paragraph(let p) = doc.children[0] else {
            XCTFail("Expected .paragraph"); return
        }
        // Verify strong and emphasis nodes exist in sequence
        let hasStrong = p.children.contains { n in
            if case .strong = n { return true }; return false
        }
        let hasEmphasis = p.children.contains { n in
            if case .emphasis = n { return true }; return false
        }
        XCTAssertTrue(hasStrong, "Expected .strong node")
        XCTAssertTrue(hasEmphasis, "Expected .emphasis node")
    }

    func testDocumentWithTitleAndParagraph() {
        let input = "= My Title\n\nThis is a paragraph.\n"
        let result = parse(input)
        guard case .document(let doc) = result else {
            XCTFail("Expected .document"); return
        }
        XCTAssertEqual(doc.children.count, 2)
        guard case .heading(let h) = doc.children[0],
              case .paragraph(let p) = doc.children[1] else {
            XCTFail("Expected heading then paragraph"); return
        }
        XCTAssertEqual(h.level, 1)
        XCTAssertEqual(p.children, [.text(TextNode(value: "This is a paragraph."))])
    }

    func testEmptyDocument() {
        let result = parse("")
        guard case .document(let doc) = result else {
            XCTFail("Expected .document"); return
        }
        XCTAssertTrue(doc.children.isEmpty)
    }

    func testBlankLinesOnly() {
        let result = parse("\n\n\n")
        guard case .document(let doc) = result else {
            XCTFail("Expected .document"); return
        }
        XCTAssertTrue(doc.children.isEmpty)
    }
}
