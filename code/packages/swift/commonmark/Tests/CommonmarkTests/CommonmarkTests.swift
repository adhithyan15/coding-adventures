// ============================================================================
// CommonmarkTests.swift — End-to-End Tests for the Commonmark Package
// ============================================================================
//
// These tests verify the full pipeline: Markdown → Document AST → HTML.
// They test the `toHtml(_:)` function directly.
//

import XCTest
@testable import Commonmark

final class CommonmarkTests: XCTestCase {

    // ── Headings ──────────────────────────────────────────────────────────

    func testH1() {
        XCTAssertEqual(toHtml("# Hello"), "<h1>Hello</h1>\n")
    }

    func testH2() {
        XCTAssertEqual(toHtml("## Section"), "<h2>Section</h2>\n")
    }

    func testH3() {
        XCTAssertEqual(toHtml("### Subsection"), "<h3>Subsection</h3>\n")
    }

    func testH6() {
        XCTAssertEqual(toHtml("###### Deep"), "<h6>Deep</h6>\n")
    }

    // ── Paragraphs ────────────────────────────────────────────────────────

    func testSimpleParagraph() {
        XCTAssertEqual(toHtml("Hello, world."), "<p>Hello, world.</p>\n")
    }

    func testTwoParagraphs() {
        let result = toHtml("First\n\nSecond")
        XCTAssertTrue(result.contains("<p>First</p>"))
        XCTAssertTrue(result.contains("<p>Second</p>"))
    }

    func testHtmlEscapingInParagraph() {
        let result = toHtml("a < b & c > d")
        XCTAssertEqual(result, "<p>a &lt; b &amp; c &gt; d</p>\n")
    }

    // ── Thematic Break ────────────────────────────────────────────────────

    func testThematicBreak() {
        XCTAssertEqual(toHtml("---"), "<hr />\n")
    }

    func testThematicBreakStars() {
        XCTAssertEqual(toHtml("***"), "<hr />\n")
    }

    // ── Code Blocks ───────────────────────────────────────────────────────

    func testFencedCodeBlockWithLang() {
        let result = toHtml("```swift\nlet x = 1\n```")
        XCTAssertEqual(result, "<pre><code class=\"language-swift\">let x = 1\n</code></pre>\n")
    }

    func testFencedCodeBlockNoLang() {
        let result = toHtml("```\nplain code\n```")
        XCTAssertEqual(result, "<pre><code>plain code\n</code></pre>\n")
    }

    // ── Blockquotes ───────────────────────────────────────────────────────

    func testBlockquote() {
        let result = toHtml("> hello")
        XCTAssertTrue(result.contains("<blockquote>"))
        XCTAssertTrue(result.contains("hello"))
        XCTAssertTrue(result.contains("</blockquote>"))
    }

    // ── Lists ─────────────────────────────────────────────────────────────

    func testUnorderedListTight() {
        let result = toHtml("- A\n- B\n- C")
        XCTAssertTrue(result.contains("<ul>"))
        XCTAssertTrue(result.contains("<li>A</li>"))
        XCTAssertTrue(result.contains("<li>B</li>"))
        XCTAssertTrue(result.contains("<li>C</li>"))
        XCTAssertTrue(result.contains("</ul>"))
    }

    func testOrderedList() {
        let result = toHtml("1. First\n2. Second")
        XCTAssertTrue(result.contains("<ol>"))
        XCTAssertTrue(result.contains("</ol>"))
        XCTAssertTrue(result.contains("First"))
        XCTAssertTrue(result.contains("Second"))
    }

    func testOrderedListCustomStart() {
        let result = toHtml("3. Third")
        XCTAssertTrue(result.contains("start=\"3\""))
    }

    // ── Inline: Emphasis and Strong ───────────────────────────────────────

    func testEmphasis() {
        let result = toHtml("*italic*")
        XCTAssertEqual(result, "<p><em>italic</em></p>\n")
    }

    func testStrong() {
        let result = toHtml("**bold**")
        XCTAssertEqual(result, "<p><strong>bold</strong></p>\n")
    }

    func testStrongAndEmphasis() {
        let result = toHtml("**bold** and *italic*")
        XCTAssertTrue(result.contains("<strong>bold</strong>"))
        XCTAssertTrue(result.contains("<em>italic</em>"))
    }

    // ── Inline: Code Span ─────────────────────────────────────────────────

    func testCodeSpan() {
        let result = toHtml("`let x = 1`")
        XCTAssertEqual(result, "<p><code>let x = 1</code></p>\n")
    }

    // ── Inline: Links ─────────────────────────────────────────────────────

    func testLink() {
        let result = toHtml("[Example](https://example.com)")
        XCTAssertEqual(result, "<p><a href=\"https://example.com\">Example</a></p>\n")
    }

    func testLinkWithTitle() {
        let result = toHtml("[click](https://example.com \"My Title\")")
        XCTAssertTrue(result.contains("title=\"My Title\""))
    }

    // ── Inline: Images ────────────────────────────────────────────────────

    func testImage() {
        let result = toHtml("![a cat](cat.png)")
        XCTAssertEqual(result, "<p><img src=\"cat.png\" alt=\"a cat\" /></p>\n")
    }

    // ── Inline: Autolinks ─────────────────────────────────────────────────

    func testAutolinkUrl() {
        let result = toHtml("<https://example.com>")
        XCTAssertTrue(result.contains("href=\"https://example.com\""))
    }

    func testAutolinkEmail() {
        let result = toHtml("<user@example.com>")
        XCTAssertTrue(result.contains("href=\"mailto:user@example.com\""))
    }

    // ── Inline: Breaks ────────────────────────────────────────────────────

    func testHardBreak() {
        let result = toHtml("line 1  \nline 2")
        XCTAssertTrue(result.contains("<br />"))
    }

    func testSoftBreak() {
        let result = toHtml("line 1\nline 2")
        // Soft break becomes \n in the HTML
        XCTAssertTrue(result.contains("line 1"))
        XCTAssertTrue(result.contains("line 2"))
    }

    // ── Inline: Strikethrough ─────────────────────────────────────────────

    func testStrikethrough() {
        let result = toHtml("~~deleted~~")
        XCTAssertEqual(result, "<p><del>deleted</del></p>\n")
    }

    // ── Empty and edge cases ──────────────────────────────────────────────

    func testEmptyString() {
        XCTAssertEqual(toHtml(""), "")
    }

    func testBlankLines() {
        XCTAssertEqual(toHtml("\n\n\n"), "")
    }

    func testComplexDocument() {
        let md = """
        # My Document

        This is a paragraph with **bold** and *italic* text.

        ## List

        - Item one
        - Item two
        - Item three

        ---

        ```swift
        let x = 42
        ```
        """
        let result = toHtml(md)
        XCTAssertTrue(result.contains("<h1>My Document</h1>"))
        XCTAssertTrue(result.contains("<strong>bold</strong>"))
        XCTAssertTrue(result.contains("<em>italic</em>"))
        XCTAssertTrue(result.contains("<h2>List</h2>"))
        XCTAssertTrue(result.contains("<ul>"))
        XCTAssertTrue(result.contains("Item one"))
        XCTAssertTrue(result.contains("<hr />"))
        XCTAssertTrue(result.contains("language-swift"))
        XCTAssertTrue(result.contains("let x = 42"))
    }
}
