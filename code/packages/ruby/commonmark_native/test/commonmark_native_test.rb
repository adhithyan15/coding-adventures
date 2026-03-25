# frozen_string_literal: true

# --------------------------------------------------------------------------
# commonmark_native_test.rb — Tests for the Rust-backed CommonMark extension
# --------------------------------------------------------------------------
#
# These tests exercise every public function of the native extension to
# ensure the Rust `commonmark` crate is correctly exposed to Ruby.
# Tests cover all CommonMark block elements, inline elements, lists,
# code blocks, and the safe vs. unsafe rendering modes.

require_relative "test_helper"

M = CodingAdventures::CommonmarkNative

class MarkdownToHtmlBlocksTest < Minitest::Test
  # ========================================================================
  # ATX headings (# H1 through ###### H6)
  # ========================================================================

  def test_h1
    assert_equal "<h1>Hello</h1>\n", M.markdown_to_html("# Hello\n")
  end

  def test_h2
    assert_equal "<h2>Hello</h2>\n", M.markdown_to_html("## Hello\n")
  end

  def test_h3
    assert_equal "<h3>Hello</h3>\n", M.markdown_to_html("### Hello\n")
  end

  def test_h4
    assert_equal "<h4>Hello</h4>\n", M.markdown_to_html("#### Hello\n")
  end

  def test_h5
    assert_equal "<h5>Hello</h5>\n", M.markdown_to_html("##### Hello\n")
  end

  def test_h6
    assert_equal "<h6>Hello</h6>\n", M.markdown_to_html("###### Hello\n")
  end

  def test_paragraph
    assert_equal "<p>Hello world</p>\n", M.markdown_to_html("Hello world\n")
  end

  def test_multiple_paragraphs
    result = M.markdown_to_html("First\n\nSecond\n")
    assert_includes result, "<p>First</p>"
    assert_includes result, "<p>Second</p>"
  end

  def test_empty_string
    assert_equal "", M.markdown_to_html("")
  end

  def test_blank_lines_only
    assert_equal "", M.markdown_to_html("\n\n\n")
  end

  def test_blockquote
    result = M.markdown_to_html("> A quote\n")
    assert_includes result, "<blockquote>"
    assert_includes result, "A quote"
  end

  def test_thematic_break
    assert_includes M.markdown_to_html("---\n"), "<hr"
  end
end

class MarkdownToHtmlInlineTest < Minitest::Test
  # ========================================================================
  # Inline elements: emphasis, strong, code, links, images
  # ========================================================================

  def test_emphasis_asterisk
    assert_equal "<p>Hello <em>world</em></p>\n", M.markdown_to_html("Hello *world*\n")
  end

  def test_emphasis_underscore
    assert_equal "<p>Hello <em>world</em></p>\n", M.markdown_to_html("Hello _world_\n")
  end

  def test_strong_asterisk
    assert_equal "<p>Hello <strong>world</strong></p>\n",
                 M.markdown_to_html("Hello **world**\n")
  end

  def test_strong_underscore
    assert_equal "<p>Hello <strong>world</strong></p>\n",
                 M.markdown_to_html("Hello __world__\n")
  end

  def test_inline_code
    result = M.markdown_to_html("Use `print()` to output\n")
    assert_includes result, "<code>print()</code>"
  end

  def test_inline_link
    result = M.markdown_to_html("[GitHub](https://github.com)\n")
    assert_includes result, '<a href="https://github.com">GitHub</a>'
  end

  def test_inline_image
    result = M.markdown_to_html("![Alt text](image.png)\n")
    assert_includes result, "<img"
    assert_includes result, 'alt="Alt text"'
    assert_includes result, 'src="image.png"'
  end

  def test_hard_break
    # Two trailing spaces create a hard line break
    result = M.markdown_to_html("line one  \nline two\n")
    assert_includes result, "<br"
  end
end

class MarkdownToHtmlCodeBlocksTest < Minitest::Test
  # ========================================================================
  # Fenced and indented code blocks
  # ========================================================================

  def test_fenced_code_block
    result = M.markdown_to_html("```\nhello world\n```\n")
    assert_includes result, "<code>"
    assert_includes result, "hello world"
  end

  def test_fenced_code_block_with_language
    result = M.markdown_to_html("```python\nprint('hello')\n```\n")
    assert_includes result, "python"
    assert_includes result, "print"
  end

  def test_indented_code_block
    result = M.markdown_to_html("    code here\n")
    assert_includes result, "<code>"
    assert_includes result, "code here"
  end
end

