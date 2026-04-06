# frozen_string_literal: true

# Block-Level Parser
#
# Phase 1 of CommonMark parsing: split the input into block-level tokens
# and build the structural skeleton of the document.
#
# === Two-Phase Overview ===
#
#   Phase 1 (this file): Block structure
#     Input text → lines → block tree with raw inline content strings
#
#   Phase 2 (inline_parser.rb): Inline content
#     Each block's raw content → inline nodes (emphasis, links, etc.)
#
# === Block Tree Construction ===
#
# Container blocks (document, blockquote, list items) form a stack.
# When a new line arrives, we walk down the stack checking continuations,
# then add the line's content to the appropriate block.
#
# === ModalStateMachine Usage ===
#
# The parser uses a ModalStateMachine from coding_adventures_state_machine
# to track multi-line block state. The key insight is that most of the
# parser runs in a "normal" scanning mode, but certain block types require
# the parser to stay in a distinct mode across multiple lines:
#
#   NORMAL mode    — scanning for block starters line by line
#   FENCED_CODE    — inside ``` or ~~~ block; accumulate raw lines
#   HTML_BLOCK     — inside various HTML block types; ends on specific markers

require "coding_adventures_state_machine"
require_relative "scanner"
require_relative "entities"

module CodingAdventures
  module CommonmarkParser
    # ─── Internal Mutable Block Representations ───────────────────────────────
    #
    # During parsing we use mutable intermediate representations, then freeze
    # them into the readonly DocumentAst node types at the end.

    MutableDocument = Struct.new(:children)
    MutableBlockquote = Struct.new(:children)
    MutableList = Struct.new(:ordered, :marker, :start, :tight, :items, :had_blank_line)
    MutableListItem = Struct.new(:marker, :marker_indent, :content_indent, :children, :had_blank_line)
    MutableParagraph = Struct.new(:lines)
    MutableFencedCode = Struct.new(:fence, :fence_len, :base_indent, :info_string, :lines, :closed)
    MutableIndentedCode = Struct.new(:lines)
    MutableHtmlBlock = Struct.new(:html_type, :lines, :closed)
    MutableHeading = Struct.new(:level, :content)
    MutableThematicBreak = Struct.new
    MutableLinkDef = Struct.new(:label, :destination, :title)

    # ─── ModalStateMachine Setup ──────────────────────────────────────────────

    def self.build_normal_mode_dfa
      # In NORMAL mode the DFA just stays in "scanning" — all interesting logic
      # is procedural in parse_blocks(). The DFA acts as a placeholder.
      CodingAdventures::StateMachine::DFA.new(
        states: Set["scanning"],
        alphabet: Set["blank", "content"],
        transitions: {
          ["scanning", "blank"] => "scanning",
          ["scanning", "content"] => "scanning"
        },
        initial: "scanning",
        accepting: Set["scanning"]
      )
    end

    def self.build_fenced_code_dfa
      # FENCED_CODE mode stays "open" until a closing fence is seen.
      CodingAdventures::StateMachine::DFA.new(
        states: Set["open", "closed"],
        alphabet: Set["blank", "content", "fence"],
        transitions: {
          ["open", "blank"] => "open",
          ["open", "content"] => "open",
          ["open", "fence"] => "closed",
          ["closed", "blank"] => "closed",
          ["closed", "content"] => "closed",
          ["closed", "fence"] => "closed"
        },
        initial: "open",
        accepting: Set["open", "closed"]
      )
    end

    def self.build_html_block_dfa
      CodingAdventures::StateMachine::DFA.new(
        states: Set["open", "closed"],
        alphabet: Set["blank", "content", "end_tag"],
        transitions: {
          ["open", "blank"] => "open",
          ["open", "content"] => "open",
          ["open", "end_tag"] => "closed",
          ["closed", "blank"] => "closed",
          ["closed", "content"] => "closed",
          ["closed", "end_tag"] => "closed"
        },
        initial: "open",
        accepting: Set["open", "closed"]
      )
    end

    PARSER_MODES = {
      "normal" => build_normal_mode_dfa,
      "fenced" => build_fenced_code_dfa,
      "html_block" => build_html_block_dfa
    }.freeze

    PARSER_MODE_TRANSITIONS = {
      ["normal", "enter_fenced"] => "fenced",
      ["fenced", "exit_fenced"] => "normal",
      ["normal", "enter_html"] => "html_block",
      ["html_block", "exit_html"] => "normal"
    }.freeze

    # ─── HTML Block Pattern Helpers ───────────────────────────────────────────

    HTML_BLOCK_1_OPEN = /\A<(?:script|pre|textarea|style)(?:\s|>|$)/i
    HTML_BLOCK_1_CLOSE = /<\/(?:script|pre|textarea|style)>/i
    HTML_BLOCK_2_OPEN = /\A<!--/
    HTML_BLOCK_2_CLOSE = /--!?>/
    HTML_BLOCK_3_OPEN = /\A<\?/
    HTML_BLOCK_3_CLOSE = /\?>/
    HTML_BLOCK_4_OPEN = /\A<![A-Z]/
    HTML_BLOCK_4_CLOSE = />/
    HTML_BLOCK_5_OPEN = /\A<!\[CDATA\[/
    HTML_BLOCK_5_CLOSE = /\]\]>/

    HTML_BLOCK_6_TAGS = Set[
      "address", "article", "aside", "base", "basefont", "blockquote", "body",
      "caption", "center", "col", "colgroup", "dd", "details", "dialog", "dir",
      "div", "dl", "dt", "fieldset", "figcaption", "figure", "footer", "form",
      "frame", "frameset", "h1", "h2", "h3", "h4", "h5", "h6", "head", "header",
      "hr", "html", "iframe", "legend", "li", "link", "main", "menu", "menuitem",
      "meta", "nav", "noframes", "ol", "optgroup", "option", "p", "param",
      "search", "section", "summary", "table", "tbody", "td", "tfoot", "th",
      "thead", "title", "tr", "track", "ul"
    ].freeze

    HTML_BLOCK_6_OPEN = Regexp.new(
      "\\A</?(?:#{HTML_BLOCK_6_TAGS.to_a.join("|")})(?:\\s|>|/>|$)",
      Regexp::IGNORECASE
    ).freeze

    HTML_BLOCK_7_ATTR_PAT = '(?:\s+[a-zA-Z_:][a-zA-Z0-9_:.\-]*(?:\s*=\s*(?:[^\s"\'=<>\x60]+|\'[^\'\n]*\'|"[^"\n]*"))?)'
    HTML_BLOCK_7_OPEN_TAG = Regexp.new("\\A<[A-Za-z][A-Za-z0-9\\-]*(#{HTML_BLOCK_7_ATTR_PAT})*\\s*/?>$").freeze
    HTML_BLOCK_7_CLOSE_TAG = /\A<\/[A-Za-z][A-Za-z0-9-]*\s*>$/

    def self.detect_html_block_type(line)
      stripped = line.lstrip
      return 1 if HTML_BLOCK_1_OPEN.match?(stripped)
      return 2 if HTML_BLOCK_2_OPEN.match?(stripped)
      return 3 if HTML_BLOCK_3_OPEN.match?(stripped)
      return 4 if HTML_BLOCK_4_OPEN.match?(stripped)
      return 5 if HTML_BLOCK_5_OPEN.match?(stripped)
      return 6 if HTML_BLOCK_6_OPEN.match?(stripped)
      return 7 if HTML_BLOCK_7_OPEN_TAG.match?(stripped) || HTML_BLOCK_7_CLOSE_TAG.match?(stripped)
      nil
    end

    def self.html_block_ends?(line, html_type)
      case html_type
      when 1 then HTML_BLOCK_1_CLOSE.match?(line)
      when 2 then HTML_BLOCK_2_CLOSE.match?(line)
      when 3 then HTML_BLOCK_3_CLOSE.match?(line)
      when 4 then HTML_BLOCK_4_CLOSE.match?(line)
      when 5 then HTML_BLOCK_5_CLOSE.match?(line)
      when 6, 7 then /\A\s*\z/.match?(line)
      else false
      end
    end

    # ─── Line Classification Helpers ─────────────────────────────────────────

    # True if the line is blank (empty or only whitespace).
    def self.blank?(line)
      /\A\s*\z/.match?(line)
    end

    # Count leading virtual spaces (expanding tabs to the next 4-column tab stop).
    #
    # `base_col` is the virtual column of line[0] in the original document —
    # necessary after partial-tab stripping, where the string may start mid-tab.
    def self.indent_of(line, base_col = 0)
      col = base_col
      line.each_char do |ch|
        if ch == " "
          col += 1
        elsif ch == "\t"
          col += 4 - (col % 4)
        else
          break
        end
      end
      col - base_col
    end

    # Strip exactly `n` virtual spaces of leading indentation, expanding tabs
    # correctly relative to `base_col`.
    #
    # Returns [stripped_line, next_base_col].
    #
    # **Partial-tab handling**: When a tab spans the strip boundary, we consume
    # the tab character and prepend the leftover expansion spaces to the result.
    def self.strip_indent(line, n, base_col = 0)
      remaining = n
      col = base_col
      i = 0
      while remaining > 0 && i < line.length
        ch = line[i]
        if ch == " "
          i += 1
          remaining -= 1
          col += 1
        elsif ch == "\t"
          w = 4 - (col % 4)
          if w <= remaining
            i += 1
            remaining -= w
            col += w
          else
            # Partial tab: consume it, prepend leftover spaces
            leftover = w - remaining
            return [(" " * leftover) + line[i + 1..], col + remaining]
          end
        else
          break
        end
      end
      [line[i..] || "", col]
    end

    # Compute the virtual column reached after consuming `char_count` characters
    # from `line`, starting at virtual column `start_col`.
    def self.virtual_col_after(line, char_count, start_col = 0)
      col = start_col
      char_count.times do |idx|
        break if idx >= line.length
        col += (line[idx] == "\t") ? (4 - (col % 4)) : 1
      end
      col
    end

    # Extract the info string from a fenced code block opening line.
    def self.extract_info_string(line)
      index = 0
      while index < line.length && (line[index] == "`" || line[index] == "~")
        index += 1
      end
      return "" if index.zero?

      index += 1 while index < line.length && (line[index] == " " || line[index] == "\t")
      raw = (line[index..] || "").strip
      token_end = 0
      token_end += 1 while token_end < raw.length && raw[token_end] != " " && raw[token_end] != "\t"
      CommonmarkParser.decode_entities(apply_backslash_escapes(raw[0...token_end]))
    end

    # Apply backslash escapes — only for ASCII punctuation characters.
    def self.apply_backslash_escapes(s)
      s.gsub(/\\./) do |match|
        ch = match[1]
        ascii_punctuation?(ch) ? ch : match
      end
    end

    # ─── ATX Heading Detection ────────────────────────────────────────────────

    AtxHeading = Struct.new(:level, :content)

    def self.parse_atx_heading(line)
      m = line.match(/\A {0,3}(\#{1,6})([ \t]|$)(.*)/)
      return nil unless m

      hashes = m[1]
      content = (m[3] || "").rstrip

      # Remove closing hash sequence: space/tab + one or more hashes + optional spaces
      content = content.sub(/[ \t]+#+[ \t]*$/, "")
      # If content is now purely hashes (e.g. ### ### → content becomes ###)
      content = "" if content.match?(/\A#+[ \t]*\z/)

      AtxHeading.new(hashes.length, content.strip)
    end

    # ─── Thematic Break Detection ─────────────────────────────────────────────

    def self.thematic_break?(line)
      /\A {0,3}((?:\*[ \t]*){3,}|(?:-[ \t]*){3,}|(?:_[ \t]*){3,})\s*\z/.match?(line)
    end

    # ─── List Item Detection ──────────────────────────────────────────────────

    ListMarker = Struct.new(:ordered, :start, :marker, :marker_len, :space_after, :indent)

    def self.parse_list_marker(line)
      # Unordered: up to 3 spaces + (- * +) + (space, tab, or end-of-line)
      m = line.match(/\A( {0,3})([-*+])( +|\t|$)/)
      if m
        indent = m[1].length
        marker = m[2]
        space = m[3]
        return ListMarker.new(false, 1, marker, indent + 1 + space.length, space.length, indent)
      end

      # Ordered: up to 3 spaces + 1-9 digits + (. or )) + (space, tab, or end-of-line)
      m = line.match(/\A( {0,3})(\d{1,9})([.)])( +|\t|$)/)
      if m
        indent = m[1].length
        num = m[2].to_i
        delim = m[3]
        space = m[4]
        marker_width = m[2].length + 1
        return ListMarker.new(true, num, delim, indent + marker_width + space.length, space.length, indent)
      end

      nil
    end

    # ─── Setext Heading Detection ─────────────────────────────────────────────

    def self.setext_underline(line)
      return 1 if /\A {0,3}=+\s*\z/.match?(line)
      return 2 if /\A {0,3}-+\s*\z/.match?(line)
      nil
    end

    # ─── Link Reference Definition Parsing ───────────────────────────────────

    ParsedLinkDef = Struct.new(:label, :destination, :title, :chars_consumed)

    def self.parse_link_definition(text)
      m = text.match(/\A {0,3}\[([^\]\\\[]*(?:\\.[^\]\\\[]*)*)\]:/)
      return nil unless m

      raw_label = m[1]
      return nil if raw_label.strip.empty?

      label = normalize_link_label(raw_label)
      pos = m[0].length

      # Skip whitespace (including one newline)
      ws_m = text[pos..].match(/\A[ \t]*\n?[ \t]*/)
      pos += ws_m[0].length if ws_m

      # Destination: either <...> or non-whitespace non-control chars
      destination = ""
      if text[pos] == "<"
        angle_m = text[pos..].match(/\A<([^<>\n\\]*(?:\\.[^<>\n\\]*)*)>/)
        return nil unless angle_m
        destination = normalize_url(CommonmarkParser.decode_entities(apply_backslash_escapes(angle_m[1])))
        pos += angle_m[0].length
      else
        # Non-angle-bracket destination: no spaces, no control chars, balanced parens
        depth = 0
        start = pos
        while pos < text.length
          ch = text[pos]
          if ch == "("
            depth += 1
            pos += 1
          elsif ch == ")"
            if depth == 0
              break
            end
            depth -= 1
            pos += 1
          elsif /[\x00-\x20]/.match?(ch)
            break
          elsif ch == "\\"
            pos += 2
          else
            pos += 1
          end
        end
        return nil if pos == start
        destination = normalize_url(CommonmarkParser.decode_entities(apply_backslash_escapes(text[start...pos])))
      end

      # Optional title
      title = nil
      before_title = pos
      spaces_m = text[pos..].match(/\A[ \t]*\n?[ \t]*/)
      if spaces_m && spaces_m[0].length > 0
        pos += spaces_m[0].length
        title_char = text[pos]
        close_char = case title_char
        when '"' then '"'
        when "'" then "'"
        when "(" then ")"
        end

        if close_char
          pos += 1
          title_start = pos
          escaped = false
          while pos < text.length
            ch = text[pos]
            if escaped
              escaped = false
              pos += 1
              next
            end
            if ch == "\\"
              escaped = true
              pos += 1
              next
            end
            if ch == close_char
              pos += 1
              break
            end
            if ch == "\n" && close_char == ")"
              break
            end
            pos += 1
          end

          if text[pos - 1] == close_char
            title = CommonmarkParser.decode_entities(apply_backslash_escapes(text[title_start...pos - 1]))
          else
            pos = before_title
            title = nil
          end
        else
          pos = before_title
        end
      end

      # Must be followed by only whitespace on the rest of the line
      eol_m = text[pos..].match(/\A[ \t]*(?:\n|$)/)
      if eol_m
        pos += eol_m[0].length
      elsif title
        pos = before_title
        title = nil
        eol_m2 = text[pos..].match(/\A[ \t]*(?:\n|$)/)
        return nil unless eol_m2
        pos += eol_m2[0].length
      else
        return nil
      end

      ParsedLinkDef.new(label, destination, title, pos)
    end

    # ─── Main Block Parser ────────────────────────────────────────────────────

    # Parse a CommonMark document into a block-level tree (Phase 1).
    #
    # Returns [mutable_document, link_refs] where link_refs is a Hash
    # mapping normalized labels to {destination:, title:} hashes.
    #
    # @param input [String] The raw Markdown input
    # @return [Array(MutableDocument, Hash)] Block tree and link reference map
    def self.parse_blocks(input)
      # Normalize line endings to LF, then split into lines.
      normalized = input.gsub("\r\n", "\n").tr("\r", "\n")
      raw_lines = normalized.split("\n", -1)
      # The trailing newline at end of input produces a spurious empty string.
      raw_lines.pop if raw_lines.length > 0 && raw_lines.last == ""

      modal = CodingAdventures::StateMachine::ModalStateMachine.new(
        modes: PARSER_MODES,
        mode_transitions: PARSER_MODE_TRANSITIONS,
        initial_mode: "normal"
      )

      link_refs = {}
      root = MutableDocument.new([])

      open_containers = [root]
      current_leaf = nil
      last_line_was_blank = false
      last_blank_inner_container = root

      raw_lines.each do |raw_line|
        orig_blank = blank?(raw_line)

        # ── Container continuation ────────────────────────────────────────
        line_content = raw_line
        line_base_col = 0
        new_containers = [root]
        lazy_paragraph_continuation = false

        container_idx = 1
        while container_idx < open_containers.length
          container = open_containers[container_idx]

          case container
          when MutableBlockquote
            # Strip the blockquote marker `> ` (up to 3 leading spaces, then `>`)
            bq_i = 0
            bq_col = line_base_col
            while bq_i < 3 && bq_i < line_content.length && line_content[bq_i] == " "
              bq_i += 1
              bq_col += 1
            end
            if bq_i < line_content.length && line_content[bq_i] == ">"
              bq_i += 1
              bq_col += 1
              if bq_i < line_content.length
                if line_content[bq_i] == " "
                  bq_i += 1
                  bq_col += 1
                elsif line_content[bq_i] == "\t"
                  w = 4 - (bq_col % 4)
                  bq_i += 1
                  if w > 1
                    line_content = (" " * (w - 1)) + line_content[bq_i..]
                    line_base_col = bq_col + 1
                    new_containers << container
                    container_idx += 1
                    next
                  end
                  bq_col += w
                end
              end
              line_content = line_content[bq_i..]
              line_base_col = bq_col
              new_containers << container
              container_idx += 1
            elsif current_leaf.is_a?(MutableParagraph) && !orig_blank &&
                !thematic_break?(line_content) &&
                !(indent_of(line_content, line_base_col) < 4 && line_content.lstrip.match?(/\A(`{3,}|~{3,})/)) &&
                !parse_atx_heading(line_content)
              lm = parse_list_marker(line_content)
              lm_blank_start = lm ? blank?(line_content[lm.marker_len..]) : false
              if !lm || lm_blank_start
                new_containers << container
                container_idx += 1
                lazy_paragraph_continuation = true
                break
              end
              break
            else
              break
            end
          when MutableList
            new_containers << container
            container_idx += 1
          when MutableListItem
            item = container
            effective_blank = orig_blank || blank?(line_content)
            indent = indent_of(line_content, line_base_col)
            if !effective_blank && indent >= item.content_indent
              line_content, line_base_col = strip_indent(line_content, item.content_indent, line_base_col)
              new_containers << container
              container_idx += 1
            elsif effective_blank
              if item.children.length > 0 || (current_leaf && item == open_containers[container_idx])
                new_containers << container
                container_idx += 1
              else
                break
              end
            elsif current_leaf.is_a?(MutableParagraph) && !orig_blank &&
                !thematic_break?(line_content) &&
                !parse_list_marker(line_content) &&
                !(indent_of(line_content, line_base_col) < 4 && line_content.lstrip.match?(/\A(`{3,}|~{3,})/)) &&
                !parse_atx_heading(line_content)
              new_containers << container
              container_idx += 1
              lazy_paragraph_continuation = true
              break
            else
              break
            end
          else
            break
          end
        end

        prev_inner_container = open_containers.last
        open_containers = new_containers

        blank = orig_blank
        blank = true if !blank && blank?(line_content)

        current_inner_after_continuation = open_containers.last

        # ── Multi-line block continuation ─────────────────────────────────

        if modal.current_mode == "fenced" && current_leaf.is_a?(MutableFencedCode)
          fence = current_leaf
          if current_inner_after_continuation != prev_inner_container
            fence.closed = true
            modal.switch_mode("exit_fenced")
            current_leaf = nil
            # Fall through to normal block processing
          else
            stripped = line_content.lstrip
            fence_char = fence.fence[0]
            closing_fence_re = Regexp.new("\\A#{Regexp.escape(fence_char)}{#{fence.fence_len},}\\s*\\z")
            if indent_of(line_content, line_base_col) < 4 &&
                stripped.match?(closing_fence_re) &&
                !stripped.start_with?((fence_char == "`") ? "~" : "`")
              fence.closed = true
              modal.switch_mode("exit_fenced")
              current_leaf = nil
            else
              fence_line, = strip_indent(line_content, fence.base_indent, line_base_col)
              fence.lines << fence_line
            end
            last_line_was_blank = orig_blank
            next
          end
        end

        if modal.current_mode == "html_block" && current_leaf.is_a?(MutableHtmlBlock)
          html_block = current_leaf
          if current_inner_after_continuation != prev_inner_container
            html_block.closed = true
            modal.switch_mode("exit_html")
            current_leaf = nil
          else
            html_block.lines << line_content
            if html_block_ends?(line_content, html_block.html_type)
              html_block.closed = true
              modal.switch_mode("exit_html")
              current_leaf = nil
            end
            last_line_was_blank = orig_blank
            next
          end
        end

        # Finalize current leaf if we left its container
        if current_inner_after_continuation != prev_inner_container &&
            current_leaf && !lazy_paragraph_continuation
          finalize_block(current_leaf, prev_inner_container, link_refs)
          current_leaf = nil
        end

        # ── Lazy paragraph continuation ────────────────────────────────────
        if lazy_paragraph_continuation && current_leaf.is_a?(MutableParagraph)
          current_leaf.lines << line_content
          last_line_was_blank = false
          next
        end

        # Pop the list if it can't continue
        while !blank && open_containers.length > 1 &&
            open_containers.last.is_a?(MutableList)
          list = open_containers.last
          marker = parse_list_marker(line_content)
          if marker && list.ordered == marker.ordered && list.marker == marker.marker &&
              !thematic_break?(line_content)
            break
          end
          open_containers.pop
        end

        inner_container = open_containers.last

        # ── Blank line handling ─────────────────────────────────────────────

        if blank
          if current_leaf.is_a?(MutableParagraph)
            finalize_block(current_leaf, inner_container, link_refs)
            current_leaf = nil
          elsif current_leaf.is_a?(MutableIndentedCode)
            blank_code_line, = strip_indent(raw_line, 4)
            current_leaf.lines << blank_code_line
          end

          if inner_container.is_a?(MutableListItem)
            inner_container.had_blank_line = true
          end
          if inner_container.is_a?(MutableList)
            inner_container.had_blank_line = true
          end

          last_line_was_blank = true
          last_blank_inner_container = inner_container
          next
        end

        # ── New block detection ─────────────────────────────────────────────

        block_detect_loop = true
        while block_detect_loop

          # After blank line in list, any new content makes the list loose
          if last_line_was_blank && inner_container.is_a?(MutableList) &&
              (last_blank_inner_container.is_a?(MutableList) ||
               last_blank_inner_container.is_a?(MutableListItem))
            inner_container.tight = false
          end

          if last_line_was_blank && inner_container.is_a?(MutableListItem)
            inner_container.had_blank_line = true
          end

          indent = indent_of(line_content, line_base_col)

          # 1. Fenced code block opener
          fence_m = line_content.lstrip.match(/\A(`{3,}|~{3,})/)
          if fence_m && indent < 4
            fence_char = fence_m[1][0]
            fence_len = fence_m[1].length
            info_line = line_content.lstrip[fence_len..]
            info_string = extract_info_string(line_content)

            if fence_char == "`" && info_line.include?("`")
              # fall through to paragraph handling
            else
              close_paragraph(current_leaf, inner_container, link_refs)
              current_leaf = nil

              fenced_block = MutableFencedCode.new(
                fence_char * fence_len, fence_len, indent, info_string, [], false
              )
              add_child(inner_container, fenced_block)
              current_leaf = fenced_block
              modal.process("content")
              modal.switch_mode("enter_fenced")
              last_line_was_blank = false
              break
            end
          end

          # 2. ATX heading
          if indent < 4
            heading = parse_atx_heading(line_content)
            if heading
              close_paragraph(current_leaf, inner_container, link_refs)
              current_leaf = nil
              heading_block = MutableHeading.new(heading.level, heading.content)
              add_child(inner_container, heading_block)
              current_leaf = nil
              last_line_was_blank = false
              break
            end
          end

          # 3. Thematic break (check before list marker to avoid --- confusion)
          if indent < 4 && thematic_break?(line_content)
            if current_leaf.is_a?(MutableParagraph)
              level = setext_underline(line_content)
              if level
                para = current_leaf
                finalize_block(para, inner_container, link_refs)
                if para.lines.length > 0
                  heading_block = MutableHeading.new(level, para.lines.join("\n").strip)
                  remove_last_child(inner_container)
                  add_child(inner_container, heading_block)
                  current_leaf = nil
                  last_line_was_blank = false
                  break
                end
                remove_last_child(inner_container)
                current_leaf = nil
              end
            end

            close_paragraph(current_leaf, inner_container, link_refs)
            current_leaf = nil
            add_child(inner_container, MutableThematicBreak.new)
            last_line_was_blank = false
            break
          end

          # 4. Setext heading underline (when no thematic break matched)
          if indent < 4 && current_leaf.is_a?(MutableParagraph)
            level = setext_underline(line_content)
            if level
              para = current_leaf
              finalize_block(para, inner_container, link_refs)
              if para.lines.length > 0
                heading_block = MutableHeading.new(level, para.lines.join("\n").strip)
                remove_last_child(inner_container)
                add_child(inner_container, heading_block)
                current_leaf = nil
                last_line_was_blank = false
                break
              end
              remove_last_child(inner_container)
              current_leaf = nil
            end
          end

          # 5. HTML block
          if indent < 4
            html_type = detect_html_block_type(line_content)
            if html_type && (html_type != 7 || !current_leaf.is_a?(MutableParagraph))
              close_paragraph(current_leaf, inner_container, link_refs)
              current_leaf = nil

              html_block = MutableHtmlBlock.new(
                html_type, [line_content], html_block_ends?(line_content, html_type)
              )
              add_child(inner_container, html_block)

              unless html_block.closed
                current_leaf = html_block
                modal.process("content")
                modal.switch_mode("enter_html")
              end
              last_line_was_blank = false
              break
            end
          end

          # 6. Blockquote
          if indent < 4 && line_content.lstrip.start_with?(">")
            close_paragraph(current_leaf, inner_container, link_refs)
            current_leaf = nil

            bq_last = last_child(inner_container)
            bq = if bq_last.is_a?(MutableBlockquote) && !last_line_was_blank
              bq_last
            else
              new_bq = MutableBlockquote.new([])
              add_child(inner_container, new_bq)
              new_bq
            end

            open_containers << bq

            # Strip the > marker
            bq_i = 0
            bq_col = line_base_col
            while bq_i < line_content.length && line_content[bq_i] == " " && bq_i < 3
              bq_i += 1
              bq_col += 1
            end
            if bq_i < line_content.length && line_content[bq_i] == ">"
              bq_i += 1
              bq_col += 1
              if bq_i < line_content.length
                if line_content[bq_i] == " "
                  bq_i += 1
                  bq_col += 1
                elsif line_content[bq_i] == "\t"
                  w = 4 - (bq_col % 4)
                  bq_i += 1
                  if w > 1
                    line_content = (" " * (w - 1)) + line_content[bq_i..]
                    line_base_col = bq_col + 1
                    inner_container = bq
                    if blank?(line_content)
                      last_line_was_blank = false
                      break
                    end
                    block_detect_loop = true
                    next
                  end
                  bq_col += w
                end
              end
            end
            line_content = line_content[bq_i..]
            line_base_col = bq_col
            inner_container = bq

            if blank?(line_content)
              last_line_was_blank = false
              break
            end
            block_detect_loop = true
            next
          end

          # 7. List item
          if indent < 4
            marker = parse_list_marker(line_content)
            if marker
              list = nil

              if inner_container.is_a?(MutableList)
                existing_list = inner_container
                if existing_list.ordered == marker.ordered && existing_list.marker == marker.marker
                  list = existing_list
                end
              end

              if list.nil?
                list_last = last_child(inner_container)
                if list_last.is_a?(MutableList)
                  if list_last.ordered == marker.ordered && list_last.marker == marker.marker
                    list = list_last
                  end
                end
              end

              new_line_base_col = virtual_col_after(line_content, marker.marker_len, line_base_col)
              item_content = line_content[marker.marker_len..]

              # Handle tab separator
              if marker.space_after == 1
                sep_char = line_content[marker.marker_len - 1]
                if sep_char == "\t"
                  sep_col = virtual_col_after(line_content, marker.marker_len - 1, line_base_col)
                  w = 4 - (sep_col % 4)
                  if w > 1
                    item_content = (" " * (w - 1)) + item_content
                    new_line_base_col = sep_col + 1
                  end
                end
              end

              blank_start = blank?(item_content)

              para_in_current = current_leaf.is_a?(MutableParagraph) &&
                last_child(inner_container) == current_leaf
              can_interrupt_para = (!marker.ordered || marker.start == 1 || !list.nil?) &&
                (!blank_start || !para_in_current)

              if !current_leaf.is_a?(MutableParagraph) || can_interrupt_para
                close_paragraph(current_leaf, inner_container, link_refs)
                current_leaf = nil
                if list.nil?
                  list = MutableList.new(
                    marker.ordered, marker.marker, marker.start, true, [], false
                  )
                  add_child(inner_container, list)
                else
                  if list.had_blank_line ||
                      (last_line_was_blank &&
                       (last_blank_inner_container.is_a?(MutableList) ||
                        last_blank_inner_container.is_a?(MutableListItem)))
                    list.tight = false
                  end
                  list.had_blank_line = false
                end

                # W+1 rule
                normal_indent = marker.marker_len
                reduced_indent = marker.marker_len - marker.space_after + 1
                content_indent = (blank_start || marker.space_after >= 5) ? reduced_indent : normal_indent

                item = MutableListItem.new(marker.marker, marker.indent, content_indent, [], false)
                list.items << item
                open_containers << list unless inner_container == list
                open_containers << item

                unless blank_start
                  inner_container = item
                  if marker.space_after >= 5
                    line_base_col = virtual_col_after(line_content, marker.marker_len - marker.space_after + 1, line_base_col)
                    line_content = (" " * (marker.space_after - 1)) + item_content
                  else
                    line_base_col = new_line_base_col
                    line_content = item_content
                  end
                  block_detect_loop = true
                  next
                end

                current_leaf = nil
                last_line_was_blank = false
                break
              end
            end
          end

          # 8. Indented code block (4+ spaces, not inside a paragraph)
          if indent >= 4 && !current_leaf.is_a?(MutableParagraph)
            stripped, = strip_indent(line_content, 4, line_base_col)
            if current_leaf.is_a?(MutableIndentedCode)
              current_leaf.lines << stripped
            else
              close_paragraph(current_leaf, inner_container, link_refs)
              icb = MutableIndentedCode.new([stripped])
              add_child(inner_container, icb)
              current_leaf = icb
            end
            last_line_was_blank = false
            break
          end

          # 9. Paragraph continuation or new paragraph
          if current_leaf.is_a?(MutableParagraph)
            current_leaf.lines << line_content
          else
            close_paragraph(current_leaf, inner_container, link_refs)
            para = MutableParagraph.new([line_content])
            add_child(inner_container, para)
            current_leaf = para
          end

          last_line_was_blank = false
          break
        end
      end

      # Finalize any remaining open leaf block
      if current_leaf
        inner_container = open_containers.last
        finalize_block(current_leaf, inner_container, link_refs)
      end

      modal.switch_mode("exit_fenced") if modal.current_mode == "fenced"
      modal.switch_mode("exit_html") if modal.current_mode == "html_block"

      [root, link_refs]
    end

    # ─── Container Helpers ────────────────────────────────────────────────────

    def self.last_child(container)
      case container
      when MutableDocument then container.children.last
      when MutableBlockquote then container.children.last
      when MutableListItem then container.children.last
      end
    end

    def self.add_child(container, block)
      case container
      when MutableDocument then container.children << block
      when MutableBlockquote then container.children << block
      when MutableListItem then container.children << block
      end
    end

    def self.remove_last_child(container)
      case container
      when MutableDocument then container.children.pop
      when MutableBlockquote then container.children.pop
      when MutableListItem then container.children.pop
      end
    end

    def self.close_paragraph(leaf, container, link_refs)
      if leaf.is_a?(MutableParagraph)
        finalize_block(leaf, container, link_refs)
      elsif leaf.is_a?(MutableIndentedCode)
        icb = leaf
        icb.lines.pop while icb.lines.length > 0 && /\A\s*\z/.match?(icb.lines.last)
      end
    end

    def self.finalize_block(block, _container, link_refs)
      if block.is_a?(MutableParagraph)
        para = block
        text = para.lines.join("\n")
        loop do
          defn = parse_link_definition(text)
          break unless defn
          key = defn.label
          link_refs[key] ||= {destination: defn.destination, title: defn.title}
          text = text[defn.chars_consumed..]
        end
        if text.strip.empty?
          para.lines = []
        else
          para.lines = text.split("\n", -1)
          para.lines[-1] = para.lines.last.rstrip if para.lines.length > 0
        end
      elsif block.is_a?(MutableIndentedCode)
        icb = block
        icb.lines.pop while icb.lines.length > 0 && icb.lines.last == ""
      end
    end

    # ─── AST Conversion ───────────────────────────────────────────────────────

    # Convert the mutable intermediate document into the final DocumentAst nodes.
    #
    # Inline content (emphasis, links, code spans, etc.) is parsed eagerly
    # during this conversion using InlineParser.parse_inline. This avoids the
    # two-phase deferred-mutation approach that would require mutating frozen
    # Ruby Data objects after construction.
    #
    # @param mutable_doc [MutableDocument]
    # @param link_refs [Hash] Link reference definitions collected in Phase 1.
    # @return [DocumentAst::DocumentNode]
    def self.convert_to_ast(mutable_doc, link_refs)
      convert_block = lambda do |block|
        case block
        when MutableDocument
          DocumentAst::DocumentNode.new(
            children: block.children.filter_map { |b| convert_block.call(b) }
          )

        when MutableHeading
          # Parse inline content eagerly — Data objects are frozen so we must
          # supply the fully-resolved children array at construction time.
          children = CommonmarkParser.parse_inline(block.content, link_refs)
          DocumentAst::HeadingNode.new(level: block.level, children: children)

        when MutableParagraph
          return nil if block.lines.empty?
          content = block.lines.map { |l| l.sub(/\A[ \t]+/, "") }.join("\n")
          children = CommonmarkParser.parse_inline(content, link_refs)
          DocumentAst::ParagraphNode.new(children: children)

        when MutableFencedCode
          DocumentAst::CodeBlockNode.new(
            language: block.info_string.empty? ? nil : block.info_string,
            value: block.lines.join("\n") + ((block.lines.length > 0) ? "\n" : "")
          )

        when MutableIndentedCode
          DocumentAst::CodeBlockNode.new(
            language: nil,
            value: block.lines.join("\n") + "\n"
          )

        when MutableBlockquote
          DocumentAst::BlockquoteNode.new(
            children: block.children.filter_map { |b| convert_block.call(b) }
          )

        when MutableList
          is_tight = block.tight && !block.had_blank_line &&
            !block.items.any? { |i| i.had_blank_line && i.children.length > 1 }
          DocumentAst::ListNode.new(
            ordered: block.ordered,
            start: block.ordered ? block.start : nil,
            tight: is_tight,
            children: block.items.filter_map { |item| convert_block.call(item) }
          )

        when MutableListItem
          DocumentAst::ListItemNode.new(
            children: block.children.filter_map { |b| convert_block.call(b) }
          )

        when MutableThematicBreak
          DocumentAst::ThematicBreakNode.new

        when MutableHtmlBlock
          lines = block.lines.dup
          lines.pop while lines.length > 0 && lines.last.strip.empty?
          DocumentAst::RawBlockNode.new(
            format: "html",
            value: lines.join("\n") + "\n"
          )

        when MutableLinkDef
          nil

        end
      end

      convert_block.call(mutable_doc)
    end
  end
end
