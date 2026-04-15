# coding_adventures/lzss/compressor.rb — LZSS algorithm core.
#
# LZSS (Lempel-Ziv-Storer-Szymanski, 1982) refines LZ77 by replacing the
# mandatory next_char byte with a flag-bit scheme:
#
#   Literal → 1 byte  (flag bit = 0)
#   Match   → 3 bytes (flag bit = 1: offset uint16 BE + length uint8)
#
# Tokens are grouped in blocks of 8. Each block starts with a flag byte
# (bit 0 = first token, bit 7 = eighth token).
#
# Wire format (CMP02):
#   Bytes 0-3: original_length (big-endian uint32)
#   Bytes 4-7: block_count     (big-endian uint32)
#   Bytes 8+:  blocks
#     Each block: [1 byte flag] [1 or 3 bytes per symbol]

module CodingAdventures
  module LZSS
    module Compressor

      # Default parameters matching the CMP02 spec.
      DEFAULT_WINDOW_SIZE = 4096
      DEFAULT_MAX_MATCH   = 255
      DEFAULT_MIN_MATCH   = 3

      # ─── Token types ────────────────────────────────────────────────────────

      # A single literal byte.
      Literal = Struct.new(:byte)

      # A back-reference match.
      Match = Struct.new(:offset, :length)

      # ─── Sliding-window encoder ─────────────────────────────────────────────

      # Find the longest match for data[cursor:] in data[win_start:cursor].
      # Returns [best_offset, best_length]. Matches may overlap (extend past
      # cursor) to enable run-length encoding as a degenerate case.
      def self.find_longest_match(data, cursor, win_start, max_match)
        best_len = 0
        best_off = 0
        lookahead_end = [cursor + max_match, data.length].min

        (win_start...cursor).each do |pos|
          len = 0
          len += 1 while cursor + len < lookahead_end && data[pos + len] == data[cursor + len]
          if len > best_len
            best_len = len
            best_off = cursor - pos
          end
        end

        [best_off, best_len]
      end
      private_class_method :find_longest_match

      # Encode bytes into an LZSS token array.
      #
      # @param data [String] binary string (use #b or encoding ASCII-8BIT)
      # @param window_size [Integer] max lookback distance
      # @param max_match   [Integer] max match length
      # @param min_match   [Integer] min match length for a Match token
      # @return [Array<Literal|Match>]
      def self.encode(data, window_size: DEFAULT_WINDOW_SIZE,
                             max_match:   DEFAULT_MAX_MATCH,
                             min_match:   DEFAULT_MIN_MATCH)
        bytes   = data.bytes
        tokens  = []
        cursor  = 0

        while cursor < bytes.length
          win_start = [cursor - window_size, 0].max
          best_off, best_len = find_longest_match(bytes, cursor, win_start, max_match)

          if best_len >= min_match
            tokens << Match.new(best_off, best_len)
            cursor += best_len
          else
            tokens << Literal.new(bytes[cursor])
            cursor += 1
          end
        end

        tokens
      end

      # Decode an LZSS token array back into the original bytes.
      #
      # @param tokens [Array<Literal|Match>]
      # @param original_length [Integer, nil] truncate to this length if given
      # @return [String] binary string
      def self.decode(tokens, original_length: nil)
        output = []

        tokens.each do |tok|
          case tok
          when Literal
            output << tok.byte
          when Match
            start = output.length - tok.offset
            tok.length.times { |i| output << output[start + i] }
          end
        end

        output = output[0, original_length] if original_length
        output.pack("C*")
      end

      # ─── Serialisation ──────────────────────────────────────────────────────

      # Serialise tokens to the CMP02 wire format.
      def self.serialise_tokens(tokens, original_length)
        blocks = []

        tokens.each_slice(8) do |chunk|
          flag = 0
          symbol_data = "".b

          chunk.each_with_index do |tok, bit|
            if tok.is_a?(Match)
              flag |= (1 << bit)
              symbol_data << [tok.offset].pack("n") << [tok.length].pack("C")
            else
              symbol_data << [tok.byte].pack("C")
            end
          end

          blocks << [flag].pack("C") + symbol_data
        end

        [original_length, blocks.length].pack("NN") + blocks.join
      end

      # Deserialise CMP02 wire-format bytes into tokens and original length.
      #
      # Security: block_count capped against actual payload size.
      def self.deserialise_tokens(data)
        return [[], 0] if data.bytesize < 8

        original_length, block_count = data.unpack("NN")

        # 1 byte minimum per block — cap to prevent DoS from crafted headers.
        max_possible = data.bytesize - 8
        block_count  = [block_count, max_possible].min

        tokens = []
        pos    = 8

        block_count.times do
          break if pos >= data.bytesize

          flag = data.getbyte(pos)
          pos += 1

          8.times do |bit|
            break if pos >= data.bytesize

            if flag & (1 << bit) != 0
              # Match: 3 bytes
              break if pos + 3 > data.bytesize
              offset = data[pos, 2].unpack1("n")
              length = data.getbyte(pos + 2)
              tokens << Match.new(offset, length)
              pos += 3
            else
              # Literal: 1 byte
              tokens << Literal.new(data.getbyte(pos))
              pos += 1
            end
          end
        end

        [tokens, original_length]
      end

      # ─── One-shot API ────────────────────────────────────────────────────────

      # Compress a binary string using LZSS, returning CMP02 wire format bytes.
      def self.compress(data, window_size: DEFAULT_WINDOW_SIZE,
                               max_match:   DEFAULT_MAX_MATCH,
                               min_match:   DEFAULT_MIN_MATCH)
        tokens = encode(data, window_size: window_size,
                               max_match:   max_match,
                               min_match:   min_match)
        serialise_tokens(tokens, data.bytesize)
      end

      # Decompress bytes produced by compress.
      def self.decompress(data)
        tokens, original_length = deserialise_tokens(data)
        decode(tokens, original_length: original_length)
      end

    end
  end
end
