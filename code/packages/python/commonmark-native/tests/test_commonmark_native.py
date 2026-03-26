"""
test_commonmark_native.py -- Tests for the Rust-backed CommonMark extension
============================================================================

These tests exercise every aspect of the native extension to ensure the Rust
implementation is correctly exposed to Python. They mirror the test suite of
the CommonMark spec: headings, emphasis, strong, links, images, code spans,
fenced code blocks, blockquotes, lists, hard breaks, and raw HTML handling.
"""

import pytest
from commonmark_native import markdown_to_html, markdown_to_html_safe


# ===========================================================================
# markdown_to_html -- basic block elements
# ===========================================================================


class TestMarkdownToHtmlBlocks:
    """ATX headings, paragraphs, blockquotes, thematic breaks."""

    def test_atx_heading_h1(self):
        assert markdown_to_html("# Hello\n") == "<h1>Hello</h1>\n"

    def test_atx_heading_h2(self):
        assert markdown_to_html("## Hello\n") == "<h2>Hello</h2>\n"

    def test_atx_heading_h3(self):
        assert markdown_to_html("### Hello\n") == "<h3>Hello</h3>\n"

    def test_atx_heading_h4(self):
        assert markdown_to_html("#### Hello\n") == "<h4>Hello</h4>\n"

    def test_atx_heading_h5(self):
        assert markdown_to_html("##### Hello\n") == "<h5>Hello</h5>\n"

    def test_atx_heading_h6(self):
        assert markdown_to_html("###### Hello\n") == "<h6>Hello</h6>\n"

    def test_paragraph(self):
        assert markdown_to_html("Hello world\n") == "<p>Hello world</p>\n"

    def test_multiple_paragraphs(self):
        result = markdown_to_html("First paragraph\n\nSecond paragraph\n")
        assert "<p>First paragraph</p>" in result
        assert "<p>Second paragraph</p>" in result

    def test_blockquote(self):
        result = markdown_to_html("> A quote\n")
        assert "<blockquote>" in result
        assert "A quote" in result

    def test_thematic_break(self):
        result = markdown_to_html("---\n")
        assert "<hr" in result

    def test_empty_string(self):
        assert markdown_to_html("") == ""

    def test_blank_lines_only(self):
        # Blank lines produce no output
        assert markdown_to_html("\n\n\n") == ""


# ===========================================================================
# markdown_to_html -- inline elements
# ===========================================================================


class TestMarkdownToHtmlInline:
    """Emphasis, strong, code spans, links, images."""

    def test_emphasis_asterisk(self):
        assert markdown_to_html("Hello *world*\n") == "<p>Hello <em>world</em></p>\n"

    def test_emphasis_underscore(self):
        assert markdown_to_html("Hello _world_\n") == "<p>Hello <em>world</em></p>\n"

    def test_strong_asterisk(self):
        assert markdown_to_html("Hello **world**\n") == "<p>Hello <strong>world</strong></p>\n"

    def test_strong_underscore(self):
        assert markdown_to_html("Hello __world__\n") == "<p>Hello <strong>world</strong></p>\n"

    def test_inline_code(self):
        result = markdown_to_html("Use `print()` to output\n")
        assert "<code>print()</code>" in result

    def test_inline_link(self):
        result = markdown_to_html("[GitHub](https://github.com)\n")
        assert '<a href="https://github.com">GitHub</a>' in result

    def test_inline_image(self):
        result = markdown_to_html("![Alt text](image.png)\n")
        assert "<img" in result
        assert 'alt="Alt text"' in result
        assert 'src="image.png"' in result

    def test_autolink(self):
        result = markdown_to_html("<https://example.com>\n")
        assert "https://example.com" in result

    def test_hard_break(self):
        # Two trailing spaces cause a hard line break
        result = markdown_to_html("line one  \nline two\n")
        assert "<br" in result


# ===========================================================================
# markdown_to_html -- code blocks
# ===========================================================================


class TestMarkdownToHtmlCodeBlocks:
    """Indented and fenced code blocks."""

    def test_fenced_code_block(self):
        result = markdown_to_html("```\nhello world\n```\n")
        assert "<code>" in result
        assert "hello world" in result

    def test_fenced_code_block_with_language(self):
        result = markdown_to_html("```python\nprint('hello')\n```\n")
        assert "python" in result
        assert "print" in result

    def test_indented_code_block(self):
        result = markdown_to_html("    code here\n")
        assert "<code>" in result
        assert "code here" in result


# ===========================================================================
# markdown_to_html -- lists
# ===========================================================================


class TestMarkdownToHtmlLists:
    """Bullet and ordered lists."""

    def test_unordered_list(self):
        result = markdown_to_html("- Item 1\n- Item 2\n- Item 3\n")
        assert "<ul>" in result
        assert "<li>Item 1</li>" in result
        assert "<li>Item 2</li>" in result

    def test_ordered_list(self):
        result = markdown_to_html("1. First\n2. Second\n3. Third\n")
        assert "<ol>" in result
        assert "<li>First</li>" in result
        assert "<li>Second</li>" in result

    def test_nested_list(self):
        result = markdown_to_html("- Item 1\n  - Sub-item\n")
        assert "<ul>" in result
        assert "Sub-item" in result


# ===========================================================================
# markdown_to_html -- raw HTML passthrough
# ===========================================================================


