# frozen_string_literal: true

# =============================================================================
# CodingAdventures::Brotli
# =============================================================================
#
# Brotli lossless compression algorithm (2013, RFC 7932).
# Part of the CMP compression series in the coding-adventures monorepo.
#
# What Is Brotli?
# ---------------
#
# Brotli is a lossless compression algorithm developed at Google that achieves
# significantly better compression ratios than DEFLATE (CMP05), particularly
# on web content. It became the standard for HTTP `Content-Encoding: br`.
#
# Brotli improves on DEFLATE with three major innovations:
#
#   1. Context-dependent literal trees — instead of one Huffman tree for all
#      literals, Brotli assigns each literal to one of 4 context buckets based
#      on the preceding byte. Each bucket gets its own Huffman tree.
#
#   2. Insert-and-copy commands — instead of flat literal + back-reference
#      tokens, Brotli bundles insert length + copy length into a single Huffman
#      symbol (ICC code), reducing encoding overhead.
#
#   3. Larger sliding window — 65535 bytes vs DEFLATE's 4096, allowing matches
#      across much longer distances in large documents.
#
# Algorithm Overview
# ------------------
#
#   Pass 1 — LZ matching: scan input for repeated sequences, building a list of
#            insert-and-copy commands (insert some literals, then copy from
#            history).
#
#   Pass 2a — Tally frequencies for all ICC codes, distance codes, and literals
#             (per context bucket).
#
#   Pass 2b — Build canonical Huffman trees (one ICC tree, one dist tree, four
#             literal trees — one per context bucket).
#
#   Pass 2c — Encode the command stream: ICC symbol first (containing the
#             insert_length range), then the insert_length literals via
#             per-context trees, then distance code + extra bits. Trailing
#             literals (after the last copy command) are emitted directly
#             (no ICC prefix) before the end-of-data sentinel.
#
# Context Buckets
# ---------------
#
#   The context of a literal is determined by the last byte output:
#
#     Bucket 0 — last byte is space/punctuation (0x00–0x2F, 0x3A–0x40,
#                0x5B–0x60, 0x7B–0xFF) or no prior byte exists.
#     Bucket 1 — last byte is a digit ('0'–'9').
#     Bucket 2 — last byte is uppercase ('A'–'Z').
#     Bucket 3 — last byte is lowercase ('a'–'z').
#
# Bit Stream Layout
# -----------------
#
#   For each insert-and-copy command (copy_length > 0):
#     [ICC Huffman code] [insert extra bits] [copy extra bits]
#     [insert_length literals — each via per-context Huffman tree]
#     [dist Huffman code] [dist extra bits]
#
#   After all commands, trailing literals (if any) from the final flush:
#     [literal Huffman code × N]   (N derived from original_length)
#
#   Finally: [sentinel ICC 63 Huffman code]
#
# Wire Format (CMP06)
# -------------------
#
#   [4B] original_length     big-endian uint32
#   [1B] icc_entry_count     uint8 (1–64)
#   [1B] dist_entry_count    uint8 (0–32)
#   [1B] ctx0_entry_count    uint8
#   [1B] ctx1_entry_count    uint8
#   [1B] ctx2_entry_count    uint8
#   [1B] ctx3_entry_count    uint8
#   [icc_entry_count × 2B]   (symbol uint8, code_length uint8)
#   [dist_entry_count × 2B]  (symbol uint8, code_length uint8)
#   [ctx0_entry_count × 3B]  (symbol uint16 BE, code_length uint8)
#   [ctx1_entry_count × 3B]  same
#   [ctx2_entry_count × 3B]  same
#   [ctx3_entry_count × 3B]  same
#   [remaining bytes]        LSB-first packed bit stream
#
# Dependencies
# ------------
#
#   coding-adventures-huffman-tree  (DT27) — canonical Huffman tree builder
#
# =============================================================================

require "coding_adventures_huffman_tree"
require_relative "brotli/version"

