# frozen_string_literal: true

require_relative "test_helper"

# Tests for CodingAdventures::Asciidoc pipeline package.
# Verifies the full AsciiDoc → HTML round-trip.
class TestAsciidoc < Minitest::Test
  def to_html(text)
    CodingAdventures::Asciidoc.to_html(text)
  end

  # ── Headings ─────────────────────────────────────────────────────────────────

  def test_h1
    assert_includes to_html("= Hello\n"), "<h1>Hello</h1>"
  end

  def test_h2
    assert_includes to_html("== Section\n"), "<h2>Section</h2>"
  end

  def test_h3
    assert_includes to_html("=== Sub\n"), "<h3>Sub</h3>"
  end

  # ── Paragraphs ───────────────────────────────────────────────────────────────

  def test_simple_paragraph
    assert_includes to_html("World\n"), "<p>World</p>"
  end

  def test_heading_and_paragraph
    html = to_html("= Title\n\nBody.\n")
    assert_includes html, "<h1>Title</h1>"
    assert_includes html, "<p>Body.</p>"
  end

  # ── Inline markup ────────────────────────────────────────────────────────────

  def test_strong_markup
    html = to_html("*bold*\n")
    assert_includes html, "<strong>bold</strong>"
  end

  def test_emphasis_markup
    html = to_html("_italic_\n")
    assert_includes html, "<em>italic</em>"
  end

  def test_code_span
    html = to_html("`code`\n")
    assert_includes html, "<code>code</code>"
  end

  # ── Code blocks ──────────────────────────────────────────────────────────────

  def test_code_block_no_lang
    html = to_html("----\nint x = 1;\n----\n")
    assert_includes html, "<pre>"
    assert_includes html, "int x = 1;"
  end

  def test_code_block_with_lang
    html = to_html("[source,python]\n----\nprint('hi')\n----\n")
    assert_includes html, "python"
  end

  # ── Lists ─────────────────────────────────────────────────────────────────────

  def test_unordered_list
    html = to_html("* one\n* two\n")
    assert_includes html, "<ul>"
    assert_includes html, "one"
    assert_includes html, "two"
  end

  def test_ordered_list
    html = to_html(". first\n. second\n")
    assert_includes html, "<ol>"
    assert_includes html, "first"
  end

  # ── Blockquote ───────────────────────────────────────────────────────────────

  def test_quote_block
    html = to_html("____\nA quote.\n____\n")
    assert_includes html, "<blockquote>"
    assert_includes html, "A quote."
  end

  # ── Thematic break ───────────────────────────────────────────────────────────

  def test_thematic_break
    html = to_html("'''\n")
    assert_includes html, "<hr"
  end

  # ── Links ─────────────────────────────────────────────────────────────────────

  def test_link_macro
    html = to_html("link:https://example.com[Click here]\n")
    assert_includes html, 'href="https://example.com"'
    assert_includes html, "Click here"
  end

  def test_bare_url
    html = to_html("https://example.com\n")
    assert_includes html, "https://example.com"
  end

  # ── Empty / comments ─────────────────────────────────────────────────────────

  def test_empty_input
    html = to_html("")
    assert html.empty? || html.strip.empty?
  end

  def test_comment_only
    html = to_html("// comment\n")
    refute_includes html, "<p>"
  end
end
