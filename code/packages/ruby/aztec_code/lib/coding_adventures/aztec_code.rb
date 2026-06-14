# frozen_string_literal: true

# =============================================================================
# coding_adventures/aztec_code — Aztec Code encoder (ISO/IEC 24778:2008)
# =============================================================================
#
# Aztec Code was invented in 1995 by Andrew Longacre Jr. at Welch Allyn and
# published as a patent-free format. Unlike QR Code (which sticks three large
# square finder patterns at three corners of the symbol), Aztec Code places a
# single **bullseye finder pattern at the center**. The scanner finds the
# bullseye first, then reads the data outward in a spiral.
#
# Why does that matter? Two practical wins:
#
#   1. NO LARGE QUIET ZONE — three of QR's four sides need a "white moat"
#      around them before the scanner can latch on. A center bullseye does
#      not, so Aztec packs more useful symbol per square inch.
#
#   2. ROTATION-INVARIANT — the bullseye is point-symmetric, so the scanner
#      can read the symbol from any of the four 90° orientations without
#      any disambiguation step. (QR uses three corners-of-three to break
#      the rotational ambiguity; Aztec uses a separate orientation ring.)
#
# ## Where Aztec Code shows up in the real world
#
#   - **IATA boarding passes** — the barcode on every airline boarding pass.
#   - **Eurostar / Amtrak rail tickets** — both printed and on-screen.
#   - **PostNL, Deutsche Post, La Poste** — European postal routing labels.
#   - **US military ID cards**.
#
# ## Symbol variants
#
# There are two flavours of Aztec Code symbol, with slightly different sizes:
#
#   Compact  — 1 to 4 layers,    size = 11 + 4·layers   (15×15  …  27×27)
#   Full     — 1 to 32 layers,   size = 15 + 4·layers   (19×19  … 143×143)
#
# A "layer" is a 2-module-wide ring around the bullseye+orientation core.
# The data spiral fills the layers from the inside out.
#
# ## Encoding pipeline (v0.1.0 — byte-mode only)
#
#   input string / bytes
#     → Binary-Shift codewords from Upper mode
#     → smallest symbol that fits at the requested ECC level
#     → pad to exact codeword count (and rescue an all-zero last codeword)
#     → GF(256)/0x12D Reed-Solomon ECC (b=1: roots α^1 … α^n)
#     → bit-stuff (insert one complement bit after every 4 identical bits)
#     → GF(16) mode message (layers + cw count + 5 or 6 RS nibbles)
#     → ModuleGrid (bullseye → orientation marks → mode msg → data spiral)
#
# ## v0.1.0 simplifications (faithful port of the TS reference)
#
#   1. **Byte-mode only** — every byte of input goes through one Binary-Shift
#      escape from Upper mode. The 5-mode (Upper / Lower / Mixed / Punct /
#      Digit) state-machine optimiser is a v0.2.0 problem.
#   2. **GF(256) RS only** — 4-bit and 5-bit codeword paths (which would let
#      smaller symbols use GF(16) / GF(32) RS) are v0.2.0.
#   3. **Default ECC = 23 %** — the standard's recommended minimum.
#   4. **Auto symbol selection only** — no `force_compact:` toggle yet.
#
# ## Dependencies (loaded first per Ruby require-ordering rules)
#
#   coding_adventures_paint_instructions — PaintScene IR
#   coding_adventures_barcode_2d         — ModuleGrid + layout()

require "coding_adventures_paint_instructions"
require "coding_adventures_barcode_2d"
require_relative "aztec_code/version"

