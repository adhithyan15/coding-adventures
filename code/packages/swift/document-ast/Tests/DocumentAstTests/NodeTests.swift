// ============================================================================
// NodeTests.swift — Tests for all Document AST node types
// ============================================================================
//
// These tests verify that every node type can be constructed and has the
// correct field values. Because the Document AST is a pure value type
// package (no logic, just data definitions), the tests are primarily
// construction + equality checks.
//
// Coverage strategy:
//   - Every struct: at least one construction test
//   - Every enum case: at least one case test
//   - Nested nodes: verify recursive structure
//   - Equality: verify Equatable conformance works correctly
//

import XCTest
@testable import DocumentAst

final class NodeTests: XCTestCase {

    // ── TableAlignment ─────────────────────────────────────────────────────

    func testTableAlignmentLeft() {
        let a: TableAlignment = .left
        XCTAssertEqual(a, .left)
        XCTAssertNotEqual(a, .center)
        XCTAssertNotEqual(a, .right)
    }

    func testTableAlignmentCenter() {
        let a: TableAlignment = .center
        XCTAssertEqual(a, .center)
    }

    func testTableAlignmentRight() {
        let a: TableAlignment = .right
        XCTAssertEqual(a, .right)
    }

    func testTableAlignmentEquality() {
        XCTAssertEqual(TableAlignment.left, TableAlignment.left)
        XCTAssertNotEqual(TableAlignment.left, TableAlignment.right)
        XCTAssertNotEqual(TableAlignment.center, TableAlignment.right)
    }

    // ── DocumentNode ──────────────────────────────────────────────────────

    func testDocumentNodeEmpty() {
        let doc = DocumentNode(children: [])
        XCTAssertTrue(doc.children.isEmpty)
    }

    func testDocumentNodeWithChildren() {
        let para = ParagraphNode(children: [])
        let doc = DocumentNode(children: [.paragraph(para)])
        XCTAssertEqual(doc.children.count, 1)
    }

    func testDocumentNodeEquality() {
        let a = DocumentNode(children: [])
        let b = DocumentNode(children: [])
        XCTAssertEqual(a, b)
    }

    func testDocumentNodeInequality() {
        let a = DocumentNode(children: [])
        let b = DocumentNode(children: [.thematicBreak])
        XCTAssertNotEqual(a, b)
    }

    // ── HeadingNode ───────────────────────────────────────────────────────

    func testHeadingH1() {
        let h = HeadingNode(level: 1, children: [.text(TextNode(value: "Title"))])
        XCTAssertEqual(h.level, 1)
        XCTAssertEqual(h.children.count, 1)
    }

    func testHeadingH6() {
        let h = HeadingNode(level: 6, children: [])
        XCTAssertEqual(h.level, 6)
        XCTAssertTrue(h.children.isEmpty)
    }

    func testHeadingEquality() {
        let a = HeadingNode(level: 2, children: [.text(TextNode(value: "A"))])
        let b = HeadingNode(level: 2, children: [.text(TextNode(value: "A"))])
        XCTAssertEqual(a, b)
    }

    func testHeadingInequality() {
        let a = HeadingNode(level: 1, children: [.text(TextNode(value: "A"))])
        let b = HeadingNode(level: 2, children: [.text(TextNode(value: "A"))])
        XCTAssertNotEqual(a, b)
    }

    // ── ParagraphNode ─────────────────────────────────────────────────────

    func testParagraphEmpty() {
        let p = ParagraphNode(children: [])
        XCTAssertTrue(p.children.isEmpty)
    }

    func testParagraphWithText() {
        let p = ParagraphNode(children: [.text(TextNode(value: "Hello"))])
        XCTAssertEqual(p.children.count, 1)
    }

    func testParagraphEquality() {
        let a = ParagraphNode(children: [.text(TextNode(value: "foo"))])
        let b = ParagraphNode(children: [.text(TextNode(value: "foo"))])
        XCTAssertEqual(a, b)
    }

    // ── CodeBlockNode ─────────────────────────────────────────────────────

    func testCodeBlockWithLanguage() {
        let cb = CodeBlockNode(language: "swift", value: "let x = 1\n")
        XCTAssertEqual(cb.language, "swift")
        XCTAssertEqual(cb.value, "let x = 1\n")
    }

    func testCodeBlockNoLanguage() {
        let cb = CodeBlockNode(language: nil, value: "plain code\n")
        XCTAssertNil(cb.language)
        XCTAssertEqual(cb.value, "plain code\n")
    }

