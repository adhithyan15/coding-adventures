"""Unit tests for the GFM parser.

Tests basic parsing functionality beyond the spec test suite.
"""

from coding_adventures_gfm_parser import parse


class TestDocumentStructure:
    def test_empty_document(self) -> None:
        doc = parse("")
        assert doc["type"] == "document"
        assert doc["children"] == []

    def test_empty_document_newline(self) -> None:
        doc = parse("\n")
        assert doc["type"] == "document"
        assert doc["children"] == []

    def test_simple_paragraph(self) -> None:
        doc = parse("Hello world\n")
        assert doc["type"] == "document"
        assert len(doc["children"]) == 1
        assert doc["children"][0]["type"] == "paragraph"

    def test_atx_heading(self) -> None:
        doc = parse("# Hello\n")
        assert len(doc["children"]) == 1
        h = doc["children"][0]
        assert h["type"] == "heading"
        assert h["level"] == 1

    def test_headings_level_1_through_6(self) -> None:
        for level in range(1, 7):
            doc = parse("#" * level + " Heading\n")
            h = doc["children"][0]
            assert h["level"] == level

    def test_setext_heading(self) -> None:
        doc = parse("Hello\n=====\n")
        assert doc["children"][0]["type"] == "heading"
        assert doc["children"][0]["level"] == 1

    def test_setext_heading_level2(self) -> None:
        doc = parse("Hello\n-----\n")
        assert doc["children"][0]["type"] == "heading"
        assert doc["children"][0]["level"] == 2

    def test_thematic_break(self) -> None:
        doc = parse("---\n")
        assert doc["children"][0]["type"] == "thematic_break"

    def test_fenced_code_block(self) -> None:
        doc = parse("```python\nprint('hello')\n```\n")
        cb = doc["children"][0]
        assert cb["type"] == "code_block"
        assert cb["language"] == "python"
        assert "print" in cb["value"]

    def test_indented_code_block(self) -> None:
        doc = parse("    code here\n")
        cb = doc["children"][0]
        assert cb["type"] == "code_block"
        assert cb["language"] is None

    def test_blockquote(self) -> None:
        doc = parse("> quote\n")
        assert doc["children"][0]["type"] == "blockquote"

    def test_unordered_list(self) -> None:
        doc = parse("- item 1\n- item 2\n")
        lst = doc["children"][0]
        assert lst["type"] == "list"
        assert lst["ordered"] is False
        assert len(lst["children"]) == 2

    def test_ordered_list(self) -> None:
        doc = parse("1. item 1\n2. item 2\n")
        lst = doc["children"][0]
        assert lst["type"] == "list"
        assert lst["ordered"] is True
        assert lst["start"] == 1

    def test_html_block(self) -> None:
        doc = parse("<div>\nhello\n</div>\n")
        rb = doc["children"][0]
        assert rb["type"] == "raw_block"
        assert rb["format"] == "html"

    def test_task_list_item(self) -> None:
        doc = parse("- [x] done\n- [ ] todo\n")
        lst = doc["children"][0]
        assert lst["children"][0]["type"] == "task_item"
        assert lst["children"][0]["checked"] is True
        assert lst["children"][1]["type"] == "task_item"
        assert lst["children"][1]["checked"] is False

    def test_pipe_table(self) -> None:
        doc = parse("| A |\n| --- |\n| B |\n")
        table = doc["children"][0]
        assert table["type"] == "table"
        assert table["children"][0]["isHeader"] is True
        assert table["children"][0]["children"][0]["children"][0]["value"] == "A"


