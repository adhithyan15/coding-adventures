# frozen_string_literal: true

require "test_helper"

# Tests for the coding_adventures_gfm convenience pipeline.
# Verifies the thin wrapper correctly delegates to the parser and renderer.

module CodingAdventures
  class TestCommonmark < Minitest::Test
    # ─── parse ──────────────────────────────────────────────────────────────────

    def test_parse_returns_document_node
      doc = Commonmark.parse("# Hello\n")
      assert_equal "document", doc.type
    end

    def test_parse_empty_string
      doc = Commonmark.parse("")
      assert_equal "document", doc.type
      assert_equal [], doc.children
    end

    def test_parse_produces_ast
      doc = Commonmark.parse("# Heading\n\nA paragraph.\n")
      assert_equal 2, doc.children.length
      assert_equal "heading", doc.children[0].type
      assert_equal "paragraph", doc.children[1].type
    end

    # ─── to_html ────────────────────────────────────────────────────────────────

    def test_to_html_renders_document
      doc = Commonmark.parse("# Hello\n\nWorld\n")
      html = Commonmark.to_html(doc)
      assert_equal "<h1>Hello</h1>\n<p>World</p>\n", html
    end

    def test_to_html_sanitize_strips_raw_html
      doc = Commonmark.parse("<div>raw</div>\n")
      sanitized = Commonmark.to_html(doc, sanitize: true)
      assert_equal "", sanitized
      unsanitized = Commonmark.to_html(doc, sanitize: false)
      assert_equal "<div>raw</div>\n", unsanitized
    end

    # ─── parse_to_html ──────────────────────────────────────────────────────────

    def test_parse_to_html_simple_heading
      html = Commonmark.parse_to_html("# Hello\n")
      assert_equal "<h1>Hello</h1>\n", html
    end

    def test_parse_to_html_paragraph
      html = Commonmark.parse_to_html("Hello world\n")
      assert_equal "<p>Hello world</p>\n", html
    end

    def test_parse_to_html_emphasis
      html = Commonmark.parse_to_html("Hello *world*\n")
      assert_equal "<p>Hello <em>world</em></p>\n", html
    end

    def test_parse_to_html_strong
      html = Commonmark.parse_to_html("Hello **world**\n")
      assert_equal "<p>Hello <strong>world</strong></p>\n", html
    end

    def test_parse_to_html_strikethrough
      html = Commonmark.parse_to_html("Hello ~~world~~\n")
      assert_equal "<p>Hello <del>world</del></p>\n", html
    end

    def test_parse_to_html_link
      html = Commonmark.parse_to_html("[text](https://example.com)\n")
      assert_equal "<p><a href=\"https://example.com\">text</a></p>\n", html
    end

    def test_parse_to_html_list
      html = Commonmark.parse_to_html("- a\n- b\n")
      assert_equal "<ul>\n<li>a</li>\n<li>b</li>\n</ul>\n", html
    end

    def test_parse_to_html_task_list
      html = Commonmark.parse_to_html("- [x] done\n")
      assert_equal "<ul>\n<li><input type=\"checkbox\" disabled=\"\" checked=\"\" /> done</li>\n</ul>\n", html
    end

    def test_parse_to_html_table
      html = Commonmark.parse_to_html("| A |\n| --- |\n| B |\n")
      assert_equal "<table>\n<thead>\n<tr>\n<th>A</th>\n</tr>\n</thead>\n<tbody>\n<tr>\n<td>B</td>\n</tr>\n</tbody>\n</table>\n", html
    end

    def test_parse_to_html_code_block
      html = Commonmark.parse_to_html("```ruby\nx = 1\n```\n")
      assert_equal "<pre><code class=\"language-ruby\">x = 1\n</code></pre>\n", html
    end

    def test_parse_to_html_sanitize
      html = Commonmark.parse_to_html("<script>alert(1)</script>\n", sanitize: true)
      assert_equal "", html
    end

    def test_parse_to_html_empty
      assert_equal "", Commonmark.parse_to_html("")
    end

    # ─── VERSION ────────────────────────────────────────────────────────────────

    def test_version_is_defined
      assert_match(/\A\d+\.\d+\.\d+\z/, Commonmark::VERSION)
    end
  end
end
