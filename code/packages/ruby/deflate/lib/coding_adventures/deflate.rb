# frozen_string_literal: true

# =============================================================================
# CodingAdventures::Deflate
# =============================================================================
#
# DEFLATE lossless compression algorithm (1996, RFC 1951).
# Part of the CMP compression series in the coding-adventures monorepo.
#
# What Is DEFLATE?
# ----------------
#
# DEFLATE is the dominant general-purpose lossless compression algorithm,
# powering ZIP, gzip, PNG, and HTTP/2 HPACK header compression. It combines:
#
#   Pass 1 — LZSS tokenization (CMP02): replace repeated substrings with
#            back-references into a 4096-byte sliding window.
#
#   Pass 2 — Dual canonical Huffman coding (DT27): entropy-code the token
#            stream with two separate Huffman trees:
#              LL tree:   literals (0-255), end-of-data (256), lengths (257-284)
#              Dist tree: distance codes (0-23, for offsets 1-4096)
#
# Wire Format (CMP05)
# -------------------
#
#   [4B] original_length    big-endian uint32
#   [2B] ll_entry_count     big-endian uint16
#   [2B] dist_entry_count   big-endian uint16 (0 if no matches)
#   [ll_entry_count × 3B]   (symbol uint16 BE, code_length uint8)
#   [dist_entry_count × 3B] same format
#   [remaining bytes]       LSB-first packed bit stream
#
# Dependencies
# ------------
#
#   coding-adventures-huffman-tree  (DT27) — Huffman tree builder
#   coding-adventures-lzss          (CMP02) — LZSS tokenizer
#
# =============================================================================

require "coding_adventures_huffman_tree"
require "coding_adventures_lzss"
require_relative "deflate/version"

