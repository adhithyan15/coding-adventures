// ============================================================================
// RendererTests.swift — Tests for the Document AST → HTML Renderer
// ============================================================================
//
// These tests verify that every node type renders correctly to HTML.
// Tests are organized by node type to make it easy to find a failing case.
//

import XCTest
@testable import DocumentAstToHtml
import DocumentAst

final class RendererTests: XCTestCase {

    // ── htmlEscape ────────────────────────────────────────────────────────

    func testHtmlEscapeAmpersand() {
        XCTAssertEqual(htmlEscape("a & b"), "a &amp; b")
    }

    func testHtmlEscapeLessThan() {
        XCTAssertEqual(htmlEscape("<tag>"), "&lt;tag&gt;")
    }

    func testHtmlEscapeGreaterThan() {
        XCTAssertEqual(htmlEscape("a > b"), "a &gt; b")
    }

    func testHtmlEscapeQuote() {
        XCTAssertEqual(htmlEscape("say \"hi\""), "say &quot;hi&quot;")
    }

    func testHtmlEscapeMultiple() {
        XCTAssertEqual(htmlEscape("<a href=\"x&y\">"), "&lt;a href=&quot;x&amp;y&quot;&gt;")
    }

    func testHtmlEscapeClean() {
        XCTAssertEqual(htmlEscape("Hello, world!"), "Hello, world!")
    }

    func testHtmlEscapeEmpty() {
        XCTAssertEqual(htmlEscape(""), "")
    }

    // ── Document ──────────────────────────────────────────────────────────

    func testEmptyDocument() {
        let doc = BlockNode.document(DocumentNode(children: []))
        XCTAssertEqual(render(doc), "")
    }

    func testDocumentWithParagraph() {
        let doc = BlockNode.document(DocumentNode(children: [
            .paragraph(ParagraphNode(children: [.text(TextNode(value: "Hello"))]))
        ]))
        XCTAssertEqual(render(doc), "<p>Hello</p>\n")
    }

    func testDocumentWithMultipleBlocks() {
        let doc = BlockNode.document(DocumentNode(children: [
            .heading(HeadingNode(level: 1, children: [.text(TextNode(value: "Title"))])),
            .paragraph(ParagraphNode(children: [.text(TextNode(value: "Body"))]))
        ]))
        XCTAssertEqual(render(doc), "<h1>Title</h1>\n<p>Body</p>\n")
    }

    // ── Headings ──────────────────────────────────────────────────────────

    func testH1() {
        let node = BlockNode.heading(HeadingNode(level: 1, children: [.text(TextNode(value: "Hello"))]))
        XCTAssertEqual(render(node), "<h1>Hello</h1>\n")
    }

    func testH2() {
        let node = BlockNode.heading(HeadingNode(level: 2, children: [.text(TextNode(value: "Section"))]))
        XCTAssertEqual(render(node), "<h2>Section</h2>\n")
    }

    func testH3() {
        let node = BlockNode.heading(HeadingNode(level: 3, children: [.text(TextNode(value: "Sub"))]))
        XCTAssertEqual(render(node), "<h3>Sub</h3>\n")
    }

    func testH6() {
        let node = BlockNode.heading(HeadingNode(level: 6, children: []))
        XCTAssertEqual(render(node), "<h6></h6>\n")
    }

    func testHeadingWithInlines() {
        let node = BlockNode.heading(HeadingNode(level: 1, children: [
            .text(TextNode(value: "Hello ")),
            .emphasis(EmphasisNode(children: [.text(TextNode(value: "world"))]))
        ]))
        XCTAssertEqual(render(node), "<h1>Hello <em>world</em></h1>\n")
    }

    // ── Paragraph ─────────────────────────────────────────────────────────

    func testParagraphPlainText() {
        let node = BlockNode.paragraph(ParagraphNode(children: [.text(TextNode(value: "Hello"))]))
        XCTAssertEqual(render(node), "<p>Hello</p>\n")
    }

    func testParagraphEmpty() {
        let node = BlockNode.paragraph(ParagraphNode(children: []))
        XCTAssertEqual(render(node), "<p></p>\n")
    }

