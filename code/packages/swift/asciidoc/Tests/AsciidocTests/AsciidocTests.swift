// ============================================================================
// AsciidocTests.swift — End-to-End AsciiDoc → HTML Tests
// ============================================================================
//
// These tests exercise the full pipeline: AsciiDoc text → Document AST → HTML.
// They test the `toHtml(_:)` function from the `Asciidoc` package.
//
// # Coverage
//
// - Document title (h1)
// - All heading levels (h2–h6)
// - Thematic break
// - Code block with language
// - Code block without language
// - Literal block
// - Passthrough block (raw HTML)
// - Quote block
// - Unordered list
// - Ordered list
// - Paragraph
// - Multiple paragraphs
// - Bold text (*bold* = <strong> in AsciiDoc!)
// - Italic text (_italic_ = <em>)
// - Inline code span
// - Link macro
// - Image macro
// - Cross-reference
// - Bare URL (autolink)
//

import XCTest
@testable import Asciidoc

final class AsciidocTests: XCTestCase {

    // ── Headings ──────────────────────────────────────────────────────────────

    func testDocumentTitle() {
        XCTAssertEqual(toHtml("= Hello\n"), "<h1>Hello</h1>\n")
    }

    func testHeadingLevel2() {
        XCTAssertEqual(toHtml("== Section\n"), "<h2>Section</h2>\n")
    }

    func testHeadingLevel3() {
        XCTAssertEqual(toHtml("=== Sub\n"), "<h3>Sub</h3>\n")
    }

    func testHeadingLevel4() {
        XCTAssertEqual(toHtml("==== L4\n"), "<h4>L4</h4>\n")
    }

    func testHeadingLevel5() {
        XCTAssertEqual(toHtml("===== L5\n"), "<h5>L5</h5>\n")
    }

    func testHeadingLevel6() {
        XCTAssertEqual(toHtml("====== L6\n"), "<h6>L6</h6>\n")
    }

    // ── Thematic Break ────────────────────────────────────────────────────────

    func testThematicBreak() {
        XCTAssertEqual(toHtml("'''\n"), "<hr />\n")
    }

    // ── Code Blocks ───────────────────────────────────────────────────────────

    func testCodeBlockWithLanguage() {
        let input = "[source,swift]\n----\nlet x = 1\n----\n"
        let expected = "<pre><code class=\"language-swift\">let x = 1\n</code></pre>\n"
        XCTAssertEqual(toHtml(input), expected)
    }

    func testCodeBlockWithoutLanguage() {
        let input = "----\nsome code here\n----\n"
        let expected = "<pre><code>some code here\n</code></pre>\n"
        XCTAssertEqual(toHtml(input), expected)
    }

    func testCodeBlockHTMLEscaping() {
        // The HTML renderer escapes < and > in code block content
        let input = "----\n<script>alert('xss')</script>\n----\n"
        let result = toHtml(input)
        XCTAssertTrue(result.contains("&lt;script&gt;"), "< should be escaped as &lt;")
        XCTAssertFalse(result.contains("<script>"), "Raw <script> should not appear in output")
    }

    // ── Literal Block ─────────────────────────────────────────────────────────

    func testLiteralBlock() {
        let input = "....\nverbatim text\n....\n"
        let expected = "<pre><code>verbatim text\n</code></pre>\n"
        XCTAssertEqual(toHtml(input), expected)
    }

    // ── Passthrough Block ─────────────────────────────────────────────────────

    func testPassthroughBlock() {
        // Raw HTML passthrough: content emitted verbatim
        let input = "++++\n<div class=\"highlight\">Special</div>\n++++\n"
        let result = toHtml(input)
        XCTAssertTrue(result.contains("<div class=\"highlight\">Special</div>"),
                      "Passthrough content should be emitted verbatim")
    }

    // ── Quote Block ───────────────────────────────────────────────────────────

