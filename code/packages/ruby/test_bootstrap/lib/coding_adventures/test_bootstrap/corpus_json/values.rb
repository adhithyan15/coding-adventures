# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module CorpusJson
      # Strict decoding of corpus values and kind-sensitive equality.
      #
      # Encoding (JSON):
      #   {"kind":"int","value":"-12"}     an Integer (decimal text, any width)
      #   {"kind":"bytes","hex":"00ff"}    a binary String
      #   true / false                     a bool
      #   [ ... ]                          an Array of the above
      #   "text"                           a text String (types, property names)
      module Values
        # A malformed case (not an engine mismatch).
        class CaseError < StandardError; end

        MAX_SIZE = 10_000
        MAX_INT_DIGITS = 1_000
        MAX_DEPTH = 64
        MAX_FRAGMENT_LEN = 200
        MAX_PROBLEM_LEN = 400

        # Decimal digits only, optional leading "-", no leading zeros, no "-0".
        def self.canonical_int?(text)
          text.match?(/\A(0|-?[1-9][0-9]*)\z/)
        end

        def self.lower_hex?(text)
          text.length.even? && text.match?(/\A[0-9a-f]*\z/)
        end

        def self.decode(node, depth = 0)
          raise CaseError, "value nesting deeper than #{MAX_DEPTH}" if depth > MAX_DEPTH

          case node
          when true, false, String then node
          when Array
            raise CaseError, "list exceeds #{MAX_SIZE} elements" if node.size > MAX_SIZE

            node.map { |item| decode(item, depth + 1) }
          when Hash then decode_object(node)
          else raise CaseError, "unsupported JSON value #{show(node)}"
          end
        end

        def self.decode_object(node)
          kind = node["kind"]
          if kind == "int" && node.keys.sort == %w[kind value]
            text = node["value"]
            raise CaseError, "non-canonical int #{show(text)}" unless text.is_a?(String) && canonical_int?(text)
            raise CaseError, "int exceeds #{MAX_INT_DIGITS} digits" if text.length > MAX_INT_DIGITS

            text.to_i
          elsif kind == "bytes" && node.keys.sort == %w[hex kind]
            text = node["hex"]
            unless text.is_a?(String) && lower_hex?(text)
              raise CaseError, "bytes hex must be even-length lowercase: #{show(text)}"
            end
            raise CaseError, "bytes exceed #{MAX_SIZE} bytes" if text.length > 2 * MAX_SIZE

            [text].pack("H*")
          else
            raise CaseError, "unrecognised value node #{show(node)}"
          end
        end

        # Equality that does not conflate true with 1, bytes with text, or lists
        # with anything else.
        def self.same?(a, b, depth = 0)
          return false if depth > MAX_DEPTH
          return false unless kind_of_value(a) == kind_of_value(b)

          case a
          when Array
            a.size == b.size && a.each_index.all? { |i| same?(a[i], b[i], depth + 1) }
          when String then a.bytes == b.bytes
          else a == b
          end
        end

        def self.kind_of_value(v)
          case v
          when true then :yes
          when false then :no
          when Integer then :int
          when Array then :list
          when String then (v.encoding == Encoding::BINARY) ? :bytes : :text
          else :other
          end
        end

        # A short, bounded rendering for problem lines.
        def self.show(value)
          Clip.clip(Canon.render(value), MAX_FRAGMENT_LEN)
        end
      end
    end
  end
end