    func testParagraphWithEscaping() {
        let node = BlockNode.paragraph(ParagraphNode(children: [.text(TextNode(value: "a < b & c > d"))]))
        XCTAssertEqual(render(node), "<p>a &lt; b &amp; c &gt; d</p>\n")
    }

    // ── Code Block ────────────────────────────────────────────────────────

    func testCodeBlockWithLanguage() {
        let node = BlockNode.codeBlock(CodeBlockNode(language: "swift", value: "let x = 1\n"))
        XCTAssertEqual(render(node), "<pre><code class=\"language-swift\">let x = 1\n</code></pre>\n")
    }

    func testCodeBlockNoLanguage() {
        let node = BlockNode.codeBlock(CodeBlockNode(language: nil, value: "plain\n"))
        XCTAssertEqual(render(node), "<pre><code>plain\n</code></pre>\n")
    }

    func testCodeBlockEmptyLanguage() {
        let node = BlockNode.codeBlock(CodeBlockNode(language: "", value: "code\n"))
        XCTAssertEqual(render(node), "<pre><code>code\n</code></pre>\n")
    }

    func testCodeBlockMultiWordLanguage() {
        // Only first word of info string is used for class
        let node = BlockNode.codeBlock(CodeBlockNode(language: "python extra", value: "x = 1\n"))
        XCTAssertEqual(render(node), "<pre><code class=\"language-python\">x = 1\n</code></pre>\n")
    }

    func testCodeBlockHtmlEscaping() {
        let node = BlockNode.codeBlock(CodeBlockNode(language: nil, value: "<script>alert('xss')</script>\n"))
        XCTAssertEqual(render(node), "<pre><code>&lt;script&gt;alert('xss')&lt;/script&gt;\n</code></pre>\n")
    }

    // ── Blockquote ────────────────────────────────────────────────────────

    func testBlockquoteEmpty() {
        let node = BlockNode.blockquote(BlockquoteNode(children: []))
        XCTAssertEqual(render(node), "<blockquote>\n</blockquote>\n")
    }

    func testBlockquoteWithParagraph() {
        let node = BlockNode.blockquote(BlockquoteNode(children: [
            .paragraph(ParagraphNode(children: [.text(TextNode(value: "Wise words."))]))
        ]))
        XCTAssertEqual(render(node), "<blockquote>\n<p>Wise words.</p>\n</blockquote>\n")
    }

    func testBlockquoteNested() {
        let inner = BlockNode.blockquote(BlockquoteNode(children: [
            .paragraph(ParagraphNode(children: [.text(TextNode(value: "inner"))]))
        ]))
        let node = BlockNode.blockquote(BlockquoteNode(children: [inner]))
        XCTAssertEqual(render(node), "<blockquote>\n<blockquote>\n<p>inner</p>\n</blockquote>\n</blockquote>\n")
    }

    // ── Unordered List ────────────────────────────────────────────────────

    func testUnorderedListLoose() {
        let item1 = ListItemNode(children: [.paragraph(ParagraphNode(children: [.text(TextNode(value: "A"))]))])
        let item2 = ListItemNode(children: [.paragraph(ParagraphNode(children: [.text(TextNode(value: "B"))]))])
        let node = BlockNode.list(ListNode(ordered: false, start: nil, tight: false, children: [item1, item2]))
        XCTAssertEqual(render(node), "<ul>\n<li>\n<p>A</p>\n</li>\n<li>\n<p>B</p>\n</li>\n</ul>\n")
    }

    func testUnorderedListTight() {
        let item1 = ListItemNode(children: [.paragraph(ParagraphNode(children: [.text(TextNode(value: "A"))]))])
        let item2 = ListItemNode(children: [.paragraph(ParagraphNode(children: [.text(TextNode(value: "B"))]))])
        let node = BlockNode.list(ListNode(ordered: false, start: nil, tight: true, children: [item1, item2]))
        XCTAssertEqual(render(node), "<ul>\n<li>A</li>\n<li>B</li>\n</ul>\n")
    }

    func testUnorderedListEmptyItem() {
        let item = ListItemNode(children: [])
        let node = BlockNode.list(ListNode(ordered: false, start: nil, tight: true, children: [item]))
        XCTAssertEqual(render(node), "<ul>\n<li></li>\n</ul>\n")
    }

    // ── Ordered List ──────────────────────────────────────────────────────