class MarkdownToHtmlListsTest < Minitest::Test
  # ========================================================================
  # Bullet and ordered lists
  # ========================================================================

  def test_unordered_list
    result = M.markdown_to_html("- Item 1\n- Item 2\n- Item 3\n")
    assert_includes result, "<ul>"
    assert_includes result, "<li>Item 1</li>"
    assert_includes result, "<li>Item 2</li>"
  end

  def test_ordered_list
    result = M.markdown_to_html("1. First\n2. Second\n3. Third\n")
    assert_includes result, "<ol>"
    assert_includes result, "<li>First</li>"
    assert_includes result, "<li>Second</li>"
  end

  def test_nested_list
    result = M.markdown_to_html("- Item 1\n  - Sub-item\n")
    assert_includes result, "<ul>"
    assert_includes result, "Sub-item"
  end
end

class MarkdownToHtmlRawHtmlTest < Minitest::Test
  # ========================================================================
  # Raw HTML passthrough (trusted mode)
  # ========================================================================

  def test_raw_block_passthrough
    result = M.markdown_to_html("<div>raw content</div>\n\nparagraph\n")
    assert_includes result, "<div>raw content</div>"
    assert_includes result, "<p>paragraph</p>"
  end

  def test_html_comment_passthrough
    result = M.markdown_to_html("<!-- a comment -->\n\nparagraph\n")
    assert_includes result, "<!-- a comment -->"
  end
end

class MarkdownToHtmlCombinedTest < Minitest::Test
  # ========================================================================
  # Combined / integration tests
  # ========================================================================

  def test_heading_and_paragraph
    result = M.markdown_to_html("# Title\n\nSome text.\n")
    assert_includes result, "<h1>Title</h1>"
    assert_includes result, "<p>Some text.</p>"
  end

  def test_full_document
    doc = <<~MD
      # My Document

      A paragraph with **bold** and *emphasis*.

      ## Section

      - Bullet one
      - Bullet two

      ```python
      print('hello')
      ```
    MD
    result = M.markdown_to_html(doc)
    assert_includes result, "<h1>My Document</h1>"
    assert_includes result, "<strong>bold</strong>"
    assert_includes result, "<em>emphasis</em>"
    assert_includes result, "<h2>Section</h2>"
    assert_includes result, "<li>Bullet one</li>"
    assert_includes result, "python"
  end

  def test_unicode
    result = M.markdown_to_html("# こんにちは\n\nHello 世界\n")
    assert_includes result, "こんにちは"
    assert_includes result, "世界"
  end

  def test_returns_string
    assert_instance_of String, M.markdown_to_html("# Hello\n")
  end
end

class MarkdownToHtmlErrorsTest < Minitest::Test
  # ========================================================================
  # Error handling: non-string arguments should raise
  # ========================================================================

  def test_raises_on_nil
    assert_raises(ArgumentError, TypeError) { M.markdown_to_html(nil) }
  end

  def test_raises_on_integer
    assert_raises(ArgumentError, TypeError) { M.markdown_to_html(42) }
  end

  def test_raises_on_array
    assert_raises(ArgumentError, TypeError) { M.markdown_to_html([]) }
  end
end

class MarkdownToHtmlSafeTest < Minitest::Test
  # ========================================================================
  # markdown_to_html_safe: XSS prevention
  # ========================================================================

  def test_strips_script_tag
    result = M.markdown_to_html_safe("<script>alert(1)</script>\n\n**bold**\n")
    refute_includes result, "<script>"
    assert_includes result, "<strong>bold</strong>"
  end

  def test_strips_raw_block
    result = M.markdown_to_html_safe("<div class='evil'>content</div>\n\nparagraph\n")
    refute_includes result, "<div"
    assert_includes result, "<p>paragraph</p>"
  end

  def test_preserves_markdown_formatting
    result = M.markdown_to_html_safe("# Heading\n\n**bold** and *em*\n")
    assert_includes result, "<h1>Heading</h1>"
    assert_includes result, "<strong>bold</strong>"
    assert_includes result, "<em>em</em>"
  end

  def test_empty_string
    assert_equal "", M.markdown_to_html_safe("")
  end

  def test_safe_link
    result = M.markdown_to_html_safe("[GitHub](https://github.com)\n")
    assert_includes result, '<a href="https://github.com">'
  end

  def test_raises_on_non_string
    assert_raises(ArgumentError, TypeError) { M.markdown_to_html_safe(42) }
  end
end

class SafeVsUnsafeTest < Minitest::Test
  # ========================================================================
  # Contrast: raw HTML present in unsafe, stripped in safe
  # ========================================================================

  def test_raw_html_present_in_unsafe_stripped_in_safe
    md = "<div>content</div>\n\nparagraph\n"
    assert_includes M.markdown_to_html(md), "<div>"
    refute_includes M.markdown_to_html_safe(md), "<div>"
  end

  def test_script_stripped_in_safe
    md = "<script>evil()</script>\n\ntext\n"
    assert_includes M.markdown_to_html(md), "<script>"
    refute_includes M.markdown_to_html_safe(md), "<script>"
  end

  def test_markdown_identical_when_no_raw_html
    md = "# Title\n\n**Bold** and *italic*.\n"
    assert_equal M.markdown_to_html(md), M.markdown_to_html_safe(md)
  end
end
