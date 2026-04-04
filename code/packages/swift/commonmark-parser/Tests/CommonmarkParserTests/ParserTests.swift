// ============================================================================
// ParserTests.swift — Tests for the CommonMark Parser
// ============================================================================
//
// These tests verify that the parser correctly converts CommonMark Markdown
// into Document AST nodes. Tests are organized by block/inline feature.
//

import XCTest
@testable import CommonmarkParser
import DocumentAst

final class ParserTests: XCTestCase {

    // ── Helper ────────────────────────────────────────────────────────────

    /// Parse markdown and return the document's children.
    private func children(_ markdown: String) -> [BlockNode] {
        if case .document(let doc) = parse(markdown) {
            return doc.children
        }
        XCTFail("Expected document node")
        return []
    }

    // ── Empty document ────────────────────────────────────────────────────

    func testEmptyString() {
        let blocks = children("")
        XCTAssertTrue(blocks.isEmpty)
    }

    func testBlankLines() {
        let blocks = children("\n\n\n")
        XCTAssertTrue(blocks.isEmpty)
    }

    // ── ATX Headings ──────────────────────────────────────────────────────

    func testH1() {
        let blocks = children("# Hello")
        XCTAssertEqual(blocks.count, 1)
        if case .heading(let h) = blocks[0] {
            XCTAssertEqual(h.level, 1)
            if case .text(let t) = h.children.first {
                XCTAssertEqual(t.value, "Hello")
            } else {
                XCTFail("Expected text child")
            }
        } else {
            XCTFail("Expected heading")
        }
    }

    func testH2() {
        let blocks = children("## Section")
        XCTAssertEqual(blocks.count, 1)
        if case .heading(let h) = blocks[0] {
            XCTAssertEqual(h.level, 2)
        } else {
            XCTFail("Expected h2")
        }
    }

    func testH3() {
        let blocks = children("### Sub")
        if case .heading(let h) = blocks[0] {
            XCTAssertEqual(h.level, 3)
        } else { XCTFail() }
    }

    func testH4() {
        let blocks = children("#### H4")
        if case .heading(let h) = blocks[0] {
            XCTAssertEqual(h.level, 4)
        } else { XCTFail() }
    }

    func testH5() {
        let blocks = children("##### H5")
        if case .heading(let h) = blocks[0] {
            XCTAssertEqual(h.level, 5)
        } else { XCTFail() }
    }

    func testH6() {
        let blocks = children("###### H6")
        if case .heading(let h) = blocks[0] {
            XCTAssertEqual(h.level, 6)
        } else { XCTFail() }
    }

    func testH7NotHeading() {
        // 7 # characters is not a valid ATX heading — becomes paragraph
        let blocks = children("####### Not a heading")
        XCTAssertEqual(blocks.count, 1)
        if case .paragraph(_) = blocks[0] {
            // Correct: 7+ hashes become paragraph text
        } else {
            XCTFail("Expected paragraph (7 hashes not valid heading)")
        }
    }

    func testHeadingTrailingHashes() {
        let blocks = children("## Hello ##")
        if case .heading(let h) = blocks[0] {
            XCTAssertEqual(h.level, 2)
            if case .text(let t) = h.children.first {
                XCTAssertEqual(t.value, "Hello")
            } else { XCTFail("Expected text") }
        } else { XCTFail() }
    }

    func testHeadingEmpty() {
        let blocks = children("#")
        if case .heading(let h) = blocks[0] {
            XCTAssertEqual(h.level, 1)
            XCTAssertTrue(h.children.isEmpty)
        } else { XCTFail() }
    }

    // ── Thematic Breaks ───────────────────────────────────────────────────

    func testThematicBreakDashes() {
        let blocks = children("---")
        XCTAssertEqual(blocks.count, 1)
        if case .thematicBreak = blocks[0] { /* pass */ }
        else { XCTFail("Expected thematicBreak") }
    }

