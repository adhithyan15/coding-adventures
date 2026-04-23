package com.codingadventures.documentast;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.Nested;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class DocumentAstTest {

    @Nested
    class NodeTypeTests {

        @Test void documentNodeType() {
            var node = new BlockNode.DocumentNode(List.of());
            assertEquals("document", node.nodeType());
        }

        @Test void headingNodeType() {
            var node = new BlockNode.HeadingNode(1, List.of(new InlineNode.TextNode("Title")));
            assertEquals("heading", node.nodeType());
            assertEquals(1, node.level());
        }

        @Test void paragraphNodeType() {
            var node = new BlockNode.ParagraphNode(List.of(new InlineNode.TextNode("Hello")));
            assertEquals("paragraph", node.nodeType());
        }

        @Test void codeBlockNodeType() {
            var node = new BlockNode.CodeBlockNode("java", "int x = 1;\n");
            assertEquals("code_block", node.nodeType());
            assertEquals("java", node.language());
        }

        @Test void blockquoteNodeType() {
            var node = new BlockNode.BlockquoteNode(List.of());
            assertEquals("blockquote", node.nodeType());
        }

        @Test void listNodeType() {
            var node = new BlockNode.ListNode(true, 1, false, List.of());
            assertEquals("list", node.nodeType());
            assertTrue(node.ordered());
        }

        @Test void listItemNodeType() {
            var node = new BlockNode.ListItemNode(List.of());
            assertEquals("list_item", node.nodeType());
        }

        @Test void taskItemNodeType() {
            var node = new BlockNode.TaskItemNode(true, List.of());
            assertEquals("task_item", node.nodeType());
            assertTrue(node.checked());
        }

        @Test void thematicBreakNodeType() {
            var node = new BlockNode.ThematicBreakNode();
            assertEquals("thematic_break", node.nodeType());
        }

        @Test void rawBlockNodeType() {
            var node = new BlockNode.RawBlockNode("html", "<div>raw</div>");
            assertEquals("raw_block", node.nodeType());
            assertEquals("html", node.format());
        }

        @Test void tableNodeType() {
            var node = new BlockNode.TableNode(List.of(), List.of());
            assertEquals("table", node.nodeType());
        }

        @Test void tableRowNodeType() {
            var node = new BlockNode.TableRowNode(true, List.of());
            assertEquals("table_row", node.nodeType());
            assertTrue(node.isHeader());
        }

        @Test void tableCellNodeType() {
            var node = new BlockNode.TableCellNode(List.of());
            assertEquals("table_cell", node.nodeType());
        }

        @Test void textNodeType() {
            var node = new InlineNode.TextNode("hello");
            assertEquals("text", node.nodeType());
            assertEquals("hello", node.value());
        }

        @Test void emphasisNodeType() {
            var node = new InlineNode.EmphasisNode(List.of(new InlineNode.TextNode("em")));
            assertEquals("emphasis", node.nodeType());
        }

        @Test void strongNodeType() {
            var node = new InlineNode.StrongNode(List.of(new InlineNode.TextNode("bold")));
            assertEquals("strong", node.nodeType());
        }

        @Test void strikethroughNodeType() {
            var node = new InlineNode.StrikethroughNode(List.of(new InlineNode.TextNode("del")));
            assertEquals("strikethrough", node.nodeType());
        }

        @Test void codeSpanNodeType() {
            var node = new InlineNode.CodeSpanNode("x = 1");
            assertEquals("code_span", node.nodeType());
        }

        @Test void linkNodeType() {
            var node = new InlineNode.LinkNode("https://example.com", "Example",
                    List.of(new InlineNode.TextNode("click")));
            assertEquals("link", node.nodeType());
            assertEquals("https://example.com", node.destination());
        }

        @Test void imageNodeType() {
            var node = new InlineNode.ImageNode("cat.png", null, "a cat");
            assertEquals("image", node.nodeType());
            assertEquals("a cat", node.alt());
        }

        @Test void autolinkNodeType() {
            var node = new InlineNode.AutolinkNode("https://example.com", false);
            assertEquals("autolink", node.nodeType());
            assertFalse(node.isEmail());
        }

        @Test void autolinkEmail() {
            var node = new InlineNode.AutolinkNode("user@example.com", true);
            assertTrue(node.isEmail());
        }

        @Test void rawInlineNodeType() {
            var node = new InlineNode.RawInlineNode("html", "<em>raw</em>");
            assertEquals("raw_inline", node.nodeType());
        }

        @Test void hardBreakNodeType() {
            var node = new InlineNode.HardBreakNode();
            assertEquals("hard_break", node.nodeType());
        }

        @Test void softBreakNodeType() {
            var node = new InlineNode.SoftBreakNode();
            assertEquals("soft_break", node.nodeType());
        }
    }

    @Nested
    class DocumentStructureTests {

        @Test void simpleDocument() {
            var doc = new BlockNode.DocumentNode(List.of(
                    new BlockNode.HeadingNode(1, List.of(new InlineNode.TextNode("Hello"))),
                    new BlockNode.ParagraphNode(List.of(new InlineNode.TextNode("World")))
            ));
            assertEquals(2, doc.children().size());
            assertInstanceOf(BlockNode.HeadingNode.class, doc.children().get(0));
            assertInstanceOf(BlockNode.ParagraphNode.class, doc.children().get(1));
        }

        @Test void nestedBlockquote() {
            var quote = new BlockNode.BlockquoteNode(List.of(
                    new BlockNode.ParagraphNode(List.of(new InlineNode.TextNode("quoted"))),
                    new BlockNode.BlockquoteNode(List.of(
                            new BlockNode.ParagraphNode(List.of(new InlineNode.TextNode("nested")))
                    ))
            ));
            assertEquals(2, quote.children().size());
        }

        @Test void orderedList() {
            var list = new BlockNode.ListNode(true, 3, false, List.of(
                    new BlockNode.ListItemNode(List.of(
                            new BlockNode.ParagraphNode(List.of(new InlineNode.TextNode("item 3")))
                    )),
                    new BlockNode.ListItemNode(List.of(
                            new BlockNode.ParagraphNode(List.of(new InlineNode.TextNode("item 4")))
                    ))
            ));
            assertTrue(list.ordered());
            assertEquals(3, list.start());
        }

        @Test void tightUnorderedList() {
            var list = new BlockNode.ListNode(false, 0, true, List.of(
                    new BlockNode.ListItemNode(List.of(
                            new BlockNode.ParagraphNode(List.of(new InlineNode.TextNode("a")))
                    ))
            ));
            assertFalse(list.ordered());
            assertTrue(list.tight());
        }

        @Test void richInlineContent() {
            var para = new BlockNode.ParagraphNode(List.of(
                    new InlineNode.TextNode("Hello "),
                    new InlineNode.StrongNode(List.of(
                            new InlineNode.TextNode("bold "),
                            new InlineNode.EmphasisNode(List.of(new InlineNode.TextNode("and italic")))
                    )),
                    new InlineNode.TextNode(".")
            ));
            assertEquals(3, para.children().size());
        }

        @Test void tableStructure() {
            var table = new BlockNode.TableNode(
                    List.of(TableAlignment.LEFT, TableAlignment.RIGHT),
                    List.of(
                            new BlockNode.TableRowNode(true, List.of(
                                    new BlockNode.TableCellNode(List.of(new InlineNode.TextNode("Name"))),
                                    new BlockNode.TableCellNode(List.of(new InlineNode.TextNode("Score")))
                            )),
                            new BlockNode.TableRowNode(false, List.of(
                                    new BlockNode.TableCellNode(List.of(new InlineNode.TextNode("Alice"))),
                                    new BlockNode.TableCellNode(List.of(new InlineNode.TextNode("100")))
                            ))
                    )
            );
            assertEquals(2, table.align().size());
            assertEquals(2, table.rows().size());
            assertTrue(table.rows().get(0).isHeader());
            assertFalse(table.rows().get(1).isHeader());
        }

        @Test void patternMatchingOnNodes() {
            Node node = new InlineNode.TextNode("hello");
            String result = switch (node) {
                case BlockNode b -> "block: " + b.nodeType();
                case InlineNode.TextNode t -> "text: " + t.value();
                case InlineNode i -> "inline: " + i.nodeType();
            };
            assertEquals("text: hello", result);
        }

        @Test void patternMatchingOnBlockNode() {
            BlockNode node = new BlockNode.HeadingNode(2, List.of(new InlineNode.TextNode("Title")));
            String result = switch (node) {
                case BlockNode.HeadingNode h -> "h" + h.level();
                default -> "other";
            };
            assertEquals("h2", result);
        }
    }
}