    func testOrderedListStartOne() {
        let item = ListItemNode(children: [.paragraph(ParagraphNode(children: [.text(TextNode(value: "First"))]))])
        let node = BlockNode.list(ListNode(ordered: true, start: 1, tight: true, children: [item]))
        XCTAssertEqual(render(node), "<ol>\n<li>First</li>\n</ol>\n")
    }

    func testOrderedListStartThree() {
        let item = ListItemNode(children: [.paragraph(ParagraphNode(children: [.text(TextNode(value: "Third"))]))])
        let node = BlockNode.list(ListNode(ordered: true, start: 3, tight: true, children: [item]))
        XCTAssertEqual(render(node), "<ol start=\"3\">\n<li>Third</li>\n</ol>\n")
    }

    func testOrderedListStartNil() {
        // nil start → no start attribute
        let item = ListItemNode(children: [.paragraph(ParagraphNode(children: [.text(TextNode(value: "Item"))]))])
        let node = BlockNode.list(ListNode(ordered: true, start: nil, tight: true, children: [item]))
        XCTAssertEqual(render(node), "<ol>\n<li>Item</li>\n</ol>\n")
    }

    // ── Task Items ────────────────────────────────────────────────────────

    func testTaskItemChecked() {
        let task = TaskItemNode(checked: true, children: [
            .paragraph(ParagraphNode(children: [.text(TextNode(value: "Done"))]))
        ])
        let node = BlockNode.list(ListNode(ordered: false, start: nil, tight: true, children: [
            ListItemNode(children: [.taskItem(task)])
        ]))
        // Task item within a list item — the tight paragraph should inline
        let result = render(node)
        XCTAssertTrue(result.contains("checked"))
        XCTAssertTrue(result.contains("Done"))
    }

    func testTaskItemUnchecked() {
        let task = TaskItemNode(checked: false, children: [])
        let node = BlockNode.taskItem(task)
        let result = render(node)
        XCTAssertTrue(result.contains("checkbox"))
        XCTAssertFalse(result.contains("checked=\"\""))
    }

    // ── Thematic Break ────────────────────────────────────────────────────

    func testThematicBreak() {
        let node: BlockNode = .thematicBreak
        XCTAssertEqual(render(node), "<hr />\n")
    }

    // ── Raw Block ─────────────────────────────────────────────────────────

    func testRawBlockHtml() {
        let node = BlockNode.rawBlock(RawBlockNode(format: "html", value: "<div>raw</div>\n"))
        XCTAssertEqual(render(node), "<div>raw</div>\n")
    }

    func testRawBlockUnknownFormat() {
        let node = BlockNode.rawBlock(RawBlockNode(format: "latex", value: "\\textbf{x}\n"))
        XCTAssertEqual(render(node), "")
    }

    // ── Table ─────────────────────────────────────────────────────────────

    func testTableEmpty() {
        let node = BlockNode.table(TableNode(align: [], children: []))
        XCTAssertEqual(render(node), "<table>\n</table>\n")
    }

    func testTableWithHeaderAndBody() {
        let hCell1 = TableCellNode(children: [.text(TextNode(value: "Name"))])
        let hCell2 = TableCellNode(children: [.text(TextNode(value: "Age"))])
        let headerRow = TableRowNode(isHeader: true, children: [hCell1, hCell2])

        let bCell1 = TableCellNode(children: [.text(TextNode(value: "Alice"))])
        let bCell2 = TableCellNode(children: [.text(TextNode(value: "30"))])
        let bodyRow = TableRowNode(isHeader: false, children: [bCell1, bCell2])

        let node = BlockNode.table(TableNode(align: [.left, .right], children: [headerRow, bodyRow]))
        let expected = """
        <table>
        <thead>
        <tr>
        <th align="left">Name</th>
        <th align="right">Age</th>
        </tr>
        </thead>
        <tbody>
        <tr>
        <td align="left">Alice</td>
        <td align="right">30</td>
        </tr>
        </tbody>
        </table>

        """
        XCTAssertEqual(render(node), expected)
    }