    func testThematicBreakStars() {
        let blocks = children("***")
        if case .thematicBreak = blocks[0] { /* pass */ }
        else { XCTFail() }
    }

    func testThematicBreakUnderscores() {
        let blocks = children("___")
        if case .thematicBreak = blocks[0] { /* pass */ }
        else { XCTFail() }
    }

    func testThematicBreakSpaces() {
        let blocks = children("- - -")
        if case .thematicBreak = blocks[0] { /* pass */ }
        else { XCTFail() }
    }

    func testThematicBreakLong() {
        let blocks = children("----------")
        if case .thematicBreak = blocks[0] { /* pass */ }
        else { XCTFail() }
    }

    func testTwoDashesNotThematic() {
        // Only 2 dashes — not a thematic break, becomes paragraph
        let blocks = children("--")
        if case .paragraph(_) = blocks[0] { /* pass */ }
        else { XCTFail("2 dashes should be paragraph") }
    }

    // ── Paragraphs ────────────────────────────────────────────────────────

    func testSingleLineParagraph() {
        let blocks = children("Hello, world.")
        XCTAssertEqual(blocks.count, 1)
        if case .paragraph(let p) = blocks[0] {
            if case .text(let t) = p.children.first {
                XCTAssertEqual(t.value, "Hello, world.")
            } else { XCTFail() }
        } else { XCTFail() }
    }

    func testTwoParagraphs() {
        let blocks = children("First\n\nSecond")
        XCTAssertEqual(blocks.count, 2)
        if case .paragraph(_) = blocks[0] { /* pass */ } else { XCTFail() }
        if case .paragraph(_) = blocks[1] { /* pass */ } else { XCTFail() }
    }

    func testParagraphMultiLine() {
        // Multi-line paragraph (no blank line separator)
        let blocks = children("Line one\nLine two")
        XCTAssertEqual(blocks.count, 1)
        if case .paragraph(let p) = blocks[0] {
            XCTAssertFalse(p.children.isEmpty)
        } else { XCTFail() }
    }

    // ── Fenced Code Blocks ────────────────────────────────────────────────

    func testFencedCodeBlockNoLang() {
        let blocks = children("```\ncode here\n```")
        XCTAssertEqual(blocks.count, 1)
        if case .codeBlock(let cb) = blocks[0] {
            XCTAssertNil(cb.language)
            XCTAssertEqual(cb.value, "code here\n")
        } else { XCTFail("Expected codeBlock") }
    }

    func testFencedCodeBlockWithLang() {
        let blocks = children("```swift\nlet x = 1\n```")
        if case .codeBlock(let cb) = blocks[0] {
            XCTAssertEqual(cb.language, "swift")
            XCTAssertEqual(cb.value, "let x = 1\n")
        } else { XCTFail() }
    }

    func testFencedCodeBlockTildes() {
        let blocks = children("~~~python\nprint('hi')\n~~~")
        if case .codeBlock(let cb) = blocks[0] {
            XCTAssertEqual(cb.language, "python")
        } else { XCTFail() }
    }

    func testFencedCodeBlockMultiLine() {
        let blocks = children("```\nline1\nline2\nline3\n```")
        if case .codeBlock(let cb) = blocks[0] {
            XCTAssertEqual(cb.value, "line1\nline2\nline3\n")
        } else { XCTFail() }
    }

    func testFencedCodeBlockEmpty() {
        let blocks = children("```\n```")
        if case .codeBlock(let cb) = blocks[0] {
            XCTAssertEqual(cb.value, "")
        } else { XCTFail() }
    }

    // ── Indented Code Blocks ──────────────────────────────────────────────

    func testIndentedCodeBlock() {
        let blocks = children("    code line")
        if case .codeBlock(let cb) = blocks[0] {
            XCTAssertNil(cb.language)
            XCTAssertEqual(cb.value, "code line\n")
        } else { XCTFail("Expected codeBlock") }
    }

