# frozen_string_literal: true

require_relative "test_helper"

# Tests for CodingAdventures::AsciidocParser
#
# Covers all block-level and inline constructs defined in spec TE03.
class TestAsciidocParser < Minitest::Test
  include CodingAdventures::DocumentAst

  # ── Helpers ─────────────────────────────────────────────────────────────────

  def parse(text)
    CodingAdventures::AsciidocParser.parse(text)
  end

  def parse_inline(text)
    CodingAdventures::AsciidocParser::InlineParser.parse(text)
  end

  def children(doc)
    doc.children
  end

  def first_child(doc)
    doc.children.first
  end

  # ── Document root ────────────────────────────────────────────────────────────

  def test_empty_string_gives_document
    doc = parse("")
    assert_equal "document", doc.type
    assert_equal [], doc.children
  end

  def test_blank_only_gives_empty_document
    doc = parse("\n\n\n")
    assert_equal [], doc.children
  end

  # ── Headings ─────────────────────────────────────────────────────────────────

  def test_heading_level_1
    node = first_child(parse("= Hello\n"))
    assert_equal "heading", node.type
    assert_equal 1, node.level
    assert_equal "Hello", node.children.first.value
  end

  def test_heading_level_2
    node = first_child(parse("== Section\n"))
    assert_equal 2, node.level
  end

  def test_heading_level_3
    node = first_child(parse("=== Subsection\n"))
    assert_equal 3, node.level
  end

  def test_heading_level_4
    node = first_child(parse("==== Level4\n"))
    assert_equal 4, node.level
  end

  def test_heading_level_5
    node = first_child(parse("===== Level5\n"))
    assert_equal 5, node.level
  end

  def test_heading_level_6
    node = first_child(parse("====== Level6\n"))
    assert_equal 6, node.level
  end

  def test_heading_level_clamped_to_6
    node = first_child(parse("======= TooDeep\n"))
    assert_equal 6, node.level
  end

  def test_heading_with_inline_markup
    node = first_child(parse("= Hello *World*\n"))
    types = node.children.map(&:type)
    assert_includes types, "strong"
  end

  # ── Paragraphs ───────────────────────────────────────────────────────────────

  def test_simple_paragraph
    node = first_child(parse("Hello world\n"))
    assert_equal "paragraph", node.type
    assert_equal "Hello world", node.children.first.value
  end

  def test_multiline_paragraph_has_soft_break
    node = first_child(parse("Line one\nLine two\n"))
    types = node.children.map(&:type)
    assert_includes types, "soft_break"
  end

  def test_two_paragraphs
    doc = parse("First\n\nSecond\n")
    assert_equal 2, doc.children.length
    assert_equal "paragraph", doc.children[0].type
    assert_equal "paragraph", doc.children[1].type
  end

  # ── Thematic break ───────────────────────────────────────────────────────────

  def test_thematic_break_three_quotes
    node = first_child(parse("'''\n"))
    assert_equal "thematic_break", node.type
  end

  def test_thematic_break_five_quotes
    node = first_child(parse("'''''\n"))
    assert_equal "thematic_break", node.type
  end

  # ── Code blocks ──────────────────────────────────────────────────────────────

  def test_simple_code_block
    src = "----\nint x = 1;\n----\n"
    node = first_child(parse(src))
    assert_equal "code_block", node.type
    assert_nil node.language
    assert_includes node.value, "int x = 1;"
  end

  def test_code_block_with_language
    src = "[source,python]\n----\nprint('hi')\n----\n"
    node = first_child(parse(src))
    assert_equal "code_block", node.type
    assert_equal "python", node.language
  end

  def test_code_block_with_spaced_language
    src = "[source, ruby]\n----\nputs 'hi'\n----\n"
    node = first_child(parse(src))
    assert_equal "ruby", node.language
  end

  def test_literal_block
    src = "....\nsome text\n....\n"
    node = first_child(parse(src))
    assert_equal "code_block", node.type
    assert_nil node.language
    assert_includes node.value, "some text"
  end

  def test_unclosed_code_block_is_lenient
    node = first_child(parse("----\norphan\n"))
    assert_equal "code_block", node.type
    assert_includes node.value, "orphan"
  end

  # ── Passthrough block ────────────────────────────────────────────────────────

  def test_passthrough_block
    src = "++++\n<div>raw</div>\n++++\n"
    node = first_child(parse(src))
    assert_equal "raw_block", node.type
    assert_equal "html", node.format
    assert_includes node.value, "<div>raw</div>"
  end

  # ── Quote block ───────────────────────────────────────────────────────────────

  def test_quote_block_produces_blockquote
    src = "____\nA quote.\n____\n"
    node = first_child(parse(src))
    assert_equal "blockquote", node.type
    assert !node.children.empty?
  end

  def test_quote_block_content_is_parsed
    src = "____\n= Inner Heading\n____\n"
    node = first_child(parse(src))
    assert_equal "blockquote", node.type
    assert_equal "heading", node.children.first.type
  end

  # ── Lists ─────────────────────────────────────────────────────────────────────

  def test_unordered_list
    src = "* Item A\n* Item B\n* Item C\n"
    node = first_child(parse(src))
    assert_equal "list", node.type
    assert_equal false, node.ordered
    assert_equal 3, node.children.length
  end

  def test_ordered_list
    src = ". First\n. Second\n. Third\n"
    node = first_child(parse(src))
    assert_equal "list", node.type
    assert_equal true, node.ordered
    assert_equal 3, node.children.length
  end

  def test_ordered_list_start
    node = first_child(parse(". One\n"))
    assert_equal 1, node.start
  end

  def test_list_item_text
    src = "* Hello\n"
    node = first_child(parse(src))
    item = node.children.first
    assert_equal "list_item", item.type
    text_node = item.children.first.children.first
    assert_equal "Hello", text_node.value
  end

  # ── Comments ─────────────────────────────────────────────────────────────────

  def test_line_comment_is_skipped
    doc = parse("// Comment\nHello\n")
    assert_equal 1, doc.children.length
    assert_equal "paragraph", doc.children.first.type
  end

  def test_comment_only_document
    doc = parse("// Only a comment\n")
    assert_equal [], doc.children
  end

  # ── Mixed blocks ─────────────────────────────────────────────────────────────

  def test_heading_then_paragraph
    nodes = parse("= Title\n\nSome text.\n").children
    assert_equal "heading", nodes[0].type
    assert_equal "paragraph", nodes[1].type
  end

  def test_paragraph_then_list
    nodes = parse("Intro\n\n* a\n* b\n").children
    assert_equal "paragraph", nodes[0].type
    assert_equal "list", nodes[1].type
  end
