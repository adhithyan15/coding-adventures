# frozen_string_literal: true

# AsciiDoc inline parser — left-to-right character scanner.
#
# Converts an inline AsciiDoc string into an array of Document AST inline nodes.
#
# === AsciiDoc inline conventions ===
#
#   *bold*    → StrongNode    (constrained single-asterisk)
#   **bold**  → StrongNode    (unconstrained double-asterisk)
#   _italic_  → EmphasisNode  (constrained)
#   __italic_ → EmphasisNode  (unconstrained)
#   `code`    → CodeSpanNode  (verbatim — no nested parsing!)
#   link:url[text]         → LinkNode
#   image:url[alt]         → ImageNode
#   <<anchor,text>>        → LinkNode { destination: "#anchor" }
#   https://url[text]      → LinkNode (explicit text)
#   https://url (bare)     → AutolinkNode
#   "  \n"  (two spaces)  → HardBreakNode
#   "\\\n"                 → HardBreakNode
#   "\n"                   → SoftBreakNode
#
# === Priority order ===
#
# Rules are checked in this sequence to avoid ambiguity:
#   1. Hard break (two trailing spaces + \n OR backslash + \n)
#   2. Soft break (\n)
#   3. Backtick → CodeSpanNode (verbatim)
#   4. ** → StrongNode (unconstrained — BEFORE single *)
#   5. __ → EmphasisNode (unconstrained — BEFORE single _)
#   6. *  → StrongNode (constrained)
#   7. _  → EmphasisNode (constrained)
#   8. link: → LinkNode
#   9. image: → ImageNode
#  10. << → cross-reference LinkNode
#  11. https:// or http:// → LinkNode or AutolinkNode
#  12. Anything else → text accumulation

require "coding_adventures_document_ast"