    func testCodeBlockEquality() {
        let a = CodeBlockNode(language: "python", value: "print(1)\n")
        let b = CodeBlockNode(language: "python", value: "print(1)\n")
        XCTAssertEqual(a, b)
    }

    func testCodeBlockInequality() {
        let a = CodeBlockNode(language: "python", value: "x\n")
        let b = CodeBlockNode(language: nil, value: "x\n")
        XCTAssertNotEqual(a, b)
    }

    // ── BlockquoteNode ────────────────────────────────────────────────────

    func testBlockquoteEmpty() {
        let bq = BlockquoteNode(children: [])
        XCTAssertTrue(bq.children.isEmpty)
    }

    func testBlockquoteWithChildren() {
        let para = ParagraphNode(children: [.text(TextNode(value: "Wise words"))])
        let bq = BlockquoteNode(children: [.paragraph(para)])
        XCTAssertEqual(bq.children.count, 1)
    }

    func testBlockquoteNestedEquality() {
        // Blockquotes can be nested: > > text
        let inner = BlockquoteNode(children: [.paragraph(ParagraphNode(children: [.text(TextNode(value: "inner"))]))])
        let outer = BlockquoteNode(children: [.blockquote(inner)])
        XCTAssertEqual(outer.children.count, 1)
    }

    // ── ListNode ──────────────────────────────────────────────────────────

    func testListUnordered() {
        let list = ListNode(ordered: false, start: nil, tight: true, children: [])
        XCTAssertFalse(list.ordered)
        XCTAssertNil(list.start)
        XCTAssertTrue(list.tight)
        XCTAssertTrue(list.children.isEmpty)
    }

    func testListOrdered() {
        let item = ListItemNode(children: [])
        let list = ListNode(ordered: true, start: 1, tight: false, children: [item])
        XCTAssertTrue(list.ordered)
        XCTAssertEqual(list.start, 1)
        XCTAssertFalse(list.tight)
        XCTAssertEqual(list.children.count, 1)
    }

    func testListOrderedCustomStart() {
        let list = ListNode(ordered: true, start: 5, tight: true, children: [])
        XCTAssertEqual(list.start, 5)
    }

    func testListEquality() {
        let a = ListNode(ordered: false, start: nil, tight: true, children: [])
        let b = ListNode(ordered: false, start: nil, tight: true, children: [])
        XCTAssertEqual(a, b)
    }

    // ── ListItemNode ──────────────────────────────────────────────────────

    func testListItemEmpty() {
        let item = ListItemNode(children: [])
        XCTAssertTrue(item.children.isEmpty)
    }

    func testListItemWithParagraph() {
        let para = ParagraphNode(children: [.text(TextNode(value: "A"))])
        let item = ListItemNode(children: [.paragraph(para)])
        XCTAssertEqual(item.children.count, 1)
    }

    // ── TaskItemNode ──────────────────────────────────────────────────────

    func testTaskItemChecked() {
        let task = TaskItemNode(checked: true, children: [])
        XCTAssertTrue(task.checked)
        XCTAssertTrue(task.children.isEmpty)
    }

    func testTaskItemUnchecked() {
        let task = TaskItemNode(checked: false, children: [])
        XCTAssertFalse(task.checked)
    }

    func testTaskItemWithChildren() {
        let para = ParagraphNode(children: [.text(TextNode(value: "Done"))])
        let task = TaskItemNode(checked: true, children: [.paragraph(para)])
        XCTAssertEqual(task.children.count, 1)
    }

    // ── ThematicBreakNode ─────────────────────────────────────────────────

    func testThematicBreakConstruction() {
        let tb = ThematicBreakNode()
        XCTAssertEqual(tb, ThematicBreakNode())
    }

    // ── RawBlockNode ──────────────────────────────────────────────────────

    func testRawBlockHtml() {
        let rb = RawBlockNode(format: "html", value: "<div>raw</div>\n")
        XCTAssertEqual(rb.format, "html")
        XCTAssertEqual(rb.value, "<div>raw</div>\n")
    }

    func testRawBlockLatex() {
        let rb = RawBlockNode(format: "latex", value: "\\textbf{bold}\n")
        XCTAssertEqual(rb.format, "latex")
    }

