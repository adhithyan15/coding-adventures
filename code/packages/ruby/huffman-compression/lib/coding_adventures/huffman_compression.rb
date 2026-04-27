# frozen_string_literal: true

# =============================================================================
# CodingAdventures::HuffmanCompression — CMP04
# =============================================================================
#
# Huffman coding is the classic entropy-based lossless compression algorithm,
# introduced by David A. Huffman in 1952.  It assigns shorter bit codes to
# more-frequent symbols and longer bit codes to rarer symbols, achieving the
# theoretically optimal code length for any symbol-frequency distribution.
#
# The analogy: imagine you send text messages all day and you could invent your
# own shorthand alphabet.  You'd write the most common words with a single
# squiggle, and the rarest words with long complicated drawings.  Huffman coding
# does exactly this, but for bytes, and provably optimally.
#
# ── How It Fits In The Compression Series ─────────────────────────────────────
#
#   CMP00 (LZ77,    1977) — Sliding-window backreferences.
#   CMP01 (LZ78,    1978) — Explicit dictionary (trie).
#   CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals.
#   CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; GIF.
#   CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE. (this module)
#   CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
#
# ── What This Module Does ─────────────────────────────────────────────────────
#
# This module uses the DT27 HuffmanTree (coding-adventures-huffman-tree gem) to:
#   1. Build a canonical Huffman code table from byte frequencies.
#   2. Encode each byte as its variable-length bit code.
#   3. Pack the bit stream into bytes LSB-first (least-significant bit first).
#   4. Emit a self-contained wire-format header so the decoder needs no
#      side-channel information — only the compressed bytes.
#
# ── CMP04 Wire Format ─────────────────────────────────────────────────────────
#
# The wire format is designed to be self-describing:
#
#   Bytes 0–3:    original_length  (big-endian uint32)
#                 How many bytes the original (uncompressed) data had.
#
#   Bytes 4–7:    symbol_count     (big-endian uint32)
#                 How many distinct byte values appeared in the input.
#                 This is N in the code-lengths table below.
#
#   Bytes 8–8+2N: code-lengths table — N entries of 2 bytes each:
#                   byte[0]: symbol value (uint8, 0–255)
#                   byte[1]: code length  (uint8, 1–16)
#                 Sorted by (code_length, symbol_value) ascending.
#                 This is the canonical form — lengths alone fully define codes.
#
#   Bytes 8+2N+:  bit stream — variable-length codes concatenated and packed
#                 LSB-first into bytes, zero-padded to the next byte boundary.
#
# LSB-first packing means bit 0 of the stream goes into bit 0 (least significant
# bit) of the first byte.  Bit 8 goes into bit 0 of the second byte, etc.
# This matches the GIF/LZW convention and makes the bit math straightforward.
#
# ── Canonical Codes ───────────────────────────────────────────────────────────
#
# Canonical Huffman codes (used by DEFLATE/zlib) have a key property: given
# ONLY the code lengths, you can reconstruct the exact code values without
# transmitting the tree structure.  The algorithm:
#
#   1. Sort symbols by (code_length, symbol_value).
#   2. Assign codes numerically, starting at 0:
#        code[0] = 0, padded to length[0] bits
#        code[i] = (code[i-1] + 1) << (length[i] - length[i-1])
#
# This is what makes the wire format compact: we store only code lengths, not
# the full Huffman tree.
#
# Example — "AAABBC" (6 bytes):
#   Frequencies: A=3, B=2, C=1
#   Huffman tree gives lengths: A→1, B→2, C→2
#   Sorted: A(1), B(2), C(2)
#   Canonical codes: A→0, B→10, C→11
#
#   Wire format header:
#     00 00 00 06  ← original_length = 6
#     00 00 00 03  ← symbol_count = 3
#     41 01        ← sym=0x41='A', len=1
#     42 02        ← sym=0x42='B', len=2
#     43 02        ← sym=0x43='C', len=2
#   Bit stream for "AAABBC":
#     A=0, A=0, A=0, B=10, B=10, C=11  →  "000101011"
#     Packed LSB-first into bytes: 0b00101000, 0b00000001 = 0x28, 0x01
#   Full wire bytes: 00000006 00000003 4101 4202 4302 2801
#
#   (Note: The spec shows A801 for the bit stream; that depends on the exact
#    canonical code assignment from the tree implementation.)
#
# =============================================================================

require "coding_adventures_huffman_tree"
require_relative "huffman_compression/version"