module CodingAdventures
  module AsciidocParser
    # Inline parser module. Call InlineParser.parse(text) to get an array of
    # Document AST inline node objects.
    module InlineParser
      include CodingAdventures::DocumentAst

      # Parse an AsciiDoc inline string into Document AST inline nodes.
      #
      # @param text [String] The inline AsciiDoc string.
      # @return [Array] An array of inline node objects (DocumentAst nodes).
      #
      # @example
      #   InlineParser.parse("Hello *world*")
      #   # => [TextNode("Hello "), StrongNode([TextNode("world")])]
      def self.parse(text)
        out = []
        buf = +""
        i = 0
        n = text.length

        while i < n
          ch = text[i]

          # Rule 1a: Hard break — two trailing spaces before \n
          if ch == " " && text[i, 2] == "  " && i + 2 < n && text[i + 2] == "\n"
            flush_text(buf, out)
            out << HardBreakNode.new
            i += 3
            next
          end

          # Rule 1b: Hard break — backslash + \n
          if ch == "\\" && i + 1 < n && text[i + 1] == "\n"
            flush_text(buf, out)
            out << HardBreakNode.new
            i += 2
            next
          end

          # Rule 2: Soft break — bare \n
          if ch == "\n"
            flush_text(buf, out)
            out << SoftBreakNode.new
            i += 1
            next
          end

          # Rule 3: Code span — backtick
          if ch == "`"
            close = text.index("`", i + 1)
            if close
              flush_text(buf, out)
              out << CodeSpanNode.new(value: text[i + 1, close - i - 1])
              i = close + 1
              next
            end
          end

          # Rule 4: Strong — unconstrained ** (check BEFORE single *)
          if text[i, 2] == "**"
            close = text.index("**", i + 2)
            if close
              flush_text(buf, out)
              inner = parse(text[i + 2, close - i - 2])
              out << StrongNode.new(children: inner)
              i = close + 2
              next
            end
          end

          # Rule 5: Emphasis — unconstrained __ (check BEFORE single _)
          if text[i, 2] == "__"
            close = text.index("__", i + 2)
            if close
              flush_text(buf, out)
              inner = parse(text[i + 2, close - i - 2])
              out << EmphasisNode.new(children: inner)
              i = close + 2
              next
            end
          end

          # Rule 6: Strong — constrained *
          # AsciiDoc: single * means BOLD (strong), not italic!
          if ch == "*"
            close = text.index("*", i + 1)
            if close
              flush_text(buf, out)
              inner = parse(text[i + 1, close - i - 1])
              out << StrongNode.new(children: inner)
              i = close + 1
              next
            end
          end

          # Rule 7: Emphasis — constrained _
          if ch == "_"
            close = text.index("_", i + 1)
            if close
              flush_text(buf, out)
              inner = parse(text[i + 1, close - i - 1])
              out << EmphasisNode.new(children: inner)
              i = close + 1
              next
            end
          end

          # Rule 8: link:url[text]
          if text[i, 5] == "link:"
            rest = text[i + 5..]
            bracket_open = rest.index("[")
            if bracket_open
              bracket_close = rest.index("]", bracket_open)
              if bracket_close
                url = rest[0, bracket_open]
                label = rest[bracket_open + 1, bracket_close - bracket_open - 1]
                link_text = label.empty? ? url : label
                flush_text(buf, out)
                out << LinkNode.new(destination: url, title: nil, children: [TextNode.new(value: link_text)])
                i += 5 + bracket_close + 1
                next
              end
            end
          end

          # Rule 9: image:url[alt]
          if text[i, 6] == "image:"
            rest = text[i + 6..]
            bracket_open = rest.index("[")
            if bracket_open
              bracket_close = rest.index("]", bracket_open)
              if bracket_close
                url = rest[0, bracket_open]
                alt = rest[bracket_open + 1, bracket_close - bracket_open - 1]
                flush_text(buf, out)
                out << ImageNode.new(destination: url, title: nil, alt: alt)
                i += 6 + bracket_close + 1
                next
              end
            end
          end

          # Rule 10: <<anchor>> and <<anchor,text>>
          if text[i, 2] == "<<"
            close = text.index(">>", i + 2)
            if close
              inner_text = text[i + 2, close - i - 2]
              if inner_text.include?(",")
                anchor, label = inner_text.split(",", 2)
                anchor = anchor.strip
                label = label.strip
              else
                anchor = inner_text.strip
                label = anchor
              end
              flush_text(buf, out)
              out << LinkNode.new(destination: "##{anchor}", title: nil, children: [TextNode.new(value: label)])
              i = close + 2
              next
            end
          end

          # Rule 11: https:// and http://
          matched_scheme = false
          ["https://", "http://"].each do |scheme|
            next unless text[i, scheme.length] == scheme

            # Scan to end of URL (stop at whitespace or [ or ])
            j = i + scheme.length
            j += 1 while j < n && !" \t\n[]".include?(text[j])
            url = text[i, j - i]

            if j < n && text[j] == "["
              # Explicit link text: https://url[text]
              bracket_close = text.index("]", j + 1)
              if bracket_close
                label = text[j + 1, bracket_close - j - 1]
                link_children = [TextNode.new(value: label.empty? ? url : label)]
                flush_text(buf, out)
                out << LinkNode.new(destination: url, title: nil, children: link_children)
                i = bracket_close + 1
                matched_scheme = true
                break
              end
            end

            # Bare URL → AutolinkNode
            flush_text(buf, out)
            out << AutolinkNode.new(destination: url, is_email: false)
            i = j
            matched_scheme = true
            break
          end
          next if matched_scheme

          # Rule 12: Text accumulation
          buf << ch
          i += 1
        end

        flush_text(buf, out)
        out
      end

      # Flush the text buffer into the output array as a TextNode.
      #
      # @param buf [String] Mutable string buffer (will be cleared).
      # @param out [Array]  Output array to append to.
      def self.flush_text(buf, out)
        return if buf.empty?

        out << TextNode.new(value: buf.dup)
        buf.clear
      end

      private_class_method :flush_text
    end
  end
end
