# frozen_string_literal: true

# coding_adventures/zstd.rb — ZStd (RFC 8878) lossless compression — CMP07.
#
# Zstandard was created by Yann Collet at Facebook (2015) and standardised in
# RFC 8878. It combines:
#
#   - LZ77 back-references via LZSS token generation (CMP02 dependency).
#   - FSE (Finite State Entropy) instead of Huffman, approaching Shannon entropy
#     in a single pass using Asymmetric Numeral Systems.
#   - Predefined decode tables from RFC 8878 Appendix B so short frames need
#     no per-frame table overhead.
#
# === Frame Layout (RFC 8878 §3) ===
#
#   ┌────────┬─────┬──────────────────────┬────────┬──────────────────┐
#   │ Magic  │ FHD │ Frame_Content_Size   │ Blocks │ [Checksum]       │
#   │ 4 B LE │ 1 B │ 1/2/4/8 B (LE)      │ ...    │ 4 B (optional)   │
#   └────────┴─────┴──────────────────────┴────────┴──────────────────┘
#
# Each block has a 3-byte header:
#   bit 0        = Last_Block flag
#   bits [2:1]   = Block_Type (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
#   bits [23:3]  = Block_Size
#
# === Compression Strategy ===
#
#   1. Split data into 128 KB blocks.
#   2. For each block, try in order:
#      a. RLE    — all bytes identical → 4 bytes total.
#      b. Compressed (LZSS + FSE) — if output < input.
#      c. Raw    — verbatim copy as fallback.
#
# === Series ===
#
#   CMP00 (LZ77)     — Sliding-window back-references
#   CMP01 (LZ78)     — Explicit dictionary (trie)
#   CMP02 (LZSS)     — LZ77 + flag bits  ← dependency
#   CMP03 (LZW)      — LZ78 + pre-initialised alphabet; GIF
#   CMP04 (Huffman)  — Entropy coding
#   CMP05 (DEFLATE)  — LZ77 + Huffman; ZIP/gzip/PNG/zlib
#   CMP06 (Brotli)   — DEFLATE + context modelling + static dict
#   CMP07 (ZStd)     — LZ77 + FSE; high ratio + speed  ← this file
#
# === Usage ===
#
#   require "coding_adventures_zstd"
#
#   data       = "the quick brown fox jumps over the lazy dog " * 25
#   compressed = CodingAdventures::Zstd.compress(data)
#   original   = CodingAdventures::Zstd.decompress(compressed)
#   # original == data

require_relative "zstd/version"
require "coding_adventures_lzss"