    // ── Blockquotes ───────────────────────────────────────────────────────

    func testSimpleBlockquote() {
        let blocks = children("> Hello")
        XCTAssertEqual(blocks.count, 1)
        if case .blockquote(let bq) = blocks[0] {
            XCTAssertFalse(bq.children.isEmpty)
            if case .paragraph(_) = bq.children[0] { /* pass */ }
            else { XCTFail("Expected paragraph inside blockquote") }
        } else { XCTFail("Expected blockquote") }
    }

    func testBlockquoteMultiLine() {
        let blocks = children("> Line one\n> Line two")
        if case .blockquote(let bq) = blocks[0] {
            XCTAssertFalse(bq.children.isEmpty)
        } else { XCTFail() }
    }

    func testNestedBlockquote() {
        let blocks = children("> > inner")
        if case .blockquote(let outer) = blocks[0] {
            if case .blockquote(_) = outer.children[0] { /* pass */ }
            else { XCTFail("Expected nested blockquote") }
        } else { XCTFail() }
    }

    // ── Unordered Lists ───────────────────────────────────────────────────

    func testUnorderedListDash() {
        let blocks = children("- Item A\n- Item B")
        XCTAssertEqual(blocks.count, 1)
        if case .list(let list) = blocks[0] {
            XCTAssertFalse(list.ordered)
            XCTAssertEqual(list.children.count, 2)
        } else { XCTFail("Expected list") }
    }

    func testUnorderedListStar() {
        let blocks = children("* First\n* Second")
        if case .list(let list) = blocks[0] {
            XCTAssertFalse(list.ordered)
            XCTAssertEqual(list.children.count, 2)
        } else { XCTFail() }
    }

    func testUnorderedListPlus() {
        let blocks = children("+ One\n+ Two\n+ Three")
        if case .list(let list) = blocks[0] {
            XCTAssertFalse(list.ordered)
            XCTAssertEqual(list.children.count, 3)
        } else { XCTFail() }
    }

    func testTightList() {
        let blocks = children("- A\n- B\n- C")
        if case .list(let list) = blocks[0] {
            XCTAssertTrue(list.tight)
        } else { XCTFail() }
    }

    func testLooseList() {
        let blocks = children("- A\n\n- B")
        if case .list(let list) = blocks[0] {
            XCTAssertFalse(list.tight)
        } else { XCTFail() }
    }

    // ── Ordered Lists ─────────────────────────────────────────────────────

    func testOrderedListDot() {
        let blocks = children("1. First\n2. Second")
        XCTAssertEqual(blocks.count, 1)
        if case .list(let list) = blocks[0] {
            XCTAssertTrue(list.ordered)
            XCTAssertEqual(list.start, 1)
            XCTAssertEqual(list.children.count, 2)
        } else { XCTFail("Expected ordered list") }
    }

    func testOrderedListParen() {
        let blocks = children("1) First\n2) Second")
        if case .list(let list) = blocks[0] {
            XCTAssertTrue(list.ordered)
        } else { XCTFail() }
    }

    func testOrderedListCustomStart() {
        let blocks = children("3. Third item")
        if case .list(let list) = blocks[0] {
            XCTAssertEqual(list.start, 3)
        } else { XCTFail() }
    }

    // ── Mixed document ────────────────────────────────────────────────────

    func testMixedDocument() {
        let md = """
        # Title

        A paragraph.

        - Item A
        - Item B

        ---
        """
        let blocks = children(md)
        XCTAssertGreaterThanOrEqual(blocks.count, 3)
        if case .heading(let h) = blocks[0] {
            XCTAssertEqual(h.level, 1)
        } else { XCTFail("Expected heading first") }
    }

    // ── Inline: Plain Text ────────────────────────────────────────────────

    func testPlainText() {
        let blocks = children("Hello, world!")
        if case .paragraph(let p) = blocks[0] {
            if case .text(let t) = p.children.first {
                XCTAssertEqual(t.value, "Hello, world!")
            } else { XCTFail() }
        } else { XCTFail() }
    }

