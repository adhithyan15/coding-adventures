# frozen_string_literal: true

# AsciiDoc block parser — state machine implementation.
#
# Reads AsciiDoc source text line by line and emits Document AST block nodes.
#
# === States ===
#
#   :normal            — between blocks; dispatching each new line
#   :paragraph         — accumulating paragraph lines
#   :code_block        — inside a ---- fenced code block
#   :literal_block     — inside a .... literal block
#   :passthrough_block — inside a ++++ passthrough block
#   :quote_block       — inside a ____ quote/blockquote block
#   :unordered_list    — accumulating * list items
#   :ordered_list      — accumulating . list items
#
# === Line dispatch in :normal state ===
#
#   blank line         → stay :normal (flush any pending list)
#   // comment         → skip
#   [source,lang]      → record pending_language
#   = text             → HeadingNode(level=1)   … up to ====== = level 6
#   ''' (≥3)           → ThematicBreakNode
#   ---- (≥4)          → enter :code_block
#   .... (≥4)          → enter :literal_block
#   ++++ (≥4)          → enter :passthrough_block
#   ____ (≥4)          → enter :quote_block
#   * text / ** text   → :unordered_list (level = count of leading *)
#   . text / .. text   → :ordered_list   (level = count of leading .)
#   other text         → enter :paragraph
#
# === List nesting ===
#
# AsciiDoc uses repeated markers for nesting depth:
#   * Level 1
#   ** Level 2
#   *** Level 3
#
# Items are collected as (level, text) pairs and then grouped into a nested
# ListNode / ListItemNode tree by _build_nested_list.
#
# === Quote blocks ===
#
# Content inside ____ is recursively re-parsed by parse_blocks.
# The result becomes the children of a BlockquoteNode.

require "coding_adventures_document_ast"