module CodingAdventures
  # ===========================================================================
  # AztecCode — top-level namespace
  # ===========================================================================
  #
  # All public surface lives here:
  #
  #   .encode(data, min_ecc_percent: 23)         → ModuleGrid
  #   .encode_and_layout(data, …, config:)       → PaintScene
  #   AztecError, InputTooLong                   ← raised on overflow
  #
  # Helper functions are `module_function`-ised below the public API so they
  # can be unit-tested and reused by future format work without leaking into
  # the user's call site.
  module AztecCode
    # =========================================================================
    # Errors
    # =========================================================================

    # Base class for every Aztec Code error so callers can `rescue AztecError`
    # without enumerating subclasses.
    class AztecError < StandardError; end

    # Raised when the input cannot be packed into any 32-layer Full symbol at
    # the requested ECC level. (For byte-mode that's roughly ~1.9 KB at 23 %
    # ECC — far more than any practical Aztec barcode you'll ever scan.)
    class InputTooLong < AztecError; end

    # =========================================================================
    # GF(16) arithmetic — used by the mode-message Reed-Solomon
    # =========================================================================
    #
    # GF(16) is the finite field with 16 elements, built from the primitive
    # polynomial p(x) = x^4 + x + 1   (binary 10011 = 0x13).
    #
    # Every non-zero element can be written as a power of the primitive
    # element α. Because α is a root of p(x), we have α^4 = α + 1.
    #
    # Iterating x ← x·α (shift-left, XOR-with-poly-if-overflow) starting at 1
    # cycles through every non-zero element with period 15:
    #
    #   α^0 = 1   α^1 = 2   α^2 = 4   α^3 = 8
    #   α^4 = 3   α^5 = 6   α^6 = 12  α^7 = 11
    #   α^8 = 5   α^9 = 10  α^10 = 7  α^11 = 14
    #   α^12 = 15 α^13 = 13 α^14 = 9  α^15 = 1   (period = 15)
    #
    # We pre-compute log/antilog tables to make multiplication O(1).

    # Discrete logarithm table: LOG16[e] = i such that α^i = e.
    # LOG16[0] is unused (log(0) is undefined); we store -1 as a sentinel.
    LOG16 = [
      -1, # log(0) — undefined sentinel
      0,  # log(1) = 0
      1,  # log(2) = 1
      4,  # log(3) = 4
      2,  # log(4) = 2
      8,  # log(5) = 8
      5,  # log(6) = 5
      10, # log(7) = 10
      3,  # log(8) = 3
      14, # log(9) = 14
      9,  # log(10) = 9
      7,  # log(11) = 7
      6,  # log(12) = 6
      13, # log(13) = 13
      11, # log(14) = 11
      12  # log(15) = 12
    ].freeze

    # Antilog table: ALOG16[i] = α^i (length 16, with index 15 wrapping to 1).
    ALOG16 = [1, 2, 4, 8, 3, 6, 12, 11, 5, 10, 7, 14, 15, 13, 9, 1].freeze

    # =========================================================================
    # GF(256)/0x12D arithmetic — used by the data Reed-Solomon
    # =========================================================================
    #
    # Aztec Code uses GF(256) with primitive polynomial
    #
    #   p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  =  0x12D
    #
    # This is the **same polynomial as Data Matrix ECC200** but **different
    # from QR Code** (which uses 0x11D). The repo's stand-alone `gf256` gem is
    # hard-coded for the QR polynomial 0x11D, so we build our own 0x12D tables
    # inline rather than introducing a flag and breaking 60+ callers.
    #
    # Generator convention: b = 1 — first root is α^1, not α^0. (This is the
    # MA02 / Aztec convention; QR uses b = 0.)

    # The Aztec / Data-Matrix primitive polynomial as a 9-bit integer.
    GF256_POLY = 0x12d

    # Doubled exponentiation table. EXP_12D[i] = α^i for i ∈ 0..254 and the
    # indices 255..509 mirror 0..254 so we can index with `LOG[a] + LOG[b]`
    # without an explicit `mod 255`.
    EXP_12D = Array.new(512, 0)

    # LOG_12D[e] = discrete log of e in GF(256)/0x12D.
    LOG_12D = Array.new(256, 0)

    # Build the tables exactly once at load time. Primitive element is α = 2.
    x = 1
    255.times do |i|
      EXP_12D[i] = x
      EXP_12D[i + 255] = x
      LOG_12D[x] = i
      x <<= 1
      x ^= GF256_POLY if (x & 0x100) != 0
      x &= 0xff
    end
    EXP_12D[255] = 1
    EXP_12D.freeze
    LOG_12D.freeze

    # =========================================================================
    # Aztec Code capacity tables  (ISO/IEC 24778:2008 Table 1)
    # =========================================================================
    #
    # Each layer count maps to:
    #   total_bits   — total data+ECC bit positions in the symbol
    #   max_bytes_8  — number of 8-bit codeword slots that fit in those bits
    #
    # Index 0 is a placeholder so layers can be 1-based.

    # Compact symbols: 1 to 4 layers (15×15 to 27×27 modules).
    COMPACT_CAPACITY = [
      {total_bits: 0, max_bytes_8: 0},      # index 0 — unused
      {total_bits: 72, max_bytes_8: 9},     # 1 layer  / 15×15
      {total_bits: 200, max_bytes_8: 25},   # 2 layers / 19×19
      {total_bits: 392, max_bytes_8: 49},   # 3 layers / 23×23
      {total_bits: 648, max_bytes_8: 81}    # 4 layers / 27×27
    ].freeze

    # Full symbols: 1 to 32 layers (19×19 to 143×143 modules).
    FULL_CAPACITY = [
      {total_bits: 0, max_bytes_8: 0},          # index 0 — unused
      {total_bits: 88, max_bytes_8: 11},        #  1
      {total_bits: 216, max_bytes_8: 27},       #  2
      {total_bits: 360, max_bytes_8: 45},       #  3
      {total_bits: 520, max_bytes_8: 65},       #  4
      {total_bits: 696, max_bytes_8: 87},       #  5
      {total_bits: 888, max_bytes_8: 111},      #  6
      {total_bits: 1096, max_bytes_8: 137},     #  7
      {total_bits: 1320, max_bytes_8: 165},     #  8
      {total_bits: 1560, max_bytes_8: 195},     #  9
      {total_bits: 1816, max_bytes_8: 227},     # 10
      {total_bits: 2088, max_bytes_8: 261},     # 11
      {total_bits: 2376, max_bytes_8: 297},     # 12
      {total_bits: 2680, max_bytes_8: 335},     # 13
      {total_bits: 3000, max_bytes_8: 375},     # 14
      {total_bits: 3336, max_bytes_8: 417},     # 15
      {total_bits: 3688, max_bytes_8: 461},     # 16
      {total_bits: 4056, max_bytes_8: 507},     # 17
      {total_bits: 4440, max_bytes_8: 555},     # 18
      {total_bits: 4840, max_bytes_8: 605},     # 19
      {total_bits: 5256, max_bytes_8: 657},     # 20
      {total_bits: 5688, max_bytes_8: 711},     # 21
      {total_bits: 6136, max_bytes_8: 767},     # 22
      {total_bits: 6600, max_bytes_8: 825},     # 23
      {total_bits: 7080, max_bytes_8: 885},     # 24
      {total_bits: 7576, max_bytes_8: 947},     # 25
      {total_bits: 8088, max_bytes_8: 1011},    # 26
      {total_bits: 8616, max_bytes_8: 1077},    # 27
      {total_bits: 9160, max_bytes_8: 1145},    # 28
      {total_bits: 9720, max_bytes_8: 1215},    # 29
      {total_bits: 10296, max_bytes_8: 1287},   # 30
      {total_bits: 10888, max_bytes_8: 1361},   # 31
      {total_bits: 11496, max_bytes_8: 1437}    # 32
    ].freeze

    # =========================================================================
    # SymbolSpec — the result of "what symbol fits this input?"
    # =========================================================================
    #
    # Returned by select_symbol; carries every dimension the rest of the
    # pipeline needs (no further capacity-table lookups required).
    SymbolSpec = Struct.new(
      :compact, :layers, :data_cw_count, :ecc_cw_count, :total_bits,
      keyword_init: true
    )

    # =========================================================================
    # module_function — every helper below this line is a module-level method.
    # =========================================================================
    #
    # `module_function` makes them callable both as `AztecCode.foo(...)` from
    # outside (handy for tests) and as `foo(...)` from inside the module.
    module_function

    # -------------------------------------------------------------------------
    # gf16_mul — multiply two GF(16) elements via log/antilog tables.
    #
    # Returns 0 if either operand is 0 (multiplicative identity short-circuit).
    # Otherwise: α^(log a + log b) mod 15.
    # -------------------------------------------------------------------------
    def gf16_mul(a, b)
      return 0 if a == 0 || b == 0

      ALOG16[(LOG16[a] + LOG16[b]) % 15]
    end

    # -------------------------------------------------------------------------
    # build_gf16_generator — build the GF(16) RS generator polynomial g(x)
    # whose roots are α^1 … α^n.
    #
    # Returned coefficient layout: [g_0, g_1, …, g_n]  with  g_n = 1 (monic).
    #
    # Construction: start with g(x) = 1, then multiply by (x - α^i) for each
    # i = 1..n. Because XOR is its own inverse in GF(2), "(x - α^i)" is the
    # same as "(x + α^i)".
    # -------------------------------------------------------------------------
    def build_gf16_generator(n)
      g = [1]
      (1..n).each do |i|
        ai = ALOG16[i % 15]
        nxt = Array.new(g.length + 1, 0)
        g.each_with_index do |gj, j|
          nxt[j + 1] ^= gj
          nxt[j] ^= gf16_mul(ai, gj)
        end
        g = nxt
      end
      g
    end

    # -------------------------------------------------------------------------
    # gf16_rs_encode — compute n GF(16) RS check nibbles for given data.
    #
    # Standard LFSR polynomial-division layout: the remainder array `rem`
    # holds the n trailing coefficients of the data·x^n / g(x) division.
    # -------------------------------------------------------------------------
    def gf16_rs_encode(data, n)
      g = build_gf16_generator(n)
      rem = Array.new(n, 0)
      data.each do |b|
        fb = b ^ rem[0]
        (0...(n - 1)).each do |i|
          rem[i] = rem[i + 1] ^ gf16_mul(g[i + 1], fb)
        end
        rem[n - 1] = gf16_mul(g[n], fb)
      end
      rem
    end

    # -------------------------------------------------------------------------
    # gf256_mul — multiply two GF(256)/0x12D elements via the doubled EXP table.
    #
    # Doubling the EXP table lets us add the logs without an explicit `% 255`.
    # -------------------------------------------------------------------------
    def gf256_mul(a, b)
      return 0 if a == 0 || b == 0

      EXP_12D[LOG_12D[a] + LOG_12D[b]]
    end

    # -------------------------------------------------------------------------
    # build_gf256_generator — RS generator for GF(256)/0x12D, roots α^1..α^n.
    #
    # Returns [g_0, g_1, …, g_n] in big-endian / highest-degree-first order
    # — the order the LFSR encoder expects.
    # -------------------------------------------------------------------------
    def build_gf256_generator(n)
      g = [1]
      (1..n).each do |i|
        ai = EXP_12D[i]
        nxt = Array.new(g.length + 1, 0)
        g.each_with_index do |gj, j|
          nxt[j] ^= gj
          nxt[j + 1] ^= gf256_mul(gj, ai)
        end
        g = nxt
      end
      g
    end

    # -------------------------------------------------------------------------
    # gf256_rs_encode — compute n_check GF(256)/0x12D RS check bytes.
    #
    # Identical LFSR shape to the GF(16) variant, just over a bigger field.
    # -------------------------------------------------------------------------
    def gf256_rs_encode(data, n_check)
      g = build_gf256_generator(n_check)
      n = g.length - 1
      rem = Array.new(n, 0)
      data.each do |b|
        fb = b ^ rem[0]
        (0...(n - 1)).each do |i|
          rem[i] = rem[i + 1] ^ gf256_mul(g[i + 1], fb)
        end
        rem[n - 1] = gf256_mul(g[n], fb)
      end
      rem
    end

    # =========================================================================
    # Data encoding — Binary-Shift from Upper mode (v0.1.0 byte-mode path)
    # =========================================================================
    #
    # All input is wrapped in a single Binary-Shift block that begins from
    # the Upper-Latch state (the Aztec encoder's startup state). That gives us:
    #
    #   1.  5 bits = 0b11111   — the Binary-Shift escape from Upper mode.
    #   2.  Length field:
    #         len ≤ 31  → 5 bits with the byte count.
    #         len > 31  → 5 bits = 0b00000  followed by 11 bits with the count.
    #   3.  Each byte as 8 bits, MSB first.
    #
    # The receiver, after consuming all `len` bytes, drops back to Upper mode
    # automatically — no closing escape needed.

    # -------------------------------------------------------------------------
    # encode_bytes_as_bits — turn raw bytes into a flat 0/1 array (MSB-first).
    # -------------------------------------------------------------------------
    def encode_bytes_as_bits(input_bytes)
      bits = []

      # Inline writer: append `count` LSBs of `value`, highest bit first.
      write_bits = lambda do |value, count|
        (count - 1).downto(0) { |i| bits << ((value >> i) & 1) }
      end

      len = input_bytes.length
      write_bits.call(31, 5) # Binary-Shift escape from Upper

      if len <= 31
        write_bits.call(len, 5)
      else
        write_bits.call(0, 5)
        write_bits.call(len, 11)
      end

      input_bytes.each { |byte| write_bits.call(byte, 8) }
      bits
    end

    # =========================================================================
    # Symbol-size selection
    # =========================================================================

    # -------------------------------------------------------------------------
    # select_symbol — find the smallest Aztec symbol that fits `data_bit_count`
    # data bits at `min_ecc_pct` % error correction.
    #
    # Strategy: try Compact 1..4 first (smallest first), then Full 1..32.
    # Bit stuffing inflates the bit stream by up to ~1/16, so we apply a
    # conservative 20 % overhead to be safe. (The exact stuffed length is
    # input-dependent, so a real round-trip after stuffing is needed for
    # tightly-packed cases; the v0.2.0 multi-mode path will tighten this.)
    #
    # Raises InputTooLong if no symbol fits.
    # -------------------------------------------------------------------------
    def select_symbol(data_bit_count, min_ecc_pct)
      stuffed_bit_count = (data_bit_count * 1.2).ceil

      # Try Compact 1..4 layers
      (1..4).each do |layers|
        cap = COMPACT_CAPACITY[layers]
        next if cap.nil?

        total_bytes = cap[:max_bytes_8]
        ecc_cw_count = ((min_ecc_pct.to_f / 100) * total_bytes).ceil
        data_cw_count = total_bytes - ecc_cw_count
        next if data_cw_count <= 0
        next unless (stuffed_bit_count.to_f / 8).ceil <= data_cw_count

        return SymbolSpec.new(
          compact: true, layers: layers,
          data_cw_count: data_cw_count, ecc_cw_count: ecc_cw_count,
          total_bits: cap[:total_bits]
        )
      end

      # Fall through to Full 1..32 layers
      (1..32).each do |layers|
        cap = FULL_CAPACITY[layers]
        next if cap.nil?

        total_bytes = cap[:max_bytes_8]
        ecc_cw_count = ((min_ecc_pct.to_f / 100) * total_bytes).ceil
        data_cw_count = total_bytes - ecc_cw_count
        next if data_cw_count <= 0
        next unless (stuffed_bit_count.to_f / 8).ceil <= data_cw_count

        return SymbolSpec.new(
          compact: false, layers: layers,
          data_cw_count: data_cw_count, ecc_cw_count: ecc_cw_count,
          total_bits: cap[:total_bits]
        )
      end

      raise InputTooLong,
        "Input is too long to fit in any Aztec Code symbol " \
        "(#{data_bit_count} bits needed)"
    end

    # =========================================================================
    # Padding
    # =========================================================================

    # -------------------------------------------------------------------------
    # pad_to_bytes — pad bit list to exactly `target_bytes`·8 bits.
    #
    # First, top up the partial byte (if any) with zeros. Then append whole
    # zero bytes until we hit the target. Finally clip — should be a no-op
    # since we never go past target, but we include it as a safety belt.
    # -------------------------------------------------------------------------
    def pad_to_bytes(bits, target_bytes)
      out = bits.dup
      out << 0 while out.length % 8 != 0
      out << 0 while out.length < target_bytes * 8
      out[0, target_bytes * 8]
    end

    # =========================================================================
    # Bit stuffing
    # =========================================================================
    #
    # Aztec disallows codewords that are all zero or all ones. Rather than
    # special-case at the codeword level, the spec defines a pre-RS bit-level
    # transformation: after every run of 4 identical bits, insert one
    # complement bit. (After unstuffing, decoders simply drop any bit that
    # follows a 4-run.)
    #
    # Worked example:
    #   raw:    1 1 1 1   0 0 0 0
    #   pass 1: 1 1 1 1 0          ← inserted 0 after the four 1s
    #   pass 2: 1 1 1 1 0   0 0 0 0 1   ← inserted 1 after the four 0s
    #   note:   the inserted bit RESETS the run counter

    # -------------------------------------------------------------------------
    # stuff_bits — apply Aztec bit stuffing to a bit array.
    # -------------------------------------------------------------------------
    def stuff_bits(bits)
      stuffed = []
      run_val = -1
      run_len = 0

      bits.each do |bit|
        if bit == run_val
          run_len += 1
        else
          run_val = bit
          run_len = 1
        end

        stuffed << bit

        next unless run_len == 4

        stuff_bit = 1 - bit
        stuffed << stuff_bit
        run_val = stuff_bit
        run_len = 1
      end

      stuffed
    end

    # =========================================================================
    # Mode-message encoding
    # =========================================================================
    #
    # The mode message is a tiny field that tells the decoder the symbol's
    # layer count and codeword count. It lives on the perimeter ring just
    # outside the bullseye, between the four orientation-mark corners. It is
    # protected by GF(16) RS so the scanner can decode the symbol layout even
    # if a couple of those modules are damaged.
    #
    # Compact (28 bits = 7 nibbles):
    #   m = ((layers-1) << 6) | (data_cw_count-1)
    #   2 data nibbles + 5 ECC nibbles
    #
    # Full (40 bits = 10 nibbles):
    #   m = ((layers-1) << 11) | (data_cw_count-1)
    #   4 data nibbles + 6 ECC nibbles

    # -------------------------------------------------------------------------
    # encode_mode_message — build the mode-message bit stream.
    # -------------------------------------------------------------------------
    def encode_mode_message(compact, layers, data_cw_count)
      if compact
        m = ((layers - 1) << 6) | (data_cw_count - 1)
        data_nibbles = [m & 0xf, (m >> 4) & 0xf]
        num_ecc = 5
      else
        m = ((layers - 1) << 11) | (data_cw_count - 1)
        data_nibbles = [m & 0xf, (m >> 4) & 0xf, (m >> 8) & 0xf, (m >> 12) & 0xf]
        num_ecc = 6
      end

      ecc_nibbles = gf16_rs_encode(data_nibbles, num_ecc)
      all_nibbles = data_nibbles + ecc_nibbles

      bits = []
      all_nibbles.each do |nibble|
        3.downto(0) { |i| bits << ((nibble >> i) & 1) }
      end
      bits
    end

    # =========================================================================
    # Geometry helpers
    # =========================================================================

    # Symbol side length in modules: compact = 11+4·L, full = 15+4·L.
    def symbol_size(compact, layers)
      compact ? 11 + 4 * layers : 15 + 4 * layers
    end

    # Chebyshev radius of the bullseye core: compact = 5, full = 7.
    def bullseye_radius(compact)
      compact ? 5 : 7
    end

    # =========================================================================
    # Grid construction — bullseye, reference grid, orientation, mode message
    # =========================================================================
    #
    # We use mutable 2D arrays during construction because the spiral writer
    # is much simpler that way; the grid is frozen into a ModuleGrid at the
    # very end, so the public API is still immutable.

    # -------------------------------------------------------------------------
    # draw_bullseye — paint the central concentric-square locator.
    #
    # Color rule by Chebyshev distance d from the centre:
    #   d ≤ 1   → DARK   (solid 3×3 inner core)
    #   d > 1   → DARK if d is odd, LIGHT if d is even.
    # -------------------------------------------------------------------------
    def draw_bullseye(modules, reserved, cx, cy, compact)
      br = bullseye_radius(compact)
      ((cy - br)..(cy + br)).each do |row|
        ((cx - br)..(cx + br)).each do |col|
          d = [(col - cx).abs, (row - cy).abs].max
          dark = (d <= 1) ? true : d.odd?
          modules[row][col] = dark
          reserved[row][col] = true
        end
      end
    end

    # -------------------------------------------------------------------------
    # draw_reference_grid — paint the alignment "reference grid" (Full only).
    #
    # Full Aztec symbols have grid lines at every row/column whose offset
    # from the centre is a multiple of 16. These provide additional anchor
    # points so very large symbols can be sampled accurately even under
    # severe perspective distortion.
    #
    # Rules:
    #   - on horizontal AND vertical grid → DARK
    #   - on horizontal only              → DARK if cx-col is even, else LIGHT
    #   - on vertical only                → DARK if cy-row is even, else LIGHT
    # -------------------------------------------------------------------------
    def draw_reference_grid(modules, reserved, cx, cy, size)
      size.times do |row|
        size.times do |col|
          on_h = ((cy - row) % 16) == 0
          on_v = ((cx - col) % 16) == 0
          next if !on_h && !on_v

          dark = if on_h && on_v
            true
          elsif on_h
            ((cx - col) % 2) == 0
          else
            ((cy - row) % 2) == 0
          end

          modules[row][col] = dark
          reserved[row][col] = true
        end
      end
    end

    # -------------------------------------------------------------------------
    # draw_orientation_and_mode_message — paint the perimeter ring just
    # outside the bullseye.
    #
    # That ring at Chebyshev radius (bullseye_radius + 1) is split into:
    #   - 4 corners      → orientation marks (always DARK)
    #   - 4 edges        → mode-message bits, clockwise from "TL+1"
    #
    # Returns the slice of ring positions that the mode message did not
    # consume, so the data spiral writer can keep filling them with payload.
    # -------------------------------------------------------------------------
    def draw_orientation_and_mode_message(modules, reserved, cx, cy, compact, mode_message_bits)
      r = bullseye_radius(compact) + 1

      # Enumerate non-corner perimeter positions clockwise from TL+1.
      # Each entry is [col, row] — note the col-first ordering matches the
      # TS reference and mirrors the way decoders walk this ring.
      non_corner = []

      # Top edge — left to right (skip both corners)
      ((cx - r + 1)..(cx + r - 1)).each { |col| non_corner << [col, cy - r] }
      # Right edge — top to bottom (skip both corners)
      ((cy - r + 1)..(cy + r - 1)).each { |row| non_corner << [cx + r, row] }
      # Bottom edge — right to left (skip both corners)
      (cx + r - 1).downto(cx - r + 1) { |col| non_corner << [col, cy + r] }
      # Left edge — bottom to top (skip both corners)
      (cy + r - 1).downto(cy - r + 1) { |row| non_corner << [cx - r, row] }

      # Place the 4 orientation marks (always DARK).
      [
        [cx - r, cy - r],
        [cx + r, cy - r],
        [cx + r, cy + r],
        [cx - r, cy + r]
      ].each do |col, row|
        modules[row][col] = true
        reserved[row][col] = true
      end

      # Place the mode-message bits.
      mode_message_bits.each_with_index do |bit, i|
        break if i >= non_corner.length

        col, row = non_corner[i]
        modules[row][col] = bit == 1
        reserved[row][col] = true
      end

      # Hand back the leftover positions for the data writer.
      non_corner[mode_message_bits.length..] || []
    end

    # =========================================================================
    # Data placement — clockwise layer spiral
    # =========================================================================
    #
    # Each "layer" is a 2-module-wide ring around the previous one. Inside
    # each ring we walk clockwise around the four edges, writing two bits
    # per column (or row): outer position first, then inner.
    #
    # First-data-layer inner radius:
    #   compact: bullseye_radius + 2 = 7
    #   full:    bullseye_radius + 2 = 9

    # -------------------------------------------------------------------------
    # place_data_bits — fill all data+ECC positions.
    #
    # 1. Drain any leftover mode-ring positions (passed in directly — they
    #    are NOT marked reserved, so we write them blindly).
    # 2. Walk each data layer clockwise, top→right→bottom→left, writing
    #    pairs of bits per step. Cells that fall on the reference grid
    #    (Full symbols only) are reserved and silently skipped.
    # -------------------------------------------------------------------------
    def place_data_bits(modules, reserved, bits, cx, cy, compact, layers, mode_ring_remaining_positions)
      size = modules.length
      bit_index = 0

      # Local closure that writes the current bit to (col, row) if and only if
      # the position is in-bounds and not reserved. Auto-pads with zeros if
      # the bit stream runs short (rare but possible at small sizes).
      place_bit = lambda do |col, row|
        return if row < 0 || row >= size || col < 0 || col >= size
        return if reserved[row][col]

        modules[row][col] = (bits[bit_index] || 0) == 1
        bit_index += 1
      end

      # Step 1 — finish the mode ring (positions passed in are unreserved).
      mode_ring_remaining_positions.each do |col, row|
        modules[row][col] = (bits[bit_index] || 0) == 1
        bit_index += 1
      end

      # Step 2 — clockwise spiral through the data layers.
      br = bullseye_radius(compact)
      d_start = br + 2 # mode message ring sits at br+1; data starts at br+2

      layers.times do |layer|
        d_i = d_start + 2 * layer # inner radius of this 2-wide ring
        d_o = d_i + 1             # outer radius of this 2-wide ring

        # Top edge — left → right
        ((cx - d_i + 1)..(cx + d_i)).each do |col|
          place_bit.call(col, cy - d_o)
          place_bit.call(col, cy - d_i)
        end
        # Right edge — top → bottom
        ((cy - d_i + 1)..(cy + d_i)).each do |row|
          place_bit.call(cx + d_o, row)
          place_bit.call(cx + d_i, row)
        end
        # Bottom edge — right → left
        (cx + d_i).downto(cx - d_i + 1) do |col|
          place_bit.call(col, cy + d_o)
          place_bit.call(col, cy + d_i)
        end
        # Left edge — bottom → top
        (cy + d_i).downto(cy - d_i + 1) do |row|
          place_bit.call(cx - d_o, row)
          place_bit.call(cx - d_i, row)
        end
      end
    end

    # =========================================================================
    # Public API — encode, encode_and_layout
    # =========================================================================

    # -------------------------------------------------------------------------
    # encode — main entry point.
    #
    # Steps:
    #   1. Encode input via Binary-Shift from Upper mode.
    #   2. Pick the smallest symbol that fits at min_ecc_percent.
    #   3. Pad the data codeword sequence to the chosen size.
    #   4. Compute GF(256)/0x12D Reed-Solomon ECC.
    #   5. Apply bit stuffing.
    #   6. Compute the GF(16)-protected mode message.
    #   7. Lay out structural patterns: bullseye, (reference grid for Full),
    #      orientation marks, mode message ring.
    #   8. Place data+ECC bits in the clockwise layer spiral.
    #
    # Parameters:
    #   data              — String or Array of byte integers (0..255).
    #   min_ecc_percent:  — minimum error-correction percentage (10..90),
    #                       default 23.
    #
    # Returns a frozen CodingAdventures::Barcode2D::ModuleGrid.
    #
    # Raises InputTooLong if the data exceeds 32-layer Full capacity.
    # -------------------------------------------------------------------------
    def encode(data, min_ecc_percent: 23)
      # Normalise input to a byte array.
      input_bytes = case data
      when String
        data.b.bytes
      when Array
        data.map { |b| b & 0xff }
      else
        raise ArgumentError,
          "encode: data must be a String or Array of bytes (got #{data.class})"
      end

      # ── Step 1: encode data ─────────────────────────────────────────────
      data_bits = encode_bytes_as_bits(input_bytes)

      # ── Step 2: pick a symbol ────────────────────────────────────────────
      spec = select_symbol(data_bits.length, min_ecc_percent)
      compact = spec.compact
      layers = spec.layers
      data_cw_count = spec.data_cw_count
      ecc_cw_count = spec.ecc_cw_count

      # ── Step 3: pad to data_cw_count whole bytes ─────────────────────────
      padded_bits = pad_to_bytes(data_bits, data_cw_count)

      data_bytes = []
      data_cw_count.times do |i|
        byte = 0
        8.times { |b| byte = (byte << 1) | padded_bits[i * 8 + b].to_i }
        # Avoid all-zero last codeword by mapping it to 0xff. This is a
        # cheap rescue for the only case where padding would otherwise
        # produce a forbidden codeword.
        byte = 0xff if byte == 0 && i == data_cw_count - 1
        data_bytes << byte
      end

      # ── Step 4: Reed-Solomon ECC over GF(256)/0x12D ─────────────────────
      ecc_bytes = gf256_rs_encode(data_bytes, ecc_cw_count)

      # ── Step 5: build raw bit stream and stuff ───────────────────────────
      all_bytes = data_bytes + ecc_bytes
      raw_bits = []
      all_bytes.each do |byte|
        7.downto(0) { |i| raw_bits << ((byte >> i) & 1) }
      end
      stuffed_bits = stuff_bits(raw_bits)

      # ── Step 6: mode message ─────────────────────────────────────────────
      mode_msg = encode_mode_message(compact, layers, data_cw_count)

      # ── Step 7: initialise grid + structural patterns ────────────────────
      size = symbol_size(compact, layers)
      cx = size / 2
      cy = size / 2

      modules = Array.new(size) { Array.new(size, false) }
      reserved = Array.new(size) { Array.new(size, false) }

      # Reference grid first (Full only). The bullseye paints over its centre
      # cells, which is fine — the bullseye / orientation marks "win".
      draw_reference_grid(modules, reserved, cx, cy, size) unless compact
      draw_bullseye(modules, reserved, cx, cy, compact)

      mode_ring_remaining_positions = draw_orientation_and_mode_message(
        modules, reserved, cx, cy, compact, mode_msg
      )

      # ── Step 8: place data+ECC spiral ───────────────────────────────────
      place_data_bits(
        modules, reserved, stuffed_bits, cx, cy, compact, layers,
        mode_ring_remaining_positions
      )

      # Return as a frozen ModuleGrid (rows of frozen arrays).
      CodingAdventures::Barcode2D::ModuleGrid.new(
        cols: size,
        rows: size,
        modules: modules.map(&:freeze).freeze,
        module_shape: "square"
      ).freeze
    end

    # -------------------------------------------------------------------------
    # layout — convert a ModuleGrid to a PaintScene.
    #
    # Aztec scanners do not need the same large quiet zone QR scanners do,
    # so we default to 2 modules (the standard's recommended minimum).
    # -------------------------------------------------------------------------
    def layout(grid, config = nil)
      cfg = {quiet_zone_modules: 2}.merge(config || {})
      CodingAdventures::Barcode2D.layout(grid, cfg)
    end

    # -------------------------------------------------------------------------
    # encode_and_layout — convenience: encode + layout in one call.
    # -------------------------------------------------------------------------
    def encode_and_layout(data, min_ecc_percent: 23, config: nil)
      grid = encode(data, min_ecc_percent: min_ecc_percent)
      layout(grid, config)
    end
  end
end