module CodingAdventures
  module Brotli
    # -------------------------------------------------------------------------
    # ICC Table — Insert-and-Copy Code table (64 codes)
    # -------------------------------------------------------------------------
    #
    # Each ICC code bundles an insert-length range and a copy-length range into
    # a single Huffman symbol. Extra bits after the symbol select the exact
    # values within each range.
    #
    # Code 63 is the end-of-data sentinel (insert=0, copy=0).
    #
    # Format: [insert_base, insert_extra_bits, copy_base, copy_extra_bits]
    #
    # Insert groups:
    #   Codes  0-15: insert_base=0  (insert always 0)
    #   Codes 16-23: insert_base=1  (insert always 1)
    #   Codes 24-31: insert_base=2  (insert always 2)
    #   Codes 32-39: insert_base=3, insert_extra=1 → insert 3 or 4
    #   Codes 40-47: insert_base=5, insert_extra=2 → insert 5..8
    #   Codes 48-55: insert_base=9, insert_extra=3 → insert 9..16
    #   Codes 56-62: insert_base=17, insert_extra=4 → insert 17..32
    #   Code 63:     sentinel (insert=0, copy=0)
    #
    # Copy groups (8 copy ranges, one per column within each insert group):
    #   col 0: copy 4
    #   col 1: copy 5
    #   col 2: copy 6
    #   col 3: copy 8 or 9 (8 base, 1 extra bit)
    #   col 4: copy 10 or 11 (10 base, 1 extra bit)
    #   col 5: copy 14..17 (14 base, 2 extra bits)
    #   col 6: copy 18..21 (18 base, 2 extra bits)
    #   col 7: copy 26..33 (26 base, 3 extra bits)

    # -------------------------------------------------------------------------
    # Valid copy lengths for ins=0 ICC codes (codes 0-15)
    # -------------------------------------------------------------------------
    #
    # The ICC table has gaps in its copy ranges: 7, 12-13, 22-25, 42-49,
    # 162-193, 386-513 are not directly encodable. The LZ matcher snaps
    # match lengths to valid ranges using ICC_VALID_COPY_MAX — the largest
    # copy length <= the actual match length that is encodable with ins=0.
    #
    # This array is indexed 0..769; each entry is the largest valid copy
    # length <= that index. Built lazily from ICC codes 0-15.

    # Valid copy lengths for ICC codes 0-15 (ins=0), sorted descending.
    # Used by decompose_into_valid_copies to split large remainders.
    icc_valid = []
    [[4, 0], [5, 0], [6, 0], [8, 1], [10, 1], [14, 2], [18, 2], [26, 3],
      [34, 3], [50, 4], [66, 4], [98, 5], [130, 5], [194, 6], [258, 7], [514, 8]].each do |base, extra|
      (base..(base + (1 << extra) - 1)).each { |v| icc_valid << v }
    end
    ICC_VALID_COPIES_DESC = icc_valid.sort.reverse.freeze

    # Pre-built snap: largest valid ins=0 copy length <= L (for LZ snapping only).
    ICC_SNAP_COPY = Array.new(770) do |l|
      ICC_VALID_COPIES_DESC.find { |v| v <= l } || 0
    end.freeze

    ICC_TABLE = [
      # ins_base  ins_extra  copy_base  copy_extra
      [0, 0, 4, 0], # 0
      [0, 0, 5, 0], # 1
      [0, 0, 6, 0], # 2
      [0, 0, 8, 1], # 3
      [0, 0, 10, 1], # 4
      [0, 0, 14, 2], # 5
      [0, 0, 18, 2], # 6
      [0, 0, 26, 3], # 7
      [0, 0, 34, 3], # 8
      [0, 0, 50, 4], # 9
      [0, 0, 66, 4], # 10
      [0, 0, 98, 5], # 11
      [0, 0, 130, 5], # 12
      [0, 0, 194, 6], # 13
      [0, 0, 258, 7], # 14
      [0, 0, 514, 8], # 15
      [1, 0, 4, 0], # 16
      [1, 0, 5, 0], # 17
      [1, 0, 6, 0], # 18
      [1, 0, 8, 1], # 19
      [1, 0, 10, 1], # 20
      [1, 0, 14, 2], # 21
      [1, 0, 18, 2], # 22
      [1, 0, 26, 3], # 23
      [2, 0, 4, 0], # 24
      [2, 0, 5, 0], # 25
      [2, 0, 6, 0], # 26
      [2, 0, 8, 1], # 27
      [2, 0, 10, 1], # 28
      [2, 0, 14, 2], # 29
      [2, 0, 18, 2], # 30
      [2, 0, 26, 3], # 31
      [3, 1, 4, 0], # 32
      [3, 1, 5, 0], # 33
      [3, 1, 6, 0], # 34
      [3, 1, 8, 1], # 35
      [3, 1, 10, 1], # 36
      [3, 1, 14, 2], # 37
      [3, 1, 18, 2], # 38
      [3, 1, 26, 3], # 39
      [5, 2, 4, 0], # 40
      [5, 2, 5, 0], # 41
      [5, 2, 6, 0], # 42
      [5, 2, 8, 1], # 43
      [5, 2, 10, 1], # 44
      [5, 2, 14, 2], # 45
      [5, 2, 18, 2], # 46
      [5, 2, 26, 3], # 47
      [9, 3, 4, 0], # 48
      [9, 3, 5, 0], # 49
      [9, 3, 6, 0], # 50
      [9, 3, 8, 1], # 51
      [9, 3, 10, 1], # 52
      [9, 3, 14, 2], # 53
      [9, 3, 18, 2], # 54
      [9, 3, 26, 3], # 55
      [17, 4, 4, 0], # 56
      [17, 4, 5, 0], # 57
      [17, 4, 6, 0], # 58
      [17, 4, 8, 1], # 59
      [17, 4, 10, 1], # 60
      [17, 4, 14, 2], # 61
      [17, 4, 18, 2], # 62
      [0, 0, 0, 0] # 63 — end-of-data sentinel
    ].freeze

    # -------------------------------------------------------------------------
    # Distance code table (codes 0–31)
    # -------------------------------------------------------------------------
    #
    # Extends DEFLATE's 24-code table with 8 more codes (24–31) covering
    # offsets up to 65535 bytes. Format: [code, base_distance, extra_bits]

    DIST_TABLE = [
      [0, 1, 0], [1, 2, 0], [2, 3, 0], [3, 4, 0],
      [4, 5, 1], [5, 7, 1], [6, 9, 2], [7, 13, 2],
      [8, 17, 3], [9, 25, 3], [10, 33, 4], [11, 49, 4],
      [12, 65, 5], [13, 97, 5], [14, 129, 6], [15, 193, 6],
      [16, 257, 7], [17, 385, 7], [18, 513, 8], [19, 769, 8],
      [20, 1025, 9], [21, 1537, 9], [22, 2049, 10], [23, 3073, 10],
      [24, 4097, 11], [25, 6145, 11], [26, 8193, 12], [27, 12289, 12],
      [28, 16385, 13], [29, 24577, 13], [30, 32769, 14], [31, 49153, 14]
    ].freeze

    DIST_BASE = DIST_TABLE.to_h { |code, base, _| [code, base] }.freeze
    DIST_EXTRA = DIST_TABLE.to_h { |code, _, extra| [code, extra] }.freeze

    class << self
      # -----------------------------------------------------------------------
      # Public: compress(data) → binary string
      # -----------------------------------------------------------------------

      # Compress a binary string using Brotli (CMP06).
      #
      # Two passes:
      #   Pass 1 — LZ matching produces insert-and-copy commands.
      #   Pass 2 — Build Huffman trees, encode command stream to bit string.
      #
      # The bit stream layout (per command with copy_length > 0):
      #   [ICC code + extra bits] [insert_length literals] [dist + extra bits]
      # After all commands, trailing literals (flush) come before sentinel ICC.
      #
      # @param data [String] Input data (any encoding; converted to binary).
      # @return [String] Compressed bytes in CMP06 wire format.
      def compress(data)
        data = data.b
        original_length = data.bytesize

        # ── Special case: empty input ─────────────────────────────────────────
        #
        # Header (10 bytes) + 1 ICC entry (sentinel=63, code_length=1) +
        # 1 byte bit stream (the single "0" bit for ICC code 63, padded).
        if original_length == 0
          header = [0].pack("N") + [1, 0, 0, 0, 0, 0].pack("CCCCCC")
          icc_entry = [63, 1].pack("CC")
          bit_stream = "\x00".b
          return (header + icc_entry + bit_stream).b
        end

        # ── Pass 1: LZ matching → insert-and-copy commands ───────────────────
        raw_commands = lz_match(data)

        # ── Pass 1b: Normalize commands ───────────────────────────────────────
        #
        # Not every (insert_length, copy_length) pair has a matching ICC code.
        # The ICC table's insert groups are limited: ins=0 supports copy up to
        # 769, but ins=1 only supports copy up to 33, ins=2 up to 33, etc.
        # Large copy lengths only support insert=0.
        #
        # When a command's (ins, copy) pair has no ICC match, we SPLIT it:
        # 1. A "partial copy" command using an ICC that covers (ins, smaller_copy).
        # 2. The remaining copy as one or more ins=0 commands.
        #
        # This preserves the correct positional order: insert bytes always come
        # first (before the copy), so they appear correctly in the output.
        flush_cmd = raw_commands.pop  # last command always has copy_length=0
        flush_literals = flush_cmd[3].dup

        copy_commands = []
        raw_commands.each do |cmd|
          ins_len, copy_len, copy_dist, literals = cmd
          icc = try_find_icc(ins_len, copy_len)
          if icc
            # Perfect match: emit as-is.
            copy_commands << cmd
          else
            # No ICC covers (ins_len, copy_len). Strategy: find the best ICC
            # that covers ins_len (with any copy range) and use it for a
            # partial copy. Then emit the remainder as ins=0 copies.
            best_icc = find_icc_for_insert(ins_len)
            if best_icc
              # Use the maximum copy length that fits in best_icc's range,
              # but cap at copy_len.
              max_partial = icc_max_copy(best_icc)
              partial_copy = [max_partial, copy_len].min
              remaining = copy_len - partial_copy
              # If remaining is un-decomposable (1-3 or 7), reduce partial_copy
              # by the minimum amount to make remaining decomposable.
              if remaining > 0 && remaining < 4
                # remaining 1-3: need to reduce partial by (4 - remaining).
                partial_copy -= (4 - remaining)
                remaining = copy_len - partial_copy
              elsif remaining == 7
                # 7 cannot be decomposed: reduce partial by 1.
                partial_copy -= 1
                remaining = copy_len - partial_copy
              end
              # Decompose remaining into valid ins=0 copy lengths summing exactly.
              sub_copies = decompose_into_valid_copies(remaining)
              # First sub-command: ins_len literals + partial_copy.
              copy_commands << [ins_len, partial_copy, copy_dist, literals]
              # Remaining sub-commands: ins=0.
              sub_copies.each do |chunk|
                copy_commands << [0, chunk, copy_dist, []]
              end
            else
              # ins_len > 32: no ICC supports this insert length.
              # This should not happen if the LZ matcher produces sane output
              # (insert groups cover 0–32). Treat excess as flush.
              flush_literals = literals + flush_literals
              # Still emit the copy with ins=0.
              remaining = copy_len
              while remaining > 0
                chunk = [remaining, 769].min
                copy_commands << [0, chunk, copy_dist, []]
                remaining -= chunk
              end
            end
          end
        end

        # ── Pass 2a: Tally frequencies ────────────────────────────────────────
        #
        # For each copy command: tally ICC code + dist code + literals.
        # For flush literals: tally only per-context literal frequencies.
        # Always tally ICC 63 for the sentinel.
        lit_freq = Array.new(4) { Hash.new(0) }
        icc_freq = Hash.new(0)
        dist_freq = Hash.new(0)
        history = []

        copy_commands.each do |cmd|
          ins_len, copy_len, copy_dist, literals = cmd

          icc = find_icc(ins_len, copy_len)
          icc_freq[icc] += 1
          dc = dist_code(copy_dist)
          dist_freq[dc] += 1

          literals.each do |byte|
            ctx = literal_context(history)
            lit_freq[ctx][byte] += 1
            history << byte
          end

          start = history.size - copy_dist
          copy_len.times { |i| history << history[start + i] }
        end

        flush_literals.each do |byte|
          ctx = literal_context(history)
          lit_freq[ctx][byte] += 1
          history << byte
        end

        icc_freq[63] += 1  # end-of-data sentinel

        # ── Pass 2b: Build Huffman trees ──────────────────────────────────────
        icc_tree = CodingAdventures::HuffmanTree.build(icc_freq.to_a)
        icc_code_table = icc_tree.canonical_code_table

        dist_code_table = {}
        unless dist_freq.empty?
          d_tree = CodingAdventures::HuffmanTree.build(dist_freq.to_a)
          dist_code_table = d_tree.canonical_code_table
        end

        lit_code_tables = (0..3).map do |ctx|
          next {} if lit_freq[ctx].empty?
          t = CodingAdventures::HuffmanTree.build(lit_freq[ctx].to_a)
          t.canonical_code_table
        end

        # ── Pass 2c: Encode command stream ────────────────────────────────────
        #
        # Per-command order: ICC → insert extra bits → copy extra bits →
        #                    insert_length literals → dist → dist extra bits.
        # After all copy commands: flush literals → sentinel ICC 63.
        bits = +""
        history = []

        copy_commands.each do |cmd|
          ins_len, copy_len, copy_dist, literals = cmd

          icc = find_icc(ins_len, copy_len)
          ins_base, ins_extra_bits, copy_base, copy_extra_bits = ICC_TABLE[icc]
          ins_extra_val = ins_len - ins_base
          copy_extra_val = copy_len - copy_base

          bits << icc_code_table[icc]
          ins_extra_bits.times { |i| bits << ((ins_extra_val >> i) & 1).to_s }
          copy_extra_bits.times { |i| bits << ((copy_extra_val >> i) & 1).to_s }

          literals.each do |byte|
            ctx = literal_context(history)
            bits << lit_code_tables[ctx][byte]
            history << byte
          end

          dc = dist_code(copy_dist)
          dextra_bits = DIST_EXTRA[dc]
          dextra_val = copy_dist - DIST_BASE[dc]
          bits << dist_code_table[dc]
          dextra_bits.times { |i| bits << ((dextra_val >> i) & 1).to_s }

          start = history.size - copy_dist
          copy_len.times { |i| history << history[start + i] }
        end

        # End-of-data sentinel: ICC code 63.
        # The sentinel comes BEFORE flush literals so that the decompressor
        # loop can break on the sentinel, then decode trailing flush literals
        # using original_length as the termination condition.
        bits << icc_code_table[63]

        # Flush literals: emitted AFTER the sentinel.
        # The decompressor reads these using the trailing literals loop
        # (while output.size < original_length).
        flush_literals.each do |byte|
          ctx = literal_context(history)
          bits << lit_code_tables[ctx][byte]
          history << byte
        end

        bit_stream = pack_bits_lsb_first(bits)

        # ── Assemble wire format ──────────────────────────────────────────────
        icc_lengths = icc_code_table
          .map { |sym, code| [sym, code.length] }
          .sort_by { |sym, len| [len, sym] }

        dist_lengths = dist_code_table
          .map { |sym, code| [sym, code.length] }
          .sort_by { |sym, len| [len, sym] }

        lit_lengths = (0..3).map do |ctx|
          lit_code_tables[ctx]
            .map { |sym, code| [sym, code.length] }
            .sort_by { |sym, len| [len, sym] }
        end

        header = [original_length].pack("N") +
          [
            icc_lengths.size,
            dist_lengths.size,
            lit_lengths[0].size,
            lit_lengths[1].size,
            lit_lengths[2].size,
            lit_lengths[3].size
          ].pack("CCCCCC")

        icc_bytes = icc_lengths.map { |sym, len| [sym, len].pack("CC") }.join
        dist_bytes = dist_lengths.map { |sym, len| [sym, len].pack("CC") }.join
        lit_bytes = (0..3).map do |ctx|
          lit_lengths[ctx].map { |sym, len| [sym, len].pack("nC") }.join
        end.join

        (header + icc_bytes + dist_bytes + lit_bytes + bit_stream).b
      end

      # -----------------------------------------------------------------------
      # Public: decompress(data) → binary string
      # -----------------------------------------------------------------------

      # Decompress CMP06 wire-format data.
      #
      # Decoding order per command: ICC symbol → insert extra bits → copy extra
      # bits → insert_length literals → dist + extra bits → copy bytes.
      # After ICC sentinel 63: decode trailing literals until output.size ==
      # original_length (trailing literals come just before the sentinel in
      # the bit stream).
      #
      # @param data [String] Compressed bytes from compress().
      # @return [String] Original uncompressed data.
      def decompress(data)
        data = data.b
        return "".b if data.bytesize < 10

        # Parse header (10 bytes).
        original_length,
        icc_entry_count,
        dist_entry_count,
        ctx0_count,
        ctx1_count,
        ctx2_count,
        ctx3_count = data.unpack("NCCCCCC")

        return "".b if original_length == 0

        off = 10

        # Parse ICC code-length table (each entry: symbol uint8, len uint8).
        icc_lengths = icc_entry_count.times.map do
          sym, code_len = data[off, 2].unpack("CC")
          off += 2
          [sym, code_len]
        end

        # Parse dist code-length table.
        dist_lengths = dist_entry_count.times.map do
          sym, code_len = data[off, 2].unpack("CC")
          off += 2
          [sym, code_len]
        end

        # Parse four literal code-length tables (each entry: uint16 BE + uint8).
        lit_lengths = [ctx0_count, ctx1_count, ctx2_count, ctx3_count].map do |count|
          count.times.map do
            sym, code_len = data[off, 3].unpack("nC")
            off += 3
            [sym, code_len]
          end
        end

        # Reconstruct canonical Huffman reverse maps (bit_string → symbol).
        icc_rev_map = reconstruct_canonical_codes(icc_lengths)
        dist_rev_map = reconstruct_canonical_codes(dist_lengths)
        lit_rev_maps = lit_lengths.map { |tbl| reconstruct_canonical_codes(tbl) }

        # Unpack bit stream (remaining bytes).
        bits = unpack_bits_lsb_first(data[off..])
        bit_pos = 0

        # Read n raw bits, LSB-first. Returns integer value.
        read_bits_lsb = lambda do |n|
          return 0 if n == 0
          val = 0
          n.times { |i| val |= bits[bit_pos + i].to_i << i }
          bit_pos += n
          val
        end

        # Decode next Huffman symbol from the bit stream.
        next_huffman_symbol = lambda do |rev_map|
          acc = +""
          loop do
            acc << bits[bit_pos]
            bit_pos += 1
            sym = rev_map[acc]
            return sym if sym
          end
        end

        # Decode command stream.
        output = []
        loop do
          icc_sym = next_huffman_symbol.call(icc_rev_map)
          break if icc_sym == 63  # end-of-data sentinel

          ins_base, ins_extra_bits, copy_base, copy_extra_bits = ICC_TABLE[icc_sym]
          insert_length = ins_base + read_bits_lsb.call(ins_extra_bits)
          copy_length = copy_base + read_bits_lsb.call(copy_extra_bits)

          # Decode insert_length literals using per-context trees.
          insert_length.times do
            ctx = literal_context(output)
            byte = next_huffman_symbol.call(lit_rev_maps[ctx])
            output << byte
          end

          # Decode distance and perform copy.
          if copy_length > 0
            dc = next_huffman_symbol.call(dist_rev_map)
            dist_extra = read_bits_lsb.call(DIST_EXTRA[dc])
            copy_distance = DIST_BASE[dc] + dist_extra
            start = output.size - copy_distance
            copy_length.times { |i| output << output[start + i] }
          end
        end

        # Decode trailing literals (emitted before sentinel in the bit stream).
        # These are the bytes from the final flush command (copy_length=0).
        # The encoder wrote them just before the sentinel ICC 63.
        # We use original_length to know exactly how many remain.
        while output.size < original_length
          ctx = literal_context(output)
          byte = next_huffman_symbol.call(lit_rev_maps[ctx])
          output << byte
        end

        output.pack("C*").b
      end

      private

      # -----------------------------------------------------------------------
      # lz_match(data) → commands array
      # -----------------------------------------------------------------------
      #
      # Pass 1: scan the input for the longest match in a 65535-byte window.
      # Returns an array of commands. Each command is an array:
      #   [insert_length, copy_length, copy_distance, literals_array]
      #
      # The LAST command always has copy_length=0 (the flush command), carrying
      # any trailing literal bytes that could not be matched.
      #
      # Minimum match length is 4 (anything shorter is emitted as a literal).
      # Maximum copy length is 258 (to stay within ICC table ranges; ICC code
      # 15 covers copy up to 514+255=769, so in practice we cap at 258 for
      # simplicity — matching common practice).
      #
      # Performance: O(n²) scan per position. Sufficient for an educational
      # implementation; production Brotli uses a hash chain or suffix array.

      def lz_match(data)
        bytes = data.bytes
        n = bytes.size
        commands = []
        insert_buf = []
        pos = 0

        while pos < n
          window_start = [0, pos - 65535].max
          best_len = 0
          best_off = 0

          # Search backwards through the window for the longest match.
          # Stopping at pos-1 prevents zero-distance "copies".
          search_start = pos - 1
          while search_start >= window_start
            max_possible = [n - pos, 258].min
            len = 0
            while len < max_possible && bytes[search_start + len] == bytes[pos + len]
              len += 1
            end
            if len > best_len
              best_len = len
              best_off = pos - search_start
              break if best_len == 258  # maximum match length reached
            end
            search_start -= 1
          end

          # Snap the copy length down to the nearest value that has a valid
          # ICC code. This avoids gaps in the ICC copy ranges (7, 12-13, etc.)
          # that would otherwise cause encoding failures.
          snapped = (best_len <= 769) ? ICC_SNAP_COPY[best_len] : ICC_SNAP_COPY[769]

          if snapped >= 4
            # Flush accumulated insert bytes + record the copy command.
            commands << [insert_buf.size, snapped, best_off, insert_buf.dup]
            insert_buf = []
            pos += snapped
          else
            insert_buf << bytes[pos]
            pos += 1
          end
        end

        # Final flush command: copy_length = 0, carries remaining literals.
        commands << [insert_buf.size, 0, 0, insert_buf.dup]
        commands
      end

      # -----------------------------------------------------------------------
      # literal_context(history) → 0..3
      # -----------------------------------------------------------------------
      #
      # Determine which literal context bucket to use based on the last byte
      # in the output history.
      #
      #   Bucket 0 — space/punctuation or no prior byte (default)
      #   Bucket 1 — digit '0' (48) to '9' (57)
      #   Bucket 2 — uppercase 'A' (65) to 'Z' (90)
      #   Bucket 3 — lowercase 'a' (97) to 'z' (122)

      def literal_context(history)
        return 0 if history.empty?
        p1 = history[-1]
        return 0 unless p1  # safety: treat nil as bucket 0
        return 3 if p1.between?(97, 122)  # 'a'..'z'
        return 2 if p1.between?(65, 90)   # 'A'..'Z'
        return 1 if p1.between?(48, 57)   # '0'..'9'
        0
      end

      # -----------------------------------------------------------------------
      # decompose_into_valid_copies(total) → Array of integers
      # -----------------------------------------------------------------------
      #
      # Decompose a copy length into a sequence of valid ICC ins=0 copy lengths
      # that sum EXACTLY to total. Returns [] for total=0.
      #
      # The ICC copy ranges for ins=0 have gaps (7, 12-13, 22-25, 42-49,
      # 82-97, 162-193, 386-513). For gap values, a greedy approach picks the
      # largest valid copy c such that (total - c) is either 0 or also
      # decomposable (i.e., ≥ 4, the minimum copy). The algorithm always
      # terminates because at worst we emit copies of 4.

      def decompose_into_valid_copies(total)
        return [] if total == 0
        result = []
        remaining = total
        while remaining > 0
          # Pick the largest valid copy c such that remaining-c is 0 or is also
          # decomposable (≥ 4 and not one of the impossible values 1-3, 7).
          chunk = ICC_VALID_COPIES_DESC.find do |v|
            next false unless v <= remaining
            leftover = remaining - v
            leftover == 0 || (leftover >= 4 && leftover != 7)
          end
          # Second attempt: allow leftover=7 (will be handled recursively).
          chunk ||= ICC_VALID_COPIES_DESC.find { |v| v <= remaining }
          # Final fallback: emit minimum copy (4) and continue.
          chunk ||= 4
          result << chunk
          remaining -= chunk
          break if remaining <= 0
        end
        result
      end

      # -----------------------------------------------------------------------
      # try_find_icc(insert_length, copy_length) → Integer or nil
      # -----------------------------------------------------------------------
      #
      # Try to find an ICC code (0–62) that covers BOTH insert_length and
      # copy_length. Returns the code index or nil if none exists.

      def try_find_icc(insert_length, copy_length)
        (0..62).each do |i|
          ins_base, ins_extra, copy_base, copy_extra = ICC_TABLE[i]
          ins_max = ins_base + (1 << ins_extra) - 1
          copy_max = copy_base + (1 << copy_extra) - 1
          next unless copy_length.between?(copy_base, copy_max)
          next unless insert_length.between?(ins_base, ins_max)
          return i
        end
        nil
      end

      # -----------------------------------------------------------------------
      # find_icc(insert_length, copy_length) → 0..62
      # -----------------------------------------------------------------------
      #
      # Find an ICC code (0–62) covering insert_length and copy_length.
      # After command normalization (via try_find_icc), all commands passed
      # here are guaranteed to have a valid ICC match.
      # Falls back to insert=0 codes if needed.

      def find_icc(insert_length, copy_length)
        result = try_find_icc(insert_length, copy_length)
        return result if result
        # Fallback: find a code covering the copy length with insert=0.
        (0..15).each do |i|
          _, _, copy_base, copy_extra = ICC_TABLE[i]
          copy_max = copy_base + (1 << copy_extra) - 1
          return i if copy_length.between?(copy_base, copy_max)
        end
        0
      end

      # -----------------------------------------------------------------------
      # find_icc_for_insert(insert_length) → Integer or nil
      # -----------------------------------------------------------------------
      #
      # Find the ICC code (0–62) with the LARGEST copy range that also covers
      # insert_length. Used when splitting large copies: we first emit as much
      # copy as possible under the insert-capable ICC, then emit the remainder
      # with ins=0 codes.

      def find_icc_for_insert(insert_length)
        best = nil
        best_copy_max = -1
        (0..62).each do |i|
          ins_base, ins_extra, copy_base, copy_extra = ICC_TABLE[i]
          ins_max = ins_base + (1 << ins_extra) - 1
          next unless insert_length.between?(ins_base, ins_max)
          copy_max = copy_base + (1 << copy_extra) - 1
          if copy_max > best_copy_max
            best_copy_max = copy_max
            best = i
          end
        end
        best
      end

      # -----------------------------------------------------------------------
      # icc_max_copy(icc_code) → Integer
      # -----------------------------------------------------------------------
      #
      # Return the maximum copy length achievable with the given ICC code.

      def icc_max_copy(icc_code)
        _, _, copy_base, copy_extra = ICC_TABLE[icc_code]
        copy_base + (1 << copy_extra) - 1
      end

      # -----------------------------------------------------------------------
      # snap_copy_for_icc(length, icc_code) → Integer
      # -----------------------------------------------------------------------
      #
      # Return the largest valid copy length <= length for the given ICC code.
      # Returns the copy_base if length < copy_base.

      def snap_copy_for_icc(length, icc_code)
        _, _, copy_base, copy_extra = ICC_TABLE[icc_code]
        copy_max = copy_base + (1 << copy_extra) - 1
        return copy_base if length < copy_base
        [length, copy_max].min
      end

      # -----------------------------------------------------------------------
      # dist_code(offset) → 0..31
      # -----------------------------------------------------------------------
      #
      # Map a copy distance (1–65535) to a distance code (0–31).

      def dist_code(offset)
        DIST_TABLE.each do |code, base, extra|
          return code if offset <= base + (1 << extra) - 1
        end
        31
      end

      # -----------------------------------------------------------------------
      # Bit packing helpers (LSB-first)
      # -----------------------------------------------------------------------
      #
      # Brotli (like DEFLATE) packs bits LSB-first: bit 0 of a Huffman code
      # goes into the least-significant bit of the first output byte.
      #
      # Example: bits "101" → byte 0 = 0b_____101 = 0x05.
      #
      # The bit string representation used here is a Ruby String of '0'/'1'
      # characters, indexed by bit position (index 0 = first bit emitted =
      # least-significant bit of first byte).

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

      # Expand bytes into a string of '0'/'1' characters, LSB-first per byte.
      def unpack_bits_lsb_first(data)
        bits = +""
        data.each_byte do |byte|
          8.times { |i| bits << ((byte >> i) & 1).to_s }
        end
        bits
      end

      # -----------------------------------------------------------------------
      # reconstruct_canonical_codes(lengths) → {bit_string => symbol}
      # -----------------------------------------------------------------------
      #
      # Given a list of [symbol, code_length] pairs sorted by (length ASC,
      # symbol ASC), reconstruct the canonical Huffman bit strings and return
      # the reverse map used during decoding.
      #
      # Canonical assignment rules:
      #   - Start at code=0 for the shortest length.
      #   - Increment by 1 for each symbol at the same length.
      #   - Left-shift by (new_length - old_length) when moving longer.
      #
      # Special case: a single-symbol tree gets code "0" (length 1).

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
