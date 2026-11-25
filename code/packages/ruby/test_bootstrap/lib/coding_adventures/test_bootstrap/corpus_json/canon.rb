# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module CorpusJson
      # A stable, TOTAL text form of a witness (used in messages and the CI
      # digest).  It never raises: a custom generator can build arbitrarily deep
      # values and a user `inspect` can raise or recurse, and this runs while a
      # verdict is being built.
      #
      #   nil                     nil
      #   true / false            true / false
      #   Integer                 decimal (above 100,000 bits: "<bigint N bits>")
      #   binary String (bytes)   0x<lowercase hex>
      #   text String             "quoted" with \" \\ \n \r \t and \uXXXX escapes
      #   Array                   [a,b]            Hash   {k:v,k:v}
      #   anything else           inspect, control characters escaped
      #
      # Nesting deeper than MAX_DEPTH renders "...", more than MAX_NODES values
      # truncates, total output is capped at MAX_OUTPUT, and an error or
      # SystemStackError from user code turns the WHOLE witness into UNRENDERABLE.
      module Canon
        MAX_DEPTH = 16
        MAX_NODES = 10_000
        MAX_OUTPUT = 1 << 20
        MAX_BIGINT_BITS = 100_000
        UNRENDERABLE = "<unrenderable>"

        class Truncated < StandardError; end
        private_constant :Truncated

        CONTROL = /[\x00-\x1f\x7f\u0080-\u009f\u2028\u2029]/
        QUOTED = /[\\"\x00-\x1f\x7f\u0080-\u009f\u2028\u2029]/

        # Escapes control characters (and, when `quote` is set, backslash and the
        # double quote) so a witness can never forge a log line.
        def self.escape(text, quote: false)
          text.gsub(quote ? QUOTED : CONTROL) do |c|
            case c
            when "\n" then "\\n"
            when "\r" then "\\r"
            when "\t" then "\\t"
            when "\\" then "\\\\"
            when '"' then '\\"'
            else format("\\u%04x", c.ord)
            end
          end
        end

        def self.render(value)
          out = +""
          state = {nodes: 0}
          begin
            emit(value, 0, out, state)
          rescue Truncated
            # one piece may have overshot the cap: cut back (scrub repairs a split character)
            out = out.byteslice(0, MAX_OUTPUT).scrub << "..."
          end
          out
        rescue Exception => e # rubocop:disable Lint/RescueException
          raise unless Safe.recoverable?(e)

          UNRENDERABLE
        end

        def self.append(out, text)
          out << text
          raise Truncated if out.bytesize > MAX_OUTPUT
        end

        def self.emit(value, depth, out, state)
          state[:nodes] += 1
          raise Truncated if state[:nodes] > MAX_NODES

          case value
          when nil then append(out, "nil")
          when true then append(out, "true")
          when false then append(out, "false")
          when Integer then append(out, integer_text(value))
          when String then append(out, string_text(value))
          when Array then container(out, depth, state, "[", "]", value) { |v| emit(v, depth + 1, out, state) }
          when Hash
            container(out, depth, state, "{", "}", value) do |(k, v)|
              emit(k, depth + 1, out, state)
              append(out, ":")
              emit(v, depth + 1, out, state)
            end
          else append(out, escape(value.inspect))
          end
        end

        def self.container(out, depth, state, open, close, items)
          if depth >= MAX_DEPTH
            append(out, "...")
            return
          end
          append(out, open)
          first = true
          items.each do |item|
            append(out, ",") unless first
            first = false
            yield item
          end
          append(out, close)
        end

        def self.integer_text(value)
          bits = value.bit_length
          (bits > MAX_BIGINT_BITS) ? "<bigint #{bits} bits>" : value.to_s
        end

        def self.string_text(value)
          return "0x#{value.unpack1("H*")}" if value.encoding == Encoding::BINARY
          return "0x#{value.b.unpack1("H*")}" unless value.valid_encoding?

          "\"#{escape(value, quote: true)}\""
        end
      end
    end
  end
end
