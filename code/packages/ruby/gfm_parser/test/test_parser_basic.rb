# frozen_string_literal: true

require "test_helper"

# Basic smoke tests for the GFM parser.
# These tests verify the parser loads correctly and handles simple inputs.
# The comprehensive spec tests are in test_commonmark_spec.rb

module CodingAdventures
  module CommonmarkParser
    class TestParserBasic < Minitest::Test
      def test_empty_document
        doc = CommonmarkParser.parse("")
        assert_equal "document", doc.type
        assert_equal [], doc.children
      end

      def test_simple_paragraph
        doc = CommonmarkParser.parse("Hello world\n")
        assert_equal "document", doc.type
        assert_equal 1, doc.children.length
        assert_equal "paragraph", doc.children[0].type
      end

      def test_atx_heading
        doc = CommonmarkParser.parse("# Hello\n")
        assert_equal 1, doc.children.length
        assert_equal "heading", doc.children[0].type
        assert_equal 1, doc.children[0].level
      end

      def test_atx_heading_levels
        (1..6).each do |level|
          doc = CommonmarkParser.parse("#{"#" * level} Heading\n")
          assert_equal "heading", doc.children[0].type
          assert_equal level, doc.children[0].level
        end
      end

      def test_fenced_code_block
        doc = CommonmarkParser.parse("```ruby\nx = 1\n```\n")
        assert_equal 1, doc.children.length
        assert_equal "code_block", doc.children[0].type
        assert_equal "ruby", doc.children[0].language
        assert_equal "x = 1\n", doc.children[0].value
      end

      def test_fenced_code_block_no_language
        doc = CommonmarkParser.parse("```\ncode\n```\n")
        assert_equal "code_block", doc.children[0].type
        assert_nil doc.children[0].language
      end

      def test_thematic_break
        doc = CommonmarkParser.parse("---\n")
        assert_equal "thematic_break", doc.children[0].type
      end

      def test_blockquote
        doc = CommonmarkParser.parse("> Hello\n")
        assert_equal "blockquote", doc.children[0].type
      end

      def test_unordered_list
        doc = CommonmarkParser.parse("- item 1\n- item 2\n")
        assert_equal "list", doc.children[0].type
        refute doc.children[0].ordered
        assert_equal 2, doc.children[0].children.length
      end

      def test_ordered_list
        doc = CommonmarkParser.parse("1. item 1\n2. item 2\n")
        assert_equal "list", doc.children[0].type
        assert doc.children[0].ordered
        assert_equal 1, doc.children[0].start
      end

      def test_setext_heading
        doc = CommonmarkParser.parse("Hello\n=====\n")
        assert_equal "heading", doc.children[0].type
        assert_equal 1, doc.children[0].level
      end

      def test_setext_heading_level2
        doc = CommonmarkParser.parse("Hello\n-----\n")
        assert_equal "heading", doc.children[0].type
        assert_equal 2, doc.children[0].level
      end

      def test_html_block
        doc = CommonmarkParser.parse("<div>\nhello\n</div>\n")
        assert_equal "raw_block", doc.children[0].type
        assert_equal "html", doc.children[0].format
      end

      def test_inline_emphasis
        doc = CommonmarkParser.parse("Hello *world*\n")
        para = doc.children[0]
        assert_equal "paragraph", para.type
        # children includes text + emphasis
        types = para.children.map(&:type)
        assert_includes types, "emphasis"
      end

      def test_inline_strong
        doc = CommonmarkParser.parse("Hello **world**\n")
        para = doc.children[0]
        types = para.children.map(&:type)
        assert_includes types, "strong"
      end

      def test_inline_strikethrough
        doc = CommonmarkParser.parse("Hello ~~world~~\n")
        para = doc.children[0]
        types = para.children.map(&:type)
        assert_includes types, "strikethrough"
      end

      def test_inline_code_span
        doc = CommonmarkParser.parse("Hello `code`\n")
        para = doc.children[0]
        types = para.children.map(&:type)
        assert_includes types, "code_span"
      end

      def test_inline_link
        doc = CommonmarkParser.parse("[text](https://example.com)\n")
        para = doc.children[0]
        link = para.children.find { |c| c.type == "link" }
        refute_nil link
        assert_equal "https://example.com", link.destination
      end

      def test_inline_image
        doc = CommonmarkParser.parse("![alt](img.png)\n")
        para = doc.children[0]
        img = para.children.find { |c| c.type == "image" }
        refute_nil img
        assert_equal "img.png", img.destination
        assert_equal "alt", img.alt
      end

      def test_autolink
        doc = CommonmarkParser.parse("<https://example.com>\n")
        para = doc.children[0]
        al = para.children.find { |c| c.type == "autolink" }
        refute_nil al
        assert_equal "https://example.com", al.destination
        refute al.is_email
      end

      def test_email_autolink
        doc = CommonmarkParser.parse("<user@example.com>\n")
        para = doc.children[0]
        al = para.children.find { |c| c.type == "autolink" }
        refute_nil al
        assert al.is_email
      end

      def test_hard_break
        doc = CommonmarkParser.parse("Hello  \nworld\n")
        para = doc.children[0]
        types = para.children.map(&:type)
        assert_includes types, "hard_break"
      end

      def test_soft_break
        doc = CommonmarkParser.parse("Hello\nworld\n")
        para = doc.children[0]
        types = para.children.map(&:type)
        assert_includes types, "soft_break"
      end

      def test_link_reference_definition
        md = "[foo]: /url\n\n[foo]\n"
        doc = CommonmarkParser.parse(md)
        para = doc.children[0]
        link = para.children.find { |c| c.type == "link" }
        refute_nil link
        assert_equal "/url", link.destination
      end

      def test_entity_decoding
        doc = CommonmarkParser.parse("Tom &amp; Jerry\n")
        para = doc.children[0]
        text_nodes = para.children.select { |c| c.type == "text" }
        combined = text_nodes.map(&:value).join
        assert_includes combined, "Tom & Jerry"
      end

      def test_multi_block_document
        md = "# Heading\n\nA paragraph.\n\n- item 1\n- item 2\n"
        doc = CommonmarkParser.parse(md)
        assert_equal 3, doc.children.length
        assert_equal "heading", doc.children[0].type
        assert_equal "paragraph", doc.children[1].type
        assert_equal "list", doc.children[2].type
      end

      def test_task_list
        doc = CommonmarkParser.parse("- [x] done\n")
        list = doc.children[0]
        assert_equal "list", list.type
        assert_equal "task_item", list.children[0].type
        assert list.children[0].checked
      end

      def test_table
        doc = CommonmarkParser.parse("| A |\n| --- |\n| B |\n")
        table = doc.children[0]
        assert_equal "table", table.type
        assert_equal true, table.children[0].is_header
        assert_equal "A", table.children[0].children[0].children[0].value
        assert_equal "B", table.children[1].children[0].children[0].value
      end
    end
  end
end
