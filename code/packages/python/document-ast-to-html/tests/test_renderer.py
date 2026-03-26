"""Tests for the Document AST HTML renderer."""

from coding_adventures_commonmark_parser import parse
from coding_adventures_document_ast import (
    BlockquoteNode,
    CodeBlockNode,
    DocumentNode,
    HeadingNode,
    ListItemNode,
    ListNode,
    ParagraphNode,
    RawBlockNode,
    TableNode,
    ThematicBreakNode,
)

from coding_adventures_document_ast_to_html import RenderOptions, to_html


def make_doc(*children) -> DocumentNode:
    return {"type": "document", "children": list(children)}


class TestBlockRendering:
    def test_empty_document(self) -> None:
        doc = make_doc()
        assert to_html(doc) == ""

    def test_heading_levels(self) -> None:
        for level in [1, 2, 3, 4, 5, 6]:
            node: HeadingNode = {
                "type": "heading",
                "level": level,
                "children": [{"type": "text", "value": "Title"}],
            }
            html = to_html(make_doc(node))
            assert html == f"<h{level}>Title</h{level}>\n"

    def test_paragraph(self) -> None:
        node: ParagraphNode = {
            "type": "paragraph",
            "children": [{"type": "text", "value": "Hello"}],
        }
        assert to_html(make_doc(node)) == "<p>Hello</p>\n"

    def test_thematic_break(self) -> None:
        node: ThematicBreakNode = {"type": "thematic_break"}
        assert to_html(make_doc(node)) == "<hr />\n"

    def test_code_block_with_language(self) -> None:
        node: CodeBlockNode = {
            "type": "code_block",
            "language": "python",
            "value": "print('hello')\n",
        }
        html = to_html(make_doc(node))
        assert 'class="language-python"' in html
        assert "print" in html

    def test_code_block_no_language(self) -> None:
        node: CodeBlockNode = {
            "type": "code_block",
            "language": None,
            "value": "code\n",
        }
        html = to_html(make_doc(node))
        assert html == "<pre><code>code\n</code></pre>\n"

    def test_blockquote(self) -> None:
        node: BlockquoteNode = {
            "type": "blockquote",
            "children": [
                {
                    "type": "paragraph",
                    "children": [{"type": "text", "value": "quote"}],
                }
            ],
        }
        html = to_html(make_doc(node))
        assert html == "<blockquote>\n<p>quote</p>\n</blockquote>\n"

    def test_unordered_list_tight(self) -> None:
        item: ListItemNode = {
            "type": "list_item",
            "children": [
                {"type": "paragraph", "children": [{"type": "text", "value": "item"}]}
            ],
        }
        node: ListNode = {
            "type": "list",
            "ordered": False,
            "start": None,
            "tight": True,
            "children": [item],
        }
        html = to_html(make_doc(node))
        assert "<ul>" in html
        assert "<li>item</li>" in html

    def test_ordered_list_with_start(self) -> None:
        item: ListItemNode = {
            "type": "list_item",
            "children": [
                {"type": "paragraph", "children": [{"type": "text", "value": "item"}]}
            ],
        }
        node: ListNode = {
            "type": "list",
            "ordered": True,
            "start": 3,
            "tight": False,
            "children": [item],
        }
        html = to_html(make_doc(node))
        assert 'start="3"' in html

    def test_raw_block_html(self) -> None:
        node: RawBlockNode = {
            "type": "raw_block",
            "format": "html",
            "value": "<div>raw</div>\n",
        }
        assert to_html(make_doc(node)) == "<div>raw</div>\n"

    def test_raw_block_latex_skipped(self) -> None:
        node: RawBlockNode = {
            "type": "raw_block",
            "format": "latex",
            "value": "\\textbf{x}\n",
        }
        assert to_html(make_doc(node)) == ""

    def test_raw_block_sanitized(self) -> None:
        node: RawBlockNode = {
            "type": "raw_block",
            "format": "html",
            "value": "<script>alert(1)</script>\n",
        }
        options = RenderOptions(sanitize=True)
        assert to_html(make_doc(node), options) == ""

    def test_task_list_item(self) -> None:
        node: ListNode = {
            "type": "list",
            "ordered": False,
            "start": None,
            "tight": True,
            "children": [
                {
                    "type": "task_item",
                    "checked": True,
                    "children": [
                        {"type": "paragraph", "children": [{"type": "text", "value": "done"}]}
                    ],
                }
            ],
        }
        html = to_html(make_doc(node))
        assert '<input type="checkbox" disabled="" checked="" /> done' in html

    def test_table(self) -> None:
        node: TableNode = {
            "type": "table",
            "align": ["left"],
            "children": [
                {
                    "type": "table_row",
                    "isHeader": True,
                    "children": [{"type": "table_cell", "children": [{"type": "text", "value": "A"}]}],
                },
                {
                    "type": "table_row",
                    "isHeader": False,
                    "children": [{"type": "table_cell", "children": [{"type": "text", "value": "B"}]}],
                },
            ],
        }
        html = to_html(make_doc(node))
        assert "<table>" in html
        assert "<thead>" in html
        assert '<th align="left">A</th>' in html
        assert '<td align="left">B</td>' in html


