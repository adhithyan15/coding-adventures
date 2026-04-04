"""Tests for the coding_adventures_asciidoc pipeline package.

These tests verify the full AsciiDoc → HTML round-trip.
"""

from coding_adventures_asciidoc import to_html


class TestHeadings:
    def test_h1(self):
        assert "<h1>Hello</h1>" in to_html("= Hello\n")

    def test_h2(self):
        assert "<h2>Section</h2>" in to_html("== Section\n")

    def test_h3(self):
        assert "<h3>Sub</h3>" in to_html("=== Sub\n")


class TestParagraph:
    def test_simple_paragraph(self):
        assert "<p>World</p>" in to_html("World\n")

    def test_heading_and_paragraph(self):
        html = to_html("= Title\n\nBody.\n")
        assert "<h1>Title</h1>" in html
        assert "<p>Body.</p>" in html


class TestInlineMarkup:
    def test_strong(self):
        html = to_html("*bold*\n")
        assert "<strong>bold</strong>" in html

    def test_emphasis(self):
        html = to_html("_italic_\n")
        assert "<em>italic</em>" in html

    def test_code_span(self):
        html = to_html("`code`\n")
        assert "<code>code</code>" in html


class TestCodeBlock:
    def test_code_block_no_lang(self):
        html = to_html("----\nint x = 1;\n----\n")
        assert "<pre>" in html
        assert "int x = 1;" in html

    def test_code_block_with_lang(self):
        html = to_html("[source,python]\n----\nprint('hi')\n----\n")
        assert "python" in html


class TestList:
    def test_unordered_list(self):
        html = to_html("* one\n* two\n")
        assert "<ul>" in html
        assert "one" in html
        assert "two" in html

    def test_ordered_list(self):
        html = to_html(". first\n. second\n")
        assert "<ol>" in html
        assert "first" in html


class TestBlockquote:
    def test_quote_block(self):
        html = to_html("____\nA quote.\n____\n")
        assert "<blockquote>" in html
        assert "A quote." in html


class TestThematicBreak:
    def test_thematic_break(self):
        html = to_html("'''\n")
        assert "<hr" in html


class TestLinks:
    def test_link_macro(self):
        html = to_html("link:https://example.com[Click here]\n")
        assert 'href="https://example.com"' in html
        assert "Click here" in html

    def test_bare_url(self):
        html = to_html("https://example.com\n")
        assert "https://example.com" in html


class TestEmpty:
    def test_empty_input(self):
        html = to_html("")
        assert html == "" or html.strip() == ""

    def test_comment_only(self):
        html = to_html("// comment\n")
        # Comment should produce no visible output
        assert "<p>" not in html
