# frozen_string_literal: true

# AST Sanitizer — policy-driven tree transformation
#
# The sanitizer performs a **single recursive descent** of the DocumentNode
# tree, applying the caller-supplied SanitizationPolicy at each node.
#
# == Transformation model
#
# For each node the sanitizer returns one of:
#
#   - An Array of nodes  (0..N replacement nodes — used when promoting
#                         link children or dropping a node entirely)
#   - A single node      (keep or transform)
#
# The caller is responsible for flattening :promote results into the parent's
# children array.
#
# == Purity and immutability
#
# The sanitizer NEVER mutates its input. All returned nodes are freshly
# constructed Data.define instances (which are frozen in Ruby 3.2+). Callers
# may safely pass the same document through multiple sanitizers with different
# policies.
#
# == Truth table (mirrors spec TE02)
#
#   Node type          | Condition                              | Action
#   ───────────────────┼────────────────────────────────────────┼──────────────────────────────
#   DocumentNode       | always                                 | recurse into children
#   HeadingNode        | max_heading_level == "drop"            | drop node
#   HeadingNode        | level < min_heading_level              | clamp up, recurse
#   HeadingNode        | level > max_heading_level              | clamp down, recurse
#   HeadingNode        | otherwise                              | recurse
#   ParagraphNode      | always                                 | recurse (drop if empty)
#   CodeBlockNode      | drop_code_blocks                       | drop
#   CodeBlockNode      | otherwise                              | keep as-is
#   BlockquoteNode     | drop_blockquotes                       | drop
#   BlockquoteNode     | otherwise                              | recurse (drop if empty)
#   ListNode           | always                                 | recurse (drop if empty)
#   ListItemNode       | always                                 | recurse (drop if empty)
#   ThematicBreakNode  | always                                 | keep as-is
#   RawBlockNode       | allow_raw_block_formats="drop-all"     | drop
#   RawBlockNode       | allow_raw_block_formats="passthrough"  | keep as-is
#   RawBlockNode       | allow_raw_block_formats=[...]          | keep if format in list
#   TextNode           | always                                 | keep as-is
#   EmphasisNode       | always                                 | recurse (drop if empty)
#   StrongNode         | always                                 | recurse (drop if empty)
#   CodeSpanNode       | transform_code_span_to_text            | → TextNode { value }
#   CodeSpanNode       | otherwise                              | keep as-is
#   LinkNode           | drop_links                             | promote children
#   LinkNode           | URL scheme blocked                     | keep, destination=""
#   LinkNode           | otherwise                              | sanitize URL, recurse
#   ImageNode          | drop_images                            | drop
#   ImageNode          | transform_image_to_text                | → TextNode { value: alt }
#   ImageNode          | URL scheme blocked                     | keep, destination=""
#   ImageNode          | otherwise                              | sanitize URL, keep
#   AutolinkNode       | URL scheme blocked                     | drop
#   AutolinkNode       | otherwise                              | sanitize URL, keep
#   RawInlineNode      | allow_raw_inline_formats="drop-all"    | drop
#   RawInlineNode      | allow_raw_inline_formats="passthrough" | keep as-is
#   RawInlineNode      | allow_raw_inline_formats=[...]         | keep if format in list
#   HardBreakNode      | always                                 | keep as-is
#   SoftBreakNode      | always                                 | keep as-is
#
# == Empty-children pruning
#
# When a container node (HeadingNode, ParagraphNode, BlockquoteNode,
# EmphasisNode, StrongNode, ListNode, ListItemNode) ends up with zero
# children after its children are recursively sanitized, the container
# itself is dropped. This prevents empty <p></p>, <em></em>, etc. in the
# output.
#
# Exception: DocumentNode is never dropped — an empty document is valid.

require_relative "url_utils"