    // ── Inline: Emphasis ──────────────────────────────────────────────────

    func testEmphasisStar() {
        let blocks = children("*hello*")
        if case .paragraph(let p) = blocks[0] {
            if case .emphasis(let em) = p.children.first {
                if case .text(let t) = em.children.first {
                    XCTAssertEqual(t.value, "hello")
                } else { XCTFail() }
            } else { XCTFail("Expected emphasis") }
        } else { XCTFail() }
    }

    func testEmphasisUnderscore() {
        let blocks = children("_hello_")
        if case .paragraph(let p) = blocks[0] {
            if case .emphasis(_) = p.children.first { /* pass */ }
            else { XCTFail("Expected emphasis") }
        } else { XCTFail() }
    }

    // ── Inline: Strong ────────────────────────────────────────────────────

    func testStrongDoubleStar() {
        let blocks = children("**bold**")
        if case .paragraph(let p) = blocks[0] {
            if case .strong(let s) = p.children.first {
                if case .text(let t) = s.children.first {
                    XCTAssertEqual(t.value, "bold")
                } else { XCTFail() }
            } else { XCTFail("Expected strong") }
        } else { XCTFail() }
    }

    func testStrongDoubleUnderscore() {
        let blocks = children("__bold__")
        if case .paragraph(let p) = blocks[0] {
            if case .strong(_) = p.children.first { /* pass */ }
            else { XCTFail("Expected strong") }
        } else { XCTFail() }
    }

    // ── Inline: Code Span ─────────────────────────────────────────────────

    func testCodeSpan() {
        let blocks = children("`code`")
        if case .paragraph(let p) = blocks[0] {
            if case .codeSpan(let cs) = p.children.first {
                XCTAssertEqual(cs.value, "code")
            } else { XCTFail("Expected codeSpan") }
        } else { XCTFail() }
    }

    func testCodeSpanDoubleBacktick() {
        let blocks = children("``code``")
        if case .paragraph(let p) = blocks[0] {
            if case .codeSpan(let cs) = p.children.first {
                XCTAssertEqual(cs.value, "code")
            } else { XCTFail() }
        } else { XCTFail() }
    }

    // ── Inline: Links ─────────────────────────────────────────────────────

    func testLink() {
        let blocks = children("[click](https://example.com)")
        if case .paragraph(let p) = blocks[0] {
            if case .link(let link) = p.children.first {
                XCTAssertEqual(link.destination, "https://example.com")
                XCTAssertNil(link.title)
            } else { XCTFail("Expected link") }
        } else { XCTFail() }
    }

    func testLinkWithTitle() {
        let blocks = children("[click](https://example.com \"Example\")")
        if case .paragraph(let p) = blocks[0] {
            if case .link(let link) = p.children.first {
                XCTAssertEqual(link.destination, "https://example.com")
                XCTAssertEqual(link.title, "Example")
            } else { XCTFail() }
        } else { XCTFail() }
    }

    func testLinkText() {
        let blocks = children("[**bold**](https://x.com)")
        if case .paragraph(let p) = blocks[0] {
            if case .link(let link) = p.children.first {
                XCTAssertFalse(link.children.isEmpty)
            } else { XCTFail("Expected link") }
        } else { XCTFail() }
    }

    // ── Inline: Images ────────────────────────────────────────────────────

    func testImage() {
        let blocks = children("![a cat](cat.png)")
        if case .paragraph(let p) = blocks[0] {
            if case .image(let img) = p.children.first {
                XCTAssertEqual(img.alt, "a cat")
                XCTAssertEqual(img.destination, "cat.png")
                XCTAssertNil(img.title)
            } else { XCTFail("Expected image") }
        } else { XCTFail() }
    }