class TestInlineRendering:
    def test_text_escaping(self) -> None:
        node: ParagraphNode = {
            "type": "paragraph",
            "children": [{"type": "text", "value": "a & b < c > d"}],
        }
        html = to_html(make_doc(node))
        assert "a &amp; b &lt; c &gt; d" in html

    def test_emphasis(self) -> None:
        node: ParagraphNode = {
            "type": "paragraph",
            "children": [
                {"type": "emphasis", "children": [{"type": "text", "value": "em"}]}
            ],
        }
        assert "<em>em</em>" in to_html(make_doc(node))

    def test_strong(self) -> None:
        node: ParagraphNode = {
            "type": "paragraph",
            "children": [
                {"type": "strong", "children": [{"type": "text", "value": "strong"}]}
            ],
        }
        assert "<strong>strong</strong>" in to_html(make_doc(node))

    def test_strikethrough(self) -> None:
        node: ParagraphNode = {
            "type": "paragraph",
            "children": [
                {"type": "strikethrough", "children": [{"type": "text", "value": "gone"}]}
            ],
        }
        assert "<del>gone</del>" in to_html(make_doc(node))

    def test_code_span(self) -> None:
        node: ParagraphNode = {
            "type": "paragraph",
            "children": [{"type": "code_span", "value": "code"}],
        }
        assert "<code>code</code>" in to_html(make_doc(node))

    def test_link(self) -> None:
        node: ParagraphNode = {
            "type": "paragraph",
            "children": [
                {
                    "type": "link",
                    "destination": "https://example.com",
                    "title": None,
                    "children": [{"type": "text", "value": "click"}],
                }
            ],
        }
        html = to_html(make_doc(node))
        assert '<a href="https://example.com">click</a>' in html

    def test_link_with_title(self) -> None:
        node: ParagraphNode = {
            "type": "paragraph",
            "children": [
                {
                    "type": "link",
                    "destination": "https://example.com",
                    "title": "Example",
                    "children": [{"type": "text", "value": "click"}],
                }
            ],
        }
        html = to_html(make_doc(node))
        assert 'title="Example"' in html

    def test_image(self) -> None:
        node: ParagraphNode = {
            "type": "paragraph",
            "children": [
                {
                    "type": "image",
                    "destination": "cat.png",
                    "title": None,
                    "alt": "a cat",
                }
            ],
        }
        html = to_html(make_doc(node))
        assert '<img src="cat.png" alt="a cat" />' in html

    def test_autolink_url(self) -> None:
        node: ParagraphNode = {
            "type": "paragraph",
            "children": [
                {
                    "type": "autolink",
                    "destination": "https://example.com",
                    "is_email": False,
                }
            ],
        }
        html = to_html(make_doc(node))
        assert 'href="https://example.com"' in html

    def test_autolink_email(self) -> None:
        node: ParagraphNode = {
            "type": "paragraph",
            "children": [
                {
                    "type": "autolink",
                    "destination": "user@example.com",
                    "is_email": True,
                }
            ],
        }
        html = to_html(make_doc(node))
        assert 'href="mailto:user@example.com"' in html

    def test_hard_break(self) -> None:
        node: ParagraphNode = {
            "type": "paragraph",
            "children": [
                {"type": "text", "value": "line1"},
                {"type": "hard_break"},
                {"type": "text", "value": "line2"},
            ],
        }
        html = to_html(make_doc(node))
        assert "<br />" in html

    def test_soft_break(self) -> None:
        node: ParagraphNode = {
            "type": "paragraph",
            "children": [
                {"type": "text", "value": "line1"},
                {"type": "soft_break"},
                {"type": "text", "value": "line2"},
            ],
        }
        html = to_html(make_doc(node))
        assert "\n" in html

    def test_raw_inline_html(self) -> None:
        node: ParagraphNode = {
            "type": "paragraph",
            "children": [{"type": "raw_inline", "format": "html", "value": "<em>raw</em>"}],
        }
        assert "<em>raw</em>" in to_html(make_doc(node))

    def test_raw_inline_sanitized(self) -> None:
        node: ParagraphNode = {
            "type": "paragraph",
            "children": [{"type": "raw_inline", "format": "html", "value": "<script>alert(1)</script>"}],
        }
        options = RenderOptions(sanitize=True)
        assert "<script>" not in to_html(make_doc(node), options)


