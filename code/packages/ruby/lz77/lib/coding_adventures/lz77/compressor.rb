# frozen_string_literal: true

module CodingAdventures
  module LZ77
    # Compressor implements the LZ77 encoding and decoding algorithms.
    #
    # == LZ77 Overview
    #
    # LZ77 (Lempel & Ziv, 1977) replaces repeated byte sequences with compact
    # backreferences into a sliding window of recently seen data. This is the
    # foundation of DEFLATE, gzip, PNG, and zlib.
    #
    # == The Sliding Window Model
    #
    #   ┌─────────────────────────────────┬──────────────────┐
    #   │         SEARCH BUFFER           │ LOOKAHEAD BUFFER  │
    #   │  (already processed — the       │  (not yet seen —  │
    #   │   last window_size bytes)       │  next max_match)  │
    #   └─────────────────────────────────┴──────────────────┘
    #                                      ↑
    #                                  cursor (current position)
    #
    # At each step, the encoder finds the longest match in the search buffer.
    # If found and long enough (≥ min_match), emits a backreference token.
    # Otherwise, emits a literal token.
    #
    # == The Token: (offset, length, next_char)
    #
    # - offset:    how many bytes back the match starts (1 = just before cursor)
    # - length:    how many bytes the match covers (0 = literal)
    # - next_char: literal byte that follows the match (always emitted)
    #
    # == Overlapping Matches
    #
    # When offset < length, the match extends into bytes not yet decoded.
    # The decoder must copy byte-by-byte (not bulk copy) to handle this.
    module Compressor
      # find_longest_match scans the search buffer for the longest match.
      #
      # @param data [Array<Integer>] input bytes
      # @param cursor [Integer] current position
      # @param window_size [Integer] maximum lookback distance
      # @param max_match [Integer] maximum match length
      # @return [Array(Integer, Integer)] [best_offset, best_length], (0,0) if none
      def self.find_longest_match(data, cursor, window_size, max_match)
        best_offset = 0
        best_length = 0

        # The search buffer starts at most window_size bytes back.
        search_start = [0, cursor - window_size].max

        # The lookahead cannot extend past the end of input.
        # Reserve 1 byte for next_char — the last position we can match to.
        lookahead_end = [cursor + max_match, data.length - 1].min

        (search_start...cursor).each do |pos|
          length = 0
          # Match byte by byte. Matches may overlap (extend past cursor).
          while cursor + length < lookahead_end && data[pos + length] == data[cursor + length]
            length += 1
          end

          if length > best_length
            best_length = length
            best_offset = cursor - pos  # Distance back from cursor.
          end
        end

        [best_offset, best_length]
      end

      # encode encodes bytes into an LZ77 token stream.
      #
      # @param data [Array<Integer>] input bytes (use String#bytes to convert)
      # @param window_size [Integer] maximum offset (default 4096)
      # @param max_match [Integer] maximum match length (default 255)
      # @param min_match [Integer] minimum length for backreference (default 3)
      # @return [Array<Token>] compressed token stream
      def self.encode(data, window_size: 4096, max_match: 255, min_match: 3)
        tokens = []
        cursor = 0

        while cursor < data.length
          # Edge case: last byte has no room for next_char after a match.
          if cursor == data.length - 1
            tokens << Token.new(0, 0, data[cursor])
            cursor += 1
            next
          end

          offset, length = find_longest_match(data, cursor, window_size, max_match)

          if length >= min_match
            # Emit a backreference token.
            next_char = data[cursor + length]
            tokens << Token.new(offset, length, next_char)
            cursor += length + 1
          else
            # Emit a literal token (no match or too short).
            tokens << Token.new(0, 0, data[cursor])
            cursor += 1
          end
        end

        tokens
      end

      # decode decodes a token stream back into the original bytes.
      #
      # Processes each token: if length > 0, copies length bytes byte-by-byte
      # from the search buffer (handling overlapping matches), then appends
      # next_char.
      #
      # @param tokens [Array<Token>] the token stream
      # @param initial_buffer [Array<Integer>] optional seed for the search buffer
      # @return [Array<Integer>] reconstructed bytes
      def self.decode(tokens, initial_buffer: [])
        output = initial_buffer.dup

        tokens.each do |token|
          if token.length > 0
            # Copy length bytes from position (output.length - offset).
            start = output.length - token.offset
            # Copy byte-by-byte to handle overlapping matches (offset < length).
            token.length.times do |i|
              output << output[start + i]
            end
          end

          # Always append next_char — it advances the stream by 1.
          output << token.next_char
        end

        output
      end

      # serialise_tokens serialises a token list to a binary string.
      #
      # Format:
      #   [4 bytes: token count (big-endian uint32)]
      #   [N × 4 bytes: each token as (offset: uint16, length: uint8, next_char: uint8)]
      #
      # @param tokens [Array<Token>] token list to serialise
      # @return [String] binary string (encoding: BINARY)
      def self.serialise_tokens(tokens)
        # Pack: N (uint32 BE), then for each token: offset (uint16 BE), length (uint8), next_char (uint8)
        buf = [tokens.length].pack("N")
        tokens.each do |t|
          buf += [t.offset, t.length, t.next_char].pack("nCC")
        end
        buf
      end

      # deserialise_tokens deserialises a binary string back into tokens.
      #
      # @param data [String] binary string (output of serialise_tokens)
      # @return [Array<Token>] token list
      def self.deserialise_tokens(data)
        return [] if data.bytesize < 4

        count = data.unpack1("N")
        tokens = []

        count.times do |i|
          base = 4 + i * 4
          break if base + 4 > data.bytesize

          offset, length, next_char = data[base, 4].unpack("nCC")
          tokens << Token.new(offset, length, next_char)
        end

        tokens
      end
    end
  end
end