module CodingAdventures
  module DocumentAstSanitizer
    # Entry point. Returns a new DocumentNode with the policy applied.
    #
    # @param document [DocumentAst::DocumentNode]
    # @param policy [SanitizationPolicy]
    # @return [DocumentAst::DocumentNode]
    def self.sanitize(document, policy)
      # Always produce a DocumentNode. DocumentNode is never dropped, even
      # when all children are eliminated.
      new_children = sanitize_children(document.children, policy)
      DocumentAst::DocumentNode.new(children: new_children)
    end

    # ─── Private helpers ───────────────────────────────────────────────────────

    # Recursively sanitize a list of block or inline nodes.
    # Returns a flat Array — some nodes may "expand" into multiple nodes
    # (link children promotion), and some are dropped (return []).
    #
    # @param nodes [Array<DocumentAst node>]
    # @param policy [SanitizationPolicy]
    # @return [Array<DocumentAst node>]
    def self.sanitize_children(nodes, policy)
      nodes.flat_map { |node| sanitize_node(node, policy) }
    end
    private_class_method :sanitize_children

    # Sanitize a single node. Returns an Array:
    #   []    — node is dropped
    #   [n]   — node is kept (possibly transformed)
    #   [a,b] — node is replaced by multiple nodes (link-children promotion)
    #
    # @param node [DocumentAst node]
    # @param policy [SanitizationPolicy]
    # @return [Array<DocumentAst node>]
    def self.sanitize_node(node, policy) # rubocop:disable Metrics/MethodLength, Metrics/CyclomaticComplexity, Metrics/PerceivedComplexity
      case node.type

      # ── Block nodes ────────────────────────────────────────────────────────

      when "heading"
        sanitize_heading(node, policy)

      when "paragraph"
        new_children = sanitize_children(node.children, policy)
        # Drop empty paragraphs — they would render as <p></p>.
        return [] if new_children.empty?
        [DocumentAst::ParagraphNode.new(children: new_children)]

      when "code_block"
        return [] if policy.drop_code_blocks
        [node]

      when "blockquote"
        return [] if policy.drop_blockquotes
        new_children = sanitize_children(node.children, policy)
        return [] if new_children.empty?
        [DocumentAst::BlockquoteNode.new(children: new_children)]

      when "list"
        new_children = sanitize_children(node.children, policy)
        return [] if new_children.empty?
        [DocumentAst::ListNode.new(
          ordered: node.ordered,
          start: node.start,
          tight: node.tight,
          children: new_children
        )]

      when "list_item"
        new_children = sanitize_children(node.children, policy)
        return [] if new_children.empty?
        [DocumentAst::ListItemNode.new(children: new_children)]

      when "task_item"
        new_children = sanitize_children(node.children, policy)
        return [] if new_children.empty?
        [DocumentAst::TaskItemNode.new(checked: node.checked, children: new_children)]

      when "thematic_break"
        [node]

      when "raw_block"
        sanitize_raw_block(node, policy.allow_raw_block_formats)

      when "table"
        new_children = sanitize_children(node.children, policy)
        return [] if new_children.empty?
        [DocumentAst::TableNode.new(align: node.align, children: new_children)]

      when "table_row"
        new_children = sanitize_children(node.children, policy)
        return [] if new_children.empty?
        [DocumentAst::TableRowNode.new(is_header: node.is_header, children: new_children)]

      when "table_cell"
        new_children = sanitize_children(node.children, policy)
        return [] if new_children.empty?
        [DocumentAst::TableCellNode.new(children: new_children)]

      # ── Inline nodes ───────────────────────────────────────────────────────

      when "text"
        [node]

      when "emphasis"
        new_children = sanitize_children(node.children, policy)
        return [] if new_children.empty?
        [DocumentAst::EmphasisNode.new(children: new_children)]

      when "strong"
        new_children = sanitize_children(node.children, policy)
        return [] if new_children.empty?
        [DocumentAst::StrongNode.new(children: new_children)]

      when "strikethrough"
        new_children = sanitize_children(node.children, policy)
        return [] if new_children.empty?
        [DocumentAst::StrikethroughNode.new(children: new_children)]

      when "code_span"
        if policy.transform_code_span_to_text
          [DocumentAst::TextNode.new(value: node.value)]
        else
          [node]
        end

      when "link"
        sanitize_link(node, policy)

      when "image"
        sanitize_image(node, policy)

      when "autolink"
        sanitize_autolink(node, policy)

      when "raw_inline"
        sanitize_raw_inline(node, policy.allow_raw_inline_formats)

      when "hard_break", "soft_break"
        [node]

      else
        # Unknown node type: spec says unknown types must never be silently
        # passed through. Drop them so future node types don't accidentally
        # bypass the sanitizer.
        []
      end
    end
    private_class_method :sanitize_node

    # ─── Heading sanitization ──────────────────────────────────────────────────
    #
    # Truth table:
    #
    #   max_heading_level == "drop"       → drop entirely
    #   level < min_heading_level         → clamp up (increase number = shallower)
    #   level > max_heading_level         → clamp down (decrease number = deeper)
    #   otherwise                         → recurse at same level

    def self.sanitize_heading(node, policy)
      return [] if policy.max_heading_level == "drop"

      level = node.level
      # Clamp level into [min, max] range.
      min = policy.min_heading_level || 1
      max = policy.max_heading_level || 6
      clamped = level.clamp(min, max)

      new_children = sanitize_children(node.children, policy)
      return [] if new_children.empty?

      [DocumentAst::HeadingNode.new(level: clamped, children: new_children)]
    end
    private_class_method :sanitize_heading

    # ─── Raw block / inline ───────────────────────────────────────────────────

    def self.sanitize_raw_block(node, policy_value)
      case policy_value
      when "drop-all"
        []
      when "passthrough", nil
        [node]
      else
        # Array of allowed format strings.
        policy_value.include?(node.format) ? [node] : []
      end
    end
    private_class_method :sanitize_raw_block

    def self.sanitize_raw_inline(node, policy_value)
      case policy_value
      when "drop-all"
        []
      when "passthrough", nil
        [node]
      else
        policy_value.include?(node.format) ? [node] : []
      end
    end
    private_class_method :sanitize_raw_inline

    # ─── Link sanitization ────────────────────────────────────────────────────
    #
    # dropLinks:   promote children to parent (link wrapper removed, text kept)
    # URL blocked: keep LinkNode but replace destination with ""
    # otherwise:   sanitize URL, recurse into children

    def self.sanitize_link(node, policy)
      if policy.drop_links
        # Promote children: the link text survives, the anchor wrapper does not.
        return sanitize_children(node.children, policy)
      end

      new_dest = UrlUtils.sanitize_url(node.destination, policy.allowed_url_schemes)
      new_children = sanitize_children(node.children, policy)
      return [] if new_children.empty?

      [DocumentAst::LinkNode.new(
        destination: new_dest,
        title: node.title,
        children: new_children
      )]
    end
    private_class_method :sanitize_link

    # ─── Image sanitization ───────────────────────────────────────────────────
    #
    # drop_images:           drop entirely
    # transform_image_to_text: replace with TextNode containing alt text
    # URL blocked:           keep with destination=""
    # otherwise:             sanitize URL, keep as-is

    def self.sanitize_image(node, policy)
      return [] if policy.drop_images

      if policy.transform_image_to_text
        # Alt text may be empty string — produce an empty TextNode rather than
        # nothing, matching the spec intent of "text fallback".
        return [DocumentAst::TextNode.new(value: node.alt)]
      end

      new_dest = UrlUtils.sanitize_url(node.destination, policy.allowed_url_schemes)
      [DocumentAst::ImageNode.new(
        destination: new_dest,
        title: node.title,
        alt: node.alt
      )]
    end
    private_class_method :sanitize_image

    # ─── Autolink sanitization ────────────────────────────────────────────────
    #
    # URL blocked: drop entirely (unlike LinkNode, autolinks have no children
    #              to promote — the text IS the URL).
    # otherwise:   sanitize URL, keep as-is.

    def self.sanitize_autolink(node, policy)
      new_dest = UrlUtils.sanitize_url(node.destination, policy.allowed_url_schemes)
      # If the URL was blocked (sanitize_url returned ""), drop the node.
      return [] if new_dest.empty? && !node.destination.empty?

      [DocumentAst::AutolinkNode.new(
        destination: new_dest,
        is_email: node.is_email
      )]
    end
    private_class_method :sanitize_autolink
  end
end
