"""Tests for coding_adventures_asciidoc_parser.

Covers all block-level and inline constructs defined in spec TE03.
"""

import pytest

from coding_adventures_asciidoc_parser import parse
from coding_adventures_asciidoc_parser.inline_parser import parse_inline


# ── Helpers ───────────────────────────────────────────────────────────────────


def children(doc):
    """Return the top-level block children of a parsed document."""
    return doc["children"]


def first(doc):
    """Return the first block child."""
    return children(doc)[0]


# ─────────────────────────────────────────────────────────────────────────────
# Block parser tests
# ─────────────────────────────────────────────────────────────────────────────


class TestDocumentRoot:
    def test_empty_string_gives_document(self):
        doc = parse("")
        assert doc["type"] == "document"
        assert doc["children"] == []

    def test_blank_only_gives_empty_document(self):
        doc = parse("\n\n\n")
        assert doc["children"] == []


class TestHeadings:
    def test_level_1(self):
        node = first(parse("= Hello\n"))
        assert node["type"] == "heading"
        assert node["level"] == 1
        assert node["children"][0]["value"] == "Hello"

    def test_level_2(self):
        node = first(parse("== Section\n"))
        assert node["level"] == 2

    def test_level_3(self):
        node = first(parse("=== Subsection\n"))
        assert node["level"] == 3

    def test_level_4(self):
        node = first(parse("==== Sub-sub\n"))
        assert node["level"] == 4

    def test_level_5(self):
        node = first(parse("===== Level5\n"))
        assert node["level"] == 5

    def test_level_6(self):
        node = first(parse("====== Level6\n"))
        assert node["level"] == 6

    def test_level_clamped_to_6(self):
        # Seven = signs should still produce level 6
        node = first(parse("======= Too deep\n"))
        assert node["level"] == 6

    def test_heading_with_inline(self):
        node = first(parse("= Hello *World*\n"))
        assert node["children"][0]["value"] == "Hello "
        assert node["children"][1]["type"] == "strong"


class TestParagraph:
    def test_simple_paragraph(self):
        node = first(parse("Hello world\n"))
        assert node["type"] == "paragraph"
        assert node["children"][0]["value"] == "Hello world"

    def test_multiline_paragraph(self):
        node = first(parse("Line one\nLine two\n"))
        assert node["type"] == "paragraph"
        # Should contain a soft break between lines
        types = [c["type"] for c in node["children"]]
        assert "soft_break" in types

    def test_two_paragraphs(self):
        doc = parse("First\n\nSecond\n")
        assert len(children(doc)) == 2
        assert children(doc)[0]["type"] == "paragraph"
        assert children(doc)[1]["type"] == "paragraph"


class TestThematicBreak:
    def test_three_single_quotes(self):
        node = first(parse("'''\n"))
        assert node["type"] == "thematic_break"

    def test_five_single_quotes(self):
        node = first(parse("'''''\n"))
        assert node["type"] == "thematic_break"


class TestCodeBlock:
    def test_simple_code_block(self):
        src = "----\nint x = 1;\n----\n"
        node = first(parse(src))
        assert node["type"] == "code_block"
        assert node["language"] is None
        assert "int x = 1;" in node["value"]

    def test_code_block_with_language(self):
        src = "[source,python]\n----\nprint('hi')\n----\n"
        node = first(parse(src))
        assert node["type"] == "code_block"
        assert node["language"] == "python"

    def test_code_block_with_language_spaced(self):
        src = "[source, ruby]\n----\nputs 'hi'\n----\n"
        node = first(parse(src))
        assert node["language"] == "ruby"

    def test_literal_block(self):
        src = "....\nsome literal text\n....\n"
        node = first(parse(src))
        assert node["type"] == "code_block"
        assert node["language"] is None
        assert "some literal text" in node["value"]

    def test_unclosed_code_block_is_lenient(self):
        src = "----\norphan content\n"
        node = first(parse(src))
        assert node["type"] == "code_block"
        assert "orphan content" in node["value"]


class TestPassthroughBlock:
    def test_raw_html_passthrough(self):
        src = "++++\n<div>raw</div>\n++++\n"
        node = first(parse(src))
        assert node["type"] == "raw_block"
        assert node["format"] == "html"
        assert "<div>raw</div>" in node["value"]


class TestQuoteBlock:
    def test_simple_quote_block(self):
        src = "____\nA quote.\n____\n"
        node = first(parse(src))
        assert node["type"] == "blockquote"
        assert len(node["children"]) > 0

    def test_quote_block_content_parsed(self):
        src = "____\n= Inner heading\n____\n"
        node = first(parse(src))
        assert node["type"] == "blockquote"
        inner = node["children"][0]
        assert inner["type"] == "heading"


class TestUnorderedList:
    def test_simple_unordered_list(self):
        src = "* Item A\n* Item B\n* Item C\n"
        node = first(parse(src))
        assert node["type"] == "list"
        assert node["ordered"] is False
        assert len(node["children"]) == 3

    def test_unordered_list_item_text(self):
        src = "* Hello\n"
        node = first(parse(src))
        item = node["children"][0]
        assert item["type"] == "list_item"
        text_node = item["children"][0]["children"][0]
        assert text_node["value"] == "Hello"


class TestOrderedList:
    def test_simple_ordered_list(self):
        src = ". First\n. Second\n. Third\n"
        node = first(parse(src))
        assert node["type"] == "list"
        assert node["ordered"] is True
        assert len(node["children"]) == 3

    def test_ordered_list_start(self):
        src = ". One\n"
        node = first(parse(src))
        assert node["start"] == 1


