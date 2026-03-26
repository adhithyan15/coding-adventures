# frozen_string_literal: true

require "test_helper"

# Tests for Document AST node types.
#
# The Document AST is a types-only package — there is no runtime logic.
# These tests verify:
#   1. Each node can be constructed with the right attributes.
#   2. The `type` method returns the correct string tag.
#   3. Ruby Data class semantics: frozen, value equality.
#   4. Nodes compose correctly into trees.

module CodingAdventures
  module DocumentAst
    class TestDocumentNode < Minitest::Test
      def test_type_tag
        doc = DocumentNode.new(children: [])
        assert_equal "document", doc.type
      end

      def test_empty_children
        doc = DocumentNode.new(children: [])
        assert_equal [], doc.children
      end

      def test_frozen
        doc = DocumentNode.new(children: [])
        assert doc.frozen?
      end

      def test_value_equality
        doc1 = DocumentNode.new(children: [])
        doc2 = DocumentNode.new(children: [])
        assert_equal doc1, doc2
      end

      def test_with_children
        heading = HeadingNode.new(level: 1, children: [TextNode.new(value: "Hello")])
        doc = DocumentNode.new(children: [heading])
        assert_equal 1, doc.children.length
        assert_equal "heading", doc.children[0].type
      end
    end

    class TestHeadingNode < Minitest::Test
      def test_type_tag
        h = HeadingNode.new(level: 1, children: [])
        assert_equal "heading", h.type
      end

      def test_levels
        (1..6).each do |level|
          h = HeadingNode.new(level: level, children: [])
          assert_equal level, h.level
        end
      end

      def test_children
        text = TextNode.new(value: "Hello")
        h = HeadingNode.new(level: 2, children: [text])
        assert_equal 1, h.children.length
        assert_equal "text", h.children[0].type
      end
    end

    class TestParagraphNode < Minitest::Test
      def test_type_tag
        p = ParagraphNode.new(children: [])
        assert_equal "paragraph", p.type
      end

      def test_children
        text = TextNode.new(value: "Hello world")
        p = ParagraphNode.new(children: [text])
        assert_equal 1, p.children.length
      end
    end

    class TestCodeBlockNode < Minitest::Test
      def test_type_tag
        cb = CodeBlockNode.new(language: nil, value: "code\n")
        assert_equal "code_block", cb.type
      end

      def test_language_nil
        cb = CodeBlockNode.new(language: nil, value: "code\n")
        assert_nil cb.language
      end

      def test_language_string
        cb = CodeBlockNode.new(language: "ruby", value: "x = 1\n")
        assert_equal "ruby", cb.language
      end

      def test_value
        cb = CodeBlockNode.new(language: nil, value: "hello\nworld\n")
        assert_equal "hello\nworld\n", cb.value
      end
    end

    class TestBlockquoteNode < Minitest::Test
      def test_type_tag
        bq = BlockquoteNode.new(children: [])
        assert_equal "blockquote", bq.type
      end

      def test_nested_blockquote
        inner = BlockquoteNode.new(children: [
          ParagraphNode.new(children: [TextNode.new(value: "inner")])
        ])
        outer = BlockquoteNode.new(children: [inner])
        assert_equal 1, outer.children.length
        assert_equal "blockquote", outer.children[0].type
      end
    end

    class TestListNode < Minitest::Test
      def test_unordered_type_tag
        list = ListNode.new(ordered: false, start: nil, tight: true, children: [])
        assert_equal "list", list.type
      end

      def test_ordered_type_tag
        list = ListNode.new(ordered: true, start: 1, tight: false, children: [])
        assert_equal "list", list.type
      end

      def test_tight_flag
        tight = ListNode.new(ordered: false, start: nil, tight: true, children: [])
        loose = ListNode.new(ordered: false, start: nil, tight: false, children: [])
        assert tight.tight
        refute loose.tight
      end

      def test_start_number
        list = ListNode.new(ordered: true, start: 42, tight: true, children: [])
        assert_equal 42, list.start
      end

      def test_unordered_start_nil
        list = ListNode.new(ordered: false, start: nil, tight: true, children: [])
        assert_nil list.start
      end
    end

    class TestListItemNode < Minitest::Test
      def test_type_tag
        item = ListItemNode.new(children: [])
        assert_equal "list_item", item.type
      end

      def test_children
        para = ParagraphNode.new(children: [TextNode.new(value: "item text")])
        item = ListItemNode.new(children: [para])
        assert_equal 1, item.children.length
        assert_equal "paragraph", item.children[0].type
      end
    end

    class TestTaskItemNode < Minitest::Test
      def test_type_tag
        item = TaskItemNode.new(checked: true, children: [])
        assert_equal "task_item", item.type
      end
    end

    class TestThematicBreakNode < Minitest::Test
      def test_type_tag
        tb = ThematicBreakNode.new
        assert_equal "thematic_break", tb.type
      end

      def test_value_equality
        assert_equal ThematicBreakNode.new, ThematicBreakNode.new
      end
    end

    class TestRawBlockNode < Minitest::Test
      def test_type_tag
        rb = RawBlockNode.new(format: "html", value: "<div>raw</div>\n")
        assert_equal "raw_block", rb.type
      end

      def test_format_and_value
        rb = RawBlockNode.new(format: "html", value: "<p>hello</p>\n")
        assert_equal "html", rb.format
        assert_equal "<p>hello</p>\n", rb.value
      end

      def test_non_html_format
        rb = RawBlockNode.new(format: "latex", value: "\\textbf{x}\n")
        assert_equal "latex", rb.format
      end
    end

    class TestTableNodes < Minitest::Test
      def test_table_type_tag
        table = TableNode.new(align: ["left"], children: [])
        assert_equal "table", table.type
      end

      def test_table_row_type_tag
        row = TableRowNode.new(is_header: true, children: [])
        assert_equal "table_row", row.type
      end

      def test_table_cell_type_tag
        cell = TableCellNode.new(children: [TextNode.new(value: "A")])
        assert_equal "table_cell", cell.type
      end
    end

    class TestTextNode < Minitest::Test
      def test_type_tag
        t = TextNode.new(value: "hello")
        assert_equal "text", t.type
      end

      def test_value
        t = TextNode.new(value: "Hello & world")
        assert_equal "Hello & world", t.value
      end

      def test_value_equality
        assert_equal TextNode.new(value: "x"), TextNode.new(value: "x")
        refute_equal TextNode.new(value: "x"), TextNode.new(value: "y")
      end
    end

    class TestEmphasisNode < Minitest::Test
      def test_type_tag
        em = EmphasisNode.new(children: [])
        assert_equal "emphasis", em.type
      end

      def test_with_text_child
        em = EmphasisNode.new(children: [TextNode.new(value: "emphasized")])
        assert_equal 1, em.children.length
        assert_equal "text", em.children[0].type
      end
    end

    class TestStrongNode < Minitest::Test
      def test_type_tag
        st = StrongNode.new(children: [])
        assert_equal "strong", st.type
      end
    end

    class TestStrikethroughNode < Minitest::Test
      def test_type_tag
        node = StrikethroughNode.new(children: [TextNode.new(value: "gone")])
        assert_equal "strikethrough", node.type
      end
    end

    class TestCodeSpanNode < Minitest::Test
      def test_type_tag
        cs = CodeSpanNode.new(value: "const x = 1")
        assert_equal "code_span", cs.type
      end

      def test_value
        cs = CodeSpanNode.new(value: "x = 42")
        assert_equal "x = 42", cs.value
      end
    end

    class TestLinkNode < Minitest::Test
      def test_type_tag
        ln = LinkNode.new(destination: "https://example.com", title: nil, children: [])
        assert_equal "link", ln.type
      end

      def test_destination
        ln = LinkNode.new(destination: "https://example.com", title: nil, children: [])
        assert_equal "https://example.com", ln.destination
      end

      def test_title_nil
        ln = LinkNode.new(destination: "https://example.com", title: nil, children: [])
        assert_nil ln.title
      end

      def test_title_present
        ln = LinkNode.new(destination: "https://example.com", title: "Example", children: [])
        assert_equal "Example", ln.title
      end
    end

    class TestImageNode < Minitest::Test
      def test_type_tag
        img = ImageNode.new(destination: "cat.png", title: nil, alt: "a cat")
        assert_equal "image", img.type
      end

      def test_alt_text
        img = ImageNode.new(destination: "cat.png", title: nil, alt: "a cat")
        assert_equal "a cat", img.alt
      end
    end

    class TestAutolinkNode < Minitest::Test
      def test_type_tag
        al = AutolinkNode.new(destination: "https://example.com", is_email: false)
        assert_equal "autolink", al.type
      end

      def test_url_autolink
        al = AutolinkNode.new(destination: "https://example.com", is_email: false)
        refute al.is_email
      end

      def test_email_autolink
        al = AutolinkNode.new(destination: "user@example.com", is_email: true)
        assert al.is_email
      end
    end

    class TestRawInlineNode < Minitest::Test
      def test_type_tag
        ri = RawInlineNode.new(format: "html", value: "<em>hi</em>")
        assert_equal "raw_inline", ri.type
      end

      def test_format_and_value
        ri = RawInlineNode.new(format: "html", value: "<em>hi</em>")
        assert_equal "html", ri.format
        assert_equal "<em>hi</em>", ri.value
      end
    end

    class TestHardBreakNode < Minitest::Test
      def test_type_tag
        hb = HardBreakNode.new
        assert_equal "hard_break", hb.type
      end

      def test_value_equality
        assert_equal HardBreakNode.new, HardBreakNode.new
      end
    end

    class TestSoftBreakNode < Minitest::Test
      def test_type_tag
        sb = SoftBreakNode.new
        assert_equal "soft_break", sb.type
      end
    end

    class TestComposition < Minitest::Test
      # Builds a realistic document tree and exercises composition.
      #
      #   # Hello
      #
      #   A paragraph with *emphasis*.
      #
      #   - item 1
      #   - item 2
      def test_realistic_document
        heading = HeadingNode.new(
          level: 1,
          children: [TextNode.new(value: "Hello")]
        )

        para = ParagraphNode.new(
          children: [
            TextNode.new(value: "A paragraph with "),
            EmphasisNode.new(children: [TextNode.new(value: "emphasis")]),
            TextNode.new(value: ".")
          ]
        )

        list = ListNode.new(
          ordered: false,
          start: nil,
          tight: true,
          children: [
            ListItemNode.new(children: [ParagraphNode.new(children: [TextNode.new(value: "item 1")])]),
            ListItemNode.new(children: [ParagraphNode.new(children: [TextNode.new(value: "item 2")])])
          ]
        )

        doc = DocumentNode.new(children: [heading, para, list])

        assert_equal 3, doc.children.length
        assert_equal "heading", doc.children[0].type
        assert_equal "paragraph", doc.children[1].type
        assert_equal "list", doc.children[2].type
        assert_equal 2, doc.children[2].children.length
      end
    end
  end
end
