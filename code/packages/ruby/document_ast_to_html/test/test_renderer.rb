# frozen_string_literal: true

require "test_helper"

# Tests for the Document AST → HTML renderer.
# Covers all 18 node types, tight/loose lists, URL sanitization, and the
# sanitize option.

module CodingAdventures
  module DocumentAstToHtml
    class TestRenderer < Minitest::Test
      include DocumentAst

      # ─── Helpers ────────────────────────────────────────────────────────────

      # Shorthand: render a single block node wrapped in a DocumentNode.
      def render_block(node)
        doc = DocumentNode.new(children: [node])
        DocumentAstToHtml.to_html(doc)
      end

      def txt(value)
        TextNode.new(value: value)
      end

      # ─── Document ───────────────────────────────────────────────────────────

      def test_empty_document
        doc = DocumentNode.new(children: [])
        assert_equal "", DocumentAstToHtml.to_html(doc)
      end

      def test_document_renders_children
        doc = DocumentNode.new(children: [
          HeadingNode.new(level: 1, children: [txt("Hello")]),
          ParagraphNode.new(children: [txt("World")])
        ])
        assert_equal "<h1>Hello</h1>\n<p>World</p>\n", DocumentAstToHtml.to_html(doc)
      end

      # ─── Headings ───────────────────────────────────────────────────────────

      def test_heading_level_1
        assert_equal "<h1>Hello</h1>\n",
          render_block(HeadingNode.new(level: 1, children: [txt("Hello")]))
      end

      def test_heading_level_6
        assert_equal "<h6>Deep</h6>\n",
          render_block(HeadingNode.new(level: 6, children: [txt("Deep")]))
      end

      def test_heading_with_inline_emphasis
        node = HeadingNode.new(level: 2, children: [
          txt("Hello "),
          EmphasisNode.new(children: [txt("world")])
        ])
        assert_equal "<h2>Hello <em>world</em></h2>\n", render_block(node)
      end

      # ─── Paragraph ──────────────────────────────────────────────────────────

      def test_paragraph
        assert_equal "<p>Hello world</p>\n",
          render_block(ParagraphNode.new(children: [txt("Hello world")]))
      end

      def test_paragraph_with_inline
        node = ParagraphNode.new(children: [
          txt("Hello "),
          StrongNode.new(children: [txt("world")])
        ])
        assert_equal "<p>Hello <strong>world</strong></p>\n", render_block(node)
      end

      # ─── Code Block ─────────────────────────────────────────────────────────

      def test_code_block_no_language
        assert_equal "<pre><code>x = 1\n</code></pre>\n",
          render_block(CodeBlockNode.new(language: nil, value: "x = 1\n"))
      end

      def test_code_block_with_language
        assert_equal "<pre><code class=\"language-ruby\">x = 1\n</code></pre>\n",
          render_block(CodeBlockNode.new(language: "ruby", value: "x = 1\n"))
      end

      def test_code_block_escapes_html
        assert_equal "<pre><code>&lt;div&gt;\n</code></pre>\n",
          render_block(CodeBlockNode.new(language: nil, value: "<div>\n"))
      end

      # ─── Blockquote ─────────────────────────────────────────────────────────

      def test_blockquote
        inner = ParagraphNode.new(children: [txt("Hello")])
        node = BlockquoteNode.new(children: [inner])
        assert_equal "<blockquote>\n<p>Hello</p>\n</blockquote>\n", render_block(node)
      end

      # ─── Thematic Break ─────────────────────────────────────────────────────

      def test_thematic_break
        assert_equal "<hr />\n", render_block(ThematicBreakNode.new)
      end

      # ─── Lists ──────────────────────────────────────────────────────────────

      def test_unordered_tight_list
        list = ListNode.new(
          ordered: false,
          start: nil,
          tight: true,
          children: [
            ListItemNode.new(children: [ParagraphNode.new(children: [txt("a")])]),
            ListItemNode.new(children: [ParagraphNode.new(children: [txt("b")])])
          ]
        )
        assert_equal "<ul>\n<li>a</li>\n<li>b</li>\n</ul>\n", render_block(list)
      end

      def test_ordered_loose_list
        list = ListNode.new(
          ordered: true,
          start: 1,
          tight: false,
          children: [
            ListItemNode.new(children: [ParagraphNode.new(children: [txt("x")])]),
            ListItemNode.new(children: [ParagraphNode.new(children: [txt("y")])])
          ]
        )
        assert_equal "<ol>\n<li>\n<p>x</p>\n</li>\n<li>\n<p>y</p>\n</li>\n</ol>\n",
          render_block(list)
      end

      def test_ordered_list_with_start_not_1
        list = ListNode.new(
          ordered: true,
          start: 3,
          tight: false,
          children: [
            ListItemNode.new(children: [ParagraphNode.new(children: [txt("item")])])
          ]
        )
        assert_includes render_block(list), "start=\"3\""
      end

      def test_ordered_list_start_1_no_attribute
        list = ListNode.new(
          ordered: true,
          start: 1,
          tight: false,
          children: [
            ListItemNode.new(children: [ParagraphNode.new(children: [txt("item")])])
          ]
        )
        refute_includes render_block(list), "start="
      end

      def test_empty_list_item
        list = ListNode.new(
          ordered: false,
          start: nil,
          tight: true,
          children: [ListItemNode.new(children: [])]
        )
        assert_equal "<ul>\n<li></li>\n</ul>\n", render_block(list)
      end

      # ─── Raw Block ──────────────────────────────────────────────────────────

      def test_raw_block_html_format
        node = RawBlockNode.new(format: "html", value: "<div>raw</div>\n")
        assert_equal "<div>raw</div>\n", render_block(node)
      end

      def test_raw_block_html_sanitized
        doc = DocumentNode.new(children: [
          RawBlockNode.new(format: "html", value: "<div>raw</div>\n")
        ])
        assert_equal "", DocumentAstToHtml.to_html(doc, sanitize: true)
      end

      def test_raw_block_other_format_skipped
        node = RawBlockNode.new(format: "latex", value: "\\textbf{x}\n")
        assert_equal "", render_block(node)
      end

      # ─── Text ───────────────────────────────────────────────────────────────

      def test_text_escaping
        node = ParagraphNode.new(children: [txt("Tom & <Jerry> say \"hi\"")])
        assert_equal "<p>Tom &amp; &lt;Jerry&gt; say &quot;hi&quot;</p>\n",
          render_block(node)
      end

      # ─── Emphasis / Strong ──────────────────────────────────────────────────

      def test_emphasis
        node = ParagraphNode.new(children: [
          EmphasisNode.new(children: [txt("em")])
        ])
        assert_equal "<p><em>em</em></p>\n", render_block(node)
      end

      def test_strong
        node = ParagraphNode.new(children: [
          StrongNode.new(children: [txt("bold")])
        ])
        assert_equal "<p><strong>bold</strong></p>\n", render_block(node)
      end

      # ─── Code Span ──────────────────────────────────────────────────────────

      def test_code_span
        node = ParagraphNode.new(children: [
          CodeSpanNode.new(value: "x = <div>")
        ])
        assert_equal "<p><code>x = &lt;div&gt;</code></p>\n", render_block(node)
      end

      # ─── Link ───────────────────────────────────────────────────────────────

      def test_link_no_title
        node = ParagraphNode.new(children: [
          LinkNode.new(destination: "https://example.com", title: nil, children: [txt("text")])
        ])
        assert_equal "<p><a href=\"https://example.com\">text</a></p>\n", render_block(node)
      end

      def test_link_with_title
        node = ParagraphNode.new(children: [
          LinkNode.new(destination: "https://example.com", title: "My title", children: [txt("text")])
        ])
        assert_equal "<p><a href=\"https://example.com\" title=\"My title\">text</a></p>\n",
          render_block(node)
      end

      def test_link_dangerous_scheme_blocked
        node = ParagraphNode.new(children: [
          LinkNode.new(destination: "javascript:alert(1)", title: nil, children: [txt("xss")])
        ])
        result = render_block(node)
        assert_includes result, "href=\"\""
      end

      # ─── Image ──────────────────────────────────────────────────────────────

      def test_image_no_title
        node = ParagraphNode.new(children: [
          ImageNode.new(destination: "cat.png", alt: "a cat", title: nil)
        ])
        assert_equal "<p><img src=\"cat.png\" alt=\"a cat\" /></p>\n", render_block(node)
      end

      def test_image_with_title
        node = ParagraphNode.new(children: [
          ImageNode.new(destination: "cat.png", alt: "a cat", title: "Kitty")
        ])
        assert_equal "<p><img src=\"cat.png\" alt=\"a cat\" title=\"Kitty\" /></p>\n",
          render_block(node)
      end

      # ─── Autolink ───────────────────────────────────────────────────────────

      def test_autolink_url
        node = ParagraphNode.new(children: [
          AutolinkNode.new(destination: "https://example.com", is_email: false)
        ])
        assert_equal "<p><a href=\"https://example.com\">https://example.com</a></p>\n",
          render_block(node)
      end

      def test_autolink_email
        node = ParagraphNode.new(children: [
          AutolinkNode.new(destination: "user@example.com", is_email: true)
        ])
        assert_equal "<p><a href=\"mailto:user@example.com\">user@example.com</a></p>\n",
          render_block(node)
      end

      # ─── Raw Inline ─────────────────────────────────────────────────────────

      def test_raw_inline_html
        node = ParagraphNode.new(children: [
          RawInlineNode.new(format: "html", value: "<em>hi</em>")
        ])
        assert_equal "<p><em>hi</em></p>\n", render_block(node)
      end

      def test_raw_inline_html_sanitized
        doc = DocumentNode.new(children: [
          ParagraphNode.new(children: [
            RawInlineNode.new(format: "html", value: "<em>hi</em>")
          ])
        ])
        assert_equal "<p></p>\n", DocumentAstToHtml.to_html(doc, sanitize: true)
      end

      def test_raw_inline_other_format_skipped
        node = ParagraphNode.new(children: [
          RawInlineNode.new(format: "latex", value: "\\emph{x}")
        ])
        assert_equal "<p></p>\n", render_block(node)
      end

      # ─── Hard / Soft Break ──────────────────────────────────────────────────

      def test_hard_break
        node = ParagraphNode.new(children: [
          txt("line1"),
          HardBreakNode.new,
          txt("line2")
        ])
        assert_equal "<p>line1<br />\nline2</p>\n", render_block(node)
      end

      def test_soft_break
        node = ParagraphNode.new(children: [
          txt("line1"),
          SoftBreakNode.new,
          txt("line2")
        ])
        assert_equal "<p>line1\nline2</p>\n", render_block(node)
      end

      # ─── URL Sanitization ───────────────────────────────────────────────────

      def test_sanitize_url_safe_url_passes_through
        assert_equal "https://example.com", DocumentAstToHtml.sanitize_url("https://example.com")
      end

      def test_sanitize_url_javascript_blocked
        assert_equal "", DocumentAstToHtml.sanitize_url("javascript:alert(1)")
      end

      def test_sanitize_url_vbscript_blocked
        assert_equal "", DocumentAstToHtml.sanitize_url("vbscript:msgbox(1)")
      end

      def test_sanitize_url_data_blocked
        assert_equal "", DocumentAstToHtml.sanitize_url("data:text/html,<h1>hi</h1>")
      end

      def test_sanitize_url_blob_blocked
        assert_equal "", DocumentAstToHtml.sanitize_url("blob:https://example.com/1234")
      end

      def test_sanitize_url_strips_control_chars
        refute_includes DocumentAstToHtml.sanitize_url("java\rscript:alert(1)"), "script"
      end

      # ─── escape_html ────────────────────────────────────────────────────────

      def test_escape_html_ampersand
        assert_equal "&amp;", DocumentAstToHtml.escape_html("&")
      end

      def test_escape_html_lt
        assert_equal "&lt;", DocumentAstToHtml.escape_html("<")
      end

      def test_escape_html_gt
        assert_equal "&gt;", DocumentAstToHtml.escape_html(">")
      end

      def test_escape_html_quote
        assert_equal "&quot;", DocumentAstToHtml.escape_html('"')
      end

      def test_escape_html_no_change_for_safe_text
        assert_equal "hello world", DocumentAstToHtml.escape_html("hello world")
      end
    end
  end
end
