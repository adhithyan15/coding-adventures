# frozen_string_literal: true

# =============================================================================
# coding_adventures/pdf417 — PDF417 stacked linear barcode encoder
#                            ISO/IEC 15438:2015 compliant
# =============================================================================
#
# PDF417 (Portable Data File 417) was invented by Ynjiun P. Wang at Symbol
# Technologies in 1991. The name encodes its own geometry:
#
#   - Every codeword has **4 bars** and **4 spaces** (8 elements).
#   - Every codeword occupies exactly **17 modules** of horizontal space.
#   - 4 + (1 implied "of each") + 7 = "417".
#
# ## Where PDF417 appears in the real world
#
#   - **AAMVA**         — North American driver's licences and government IDs.
#   - **IATA BCBP**     — Airline boarding passes (the long thin barcode you
#                         scan at the gate alongside your Aztec Code).
#   - **USPS**          — Domestic shipping labels.
#   - **US immigration**— Form I-94, customs declarations.
#   - **Healthcare**    — Patient wristbands, medication labels.
#
# ## Encoding pipeline
#
#   raw bytes (string or byte array)
#     → byte compaction      (codeword 924 latch + 6-bytes-to-5-codewords in base 900)
#     → length descriptor    (codeword 0 = total codewords in the symbol)
#     → RS ECC               (GF(929) Reed-Solomon, b=3 convention, α=3)
#     → dimension selection  (auto: aim for a roughly square symbol)
#     → padding              (codeword 900 fills unused codeword slots)
#     → row indicators       (LRI + RRI per row encode row#, total rows, cols, ECC level)
#     → cluster table lookup (codeword → 17-module bar/space packed pattern)
#     → start / stop patterns (fixed bit patterns at each row's left and right edge)
#     → ModuleGrid           (2D boolean array: true = dark module)
#
# ## v0.1.0 scope — byte compaction only
#
# This release encodes every input byte directly via byte-compaction mode.
# Text and numeric compaction (which would produce shorter codeword sequences
# for ASCII text and digit strings respectively) are planned for v0.2.0.
# Byte mode handles arbitrary binary content correctly, so it is the safe
# default for all general-purpose encoding.
#
# ## Quick start
#
#   require "coding_adventures/pdf417"
#   grid = CodingAdventures::PDF417.encode("HELLO WORLD")
#   grid.rows    # integer — height of the symbol in modules
#   grid.cols    # integer — width  of the symbol in modules
#   grid.modules # Array<Array<Boolean>> — true = dark, 0-indexed [row][col]

require_relative "pdf417/version"
require_relative "pdf417/errors"
require_relative "pdf417/cluster_tables"

