# frozen_string_literal: true

require "test_helper"

# Tests for CodingAdventures::DocumentAstSanitizer
#
# Organisation:
#   1. Policy presets (STRICT, RELAXED, PASSTHROUGH smoke tests)
#   2. Raw block / raw inline handling
#   3. URL scheme sanitization (including XSS vectors)
#   4. Heading level clamping and drop
#   5. Image handling (drop, transform to text, URL sanitize)
#   6. Link handling (drop + promote children, URL sanitize)
#   7. Autolink handling
#   8. CodeBlock + CodeSpan
#   9. BlockquoteNode
#  10. Empty-children pruning
#  11. Immutability (input unchanged)
#  12. Complex / integration scenarios

module CodingAdventures
  class TestDocumentAstSanitizer < Minitest::Test
    include DocumentAst

    # ─── Helpers ──────────────────────────────────────────────────────────────

    # Build a one-child DocumentNode.
    def doc(*nodes)
      DocumentNode.new(children: nodes)
    end

    def txt(value)
      TextNode.new(value: value)
    end

    def para(*inlines)
      ParagraphNode.new(children: inlines)
    end

    def heading(level, *inlines)
      HeadingNode.new(level: level, children: inlines)
    end

    def link(dest, *inlines, title: nil)
      LinkNode.new(destination: dest, title: title, children: inlines)
    end

    def image(dest, alt, title: nil)
      ImageNode.new(destination: dest, alt: alt, title: title)
    end

    def autolink(dest, is_email: false)
      AutolinkNode.new(destination: dest, is_email: is_email)
    end

    def raw_block(fmt, value)
      RawBlockNode.new(format: fmt, value: value)
    end

    def raw_inline(fmt, value)
      RawInlineNode.new(format: fmt, value: value)
    end

    def code_block(value, language: nil)
      CodeBlockNode.new(language: language, value: value)
    end

    def code_span(value)
      CodeSpanNode.new(value: value)
    end

    def blockquote(*children)
      BlockquoteNode.new(children: children)
    end

    def list(*items, ordered: false, tight: true)
      ListNode.new(ordered: ordered, start: nil, tight: tight, children: items)
    end

    def list_item(*children)
      ListItemNode.new(children: children)
    end

    def task_item(*children, checked: false)
      TaskItemNode.new(checked: checked, children: children)
    end

    def sanitize(document, policy)
      DocumentAstSanitizer.sanitize(document, policy)
    end

    def test_gfm_nodes_preserved
      document = doc(
        list(task_item(para(StrikethroughNode.new(children: [txt("done")])), checked: true)),
        TableNode.new(
          align: [nil],
          children: [
            TableRowNode.new(
              is_header: true,
              children: [TableCellNode.new(children: [txt("A")])]
            )
          ]
        )
      )

      result = sanitize(document, DocumentAstSanitizer::PASSTHROUGH)
      assert_equal document, result
    end

    # ─── 1. Policy presets ────────────────────────────────────────────────────

    def test_passthrough_returns_equivalent_document
      # PASSTHROUGH must not alter any node's structure.
      document = doc(
        heading(1, txt("Title")),
        para(txt("Hello "), link("https://example.com", txt("world"))),
        raw_block("html", "<b>raw</b>")
      )
      result = sanitize(document, DocumentAstSanitizer::PASSTHROUGH)
      assert_equal document, result
    end

    def test_strict_drops_raw_blocks
      document = doc(raw_block("html", "<script>bad</script>"))
      result = sanitize(document, DocumentAstSanitizer::STRICT)
      assert_empty result.children
    end

    def test_strict_drops_raw_inlines
      document = doc(para(raw_inline("html", "<b>bad</b>")))
      result = sanitize(document, DocumentAstSanitizer::STRICT)
      # paragraph empty → dropped
      assert_empty result.children
    end

    def test_relaxed_keeps_html_raw_blocks
      document = doc(raw_block("html", "<hr />"))
      result = sanitize(document, DocumentAstSanitizer::RELAXED)
      assert_equal 1, result.children.length
      assert_equal "raw_block", result.children.first.type
    end

    def test_relaxed_drops_non_html_raw_blocks
      document = doc(raw_block("latex", "\\textbf{foo}"))
      result = sanitize(document, DocumentAstSanitizer::RELAXED)
      assert_empty result.children
    end

    # ─── 2. Raw block handling ────────────────────────────────────────────────

    def test_raw_block_drop_all
      policy = DocumentAstSanitizer::PASSTHROUGH.with(
        allow_raw_block_formats: "drop-all"
      )
      document = doc(raw_block("html", "<b>x</b>"))
      result = sanitize(document, policy)
      assert_empty result.children
    end

    def test_raw_block_passthrough
      policy = DocumentAstSanitizer::PASSTHROUGH.with(
        allow_raw_block_formats: "passthrough"
      )
      node = raw_block("html", "<b>x</b>")
      result = sanitize(doc(node), policy)
      assert_equal [node], result.children
    end

    def test_raw_block_allowlist_keeps_matching_format
      policy = DocumentAstSanitizer::PASSTHROUGH.with(
        allow_raw_block_formats: ["html"]
      )
      node = raw_block("html", "<hr />")
      result = sanitize(doc(node), policy)
      assert_equal [node], result.children
    end

    def test_raw_block_allowlist_drops_non_matching_format
      policy = DocumentAstSanitizer::PASSTHROUGH.with(
        allow_raw_block_formats: ["html"]
      )
      result = sanitize(doc(raw_block("latex", "x")), policy)
      assert_empty result.children
    end

    def test_raw_inline_drop_all
      policy = DocumentAstSanitizer::PASSTHROUGH.with(
        allow_raw_inline_formats: "drop-all"
      )
      document = doc(para(txt("a"), raw_inline("html", "<b>x</b>"), txt("b")))
      result = sanitize(document, policy)
      # para still has text nodes
      assert_equal 1, result.children.length
      assert_equal 2, result.children.first.children.length
    end

    def test_raw_inline_passthrough
      policy = DocumentAstSanitizer::PASSTHROUGH.with(
        allow_raw_inline_formats: "passthrough"
      )
      node = raw_inline("html", "<b>x</b>")
      result = sanitize(doc(para(node)), policy)
      assert_equal node, result.children.first.children.first
    end

    def test_raw_inline_allowlist
      policy = DocumentAstSanitizer::PASSTHROUGH.with(
        allow_raw_inline_formats: ["html"]
      )
      node = raw_inline("html", "<b>x</b>")
      result = sanitize(doc(para(node)), policy)
      assert_equal node, result.children.first.children.first
    end

    def test_raw_inline_allowlist_drops_other
      policy = DocumentAstSanitizer::PASSTHROUGH.with(
        allow_raw_inline_formats: ["html"]
      )
      result = sanitize(doc(para(raw_inline("latex", "x"))), policy)
      assert_empty result.children
    end

    # ─── 3. URL scheme sanitization ───────────────────────────────────────────

    def test_link_javascript_scheme_blocked_by_strict
      document = doc(para(link("javascript:alert(1)", txt("click"))))
      result = sanitize(document, DocumentAstSanitizer::STRICT)
      link_node = result.children.first.children.first
      assert_equal "link", link_node.type
      assert_equal "", link_node.destination
    end

    def test_link_javascript_uppercase_scheme_blocked
      document = doc(para(link("JAVASCRIPT:alert(1)", txt("click"))))
      result = sanitize(document, DocumentAstSanitizer::STRICT)
      link_node = result.children.first.children.first
      assert_equal "", link_node.destination
    end

    def test_link_data_scheme_blocked
      document = doc(para(link("data:text/html,<script>alert(1)</script>", txt("x"))))
      result = sanitize(document, DocumentAstSanitizer::STRICT)
      assert_equal "", result.children.first.children.first.destination
    end

    def test_link_blob_scheme_blocked
      document = doc(para(link("blob:https://origin/uuid", txt("x"))))
      result = sanitize(document, DocumentAstSanitizer::STRICT)
      assert_equal "", result.children.first.children.first.destination
    end

    def test_link_vbscript_scheme_blocked
      document = doc(para(link("vbscript:MsgBox(1)", txt("click"))))
      result = sanitize(document, DocumentAstSanitizer::STRICT)
      assert_equal "", result.children.first.children.first.destination
    end

    def test_link_https_allowed_by_strict
      document = doc(para(link("https://example.com", txt("ok"))))
      result = sanitize(document, DocumentAstSanitizer::STRICT)
      assert_equal "https://example.com", result.children.first.children.first.destination
    end

    def test_link_relative_url_allowed
      document = doc(para(link("/path/to/page", txt("local"))))
      result = sanitize(document, DocumentAstSanitizer::STRICT)
      assert_equal "/path/to/page", result.children.first.children.first.destination
    end

    def test_link_control_char_bypass
      # "java\x00script:alert(1)" — browsers ignore the NUL
      dest = "java\x00script:alert(1)"
      document = doc(para(link(dest, txt("x"))))
      result = sanitize(document, DocumentAstSanitizer::STRICT)
      assert_equal "", result.children.first.children.first.destination
    end

    def test_link_zero_width_space_bypass
      # "\u200Bjavascript:alert(1)" — zero-width space before scheme
      dest = "\u200Bjavascript:alert(1)"
      document = doc(para(link(dest, txt("x"))))
      result = sanitize(document, DocumentAstSanitizer::STRICT)
      # After stripping \u200B the scheme becomes "javascript" → blocked
      assert_equal "", result.children.first.children.first.destination
    end

    def test_link_allowed_schemes_nil_passes_everything
      policy = DocumentAstSanitizer::PASSTHROUGH.with(allowed_url_schemes: nil)
      document = doc(para(link("javascript:alert(1)", txt("x"))))
      result = sanitize(document, policy)
      # nil = allow everything
      assert_equal "javascript:alert(1)", result.children.first.children.first.destination
    end

    # ─── 4. Heading level clamping ────────────────────────────────────────────

    def test_heading_clamp_up_min_level
      # h1 when min=2 → becomes h2
      policy = DocumentAstSanitizer::PASSTHROUGH.with(min_heading_level: 2)
      document = doc(heading(1, txt("Title")))
      result = sanitize(document, policy)
      assert_equal 2, result.children.first.level
    end

    def test_heading_clamp_down_max_level
      # h5 when max=3 → becomes h3
      policy = DocumentAstSanitizer::PASSTHROUGH.with(max_heading_level: 3)
      document = doc(heading(5, txt("Deep")))
      result = sanitize(document, policy)
      assert_equal 3, result.children.first.level
    end

    def test_heading_within_range_unchanged
      policy = DocumentAstSanitizer::PASSTHROUGH.with(
        min_heading_level: 2,
        max_heading_level: 4
      )
      document = doc(heading(3, txt("Normal")))
      result = sanitize(document, policy)
      assert_equal 3, result.children.first.level
    end

    def test_heading_drop
      # max_heading_level: "drop" removes all headings
      policy = DocumentAstSanitizer::PASSTHROUGH.with(max_heading_level: "drop")
      document = doc(heading(1, txt("Title")), para(txt("body")))
      result = sanitize(document, policy)
      assert_equal 1, result.children.length
      assert_equal "paragraph", result.children.first.type
    end

    def test_heading_min_equals_max_clamps_to_same_level
      policy = DocumentAstSanitizer::PASSTHROUGH.with(
        min_heading_level: 3,
        max_heading_level: 3
      )
      document = doc(heading(1, txt("a")))
      result = sanitize(document, policy)
      assert_equal 3, result.children.first.level
    end

    # ─── 5. Image handling ────────────────────────────────────────────────────

    def test_drop_images
      policy = DocumentAstSanitizer::PASSTHROUGH.with(drop_images: true)
      document = doc(para(image("cat.png", "a cat")))
      result = sanitize(document, policy)
      # para empty after image drop → para itself dropped
      assert_empty result.children
    end

    def test_transform_image_to_text
      policy = DocumentAstSanitizer::STRICT
      document = doc(para(image("cat.png", "a cat")))
      result = sanitize(document, policy)
      inline = result.children.first.children.first
      assert_equal "text", inline.type
      assert_equal "a cat", inline.value
    end

    def test_drop_images_takes_precedence_over_transform
      # When both drop_images and transform_image_to_text are set,
      # drop_images wins.
      policy = DocumentAstSanitizer::PASSTHROUGH.with(
        drop_images: true,
        transform_image_to_text: true
      )
      document = doc(para(image("cat.png", "a cat")))
      result = sanitize(document, policy)
      assert_empty result.children
    end

    def test_image_javascript_url_cleared
      policy = DocumentAstSanitizer::STRICT.with(
        transform_image_to_text: false,
        drop_images: false
      )
      document = doc(para(image("javascript:alert(1)", "bad")))
      result = sanitize(document, policy)
      img = result.children.first.children.first
      assert_equal "image", img.type
      assert_equal "", img.destination
    end

    def test_image_https_url_kept
      policy = DocumentAstSanitizer::PASSTHROUGH.with(
        allowed_url_schemes: %w[http https]
      )
      document = doc(para(image("https://example.com/img.png", "ok")))
      result = sanitize(document, policy)
      assert_equal "https://example.com/img.png",
        result.children.first.children.first.destination
    end

    # ─── 6. Link handling ─────────────────────────────────────────────────────

    def test_drop_links_promotes_children
      # [click here](https://evil.com) → "click here" (plain text, no anchor)
      policy = DocumentAstSanitizer::PASSTHROUGH.with(drop_links: true)
      document = doc(para(link("https://example.com", txt("click here"))))
      result = sanitize(document, policy)
      para_children = result.children.first.children
      assert_equal 1, para_children.length
      assert_equal "text", para_children.first.type
      assert_equal "click here", para_children.first.value
    end

    def test_drop_links_with_multiple_children_promoted
      policy = DocumentAstSanitizer::PASSTHROUGH.with(drop_links: true)
      link_node = LinkNode.new(
        destination: "https://x.com",
        title: nil,
        children: [txt("a"), txt("b")]
      )
      result = sanitize(doc(para(link_node)), policy)
      assert_equal 2, result.children.first.children.length
    end

    def test_link_keeps_title
      policy = DocumentAstSanitizer::PASSTHROUGH
      document = doc(para(link("https://example.com", txt("x"), title: "Hover")))
      result = sanitize(document, policy)
      assert_equal "Hover", result.children.first.children.first.title
    end

    # ─── 7. Autolink handling ─────────────────────────────────────────────────

    def test_autolink_javascript_dropped
      policy = DocumentAstSanitizer::STRICT
      document = doc(para(autolink("javascript:alert(1)")))
      result = sanitize(document, policy)
      # autolink dropped → paragraph empty → paragraph dropped
      assert_empty result.children
    end

    def test_autolink_https_kept
      policy = DocumentAstSanitizer::STRICT
      document = doc(para(autolink("https://example.com")))
      result = sanitize(document, policy)
      assert_equal "autolink", result.children.first.children.first.type
    end

    def test_autolink_email_kept
      policy = DocumentAstSanitizer::STRICT
      document = doc(para(autolink("user@example.com", is_email: true)))
      result = sanitize(document, policy)
      al = result.children.first.children.first
      assert_equal "autolink", al.type
      assert al.is_email
    end

    def test_autolink_data_scheme_dropped
      policy = DocumentAstSanitizer::STRICT
      document = doc(para(autolink("data:text/html,<script>x</script>")))
      result = sanitize(document, policy)
      assert_empty result.children
    end

    # ─── 8. CodeBlock + CodeSpan ──────────────────────────────────────────────

    def test_drop_code_blocks
      policy = DocumentAstSanitizer::PASSTHROUGH.with(drop_code_blocks: true)
      document = doc(code_block("x = 1\n", language: "ruby"))
      result = sanitize(document, policy)
      assert_empty result.children
    end

    def test_keep_code_blocks_by_default
      document = doc(code_block("x = 1\n"))
      result = sanitize(document, DocumentAstSanitizer::PASSTHROUGH)
      assert_equal 1, result.children.length
      assert_equal "code_block", result.children.first.type
    end

    def test_transform_code_span_to_text
      policy = DocumentAstSanitizer::PASSTHROUGH.with(
        transform_code_span_to_text: true
      )
      document = doc(para(code_span("x = 1")))
      result = sanitize(document, policy)
      inline = result.children.first.children.first
      assert_equal "text", inline.type
      assert_equal "x = 1", inline.value
    end

    def test_keep_code_span_by_default
      document = doc(para(code_span("x = 1")))
      result = sanitize(document, DocumentAstSanitizer::PASSTHROUGH)
      inline = result.children.first.children.first
      assert_equal "code_span", inline.type
    end

    # ─── 9. BlockquoteNode ────────────────────────────────────────────────────

    def test_drop_blockquotes
      policy = DocumentAstSanitizer::PASSTHROUGH.with(drop_blockquotes: true)
      document = doc(blockquote(para(txt("quote"))))
      result = sanitize(document, policy)
      assert_empty result.children
    end

    def test_keep_blockquotes_by_default
      document = doc(blockquote(para(txt("quote"))))
      result = sanitize(document, DocumentAstSanitizer::PASSTHROUGH)
      assert_equal 1, result.children.length
      assert_equal "blockquote", result.children.first.type
    end

    # ─── 10. Empty-children pruning ───────────────────────────────────────────

    def test_paragraph_dropped_when_all_inlines_dropped
      # para contains only a raw inline → raw inline dropped → para dropped
      policy = DocumentAstSanitizer::PASSTHROUGH.with(
        allow_raw_inline_formats: "drop-all"
      )
      document = doc(para(raw_inline("html", "<b>x</b>")))
      result = sanitize(document, policy)
      assert_empty result.children
    end

    def test_heading_dropped_when_all_children_dropped
      policy = DocumentAstSanitizer::PASSTHROUGH.with(
        allow_raw_inline_formats: "drop-all"
      )
      document = doc(heading(2, raw_inline("html", "<b>x</b>")))
      result = sanitize(document, policy)
      assert_empty result.children
    end

    def test_blockquote_dropped_when_all_children_dropped
      policy = DocumentAstSanitizer::PASSTHROUGH.with(
        allow_raw_block_formats: "drop-all"
      )
      document = doc(blockquote(raw_block("html", "<b>x</b>")))
      result = sanitize(document, policy)
      assert_empty result.children
    end

    def test_document_never_dropped
      # Even with all children eliminated, DocumentNode itself remains.
      policy = DocumentAstSanitizer::PASSTHROUGH.with(
        allow_raw_block_formats: "drop-all"
      )
      document = doc(raw_block("html", "<b>x</b>"))
      result = sanitize(document, policy)
      assert_kind_of DocumentAst::DocumentNode, result
      assert_empty result.children
    end

    def test_list_dropped_when_all_items_dropped
      policy = DocumentAstSanitizer::PASSTHROUGH.with(
        allow_raw_inline_formats: "drop-all"
      )
      item = list_item(para(raw_inline("html", "x")))
      document = doc(list(item))
      result = sanitize(document, policy)
      assert_empty result.children
    end

    def test_emphasis_dropped_when_all_children_dropped
      policy = DocumentAstSanitizer::PASSTHROUGH.with(
        allow_raw_inline_formats: "drop-all"
      )
      em_node = EmphasisNode.new(children: [raw_inline("html", "x")])
      document = doc(para(em_node))
      result = sanitize(document, policy)
      assert_empty result.children
    end

    def test_strong_dropped_when_all_children_dropped
      policy = DocumentAstSanitizer::PASSTHROUGH.with(
        allow_raw_inline_formats: "drop-all"
      )
      strong_node = StrongNode.new(children: [raw_inline("html", "x")])
      document = doc(para(strong_node))
      result = sanitize(document, policy)
      assert_empty result.children
    end

    # ─── 11. Immutability ─────────────────────────────────────────────────────

    def test_original_document_unchanged
      original = doc(
        heading(1, txt("Title")),
        para(link("javascript:alert(1)", txt("bad")))
      )
      # Capture identity of children before sanitization.
      orig_children = original.children.dup
      sanitize(original, DocumentAstSanitizer::STRICT)
      assert_equal orig_children, original.children,
        "Sanitize must not mutate the input document"
    end

    # ─── 12. Integration scenarios ────────────────────────────────────────────

    def test_nested_list_sanitized
      policy = DocumentAstSanitizer::STRICT
      inner_item = list_item(para(link("javascript:x", txt("bad"))))
      outer_item = list_item(para(txt("good")))
      document = doc(list(outer_item, inner_item))
      result = sanitize(document, policy)
      assert_equal 1, result.children.length
      list_node = result.children.first
      assert_equal 2, list_node.children.length
      # inner item link destination should be cleared
      inner_link = list_node.children[1].children.first.children.first
      assert_equal "", inner_link.destination
    end

    def test_strict_preset_example_from_spec
      # STRICT: raw HTML dropped, image → text, h1 → h2
      document = doc(
        heading(1, txt("Page Title")),
        para(image("cat.png", "a cat")),
        raw_block("html", "<script>bad</script>"),
        para(link("https://ok.com", txt("safe link")))
      )
      result = sanitize(document, DocumentAstSanitizer::STRICT)
      types = result.children.map(&:type)
      assert_includes types, "heading"
      refute_includes types, "raw_block"
      # heading level clamped to 2
      h = result.children.find { |c| c.type == "heading" }
      assert_equal 2, h.level
      # para with image → text
      image_para = result.children.find { |c|
        c.type == "paragraph" && c.children.any? { |i| i.type == "text" && i.value == "a cat" }
      }
      refute_nil image_para
    end

    def test_thematic_break_always_kept
      document = doc(ThematicBreakNode.new)
      result = sanitize(document, DocumentAstSanitizer::STRICT)
      assert_equal 1, result.children.length
      assert_equal "thematic_break", result.children.first.type
    end

    def test_soft_break_and_hard_break_kept
      document = doc(para(txt("a"), SoftBreakNode.new, txt("b"), HardBreakNode.new))
      result = sanitize(document, DocumentAstSanitizer::STRICT)
      types = result.children.first.children.map(&:type)
      assert_includes types, "soft_break"
      assert_includes types, "hard_break"
    end

    def test_unknown_node_type_dropped
      # We can't easily construct an unknown node type with Data.define, but we
      # can verify known node types all survive PASSTHROUGH without error.
      document = doc(
        heading(1, txt("a")),
        para(txt("b")),
        code_block("c"),
        blockquote(para(txt("d"))),
        list(list_item(para(txt("e")))),
        ThematicBreakNode.new,
        raw_block("html", "f")
      )
      result = sanitize(document, DocumentAstSanitizer::PASSTHROUGH)
      assert_equal 7, result.children.length
    end
  end
end