module CodingAdventures
  module AsciidocParser
    # Block parser module. Call BlockParser.parse(text) to get a DocumentNode.
    module BlockParser
      include CodingAdventures::DocumentAst

      # Heading line: one or more = signs, a space, and text.
      # Levels beyond 6 are clamped to 6 at construction time.
      HEADING_RE = /\A(={1,})\s+(.*)/

      # [source,lang] attribute line.
      SOURCE_ATTR_RE = /\A\[source\s*,\s*(\S+?)\s*\]\z/i

      # Line comment.
      COMMENT_RE = /\A\/\//

      # Unordered list item: one or more * and a space.
      ULIST_RE = /\A(\*+)\s+(.*)/

      # Ordered list item: one or more . and a space.
      OLIST_RE = /\A(\.+)\s+(.*)/

      # Parse AsciiDoc source text into a DocumentNode.
      #
      # @param text [String] The full AsciiDoc source string.
      # @return [DocumentNode] The root document node.
      #
      # @example
      #   doc = BlockParser.parse("= Hello\n\nWorld.\n")
      #   doc.type               # => "document"
      #   doc.children[0].type   # => "heading"
      def self.parse(text)
        lines = text.split("\n", -1)
        # Ensure a trailing blank line so the final paragraph/list gets flushed.
        lines << "" unless lines.last == ""

        blocks = []
        state = :normal
        para_lines = []
        code_lines = []
        pending_language = nil
        list_items = []   # Array of [level, text]
        list_ordered = false

        flush_paragraph = lambda do
          return if para_lines.empty?

          joined = para_lines.join("\n")
          inline = InlineParser.parse(joined)
          blocks << ParagraphNode.new(children: inline)
          para_lines.clear
        end

        flush_list = lambda do
          return if list_items.empty?

          blocks << build_nested_list(list_items, list_ordered)
          list_items.clear
        end

        lines.each do |line|
          stripped = line.rstrip

          # ── State: :code_block ─────────────────────────────────────────────
          if state == :code_block
            if stripped.match?(/\A-{4,}\z/)
              value = code_lines.join("\n") + (code_lines.empty? ? "" : "\n")
              blocks << CodeBlockNode.new(language: pending_language, value: value)
              pending_language = nil
              code_lines.clear
              state = :normal
            else
              code_lines << line.rstrip
            end
            next
          end

          # ── State: :literal_block ──────────────────────────────────────────
          if state == :literal_block
            if stripped.match?(/\A\.{4,}\z/)
              value = code_lines.join("\n") + (code_lines.empty? ? "" : "\n")
              blocks << CodeBlockNode.new(language: nil, value: value)
              code_lines.clear
              state = :normal
            else
              code_lines << line.rstrip
            end
            next
          end

          # ── State: :passthrough_block ──────────────────────────────────────
          if state == :passthrough_block
            if stripped.match?(/\A\+{4,}\z/)
              value = code_lines.join("\n") + (code_lines.empty? ? "" : "\n")
              blocks << RawBlockNode.new(format: "html", value: value)
              code_lines.clear
              state = :normal
            else
              code_lines << line.rstrip
            end
            next
          end

          # ── State: :quote_block ────────────────────────────────────────────
          if state == :quote_block
            if stripped.match?(/\A_{4,}\z/)
              inner_text = code_lines.join("\n")
              inner_doc = parse(inner_text)
              blocks << BlockquoteNode.new(children: inner_doc.children)
              code_lines.clear
              state = :normal
            else
              code_lines << line.rstrip
            end
            next
          end

          # ── State: :paragraph ──────────────────────────────────────────────
          if state == :paragraph
            if stripped.empty?
              flush_paragraph.call
              state = :normal
              next
            end

            # Detect lines that start a new block construct
            is_new_block = stripped.match?(HEADING_RE) ||
              stripped.match?(/\A'{3,}\z/) ||
              stripped.match?(/\A-{4,}\z/) ||
              stripped.match?(/\A\.{4,}\z/) ||
              stripped.match?(/\A\+{4,}\z/) ||
              stripped.match?(/\A_{4,}\z/)

            if is_new_block
              flush_paragraph.call
              state = :normal
              # Fall through to normal dispatch below
            else
              para_lines << stripped
              next
            end
          end

          # ── State: :unordered_list ─────────────────────────────────────────
          if state == :unordered_list
            if stripped.empty?
              flush_list.call
              state = :normal
              next
            end
            m = ULIST_RE.match(stripped)
            if m
              list_items << [m[1].length, m[2]]
              next
            end
            flush_list.call
            state = :normal
            # Fall through to normal dispatch
          end

          # ── State: :ordered_list ───────────────────────────────────────────
          if state == :ordered_list
            if stripped.empty?
              flush_list.call
              state = :normal
              next
            end
            m = OLIST_RE.match(stripped)
            if m
              list_items << [m[1].length, m[2]]
              next
            end
            flush_list.call
            state = :normal
            # Fall through to normal dispatch
          end

          # ── Normal dispatch ────────────────────────────────────────────────

          next if stripped.empty?

          next if stripped.match?(COMMENT_RE)

          # [source,lang] attribute
          if (m = SOURCE_ATTR_RE.match(stripped))
            pending_language = m[1]
            next
          end

          # Heading
          if (m = HEADING_RE.match(stripped))
            level = [m[1].length, 6].min
            inline = InlineParser.parse(m[2])
            blocks << HeadingNode.new(level: level, children: inline)
            state = :normal
            next
          end

          # Thematic break: three or more single-quotes
          if stripped.match?(/\A'{3,}\z/)
            blocks << ThematicBreakNode.new
            state = :normal
            next
          end

          # Code block: four or more dashes
          if stripped.match?(/\A-{4,}\z/)
            state = :code_block
            next
          end

          # Literal block: four or more dots
          if stripped.match?(/\A\.{4,}\z/)
            state = :literal_block
            next
          end

          # Passthrough block: four or more plus signs
          if stripped.match?(/\A\+{4,}\z/)
            state = :passthrough_block
            next
          end

          # Quote block: four or more underscores
          if stripped.match?(/\A_{4,}\z/)
            state = :quote_block
            next
          end

          # Unordered list item
          if (m = ULIST_RE.match(stripped))
            list_ordered = false
            list_items << [m[1].length, m[2]]
            state = :unordered_list
            next
          end

          # Ordered list item
          if (m = OLIST_RE.match(stripped))
            list_ordered = true
            list_items << [m[1].length, m[2]]
            state = :ordered_list
            next
          end

          # Plain text → paragraph
          para_lines << stripped
          state = :paragraph
        end

        # Flush any remaining state at end of input
        flush_paragraph.call
        flush_list.call

        # Handle unclosed delimited blocks (lenient)
        if state == :code_block && !code_lines.empty?
          value = code_lines.join("\n") + "\n"
          blocks << CodeBlockNode.new(language: pending_language, value: value)
        elsif state == :literal_block && !code_lines.empty?
          value = code_lines.join("\n") + "\n"
          blocks << CodeBlockNode.new(language: nil, value: value)
        elsif state == :passthrough_block && !code_lines.empty?
          value = code_lines.join("\n") + "\n"
          blocks << RawBlockNode.new(format: "html", value: value)
        elsif state == :quote_block && !code_lines.empty?
          inner_text = code_lines.join("\n")
          inner_doc = parse(inner_text)
          blocks << BlockquoteNode.new(children: inner_doc.children)
        end

        DocumentNode.new(children: blocks)
      end

      # Build a nested ListNode tree from a flat array of [level, text] pairs.
      #
      # AsciiDoc uses repeated markers for nesting:
      #   * Level 1
      #   ** Level 2 (nested inside previous)
      #
      # Strategy: since Ruby Data objects are frozen, we cannot mutate existing
      # nodes. Instead we use mutable intermediate hashes to build the tree, then
      # convert them to immutable DocumentAst nodes in a final pass.
      #
      # Each mutable item is a hash:
      #   { text: "...", level: N, sub_items: [...] }
      #
      # We use a stack of (level, mutable_items_array) frames. When a deeper
      # level is encountered we push a new frame; when a shallower level is
      # found we pop frames, attach them as sub_items to the last parent item,
      # then convert to immutable nodes at the end.
      #
      # @param items   [Array<Array(Integer, String)>] Flat list of [level, text].
      # @param ordered [Boolean] True for <ol>, false for <ul>.
      # @return [ListNode]
      def self.build_nested_list(items, ordered)
        # Stack of [level, mutable_items_array] frames.
        # Each mutable item: { inline: [...], sub_items: [] }
        stack = [[1, []]]

        items.each do |level, text|
          inline = InlineParser.parse(text)
          entry = {inline: inline, sub_items: []}

          # Pop frames that are deeper than the current level, attaching them
          # as sub_items of the last entry in the parent frame.
          while stack.length > 1 && stack.last[0] > level
            _completed_level, completed_items = stack.pop
            parent_frame_items = stack.last[1]
            parent_frame_items.last[:sub_items] = completed_items unless parent_frame_items.empty?
          end

          # Push a new deeper frame if needed
          stack.push([level, []]) if stack.last[0] < level

          stack.last[1] << entry
        end

        # Collapse any remaining deeper frames
        while stack.length > 1
          _completed_level, completed_items = stack.pop
          parent_frame_items = stack.last[1]
          parent_frame_items.last[:sub_items] = completed_items unless parent_frame_items.empty?
        end

        # Convert the mutable tree to immutable DocumentAst nodes.
        convert_items(stack[0][1], ordered)
      end

      # Recursively convert mutable item hashes to immutable ListNode.
      #
      # @param mutable_items [Array<Hash>] Array of {inline:, sub_items:} hashes.
      # @param ordered [Boolean]
      # @return [ListNode]
      def self.convert_items(mutable_items, ordered)
        list_children = mutable_items.map do |entry|
          # Build paragraph from the item's inline content
          para = ParagraphNode.new(children: entry[:inline])
          # Recursively build sub-list if there are nested items
          if entry[:sub_items].empty?
            ListItemNode.new(children: [para])
          else
            sub_list = convert_items(entry[:sub_items], ordered)
            ListItemNode.new(children: [para, sub_list])
          end
        end

        ListNode.new(
          ordered: ordered,
          start: ordered ? 1 : nil,
          tight: true,
          children: list_children
        )
      end

      private_class_method :build_nested_list
      private_class_method :convert_items
    end
  end
end
