package com.codingadventures.documentast

import kotlin.test.*

class DocumentAstTest {

    // === Node types ===

    @Test fun documentNodeType() = assertEquals("document", DocumentNode(emptyList()).nodeType)
    @Test fun headingNodeType() = assertEquals("heading", HeadingNode(1, listOf(TextNode("T"))).nodeType)
    @Test fun paragraphNodeType() = assertEquals("paragraph", ParagraphNode(listOf(TextNode("T"))).nodeType)
    @Test fun codeBlockNodeType() = assertEquals("code_block", CodeBlockNode("java", "x").nodeType)
    @Test fun blockquoteNodeType() = assertEquals("blockquote", BlockquoteNode(emptyList()).nodeType)
    @Test fun listNodeType() = assertEquals("list", ListNode(true, 1, false, emptyList()).nodeType)
    @Test fun listItemNodeType() = assertEquals("list_item", ListItemNode(emptyList()).nodeType)
    @Test fun taskItemNodeType() = assertEquals("task_item", TaskItemNode(true, emptyList()).nodeType)
    @Test fun thematicBreakNodeType() = assertEquals("thematic_break", ThematicBreakNode.nodeType)
    @Test fun rawBlockNodeType() = assertEquals("raw_block", RawBlockNode("html", "<div>").nodeType)
    @Test fun tableNodeType() = assertEquals("table", TableNode(emptyList(), emptyList()).nodeType)
    @Test fun tableRowNodeType() = assertEquals("table_row", TableRowNode(true, emptyList()).nodeType)
    @Test fun tableCellNodeType() = assertEquals("table_cell", TableCellNode(emptyList()).nodeType)
    @Test fun textNodeType() = assertEquals("text", TextNode("hello").nodeType)
    @Test fun emphasisNodeType() = assertEquals("emphasis", EmphasisNode(listOf(TextNode("em"))).nodeType)
    @Test fun strongNodeType() = assertEquals("strong", StrongNode(listOf(TextNode("b"))).nodeType)
    @Test fun strikethroughNodeType() = assertEquals("strikethrough", StrikethroughNode(listOf(TextNode("d"))).nodeType)
    @Test fun codeSpanNodeType() = assertEquals("code_span", CodeSpanNode("x").nodeType)
    @Test fun linkNodeType() = assertEquals("link", LinkNode("url", null, listOf(TextNode("t"))).nodeType)
    @Test fun imageNodeType() = assertEquals("image", ImageNode("cat.png", null, "cat").nodeType)
    @Test fun autolinkNodeType() = assertEquals("autolink", AutolinkNode("url", false).nodeType)
    @Test fun rawInlineNodeType() = assertEquals("raw_inline", RawInlineNode("html", "<em>").nodeType)
    @Test fun hardBreakNodeType() = assertEquals("hard_break", HardBreakNode.nodeType)
    @Test fun softBreakNodeType() = assertEquals("soft_break", SoftBreakNode.nodeType)

    // === Structure ===

    @Test fun simpleDocument() {
        val doc = DocumentNode(listOf(
            HeadingNode(1, listOf(TextNode("Hello"))),
            ParagraphNode(listOf(TextNode("World")))
        ))
        assertEquals(2, doc.children.size)
        assertIs<HeadingNode>(doc.children[0])
        assertIs<ParagraphNode>(doc.children[1])
    }

    @Test fun nestedBlockquote() {
        val q = BlockquoteNode(listOf(
            ParagraphNode(listOf(TextNode("outer"))),
            BlockquoteNode(listOf(ParagraphNode(listOf(TextNode("inner")))))
        ))
        assertEquals(2, q.children.size)
    }

    @Test fun orderedList() {
        val list = ListNode(true, 3, false, listOf(
            ListItemNode(listOf(ParagraphNode(listOf(TextNode("item 3")))))
        ))
        assertTrue(list.ordered)
        assertEquals(3, list.start)
    }

    @Test fun richInline() {
        val p = ParagraphNode(listOf(
            TextNode("Hello "),
            StrongNode(listOf(TextNode("bold"), EmphasisNode(listOf(TextNode(" and italic"))))),
            TextNode(".")
        ))
        assertEquals(3, p.children.size)
    }

    @Test fun tableStructure() {
        val table = TableNode(
            listOf(TableAlignment.LEFT, TableAlignment.RIGHT),
            listOf(
                TableRowNode(true, listOf(
                    TableCellNode(listOf(TextNode("Name"))),
                    TableCellNode(listOf(TextNode("Score")))
                )),
                TableRowNode(false, listOf(
                    TableCellNode(listOf(TextNode("Alice"))),
                    TableCellNode(listOf(TextNode("100")))
                ))
            )
        )
        assertEquals(2, table.align.size)
        assertEquals(2, table.rows.size)
        assertTrue(table.rows[0].isHeader)
    }

    @Test fun whenExhaustive() {
        val node: InlineNode = TextNode("hello")
        val result = when (node) {
            is TextNode -> "text: ${node.value}"
            is EmphasisNode -> "em"
            is StrongNode -> "strong"
            is StrikethroughNode -> "strike"
            is CodeSpanNode -> "code"
            is LinkNode -> "link"
            is ImageNode -> "img"
            is AutolinkNode -> "autolink"
            is RawInlineNode -> "raw"
            HardBreakNode -> "hard"
            SoftBreakNode -> "soft"
        }
        assertEquals("text: hello", result)
    }

    @Test fun taskItemChecked() {
        assertTrue(TaskItemNode(true, emptyList()).checked)
        assertFalse(TaskItemNode(false, emptyList()).checked)
    }

    @Test fun autolinkEmail() {
        assertTrue(AutolinkNode("user@example.com", true).isEmail)
        assertFalse(AutolinkNode("https://example.com", false).isEmail)
    }

    @Test fun headingLevel() {
        assertEquals(2, HeadingNode(2, listOf(TextNode("H2"))).level)
    }

    @Test fun codeBlockLanguage() {
        assertEquals("kotlin", CodeBlockNode("kotlin", "val x = 1\n").language)
    }
}