module CodingAdventures
  # ===========================================================================
  # PDF417 — top-level encoding namespace
  # ===========================================================================
  #
  # All public surface lives here as `module_function` methods so they can be
  # called both as `CodingAdventures::PDF417.encode(...)` (the normal external
  # path) and as bare `encode(...)` from inside the module (useful in tests).
  module PDF417
    # =========================================================================
    # Public constants
    # =========================================================================
    #
    # Exposing these lets test code verify that the encoder uses the values the
    # standard mandates without white-box-testing the internals.

    GF929_PRIME = 929     # The prime modulus of GF(929).
    GF929_ALPHA = 3       # Primitive root of GF(929) — the generator element.
    GF929_ORDER = 928     # |GF(929)*| = PRIME - 1 — the multiplicative group order.

    LATCH_BYTE  = 924     # Byte-compaction latch codeword.
    PADDING_CW  = 900     # Neutral padding codeword (fills unused grid slots).

    MIN_ROWS = 3          # PDF417 standard minimum row count.
    MAX_ROWS = 90         # PDF417 standard maximum row count.
    MIN_COLS = 1          # PDF417 standard minimum data column count.
    MAX_COLS = 30         # PDF417 standard maximum data column count.

    # START_PATTERN — the fixed bar/space width sequence that begins every row.
    #
    # Eight elements (alternating bar then space): [8,1,1,1,1,1,1,3]
    # expands to 17 modules total:
    #   11111111 0 1 0 1 0 000  →  dark dark dark dark dark dark dark dark
    #                               light dark light dark light dark dark dark
    # Every compliant PDF417 scanner recognises this exact pattern as a row
    # start guard.
    START_PATTERN = [8, 1, 1, 1, 1, 1, 1, 3].freeze

    # STOP_PATTERN — the fixed bar/space width sequence that ends every row.
    #
    # Nine elements: [7,1,1,3,1,1,1,2,1] expands to 18 modules.
    # The extra module (18 vs. 17) makes the stop pattern asymmetric and
    # therefore unambiguous to scanners reading from right to left.
    STOP_PATTERN = [7, 1, 1, 3, 1, 1, 1, 2, 1].freeze

    # Output structure — a plain Struct so callers get named fields without
    # having to depend on barcode_2d (PDF417 is self-contained in this gem).
    #
    # Fields:
    #   rows    — Integer: module height of the complete symbol
    #   cols    — Integer: module width  of the complete symbol
    #   modules — Array<Array<Boolean>>: modules[r][c] is true if dark
    ModuleGrid = Struct.new(:rows, :cols, :modules)

    # =========================================================================
    # GF(929) arithmetic — exp/log tables built once at module load time
    # =========================================================================
    #
    # GF(929) is the integers modulo 929. Since 929 is prime, every non-zero
    # element has a multiplicative inverse, making it a proper finite field.
    #
    # We use log/antilog lookup tables for O(1) multiplication:
    #
    #   GF_EXP[i] = α^i       for i ∈ 0..927, wrapped at index 928 = index 0
    #   GF_LOG[v] = i          such that α^i = v  for v ∈ 1..928
    #
    # Building the tables takes ~0.1 ms and costs ~7 KB of memory. Both are
    # frozen arrays so Ruby's object model prevents accidental mutation.

    GF_EXP = Array.new(GF929_ORDER + 1, 0)
    GF_LOG = Array.new(GF929_PRIME,   0)

    # Iterate α^0, α^1, … α^{ORDER-1}. Starting at val = 1 = α^0, each step
    # multiplies by α = 3 (mod 929). Because α is primitive, every non-zero
    # element of GF(929) appears exactly once in this sequence.
    gf_init_val = 1
    GF929_ORDER.times do |i|
      GF_EXP[i] = gf_init_val
      GF_LOG[gf_init_val] = i
      gf_init_val = (gf_init_val * GF929_ALPHA) % GF929_PRIME
    end
    # Store α^ORDER = α^0 = 1 at index ORDER so gf_mul can index with
    # `(la + lb) % ORDER` without a separate bounds check.
    GF_EXP[GF929_ORDER] = GF_EXP[0]

    GF_EXP.freeze
    GF_LOG.freeze

    # =========================================================================
    # module_function — every method below is callable as PDF417.foo(...)
    # =========================================================================
    module_function

    # -------------------------------------------------------------------------
    # gf_mul — multiply two elements of GF(929) via log/antilog tables.
    #
    # For non-zero a, b:  a * b = α^{ (log a + log b) mod ORDER }.
    # If either operand is zero, the product is zero (zero absorbs under ×).
    #
    # This three-line implementation runs in O(1) because both lookups are
    # simple array accesses — no loops, no modular exponentiation.
    # -------------------------------------------------------------------------
    def gf_mul(a, b)
      return 0 if a == 0 || b == 0

      GF_EXP[(GF_LOG[a] + GF_LOG[b]) % GF929_ORDER]
    end

    # -------------------------------------------------------------------------
    # gf_add — add two elements of GF(929) (ordinary integer addition mod 929).
    #
    # Unlike GF(2^n) where addition is XOR, GF(prime) uses ordinary modular
    # arithmetic.  There is no log-table shortcut for addition.
    # -------------------------------------------------------------------------
    def gf_add(a, b)
      (a + b) % GF929_PRIME
    end

    # =========================================================================
    # Reed-Solomon generator polynomial — b=3 convention
    # =========================================================================
    #
    # For ECC level L we generate k = 2^(L+1) ECC codewords. The PDF417
    # specification uses the "b=3 convention": the roots of the generator
    # polynomial are α^3, α^4, …, α^{k+2}.
    #
    # (QR Code uses b=0; Data Matrix uses b=1; PDF417 uses b=3 — the "b"
    # value is a design choice baked into every Reed-Solomon variant's spec.)
    #
    # We construct g(x) iteratively:
    #
    #   g(x) = 1
    #   for j = 3 to k+2:
    #     g(x) = g(x) * (x - α^j)
    #
    # After k iterations g has k+1 coefficients stored big-endian:
    # [g_k, g_{k-1}, …, g_1, g_0] with g_k = 1 (monic polynomial).

    # build_generator(ecc_level) → Array of k+1 GF(929) coefficients.
    def build_generator(ecc_level)
      k = 1 << (ecc_level + 1)   # 2^(ecc_level+1)

      g = [1]                     # g(x) = 1 — degree 0, coefficient = 1

      (3..(k + 2)).each do |j|
        root      = GF_EXP[j % GF929_ORDER]                # α^j
        neg_root  = (GF929_PRIME - root) % GF929_PRIME     # -α^j  in GF(929)

        # Multiply g(x) by (x + neg_root) — "+neg_root" is the same as
        # "-root" in GF(929) because -v ≡ (929 - v) (mod 929).
        new_g = Array.new(g.length + 1, 0)
        g.each_with_index do |coeff, i|
          new_g[i]     = gf_add(new_g[i],     coeff)
          new_g[i + 1] = gf_add(new_g[i + 1], gf_mul(coeff, neg_root))
        end
        g = new_g
      end

      g
    end

    # =========================================================================
    # Reed-Solomon encoder — shift-register (LFSR) polynomial division
    # =========================================================================
    #
    # Standard RS encoding via polynomial long division. We feed the data
    # codewords one at a time into a shift register of length k (the ECC count).
    #
    # For each data codeword d:
    #
    #   feedback = (d + ecc[0]) mod 929
    #   shift register left by one (ecc[0] ← ecc[1], …, ecc[k-2] ← ecc[k-1])
    #   ecc[k-1] ← 0
    #   for each register cell i: ecc[i] += g[k-i] * feedback  (mod 929)
    #
    # After processing all data, `ecc` holds the k ECC codewords.
    #
    # No interleaving is used in PDF417 (unlike QR Code). The row-cluster
    # structure distributes burst errors across multiple rows instead.

    # rs_encode(data, ecc_level) → Array of k ECC codewords.
    def rs_encode(data, ecc_level)
      g = build_generator(ecc_level)
      k = g.length - 1            # number of ECC codewords = degree of g

      ecc = Array.new(k, 0)

      data.each do |d|
        feedback = gf_add(d, ecc[0])
        # Shift the register one position to the left.
        (0...(k - 1)).each { |i| ecc[i] = ecc[i + 1] }
        ecc[k - 1] = 0
        # Accumulate feedback × each generator coefficient.
        # g is big-endian: g[0] = leading coefficient (degree k), g[k] = constant term.
        # Cell i of the register corresponds to generator coefficient g[k - i].
        k.times { |i| ecc[i] = gf_add(ecc[i], gf_mul(g[k - i], feedback)) }
      end

      ecc
    end

    # =========================================================================
    # Byte compaction
    # =========================================================================
    #
    # Byte compaction encodes arbitrary 8-bit data. Six input bytes pack into
    # five codewords by treating the bytes as a 48-bit big-endian unsigned
    # integer and expressing it in base 900:
    #
    #   n = b0·256^5 + b1·256^4 + b2·256^3 + b3·256^2 + b4·256 + b5
    #   codewords = digits(n, base = 900)   — exactly 5 digits, big-endian
    #
    # Lossless proof: 2^48 = 281,474,976,710,656 < 900^5 = 590,490,000,000,000.
    #
    # Remaining 1–5 bytes (the "tail") are emitted directly: each byte becomes
    # one codeword in the range 0..255. Decoders know whether a trailing group
    # was a full six-byte block or a tail by the symbol's total codeword count.
    #
    # Ruby integers are arbitrary-precision, so 48-bit arithmetic is exact with
    # no risk of overflow.

    # byte_compact(bytes) → Array of codewords starting with LATCH_BYTE (924).
    def byte_compact(bytes)
      cws = [LATCH_BYTE]
      i = 0
      n = bytes.length

      # Process complete 6-byte groups.
      while i + 6 <= n
        # Pack six bytes into a 48-bit integer.
        v = 0
        6.times { |j| v = v * 256 + bytes[i + j] }

        # Convert to five base-900 digits, most-significant first.
        group = Array.new(5, 0)
        4.downto(0) do |j|
          group[j] = v % 900
          v /= 900
        end

        cws.concat(group)
        i += 6
      end

      # Tail: 0–5 remaining bytes, each directly as a codeword.
      while i < n
        cws << bytes[i]
        i += 1
      end

      cws
    end

    # =========================================================================
    # Auto-selection of ECC level
    # =========================================================================
    #
    # These thresholds match the recommendation table from ISO/IEC 15438:2015.
    # The idea: pick a level whose ECC overhead is roughly proportional to the
    # data size so that small symbols stay compact and large symbols still
    # recover from realistic physical damage (scratches, smudges, partial
    # occlusion).
    #
    #   data codewords ≤  40  → level 2  (  8 ECC codewords)
    #   data codewords ≤ 160  → level 3  ( 16 ECC codewords)
    #   data codewords ≤ 320  → level 4  ( 32 ECC codewords)
    #   data codewords ≤ 863  → level 5  ( 64 ECC codewords)
    #   data codewords >  863 → level 6  (128 ECC codewords)

    # auto_ecc_level(data_count) → Integer in 2..6.
    def auto_ecc_level(data_count)
      return 2 if data_count <= 40
      return 3 if data_count <= 160
      return 4 if data_count <= 320
      return 5 if data_count <= 863

      6
    end

    # =========================================================================
    # Dimension selection — choose (rows, cols) for a given total codeword count
    # =========================================================================
    #
    # Heuristic: aim for a roughly *square* visual symbol. Because each PDF417
    # codeword is 17 modules wide and typically 3–4 modules tall (with the
    # default row_height = 3), a square symbol requires roughly three times as
    # many codewords per row as per column. Hence we take:
    #
    #   c = ceil(sqrt(total / 3))    — starting column estimate
    #   r = ceil(total / c)          — rows needed given that column count
    #
    # Both are clamped to the legal ranges [1..30] and [3..90].

    # ceil_div(a, b) → Integer — ceiling division for positive a, b.
    def ceil_div(a, b)
      (a + b - 1) / b
    end

    # clamp(v, lo, hi) → Integer — constrain v to [lo, hi].
    def clamp(v, lo, hi)
      return lo if v < lo
      return hi if v > hi

      v
    end

    # choose_dimensions(total) → [cols, rows]
    def choose_dimensions(total)
      c = clamp((Math.sqrt(total / 3.0)).ceil, MIN_COLS, MAX_COLS)
      r = [MIN_ROWS, ceil_div(total, c)].max

      # If the first estimate gives fewer than the minimum 3 rows (edge case
      # for very small payloads), force r = 3 and recompute c.
      if r < MIN_ROWS
        r = MIN_ROWS
        c = clamp(ceil_div(total, r), MIN_COLS, MAX_COLS)
        r = [MIN_ROWS, ceil_div(total, c)].max
      end

      r = [MAX_ROWS, r].min
      [c, r]
    end

    # =========================================================================
    # Row indicator computation
    # =========================================================================
    #
    # Each row carries a Left Row Indicator (LRI) and a Right Row Indicator
    # (RRI). Together they encode the symbol's full metadata so a scanner can
    # reconstruct R (row count), C (column count), and L (ECC level) from any
    # individual row, even if the other rows are damaged.
    #
    # Three intermediate values (all in 0..29):
    #
    #   R_info = floor((rows - 1) / 3)   — encodes total row count
    #   C_info = cols - 1                — encodes column count
    #   L_info = 3 * ecc_level + (rows - 1) mod 3  — encodes ECC + row-triplet remainder
    #
    # For row r (0-indexed), cluster = r mod 3, row_group = r div 3:
    #
    #   cluster 0: LRI = 30·row_group + R_info,   RRI = 30·row_group + C_info
    #   cluster 1: LRI = 30·row_group + L_info,   RRI = 30·row_group + R_info
    #   cluster 2: LRI = 30·row_group + C_info,   RRI = 30·row_group + L_info
    #
    # The "30·row_group" prefix encodes which group of three rows we are in.
    # R_info, C_info, L_info each fit in 0..29, so the sum stays within a
    # single 0..928 codeword.

    # compute_lri(r, rows, cols, ecc_level) → Integer codeword for LRI.
    def compute_lri(r, rows, cols, ecc_level)
      r_info    = (rows - 1) / 3
      c_info    = cols - 1
      l_info    = 3 * ecc_level + (rows - 1) % 3
      row_group = r / 3
      cluster   = r % 3

      case cluster
      when 0 then 30 * row_group + r_info
      when 1 then 30 * row_group + l_info
      else        30 * row_group + c_info
      end
    end

    # compute_rri(r, rows, cols, ecc_level) → Integer codeword for RRI.
    def compute_rri(r, rows, cols, ecc_level)
      r_info    = (rows - 1) / 3
      c_info    = cols - 1
      l_info    = 3 * ecc_level + (rows - 1) % 3
      row_group = r / 3
      cluster   = r % 3

      case cluster
      when 0 then 30 * row_group + c_info
      when 1 then 30 * row_group + r_info
      else        30 * row_group + l_info
      end
    end

    # =========================================================================
    # Pattern expansion — codeword packed integer → boolean module array
    # =========================================================================
    #
    # Every codeword in the CLUSTER_TABLES is stored as a packed 32-bit integer
    # with 4 bits per bar/space width, alternating bar then space, 8 elements:
    #
    #   bits 31..28 = b1, 27..24 = s1, 23..20 = b2, 19..16 = s2,
    #   bits 15..12 = b3, 11.. 8 = s3,  7.. 4 = b4,  3.. 0 = s4
    #
    # We expand this into a flat boolean slice: true = dark, false = light.
    # The total run length is always exactly 17 modules per codeword.
    #
    # Rather than allocating a new array per codeword, callers pass in a target
    # array to append into — this keeps the garbage collector happy for symbols
    # with many rows.

    # expand_pattern(packed, out) — append 17 modules from a packed codeword.
    def expand_pattern(packed, out)
      widths = [
        (packed >> 28) & 0xf,  # b1 (bar)
        (packed >> 24) & 0xf,  # s1 (space)
        (packed >> 20) & 0xf,  # b2
        (packed >> 16) & 0xf,  # s2
        (packed >> 12) & 0xf,  # b3
        (packed >>  8) & 0xf,  # s3
        (packed >>  4) & 0xf,  # b4
        (packed       ) & 0xf  # s4
      ]
      dark = true
      widths.each do |w|
        w.times { out << dark }
        dark = !dark
      end
    end

    # expand_widths(widths, out) — append modules from a plain width array.
    #
    # Used for START_PATTERN and STOP_PATTERN, which are not stored in the
    # cluster tables.
    def expand_widths(widths, out)
      dark = true
      widths.each do |w|
        w.times { out << dark }
        dark = !dark
      end
    end

    # =========================================================================
    # Rasterisation: codeword sequence → ModuleGrid
    # =========================================================================
    #
    # Each logical PDF417 row consists of:
    #
    #   start pattern (17) | LRI (17) | data × cols (17 each) | RRI (17) | stop (18)
    #
    # Total module width = 17 + 17 + 17·cols + 17 + 18 = 69 + 17·cols.
    #
    # Vertically, each logical row is repeated `row_height` times to give the
    # barcode physical height. The default row_height = 3 produces a symbol
    # that most flatbed scanners can reliably decode.
    #
    # The output modules grid is 0-indexed: modules[r][c] is true when the
    # module at row r, column c is dark (black).

    # rasterize(sequence, rows, cols, ecc_level, row_height) → ModuleGrid
    def rasterize(sequence, rows, cols, ecc_level, row_height)
      module_width  = 69 + 17 * cols
      module_height = rows * row_height

      # Allocate the full grid, all modules initially light (false).
      modules = Array.new(module_height) { Array.new(module_width, false) }

      # Pre-expand start and stop patterns — they are the same for every row.
      start_mods = []
      expand_widths(START_PATTERN, start_mods)
      stop_mods = []
      expand_widths(STOP_PATTERN, stop_mods)

      rows.times do |r|
        cluster       = r % 3
        cluster_table = CLUSTER_TABLES[cluster]  # 0-indexed in Ruby

        row_mods = []

        # 1. Start pattern (17 modules) — identical for every row.
        row_mods.concat(start_mods)

        # 2. Left Row Indicator (17 modules).
        #    The LRI is looked up in the cluster table just like a data codeword.
        lri = compute_lri(r, rows, cols, ecc_level)
        expand_pattern(cluster_table[lri], row_mods)

        # 3. Data codewords (17 modules each).
        #    sequence is a flat 0-indexed array; row r occupies positions
        #    r*cols .. (r+1)*cols-1.
        cols.times do |j|
          cw = sequence[r * cols + j]
          expand_pattern(cluster_table[cw], row_mods)
        end

        # 4. Right Row Indicator (17 modules).
        rri = compute_rri(r, rows, cols, ecc_level)
        expand_pattern(cluster_table[rri], row_mods)

        # 5. Stop pattern (18 modules).
        row_mods.concat(stop_mods)

        # Sanity check: every row must be exactly module_width modules wide.
        # A mismatch indicates a bug in the cluster table or an off-by-one
        # in the pipeline above.
        if row_mods.length != module_width
          raise PDF417Error,
            "PDF417 internal error: row #{r} has #{row_mods.length} modules, " \
            "expected #{module_width}"
        end

        # Write this logical row `row_height` times into the grid.
        base = r * row_height
        row_height.times do |h|
          target = modules[base + h]
          module_width.times do |c|
            target[c] = row_mods[c]
          end
        end
      end

      ModuleGrid.new(module_height, module_width, modules)
    end

    # =========================================================================
    # Public API: encode
    # =========================================================================
    #
    # encode(data, opts = {}) → ModuleGrid
    #
    # Encode arbitrary bytes as a PDF417 symbol and return the ModuleGrid.
    #
    # Parameters:
    #   data        — String or Array<Integer 0..255>.  Strings are treated as
    #                 raw bytes (each character's byte value is used directly).
    #   opts        — Hash of optional encoder parameters:
    #     ecc_level : Integer 0..8   — ECC level.  Default: auto-selected.
    #     columns   : Integer 1..30  — data column count.  Default: auto.
    #     row_height: Integer ≥ 1    — module rows per logical row. Default: 3.
    #
    # Returns:
    #   ModuleGrid struct with:
    #     .rows    — Integer module height of the complete symbol
    #     .cols    — Integer module width  of the complete symbol
    #     .modules — Array<Array<Boolean>>: modules[r][c] is true when dark
    #
    # Raises:
    #   InvalidECCLevelError     if ecc_level is outside 0..8
    #   InvalidDimensionsError   if columns is outside 1..30 or rows would exceed 90
    #   InputTooLongError        if data is too large for any valid PDF417 symbol
    #   ArgumentError            if data is neither a String nor an Array

    # to_byte_array — normalise the caller's data argument to Array<Integer>.
    def to_byte_array(data)
      case data
      when String
        data.b.bytes
      when Array
        data.each_with_index do |v, i|
          unless v.is_a?(Integer) && v >= 0 && v <= 255
            raise PDF417Error,
              "PDF417Error: data[#{i}] must be an integer in 0..255, got #{v.inspect}"
          end
        end
        data
      else
        raise ArgumentError,
          "encode: data must be a String or Array of bytes (got #{data.class})"
      end
    end

    def encode(data, opts = {})
      bytes = to_byte_array(data)

      # ── Validate ecc_level option ─────────────────────────────────────────
      ecc_level = opts[:ecc_level]
      if !ecc_level.nil? && !(ecc_level.is_a?(Integer) && ecc_level >= 0 && ecc_level <= 8)
        raise InvalidECCLevelError,
          "InvalidECCLevelError: ecc_level must be an integer in 0..8, " \
          "got #{ecc_level.inspect}"
      end

      # ── Byte compaction ────────────────────────────────────────────────────
      data_cwords = byte_compact(bytes)

      # ── Auto-select ECC level ──────────────────────────────────────────────
      # The "+1" accounts for the length descriptor that we prepend next.
      if ecc_level.nil?
        ecc_level = auto_ecc_level(data_cwords.length + 1)
      end
      ecc_count = 1 << (ecc_level + 1)   # 2^(ecc_level+1)

      # ── Length descriptor ──────────────────────────────────────────────────
      # The very first codeword counts itself + all data codewords + all ECC
      # codewords (but NOT the padding that fills the grid). Decoders use this
      # number to locate the boundary between data and ECC.
      length_desc = 1 + data_cwords.length + ecc_count

      # Build the full data sequence for RS encoding.
      full_data = [length_desc] + data_cwords

      # ── Reed-Solomon ECC ───────────────────────────────────────────────────
      ecc_cwords = rs_encode(full_data, ecc_level)

      # ── Choose symbol dimensions ────────────────────────────────────────────
      total = full_data.length + ecc_cwords.length

      columns_opt = opts[:columns]
      if columns_opt.nil?
        cols, rows = choose_dimensions(total)
      else
        unless columns_opt.is_a?(Integer) && columns_opt >= MIN_COLS && columns_opt <= MAX_COLS
          raise InvalidDimensionsError,
            "InvalidDimensionsError: columns must be an integer in #{MIN_COLS}..#{MAX_COLS}, " \
            "got #{columns_opt.inspect}"
        end
        cols = columns_opt
        rows = [MIN_ROWS, ceil_div(total, cols)].max
        if rows > MAX_ROWS
          raise InputTooLongError,
            "InputTooLongError: data requires #{rows} rows (max #{MAX_ROWS}) " \
            "with #{cols} columns"
        end
      end

      # Defence-in-depth: verify the grid can actually hold the codewords.
      if cols * rows < total
        raise InputTooLongError,
          "InputTooLongError: cannot fit #{total} codewords in #{rows}×#{cols} grid"
      end

      # ── Pad the data to fill the grid exactly ────────────────────────────
      padding_count = cols * rows - total
      padded = full_data + Array.new(padding_count, PADDING_CW)

      # Final codeword sequence: padded data then ECC codewords appended.
      sequence = padded + ecc_cwords

      # ── Validate row_height option ─────────────────────────────────────────
      row_height = opts[:row_height] || 3
      row_height = [1, row_height.to_i].max

      # ── Rasterise to boolean module grid ──────────────────────────────────
      rasterize(sequence, rows, cols, ecc_level, row_height)
    end
  end
end