module CodingAdventures
  module Zstd
    # =========================================================================
    # Constants
    # =========================================================================

    # ZStd magic number: 0xFD2FB528 (little-endian bytes: 28 B5 2F FD).
    # Every valid ZStd frame begins with these four bytes. The value was
    # chosen to be unlikely in plaintext so decoders can quickly reject
    # non-ZStd data.
    MAGIC = 0xFD2FB528

    # Maximum block payload: 128 KiB.
    # ZStd spec allows blocks up to min(WindowSize, 128 KiB). We use this
    # fixed upper bound; larger inputs are split into multiple blocks.
    MAX_BLOCK_SIZE = 128 * 1024

    # Maximum decompressed output per frame: 256 MiB.
    # Guards against decompression bombs (tiny compressed input → huge output).
    MAX_OUTPUT = 256 * 1024 * 1024

    # =========================================================================
    # LL / ML / OF Code Tables (RFC 8878 §3.1.1.3)
    # =========================================================================
    #
    # These tables map a *code number* to a [baseline, extra_bits] pair.
    #
    # To decode a field value from a code number `c`:
    #   value = LL_CODES[c][0] + read(LL_CODES[c][1] extra bits)
    #
    # The FSE state machine tracks one code per field; extra bits come
    # directly from the bitstream after state transitions.

    # Literal Length codes: [baseline, extra_bits] for codes 0..35.
    # Codes 0–15 are identity (one code per value, 0 extra bits).
    # Codes 16+ group larger lengths with increasing ranges.
    LL_CODES = [
      # Codes 0–15: individual literal lengths 0–15 (no extra bits needed)
      [0, 0], [1, 0], [2, 0], [3, 0], [4, 0], [5, 0],
      [6, 0], [7, 0], [8, 0], [9, 0], [10, 0], [11, 0],
      [12, 0], [13, 0], [14, 0], [15, 0],
      # Codes 16–19: pairs (1 extra bit each) — values 16–23
      [16, 1], [18, 1], [20, 1], [22, 1],
      # Codes 20–21: quads (2 extra bits each) — values 24–31
      [24, 2], [28, 2],
      # Codes 22–23: octets (3 extra bits each) — values 32–47
      [32, 3], [40, 3],
      # Codes 24–25: wide ranges — 4 and 6 extra bits
      [48, 4], [64, 6],
      # Codes 26–35: very large literal runs — 7 through 16 extra bits
      [128, 7], [256, 8], [512, 9], [1024, 10], [2048, 11], [4096, 12],
      [8192, 13], [16384, 14], [32768, 15], [65536, 16]
    ].freeze

    # Match Length codes: [baseline, extra_bits] for codes 0..52.
    # Minimum match length in ZStd is 3 (not 0). Code 0 → match length 3.
    ML_CODES = [
      # Codes 0–31: individual match lengths 3–34 (no extra bits)
      [3, 0], [4, 0], [5, 0], [6, 0], [7, 0], [8, 0],
      [9, 0], [10, 0], [11, 0], [12, 0], [13, 0], [14, 0],
      [15, 0], [16, 0], [17, 0], [18, 0], [19, 0], [20, 0],
      [21, 0], [22, 0], [23, 0], [24, 0], [25, 0], [26, 0],
      [27, 0], [28, 0], [29, 0], [30, 0], [31, 0], [32, 0],
      [33, 0], [34, 0],
      # Codes 32+: grouped ranges with increasing extra bits
      [35, 1], [37, 1], [39, 1], [41, 1],
      [43, 2], [47, 2],
      [51, 3], [59, 3],
      [67, 4], [83, 4],
      [99, 5], [131, 7],
      [259, 8], [515, 9], [1027, 10], [2051, 11],
      [4099, 12], [8195, 13], [16387, 14], [32771, 15], [65539, 16]
    ].freeze

    # =========================================================================
    # FSE Predefined Distributions (RFC 8878 Appendix B)
    # =========================================================================
    #
    # "Predefined_Mode" means no per-frame table description is transmitted.
    # The decoder reconstructs the same decode table from these fixed
    # normalised probability distributions.
    #
    # Entries of -1 are "probability 1/table_size" (extremely rare symbols).
    # These get exactly one slot in the decode table.

    # Normalised distribution for Literal Length FSE.
    # Table accuracy log = 6 → table has 2^6 = 64 slots.
    LL_NORM = [4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1,
      2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1,
      -1, -1, -1, -1].freeze
    LL_ACC_LOG = 6

    # Normalised distribution for Match Length FSE.
    # Table accuracy log = 6 → 64 slots.
    ML_NORM = [1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1,
      1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
      1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1,
      -1, -1, -1, -1, -1].freeze
    ML_ACC_LOG = 6

    # Normalised distribution for Offset FSE.
    # Table accuracy log = 5 → 32 slots.
    OF_NORM = [1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1,
      1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1].freeze
    OF_ACC_LOG = 5

    # =========================================================================
    # RevBitWriter — Reverse Bit Writer
    # =========================================================================
    #
    # ZStd's sequence bitstream is written *backwards* relative to data flow:
    # the encoder writes bits that the decoder will read LAST, FIRST.
    # This lets the decoder do a single forward pass while decoding in order.
    #
    # Byte layout: [byte0, byte1, ..., byteN] where byteN is the last byte
    # written and contains a *sentinel bit* marking the end of meaningful data.
    #
    # Bit layout within each byte: LSB = earliest bit written within that byte.
    #
    # Example — write 4 bits 0b1011 then flush:
    #   reg = 0b1011, bits = 4
    #   flush: sentinel at bit 4 → last byte = 0b00011011 = 0x1B
    #   buf = [0x1B]
    # The decoder finds MSB (bit 4 = sentinel), then reads bits 3..0 = 0b1011.

    class RevBitWriter
      def initialize
        @buf = []  # output bytes (Array of Integer 0..255)
        @reg = 0   # accumulation register; bits fill from LSB
        @bits = 0  # how many valid bits are currently in @reg
      end

      # Add the low-order +nb+ bits of +val+ to the backward bitstream.
      #
      # Ruby integers are arbitrary precision so no overflow is possible.
      # The mask `(1 << nb) - 1` truncates to exactly nb bits.
      def add_bits(val, nb)
        return if nb == 0
        mask = (1 << nb) - 1
        @reg |= (val & mask) << @bits
        @bits += nb
        # Drain complete bytes from the LSB of the register.
        while @bits >= 8
          @buf << (@reg & 0xFF)
          @reg >>= 8
          @bits -= 8
        end
      end

      # Flush any remaining partial byte plus the sentinel bit, then reset.
      #
      # The sentinel is a 1-bit placed just above the topmost data bit. The
      # decoder uses it to find where the data ends inside the last byte.
      # Example: if @bits == 3 and @reg = 0b101, sentinel = 1 << 3 = 0b1000,
      # so last byte = 0b1000 | 0b101 = 0b00001101 = 0x0D.
      def flush
        sentinel = 1 << @bits   # one bit above all remaining data bits
        @buf << ((@reg & 0xFF) | sentinel)
        @reg = 0
        @bits = 0
      end

      # Return the accumulated bytes as a binary String (ASCII-8BIT).
      def finish
        @buf.pack("C*")
      end
    end

    # =========================================================================
    # RevBitReader — Reverse Bit Reader
    # =========================================================================
    #
    # Mirrors RevBitWriter. Reads bits from the END of the byte buffer going
    # backwards toward byte 0. The LAST-written bits are read FIRST.
    #
    # Register layout: valid bits are LEFT-ALIGNED (packed into the MSB side
    # of a simulated 64-bit register). read_bits(n) extracts the top n bits
    # and shifts the register left by n.
    #
    # Why left-aligned? The writer accumulates bits LSB-first. Within each
    # flushed byte, bit-0 = earliest written, bit-7 = latest. To read the
    # LATEST bits first (highest-position in each byte), we invert by
    # packing into the top of the register.
    #
    # Ruby integer arithmetic: we keep the register in [0, 2^64) range by
    # masking with 0xFFFFFFFFFFFFFFFF after left-shifts.

    class RevBitReader
      # @param data [String] binary-encoded byte string (ASCII-8BIT)
      def initialize(data)
        bytes = data.is_a?(String) ? data.bytes : data
        raise "empty bitstream" if bytes.empty?

        last = bytes.last
        raise "bitstream last byte is zero (no sentinel)" if last == 0

        # Find the sentinel: the highest set bit in the last byte.
        # bit_length returns floor(log2(n)) + 1 for n > 0, so
        # sentinel_pos = last.bit_length - 1  (0-indexed bit position).
        sentinel_pos = last.bit_length - 1
        valid_bits = sentinel_pos  # bits below the sentinel carry data

        # Place valid data bits of the sentinel byte at the TOP of @reg,
        # so that read_bits(n) extracts the earliest-written bits first.
        #
        # Example: last = 0b00011110, sentinel at bit 4, valid_bits = 4
        #   data bits = last & 0b1111 = 0b1110
        #   After shift: @reg bit 63 = 1, bit 62 = 1, bit 61 = 1, bit 60 = 0
        mask = (valid_bits > 0) ? (1 << valid_bits) - 1 : 0
        @reg = (valid_bits > 0) ? ((last & mask) << (64 - valid_bits)) : 0
        @bits = valid_bits
        @pos = bytes.size - 1  # sentinel byte already consumed; load below
        @bytes = bytes
        reload
      end

      # Load more bytes into the register from the backward stream.
      #
      # Each new byte is placed just BELOW the currently loaded bits.
      # In our left-aligned scheme, that means at bit position 64 - @bits - 8.
      def reload
        while @bits <= 56 && @pos > 0
          @pos -= 1
          shift = 64 - @bits - 8
          @reg |= @bytes[@pos] << shift
          @bits += 8
        end
        # Keep @reg within 64-bit range to match the Rust u64 behaviour.
        @reg &= 0xFFFFFFFFFFFFFFFF
      end

      # Read the next +nb+ bits from the top of the register.
      #
      # Returns the integer value of those bits (MSB of register first).
      # Returns 0 if nb == 0 (no-op guard for 0-extra-bit fields).
      def read_bits(nb)
        return 0 if nb == 0
        val = @reg >> (64 - nb)
        # Left-shift consumes the top nb bits; mask keeps in 64-bit range.
        @reg = (nb == 64) ? 0 : (@reg << nb) & 0xFFFFFFFFFFFFFFFF
        @bits = [@bits - nb, 0].max
        reload if @bits < 24
        val
      end
    end

    # =========================================================================
    # FSE (Finite State Entropy) Table Construction
    # =========================================================================
    #
    # FSE is an Asymmetric Numeral System (ANS) codec. The key insight:
    #
    #   - The "state" is an integer in [sz, 2*sz) where sz = 1 << acc_log.
    #   - Each symbol is associated with `count` slots in a decode table of
    #     size sz.  Symbols with higher probability get more slots.
    #   - Decoding: look up state in the table → get (sym, nb, base).
    #     New state = base + read(nb bits from stream).
    #   - Encoding (reverse): given symbol, compute nb bits to flush, flush
    #     them, then look up new state in the encode table.
    #
    # This gives near-Shannon-entropy coding in a single pass.

    # Build an FSE decode table from a normalised probability distribution.
    #
    # @param norm     [Array<Integer>] normalised counts; -1 = prob 1/sz
    # @param acc_log  [Integer]        table accuracy log (sz = 1 << acc_log)
    # @return         [Array<Hash>]    array of {sym:, nb:, base:} hashes
    #
    # Algorithm:
    #   Phase 1: symbols with prob -1 fill the HIGH end of the table (one slot
    #            each, spilling in from the top).
    #   Phase 2: remaining symbols fill the LOW end using a step function that
    #            distributes them evenly across the available slots.
    #   Phase 3: assign nb (bits to read for next state) and base to each slot.
    #
    # The step function step = (sz >> 1) + (sz >> 3) + 3 is co-prime to sz
    # when sz is a power of two, so it visits every slot exactly once.
    def self.build_decode_table(norm, acc_log)
      sz = 1 << acc_log
      step = (sz >> 1) + (sz >> 3) + 3
      tbl = Array.new(sz) { {sym: 0, nb: 0, base: 0} }
      sym_next = Array.new(norm.size, 0)

      # Phase 1: prob -1 symbols at the top (high indices, one slot each).
      # Their sym_next starts at 1 (they appear exactly once in the table).
      high = sz - 1
      norm.each_with_index do |c, s|
        if c == -1
          tbl[high] = {sym: s, nb: 0, base: 0}
          high -= 1 if high > 0
          sym_next[s] = 1
        end
      end

      # Phase 2: spread remaining symbols into indices 0..high.
      # Two passes: first symbols with count > 1, then count == 1.
      # This matches the reference implementation's deterministic ordering.
      pos = 0
      2.times do |pass|
        norm.each_with_index do |c, s|
          next if c <= 0
          cnt = c
          next if (pass == 0) != (cnt > 1)
          sym_next[s] = cnt
          cnt.times do
            tbl[pos][:sym] = s
            pos = (pos + step) & (sz - 1)
            pos = (pos + step) & (sz - 1) while pos > high
          end
        end
      end

      # Phase 3: assign nb and base to each slot.
      #
      # For symbol s with `count` occurrences, the j-th occurrence (in index
      # order) has:
      #   ns = count + j   (j starts at 0, so ns = count on first occurrence)
      #   nb = acc_log - floor(log2(ns))   (= acc_log - (ns.bit_length - 1))
      #   base = ns * (1 << nb) - sz
      #
      # Invariant: base + read(nb bits) ∈ [0, sz), so it's a valid next state.
      sn = sym_next.dup
      sz.times do |i|
        s = tbl[i][:sym]
        ns = sn[s]
        sn[s] += 1
        # nb = acc_log - floor(log2(ns)); for ns > 0, floor(log2(ns)) = ns.bit_length - 1
        nb = acc_log - (ns.bit_length - 1)
        base = (ns << nb) - sz
        tbl[i][:nb] = nb
        tbl[i][:base] = base
      end

      tbl
    end

    # Build FSE encode tables from a normalised distribution.
    #
    # Returns:
    #   ee  — Array of {delta_nb:, delta_fs:} encode entries (one per symbol)
    #   st  — Array of Integer encode state table (slot → output state)
    #
    # The encode/decode symmetry:
    #   The decoder assigns (sym, nb, base) to each table cell in INDEX order.
    #   For symbol s, the j-th cell (ascending index) has ns = count[s] + j.
    #   The encoder uses cumulative slot indexing: slot = cumul[s] + j maps
    #   to the j-th cell for symbol s. After encoding, new state = cell_index + sz.
    #
    # Encode step for symbol s, current state E ∈ [sz, 2*sz):
    #   nb_out = (E + delta_nb) >> 16
    #   emit low nb_out bits of E
    #   new_E = st[(E >> nb_out) + delta_fs]
    def self.build_encode_tables(norm, acc_log)
      sz = 1 << acc_log

      # Step 1: cumulative sums (prob -1 counts as 1).
      cumul = Array.new(norm.size, 0)
      total = 0
      norm.each_with_index do |c, s|
        cumul[s] = total
        cnt = (c == -1) ? 1 : [c, 0].max
        total += cnt
      end

      # Step 2: spread table (same algorithm as build_decode_table phases 1+2).
      step = (sz >> 1) + (sz >> 3) + 3
      spread = Array.new(sz, 0)
      idx_high = sz - 1

      norm.each_with_index do |c, s|
        if c == -1
          spread[idx_high] = s
          idx_high -= 1 if idx_high > 0
        end
      end
      idx_limit = idx_high

      pos = 0
      2.times do |pass|
        norm.each_with_index do |c, s|
          next if c <= 0
          cnt = c
          next if (pass == 0) != (cnt > 1)
          cnt.times do
            spread[pos] = s
            pos = (pos + step) & (sz - 1)
            pos = (pos + step) & (sz - 1) while pos > idx_limit
          end
        end
      end

      # Step 3: build state table by iterating spread in INDEX order.
      # For table index i, symbol s = spread[i], j-th occurrence of s:
      #   slot = cumul[s] + j
      #   st[slot] = i + sz   (encoder output state)
      sym_occ = Array.new(norm.size, 0)
      st = Array.new(sz, 0)
      sz.times do |i|
        s = spread[i]
        j = sym_occ[s]
        sym_occ[s] += 1
        slot = cumul[s] + j
        st[slot] = i + sz
      end

      # Step 4: build FseEe entries.
      # For symbol s with count c and max_bits_out mbo:
      #   delta_nb = (mbo << 16) - (c << mbo)
      #   delta_fs = cumul[s] - c
      ee = norm.each_with_index.map do |c, s|
        cnt = (c == -1) ? 1 : [c, 0].max
        next {delta_nb: 0, delta_fs: 0} if cnt == 0
        # mbo = acc_log - floor(log2(cnt)); special-case cnt == 1
        mbo = (cnt == 1) ? acc_log : acc_log - (cnt.bit_length - 1)
        delta_nb = (mbo << 16) - (cnt << mbo)
        delta_fs = cumul[s] - cnt
        {delta_nb: delta_nb, delta_fs: delta_fs}
      end

      [ee, st]
    end

    # =========================================================================
    # Helper: map a value to its LL/ML code number
    # =========================================================================

    # Map a literal length value to its LL code (0..35).
    #
    # Scan LL_CODES in order; the last entry whose baseline ≤ ll is the code.
    # Simple linear scan is fine: only 36 entries.
    def self.ll_to_code(ll)
      code = 0
      LL_CODES.each_with_index do |(base, _), i|
        if base <= ll
          code = i
        else
          break
        end
      end
      code
    end

    # Map a match length value to its ML code (0..52).
    def self.ml_to_code(ml)
      code = 0
      ML_CODES.each_with_index do |(base, _), i|
        if base <= ml
          code = i
        else
          break
        end
      end
      code
    end

    # =========================================================================
    # FSE encode/decode helpers
    # =========================================================================

    # Encode one symbol into the backward bitstream, updating the FSE state.
    #
    # The FSE encoder state lives in [sz, 2*sz). To emit symbol sym:
    #   1. nb_out = (state + delta_nb) >> 16    — how many bits to flush
    #   2. Write the low nb_out bits of state.
    #   3. new_state = st[(state >> nb_out) + delta_fs]
    #
    # After all symbols are encoded, the final state (minus sz) is written as
    # acc_log bits so the decoder can initialise to the same position.
    def self.fse_encode_sym(state, sym, ee, st)
      # This method returns [new_state, nb_out, bits_to_emit].
      # The caller writes bits_to_emit (nb_out bits) to the bitstream.
      e = ee[sym]
      nb_out = (state + e[:delta_nb]) >> 16
      bits_to_emit = state & ((1 << nb_out) - 1)  # low nb_out bits
      slot = (state >> nb_out) + e[:delta_fs]
      new_state = st[slot]
      [new_state, nb_out, bits_to_emit]
    end

    # Decode one symbol from the backward bitstream, updating the FSE state.
    #
    # Look up de[state] → {sym, nb, base}.
    # New state = base + read(nb bits from br).
    # Returns the decoded symbol.
    def self.fse_decode_sym(state, de, br)
      e = de[state]
      sym = e[:sym]
      next_state = e[:base] + br.read_bits(e[:nb])
      [sym, next_state]
    end

    # =========================================================================
    # Sequence struct
    # =========================================================================

    # One ZStd sequence: literal_length, match_length, match_offset.
    #
    # Semantics:
    #   1. Emit ll literal bytes from the literals buffer.
    #   2. Copy ml bytes starting offset positions back in the output.
    #   3. After all sequences, emit remaining literals.
    Seq = Struct.new(:ll, :ml, :off)

    # =========================================================================
    # Token → Sequence conversion
    # =========================================================================

    # Convert LZSS tokens into (flat literals buffer, ZStd sequence array).
    #
    # LZSS produces Literal(byte) and Match{offset, length} tokens.
    # ZStd groups all consecutive literals before each match into one Seq.
    # Trailing literals (after the last match) go into lits without a Seq.
    def self.tokens_to_seqs(tokens)
      lits = []
      seqs = []
      lit_run = 0

      tokens.each do |tok|
        case tok
        when CodingAdventures::LZSS::Literal
          lits << tok.byte
          lit_run += 1
        when CodingAdventures::LZSS::Match
          seqs << Seq.new(lit_run, tok.length, tok.offset)
          lit_run = 0
        end
      end
      # Trailing literals: no sequence for them; they go at the end of lits.
      [lits, seqs]
    end

    # =========================================================================
    # Literals section encoding / decoding
    # =========================================================================
    #
    # ZStd literals can be Huffman-coded or Raw. We use Raw_Literals (type=0):
    # no Huffman table, bytes stored verbatim.
    #
    # Raw_Literals header (RFC 8878 §3.1.1.2.1):
    #   bits [1:0] = Literals_Block_Type = 0b00 (Raw)
    #   bits [3:2] = Size_Format:
    #     0b00 or 0b10 → 1-byte header: size = b0[7:3] (5 bits, 0..31)
    #     0b01          → 2-byte LE: size in bits [11:4] (12 bits, 0..4095)
    #     0b11          → 3-byte LE: size in bits [19:4] (16 bits, 0..65535)

    # Encode a literals array into the Raw_Literals wire format.
    # Returns a binary String (header + verbatim bytes).
    def self.encode_literals_section(lits)
      n = lits.size
      header = if n <= 31
        # 1-byte header: size_format = 0b00, type = 0b00 → (n << 3) | 0b000
        [n << 3].pack("C")
      elsif n <= 4095
        # 2-byte header: size_format = 0b01, type = 0b00 → low nibble = 0b0100
        hdr = (n << 4) | 0b0100
        [hdr & 0xFF, (hdr >> 8) & 0xFF].pack("CC")
      else
        # 3-byte header: size_format = 0b11, type = 0b00 → low nibble = 0b1100
        hdr = (n << 4) | 0b1100
        [hdr & 0xFF, (hdr >> 8) & 0xFF, (hdr >> 16) & 0xFF].pack("CCC")
      end
      header + lits.pack("C*")
    end

    # Decode a Raw_Literals section, returning [literals_array, bytes_consumed].
    def self.decode_literals_section(data)
      raise "empty literals section" if data.empty?

      b0 = data.getbyte(0)
      ltype = b0 & 0b11  # bottom 2 bits = Literals_Block_Type

      # Only Raw_Literals (type=0) is produced by our encoder. Huffman types
      # (2 or 3) may appear in frames from other encoders but are not supported.
      raise "unsupported literals type #{ltype} (only Raw=0 supported)" if ltype != 0

      size_format = (b0 >> 2) & 0b11

      n, header_bytes = case size_format
      when 0, 2
        # 1-byte header: n in bits [7:3] (5 bits)
        [(b0 >> 3), 1]
      when 1
        # 2-byte LE header: n in bits [11:4]
        raise "truncated literals header (2-byte)" if data.bytesize < 2
        n = (b0 >> 4) | (data.getbyte(1) << 4)
        [n, 2]
      when 3
        # 3-byte LE header: n in bits [19:4]
        raise "truncated literals header (3-byte)" if data.bytesize < 3
        n = (b0 >> 4) | (data.getbyte(1) << 4) | (data.getbyte(2) << 12)
        [n, 3]
      end

      finish = header_bytes + n
      raise "literals data truncated: need #{finish}, have #{data.bytesize}" if finish > data.bytesize

      lits = data.byteslice(header_bytes, n).bytes
      [lits, finish]
    end

    # =========================================================================
    # Sequences section encoding / decoding
    # =========================================================================
    #
    # Layout:
    #   [sequence_count: 1-3 bytes]
    #   [symbol_compression_modes: 1 byte]  (0x00 = all Predefined)
    #   [FSE bitstream: variable]
    #
    # Symbol_Compression_Modes byte:
    #   bits [7:6] = LL mode
    #   bits [5:4] = OF mode
    #   bits [3:2] = ML mode
    #   bits [1:0] = reserved (0)
    # Mode 0 = Predefined, 1 = RLE, 2 = FSE_Compressed, 3 = Repeat.
    # We always write 0x00 (all Predefined).
    #
    # FSE bitstream — backward bit stream:
    #   Sequences are encoded in REVERSE ORDER (last sequence first).
    #   Per sequence (in encode order for backward stream):
    #     OF extra bits, ML extra bits, LL extra bits
    #     FSE symbol for OF, ML, LL  (written reversed so decoder reads LL first)
    #   After all sequences:
    #     (state_of - sz_of) as OF_ACC_LOG bits
    #     (state_ml - sz_ml) as ML_ACC_LOG bits
    #     (state_ll - sz_ll) as LL_ACC_LOG bits
    #     sentinel flush
    #
    # Decoder mirrors:
    #   1. Read LL_ACC_LOG bits → initial state_ll
    #   2. Read ML_ACC_LOG bits → initial state_ml
    #   3. Read OF_ACC_LOG bits → initial state_of
    #   4. Per sequence: decode LL, OF, ML symbols; read extra bits.
    #   5. Apply sequence to output.

    # Encode sequence count as 1, 2, or 3 bytes (RFC 8878 §3.1.1.3.1).
    #
    # The encoding is:
    #   0..127:       1 byte = count
    #   128..0x7EFF:  2 bytes: byte[0] = 0x80 | (count >> 8),
    #                          byte[1] = count & 0xFF
    #                 Decoder detects 2-byte by byte[0] ∈ [128, 254].
    #   0x7F00+:      3 bytes: byte[0] = 0xFF, bytes[1..2] = (count-0x7F00) LE
    #
    # Returns a binary (ASCII-8BIT) String.
    def self.encode_seq_count(count)
      if count < 128
        [count].pack("C")
      elsif count < 0x7F00
        # 2-byte: byte[0] has the high bit set, carries bits [14:8],
        # byte[1] carries bits [7:0].
        hi = 0x80 | (count >> 8)   # bits 14..8 in low 7, high bit = 1
        lo = count & 0xFF
        [hi, lo].pack("CC")
      else
        # 3-byte: first byte 0xFF, next 2 bytes = (count - 0x7F00) as LE u16
        r = count - 0x7F00
        [0xFF, r & 0xFF, (r >> 8) & 0xFF].pack("CCC")
      end
    end

    # Decode sequence count, returning [count, bytes_consumed].
    def self.decode_seq_count(data)
      raise "empty sequence count" if data.empty?
      b0 = data.getbyte(0)
      if b0 < 128
        # 1-byte: count is directly b0
        [b0, 1]
      elsif b0 < 0xFF
        # 2-byte: byte[0] = 0x80 | high bits, byte[1] = low bits
        raise "truncated sequence count" if data.bytesize < 2
        b1 = data.getbyte(1)
        count = ((b0 & 0x7F) << 8) | b1
        [count, 2]
      else
        # 3-byte: byte[0] = 0xFF, bytes[1..2] = (count - 0x7F00) LE
        raise "truncated sequence count (3-byte)" if data.bytesize < 3
        count = 0x7F00 + data.getbyte(1) + (data.getbyte(2) << 8)
        [count, 3]
      end
    end

    # Encode the sequences section (sequence count + modes byte + FSE bitstream).
    def self.encode_sequences_section(seqs)
      ee_ll, st_ll = build_encode_tables(LL_NORM, LL_ACC_LOG)
      ee_ml, st_ml = build_encode_tables(ML_NORM, ML_ACC_LOG)
      ee_of, st_of = build_encode_tables(OF_NORM, OF_ACC_LOG)

      sz_ll = 1 << LL_ACC_LOG
      sz_ml = 1 << ML_ACC_LOG
      sz_of = 1 << OF_ACC_LOG

      # FSE states start at table_size.
      # The state range [sz, 2*sz) maps to encode slot range [0, sz).
      state_ll = sz_ll
      state_ml = sz_ml
      state_of = sz_of

      bw = RevBitWriter.new

      # Encode sequences in REVERSE order (so the decoder reading forward
      # will encounter them in the original order).
      seqs.reverse_each do |seq|
        ll_code = ll_to_code(seq.ll)
        ml_code = ml_to_code(seq.ml)

        # Offset encoding: raw = offset + 3 (RFC 8878 §3.1.1.3.2.1).
        # of_code = floor(log2(raw)); of_extra = raw - (1 << of_code).
        raw_off = seq.off + 3
        of_code = (raw_off <= 1) ? 0 : (raw_off.bit_length - 1)
        of_extra = raw_off - (1 << of_code)

        # Write extra bits (OF, then ML, then LL) for this sequence.
        # In the backward stream, these come BEFORE the FSE state bits
        # so the decoder reads FSE first, then extra bits.
        bw.add_bits(of_extra, of_code)
        ml_extra = seq.ml - ML_CODES[ml_code][0]
        bw.add_bits(ml_extra, ML_CODES[ml_code][1])
        ll_extra = seq.ll - LL_CODES[ll_code][0]
        bw.add_bits(ll_extra, LL_CODES[ll_code][1])

        # FSE encode symbols in REVERSE decode order.
        # Decode order: LL, OF, ML.
        # Encode order (reversed): ML, OF, LL.
        # (LL is written last = at the TOP of backward stream = read first.)
        state_ml, nb, bits = fse_encode_sym(state_ml, ml_code, ee_ml, st_ml)
        bw.add_bits(bits, nb)
        state_of, nb, bits = fse_encode_sym(state_of, of_code, ee_of, st_of)
        bw.add_bits(bits, nb)
        state_ll, nb, bits = fse_encode_sym(state_ll, ll_code, ee_ll, st_ll)
        bw.add_bits(bits, nb)
      end

      # Flush final states (low acc_log bits of state - sz).
      # Decoder reads these first to initialise its FSE states.
      bw.add_bits(state_of - sz_of, OF_ACC_LOG)
      bw.add_bits(state_ml - sz_ml, ML_ACC_LOG)
      bw.add_bits(state_ll - sz_ll, LL_ACC_LOG)
      bw.flush

      bw.finish
    end

    # =========================================================================
    # Block-level compress / decompress
    # =========================================================================

    # Attempt to compress one block using LZSS + FSE.
    # Returns the compressed binary String, or nil if it's not beneficial.
    def self.compress_block(block)
      # LZSS with a 32 KB window (larger than the 4 KB LZSS default) to improve
      # the match ratio. Max match 255, min match 3.
      tokens = CodingAdventures::LZSS.encode(
        block.b,
        window_size: 32768,
        max_match: 255,
        min_match: 3
      )

      lits, seqs = tokens_to_seqs(tokens)

      # If no sequences were found, LZ77 found nothing to back-reference.
      # A compressed block still has overhead, so fall back to raw.
      return nil if seqs.empty?

      out = "".b
      out << encode_literals_section(lits)
      out << encode_seq_count(seqs.size)
      out << "\x00".b  # Symbol_Compression_Modes = all Predefined (0)
      out << encode_sequences_section(seqs)

      (out.bytesize < block.bytesize) ? out : nil
    end

    # Decompress one ZStd Compressed block.
    #
    # Reads the literals section, sequences section, applies sequences to
    # rebuild the original data appended to +out+.
    #
    # @param data [String] binary block payload (after the 3-byte block header)
    # @param out  [Array<Integer>] accumulating output bytes (modified in place)
    def self.decompress_block(data, out)
      # ── Literals ─────────────────────────────────────────────────────────
      lits, lit_consumed = decode_literals_section(data)
      pos = lit_consumed

      # ── Sequence count ────────────────────────────────────────────────────
      if pos >= data.bytesize
        # Block contains only literals, no sequences.
        out.concat(lits)
        return
      end

      n_seqs, sc_bytes = decode_seq_count(data.byteslice(pos..))
      pos += sc_bytes

      if n_seqs == 0
        out.concat(lits)
        return
      end

      # ── Symbol compression modes ──────────────────────────────────────────
      raise "missing symbol compression modes byte" if pos >= data.bytesize
      modes_byte = data.getbyte(pos)
      pos += 1

      ll_mode = (modes_byte >> 6) & 3
      of_mode = (modes_byte >> 4) & 3
      ml_mode = (modes_byte >> 2) & 3

      unless ll_mode == 0 && of_mode == 0 && ml_mode == 0
        raise "unsupported FSE modes: LL=#{ll_mode} OF=#{of_mode} ML=#{ml_mode} (only Predefined=0 supported)"
      end

      # ── FSE bitstream ─────────────────────────────────────────────────────
      bitstream = data.byteslice(pos..)
      raise "empty FSE bitstream" if bitstream.nil? || bitstream.empty?
      br = RevBitReader.new(bitstream)

      dt_ll = build_decode_table(LL_NORM, LL_ACC_LOG)
      dt_ml = build_decode_table(ML_NORM, ML_ACC_LOG)
      dt_of = build_decode_table(OF_NORM, OF_ACC_LOG)

      # Initialise FSE states from bitstream (encoder wrote them last → read first).
      state_ll = br.read_bits(LL_ACC_LOG)
      state_ml = br.read_bits(ML_ACC_LOG)
      state_of = br.read_bits(OF_ACC_LOG)

      # Validate that each initial state is a legal index into its decode table.
      # An out-of-bounds state would cause a silent array read at an arbitrary
      # position, producing wrong output or an exception far from the cause.
      raise "invalid initial state_ll #{state_ll}" if state_ll >= dt_ll.size
      raise "invalid initial state_ml #{state_ml}" if state_ml >= dt_ml.size
      raise "invalid initial state_of #{state_of}" if state_of >= dt_of.size

      lit_pos = 0

      n_seqs.times do
        # Decode symbols (state transitions) — LL, then OF, then ML.
        ll_code, state_ll = fse_decode_sym(state_ll, dt_ll, br)
        of_code, state_of = fse_decode_sym(state_of, dt_of, br)
        ml_code, state_ml = fse_decode_sym(state_ml, dt_ml, br)

        raise "invalid LL code #{ll_code}" if ll_code >= LL_CODES.size
        raise "invalid ML code #{ml_code}" if ml_code >= ML_CODES.size

        ll_base, ll_extra_bits = LL_CODES[ll_code]
        ml_base, ml_extra_bits = ML_CODES[ml_code]

        ll = ll_base + br.read_bits(ll_extra_bits)
        ml = ml_base + br.read_bits(ml_extra_bits)

        # Offset: raw = (1 << of_code) | extra_bits; offset = raw - 3.
        of_extra = br.read_bits(of_code)
        of_raw = (1 << of_code) | of_extra
        offset = of_raw - 3
        raise "decoded offset underflow: of_raw=#{of_raw}" if offset < 0

        # Emit ll literal bytes from the literals buffer.
        lit_end = lit_pos + ll
        raise "literal run #{ll} overflows literals buffer (pos=#{lit_pos} len=#{lits.size})" if lit_end > lits.size
        raise "decompressed size exceeds limit of #{MAX_OUTPUT} bytes" if out.length + ll > MAX_OUTPUT
        out.concat(lits[lit_pos...lit_end])
        lit_pos = lit_end

        # Copy ml bytes from offset positions back in the output.
        # offset = 1 means "last byte written".
        raise "bad match offset #{offset} (output len #{out.size})" if offset < 1 || offset > out.size
        raise "decompressed size exceeds limit of #{MAX_OUTPUT} bytes" if out.length + ml > MAX_OUTPUT
        copy_start = out.size - offset
        ml.times { |i| out << out[copy_start + i] }
      end

      # Remaining literals after the last sequence.
      raise "decompressed size exceeds limit of #{MAX_OUTPUT} bytes" if out.length + (lits.length - lit_pos) > MAX_OUTPUT
      out.concat(lits[lit_pos..])
    end

    # =========================================================================
    # Public API
    # =========================================================================

    # Compress +data+ to ZStd format (RFC 8878).
    #
    # The output is a valid ZStd frame with:
    #   - 4-byte magic
    #   - 1-byte FHD (Single_Segment=1, FCS_Field_Size=11 → 8-byte FCS)
    #   - 8-byte Frame_Content_Size
    #   - One or more blocks (RLE / Compressed / Raw)
    #
    # @param data [String] input bytes (any encoding; treated as binary)
    # @return     [String] compressed binary String
    #
    # Example:
    #   compressed = CodingAdventures::Zstd.compress("hello world" * 100)
    def self.compress(data)
      data = data.b  # ensure ASCII-8BIT encoding
      out = "".b

      # ── Frame header ──────────────────────────────────────────────────────
      # Magic number (4 bytes little-endian).
      out << [MAGIC].pack("V")

      # Frame Header Descriptor (FHD):
      #   bits [7:6] = FCS_Field_Size flag = 0b11 → 8-byte FCS
      #   bit  [5]   = Single_Segment_Flag = 1 (no Window_Descriptor)
      #   bit  [4]   = Content_Checksum_Flag = 0
      #   bits [3:2] = reserved = 0
      #   bits [1:0] = Dict_ID_Flag = 0
      #   = 0b11100000 = 0xE0
      out << "\xE0".b

      # Frame_Content_Size: uncompressed input size (8 bytes LE).
      # Lets decoders pre-allocate the output buffer.
      out << [data.bytesize].pack("Q<")

      # ── Blocks ────────────────────────────────────────────────────────────
      # Special case: empty input → one empty raw block.
      if data.empty?
        # last=1, type=Raw(00), size=0 → header bits = 0b001 = 0x01
        hdr = 0b001
        out << [hdr & 0xFF, (hdr >> 8) & 0xFF, (hdr >> 16) & 0xFF].pack("CCC")
        return out
      end

      offset = 0
      while offset < data.bytesize
        block_end = [offset + MAX_BLOCK_SIZE, data.bytesize].min
        block = data.byteslice(offset, block_end - offset)
        last = block_end == data.bytesize

        last_bit = last ? 1 : 0

        if !block.empty? && block.bytes.uniq.size == 1
          # ── RLE block ────────────────────────────────────────────────────
          # All bytes identical → encode as 1 byte + 3-byte header.
          hdr = (block.bytesize << 3) | (0b01 << 1) | last_bit
          out << [hdr & 0xFF, (hdr >> 8) & 0xFF, (hdr >> 16) & 0xFF].pack("CCC")
          out << block.byteslice(0, 1)
        else
          compressed = compress_block(block)
          if compressed
            # ── Compressed block ──────────────────────────────────────────
            hdr = (compressed.bytesize << 3) | (0b10 << 1) | last_bit
            out << [hdr & 0xFF, (hdr >> 8) & 0xFF, (hdr >> 16) & 0xFF].pack("CCC")
            out << compressed
          else
            # ── Raw block (fallback) ──────────────────────────────────────
            hdr = (block.bytesize << 3) | (0b00 << 1) | last_bit
            out << [hdr & 0xFF, (hdr >> 8) & 0xFF, (hdr >> 16) & 0xFF].pack("CCC")
            out << block
          end
        end

        offset = block_end
      end

      out
    end

    # Decompress a ZStd frame back to the original data.
    #
    # Accepts frames with:
    #   - Single-segment or multi-segment layout
    #   - Raw, RLE, or Compressed blocks
    #   - Predefined FSE modes only
    #
    # Raises RuntimeError on bad magic, truncation, or unsupported features.
    # Caps decompressed output at MAX_OUTPUT (256 MiB) to prevent zip bombs.
    #
    # @param data [String] ZStd-compressed binary String
    # @return     [String] decompressed binary String
    #
    # Example:
    #   original = CodingAdventures::Zstd.decompress(compressed)
    def self.decompress(data)
      data = data.b
      raise "frame too short" if data.bytesize < 5

      # ── Magic ─────────────────────────────────────────────────────────────
      magic = data.byteslice(0, 4).unpack1("V")
      raise "bad magic: 0x#{magic.to_s(16)} (expected 0x#{MAGIC.to_s(16)})" if magic != MAGIC

      pos = 4

      # ── Frame Header Descriptor ───────────────────────────────────────────
      fhd = data.getbyte(pos)
      pos += 1

      # FCS_Field_Size: bits [7:6].
      #   0b00 → 0 bytes (unless Single_Segment=1, then 1 byte)
      #   0b01 → 2 bytes (value + 256)
      #   0b10 → 4 bytes
      #   0b11 → 8 bytes
      fcs_flag = (fhd >> 6) & 3

      # Single_Segment_Flag: bit 5. When set, Window_Descriptor is omitted.
      single_seg = (fhd >> 5) & 1

      # Dict_ID_Flag: bits [1:0]. How many bytes the dict ID occupies.
      dict_flag = fhd & 3

      # ── Window Descriptor ─────────────────────────────────────────────────
      # Present only when Single_Segment_Flag = 0. Skip it (we don't enforce
      # window size limits in this implementation).
      pos += 1 if single_seg == 0

      # ── Dict ID ───────────────────────────────────────────────────────────
      dict_id_bytes = [0, 1, 2, 4][dict_flag]
      pos += dict_id_bytes  # skip dict ID (no custom dict support)

      # ── Frame Content Size ────────────────────────────────────────────────
      # Read but don't validate; we trust the block data to be correct.
      fcs_bytes = case fcs_flag
      when 0 then (single_seg == 1) ? 1 : 0
      when 1 then 2
      when 2 then 4
      when 3 then 8
      end
      pos += fcs_bytes  # skip FCS

      # ── Blocks ────────────────────────────────────────────────────────────
      out = []  # accumulate output bytes as integers

      loop do
        raise "truncated block header" if pos + 3 > data.bytesize

        # 3-byte little-endian block header.
        b0 = data.getbyte(pos)
        b1 = data.getbyte(pos + 1)
        b2 = data.getbyte(pos + 2)
        hdr = b0 | (b1 << 8) | (b2 << 16)
        pos += 3

        last = (hdr & 1) != 0
        btype = (hdr >> 1) & 3
        bsize = hdr >> 3

        case btype
        when 0
          # Raw block: bsize bytes of verbatim content.
          raise "raw block truncated" if pos + bsize > data.bytesize
          raise "decompressed size exceeds limit of #{MAX_OUTPUT} bytes" if out.size + bsize > MAX_OUTPUT
          out.concat(data.byteslice(pos, bsize).bytes)
          pos += bsize

        when 1
          # RLE block: 1 byte repeated bsize times.
          raise "RLE block missing byte" if pos >= data.bytesize
          raise "decompressed size exceeds limit of #{MAX_OUTPUT} bytes" if out.size + bsize > MAX_OUTPUT
          byte = data.getbyte(pos)
          pos += 1
          bsize.times { out << byte }

        when 2
          # Compressed block.
          raise "compressed block truncated" if pos + bsize > data.bytesize
          block_data = data.byteslice(pos, bsize)
          pos += bsize
          decompress_block(block_data, out)
          raise "decompressed size exceeds limit of #{MAX_OUTPUT} bytes" if out.size > MAX_OUTPUT

        when 3
          raise "reserved block type 3"
        end

        break if last
      end

      out.pack("C*")
    end
  end
end