    func testTableNoAlignment() {
        let cell = TableCellNode(children: [.text(TextNode(value: "X"))])
        let row = TableRowNode(isHeader: false, children: [cell])
        let node = BlockNode.table(TableNode(align: [nil], children: [row]))
        let result = render(node)
        XCTAssertTrue(result.contains("<td>X</td>"))
        XCTAssertFalse(result.contains("align"))
    }

    func testTableHeaderOnly() {
        let cell = TableCellNode(children: [.text(TextNode(value: "Header"))])
        let row = TableRowNode(isHeader: true, children: [cell])
        let node = BlockNode.table(TableNode(align: [], children: [row]))
        let result = render(node)
        XCTAssertTrue(result.contains("<thead>"))
        XCTAssertFalse(result.contains("<tbody>"))
    }

    // ── Inline: Text ──────────────────────────────────────────────────────

    func testTextNodeEscaping() {
        let node = BlockNode.paragraph(ParagraphNode(children: [
            .text(TextNode(value: "<b>not bold</b>"))
        ]))
        XCTAssertEqual(render(node), "<p>&lt;b&gt;not bold&lt;/b&gt;</p>\n")
    }

    // ── Inline: Emphasis ──────────────────────────────────────────────────

    func testEmphasis() {
        let node = BlockNode.paragraph(ParagraphNode(children: [
            .emphasis(EmphasisNode(children: [.text(TextNode(value: "italic"))]))
        ]))
        XCTAssertEqual(render(node), "<p><em>italic</em></p>\n")
    }

    // ── Inline: Strong ────────────────────────────────────────────────────

    func testStrong() {
        let node = BlockNode.paragraph(ParagraphNode(children: [
            .strong(StrongNode(children: [.text(TextNode(value: "bold"))]))
        ]))
        XCTAssertEqual(render(node), "<p><strong>bold</strong></p>\n")
    }

    // ── Inline: CodeSpan ──────────────────────────────────────────────────

    func testCodeSpan() {
        let node = BlockNode.paragraph(ParagraphNode(children: [
            .codeSpan(CodeSpanNode(value: "let x = 1"))
        ]))
        XCTAssertEqual(render(node), "<p><code>let x = 1</code></p>\n")
    }

    func testCodeSpanHtmlEscaping() {
        let node = BlockNode.paragraph(ParagraphNode(children: [
            .codeSpan(CodeSpanNode(value: "<b>"))
        ]))
        XCTAssertEqual(render(node), "<p><code>&lt;b&gt;</code></p>\n")
    }

    // ── Inline: Link ──────────────────────────────────────────────────────

    func testLinkNoTitle() {
        let node = BlockNode.paragraph(ParagraphNode(children: [
            .link(LinkNode(destination: "https://example.com", title: nil, children: [
                .text(TextNode(value: "Example"))
            ]))
        ]))
        XCTAssertEqual(render(node), "<p><a href=\"https://example.com\">Example</a></p>\n")
    }

    func testLinkWithTitle() {
        let node = BlockNode.paragraph(ParagraphNode(children: [
            .link(LinkNode(destination: "https://example.com", title: "Example Site", children: [
                .text(TextNode(value: "click"))
            ]))
        ]))
        XCTAssertEqual(render(node), "<p><a href=\"https://example.com\" title=\"Example Site\">click</a></p>\n")
    }

    func testLinkAmpersandInUrl() {
        let node = BlockNode.paragraph(ParagraphNode(children: [
            .link(LinkNode(destination: "https://example.com?a=1&b=2", title: nil, children: [
                .text(TextNode(value: "link"))
            ]))
        ]))
        XCTAssertEqual(render(node), "<p><a href=\"https://example.com?a=1&amp;b=2\">link</a></p>\n")
    }

    // ── Inline: Image ─────────────────────────────────────────────────────

    func testImageNoTitle() {
        let node = BlockNode.paragraph(ParagraphNode(children: [
            .image(ImageNode(destination: "cat.png", title: nil, alt: "a cat"))
        ]))
        XCTAssertEqual(render(node), "<p><img src=\"cat.png\" alt=\"a cat\" /></p>\n")
    }

    func testImageWithTitle() {
        let node = BlockNode.paragraph(ParagraphNode(children: [
            .image(ImageNode(destination: "cat.png", title: "My cat", alt: "cat"))
        ]))
        XCTAssertEqual(render(node), "<p><img src=\"cat.png\" alt=\"cat\" title=\"My cat\" /></p>\n")
    }

