# frozen_string_literal: true

# =============================================================================
# coding_adventures/data_matrix — Data Matrix ECC200 encoder
#                                 ISO/IEC 16022:2006 compliant
# =============================================================================
#
# Data Matrix is a two-dimensional matrix barcode invented in 1989 (originally
# called "DataCode") and standardised as ISO/IEC 16022:2006.  ECC200 is the
# modern variant — using Reed-Solomon over GF(256) — that has entirely
# displaced the older ECC000–ECC140 lineage.
#
# ## Where Data Matrix appears in the real world
#
#   - **PCBs** — every modern circuit board carries a tiny Data Matrix etched
#     directly on the substrate for traceability through automated assembly.
#   - **Pharmaceuticals** — US FDA DSCSA mandates Data Matrix on unit-dose
#     packages; the GS1 DataMatrix standard handles the encoding.
#   - **Aerospace parts** — etched / dot-peened marks survive decades of heat
#     and abrasion that would destroy ink-printed labels.
#   - **Medical devices** — GS1 DataMatrix on surgical instruments and implants.
#   - **USPS registered mail** — customs forms and domestic parcels.
#
# ## Key differences from QR Code
#
#   ┌──────────────────┬─────────────────────┬──────────────────────────┐
#   │ Property         │ QR Code             │ Data Matrix ECC200       │
#   ├──────────────────┼─────────────────────┼──────────────────────────┤
#   │ GF(256) poly     │ 0x11D               │ 0x12D                    │
#   │ RS root start    │ b = 0 (α^0..)       │ b = 1 (α^1..)            │
#   │ Finder           │ three corner squares │ one L-shape (left+bottom)│
#   │ Placement        │ column zigzag       │ "Utah" diagonal          │
#   │ Masking          │ 8 patterns, scored  │ NONE                     │
#   │ Sizes            │ 40 versions         │ 30 square + 6 rectangular│
#   └──────────────────┴─────────────────────┴──────────────────────────┘
#
# ## Encoding pipeline
#
#   input string
#     → ASCII encoding      (char+1; digit pairs packed into one codeword)
#     → symbol selection    (smallest symbol whose capacity ≥ codeword count)
#     → pad to capacity     (scrambled-pad codewords fill unused slots)
#     → RS blocks + ECC     (GF(256)/0x12D, b=1 convention, per-block)
#     → interleave blocks   (data round-robin then ECC round-robin)
#     → grid init           (L-finder + timing border + alignment borders)
#     → Utah placement      (diagonal codeword placement, NO masking)
#     → ModuleGrid          (abstract boolean grid, true = dark)
#
# ## Quick start
#
#   require "coding_adventures/data_matrix"
#   grid = CodingAdventures::DataMatrix.encode("HELLO WORLD")
#   grid.rows    # 10   (10×10 is the smallest symbol, fits short strings)
#   grid.cols    # 10
#   grid.modules # Array<Array<Boolean>> — true = dark, [row][col]
#
# ## GF(256)/0x12D vs GF(256)/0x11D
#
# Both QR Code and Data Matrix live in GF(256) — the same field of 256
# elements.  But the "primitive polynomial" used to build the field determines
# how multiplication wraps at the boundary.  Using the wrong polynomial gives
# an entirely different field isomorphism, which silently produces invalid ECC.
#
# Data Matrix uses  p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  = 0x12D = 301
# QR Code uses      p(x) = x^8 + x^4 + x^3 + x^2 + 1      = 0x11D = 285
#
# Never mix their exp/log tables.

require_relative "data_matrix/version"
require_relative "data_matrix/errors"