    func testRawBlockEquality() {
        let a = RawBlockNode(format: "html", value: "<p>x</p>\n")
        let b = RawBlockNode(format: "html", value: "<p>x</p>\n")
        XCTAssertEqual(a, b)
    }

    // ── TableNode ─────────────────────────────────────────────────────────

    func testTableEmpty() {
        let t = TableNode(align: [], children: [])
        XCTAssertTrue(t.align.isEmpty)
        XCTAssertTrue(t.children.isEmpty)
    }

    func testTableWithAlignments() {
        let t = TableNode(align: [.left, .center, .right, nil], children: [])
        XCTAssertEqual(t.align.count, 4)
        XCTAssertEqual(t.align[0], .left)
        XCTAssertEqual(t.align[1], .center)
        XCTAssertEqual(t.align[2], .right)
        XCTAssertNil(t.align[3])
    }

    // ── TableRowNode ──────────────────────────────────────────────────────

    func testTableRowHeader() {
        let row = TableRowNode(isHeader: true, children: [])
        XCTAssertTrue(row.isHeader)
        XCTAssertTrue(row.children.isEmpty)
    }

    func testTableRowBody() {
        let cell = TableCellNode(children: [.text(TextNode(value: "A"))])
        let row = TableRowNode(isHeader: false, children: [cell])
        XCTAssertFalse(row.isHeader)
        XCTAssertEqual(row.children.count, 1)
    }

    // ── TableCellNode ─────────────────────────────────────────────────────

    func testTableCellEmpty() {
        let cell = TableCellNode(children: [])
        XCTAssertTrue(cell.children.isEmpty)
    }

    func testTableCellWithInlines() {
        let cell = TableCellNode(children: [.text(TextNode(value: "Name"))])
        XCTAssertEqual(cell.children.count, 1)
    }

    // ── TextNode ──────────────────────────────────────────────────────────

    func testTextNode() {
        let t = TextNode(value: "Hello, world!")
        XCTAssertEqual(t.value, "Hello, world!")
    }

    func testTextNodeEquality() {
        let a = TextNode(value: "foo")
        let b = TextNode(value: "foo")
        XCTAssertEqual(a, b)
    }

    func testTextNodeInequality() {
        let a = TextNode(value: "foo")
        let b = TextNode(value: "bar")
        XCTAssertNotEqual(a, b)
    }

    // ── EmphasisNode ──────────────────────────────────────────────────────

    func testEmphasisNode() {
        let em = EmphasisNode(children: [.text(TextNode(value: "hello"))])
        XCTAssertEqual(em.children.count, 1)
    }

    func testEmphasisEmpty() {
        let em = EmphasisNode(children: [])
        XCTAssertTrue(em.children.isEmpty)
    }

    // ── StrongNode ────────────────────────────────────────────────────────

    func testStrongNode() {
        let s = StrongNode(children: [.text(TextNode(value: "bold"))])
        XCTAssertEqual(s.children.count, 1)
    }

    func testStrongEquality() {
        let a = StrongNode(children: [.text(TextNode(value: "x"))])
        let b = StrongNode(children: [.text(TextNode(value: "x"))])
        XCTAssertEqual(a, b)
    }

    // ── CodeSpanNode ──────────────────────────────────────────────────────

    func testCodeSpanNode() {
        let cs = CodeSpanNode(value: "let x = 1")
        XCTAssertEqual(cs.value, "let x = 1")
    }

    func testCodeSpanEquality() {
        let a = CodeSpanNode(value: "foo")
        let b = CodeSpanNode(value: "foo")
        XCTAssertEqual(a, b)
    }

    // ── LinkNode ──────────────────────────────────────────────────────────

    func testLinkWithTitle() {
        let link = LinkNode(
            destination: "https://example.com",
            title: "Example Site",
            children: [.text(TextNode(value: "click"))]
        )
        XCTAssertEqual(link.destination, "https://example.com")
        XCTAssertEqual(link.title, "Example Site")
        XCTAssertEqual(link.children.count, 1)
    }

    func testLinkWithoutTitle() {
        let link = LinkNode(destination: "https://example.com", title: nil, children: [])
        XCTAssertNil(link.title)
    }

    func testLinkEquality() {
        let a = LinkNode(destination: "https://a.com", title: nil, children: [])
        let b = LinkNode(destination: "https://a.com", title: nil, children: [])
        XCTAssertEqual(a, b)
    }

    // ── ImageNode ─────────────────────────────────────────────────────────