    func testImageWithTitle() {
        let blocks = children("![alt](x.png \"My title\")")
        if case .paragraph(let p) = blocks[0] {
            if case .image(let img) = p.children.first {
                XCTAssertEqual(img.title, "My title")
            } else { XCTFail() }
        } else { XCTFail() }
    }

    // ── Inline: Autolinks ─────────────────────────────────────────────────

    func testAutolinkUrl() {
        let blocks = children("<https://example.com>")
        if case .paragraph(let p) = blocks[0] {
            if case .autolink(let al) = p.children.first {
                XCTAssertEqual(al.destination, "https://example.com")
                XCTAssertFalse(al.isEmail)
            } else { XCTFail("Expected autolink") }
        } else { XCTFail() }
    }

    func testAutolinkEmail() {
        let blocks = children("<user@example.com>")
        if case .paragraph(let p) = blocks[0] {
            if case .autolink(let al) = p.children.first {
                XCTAssertEqual(al.destination, "user@example.com")
                XCTAssertTrue(al.isEmail)
            } else { XCTFail("Expected email autolink") }
        } else { XCTFail() }
    }

    // ── Inline: Hard Break ────────────────────────────────────────────────

    func testHardBreakTrailingSpaces() {
        let blocks = children("line 1  \nline 2")
        if case .paragraph(let p) = blocks[0] {
            let hasHardBreak = p.children.contains { node in
                if case .hardBreak = node { return true }
                return false
            }
            XCTAssertTrue(hasHardBreak, "Expected hard break from trailing spaces")
        } else { XCTFail() }
    }

    // ── Inline: Soft Break ────────────────────────────────────────────────

    func testSoftBreak() {
        let blocks = children("line 1\nline 2")
        if case .paragraph(let p) = blocks[0] {
            let hasSoftBreak = p.children.contains { node in
                if case .softBreak = node { return true }
                return false
            }
            XCTAssertTrue(hasSoftBreak, "Expected soft break from newline in paragraph")
        } else { XCTFail() }
    }

    // ── Inline: Strikethrough ─────────────────────────────────────────────

    func testStrikethrough() {
        let blocks = children("~~deleted~~")
        if case .paragraph(let p) = blocks[0] {
            if case .strikethrough(let s) = p.children.first {
                if case .text(let t) = s.children.first {
                    XCTAssertEqual(t.value, "deleted")
                } else { XCTFail() }
            } else { XCTFail("Expected strikethrough") }
        } else { XCTFail() }
    }

    // ── Inline: Mixed ─────────────────────────────────────────────────────

    func testMixedInlines() {
        let blocks = children("Hello **world** and *earth*.")
        if case .paragraph(let p) = blocks[0] {
            XCTAssertGreaterThan(p.children.count, 1)
        } else { XCTFail() }
    }

    func testInlineInHeading() {
        let blocks = children("# Hello *world*")
        if case .heading(let h) = blocks[0] {
            XCTAssertGreaterThan(h.children.count, 1)
        } else { XCTFail() }
    }

    // ── Backslash escapes ─────────────────────────────────────────────────

    func testBackslashEscape() {
        let blocks = children("\\*not emphasis\\*")
        if case .paragraph(let p) = blocks[0] {
            // Should contain text, not emphasis
            let hasEmphasis = p.children.contains { node in
                if case .emphasis(_) = node { return true }
                return false
            }
            XCTAssertFalse(hasEmphasis, "Escaped * should not create emphasis")
        } else { XCTFail() }
    }

    // ── Return type check ─────────────────────────────────────────────────

    func testReturnsDocumentNode() {
        let result = parse("# Hello")
        if case .document(_) = result { /* pass */ }
        else { XCTFail("parse() must return a .document node") }
    }

    func testDocumentStructure() {
        let result = parse("# Title\n\nParagraph.\n\n---")
        if case .document(let doc) = result {
            XCTAssertGreaterThanOrEqual(doc.children.count, 3)
        } else { XCTFail() }
    }
}
