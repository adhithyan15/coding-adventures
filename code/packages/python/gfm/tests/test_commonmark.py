"""Tests for the commonmark pipeline package."""

from coding_adventures_gfm import RenderOptions, parse, to_html


class TestPipelineIntegration:
    def test_heading_paragraph(self) -> None:
        html = to_html(parse("# Hello\n\nWorld\n"))
        assert html == "<h1>Hello</h1>\n<p>World</p>\n"

    def test_emphasis_strong(self) -> None:
        html = to_html(parse("*em* **strong**\n"))
        assert "<em>em</em>" in html
        assert "<strong>strong</strong>" in html

    def test_unordered_list(self) -> None:
        html = to_html(parse("- item 1\n- item 2\n"))
        assert "<ul>" in html
        assert "<li>item 1</li>" in html

    def test_ordered_list(self) -> None:
        html = to_html(parse("1. one\n2. two\n"))
        assert "<ol>" in html

    def test_code_block(self) -> None:
        html = to_html(parse("```python\nprint('hello')\n```\n"))
        assert 'class="language-python"' in html

    def test_link(self) -> None:
        html = to_html(parse("[click](https://example.com)\n"))
        assert 'href="https://example.com"' in html

    def test_image(self) -> None:
        html = to_html(parse("![alt](cat.png)\n"))
        assert 'src="cat.png"' in html
        assert 'alt="alt"' in html

    def test_blockquote(self) -> None:
        html = to_html(parse("> quote\n"))
        assert "<blockquote>" in html

    def test_thematic_break(self) -> None:
        html = to_html(parse("---\n"))
        assert "<hr />" in html

    def test_sanitize_option(self) -> None:
        html = to_html(parse("<script>alert(1)</script>\n"), RenderOptions(sanitize=True))
        assert "<script>" not in html

    def test_empty_document(self) -> None:
        html = to_html(parse(""))
        assert html == ""

    def test_crlf_line_endings(self) -> None:
        html = to_html(parse("# Hello\r\n\r\nWorld\r\n"))
        assert html == "<h1>Hello</h1>\n<p>World</p>\n"

    def test_entity_decoding(self) -> None:
        html = to_html(parse("&amp; &lt; &gt;\n"))
        # Entities decoded then re-escaped for HTML output
        assert "&amp;" in html  # & decoded to & then re-escaped

    def test_hard_break(self) -> None:
        html = to_html(parse("line1  \nline2\n"))
        assert "<br />" in html

    def test_soft_break(self) -> None:
        html = to_html(parse("line1\nline2\n"))
        # soft break renders as newline
        assert "\n" in html

    def test_autolink(self) -> None:
        html = to_html(parse("<https://example.com>\n"))
        assert 'href="https://example.com"' in html

    def test_reference_links(self) -> None:
        html = to_html(parse("[text][ref]\n\n[ref]: https://example.com\n"))
        assert 'href="https://example.com"' in html

    def test_fenced_code_with_tilde(self) -> None:
        html = to_html(parse("~~~python\ncode\n~~~\n"))
        assert 'class="language-python"' in html

    def test_nested_blockquote(self) -> None:
        html = to_html(parse("> > nested\n"))
        assert html.count("<blockquote>") == 2

    def test_loose_list(self) -> None:
        html = to_html(parse("- item 1\n\n- item 2\n"))
        assert "<p>item 1</p>" in html

    def test_html_block_passthrough(self) -> None:
        html = to_html(parse("<div>\nhello\n</div>\n"))
        assert "<div>" in html

    def test_inline_html(self) -> None:
        html = to_html(parse("a <em>b</em> c\n"))
        assert "<em>b</em>" in html

    def test_strikethrough(self) -> None:
        html = to_html(parse("~~gone~~\n"))
        assert html == "<p><del>gone</del></p>\n"

    def test_task_list(self) -> None:
        html = to_html(parse("- [x] done\n"))
        assert '<input type="checkbox" disabled="" checked="" /> done' in html

    def test_table(self) -> None:
        html = to_html(parse("| A |\n| --- |\n| B |\n"))
        assert "<table>" in html
        assert "<th>A</th>" in html
        assert "<td>B</td>" in html