    func testImageNode() {
        let img = ImageNode(destination: "cat.png", title: nil, alt: "a cat")
        XCTAssertEqual(img.destination, "cat.png")
        XCTAssertNil(img.title)
        XCTAssertEqual(img.alt, "a cat")
    }

    func testImageWithTitle() {
        let img = ImageNode(destination: "cat.png", title: "My cat", alt: "cat")
        XCTAssertEqual(img.title, "My cat")
    }

    func testImageEquality() {
        let a = ImageNode(destination: "x.png", title: nil, alt: "x")
        let b = ImageNode(destination: "x.png", title: nil, alt: "x")
        XCTAssertEqual(a, b)
    }

    // ── AutolinkNode ──────────────────────────────────────────────────────

    func testAutolinkUrl() {
        let a = AutolinkNode(destination: "https://example.com", isEmail: false)
        XCTAssertEqual(a.destination, "https://example.com")
        XCTAssertFalse(a.isEmail)
    }

    func testAutolinkEmail() {
        let a = AutolinkNode(destination: "user@example.com", isEmail: true)
        XCTAssertEqual(a.destination, "user@example.com")
        XCTAssertTrue(a.isEmail)
    }

    func testAutolinkEquality() {
        let a = AutolinkNode(destination: "https://x.com", isEmail: false)
        let b = AutolinkNode(destination: "https://x.com", isEmail: false)
        XCTAssertEqual(a, b)
    }

    // ── RawInlineNode ─────────────────────────────────────────────────────

    func testRawInlineHtml() {
        let r = RawInlineNode(format: "html", value: "<kbd>Ctrl</kbd>")
        XCTAssertEqual(r.format, "html")
        XCTAssertEqual(r.value, "<kbd>Ctrl</kbd>")
    }

    func testRawInlineEquality() {
        let a = RawInlineNode(format: "html", value: "<b>x</b>")
        let b = RawInlineNode(format: "html", value: "<b>x</b>")
        XCTAssertEqual(a, b)
    }

    // ── HardBreakNode ─────────────────────────────────────────────────────

    func testHardBreakConstruction() {
        let hb = HardBreakNode()
        XCTAssertEqual(hb, HardBreakNode())
    }

    // ── SoftBreakNode ─────────────────────────────────────────────────────

    func testSoftBreakConstruction() {
        let sb = SoftBreakNode()
        XCTAssertEqual(sb, SoftBreakNode())
    }

    // ── StrikethroughNode ─────────────────────────────────────────────────

    func testStrikethroughNode() {
        let s = StrikethroughNode(children: [.text(TextNode(value: "deleted"))])
        XCTAssertEqual(s.children.count, 1)
    }

    func testStrikethroughEquality() {
        let a = StrikethroughNode(children: [.text(TextNode(value: "x"))])
        let b = StrikethroughNode(children: [.text(TextNode(value: "x"))])
        XCTAssertEqual(a, b)
    }

    // ── BlockNode enum ────────────────────────────────────────────────────

    func testBlockNodeDocumentCase() {
        let doc = DocumentNode(children: [])
        let node: BlockNode = .document(doc)
        if case .document(let d) = node {
            XCTAssertEqual(d, doc)
        } else {
            XCTFail("Expected .document case")
        }
    }

    func testBlockNodeHeadingCase() {
        let heading = HeadingNode(level: 1, children: [])
        let node: BlockNode = .heading(heading)
        if case .heading(let h) = node {
            XCTAssertEqual(h.level, 1)
        } else {
            XCTFail("Expected .heading case")
        }
    }

    func testBlockNodeParagraphCase() {
        let para = ParagraphNode(children: [])
        let node: BlockNode = .paragraph(para)
        if case .paragraph(let p) = node {
            XCTAssertTrue(p.children.isEmpty)
        } else {
            XCTFail("Expected .paragraph case")
        }
    }

    func testBlockNodeCodeBlockCase() {
        let cb = CodeBlockNode(language: "swift", value: "let x = 1\n")
        let node: BlockNode = .codeBlock(cb)
        if case .codeBlock(let c) = node {
            XCTAssertEqual(c.language, "swift")
        } else {
            XCTFail("Expected .codeBlock case")
        }
    }

    func testBlockNodeBlockquoteCase() {
        let bq = BlockquoteNode(children: [])
        let node: BlockNode = .blockquote(bq)
        if case .blockquote(let b) = node {
            XCTAssertTrue(b.children.isEmpty)
        } else {
            XCTFail("Expected .blockquote case")
        }
    }