class TestInlineNodes:
    def test_text_node(self) -> None:
        doc = parse("Hello world\n")
        para = doc["children"][0]
        assert para["children"][0]["type"] == "text"
        assert para["children"][0]["value"] == "Hello world"

    def test_emphasis(self) -> None:
        doc = parse("*hello*\n")
        para = doc["children"][0]
        # Find emphasis in children
        em = para["children"][0]
        assert em["type"] == "emphasis"

    def test_strong(self) -> None:
        doc = parse("**hello**\n")
        para = doc["children"][0]
        strong = para["children"][0]
        assert strong["type"] == "strong"

    def test_strikethrough(self) -> None:
        doc = parse("~~hello~~\n")
        para = doc["children"][0]
        struck = para["children"][0]
        assert struck["type"] == "strikethrough"

    def test_code_span(self) -> None:
        doc = parse("`code`\n")
        para = doc["children"][0]
        cs = para["children"][0]
        assert cs["type"] == "code_span"
        assert cs["value"] == "code"

    def test_link(self) -> None:
        doc = parse("[text](https://example.com)\n")
        para = doc["children"][0]
        link = para["children"][0]
        assert link["type"] == "link"
        assert link["destination"] == "https://example.com"

    def test_image(self) -> None:
        doc = parse("![alt](image.png)\n")
        para = doc["children"][0]
        img = para["children"][0]
        assert img["type"] == "image"
        assert img["alt"] == "alt"

    def test_autolink_url(self) -> None:
        doc = parse("<https://example.com>\n")
        para = doc["children"][0]
        al = para["children"][0]
        assert al["type"] == "autolink"
        assert al["is_email"] is False

    def test_autolink_email(self) -> None:
        doc = parse("<user@example.com>\n")
        para = doc["children"][0]
        al = para["children"][0]
        assert al["type"] == "autolink"
        assert al["is_email"] is True

    def test_hard_break(self) -> None:
        doc = parse("line1  \nline2\n")
        para = doc["children"][0]
        # Find the hard break
        hb_nodes = [n for n in para["children"] if n["type"] == "hard_break"]
        assert len(hb_nodes) == 1

    def test_soft_break(self) -> None:
        doc = parse("line1\nline2\n")
        para = doc["children"][0]
        sb_nodes = [n for n in para["children"] if n["type"] == "soft_break"]
        assert len(sb_nodes) == 1

    def test_raw_inline(self) -> None:
        doc = parse("<em>raw</em>\n")
        para = doc["children"][0]
        ri = para["children"][0]
        assert ri["type"] == "raw_inline"
        assert ri["format"] == "html"


class TestLinkReferences:
    def test_reference_link(self) -> None:
        doc = parse("[text][ref]\n\n[ref]: https://example.com\n")
        para = doc["children"][0]
        link = para["children"][0]
        assert link["type"] == "link"
        assert link["destination"] == "https://example.com"

    def test_collapsed_reference(self) -> None:
        doc = parse("[foo][]\n\n[foo]: /url\n")
        para = doc["children"][0]
        link = para["children"][0]
        assert link["type"] == "link"
        assert link["destination"] == "/url"

    def test_shortcut_reference(self) -> None:
        doc = parse("[foo]\n\n[foo]: /url\n")
        para = doc["children"][0]
        link = para["children"][0]
        assert link["type"] == "link"

    def test_unresolved_reference_is_text(self) -> None:
        doc = parse("[text][no-such-ref]\n")
        para = doc["children"][0]
        # No link — should be text
        assert not any(n["type"] == "link" for n in para["children"])


class TestEntityDecoding:
    def test_named_entity_in_text(self) -> None:
        doc = parse("&amp; &lt; &gt;\n")
        para = doc["children"][0]
        text = para["children"][0]
        assert "&" in text["value"]
        assert "<" in text["value"]
        assert ">" in text["value"]

    def test_numeric_entity_decimal(self) -> None:
        doc = parse("&#65;\n")
        para = doc["children"][0]
        text = para["children"][0]
        assert "A" in text["value"]

    def test_numeric_entity_hex(self) -> None:
        doc = parse("&#x41;\n")
        para = doc["children"][0]
        text = para["children"][0]
        assert "A" in text["value"]
