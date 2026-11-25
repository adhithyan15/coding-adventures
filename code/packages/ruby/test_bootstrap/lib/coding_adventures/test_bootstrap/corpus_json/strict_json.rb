# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module CorpusJson
      # A small, STRICT JSON reader.
      #
      # Ruby's stdlib `JSON.parse` keeps the last of two duplicate keys and is
      # lenient in other ways.  A conformance fixture is DATA, and a lenient
      # parser quietly papers over exactly the corruption a conformance check
      # exists to notice, so the corpus loader owns its parser.  Refused:
      #
      #   * duplicate object keys, trailing data after the document;
      #   * nesting deeper than MAX_DEPTH containers (refused before recursing,
      #     so a hostile million-deep list cannot exhaust the stack);
      #   * documents over MAX_BYTES bytes or MAX_NODES values;
      #   * invalid UTF-8 and a byte-order mark;
      #   * raw control characters in strings, lone or reversed surrogate escapes,
      #     leading zeros, `+1`, `NaN`, comments, trailing commas.
      #
      # Values become Hash (insertion ordered), Array, String, true, false, nil
      # and Number (a JSON number kept as its literal text: a 30-digit integer
      # must not lose its low digits).
      class StrictJson
        MAX_DEPTH = 64
        MAX_BYTES = 8 * 1024 * 1024
        MAX_NODES = 200_000

        Number = Struct.new(:text)

        class JsonError < StandardError; end

        def self.parse(data)
          raise JsonError, "JSON input must be a String" unless data.is_a?(String)
          raise JsonError, "document of #{data.bytesize} bytes exceeds #{MAX_BYTES}" if data.bytesize > MAX_BYTES

          bytes = data.b
          raise JsonError, "document is not valid UTF-8" unless bytes.dup.force_encoding(Encoding::UTF_8).valid_encoding?

          new(bytes).parse_document
        end

        def initialize(bytes)
          @s = bytes
          @pos = 0
          @nodes = 0
        end

        def parse_document
          skip_ws
          value = parse_value(0)
          skip_ws
          fail_at("trailing data after the document") if @pos < @s.bytesize
          value
        end

        private

        def fail_at(message)
          raise JsonError, "#{message} at byte #{@pos}"
        end

        def peek = @s.getbyte(@pos)

        def skip_ws
          @pos += 1 while [0x20, 0x09, 0x0a, 0x0d].include?(peek)
        end

        def count_node
          @nodes += 1
          fail_at("more than #{MAX_NODES} values") if @nodes > MAX_NODES
        end

        def parse_value(depth)
          count_node
          case peek
          when 0x7b then parse_object(depth)
          when 0x5b then parse_array(depth)
          when 0x22 then parse_string
          when 0x74 then literal("true", true)
          when 0x66 then literal("false", false)
          when 0x6e then literal("null", nil)
          when 0x2d, 0x30..0x39 then parse_number
          when nil then fail_at("unexpected end of input")
          else fail_at("unexpected byte 0x#{peek.to_s(16)}")
          end
        end

        def literal(word, value)
          fail_at("invalid literal") unless @s.byteslice(@pos, word.bytesize) == word
          @pos += word.bytesize
          value
        end

        def parse_array(depth)
          fail_at("nesting deeper than #{MAX_DEPTH}") if depth + 1 > MAX_DEPTH
          @pos += 1
          out = []
          skip_ws
          if peek == 0x5d
            @pos += 1
            return out
          end
          loop do
            skip_ws
            out << parse_value(depth + 1)
            skip_ws
            case peek
            when 0x2c then @pos += 1
            when 0x5d
              @pos += 1
              return out
            else fail_at("expected , or ]")
            end
          end
        end

        def parse_object(depth)
          fail_at("nesting deeper than #{MAX_DEPTH}") if depth + 1 > MAX_DEPTH
          @pos += 1
          out = {}
          skip_ws
          if peek == 0x7d
            @pos += 1
            return out
          end
          loop do
            skip_ws
            fail_at("expected a string key") unless peek == 0x22
            count_node
            key = parse_string
            fail_at("duplicate key #{key[0, 40].inspect}") if out.key?(key)
            skip_ws
            fail_at("expected :") unless peek == 0x3a
            @pos += 1
            skip_ws
            out[key] = parse_value(depth + 1)
            skip_ws
            case peek
            when 0x2c then @pos += 1
            when 0x7d
              @pos += 1
              return out
            else fail_at("expected , or }")
            end
          end
        end

        def parse_number
          start = @pos
          @pos += 1 if peek == 0x2d
          if peek == 0x30
            @pos += 1
            fail_at("leading zero") if digit?(peek)
          elsif digit?(peek)
            @pos += 1 while digit?(peek)
          else
            fail_at("invalid number")
          end
          if peek == 0x2e
            @pos += 1
            fail_at("digits required after .") unless digit?(peek)
            @pos += 1 while digit?(peek)
          end
          if [0x65, 0x45].include?(peek)
            @pos += 1
            @pos += 1 if [0x2b, 0x2d].include?(peek)
            fail_at("digits required in exponent") unless digit?(peek)
            @pos += 1 while digit?(peek)
          end
          Number.new(@s.byteslice(start, @pos - start).force_encoding(Encoding::UTF_8))
        end

        def digit?(byte) = !byte.nil? && byte >= 0x30 && byte <= 0x39

        ESCAPES = {0x22 => "\"", 0x5c => "\\", 0x2f => "/", 0x62 => "\b", 0x66 => "\f", 0x6e => "\n", 0x72 => "\r", 0x74 => "\t"}.freeze
        private_constant :ESCAPES

        def parse_string
          @pos += 1
          out = String.new(encoding: Encoding::UTF_8)
          loop do
            byte = peek
            fail_at("unterminated string") if byte.nil?
            if byte == 0x22
              @pos += 1
              return out
            elsif byte < 0x20
              fail_at("raw control character in string")
            elsif byte == 0x5c
              @pos += 1
              out << escape
            else
              start = @pos
              @pos += 1 until (b = peek).nil? || b == 0x22 || b == 0x5c || b < 0x20
              out << @s.byteslice(start, @pos - start).force_encoding(Encoding::UTF_8)
            end
          end
        end

        def escape
          code = peek
          fail_at("unterminated escape") if code.nil?
          @pos += 1
          return ESCAPES[code] if ESCAPES.key?(code)
          fail_at("invalid escape") unless code == 0x75

          unit = hex4
          if unit.between?(0xD800, 0xDBFF)
            fail_at("lone high surrogate") unless peek == 0x5c && @s.getbyte(@pos + 1) == 0x75
            @pos += 2
            low = hex4
            fail_at("invalid surrogate pair") unless low.between?(0xDC00, 0xDFFF)
            [0x10000 + ((unit - 0xD800) << 10) + (low - 0xDC00)].pack("U")
          elsif unit.between?(0xDC00, 0xDFFF)
            fail_at("lone low surrogate")
          else
            [unit].pack("U")
          end
        end

        def hex4
          text = @s.byteslice(@pos, 4)
          fail_at("invalid \\u escape") unless text && text.bytesize == 4 && text.match?(/\A[0-9a-fA-F]{4}\z/n)
          @pos += 4
          text.to_i(16)
        end
      end
    end
  end
end