    func testBlockNodeListCase() {
        let list = ListNode(ordered: false, start: nil, tight: true, children: [])
        let node: BlockNode = .list(list)
        if case .list(let l) = node {
            XCTAssertFalse(l.ordered)
        } else {
            XCTFail("Expected .list case")
        }
    }

    func testBlockNodeListItemCase() {
        let item = ListItemNode(children: [])
        let node: BlockNode = .listItem(item)
        if case .listItem(let i) = node {
            XCTAssertTrue(i.children.isEmpty)
        } else {
            XCTFail("Expected .listItem case")
        }
    }

    func testBlockNodeTaskItemCase() {
        let task = TaskItemNode(checked: true, children: [])
        let node: BlockNode = .taskItem(task)
        if case .taskItem(let t) = node {
            XCTAssertTrue(t.checked)
        } else {
            XCTFail("Expected .taskItem case")
        }
    }

    func testBlockNodeThematicBreakCase() {
        let node: BlockNode = .thematicBreak
        if case .thematicBreak = node {
            // pass
        } else {
            XCTFail("Expected .thematicBreak case")
        }
    }

    func testBlockNodeRawBlockCase() {
        let rb = RawBlockNode(format: "html", value: "<hr>\n")
        let node: BlockNode = .rawBlock(rb)
        if case .rawBlock(let r) = node {
            XCTAssertEqual(r.format, "html")
        } else {
            XCTFail("Expected .rawBlock case")
        }
    }

    func testBlockNodeTableCase() {
        let table = TableNode(align: [.left], children: [])
        let node: BlockNode = .table(table)
        if case .table(let t) = node {
            XCTAssertEqual(t.align.count, 1)
        } else {
            XCTFail("Expected .table case")
        }
    }

    func testBlockNodeTableRowCase() {
        let row = TableRowNode(isHeader: true, children: [])
        let node: BlockNode = .tableRow(row)
        if case .tableRow(let r) = node {
            XCTAssertTrue(r.isHeader)
        } else {
            XCTFail("Expected .tableRow case")
        }
    }

    func testBlockNodeTableCellCase() {
        let cell = TableCellNode(children: [])
        let node: BlockNode = .tableCell(cell)
        if case .tableCell(let c) = node {
            XCTAssertTrue(c.children.isEmpty)
        } else {
            XCTFail("Expected .tableCell case")
        }
    }

    func testBlockNodeEquality() {
        let a: BlockNode = .thematicBreak
        let b: BlockNode = .thematicBreak
        XCTAssertEqual(a, b)
    }

    func testBlockNodeInequality() {
        let a: BlockNode = .thematicBreak
        let b: BlockNode = .paragraph(ParagraphNode(children: []))
        XCTAssertNotEqual(a, b)
    }

    // ── InlineNode enum ───────────────────────────────────────────────────

    func testInlineNodeTextCase() {
        let node: InlineNode = .text(TextNode(value: "Hello"))
        if case .text(let t) = node {
            XCTAssertEqual(t.value, "Hello")
        } else {
            XCTFail("Expected .text case")
        }
    }

    func testInlineNodeEmphasisCase() {
        let node: InlineNode = .emphasis(EmphasisNode(children: [.text(TextNode(value: "em"))]))
        if case .emphasis(let e) = node {
            XCTAssertEqual(e.children.count, 1)
        } else {
            XCTFail("Expected .emphasis case")
        }
    }

    func testInlineNodeStrongCase() {
        let node: InlineNode = .strong(StrongNode(children: []))
        if case .strong(let s) = node {
            XCTAssertTrue(s.children.isEmpty)
        } else {
            XCTFail("Expected .strong case")
        }
    }

    func testInlineNodeCodeSpanCase() {
        let node: InlineNode = .codeSpan(CodeSpanNode(value: "x"))
        if case .codeSpan(let c) = node {
            XCTAssertEqual(c.value, "x")
        } else {
            XCTFail("Expected .codeSpan case")
        }
    }

    func testInlineNodeLinkCase() {
        let link = LinkNode(destination: "https://example.com", title: nil, children: [])
        let node: InlineNode = .link(link)
        if case .link(let l) = node {
            XCTAssertEqual(l.destination, "https://example.com")
        } else {
            XCTFail("Expected .link case")
        }
    }