end

# ── Inline parser tests ────────────────────────────────────────────────────────
class TestInlineParser < Minitest::Test
  include CodingAdventures::DocumentAst

  def parse_inline(text)
    CodingAdventures::AsciidocParser::InlineParser.parse(text)
  end

  # ── Text ──────────────────────────────────────────────────────────────────────

  def test_plain_text
    nodes = parse_inline("hello world")
    assert_equal 1, nodes.length
    assert_equal "text", nodes.first.type
    assert_equal "hello world", nodes.first.value
  end

  def test_empty_string
    assert_equal [], parse_inline("")
  end

  # ── Strong ────────────────────────────────────────────────────────────────────

  def test_single_star_is_strong
    nodes = parse_inline("*bold*")
    assert_equal "strong", nodes.first.type
    assert_equal "bold", nodes.first.children.first.value
  end

  def test_double_star_is_strong
    nodes = parse_inline("**bold**")
    assert_equal "strong", nodes.first.type
  end

  def test_strong_in_sentence
    nodes = parse_inline("Hello *world* foo")
    assert_equal "text", nodes[0].type
    assert_equal "strong", nodes[1].type
    assert_equal "text", nodes[2].type
  end

  # ── Emphasis ─────────────────────────────────────────────────────────────────

  def test_single_underscore_is_emphasis
    nodes = parse_inline("_italic_")
    assert_equal "emphasis", nodes.first.type
    assert_equal "italic", nodes.first.children.first.value
  end

  def test_double_underscore_is_emphasis
    nodes = parse_inline("__italic__")
    assert_equal "emphasis", nodes.first.type
  end

  # ── Code span ────────────────────────────────────────────────────────────────

  def test_backtick_code_span
    nodes = parse_inline("`code`")
    assert_equal "code_span", nodes.first.type
    assert_equal "code", nodes.first.value
  end

  def test_code_span_is_verbatim
    # Markup inside code span must NOT be parsed
    nodes = parse_inline("`*not bold*`")
    assert_equal "code_span", nodes.first.type
    assert_equal "*not bold*", nodes.first.value
  end

  # ── Links ─────────────────────────────────────────────────────────────────────

  def test_link_macro
    nodes = parse_inline("link:https://example.com[Click]")
    assert_equal "link", nodes.first.type
    assert_equal "https://example.com", nodes.first.destination
    assert_equal "Click", nodes.first.children.first.value
  end

  def test_link_macro_empty_text_uses_url
    nodes = parse_inline("link:https://example.com[]")
    assert_equal "https://example.com", nodes.first.children.first.value
  end

  def test_https_with_bracket_text
    nodes = parse_inline("https://example.com[Go]")
    assert_equal "link", nodes.first.type
    assert_equal "Go", nodes.first.children.first.value
  end

  def test_bare_https_is_autolink
    nodes = parse_inline("https://example.com")
    assert_equal "autolink", nodes.first.type
    assert_equal "https://example.com", nodes.first.destination
    assert_equal false, nodes.first.is_email
  end

  def test_bare_http_is_autolink
    nodes = parse_inline("http://example.com")
    assert_equal "autolink", nodes.first.type
  end

  # ── Image ────────────────────────────────────────────────────────────────────

  def test_image_macro
    nodes = parse_inline("image:cat.png[A cat]")
    assert_equal "image", nodes.first.type
    assert_equal "cat.png", nodes.first.destination
    assert_equal "A cat", nodes.first.alt
  end

  # ── Cross-references ─────────────────────────────────────────────────────────

  def test_xref_with_text
    nodes = parse_inline("<<section-1,Section 1>>")
    assert_equal "link", nodes.first.type
    assert_equal "#section-1", nodes.first.destination
    assert_equal "Section 1", nodes.first.children.first.value
  end

  def test_xref_without_text
    nodes = parse_inline("<<intro>>")
    assert_equal "link", nodes.first.type
    assert_equal "#intro", nodes.first.destination
    assert_equal "intro", nodes.first.children.first.value
  end

  # ── Breaks ───────────────────────────────────────────────────────────────────

  def test_soft_break
    nodes = parse_inline("line one\nline two")
    types = nodes.map(&:type)
    assert_includes types, "soft_break"
  end

  def test_hard_break_two_spaces
    nodes = parse_inline("end  \nnext")
    types = nodes.map(&:type)
    assert_includes types, "hard_break"
  end

  def test_hard_break_backslash
    nodes = parse_inline("end\\\nnext")
    types = nodes.map(&:type)
    assert_includes types, "hard_break"
  end

  # ── Nested inline ─────────────────────────────────────────────────────────────

  def test_strong_containing_emphasis
    # **hello _world_** — strong wrapping emphasis
    nodes = parse_inline("**hello _world_**")
    assert_equal "strong", nodes.first.type
    inner_types = nodes.first.children.map(&:type)
    assert_includes inner_types, "emphasis"
  end
end
