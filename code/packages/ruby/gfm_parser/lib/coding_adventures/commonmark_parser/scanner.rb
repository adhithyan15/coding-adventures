# frozen_string_literal: true

# String Scanner
#
# A cursor-based scanner over a string. Used by both the block parser
# (to scan individual lines) and the inline parser (to scan inline
# content character by character).
#
# === Design ===
#
# The scanner maintains a position `pos` into the string. All read
# operations advance `pos`. The scanner never backtracks on its own —
# callers must save and restore `pos` explicitly when lookahead fails.
#
# This is the same pattern used by hand-rolled recursive descent parsers
# everywhere: try to match, if it fails, restore the saved position.
#
#   saved = scanner.pos
#   unless scanner.match("```")
#     scanner.pos = saved  # backtrack
#   end
#
# === Character Classification ===
#
# GFM cares about several Unicode character categories:
#   - ASCII punctuation: !"#$%&'()*+,-./:;<=>?@[\]^_`{|}~
#   - Unicode punctuation (for emphasis rules)
#   - ASCII whitespace: space, tab, CR, LF, FF
#   - Unicode whitespace

module CodingAdventures
  module CommonmarkParser
    # ─── Scanner ──────────────────────────────────────────────────────────────

    class Scanner
      attr_accessor :pos
      attr_reader :source

      def initialize(source, start = 0)
        @source = source
        @pos = start
      end

      # True if the scanner has consumed all input.
      def done?
        @pos >= @source.length
      end

      # Number of characters remaining.
      def remaining
        @source.length - @pos
      end

      # Peek at the character at pos+offset without advancing.
      # Returns "" if out of bounds.
      def peek(offset = 0)
        @source[@pos + offset] || ""
      end

      # Peek at `n` characters starting at pos without advancing.
      def peek_slice(n)
        @source.slice(@pos, n) || ""
      end

      # Advance pos by one and return the consumed character.
      def advance
        ch = @source[@pos] || ""
        @pos += 1
        ch
      end

      # Advance pos by `n` characters.
      def skip(n)
        @pos = [@pos + n, @source.length].min
      end

      # If the next characters exactly match `str`, advance past them
      # and return true. Otherwise leave pos unchanged and return false.
      def match(str)
        if @source[@pos, str.length] == str
          @pos += str.length
          true
        else
          false
        end
      end

      # Alias so callers can use the idiomatic `match?` predicate name.
      # Like `match`, this DOES advance `pos` on success — it is not a
      # pure lookahead. The `?` suffix simply signals that a boolean is
      # returned, following the Ruby convention for predicate methods.
      alias_method :match?, :match

      # If the next characters match the regex (anchored at current pos),
      # advance past the match and return the matched string.
      # Otherwise return nil and leave pos unchanged.
      #
      # The regex must NOT have the global flag — we use sticky matching
      # via \G anchor.
      def match_regex(re)
        # Use \G to anchor at current position
        anchored = if re.source.start_with?("\\G")
          re
        else
          Regexp.new("\\G(?:#{re.source})", re.options)
        end
        m = anchored.match(@source, @pos)
        return nil if m.nil?
        @pos += m[0].length
        m[0]
      end

      # Consume characters while the block returns true.
      # Returns the consumed string.
      def consume_while
        start = @pos
        @pos += 1 while !done? && yield(@source[@pos])
        @source.slice(start, @pos - start) || ""
      end

      # Consume the rest of the line (up to but not including the newline).
      def consume_line
        start = @pos
        @pos += 1 while !done? && @source[@pos] != "\n"
        @source.slice(start, @pos - start) || ""
      end

      # Return the rest of the input from current pos without advancing.
      def rest
        @source.slice(@pos..) || ""
      end

      # Return a slice of source from `start` to current pos.
      def slice_from(start)
        @source.slice(start, @pos - start) || ""
      end

      # Skip ASCII spaces and tabs. Returns number of spaces skipped.
      def skip_spaces
        start = @pos
        @pos += 1 while !done? && (@source[@pos] == " " || @source[@pos] == "\t")
        @pos - start
      end

      # Count leading virtual spaces (expanding tabs to 4-space tab stops).
      # Does not advance pos. Returns virtual column from the start of the line.
      def count_indent
        indent = 0
        i = @pos
        while i < @source.length
          ch = @source[i]
          if ch == " "
            indent += 1
            i += 1
          elsif ch == "\t"
            indent += 4 - (indent % 4)
            i += 1
          else
            break
          end
        end
        indent
      end

      # Advance past exactly `n` virtual spaces of indentation (expanding tabs).
      def skip_indent(n)
        remaining = n
        while remaining > 0 && !done?
          ch = @source[@pos]
          if ch == " "
            @pos += 1
            remaining -= 1
          elsif ch == "\t"
            tab_width = 4 - (@pos % 4)
            if tab_width <= remaining
              @pos += 1
              remaining -= tab_width
            else
              break  # partial tab — don't consume
            end
          else
            break
          end
        end
      end
    end

    # ─── Character Classification ─────────────────────────────────────────────

    # ASCII punctuation characters as defined by GFM.
    # Exactly: ! " # $ % & ' ( ) * + , - . / : ; < = > ? @ [ \ ] ^ _ ` { | } ~
    ASCII_PUNCTUATION = '!"#$%&\'()*+,-./:;<=>?@[\\]^_`{|}~'.chars.to_set.freeze

    # True if `ch` is an ASCII punctuation character (GFM definition).
    # Used in the emphasis rules to determine flanking delimiter runs.
    #
    # @param ch [String] A single character
    # @return [Boolean]
    def self.ascii_punctuation?(ch)
      ASCII_PUNCTUATION.include?(ch)
    end

    # True if `ch` is a Unicode punctuation character for GFM flanking.
    #
    # GFM defines this (per the cmark reference implementation) as any
    # ASCII punctuation character OR any character in Unicode categories:
    #   Pc, Pd, Pe, Pf, Pi, Po, Ps (punctuation) or Sm, Sc, Sk, So (symbols).
    #
    # The symbol categories (S*) are included because cmark treats them as
    # punctuation for delimiter flanking (e.g. £ U+00A3 Sc, € U+20AC Sc).
    #
    # @param ch [String] A single character
    # @return [Boolean]
    def self.unicode_punctuation?(ch)
      return false if ch.empty?
      return true if ASCII_PUNCTUATION.include?(ch)
      # Unicode punctuation categories (P*) and symbol categories (S*)
      # Ruby's \p{P} matches punctuation, \p{S} matches symbols
      !!(ch =~ /\p{P}|\p{S}/u)
    end

    # True if `ch` is ASCII whitespace: space (U+0020), tab (U+0009),
    # newline (U+000A), form feed (U+000C), carriage return (U+000D).
    #
    # @param ch [String] A single character
    # @return [Boolean]
    def self.ascii_whitespace?(ch)
      ch == " " || ch == "\t" || ch == "\n" || ch == "\r" || ch == "\f"
    end

    # True if `ch` is Unicode whitespace (any code point with Unicode
    # property White_Space=yes).
    #
    # @param ch [String] A single character
    # @return [Boolean]
    def self.unicode_whitespace?(ch)
      return false if ch.empty?
      # Ruby's \s matches ASCII whitespace (\t\n\f\r\v\x20) plus Unicode
      # whitespace when used with the /u modifier and \p{Zs}.
      # We match the TypeScript implementation: \s test plus special chars.
      !!(ch =~ /[[:space:]]/u) || ch == "\u00A0" || ch == "\u1680" ||
        ch.between?("\u2000", "\u200A") || ch == "\u202F" ||
        ch == "\u205F" || ch == "\u3000"
    end

    # True if `ch` is an ASCII digit (0-9).
    #
    # @param ch [String] A single character
    # @return [Boolean]
    def self.digit?(ch)
      ch.between?("0", "9")
    end

    # Normalize a link label per GFM:
    #   - Strip leading and trailing whitespace
    #   - Collapse internal whitespace runs to a single space
    #   - Fold to lowercase
    #
    # Two labels are equivalent if their normalized forms are equal.
    #
    # Note: GFM §4.7 requires Unicode case folding. In particular,
    # ß (U+00DF) and ẞ (U+1E9E) should fold to "ss". Ruby's downcase
    # handles basic lowercasing; we post-process for these cases.
    #
    # @param label [String] The raw link label (without brackets)
    # @return [String] The normalized label
    def self.normalize_link_label(label)
      label.strip.gsub(/\s+/, " ").downcase.gsub("ß", "ss")
    end

    # Normalize a URL: percent-encode spaces and certain characters that
    # should not appear unencoded in HTML href/src attributes.
    #
    # @param url [String] The raw URL
    # @return [String] The normalized URL
    def self.normalize_url(url)
      # Encode characters that need percent-encoding in HTML attributes
      # but are not already encoded.
      # We use URI::DEFAULT_PARSER.escape but it's deprecated in newer Ruby.
      # Instead, manually encode characters outside the safe set.
      url.gsub(%r{[^\w\-.~:/?#@!$&'()*+,;=%]}) do |ch|
        ch.bytes.map { |b| "%%%02X" % b }.join
      end
    end
  end
end