    func testInlineNodeImageCase() {
        let img = ImageNode(destination: "x.png", title: nil, alt: "x")
        let node: InlineNode = .image(img)
        if case .image(let i) = node {
            XCTAssertEqual(i.alt, "x")
        } else {
            XCTFail("Expected .image case")
        }
    }

    func testInlineNodeAutolinkCase() {
        let al = AutolinkNode(destination: "https://x.com", isEmail: false)
        let node: InlineNode = .autolink(al)
        if case .autolink(let a) = node {
            XCTAssertFalse(a.isEmail)
        } else {
            XCTFail("Expected .autolink case")
        }
    }

    func testInlineNodeRawInlineCase() {
        let ri = RawInlineNode(format: "html", value: "<b>x</b>")
        let node: InlineNode = .rawInline(ri)
        if case .rawInline(let r) = node {
            XCTAssertEqual(r.format, "html")
        } else {
            XCTFail("Expected .rawInline case")
        }
    }

    func testInlineNodeHardBreakCase() {
        let node: InlineNode = .hardBreak
        if case .hardBreak = node {
            // pass
        } else {
            XCTFail("Expected .hardBreak case")
        }
    }

    func testInlineNodeSoftBreakCase() {
        let node: InlineNode = .softBreak
        if case .softBreak = node {
            // pass
        } else {
            XCTFail("Expected .softBreak case")
        }
    }

    func testInlineNodeStrikethroughCase() {
        let s = StrikethroughNode(children: [.text(TextNode(value: "del"))])
        let node: InlineNode = .strikethrough(s)
        if case .strikethrough(let st) = node {
            XCTAssertEqual(st.children.count, 1)
        } else {
            XCTFail("Expected .strikethrough case")
        }
    }

    func testInlineNodeEquality() {
        let a: InlineNode = .hardBreak
        let b: InlineNode = .hardBreak
        XCTAssertEqual(a, b)
    }

    func testInlineNodeInequality() {
        let a: InlineNode = .hardBreak
        let b: InlineNode = .softBreak
        XCTAssertNotEqual(a, b)
    }

    // ── Complex nested structure ───────────────────────────────────────────

    func testComplexDocumentStructure() {
        // Build a realistic document and verify the structure round-trips
        let doc = DocumentNode(children: [
            .heading(HeadingNode(level: 1, children: [.text(TextNode(value: "Title"))])),
            .paragraph(ParagraphNode(children: [
                .text(TextNode(value: "Some ")),
                .emphasis(EmphasisNode(children: [.text(TextNode(value: "italic"))])),
                .text(TextNode(value: " text."))
            ])),
            .list(ListNode(ordered: false, start: nil, tight: true, children: [
                ListItemNode(children: [
                    .paragraph(ParagraphNode(children: [.text(TextNode(value: "Item 1"))]))
                ]),
                ListItemNode(children: [
                    .paragraph(ParagraphNode(children: [.text(TextNode(value: "Item 2"))]))
                ])
            ])),
            .thematicBreak,
            .codeBlock(CodeBlockNode(language: "swift", value: "print(\"hello\")\n"))
        ])

        XCTAssertEqual(doc.children.count, 5)

        if case .heading(let h) = doc.children[0] {
            XCTAssertEqual(h.level, 1)
        } else {
            XCTFail("Expected heading as first child")
        }

        if case .list(let l) = doc.children[2] {
            XCTAssertEqual(l.children.count, 2)
        } else {
            XCTFail("Expected list as third child")
        }
    }

    func testTableFullStructure() {
        // Build a 2-column table with header and one body row
        let headerCell1 = TableCellNode(children: [.text(TextNode(value: "Name"))])
        let headerCell2 = TableCellNode(children: [.text(TextNode(value: "Age"))])
        let headerRow = TableRowNode(isHeader: true, children: [headerCell1, headerCell2])

        let bodyCell1 = TableCellNode(children: [.text(TextNode(value: "Alice"))])
        let bodyCell2 = TableCellNode(children: [.text(TextNode(value: "30"))])
        let bodyRow = TableRowNode(isHeader: false, children: [bodyCell1, bodyCell2])

        let table = TableNode(align: [.left, .right], children: [headerRow, bodyRow])

        XCTAssertEqual(table.align.count, 2)
        XCTAssertEqual(table.children.count, 2)
        XCTAssertTrue(table.children[0].isHeader)
        XCTAssertFalse(table.children[1].isHeader)
    }
}