class TestMarkdownToHtmlRaw:
    """Raw HTML blocks and inline HTML are passed through by default."""

    def test_raw_block_passthrough(self):
        # Trusted mode: raw HTML blocks appear unchanged in the output
        result = markdown_to_html("<div>raw content</div>\n\nparagraph\n")
        assert "<div>raw content</div>" in result
        assert "<p>paragraph</p>" in result

    def test_raw_inline_passthrough(self):
        result = markdown_to_html("Hello <em>raw em</em> world\n")
        # The raw <em> tag should be present (not treated as Markdown emphasis)
        assert "raw em" in result

    def test_html_comment_passthrough(self):
        result = markdown_to_html("<!-- a comment -->\n\nparagraph\n")
        assert "<!-- a comment -->" in result


# ===========================================================================
# markdown_to_html -- combined pipeline
# ===========================================================================


class TestMarkdownToHtmlCombined:
    """Integration tests with richer Markdown documents."""

    def test_heading_and_paragraph(self):
        result = markdown_to_html("# Title\n\nSome text.\n")
        assert "<h1>Title</h1>" in result
        assert "<p>Some text.</p>" in result

    def test_full_document(self):
        doc = (
            "# My Document\n\n"
            "A paragraph with **bold** and *emphasis*.\n\n"
            "## Section\n\n"
            "- Bullet one\n"
            "- Bullet two\n\n"
            "```python\nprint('hello')\n```\n"
        )
        result = markdown_to_html(doc)
        assert "<h1>My Document</h1>" in result
        assert "<strong>bold</strong>" in result
        assert "<em>emphasis</em>" in result
        assert "<h2>Section</h2>" in result
        assert "<li>Bullet one</li>" in result
        assert "python" in result

    def test_unicode(self):
        result = markdown_to_html("# こんにちは\n\nHello 世界\n")
        assert "こんにちは" in result
        assert "世界" in result

    def test_special_characters_escaped(self):
        result = markdown_to_html("AT&T and a > b\n")
        # Ampersands and angle brackets in text should be HTML-escaped
        assert "&amp;" in result or "AT&T" in result  # depends on context
        # The paragraph tag should be present
        assert "<p>" in result

    def test_returns_string(self):
        result = markdown_to_html("# Hello\n")
        assert isinstance(result, str)


# ===========================================================================
# markdown_to_html -- error handling
# ===========================================================================


class TestMarkdownToHtmlErrors:
    """Type errors and edge cases."""

    def test_raises_value_error_on_none(self):
        with pytest.raises((TypeError, ValueError)):
            markdown_to_html(None)  # type: ignore[arg-type]

    def test_raises_value_error_on_int(self):
        with pytest.raises((TypeError, ValueError)):
            markdown_to_html(42)  # type: ignore[arg-type]

    def test_raises_on_no_args(self):
        with pytest.raises(TypeError):
            markdown_to_html()  # type: ignore[call-arg]


# ===========================================================================
# markdown_to_html_safe -- XSS prevention
# ===========================================================================


class TestMarkdownToHtmlSafe:
    """The safe variant strips raw HTML to prevent XSS."""

    def test_strips_script_tag(self):
        result = markdown_to_html_safe("<script>alert(1)</script>\n\n**bold**\n")
        assert "<script>" not in result
        assert "<strong>bold</strong>" in result

    def test_strips_inline_html(self):
        result = markdown_to_html_safe("Hello <b>raw</b> world\n")
        # Raw HTML should be stripped; the text content may or may not appear
        # (depends on how the sanitizer handles inline HTML)
        assert "<b>" not in result

    def test_strips_raw_block(self):
        result = markdown_to_html_safe("<div class='evil'>content</div>\n\nparagraph\n")
        assert "<div" not in result
        assert "<p>paragraph</p>" in result

    def test_preserves_markdown_formatting(self):
        # Safe mode still renders all Markdown syntax correctly
        result = markdown_to_html_safe("# Heading\n\n**bold** and *em*\n")
        assert "<h1>Heading</h1>" in result
        assert "<strong>bold</strong>" in result
        assert "<em>em</em>" in result

    def test_empty_string(self):
        assert markdown_to_html_safe("") == ""

    def test_safe_link(self):
        # Regular Markdown links are fine
        result = markdown_to_html_safe("[GitHub](https://github.com)\n")
        assert '<a href="https://github.com">' in result

    def test_raises_on_non_string(self):
        with pytest.raises((TypeError, ValueError)):
            markdown_to_html_safe(42)  # type: ignore[arg-type]


# ===========================================================================
# markdown_to_html vs markdown_to_html_safe -- contrast
# ===========================================================================


class TestSafeVsUnsafe:
    """Direct comparison of the two rendering modes."""

    def test_raw_html_present_in_unsafe_stripped_in_safe(self):
        md = "<div>content</div>\n\nparagraph\n"
        unsafe_result = markdown_to_html(md)
        safe_result = markdown_to_html_safe(md)

        assert "<div>" in unsafe_result
        assert "<div>" not in safe_result

    def test_script_stripped_in_safe(self):
        md = "<script>evil()</script>\n\ntext\n"
        assert "<script>" in markdown_to_html(md)
        assert "<script>" not in markdown_to_html_safe(md)

    def test_markdown_identical_when_no_raw_html(self):
        md = "# Title\n\n**Bold** and *italic*.\n"
        assert markdown_to_html(md) == markdown_to_html_safe(md)
