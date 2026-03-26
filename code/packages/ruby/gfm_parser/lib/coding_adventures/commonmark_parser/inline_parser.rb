# frozen_string_literal: true

# Inline Parser
#
# Phase 2 of GFM parsing: scan raw inline content strings (produced
# by the block parser) and emit inline AST nodes — emphasis, links, code
# spans, etc.
#
# === Overview of Inline Constructs ===
#
# GFM recognises ten inline constructs, processed left-to-right:
#
#   1. Backslash escapes       `\*`    → literal `*`
#   2. HTML character refs     `&amp;` → `&`
#   3. Code spans              `` `code` ``
#   4. HTML inline             `<em>`, `<!-- -->`, `<?...?>`
#   5. Autolinks               `<https://example.com>`, `<me@example.com>`
#   6. Hard line breaks        two trailing spaces + newline, or `\` + newline
#   7. Soft line breaks        single newline within a paragraph
#   8. Emphasis / strong       `*em*`, `**strong**`, `_em_`, `__strong__`
#   9. Links                   `[text](url)`, `[text][label]`, `[text][]`
#  10. Images                  `![alt](url)`, `![alt][label]`
#
# === The Delimiter Stack Algorithm ===
#
# Emphasis is the hardest part of GFM inline parsing. The rules are
# context-sensitive: whether `*` or `_` can open or close emphasis depends
# on what precedes and follows the run.
#
# The algorithm has two phases:
#
#   A. SCAN — read the input left-to-right, building a flat list of "tokens":
#      ordinary text, delimiter runs (* ** _ __), code spans, links, etc.
#      Each delimiter run is tagged as can_open, can_close, or both.
#
#   B. RESOLVE — walk the token list, matching openers with the nearest
#      valid closers. For each matched pair, wrap the intervening tokens
#      in an emphasis or strong node.

require_relative "scanner"
require_relative "entities"