    func testQuoteBlock() {
        let input = "____\nWise words.\n____\n"
        let expected = "<blockquote>\n<p>Wise words.</p>\n</blockquote>\n"
        XCTAssertEqual(toHtml(input), expected)
    }

    // ── Lists ─────────────────────────────────────────────────────────────────

    func testUnorderedList() {
        let input = "* Alpha\n* Beta\n"
        let result = toHtml(input)
        XCTAssertTrue(result.contains("<ul>"), "Expected <ul>")
        XCTAssertTrue(result.contains("<li>Alpha</li>"), "Expected list item Alpha")
        XCTAssertTrue(result.contains("<li>Beta</li>"), "Expected list item Beta")
    }

    func testOrderedList() {
        let input = ". First\n. Second\n"
        let result = toHtml(input)
        XCTAssertTrue(result.contains("<ol>"), "Expected <ol>")
        XCTAssertTrue(result.contains("<li>First</li>"), "Expected list item First")
        XCTAssertTrue(result.contains("<li>Second</li>"), "Expected list item Second")
    }

    // ── Paragraphs ────────────────────────────────────────────────────────────

    func testSingleParagraph() {
        XCTAssertEqual(toHtml("Hello, world.\n"), "<p>Hello, world.</p>\n")
    }

    func testMultipleParagraphs() {
        let input = "First paragraph.\n\nSecond paragraph.\n"
        let expected = "<p>First paragraph.</p>\n<p>Second paragraph.</p>\n"
        XCTAssertEqual(toHtml(input), expected)
    }

    // ── Inline Markup ─────────────────────────────────────────────────────────

    func testBoldText() {
        // AsciiDoc: *text* = <strong>text</strong> (NOT <em>!)
        XCTAssertEqual(toHtml("*bold*\n"), "<p><strong>bold</strong></p>\n")
    }

    func testDoubleBoldText() {
        XCTAssertEqual(toHtml("**bold**\n"), "<p><strong>bold</strong></p>\n")
    }

    func testItalicText() {
        XCTAssertEqual(toHtml("_italic_\n"), "<p><em>italic</em></p>\n")
    }

    func testDoubleItalicText() {
        XCTAssertEqual(toHtml("__italic__\n"), "<p><em>italic</em></p>\n")
    }

    func testInlineCode() {
        XCTAssertEqual(toHtml("`code`\n"), "<p><code>code</code></p>\n")
    }

    func testLinkMacro() {
        let input = "link:https://example.com[Example]\n"
        let expected = "<p><a href=\"https://example.com\">Example</a></p>\n"
        XCTAssertEqual(toHtml(input), expected)
    }

    func testImageMacro() {
        let input = "image:cat.png[A cat]\n"
        let expected = "<p><img src=\"cat.png\" alt=\"A cat\" /></p>\n"
        XCTAssertEqual(toHtml(input), expected)
    }

    func testXrefWithDisplayText() {
        let input = "See <<intro,Introduction>>.\n"
        let result = toHtml(input)
        XCTAssertTrue(result.contains("href=\"#intro\""), "Expected #intro href")
        XCTAssertTrue(result.contains(">Introduction<"), "Expected Introduction link text")
    }

    func testBareUrl() {
        let input = "Visit https://example.com today.\n"
        let result = toHtml(input)
        XCTAssertTrue(result.contains("href=\"https://example.com\""), "Expected autolink href")
        XCTAssertTrue(result.contains("https://example.com"), "Expected URL in output")
    }

    // ── HTML Special Characters ───────────────────────────────────────────────

    func testHTMLEscapingInParagraph() {
        let input = "Use <strong> tags.\n"
        let result = toHtml(input)
        XCTAssertTrue(result.contains("&lt;strong&gt;"),
                      "< and > should be HTML-escaped in paragraphs")
    }

    // ── Empty Input ───────────────────────────────────────────────────────────

    func testEmptyInput() {
        XCTAssertEqual(toHtml(""), "")
    }
}