module CodingAdventures
  # ===========================================================================
  # DataMatrix — top-level encoding namespace
  # ===========================================================================
  #
  # All public surface lives here as `module_function` methods so they can be
  # called as `CodingAdventures::DataMatrix.encode(...)` (the normal external
  # path) and as bare `encode(...)` from inside the module (useful in tests).
  module DataMatrix
    # =========================================================================
    # Public constants
    # =========================================================================

    # Primitive polynomial for GF(256)/0x12D — the field used by Data Matrix.
    # In binary: 100101101 = x^8 + x^5 + x^4 + x^2 + x + 1.
    GF256_PRIME = 0x12D

    # Smallest and largest square Data Matrix symbol side lengths.
    MIN_SIZE = 10
    MAX_SIZE = 144

    # Output structure — a plain Struct so callers get named fields.
    #
    # Fields:
    #   rows    — Integer: height of the complete symbol in modules
    #   cols    — Integer: width  of the complete symbol in modules
    #   modules — Array<Array<Boolean>>: modules[r][c] is true if dark
    ModuleGrid = Struct.new(:rows, :cols, :modules)

    # =========================================================================
    # Symbol size table — ISO/IEC 16022:2006 Table 7
    # =========================================================================
    #
    # Every Data Matrix symbol decomposes as:
    #
    #   symbol = outer_border + (region_rows × region_cols) data regions
    #
    # Each data region is (data_region_height × data_region_width) modules.
    # Regions are separated by 2-module alignment borders, and the whole
    # symbol is wrapped in a 1-module finder/timing border.
    #
    # Fields per entry (in order):
    #   symbol_rows, symbol_cols   — total symbol size (includes all borders)
    #   region_rows, region_cols   — how many data regions vertically/horizontally
    #   data_region_height         — interior data height per region
    #   data_region_width          — interior data width  per region
    #   data_cw                    — total data codeword capacity
    #   ecc_cw                     — total ECC codewords
    #   num_blocks                 — number of interleaved RS blocks
    #   ecc_per_block              — ECC codewords per block

    # SymbolEntry — frozen struct holding all parameters for one symbol size.
    SymbolEntry = Struct.new(
      :symbol_rows, :symbol_cols,
      :region_rows, :region_cols,
      :data_region_height, :data_region_width,
      :data_cw, :ecc_cw,
      :num_blocks, :ecc_per_block
    )

    # The 24 square symbol sizes, ascending capacity order.
    # Source: ISO/IEC 16022:2006 Table 7.
    SQUARE_SIZES = [
      SymbolEntry.new(10, 10, 1, 1, 8, 8, 3, 5, 1, 5),
      SymbolEntry.new(12, 12, 1, 1, 10, 10, 5, 7, 1, 7),
      SymbolEntry.new(14, 14, 1, 1, 12, 12, 8, 10, 1, 10),
      SymbolEntry.new(16, 16, 1, 1, 14, 14, 12, 12, 1, 12),
      SymbolEntry.new(18, 18, 1, 1, 16, 16, 18, 14, 1, 14),
      SymbolEntry.new(20, 20, 1, 1, 18, 18, 22, 18, 1, 18),
      SymbolEntry.new(22, 22, 1, 1, 20, 20, 30, 20, 1, 20),
      SymbolEntry.new(24, 24, 1, 1, 22, 22, 36, 24, 1, 24),
      SymbolEntry.new(26, 26, 1, 1, 24, 24, 44, 28, 1, 28),
      SymbolEntry.new(32, 32, 2, 2, 14, 14, 62, 36, 2, 18),
      SymbolEntry.new(36, 36, 2, 2, 16, 16, 86, 42, 2, 21),
      SymbolEntry.new(40, 40, 2, 2, 18, 18, 114, 48, 2, 24),
      SymbolEntry.new(44, 44, 2, 2, 20, 20, 144, 56, 4, 14),
      SymbolEntry.new(48, 48, 2, 2, 22, 22, 174, 68, 4, 17),
      SymbolEntry.new(52, 52, 2, 2, 24, 24, 204, 84, 4, 21),
      SymbolEntry.new(64, 64, 4, 4, 14, 14, 280, 112, 4, 28),
      SymbolEntry.new(72, 72, 4, 4, 16, 16, 368, 144, 4, 36),
      SymbolEntry.new(80, 80, 4, 4, 18, 18, 456, 192, 4, 48),
      SymbolEntry.new(88, 88, 4, 4, 20, 20, 576, 224, 4, 56),
      SymbolEntry.new(96, 96, 4, 4, 22, 22, 696, 272, 4, 68),
      SymbolEntry.new(104, 104, 4, 4, 24, 24, 816, 336, 6, 56),
      SymbolEntry.new(120, 120, 6, 6, 18, 18, 1050, 408, 6, 68),
      SymbolEntry.new(132, 132, 6, 6, 20, 20, 1304, 496, 8, 62),
      SymbolEntry.new(144, 144, 6, 6, 22, 22, 1558, 620, 10, 62)
    ].map(&:freeze).freeze

    # The 6 rectangular symbol sizes.
    RECT_SIZES = [
      SymbolEntry.new(8, 18, 1, 1, 6, 16, 5, 7, 1, 7),
      SymbolEntry.new(8, 32, 1, 2, 6, 14, 10, 11, 1, 11),
      SymbolEntry.new(12, 26, 1, 1, 10, 24, 16, 14, 1, 14),
      SymbolEntry.new(12, 36, 1, 2, 10, 16, 22, 18, 1, 18),
      SymbolEntry.new(16, 36, 1, 2, 14, 16, 32, 24, 1, 24),
      SymbolEntry.new(16, 48, 1, 2, 14, 22, 49, 28, 1, 28)
    ].map(&:freeze).freeze

    # Maximum data codeword capacity — used in error messages.
    MAX_DATA_CW = 1558

    # =========================================================================
    # GF(256)/0x12D — exp and log tables built once at module load time
    # =========================================================================
    #
    # GF(256) is the field of 256 elements.  It is constructed as the
    # polynomial ring GF(2)[x] / p(x) where p(x) = 0x12D is the primitive
    # polynomial.
    #
    # The generator α = 2 (= x, the polynomial "x") is primitive: α^0, α^1,
    # …, α^254 enumerate all 255 non-zero field elements before cycling back.
    #
    # We build:
    #   GF_EXP[i] = α^i   for i in 0..254  (α^255 stored as GF_EXP[255] = 1)
    #   GF_LOG[v] = i      such that α^i = v  (GF_LOG[0] undefined / unused)
    #
    # Multiplication via log/antilog:  a × b = α^{(log[a] + log[b]) mod 255}
    # This is O(1) — two table lookups and one modular addition.

    GF_EXP = Array.new(256, 0)
    GF_LOG = Array.new(256, 0)

    # Build the tables: left-shift val (= multiply by α = x in poly form).
    # When bit 8 is set (val >= 256), XOR with 0x12D to reduce modulo p(x).
    gf_val = 1
    255.times do |i|
      GF_EXP[i] = gf_val
      GF_LOG[gf_val] = i
      gf_val <<= 1
      gf_val ^= GF256_PRIME if gf_val >= 256
    end
    # α^255 = α^0 = 1 (multiplicative group has order 255).
    GF_EXP[255] = GF_EXP[0]

    GF_EXP.freeze
    GF_LOG.freeze

    # =========================================================================
    # module_function — every method below callable as DataMatrix.foo(...)
    # =========================================================================
    module_function

    # -------------------------------------------------------------------------
    # gf_mul — multiply two GF(256)/0x12D elements via log/antilog tables.
    #
    # For a, b ≠ 0:   a × b = α^{(log[a] + log[b]) mod 255}.
    # If either operand is zero, the product is zero (zero absorbs under ×).
    #
    # GF(256) multiplication (unlike integer multiplication) cannot overflow —
    # every product is guaranteed to be in the range 0..255.
    # -------------------------------------------------------------------------
    def gf_mul(a, b)
      return 0 if a == 0 || b == 0

      GF_EXP[(GF_LOG[a] + GF_LOG[b]) % 255]
    end

    # =========================================================================
    # RS generator polynomial — b=1 convention (Data Matrix standard)
    # =========================================================================
    #
    # The RS generator for n_ecc ECC bytes is:
    #
    #   g(x) = (x + α^1)(x + α^2) ··· (x + α^{n_ecc})
    #
    # This is the b=1 convention — roots start at α^1, not α^0.
    #
    # Algorithm: start with g = [1], then for each i from 1 to n_ecc,
    # multiply g by the linear factor (x + α^i):
    #
    #   new_g[j]   ^= g[j]              (coeff × x term)
    #   new_g[j+1] ^= g[j] × α^i       (coeff × constant term)
    #
    # In GF(256), "–α^i" = "+α^i" because subtraction is XOR.
    # Format: highest-degree coefficient first, length = n_ecc + 1.
    #
    # Generators are cached in GEN_CACHE to avoid recomputation. The set
    # of distinct n_ecc values across all 30 symbols is small:
    # {5, 7, 10, 11, 12, 14, 17, 18, 21, 24, 28, 36, 42, 48, 56, 62, 68}

    # Generator polynomial cache — keyed by n_ecc.
    GEN_CACHE = {}
    private_constant :GEN_CACHE

    # build_generator(n_ecc) → frozen Array of n_ecc+1 GF(256) coefficients.
    def build_generator(n_ecc)
      cached = GEN_CACHE[n_ecc]
      return cached if cached

      g = [1]
      (1..n_ecc).each do |i|
        ai = GF_EXP[i]  # α^i
        new_g = Array.new(g.length + 1, 0)
        g.each_with_index do |coeff, j|
          new_g[j] ^= coeff
          new_g[j + 1] ^= gf_mul(coeff, ai)
        end
        g = new_g
      end

      result = g.freeze
      GEN_CACHE[n_ecc] = result
      result
    end

    # =========================================================================
    # Reed-Solomon encoder — LFSR shift-register method
    # =========================================================================
    #
    # Computes R(x) = D(x) · x^{n_ecc} mod G(x) over GF(256)/0x12D.
    # This is the "remainder" of dividing the data polynomial by the generator.
    #
    # For each input byte d:
    #   feedback = d XOR rem[0]
    #   shift rem left by one (drop rem[0], append 0)
    #   for i in 0..n_ecc-1:
    #     rem[i] ^= generator[i+1] × feedback
    #
    # After processing all data bytes, rem holds the n_ecc ECC bytes.
    # The XOR operations are GF(256) addition (identical to XOR in char-2 fields).

    # rs_encode_block(data, generator) → Array of ECC bytes.
    def rs_encode_block(data, generator)
      n_ecc = generator.length - 1
      rem = Array.new(n_ecc, 0)

      data.each do |d|
        fb = d ^ rem[0]
        # Shift register one position to the left.
        rem = rem[1..] + [0]
        next if fb == 0

        n_ecc.times do |i|
          rem[i] ^= gf_mul(generator[i + 1], fb)
        end
      end

      rem
    end

    # =========================================================================
    # ASCII encoding — ISO/IEC 16022:2006 §5.2.4
    # =========================================================================
    #
    # ASCII mode maps each character to one or two codewords:
    #
    #   Two consecutive ASCII digits (0x30–0x39) → one codeword:
    #     130 + (d1 × 10 + d2)
    #     e.g. "12" → 130 + 12 = 142, "00" → 130, "99" → 229
    #     This "digit pair compaction" halves the codeword budget for
    #     numeric strings — critical for manufacturing serial numbers.
    #
    #   Single ASCII char (0–127) → one codeword = char + 1.
    #     e.g. 'A' (65) → 66, space (32) → 33.
    #     The +1 shift exists because codeword 0 is reserved as "end of data".
    #
    #   Extended ASCII (128–255) → two codewords: 235 (UPPER_SHIFT), then
    #     (char - 127).  Enables Latin-1 / Windows-1252 but rarely needed.
    #
    # Truth table:
    #   "A"    → [66]         (65 + 1)
    #   " "    → [33]         (32 + 1)
    #   "12"   → [142]        (130 + 12, digit pair)
    #   "1234" → [142, 174]   (two digit pairs)
    #   "1A"   → [50, 66]     ('1' alone because next char is not a digit)
    #   "\xFF" → [235, 128]   (UPPER_SHIFT, 255 - 127)

    # encode_ascii(input_bytes) → Array of codeword integers.
    def encode_ascii(input_bytes)
      codewords = []
      i = 0
      n = input_bytes.length

      while i < n
        c = input_bytes[i]
        # Digit pair: both current and next bytes are ASCII digits (0x30–0x39).
        if c.between?(0x30, 0x39) && i + 1 < n &&
            input_bytes[i + 1].between?(0x30, 0x39)
          d1 = c - 0x30
          d2 = input_bytes[i + 1] - 0x30
          codewords << (130 + d1 * 10 + d2)
          i += 2
        elsif c <= 127
          codewords << (c + 1)
          i += 1
        else
          # Extended ASCII: UPPER_SHIFT (235) then (value - 127).
          codewords << 235
          codewords << (c - 127)
          i += 1
        end
      end

      codewords
    end

    # =========================================================================
    # Pad codewords — ISO/IEC 16022:2006 §5.2.3
    # =========================================================================
    #
    # After ASCII encoding, we must pad the codeword list to exactly data_cw
    # bytes so the Utah placement algorithm fills all available modules.
    #
    # Padding rules:
    #   1. The first pad codeword is always the literal value 129 ("EOM").
    #   2. Subsequent pads use a *scrambled* value based on 1-indexed position k
    #      within the full stream:
    #         scrambled = 129 + (149 × k mod 253) + 1
    #         if scrambled > 254: scrambled -= 254
    #
    # The scrambling prevents a long run of "129 129 129 …" from creating a
    # degenerate placement pattern in the Utah algorithm — identical codewords
    # would cluster related modules and bias the error-correction structure.
    #
    # Worked example — encode "A" into 10×10 (data_cw = 3):
    #   Input codewords:  [66]  ('A' + 1)
    #   k=2: first pad  = 129
    #   k=3: scrambled  = 129 + (149×3 mod 253) + 1 = 129 + 194 + 1 = 324
    #                     324 > 254 → 324 - 254 = 70
    #   Padded result:    [66, 129, 70]

    # pad_codewords(codewords, data_cw) → Array padded to exactly data_cw bytes.
    def pad_codewords(codewords, data_cw)
      padded = codewords.dup
      first_pad = true
      k = codewords.length + 1  # 1-indexed position of the first pad byte

      while padded.length < data_cw
        if first_pad
          padded << 129
          first_pad = false
        else
          scrambled = 129 + (149 * k) % 253 + 1
          scrambled -= 254 if scrambled > 254
          padded << scrambled
        end
        k += 1
      end

      padded
    end

    # =========================================================================
    # Symbol selection
    # =========================================================================
    #
    # Find the smallest symbol whose data_cw capacity >= codeword_count.
    # Candidates are sorted by capacity ascending, ties broken by area.
    #
    # shape controls which symbol shapes are considered:
    #   :square      — only the 24 square symbols (10×10 … 144×144)
    #   :rectangle   — only the 6 rectangular symbols (8×18 … 16×48)
    #   :any         — both shapes, picks smallest total area

    # select_symbol(codeword_count, shape) → SymbolEntry.
    def select_symbol(codeword_count, shape)
      candidates = case shape
      when :rectangle
        RECT_SIZES.to_a
      when :any
        SQUARE_SIZES.to_a + RECT_SIZES.to_a
      else
        # :square (default)
        SQUARE_SIZES.to_a
      end

      # Sort ascending by capacity, break ties by area.
      candidates = candidates.sort_by { |e| [e.data_cw, e.symbol_rows * e.symbol_cols] }

      entry = candidates.find { |e| e.data_cw >= codeword_count }
      if entry.nil?
        raise InputTooLongError,
          "DataMatrix: input too long — encoded #{codeword_count} codewords, " \
          "maximum is #{MAX_DATA_CW} (144×144 symbol)."
      end

      entry
    end

    # =========================================================================
    # Block splitting, ECC computation, and interleaving
    # =========================================================================
    #
    # For burst-error resilience, larger Data Matrix symbols split the data
    # across multiple Reed-Solomon blocks and then interleave the results.
    #
    # Block splitting
    # ---------------
    # Given num_blocks and total data_cw:
    #   base_len     = data_cw / num_blocks        (integer division)
    #   extra_blocks = data_cw % num_blocks
    #   Blocks 0..extra_blocks-1   receive (base_len + 1) data bytes.
    #   Blocks extra_blocks..end-1 receive  base_len      data bytes.
    #
    # Earlier blocks receive the extra byte when data_cw is not evenly
    # divisible — this is the standard ISO interleaving convention.
    #
    # Interleaving
    # ------------
    # Data interleaved first (round-robin across blocks), then ECC:
    #
    #   for pos in 0..max_data_per_block-1:
    #     for blk in 0..num_blocks-1:
    #       emit data[blk][pos] if pos < len(data[blk])
    #   for pos in 0..ecc_per_block-1:
    #     for blk in 0..num_blocks-1:
    #       emit ecc[blk][pos]
    #
    # Interleaving means a physical scratch of N modules hits at most
    # ceil(N / num_blocks) codewords in any one RS block — far more likely
    # to be within the block's correction capacity.

    # compute_interleaved(data, entry) → flat Array of data+ECC codewords.
    def compute_interleaved(data, entry)
      num_blocks = entry.num_blocks
      ecc_per_block = entry.ecc_per_block
      data_cw = entry.data_cw
      gen = build_generator(ecc_per_block)

      # ── Split data into blocks ──────────────────────────────────────────────
      base_len = data_cw / num_blocks
      extra_blocks = data_cw % num_blocks

      data_blocks = []
      offset = 0
      num_blocks.times do |b|
        l = (b < extra_blocks) ? base_len + 1 : base_len
        data_blocks << data[offset, l]
        offset += l
      end

      # ── Compute ECC for each block ──────────────────────────────────────────
      ecc_blocks = data_blocks.map { |blk| rs_encode_block(blk, gen) }

      # ── Interleave data round-robin ─────────────────────────────────────────
      interleaved = []
      max_data_len = data_blocks.map(&:length).max
      max_data_len.times do |pos|
        num_blocks.times do |b|
          interleaved << data_blocks[b][pos] if pos < data_blocks[b].length
        end
      end

      # ── Interleave ECC round-robin ──────────────────────────────────────────
      ecc_per_block.times do |pos|
        num_blocks.times do |b|
          interleaved << ecc_blocks[b][pos]
        end
      end

      interleaved
    end

    # =========================================================================
    # Grid initialization — finder + timing + alignment borders
    # =========================================================================
    #
    # Every Data Matrix symbol has an outer "finder + clock" border:
    #
    #   Left column  (col 0):   ALL DARK  — vertical leg of the L-finder.
    #   Bottom row   (row R-1): ALL DARK  — horizontal leg of the L-finder.
    #   Top row      (row 0):   ALTERNATING dark/light starting dark — timing.
    #   Right column (col C-1): ALTERNATING dark/light starting dark — timing.
    #
    # The solid L-shape tells a scanner where the symbol starts and what
    # orientation it is in.  The alternating timing edges on the opposite two
    # sides distinguish all four 90° rotations unambiguously.
    #
    # Multi-region symbols (those with region_rows × region_cols > 1) have
    # additional 2-module alignment borders separating adjacent data regions.
    # Each alignment border consists of:
    #   Row/col 0 of the AB: ALL DARK
    #   Row/col 1 of the AB: ALTERNATING dark/light starting dark
    #
    # Writing order is critical at corner intersections:
    #   1. Alignment borders first  (so the outer border can override)
    #   2. Top row timing
    #   3. Right column timing
    #   4. Left column L-finder     (overrides timing at corner (0,0))
    #   5. Bottom row L-finder LAST (overrides everything at all four corners)

    # init_grid(entry) → 2D Array<Array<Boolean>> (rows × cols, all false initially).
    def init_grid(entry)
      r_size = entry.symbol_rows
      c_size = entry.symbol_cols

      grid = Array.new(r_size) { Array.new(c_size, false) }

      # ── Alignment borders (multi-region symbols only) ───────────────────────
      (entry.region_rows - 1).times do |rr|
        # Physical row of the first AB row after data region rr+1:
        # 1 (outer border) + (rr+1)*(data_region_height) + rr*2 (prev ABs)
        ab_row0 = 1 + (rr + 1) * entry.data_region_height + rr * 2
        ab_row1 = ab_row0 + 1
        c_size.times do |c|
          grid[ab_row0][c] = true          # solid dark row
          grid[ab_row1][c] = (c % 2 == 0) # alternating, starts dark
        end
      end

      (entry.region_cols - 1).times do |rc|
        ab_col0 = 1 + (rc + 1) * entry.data_region_width + rc * 2
        ab_col1 = ab_col0 + 1
        r_size.times do |r|
          grid[r][ab_col0] = true          # solid dark col
          grid[r][ab_col1] = (r % 2 == 0) # alternating, starts dark
        end
      end

      # ── Top row: timing clock — alternating dark/light starting dark ────────
      c_size.times { |c| grid[0][c] = (c % 2 == 0) }

      # ── Right column: timing clock — alternating, starts dark ───────────────
      r_size.times { |r| grid[r][c_size - 1] = (r % 2 == 0) }

      # ── Left column: L-finder left leg — all dark ───────────────────────────
      r_size.times { |r| grid[r][0] = true }

      # ── Bottom row: L-finder bottom leg — all dark (written LAST) ───────────
      c_size.times { |c| grid[r_size - 1][c] = true }

      grid
    end

    # =========================================================================
    # Utah placement algorithm
    # =========================================================================
    #
    # The Utah placement algorithm is the most distinctive part of Data Matrix
    # encoding.  Its name comes from the 8-module codeword shape, which
    # resembles the US state of Utah — a rectangle with a notch cut from its
    # top-left corner.
    #
    # The algorithm scans the *logical* grid (all data region interiors
    # concatenated) in a diagonal zigzag.  For each codeword, 8 bits are
    # placed at 8 fixed offsets relative to the current reference position.
    #
    # The reference starts at (4, 0) and alternates between:
    #   Upward-right legs:   move row -= 2, col += 2 each step
    #   Downward-left legs:  move row += 2, col -= 2 each step
    #
    # Four special "corner" patterns fire when the reference is at or near
    # specific boundary positions.  There is NO masking step (unlike QR Code).
    # The diagonal traversal naturally distributes bits across the symbol.
    #
    # Standard Utah shape at reference (row, col):
    #
    #              col-2   col-1    col
    #   row-2 :     .     [bit1]  [bit2]
    #   row-1 :   [bit3]  [bit4]  [bit5]
    #   row   :   [bit6]  [bit7]  [bit8]
    #
    # Bit 8 is the MSB (placed at row, col); bit 1 is the LSB (placed at
    # row-2, col-1).

    # apply_wrap(row, col, n_rows, n_cols) → [row, col] after boundary rules.
    #
    # The four wrap rules from ISO/IEC 16022:2006 Annex F, applied in order:
    #   1. row < 0 AND col == 0        → (1, 3)         top-left singularity
    #   2. row < 0 AND col == n_cols   → (0, col-2)     wrapped past right
    #   3. row < 0                     → (row+n_rows, col-4)  top→bottom
    #   4. col < 0                     → (row-4, col+n_cols)  left→right
    def apply_wrap(row, col, n_rows, n_cols)
      return [1, 3] if row < 0 && col == 0
      return [0, col - 2] if row < 0 && col == n_cols
      return [row + n_rows, col - 4] if row < 0
      return [row - 4, col + n_cols] if col < 0

      [row, col]
    end

    # place_utah — place one codeword's 8 bits in the standard Utah shape.
    def place_utah(cw, row, col, n_rows, n_cols, grid, used)
      placements = [
        [row, col, 7],  # bit 8 (MSB)
        [row, col - 1, 6],  # bit 7
        [row, col - 2, 5],  # bit 6
        [row - 1, col, 4],  # bit 5
        [row - 1, col - 1, 3],  # bit 4
        [row - 1, col - 2, 2],  # bit 3
        [row - 2, col, 1],  # bit 2
        [row - 2, col - 1, 0]   # bit 1 (LSB)
      ]
      placements.each do |raw_r, raw_c, bit|
        r, c = apply_wrap(raw_r, raw_c, n_rows, n_cols)
        next unless r >= 0 && r < n_rows && c >= 0 && c < n_cols && !used[r][c]

        grid[r][c] = ((cw >> bit) & 1) == 1
        used[r][c] = true
      end
    end

    # place_with_positions — place a codeword at explicit (row, col, bit) positions.
    def place_with_positions(cw, positions, n_rows, n_cols, grid, used)
      positions.each do |r, c, bit|
        next unless r >= 0 && r < n_rows && c >= 0 && c < n_cols && !used[r][c]

        grid[r][c] = ((cw >> bit) & 1) == 1
        used[r][c] = true
      end
    end

    # Corner pattern 1 — triggered at reference (n_rows, 0) when n_rows or
    # n_cols ≡ 0 (mod 4). Handles the top-left boundary singularity.
    def place_corner1(cw, n_rows, n_cols, grid, used)
      positions = [
        [0, n_cols - 2, 7],
        [0, n_cols - 1, 6],
        [1, 0, 5],
        [2, 0, 4],
        [n_rows - 2, 0, 3],
        [n_rows - 1, 0, 2],
        [n_rows - 1, 1, 1],
        [n_rows - 1, 2, 0]
      ]
      place_with_positions(cw, positions, n_rows, n_cols, grid, used)
    end

    # Corner pattern 2 — triggered at reference (n_rows-2, 0) when
    # n_cols mod 4 ≠ 0. Handles the top-right boundary.
    def place_corner2(cw, n_rows, n_cols, grid, used)
      positions = [
        [0, n_cols - 2, 7],
        [0, n_cols - 1, 6],
        [1, n_cols - 1, 5],
        [2, n_cols - 1, 4],
        [n_rows - 1, 0, 3],
        [n_rows - 1, 1, 2],
        [n_rows - 1, 2, 1],
        [n_rows - 1, 3, 0]
      ]
      place_with_positions(cw, positions, n_rows, n_cols, grid, used)
    end

    # Corner pattern 3 — triggered at reference (n_rows-2, 0) when
    # n_cols mod 8 == 4. Handles the bottom-left boundary.
    def place_corner3(cw, n_rows, n_cols, grid, used)
      positions = [
        [0, n_cols - 1, 7],
        [1, 0, 6],
        [2, 0, 5],
        [n_rows - 2, 0, 4],
        [n_rows - 1, 0, 3],
        [n_rows - 1, 1, 2],
        [n_rows - 1, 2, 1],
        [n_rows - 1, 3, 0]
      ]
      place_with_positions(cw, positions, n_rows, n_cols, grid, used)
    end

    # Corner pattern 4 — triggered at reference (n_rows+4, 2) when
    # n_cols mod 8 == 0.
    def place_corner4(cw, n_rows, n_cols, grid, used)
      positions = [
        [n_rows - 3, n_cols - 1, 7],
        [n_rows - 2, n_cols - 1, 6],
        [n_rows - 1, n_cols - 3, 5],
        [n_rows - 1, n_cols - 2, 4],
        [n_rows - 1, n_cols - 1, 3],
        [0, 0, 2],
        [1, 0, 1],
        [2, 0, 0]
      ]
      place_with_positions(cw, positions, n_rows, n_cols, grid, used)
    end

    # utah_placement(codewords, n_rows, n_cols) → 2D boolean grid.
    #
    # Runs the full Utah diagonal placement algorithm on the logical data
    # matrix (all data regions concatenated as a flat grid of n_rows × n_cols).
    #
    # After placement, any modules not visited by the diagonal walk receive
    # the ISO "fill" value: (r + c) mod 2 == 1 (dark), per §10.
    def utah_placement(codewords, n_rows, n_cols)
      grid = Array.new(n_rows) { Array.new(n_cols, false) }
      used = Array.new(n_rows) { Array.new(n_cols, false) }

      cw_idx = 0
      row = 4
      col = 0

      loop do
        # ── Corner special cases ───────────────────────────────────────────────
        if row == n_rows && col == 0 && (n_rows % 4 == 0 || n_cols % 4 == 0)
          if cw_idx < codewords.length
            place_corner1(codewords[cw_idx], n_rows, n_cols, grid, used)
            cw_idx += 1
          end
        end
        if row == n_rows - 2 && col == 0 && n_cols % 4 != 0
          if cw_idx < codewords.length
            place_corner2(codewords[cw_idx], n_rows, n_cols, grid, used)
            cw_idx += 1
          end
        end
        if row == n_rows - 2 && col == 0 && n_cols % 8 == 4
          if cw_idx < codewords.length
            place_corner3(codewords[cw_idx], n_rows, n_cols, grid, used)
            cw_idx += 1
          end
        end
        if row == n_rows + 4 && col == 2 && n_cols % 8 == 0
          if cw_idx < codewords.length
            place_corner4(codewords[cw_idx], n_rows, n_cols, grid, used)
            cw_idx += 1
          end
        end

        # ── Upward-right diagonal leg (row -= 2, col += 2) ────────────────────
        loop do
          if row >= 0 && row < n_rows && col >= 0 && col < n_cols &&
              !used[row][col] && cw_idx < codewords.length
            place_utah(codewords[cw_idx], row, col, n_rows, n_cols, grid, used)
            cw_idx += 1
          end
          row -= 2
          col += 2
          break if row < 0 || col >= n_cols
        end

        # Step to next diagonal start.
        row += 1
        col += 3

        # ── Downward-left diagonal leg (row += 2, col -= 2) ───────────────────
        loop do
          if row >= 0 && row < n_rows && col >= 0 && col < n_cols &&
              !used[row][col] && cw_idx < codewords.length
            place_utah(codewords[cw_idx], row, col, n_rows, n_cols, grid, used)
            cw_idx += 1
          end
          row += 2
          col -= 2
          break if row >= n_rows || col < 0
        end

        # Step to next diagonal start.
        row += 3
        col += 1

        # ── Termination ────────────────────────────────────────────────────────
        break if row >= n_rows && col >= n_cols
        break if cw_idx >= codewords.length
      end

      # ── Fill remaining unvisited modules (ISO §10 right-and-bottom fill) ────
      # Some symbol sizes have residual modules the diagonal walk never reaches.
      # ISO/IEC 16022 §10 specifies (r+c) mod 2 == 1 (dark) for these.
      n_rows.times do |r|
        n_cols.times do |c|
          grid[r][c] = (r + c) % 2 == 1 unless used[r][c]
        end
      end

      grid
    end

    # =========================================================================
    # Logical → physical coordinate mapping
    # =========================================================================
    #
    # Utah placement works in the "logical" space — all data regions stitched
    # together as one flat grid of (region_rows × data_region_height) rows by
    # (region_cols × data_region_width) cols.
    #
    # After placement we map each logical (r, c) back to its physical location
    # in the full symbol, which includes:
    #   - 1-module outer border on all four sides.
    #   - 2-module alignment borders between adjacent data regions.
    #
    # For data region size rh × rw:
    #   phys_row = floor(r / rh) × (rh + 2) + (r mod rh) + 1
    #   phys_col = floor(c / rw) × (rw + 2) + (c mod rw) + 1
    #
    # The "(+2)" accounts for the 2-module alignment border between regions.
    # The "(+1)" accounts for the 1-module outer border.
    # For single-region symbols this simplifies to phys = logical + 1.

    # logical_to_physical(r, c, entry) → [phys_row, phys_col].
    def logical_to_physical(r, c, entry)
      rh = entry.data_region_height
      rw = entry.data_region_width
      phys_row = (r / rh) * (rh + 2) + (r % rh) + 1
      phys_col = (c / rw) * (rw + 2) + (c % rw) + 1
      [phys_row, phys_col]
    end

    # =========================================================================
    # find_entry_by_size — look up a SymbolEntry by explicit (rows, cols)
    # =========================================================================

    # find_entry_by_size(rows, cols) → SymbolEntry or raise InvalidSizeError.
    def find_entry_by_size(rows, cols)
      all = SQUARE_SIZES + RECT_SIZES
      entry = all.find { |e| e.symbol_rows == rows && e.symbol_cols == cols }
      if entry.nil?
        raise InvalidSizeError,
          "DataMatrix: #{rows}×#{cols} is not a valid ECC200 symbol size. " \
          "Square sizes: 10×10, 12×12, …, 144×144. " \
          "Rect sizes: 8×18, 8×32, 12×26, 12×36, 16×36, 16×48."
      end
      entry
    end

    # =========================================================================
    # Public API: encode
    # =========================================================================
    #
    # encode(data, opts = {}) → ModuleGrid
    #
    # Encode a string into a Data Matrix ECC200 ModuleGrid.
    #
    # Pipeline:
    #   1. ASCII-encode the input (with digit-pair compression).
    #   2. Select the smallest fitting symbol (or use opts[:size]).
    #   3. Pad to data capacity with ECC200 scrambled-pad sequence.
    #   4. Compute RS ECC for each block over GF(256)/0x12D.
    #   5. Interleave data + ECC blocks round-robin.
    #   6. Initialize the physical grid (finder + timing + alignment borders).
    #   7. Run Utah diagonal placement on the logical data matrix.
    #   8. Map logical → physical coordinates.
    #   9. Return a ModuleGrid struct.
    #
    # Parameters:
    #   data — String. Encoded as UTF-8.
    #   opts — Hash:
    #     size:  [rows, cols] to force a specific symbol size.
    #            Raises InvalidSizeError if the size is not one of the 30 ECC200
    #            sizes.  Raises InputTooLongError if the input does not fit.
    #     shape: :square (default), :rectangle, or :any.
    #            Ignored when size: is provided.
    #
    # Returns:
    #   ModuleGrid struct with .rows, .cols, .modules (Array<Array<Boolean>>).
    #
    # Raises:
    #   InputTooLongError  if data exceeds max symbol capacity.
    #   InvalidSizeError   if size: is given but does not match any ECC200 size.

    def encode(data, opts = {})
      # ── Step 1: ASCII encode ─────────────────────────────────────────────────
      input_bytes = data.encode("UTF-8").b.bytes
      codewords = encode_ascii(input_bytes)

      # ── Step 2: Select symbol ────────────────────────────────────────────────
      size_opt = opts[:size]
      if size_opt
        entry = find_entry_by_size(size_opt[0], size_opt[1])
        if codewords.length > entry.data_cw
          raise InputTooLongError,
            "DataMatrix: input encodes to #{codewords.length} codewords " \
            "but #{entry.symbol_rows}×#{entry.symbol_cols} holds only #{entry.data_cw}."
        end
      else
        shape = opts[:shape] || :square
        entry = select_symbol(codewords.length, shape)
      end

      # ── Step 3: Pad to data capacity ─────────────────────────────────────────
      padded = pad_codewords(codewords, entry.data_cw)

      # ── Steps 4–5: Compute ECC and interleave ────────────────────────────────
      interleaved = compute_interleaved(padded, entry)

      # ── Step 6: Initialize physical grid ─────────────────────────────────────
      phys_grid = init_grid(entry)

      # ── Step 7: Run Utah placement on the logical grid ───────────────────────
      n_rows = entry.region_rows * entry.data_region_height
      n_cols = entry.region_cols * entry.data_region_width
      logical_grid = utah_placement(interleaved, n_rows, n_cols)

      # ── Step 8: Map logical → physical ───────────────────────────────────────
      n_rows.times do |r|
        n_cols.times do |c|
          pr, pc = logical_to_physical(r, c, entry)
          phys_grid[pr][pc] = logical_grid[r][c]
        end
      end

      # ── Step 9: Wrap in ModuleGrid and return ─────────────────────────────────
      ModuleGrid.new(entry.symbol_rows, entry.symbol_cols, phys_grid)
    end

    # =========================================================================
    # Public API: encode_and_layout
    # =========================================================================
    #
    # encode_and_layout(data, opts = {}) → { grid:, scene: nil }
    #
    # Convenience: encode + layout placeholder. Returns a hash with :grid and
    # :scene keys. Scene is nil in v0.1.0 (no paint_instructions dependency).
    # This signature mirrors the Python API for cross-language parity tests.

    def encode_and_layout(data, opts = {})
      grid = encode(data, opts)
      {grid: grid, scene: nil} # rubocop:disable Style/HashLiteralBraces -- standardrb wants spaces
    end

    # =========================================================================
    # Public API: grid_to_string — debug/test utility
    # =========================================================================
    #
    # Renders a ModuleGrid as a multiline "0"/"1" string for snapshot testing
    # and cross-language corpus comparison. Each row is one line; no trailing
    # newline.
    #
    # Example output (10×10 encoding of "A"):
    #   1010101010
    #   1100000001
    #   ...
    #   1111111111

    def grid_to_string(grid)
      grid.modules.map do |row|
        row.map { |dark| dark ? "1" : "0" }.join
      end.join("\n")
    end
  end
end