    func testImageAltHtmlEscaping() {
        let node = BlockNode.paragraph(ParagraphNode(children: [
            .image(ImageNode(destination: "x.png", title: nil, alt: "a & b < c"))
        ]))
        XCTAssertEqual(render(node), "<p><img src=\"x.png\" alt=\"a &amp; b &lt; c\" /></p>\n")
    }

    // ── Inline: Autolink ──────────────────────────────────────────────────

    func testAutolinkUrl() {
        let node = BlockNode.paragraph(ParagraphNode(children: [
            .autolink(AutolinkNode(destination: "https://example.com", isEmail: false))
        ]))
        XCTAssertEqual(render(node), "<p><a href=\"https://example.com\">https://example.com</a></p>\n")
    }

    func testAutolinkEmail() {
        let node = BlockNode.paragraph(ParagraphNode(children: [
            .autolink(AutolinkNode(destination: "user@example.com", isEmail: true))
        ]))
        XCTAssertEqual(render(node), "<p><a href=\"mailto:user@example.com\">user@example.com</a></p>\n")
    }

    // ── Inline: Raw Inline ────────────────────────────────────────────────

    func testRawInlineHtml() {
        let node = BlockNode.paragraph(ParagraphNode(children: [
            .rawInline(RawInlineNode(format: "html", value: "<kbd>Ctrl</kbd>"))
        ]))
        XCTAssertEqual(render(node), "<p><kbd>Ctrl</kbd></p>\n")
    }

    func testRawInlineUnknownFormat() {
        let node = BlockNode.paragraph(ParagraphNode(children: [
            .rawInline(RawInlineNode(format: "latex", value: "\\textbf{x}"))
        ]))
        XCTAssertEqual(render(node), "<p></p>\n")
    }

    // ── Inline: Breaks ────────────────────────────────────────────────────

    func testHardBreak() {
        let node = BlockNode.paragraph(ParagraphNode(children: [
            .text(TextNode(value: "line 1")),
            .hardBreak,
            .text(TextNode(value: "line 2"))
        ]))
        XCTAssertEqual(render(node), "<p>line 1<br />\nline 2</p>\n")
    }

    func testSoftBreak() {
        let node = BlockNode.paragraph(ParagraphNode(children: [
            .text(TextNode(value: "line 1")),
            .softBreak,
            .text(TextNode(value: "line 2"))
        ]))
        XCTAssertEqual(render(node), "<p>line 1\nline 2</p>\n")
    }

    // ── Inline: Strikethrough ─────────────────────────────────────────────

    func testStrikethrough() {
        let node = BlockNode.paragraph(ParagraphNode(children: [
            .strikethrough(StrikethroughNode(children: [.text(TextNode(value: "deleted"))]))
        ]))
        XCTAssertEqual(render(node), "<p><del>deleted</del></p>\n")
    }

    // ── Nested inline ─────────────────────────────────────────────────────

    func testNestedInlines() {
        // **_bold italic_** — strong wrapping emphasis
        let node = BlockNode.paragraph(ParagraphNode(children: [
            .strong(StrongNode(children: [
                .emphasis(EmphasisNode(children: [.text(TextNode(value: "bold italic"))]))
            ]))
        ]))
        XCTAssertEqual(render(node), "<p><strong><em>bold italic</em></strong></p>\n")
    }

    // ── Tight list with nested block ──────────────────────────────────────

    func testTightListWithNestedList() {
        // Tight list where item contains another list
        let innerItem = ListItemNode(children: [.paragraph(ParagraphNode(children: [.text(TextNode(value: "inner"))]))])
        let innerList = BlockNode.list(ListNode(ordered: false, start: nil, tight: true, children: [innerItem]))

        let outerItem = ListItemNode(children: [
            .paragraph(ParagraphNode(children: [.text(TextNode(value: "outer"))])),
            innerList
        ])
        let outerList = BlockNode.list(ListNode(ordered: false, start: nil, tight: true, children: [outerItem]))
        let result = render(outerList)
        XCTAssertTrue(result.contains("<ul>"))
        XCTAssertTrue(result.contains("outer"))
        XCTAssertTrue(result.contains("inner"))
    }
}