module CodingAdventures
  # HuffmanCompression implements CMP04 wire-format compression and decompression.
  #
  # The public interface is two class methods:
  #   HuffmanCompression.compress(data)    → binary String (CMP04 wire format)
  #   HuffmanCompression.decompress(data)  → binary String (original bytes)
  #
  # Both methods accept and return binary Strings (encoding: ASCII-8BIT / "binary").
  module HuffmanCompression
    # ── Public API ────────────────────────────────────────────────────────────

    # Compress +data+ (String) using Huffman coding and return CMP04 wire bytes.
    #
    # Steps:
    #   1. Count byte frequencies with String#bytes + Enumerable#tally.
    #   2. Build a HuffmanTree from the frequency distribution.
    #   3. Retrieve the canonical code table (symbol → bit-string).
    #   4. Concatenate bit-strings for each input byte.
    #   5. Pack the bit stream LSB-first into bytes.
    #   6. Prepend the CMP04 header (original_length, symbol_count, lengths table).
    #
    # Edge case — single distinct byte (e.g. "AAAAA"):
    #   The canonical code table returns {"A" => "0"} (1 bit per symbol).
    #   This is the conventional minimum code length for single-symbol alphabets.
    #
    # Edge case — empty input:
    #   Returns an 8-byte header with original_length=0, symbol_count=0, no bit stream.
    #
    # @param data [String] binary or text string to compress
    # @return     [String] CMP04 wire-format bytes (binary encoding)
    def self.compress(data)
      data = data.b  # force binary encoding so .bytes gives raw byte values

      # ── Step 1: Count symbol frequencies ──────────────────────────────────
      #
      # tally returns {byte_value => count} for each distinct byte.
      # E.g. "AAABBC".bytes.tally → {65=>3, 66=>2, 67=>1}
      freq = data.bytes.tally

      # Empty-input edge case: nothing to encode; emit minimal header.
      return [0, 0].pack("NN") if freq.empty?

      # ── Step 2: Build the Huffman tree ────────────────────────────────────
      #
      # HuffmanTree.build expects [[symbol, frequency], ...] pairs.
      tree = CodingAdventures::HuffmanTree.build(freq.to_a)

      # ── Step 3: Get canonical code table ──────────────────────────────────
      #
      # canonical_code_table returns {symbol_integer => bit_string} where each
      # bit_string is a '0'/'1' character string, e.g. {65=>"0", 66=>"10", 67=>"11"}.
      table = tree.canonical_code_table

      # ── Step 4: Build the code-lengths table ──────────────────────────────
      #
      # We store (symbol, code_length) pairs sorted by (length, symbol).
      # This canonical ordering lets the decompressor reconstruct the exact
      # same code table from lengths alone — no tree structure needed.
      lengths = table.map { |sym, bits| [sym, bits.length] }
        .sort_by { |sym, len| [len, sym] }

      # ── Step 5: Encode the input as a bit string ──────────────────────────
      #
      # For each byte value in the input, look up its bit-string code and
      # concatenate everything.  Example: "AAABBC" → "0" + "0" + "0" + "10"
      # + "10" + "11" = "000101011"
      bits = data.bytes.map { |b| table[b] }.join

      # ── Step 6: Pack bits LSB-first ───────────────────────────────────────
      #
      # Convert the '0'/'1' bit string into a compact binary string where
      # bits are stored least-significant-bit first within each byte.
      bit_bytes = pack_bits_lsb_first(bits)

      # ── Step 7: Assemble the wire format ──────────────────────────────────
      #
      # Header: 4-byte original_length (big-endian) + 4-byte symbol_count (big-endian)
      # Table:  N * 2 bytes (symbol byte + length byte), sorted canonical order
      # Body:   packed bit stream
      header = [data.bytesize, lengths.size].pack("NN")
      table_bytes = lengths.map { |sym, len| [sym, len].pack("CC") }.join
      (header + table_bytes + bit_bytes).b
    end

    # Decompress CMP04 wire-format +data+ and return the original bytes.
    #
    # Steps:
    #   1. Parse the header: original_length, symbol_count.
    #   2. Parse the code-lengths table: N (symbol, length) pairs.
    #   3. Reconstruct canonical codes from lengths using the standard algorithm.
    #   4. Unpack the bit stream LSB-first.
    #   5. Decode exactly original_length symbols from the bit stream.
    #
    # @param data [String] CMP04 wire-format bytes
    # @return     [String] original bytes (binary encoding)
    def self.decompress(data)
      data = data.b

      # ── Step 1: Parse header ──────────────────────────────────────────────
      #
      # Bytes 0–3: original_length (big-endian uint32)
      # Bytes 4–7: symbol_count    (big-endian uint32)
      return "".b if data.bytesize < 8

      original_length = data[0, 4].unpack1("N")
      symbol_count = data[4, 4].unpack1("N")

      # Edge case: empty original data
      return "".b if original_length.zero?

      # ── Step 2: Parse the code-lengths table ──────────────────────────────
      #
      # N entries at bytes 8+, each 2 bytes: [symbol_uint8, length_uint8]
      table_start = 8
      table_end = table_start + (symbol_count * 2)
      return "".b if data.bytesize < table_end

      lengths = symbol_count.times.map do |i|
        offset = table_start + (i * 2)
        sym = data[offset, 1].unpack1("C")
        len = data[offset + 1, 1].unpack1("C")
        [sym, len]
      end

      # ── Step 3: Reconstruct canonical codes ───────────────────────────────
      #
      # Given (symbol, length) pairs sorted by (length, symbol), we reproduce
      # the canonical code assignment without any tree structure:
      #
      #   code = 0
      #   prev_len = first length
      #   for each (symbol, length):
      #     code <<= (length - prev_len)   ← left-shift when length increases
      #     code_to_sym[format("%0Xb", code)] = symbol
      #     code += 1
      #     prev_len = length
      #
      # This is the same algorithm HuffmanTree#canonical_code_table uses in
      # reverse — proving that lengths alone define the code table.
      code = 0
      prev_len = lengths[0][1]
      code_to_sym = {}

      lengths.each do |sym, len|
        code <<= (len - prev_len) if len > prev_len
        # Store the code as a fixed-width bit string, e.g. 3-bit code 5 → "101"
        code_to_sym[code.to_s(2).rjust(len, "0")] = sym
        code += 1
        prev_len = len
      end

      # ── Step 4: Unpack the bit stream ─────────────────────────────────────
      #
      # The bit stream starts immediately after the code-lengths table.
      # Bits are stored LSB-first: bit 0 of byte 0 → first bit of stream, etc.
      bit_data = data[table_end..]
      bits = unpack_bits_lsb_first(bit_data)

      # ── Step 5: Decode symbols ────────────────────────────────────────────
      #
      # Walk the bit stream, accumulating a bit-prefix until we find it in
      # code_to_sym.  This is the "canonical code trie walk" done on a flat
      # hash — no tree needed because canonical codes have the prefix-free
      # property: no code is a prefix of another.
      #
      # We decode exactly original_length symbols (the header tells us when
      # to stop, so trailing zero-padding bits are ignored automatically).
      output = []
      prefix = ""
      max_len = lengths.map { |_, len| len }.max || 0

      bits.each_char do |bit|
        prefix += bit
        if (sym = code_to_sym[prefix])
          output << sym
          prefix = ""
          break if output.length == original_length
        end
        # Guard: prefix longer than max code length means corrupted data
        break if prefix.length > max_len
      end

      output.pack("C*")
    end

    # ── Private bit-packing helpers ───────────────────────────────────────────

    # Pack a '0'/'1' bit string into bytes, least-significant-bit first.
    #
    # LSB-first (also called "bit-reverse" order) puts bit[0] into the
    # LEAST significant position of the first output byte.  So the bit
    # string "10110001" becomes one byte where bit 0 = '1', bit 1 = '0', etc.
    #
    # Visual example (8 bits → 1 byte):
    #   bits:     "10110001"
    #   position:  76543210   (bit 0 is leftmost in the stream, LSB of byte)
    #   byte:      0b10001101  = 0x8D
    #
    # If bits.length is not a multiple of 8, the last byte is zero-padded
    # in the HIGH bits (the low bits are the real data).
    #
    # @param bits [String] string of '0' and '1' characters
    # @return     [String] packed binary string
    def self.pack_bits_lsb_first(bits)
      output = []
      buffer = 0
      bit_pos = 0

      bits.each_char do |b|
        # Set bit at position bit_pos in the buffer.
        # b.to_i converts '0'→0 or '1'→1.
        buffer |= b.to_i << bit_pos
        bit_pos += 1

        # Once we've filled 8 bits, flush to output.
        if bit_pos == 8
          output << buffer
          buffer = 0
          bit_pos = 0
        end
      end

      # Flush any remaining bits as a partial (zero-padded high) byte.
      output << buffer if bit_pos > 0
      output.pack("C*")
    end

    # Unpack bytes into a '0'/'1' bit string, least-significant-bit first.
    #
    # This is the inverse of pack_bits_lsb_first.  For each byte we extract
    # bits from position 0 (LSB) to position 7 (MSB) and concatenate them.
    #
    # Visual example:
    #   byte: 0x8D = 0b10001101
    #   bits extracted LSB→MSB: 1, 0, 1, 1, 0, 0, 0, 1 → "10110001"
    #
    # @param data [String] binary string
    # @return     [String] '0'/'1' bit string
    def self.unpack_bits_lsb_first(data)
      data.bytes.flat_map { |byte| 8.times.map { |i| (byte >> i) & 1 } }.join
    end

    private_class_method :pack_bits_lsb_first, :unpack_bits_lsb_first
  end
end