module CodingAdventures
  module Deflate
    # -------------------------------------------------------------------------
    # Length code table (LL symbols 257-284)
    # -------------------------------------------------------------------------
    #
    # Each length symbol covers a range of match lengths. The exact length is
    # encoded as extra_bits raw bits after the Huffman code.
    #
    # Table: [symbol, base_length, extra_bits]

    LENGTH_TABLE = [
      [257, 3, 0], [258, 4, 0], [259, 5, 0], [260, 6, 0],
      [261, 7, 0], [262, 8, 0], [263, 9, 0], [264, 10, 0],
      [265, 11, 1], [266, 13, 1], [267, 15, 1], [268, 17, 1],
      [269, 19, 2], [270, 23, 2], [271, 27, 2], [272, 31, 2],
      [273, 35, 3], [274, 43, 3], [275, 51, 3], [276, 59, 3],
      [277, 67, 4], [278, 83, 4], [279, 99, 4], [280, 115, 4],
      [281, 131, 5], [282, 163, 5], [283, 195, 5], [284, 227, 5]
    ].freeze

    LENGTH_BASE = LENGTH_TABLE.to_h { |sym, base, _| [sym, base] }.freeze
    LENGTH_EXTRA = LENGTH_TABLE.to_h { |sym, _, extra| [sym, extra] }.freeze

    # -------------------------------------------------------------------------
    # Distance code table (codes 0-23)
    # -------------------------------------------------------------------------

    DIST_TABLE = [
      [0, 1, 0], [1, 2, 0], [2, 3, 0], [3, 4, 0],
      [4, 5, 1], [5, 7, 1], [6, 9, 2], [7, 13, 2],
      [8, 17, 3], [9, 25, 3], [10, 33, 4], [11, 49, 4],
      [12, 65, 5], [13, 97, 5], [14, 129, 6], [15, 193, 6],
      [16, 257, 7], [17, 385, 7], [18, 513, 8], [19, 769, 8],
      [20, 1025, 9], [21, 1537, 9], [22, 2049, 10], [23, 3073, 10]
    ].freeze

    DIST_BASE = DIST_TABLE.to_h { |code, base, _| [code, base] }.freeze
    DIST_EXTRA = DIST_TABLE.to_h { |code, _, extra| [code, extra] }.freeze

    class << self
      # -----------------------------------------------------------------------
      # Public: compress(data) → binary string
      # -----------------------------------------------------------------------

      # Compress a binary string using DEFLATE (CMP05).
      #
      # @param data [String] The data to compress (binary encoding).
      # @return [String] Compressed bytes in CMP05 wire format.
      def compress(data)
        data = data.b  # force binary encoding
        original_length = data.bytesize

        if original_length == 0
          # Empty input: LL tree has only symbol 256 (end-of-data), code "0".
          header = [0].pack("N") + [1].pack("n") + [0].pack("n")
          ll_entry = [256, 1].pack("nC")
          bit_stream = "\x00".b
          return (header + ll_entry + bit_stream).b
        end

        # Pass 1: LZSS tokenization.
        # Compressor.encode expects a binary String and calls .bytes internally.
        tokens = CodingAdventures::LZSS::Compressor.encode(data)

        # Pass 2a: Tally frequencies.
        ll_freq = Hash.new(0)
        dist_freq = Hash.new(0)

        tokens.each do |tok|
          case tok
          when CodingAdventures::LZSS::Compressor::Literal
            ll_freq[tok.byte] += 1
          when CodingAdventures::LZSS::Compressor::Match
            sym = length_symbol(tok.length)
            ll_freq[sym] += 1
            dc = dist_code(tok.offset)
            dist_freq[dc] += 1
          end
        end
        ll_freq[256] += 1  # end-of-data marker

        # Pass 2b: Build canonical Huffman trees.
        ll_tree = CodingAdventures::HuffmanTree.build(ll_freq.to_a)
        ll_code_table = ll_tree.canonical_code_table  # {symbol => bit_string}

        dist_code_table = {}
        unless dist_freq.empty?
          dist_tree = CodingAdventures::HuffmanTree.build(dist_freq.to_a)
          dist_code_table = dist_tree.canonical_code_table
        end

        # Pass 2c: Encode token stream to bit string.
        bits = +""
        tokens.each do |tok|
          case tok
          when CodingAdventures::LZSS::Compressor::Literal
            bits << ll_code_table[tok.byte]
          when CodingAdventures::LZSS::Compressor::Match
            sym = length_symbol(tok.length)
            extra_bits_count = LENGTH_EXTRA[sym]
            extra_val = tok.length - LENGTH_BASE[sym]

            dc = dist_code(tok.offset)
            dist_extra_bits = DIST_EXTRA[dc]
            dist_extra_val = tok.offset - DIST_BASE[dc]

            bits << ll_code_table[sym]
            # Extra bits for length, LSB-first.
            extra_bits_count.times { |i| bits << ((extra_val >> i) & 1).to_s }
            bits << dist_code_table[dc]
            # Extra bits for distance, LSB-first.
            dist_extra_bits.times { |i| bits << ((dist_extra_val >> i) & 1).to_s }
          end
        end
        # End-of-data symbol.
        bits << ll_code_table[256]

        bit_stream = pack_bits_lsb_first(bits)

        # Assemble wire format.
        ll_lengths = ll_code_table
          .map { |sym, code| [sym, code.length] }
          .sort_by { |sym, len| [len, sym] }

        dist_lengths = dist_code_table
          .map { |sym, code| [sym, code.length] }
          .sort_by { |sym, len| [len, sym] }

        header = [original_length].pack("N") +
          [ll_lengths.size].pack("n") +
          [dist_lengths.size].pack("n")

        ll_bytes = ll_lengths.map { |sym, len| [sym, len].pack("nC") }.join
        dist_bytes = dist_lengths.map { |sym, len| [sym, len].pack("nC") }.join

        (header + ll_bytes + dist_bytes + bit_stream).b
      end

      # -----------------------------------------------------------------------
      # Public: decompress(data) → binary string
      # -----------------------------------------------------------------------

      # Decompress CMP05 wire-format data.
      #
      # @param data [String] Compressed bytes from compress().
      # @return [String] Original uncompressed data.
      def decompress(data)
        data = data.b
        return "".b if data.bytesize < 8

        original_length, ll_entry_count, dist_entry_count =
          data.unpack("Nnn")

        return "".b if original_length == 0

        off = 8

        # Parse LL code-length table.
        ll_lengths = ll_entry_count.times.map do
          sym, code_len = data[off, 3].unpack("nC")
          off += 3
          [sym, code_len]
        end

        # Parse dist code-length table.
        dist_lengths = dist_entry_count.times.map do
          sym, code_len = data[off, 3].unpack("nC")
          off += 3
          [sym, code_len]
        end

        # Reconstruct canonical codes (bit_string → symbol).
        ll_rev_map = reconstruct_canonical_codes(ll_lengths)
        dist_rev_map = reconstruct_canonical_codes(dist_lengths)

        # Unpack bit stream.
        bits = unpack_bits_lsb_first(data[off..])
        bit_pos = 0

        read_bits_lsb = lambda do |n|
          return 0 if n == 0
          val = 0
          n.times { |i| val |= bits[bit_pos + i].to_i << i }
          bit_pos += n
          val
        end

        next_huffman_symbol = lambda do |rev_map|
          acc = +""
          loop do
            acc << bits[bit_pos]
            bit_pos += 1
            sym = rev_map[acc]
            return sym if sym
          end
        end

        # Decode token stream.
        output = []
        loop do
          ll_sym = next_huffman_symbol.call(ll_rev_map)

          if ll_sym == 256
            break  # end-of-data
          elsif ll_sym < 256
            output << ll_sym  # literal byte
          else
            # Length code 257-284.
            extra = LENGTH_EXTRA[ll_sym]
            length_val = LENGTH_BASE[ll_sym] + read_bits_lsb.call(extra)

            dist_sym = next_huffman_symbol.call(dist_rev_map)
            dextra = DIST_EXTRA[dist_sym]
            dist_off = DIST_BASE[dist_sym] + read_bits_lsb.call(dextra)

            # Copy byte-by-byte (supports overlapping matches).
            start = output.size - dist_off
            length_val.times { |i| output << output[start + i] }
          end
        end

        output.pack("C*").b
      end

      private

      # Map a match length (3-255) to the LL alphabet symbol (257-284).
      def length_symbol(length)
        LENGTH_TABLE.each do |sym, base, extra|
          return sym if length <= base + (1 << extra) - 1
        end
        284
      end

      # Map an offset (1-4096) to a distance code (0-23).
      def dist_code(offset)
        DIST_TABLE.each do |code, base, extra|
          return code if offset <= base + (1 << extra) - 1
        end
        23
      end

      # Pack a string of '0'/'1' characters into bytes, LSB-first.
      def pack_bits_lsb_first(bits)
        output = []
        buffer = 0
        bit_pos = 0
        bits.each_char do |ch|
          buffer |= 1 << bit_pos if ch == "1"
          bit_pos += 1
          if bit_pos == 8
            output << buffer
            buffer = 0
            bit_pos = 0
          end
        end
        output << buffer if bit_pos > 0
        output.pack("C*").b
      end

      # Expand bytes into a bit string, reading each byte LSB-first.
      def unpack_bits_lsb_first(data)
        bits = +""
        data.each_byte do |byte|
          8.times { |i| bits << ((byte >> i) & 1).to_s }
        end
        bits
      end

      # Reconstruct canonical codes (bit_string → symbol) from sorted pairs.
      def reconstruct_canonical_codes(lengths)
        return {} if lengths.empty?
        return {"0" => lengths[0][0]} if lengths.size == 1

        result = {}
        code = 0
        prev_len = lengths[0][1]
        lengths.each do |sym, code_len|
          code <<= (code_len - prev_len) if code_len > prev_len
          bit_str = code.to_s(2).rjust(code_len, "0")
          result[bit_str] = sym
          code += 1
          prev_len = code_len
        end
        result
      end
    end
  end
end