module CodingAdventures
  module CommonmarkParser
    # ─── Delimiter Token Types ────────────────────────────────────────────────

    # A delimiter run: maximal run of `*` or `_`.
    DelimiterToken = Struct.new(:char, :count, :can_open, :can_close, :active) do
      def kind = "delimiter"
    end

    # A fully-resolved inline node (produced during scanning).
    NodeToken = Struct.new(:node) do
      def kind = "node"
    end

    # A bracket opener `[` or `![` — may become a link or image.
    BracketToken = Struct.new(:is_image, :active, :source_pos) do
      def kind = "bracket"
    end

    # ─── Main Inline Parser ───────────────────────────────────────────────────

    # Parse a raw inline content string into a list of InlineNode trees.
    #
    # @param raw [String] The raw inline string from the block parser.
    # @param link_refs [Hash] Link reference definitions collected in Phase 1.
    # @return [Array<DocumentAst::InlineNode>]
    def self.parse_inline(raw, link_refs)
      scanner = Scanner.new(raw)
      tokens = []
      bracket_stack = []  # indices into tokens array
      text_buf = ""

      flush_text = -> {
        if text_buf.length > 0
          tokens << NodeToken.new(DocumentAst::TextNode.new(value: text_buf))
          text_buf = ""
        end
      }

      until scanner.done?
        ch = scanner.peek

        # ── 1. Backslash escape ──────────────────────────────────────────────
        if ch == "\\"
          nxt = scanner.peek(1)
          if nxt != "" && ascii_punctuation?(nxt)
            scanner.skip(2)
            text_buf += nxt
            next
          end
          if nxt == "\n"
            scanner.skip(2)
            flush_text.call
            tokens << NodeToken.new(DocumentAst::HardBreakNode.new)
            next
          end
          scanner.skip(1)
          text_buf += "\\"
          next
        end

        # ── 2. HTML character reference ──────────────────────────────────────
        if ch == "&"
          m = scanner.match_regex(/&(?:#[xX][0-9a-fA-F]{1,6}|#[0-9]{1,7}|[a-zA-Z][a-zA-Z0-9]{0,31});/)
          if m
            text_buf += CommonmarkParser.decode_entity(m)
            next
          end
          scanner.skip(1)
          text_buf += "&"
          next
        end

        # ── 3. Code span ──────────────────────────────────────────────────────
        if ch == "`"
          span = try_code_span(scanner)
          if span
            flush_text.call
            tokens << NodeToken.new(span)
            next
          end
          ticks = scanner.consume_while { |c| c == "`" }
          text_buf += ticks
          next
        end

        # ── 4 & 5. HTML inline and autolinks (both start with `<`) ──────────
        if ch == "<"
          autolink = try_autolink(scanner)
          if autolink
            flush_text.call
            tokens << NodeToken.new(autolink)
            next
          end
          html = try_html_inline(scanner)
          if html
            flush_text.call
            tokens << NodeToken.new(html)
            next
          end
          scanner.skip(1)
          text_buf += "<"
          next
        end

        # ── Image opener `![` ────────────────────────────────────────────────
        if ch == "!" && scanner.peek(1) == "["
          flush_text.call
          bracket_stack << tokens.length
          scanner.skip(2)
          tokens << BracketToken.new(true, true, scanner.pos)
          next
        end

        # ── Link opener `[` ──────────────────────────────────────────────────
        if ch == "["
          flush_text.call
          bracket_stack << tokens.length
          scanner.skip(1)
          tokens << BracketToken.new(false, true, scanner.pos)
          next
        end

        # ── Link/image closer `]` ────────────────────────────────────────────
        if ch == "]"
          scanner.skip(1)

          # Handle deactivated non-image bracket opener at top of stack
          if bracket_stack.length > 0
            top_idx = bracket_stack.last
            top_tok = tokens[top_idx]
            if top_tok.is_a?(BracketToken) && !top_tok.active && !top_tok.is_image
              bracket_stack.pop
              text_buf += "]"
              next
            end
          end

          opener_stack_idx = find_active_bracket_opener(bracket_stack, tokens)

          if opener_stack_idx == -1
            text_buf += "]"
            next
          end

          opener_token_idx = bracket_stack[opener_stack_idx]
          opener = tokens[opener_token_idx]

          flush_text.call

          closer_pos = scanner.pos - 1
          inner_text_for_label = scanner.source[opener.source_pos...closer_pos]

          link_result = try_link_after_close(scanner, link_refs, inner_text_for_label)

          if link_result.nil?
            opener.active = false
            bracket_stack.delete_at(opener_stack_idx)
            text_buf += "]"
            next
          end

          flush_text.call

          # Extract inner tokens
          inner_tokens = tokens.slice!(opener_token_idx + 1, tokens.length - opener_token_idx - 1)
          tokens.delete_at(opener_token_idx)
          bracket_stack.delete_at(opener_stack_idx)

          inner_nodes = resolve_emphasis(inner_tokens)

          if opener.is_image
            alt_text = extract_plain_text(inner_nodes)
            tokens << NodeToken.new(DocumentAst::ImageNode.new(
              destination: link_result[:destination],
              title: link_result[:title],
              alt: alt_text
            ))
          else
            tokens << NodeToken.new(DocumentAst::LinkNode.new(
              destination: link_result[:destination],
              title: link_result[:title],
              children: inner_nodes
            ))
            # Deactivate all preceding non-image link openers
            bracket_stack.each do |idx|
              t = tokens[idx]
              t.active = false if t.is_a?(BracketToken) && !t.is_image
            end
          end
          next
        end

        # ── 8. Emphasis / strong delimiter run ────────────────────────────────
        if ch == "*" || ch == "_" || (ch == "~" && scanner.peek(1) == "~")
          flush_text.call
          delim = scan_delimiter_run(scanner, ch)
          tokens << delim
          next
        end

        # ── 6 & 7. Hard break or soft break ──────────────────────────────────
        if ch == "\n"
          scanner.skip(1)
          if text_buf.end_with?("  ") || text_buf.match?(/[ \t]{2,}$/)
            text_buf = text_buf.sub(/[ \t]+$/, "")
            flush_text.call
            tokens << NodeToken.new(DocumentAst::HardBreakNode.new)
          else
            text_buf = text_buf.rstrip
            flush_text.call
            tokens << NodeToken.new(DocumentAst::SoftBreakNode.new)
          end
          next
        end

        # ── Regular character ─────────────────────────────────────────────────
        text_buf += scanner.advance
      end

      flush_text.call

      resolve_emphasis(tokens)
    end

    # ─── Delimiter Run Scanning ───────────────────────────────────────────────

    def self.scan_delimiter_run(scanner, char)
      source = scanner.source
      run_start = scanner.pos
      pre_char = (run_start > 0) ? source[run_start - 1] : ""

      run = scanner.consume_while { |c| c == char }
      count = run.length
      post_char = (scanner.pos < source.length) ? source[scanner.pos] : ""

      after_whitespace = post_char.empty? || unicode_whitespace?(post_char)
      after_punctuation = !post_char.empty? && unicode_punctuation?(post_char)
      before_whitespace = pre_char.empty? || unicode_whitespace?(pre_char)
      before_punctuation = !pre_char.empty? && unicode_punctuation?(pre_char)

      left_flanking =
        !after_whitespace &&
        (!after_punctuation || before_whitespace || before_punctuation)

      right_flanking =
        !before_whitespace &&
        (!before_punctuation || after_whitespace || after_punctuation)

      can_open, can_close = if char == "*"
        [left_flanking, right_flanking]
      elsif char == "~"
        [count >= 2, count >= 2]
      else
        open = left_flanking && (!right_flanking || before_punctuation)
        close = right_flanking && (!left_flanking || after_punctuation)
        [open, close]
      end

      DelimiterToken.new(char, count, can_open, can_close, true)
    end

    # ─── Emphasis Resolution ─────────────────────────────────────────────────

    def self.resolve_emphasis(tokens)
      i = 0
      while i < tokens.length
        token = tokens[i]
        unless token.is_a?(DelimiterToken) && token.can_close && token.active
          i += 1
          next
        end

        closer = token
        opener_idx = -1

        (i - 1).downto(0) do |j|
          t = tokens[j]
          next unless t.is_a?(DelimiterToken) && t.can_open && t.active && t.char == closer.char

          # Mod-3 rule
          if (t.can_open && t.can_close) || (closer.can_open && closer.can_close)
            if (t.count + closer.count) % 3 == 0 && t.count % 3 != 0
              next
            end
          end

          opener_idx = j
          break
        end

        if opener_idx == -1
          i += 1
          next
        end

        opener = tokens[opener_idx]

        use_len = closer.char == "~" ? 2 : ((opener.count >= 2 && closer.count >= 2) ? 2 : 1)
        is_strong = closer.char != "~" && use_len == 2

        inner_slice = tokens[(opener_idx + 1)...i]
        inner_nodes = resolve_emphasis(inner_slice)

        emph_node = if closer.char == "~"
          DocumentAst::StrikethroughNode.new(children: inner_nodes)
        elsif is_strong
          DocumentAst::StrongNode.new(children: inner_nodes)
        else
          DocumentAst::EmphasisNode.new(children: inner_nodes)
        end

        tokens.slice!(opener_idx + 1, i - opener_idx - 1)
        tokens.insert(opener_idx + 1, NodeToken.new(emph_node))

        opener.count -= use_len
        closer.count -= use_len

        if opener.count == 0
          tokens.delete_at(opener_idx)
          i = opener_idx + 1
        else
          i = opener_idx + 2
        end

        if closer.count == 0
          tokens.delete_at(i)
        end

        next
      end

      tokens.flat_map do |tok|
        case tok
        when NodeToken then [tok.node]
        when BracketToken then [DocumentAst::TextNode.new(value: tok.is_image ? "![" : "[")]
        when DelimiterToken then [DocumentAst::TextNode.new(value: tok.char * tok.count)]
        else []
        end
      end
    end

    # ─── Code Span Parsing ────────────────────────────────────────────────────

    def self.try_code_span(scanner)
      saved_pos = scanner.pos
      open_ticks = scanner.consume_while { |c| c == "`" }
      tick_len = open_ticks.length

      content = ""
      until scanner.done?
        if scanner.peek == "`"
          close_ticks = scanner.consume_while { |c| c == "`" }
          if close_ticks.length == tick_len
            # Matching close found
            content = content.gsub(/\r\n|\r|\n/, " ")
            if content.length >= 2 &&
                content[0] == " " &&
                content[-1] == " " &&
                content.strip != ""
              content = content[1..-2]
            end
            return DocumentAst::CodeSpanNode.new(value: content)
          end
          content += close_ticks
        else
          content += scanner.advance
        end
      end

      scanner.pos = saved_pos
      nil
    end

    # ─── HTML Inline Parsing ──────────────────────────────────────────────────

    def self.try_html_inline(scanner)
      return nil unless scanner.peek == "<"
      saved_pos = scanner.pos
      scanner.skip(1)

      ch = scanner.peek

      # HTML comment: <!-- ... -->
      if scanner.match?("!--")
        content_start = scanner.pos
        if scanner.peek == ">" || scanner.peek_slice(2) == "->"
          invalid = (scanner.peek == ">") ? ">" : "->"
          scanner.skip(invalid.length)
          return DocumentAst::RawInlineNode.new(format: "html", value: scanner.source[saved_pos...scanner.pos])
        end
        until scanner.done?
          if scanner.match?("-->")
            content = scanner.source[content_start...scanner.pos - 3]
            if content.end_with?("-")
              scanner.pos = saved_pos
              return nil
            end
            return DocumentAst::RawInlineNode.new(format: "html", value: scanner.source[saved_pos...scanner.pos])
          end
          scanner.skip(1)
        end
        scanner.pos = saved_pos
        return nil
      end

      # Processing instruction: <? ... ?>
      if scanner.match?("?")
        until scanner.done?
          if scanner.match?("?>")
            return DocumentAst::RawInlineNode.new(format: "html", value: scanner.source[saved_pos...scanner.pos])
          end
          scanner.skip(1)
        end
        scanner.pos = saved_pos
        return nil
      end

      # CDATA section: <![CDATA[ ... ]]>
      if scanner.match?("![CDATA[")
        until scanner.done?
          if scanner.match?("]]>")
            return DocumentAst::RawInlineNode.new(format: "html", value: scanner.source[saved_pos...scanner.pos])
          end
          scanner.skip(1)
        end
        scanner.pos = saved_pos
        return nil
      end

      # Declaration: <!UPPER...>
      if scanner.match?("!")
        if /[A-Z]/.match?(scanner.peek)
          scanner.consume_while { |c| c != ">" }
          if scanner.match?(">")
            return DocumentAst::RawInlineNode.new(format: "html", value: scanner.source[saved_pos...scanner.pos])
          end
        end
        scanner.pos = saved_pos
        return nil
      end

      # Closing tag: </tagname>
      if ch == "/"
        scanner.skip(1)
        tag = scanner.consume_while { |c| c =~ /[a-zA-Z0-9-]/ }
        if tag.empty?
          scanner.pos = saved_pos
          return nil
        end
        scanner.skip_spaces
        unless scanner.match?(">")
          scanner.pos = saved_pos
          return nil
        end
        return DocumentAst::RawInlineNode.new(format: "html", value: scanner.source[saved_pos...scanner.pos])
      end

      # Open tag: <tagname attr...> or <tagname attr.../>
      if /[a-zA-Z]/.match?(ch)
        tag_name = scanner.consume_while { |c| c =~ /[a-zA-Z0-9-]/ }
        if tag_name.empty?
          scanner.pos = saved_pos
          return nil
        end

        newlines_in_tag = 0

        loop do
          space_len = scanner.skip_spaces
          if newlines_in_tag == 0 && scanner.peek == "\n"
            newlines_in_tag += 1
            scanner.skip(1)
            space_len += 1 + scanner.skip_spaces
          end
          nxt = scanner.peek
          break if nxt == ">" || nxt == "/" || nxt.empty?
          if nxt == "\n"
            scanner.pos = saved_pos
            return nil
          end
          if space_len == 0
            scanner.pos = saved_pos
            return nil
          end

          unless /[a-zA-Z_:]/.match?(scanner.peek)
            scanner.pos = saved_pos
            return nil
          end
          scanner.consume_while { |c| c =~ /[a-zA-Z0-9_:.-]/ }

          pos_before_eq_spaces = scanner.pos
          scanner.skip_spaces
          if scanner.peek == "="
            scanner.skip(1)
            scanner.skip_spaces
            q = scanner.peek
            if q == '"' || q == "'"
              scanner.skip(1)
              closed = false
              until scanner.done?
                vc = scanner.source[scanner.pos]
                if vc == q
                  scanner.skip(1)
                  closed = true
                  break
                end
                if vc == "\n"
                  if newlines_in_tag >= 1
                    scanner.pos = saved_pos
                    return nil
                  end
                  newlines_in_tag += 1
                end
                scanner.skip(1)
              end
              unless closed
                scanner.pos = saved_pos
                return nil
              end
            else
              unquoted = scanner.consume_while { |c| c !~ /[\s"'=<>`]/ }
              if unquoted.empty?
                scanner.pos = saved_pos
                return nil
              end
            end
          else
            scanner.pos = pos_before_eq_spaces
          end
        end

        self_close = scanner.match("/>")
        unless self_close || scanner.match(">")
          scanner.pos = saved_pos
          return nil
        end
        return DocumentAst::RawInlineNode.new(format: "html", value: scanner.source[saved_pos...scanner.pos])
      end

      scanner.pos = saved_pos
      nil
    end

    # ─── Autolink Parsing ────────────────────────────────────────────────────

    def self.try_autolink(scanner)
      return nil unless scanner.peek == "<"
      saved_pos = scanner.pos
      scanner.skip(1)

      start = scanner.pos

      # Try email autolink: local@domain
      local_part = scanner.consume_while { |c| c !~ /[\s<>@]/ }
      if local_part.length > 0 && scanner.peek == "@"
        scanner.skip(1)
        domain_part = scanner.consume_while { |c| c !~ /[\s<>]/ }
        if domain_part.length > 0 && scanner.match(">")
          if local_part =~ /\A[a-zA-Z0-9.!#$%&'*+\/=?^_`{|}~-]+\z/ &&
              domain_part =~ /\A[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*\z/
            return DocumentAst::AutolinkNode.new(
              destination: local_part + "@" + domain_part,
              is_email: true
            )
          end
        end
      end

      # Retry as URL autolink
      scanner.pos = start
      scheme = scanner.consume_while { |c| c =~ /[a-zA-Z0-9+\-.]/ }
      if scheme.length.between?(2, 32) && scanner.match(":")
        path = scanner.consume_while { |c| c != " " && c != "<" && c != ">" && c != "\n" }
        if scanner.match?(">")
          return DocumentAst::AutolinkNode.new(
            destination: scheme + ":" + path,
            is_email: false
          )
        end
      end

      scanner.pos = saved_pos
      nil
    end

    # ─── Link Destination Parsing ─────────────────────────────────────────────

    def self.try_link_after_close(scanner, link_refs, inner_text)
      saved_pos = scanner.pos

      # Inline link: (destination "title")
      if scanner.peek == "("
        inline_result = try_inline_link(scanner, saved_pos)
        return inline_result if inline_result
        scanner.pos = saved_pos
      end

      # Full/collapsed reference: [label] or []
      if scanner.peek == "["
        scanner.skip(1)
        label_buf = ""
        valid_label = true
        until scanner.done?
          c = scanner.peek
          if c == "]"
            scanner.skip(1)
            break
          end
          if c == "\n" || c == "["
            valid_label = false
            break
          end
          if c == "\\"
            scanner.skip(1)
            label_buf += "\\" + scanner.advance unless scanner.done?
          else
            label_buf += scanner.advance
          end
        end

        if valid_label
          raw_label = (label_buf.strip != "") ? label_buf : inner_text
          label = normalize_link_label(raw_label)
          ref = link_refs[label]
          return {destination: ref[:destination], title: ref[:title]} if ref
        end

        scanner.pos = saved_pos
        return nil
      end

      # Shortcut reference
      label = normalize_link_label(inner_text)
      ref = link_refs[label]
      return {destination: ref[:destination], title: ref[:title]} if ref

      nil
    end

    def self.try_inline_link(scanner, saved_pos)
      scanner.skip(1)  # consume `(`
      skip_optional_spaces_and_newline(scanner)

      destination = ""

      if scanner.peek == "<"
        scanner.skip(1)
        dest_buf = ""
        until scanner.done?
          c = scanner.peek
          return nil if c == "\n" || c == "\r"
          if c == "\\"
            scanner.skip(1)
            nxt = scanner.advance
            dest_buf += ascii_punctuation?(nxt) ? nxt : "\\" + nxt
          elsif c == ">"
            scanner.skip(1)
            break
          elsif c == "<"
            return nil
          else
            dest_buf += scanner.advance
          end
        end
        destination = normalize_url(CommonmarkParser.decode_entities(dest_buf))
      else
        depth = 0
        dest_start = scanner.pos
        until scanner.done?
          c = scanner.peek
          if c == "("
            depth += 1
            scanner.skip(1)
          elsif c == ")"
            break if depth == 0
            depth -= 1
            scanner.skip(1)
          elsif c == "\\"
            scanner.skip(2)
          elsif ascii_whitespace?(c)
            break
          else
            scanner.skip(1)
          end
        end
        dest_raw = scanner.source[dest_start...scanner.pos]
        destination = normalize_url(CommonmarkParser.decode_entities(apply_backslash_escapes(dest_raw)))
      end

      skip_optional_spaces_and_newline(scanner)

      title = nil
      q = scanner.peek
      if q == '"' || q == "'" || q == "("
        close_q = (q == "(") ? ")" : q
        scanner.skip(1)
        title_buf = ""
        until scanner.done?
          c = scanner.peek
          if c == "\\"
            scanner.skip(1)
            nxt = scanner.advance
            title_buf += ascii_punctuation?(nxt) ? nxt : "\\" + nxt
          elsif c == close_q
            scanner.skip(1)
            title = CommonmarkParser.decode_entities(title_buf)
            break
          elsif c == "\n" && q == "("
            break
          else
            title_buf += scanner.advance
          end
        end
      end

      scanner.skip_spaces
      return nil unless scanner.match?(")")

      {destination: destination, title: title}
    end

    def self.skip_optional_spaces_and_newline(scanner)
      scanner.skip_spaces
      if scanner.peek == "\n"
        scanner.skip(1)
        scanner.skip_spaces
      elsif scanner.peek == "\r" && scanner.peek(1) == "\n"
        scanner.skip(2)
        scanner.skip_spaces
      end
    end

    def self.find_active_bracket_opener(bracket_stack, tokens)
      (bracket_stack.length - 1).downto(0) do |i|
        idx = bracket_stack[i]
        t = tokens[idx]
        return i if t.is_a?(BracketToken) && t.active
      end
      -1
    end

    def self.extract_plain_text(nodes)
      result = ""
      nodes.each do |node|
        case node.type
        when "text" then result += node.value
        when "code_span" then result += node.value
        when "hard_break" then result += "\n"
        when "soft_break" then result += " "
        when "emphasis", "strong", "strikethrough", "link"
          result += extract_plain_text(node.children)
        when "image"
          result += node.alt
        when "autolink"
          result += node.destination
        end
      end
      result
    end

    # ─── Document-Level Inline Resolution ────────────────────────────────────

    # Walk the block AST produced by BlockParser.convert_to_ast and fill
    # in inline content.
    #
    # Heading and paragraph nodes produced by convert_to_ast have a @_raw_id
    # instance variable pointing into raw_inline_content. This function
    # retrieves each raw string, parses it into inline nodes, and replaces
    # the empty children arrays with the parsed results.
    #
    # @param document [DocumentAst::DocumentNode]
    # @param raw_inline_content [Hash]
    # @param link_refs [Hash]
    def self.resolve_inline_content(document, raw_inline_content, link_refs)
      walk_block = ->(block) {
        if (block.type == "heading" || block.type == "paragraph") &&
            block.respond_to?(:_raw_id) && block._raw_id
          raw = raw_inline_content[block._raw_id]
          if raw
            inline_nodes = parse_inline(raw, link_refs)
            # Ruby Data objects are frozen — we need to use a mutable wrapper
            block.instance_variable_set(:@_children_override, inline_nodes)
            # Patch the children method on this specific object
            def block.children
              @_children_override || super
            end
          end
        end

        if block.respond_to?(:children) && block.children.is_a?(Array)
          block.children.each { |child| walk_block.call(child) }
        end
      }

      walk_block.call(document)
    end
  end
end