class TestComments:
    def test_line_comment_is_skipped(self):
        src = "// This is a comment\nHello\n"
        doc = parse(src)
        # Only one block — the paragraph. Comment is not in output.
        assert len(children(doc)) == 1
        assert children(doc)[0]["type"] == "paragraph"

    def test_comment_only_document(self):
        src = "// Comment only\n"
        doc = parse(src)
        assert children(doc) == []


class TestMixedBlocks:
    def test_heading_then_paragraph(self):
        src = "= Title\n\nSome text.\n"
        nodes = children(parse(src))
        assert nodes[0]["type"] == "heading"
        assert nodes[1]["type"] == "paragraph"

    def test_paragraph_then_list(self):
        src = "Intro\n\n* a\n* b\n"
        nodes = children(parse(src))
        assert nodes[0]["type"] == "paragraph"
        assert nodes[1]["type"] == "list"


# ─────────────────────────────────────────────────────────────────────────────
# Inline parser tests
# ─────────────────────────────────────────────────────────────────────────────


class TestInlineText:
    def test_plain_text(self):
        nodes = parse_inline("hello world")
        assert len(nodes) == 1
        assert nodes[0]["type"] == "text"
        assert nodes[0]["value"] == "hello world"

    def test_empty_string(self):
        assert parse_inline("") == []


class TestInlineStrong:
    def test_single_star(self):
        nodes = parse_inline("*bold*")
        assert nodes[0]["type"] == "strong"
        assert nodes[0]["children"][0]["value"] == "bold"

    def test_double_star(self):
        nodes = parse_inline("**bold**")
        assert nodes[0]["type"] == "strong"

    def test_strong_in_sentence(self):
        nodes = parse_inline("Hello *world* foo")
        assert nodes[0]["value"] == "Hello "
        assert nodes[1]["type"] == "strong"
        assert nodes[2]["value"] == " foo"


class TestInlineEmphasis:
    def test_single_underscore(self):
        nodes = parse_inline("_italic_")
        assert nodes[0]["type"] == "emphasis"
        assert nodes[0]["children"][0]["value"] == "italic"

    def test_double_underscore(self):
        nodes = parse_inline("__italic__")
        assert nodes[0]["type"] == "emphasis"


class TestInlineCodeSpan:
    def test_backtick_code(self):
        nodes = parse_inline("`code`")
        assert nodes[0]["type"] == "code_span"
        assert nodes[0]["value"] == "code"

    def test_code_span_is_verbatim(self):
        # Markup inside code span must NOT be parsed
        nodes = parse_inline("`*not bold*`")
        assert nodes[0]["type"] == "code_span"
        assert nodes[0]["value"] == "*not bold*"


class TestInlineLinks:
    def test_link_macro(self):
        nodes = parse_inline("link:https://example.com[Click]")
        assert nodes[0]["type"] == "link"
        assert nodes[0]["destination"] == "https://example.com"
        assert nodes[0]["children"][0]["value"] == "Click"

    def test_link_macro_empty_text_uses_url(self):
        nodes = parse_inline("link:https://example.com[]")
        assert nodes[0]["destination"] == "https://example.com"
        assert nodes[0]["children"][0]["value"] == "https://example.com"

    def test_https_with_bracket_text(self):
        nodes = parse_inline("https://example.com[Go]")
        assert nodes[0]["type"] == "link"
        assert nodes[0]["children"][0]["value"] == "Go"

    def test_bare_url_autolink(self):
        nodes = parse_inline("https://example.com")
        assert nodes[0]["type"] == "autolink"
        assert nodes[0]["destination"] == "https://example.com"
        assert nodes[0]["is_email"] is False

    def test_http_autolink(self):
        nodes = parse_inline("http://example.com")
        assert nodes[0]["type"] == "autolink"


class TestInlineImage:
    def test_image_macro(self):
        nodes = parse_inline("image:cat.png[A cat]")
        assert nodes[0]["type"] == "image"
        assert nodes[0]["destination"] == "cat.png"
        assert nodes[0]["alt"] == "A cat"


class TestInlineCrossRef:
    def test_xref_with_text(self):
        nodes = parse_inline("<<section-1,Section 1>>")
        assert nodes[0]["type"] == "link"
        assert nodes[0]["destination"] == "#section-1"
        assert nodes[0]["children"][0]["value"] == "Section 1"

    def test_xref_without_text(self):
        nodes = parse_inline("<<intro>>")
        assert nodes[0]["type"] == "link"
        assert nodes[0]["destination"] == "#intro"
        assert nodes[0]["children"][0]["value"] == "intro"


class TestInlineBreaks:
    def test_soft_break(self):
        nodes = parse_inline("line one\nline two")
        types = [n["type"] for n in nodes]
        assert "soft_break" in types

    def test_hard_break_two_spaces(self):
        nodes = parse_inline("end  \nnext")
        types = [n["type"] for n in nodes]
        assert "hard_break" in types

    def test_hard_break_backslash(self):
        nodes = parse_inline("end\\\nnext")
        types = [n["type"] for n in nodes]
        assert "hard_break" in types


class TestNestedInline:
    def test_strong_with_emphasis_inside(self):
        # In AsciiDoc, ** ... ** is strong; _..._ inside is emphasis
        nodes = parse_inline("**hello _world_**")
        assert nodes[0]["type"] == "strong"
        inner_types = [c["type"] for c in nodes[0]["children"]]
        assert "emphasis" in inner_types