class TestUrlSanitization:
    def test_javascript_link_blocked(self) -> None:
        node: ParagraphNode = {
            "type": "paragraph",
            "children": [
                {
                    "type": "link",
                    "destination": "javascript:alert(1)",
                    "title": None,
                    "children": [{"type": "text", "value": "click"}],
                }
            ],
        }
        html = to_html(make_doc(node))
        assert 'href=""' in html

    def test_data_link_blocked(self) -> None:
        node: ParagraphNode = {
            "type": "paragraph",
            "children": [
                {
                    "type": "link",
                    "destination": "data:text/html,<script>",
                    "title": None,
                    "children": [{"type": "text", "value": "click"}],
                }
            ],
        }
        html = to_html(make_doc(node))
        assert 'href=""' in html

    def test_https_link_allowed(self) -> None:
        node: ParagraphNode = {
            "type": "paragraph",
            "children": [
                {
                    "type": "link",
                    "destination": "https://example.com",
                    "title": None,
                    "children": [{"type": "text", "value": "click"}],
                }
            ],
        }
        html = to_html(make_doc(node))
        assert "https://example.com" in html


class TestEndToEnd:
    """End-to-end tests using parse() + to_html()."""

    def test_heading_paragraph(self) -> None:
        html = to_html(parse("# Hello\n\nWorld\n"))
        assert html == "<h1>Hello</h1>\n<p>World</p>\n"

    def test_emphasis_strong(self) -> None:
        html = to_html(parse("*em* **strong**\n"))
        assert "<em>em</em>" in html
        assert "<strong>strong</strong>" in html

    def test_link_in_paragraph(self) -> None:
        html = to_html(parse("[click](https://example.com)\n"))
        assert 'href="https://example.com"' in html

    def test_ordered_list(self) -> None:
        html = to_html(parse("1. one\n2. two\n"))
        assert "<ol>" in html
        assert "<li>one</li>" in html

    def test_code_block_escaping(self) -> None:
        html = to_html(parse("    a & b < c\n"))
        assert "&amp;" in html
        assert "&lt;" in html
