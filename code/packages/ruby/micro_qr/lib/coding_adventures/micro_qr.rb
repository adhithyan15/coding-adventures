# frozen_string_literal: true

# =============================================================================
# coding_adventures/micro_qr — Micro QR Code encoder
# =============================================================================
#
# Micro QR Code is the compact variant of QR Code, standardized in
# ISO/IEC 18004:2015 Annex E.  It was designed for applications where even
# the smallest regular QR Code (21×21 at version 1) is too large — think
# surface-mount component labels, circuit board markings, miniature industrial
# tags.
#
# The key structural difference from regular QR:
#
#   - ONE finder pattern (top-left only), not three.
#   - Timing patterns along ROW 0 and COL 0, not row 6 / col 6.
#   - Only 4 mask patterns (not 8).
#   - Format XOR mask 0x4445 (not 0x5412).
#   - Single copy of format information (not two).
#   - 2-module quiet zone (not 4).
#   - Narrower mode indicators (0–3 bits instead of 4).
#   - Single-block RS — no interleaving.
#
# Symbol sizes (formula: size = 2 × version_number + 9):
#
#   M1: 11×11    M2: 13×13    M3: 15×15    M4: 17×17
#
# Encoding pipeline:
#
#   input string
#     → choose smallest symbol (M1..M4) and encoding mode
#     → build bit stream (mode indicator + char count + data + terminator + padding)
#     → Reed-Solomon ECC over GF(256)/0x11D with b=0 convention
#     → initialize grid (finder, L-shaped separator, timing, format reserved)
#     → zigzag data placement (two-column snake from bottom-right)
#     → evaluate 4 mask patterns, pick lowest penalty
#     → write format information (15 bits, single copy, XOR 0x4445)
#     → ModuleGrid
#
# Dependencies (loaded first per Ruby require-ordering rules):
#
#   gf256                                — GF(256) arithmetic (local path dep)
#   coding_adventures_paint_instructions — PaintScene IR
#   coding_adventures_barcode_2d         — ModuleGrid + layout()

require_relative "../../../gf256/lib/gf256"
require "coding_adventures_paint_instructions"
require "coding_adventures_barcode_2d"
require_relative "micro_qr/version"

module CodingAdventures
  # ===========================================================================
  # MicroQR — top-level namespace
  # ===========================================================================
  module MicroQR
    # =========================================================================
    # Public types — MicroQRVersion and MicroQREccLevel
    # =========================================================================
    #
    # Use symbol constants instead of plain strings to get cheap identity
    # comparison (===) and readable error messages.

    # Micro QR symbol designators.  Each step up adds two rows/columns.
    module MicroQRVersion
      M1 = :M1
      M2 = :M2
      M3 = :M3
      M4 = :M4

      ALL = [M1, M2, M3, M4].freeze
    end

    # Error correction (or detection) levels.
    #
    #   Level      | Available in | Recovery
    #   -----------|-------------|-------------------------------
    #   Detection  | M1 only     | detects errors only (2-byte check)
    #   L          | M2, M3, M4  | ~7 % of codewords recoverable
    #   M          | M2, M3, M4  | ~15 % of codewords recoverable
    #   Q          | M4 only     | ~25 % of codewords recoverable
    #
    # Level H is not available in any Micro QR symbol.
    module MicroQREccLevel
      Detection = :Detection
      L = :L
      M = :M
      Q = :Q

      ALL = [Detection, L, M, Q].freeze
    end

    # =========================================================================
    # Errors
    # =========================================================================

    # Input is too long to fit in any M1–M4 symbol at any ECC level.
    class InputTooLong < StandardError; end

    # The requested encoding mode is not available for the chosen symbol.
    class UnsupportedMode < StandardError; end

    # The requested ECC level is not available for the chosen symbol.
    class ECCNotAvailable < StandardError; end

    # A character cannot be encoded in the selected mode.
    class InvalidCharacter < StandardError; end

    # =========================================================================
    # Symbol configuration table
    # =========================================================================
    #
    # All compile-time constants for one (version, ECC) combination.
    # There are exactly 8 valid combinations:
    #   M1/Detection, M2/L, M2/M, M3/L, M3/M, M4/L, M4/M, M4/Q.
    #
    # Fields:
    #   version           — MicroQRVersion symbol
    #   ecc               — MicroQREccLevel symbol
    #   symbol_indicator  — 3-bit integer placed in format information (0..7)
    #   size              — symbol side length in modules (11, 13, 15, or 17)
    #   data_cw           — data codeword count (full 8-bit bytes)
    #   ecc_cw            — ECC codeword count
    #   numeric_cap       — max numeric chars (0 = not supported)
    #   alpha_cap         — max alphanumeric chars (0 = not supported)
    #   byte_cap          — max byte-mode chars (0 = not supported)
    #   terminator_bits   — terminator length: 3/5/7/9 bits
    #   mode_indicator_bits — width of mode indicator: 0=M1, 1=M2, 2=M3, 3=M4
    #   cc_bits_numeric   — character count field width for numeric mode
    #   cc_bits_alpha     — character count field width for alphanumeric mode
    #   cc_bits_byte      — character count field width for byte mode
    #   m1_half_cw        — true for M1 only (last data "codeword" is 4 bits)

    SymbolConfig = Struct.new(
      :version, :ecc, :symbol_indicator, :size,
      :data_cw, :ecc_cw,
      :numeric_cap, :alpha_cap, :byte_cap,
      :terminator_bits, :mode_indicator_bits,
      :cc_bits_numeric, :cc_bits_alpha, :cc_bits_byte,
      :m1_half_cw,
      keyword_init: true
    )

    # All 8 valid Micro QR symbol configurations from ISO 18004:2015 Annex E.
    SYMBOL_CONFIGS = [
      # M1 / Detection — numeric only, half final codeword
      SymbolConfig.new(
        version: MicroQRVersion::M1, ecc: MicroQREccLevel::Detection,
        symbol_indicator: 0, size: 11,
        data_cw: 3, ecc_cw: 2,
        numeric_cap: 5, alpha_cap: 0, byte_cap: 0,
        terminator_bits: 3, mode_indicator_bits: 0,
        cc_bits_numeric: 3, cc_bits_alpha: 0, cc_bits_byte: 0,
        m1_half_cw: true
      ),
      # M2 / L — numeric + alphanumeric + byte at low ECC
      SymbolConfig.new(
        version: MicroQRVersion::M2, ecc: MicroQREccLevel::L,
        symbol_indicator: 1, size: 13,
        data_cw: 5, ecc_cw: 5,
        numeric_cap: 10, alpha_cap: 6, byte_cap: 4,
        terminator_bits: 5, mode_indicator_bits: 1,
        cc_bits_numeric: 4, cc_bits_alpha: 3, cc_bits_byte: 4,
        m1_half_cw: false
      ),
      # M2 / M — same size as M2/L but 1 fewer data codeword (more ECC)
      SymbolConfig.new(
        version: MicroQRVersion::M2, ecc: MicroQREccLevel::M,
        symbol_indicator: 2, size: 13,
        data_cw: 4, ecc_cw: 6,
        numeric_cap: 8, alpha_cap: 5, byte_cap: 3,
        terminator_bits: 5, mode_indicator_bits: 1,
        cc_bits_numeric: 4, cc_bits_alpha: 3, cc_bits_byte: 4,
        m1_half_cw: false
      ),
      # M3 / L — 11 data codewords, 6 ECC
      SymbolConfig.new(
        version: MicroQRVersion::M3, ecc: MicroQREccLevel::L,
        symbol_indicator: 3, size: 15,
        data_cw: 11, ecc_cw: 6,
        numeric_cap: 23, alpha_cap: 14, byte_cap: 9,
        terminator_bits: 7, mode_indicator_bits: 2,
        cc_bits_numeric: 5, cc_bits_alpha: 4, cc_bits_byte: 4,
        m1_half_cw: false
      ),
      # M3 / M — 9 data codewords, 8 ECC
      SymbolConfig.new(
        version: MicroQRVersion::M3, ecc: MicroQREccLevel::M,
        symbol_indicator: 4, size: 15,
        data_cw: 9, ecc_cw: 8,
        numeric_cap: 18, alpha_cap: 11, byte_cap: 7,
        terminator_bits: 7, mode_indicator_bits: 2,
        cc_bits_numeric: 5, cc_bits_alpha: 4, cc_bits_byte: 4,
        m1_half_cw: false
      ),
      # M4 / L — 16 data codewords, 8 ECC
      SymbolConfig.new(
        version: MicroQRVersion::M4, ecc: MicroQREccLevel::L,
        symbol_indicator: 5, size: 17,
        data_cw: 16, ecc_cw: 8,
        numeric_cap: 35, alpha_cap: 21, byte_cap: 15,
        terminator_bits: 9, mode_indicator_bits: 3,
        cc_bits_numeric: 6, cc_bits_alpha: 5, cc_bits_byte: 5,
        m1_half_cw: false
      ),
      # M4 / M — 14 data codewords, 10 ECC
      SymbolConfig.new(
        version: MicroQRVersion::M4, ecc: MicroQREccLevel::M,
        symbol_indicator: 6, size: 17,
        data_cw: 14, ecc_cw: 10,
        numeric_cap: 30, alpha_cap: 18, byte_cap: 13,
        terminator_bits: 9, mode_indicator_bits: 3,
        cc_bits_numeric: 6, cc_bits_alpha: 5, cc_bits_byte: 5,
        m1_half_cw: false
      ),
      # M4 / Q — 10 data codewords, 14 ECC (highest error correction)
      SymbolConfig.new(
        version: MicroQRVersion::M4, ecc: MicroQREccLevel::Q,
        symbol_indicator: 7, size: 17,
        data_cw: 10, ecc_cw: 14,
        numeric_cap: 21, alpha_cap: 13, byte_cap: 9,
        terminator_bits: 9, mode_indicator_bits: 3,
        cc_bits_numeric: 6, cc_bits_alpha: 5, cc_bits_byte: 5,
        m1_half_cw: false
      )
    ].freeze

    # =========================================================================
    # RS generator polynomials (compile-time constants)
    # =========================================================================
    #
    # Monic RS generator polynomials for GF(256)/0x11D with b=0 convention.
    #
    # g(x) = (x + α^0)(x + α^1) · · · (x + α^{n-1})
    #
    # Array length is n+1 (leading monic term 0x01 included at index 0).
    # Only the counts {2, 5, 6, 8, 10, 14} are needed for Micro QR.
    #
    # These match the regular QR Code generator polynomials for the same ECC
    # codeword counts — both use the same GF(256)/0x11D field and b=0 convention.
    GENERATORS = {
      2  => [0x01, 0x03, 0x02].freeze,
      5  => [0x01, 0x1f, 0xf6, 0x44, 0xd9, 0x68].freeze,
      6  => [0x01, 0x3f, 0x4e, 0x17, 0x9b, 0x05, 0x37].freeze,
      8  => [0x01, 0x63, 0x0d, 0x60, 0x6d, 0x5b, 0x10, 0xa2, 0xa3].freeze,
      10 => [0x01, 0xf6, 0x75, 0xa8, 0xd0, 0xc3, 0xe3, 0x36, 0xe1, 0x3c, 0x45].freeze,
      14 => [0x01, 0xf6, 0x9a, 0x60, 0x97, 0x8a, 0xf1, 0xa4, 0xa1, 0x8e, 0xfc, 0x7a, 0x52, 0xad, 0xac].freeze
    }.freeze

    # =========================================================================
    # Pre-computed format information table
    # =========================================================================
    #
    # All 32 format words (after XOR with 0x4445), indexed by
    # FORMAT_TABLE[symbol_indicator][mask_pattern].
    #
    # The 15-bit format word structure:
    #   [symbol_indicator (3b)] [mask_pattern (2b)] [BCH-10 remainder]
    #
    # XOR mask 0x4445 is Micro QR specific (regular QR uses 0x5412).
    # This prevents Micro QR format info from being misread as regular QR.
    FORMAT_TABLE = [
      [0x4445, 0x4172, 0x4e2b, 0x4b1c],  # M1 (symbol_indicator = 0)
      [0x5528, 0x501f, 0x5f46, 0x5a71],  # M2-L (symbol_indicator = 1)
      [0x6649, 0x637e, 0x6c27, 0x6910],  # M2-M (symbol_indicator = 2)
      [0x7764, 0x7253, 0x7d0a, 0x783d],  # M3-L (symbol_indicator = 3)
      [0x06de, 0x03e9, 0x0cb0, 0x0987],  # M3-M (symbol_indicator = 4)
      [0x17f3, 0x12c4, 0x1d9d, 0x18aa],  # M4-L (symbol_indicator = 5)
      [0x24b2, 0x2185, 0x2edc, 0x2beb],  # M4-M (symbol_indicator = 6)
      [0x359f, 0x30a8, 0x3ff1, 0x3ac6]   # M4-Q (symbol_indicator = 7)
    ].freeze

    # =========================================================================
    # Encoding modes
    # =========================================================================

    # The 45-character alphanumeric set shared with regular QR Code.
    # Indices 0–9: digits, 10–35: A–Z, 36: SP, 37: $, 38: %, 39: *,
    # 40: +, 41: -, 42: ., 43: /, 44: :
    ALPHANUM_CHARS = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:".freeze

    # Encoding mode symbols used internally.
    MODE_NUMERIC      = :numeric
    MODE_ALPHANUMERIC = :alphanumeric
    MODE_BYTE         = :byte

    # =========================================================================
    # module_function declarations — all helpers are private module functions
    # =========================================================================

    module_function

    # -------------------------------------------------------------------------
    # select_config — choose the smallest symbol config that fits the input
    #
    # Parameters:
    #   input   — the string to encode
    #   version — MicroQRVersion symbol or nil (auto-select)
    #   ecc     — MicroQREccLevel symbol or nil (auto-select, defaults to M)
    #
    # Returns the matching SymbolConfig or raises InputTooLong / ECCNotAvailable.
    # -------------------------------------------------------------------------
    def select_config(input, version, ecc)
      # Filter by requested version and ECC level.
      candidates = SYMBOL_CONFIGS.select do |c|
        (version.nil? || c.version == version) &&
          (ecc.nil? || c.ecc == ecc)
      end

      if candidates.empty?
        raise ECCNotAvailable,
          "No symbol configuration for version=#{version.inspect} ecc=#{ecc.inspect}. " \
          "Valid combinations: M1/Detection, M2-M3/L-M, M4/L-M-Q."
      end

      # Try each candidate in order (already ordered M1→M4, L→M→Q).
      candidates.each do |cfg|
        mode = select_mode_for(input, cfg)
        next if mode.nil?

        len = (mode == MODE_BYTE) ? input.bytesize : input.length
        cap = capacity_for_mode(cfg, mode)
        return cfg if cap > 0 && len <= cap
      end

      raise InputTooLong,
        "Input (#{input.length} chars) does not fit in any Micro QR symbol " \
        "(version=#{version.inspect}, ecc=#{ecc.inspect}). " \
        "Maximum: 35 numeric chars in M4-L."
    end

    # -------------------------------------------------------------------------
    # select_mode_for — return the most compact encoding mode supported
    #
    # Selection priority: numeric > alphanumeric > byte.
    # Returns nil if no mode can encode the input for this config.
    # -------------------------------------------------------------------------
    def select_mode_for(input, cfg)
      # Numeric: all characters are ASCII digits 0–9.
      if cfg.cc_bits_numeric > 0 && numeric?(input)
        return MODE_NUMERIC
      end

      # Alphanumeric: all characters are in the 45-char QR alphanumeric set.
      if cfg.alpha_cap > 0 && alphanumeric?(input)
        return MODE_ALPHANUMERIC
      end

      # Byte: any string — each UTF-8 byte is one byte-mode byte.
      if cfg.byte_cap > 0
        return MODE_BYTE
      end

      nil
    end

    # -------------------------------------------------------------------------
    # Predicate helpers
    # -------------------------------------------------------------------------

    def numeric?(input)
      input.empty? || input.chars.all? { |c| c >= "0" && c <= "9" }
    end

    def alphanumeric?(input)
      input.chars.all? { |c| ALPHANUM_CHARS.include?(c) }
    end

    def capacity_for_mode(cfg, mode)
      case mode
      when MODE_NUMERIC      then cfg.numeric_cap
      when MODE_ALPHANUMERIC then cfg.alpha_cap
      when MODE_BYTE         then cfg.byte_cap
      else 0
      end
    end

    # =========================================================================
    # Bit-stream builder
    # =========================================================================
    #
    # Accumulates bits MSB-first (big-endian within each codeword), matching
    # the QR / Micro QR convention.  Each write(value, count) appends the
    # count least-significant bits of value, highest bit first.

    # Internal mutable bit buffer — only used inside build_data_codewords.
    class BitWriter
      def initialize
        @bits = []  # Array of 0/1 integers
      end

      # Append count bits from value, MSB first.
      def write(value, count)
        (count - 1).downto(0) { |i| @bits << ((value >> i) & 1) }
      end

      # Total bits accumulated so far.
      def bit_length
        @bits.length
      end

      # Serialize bits to bytes (MSB-first), padding the last byte with zeros.
      def to_bytes
        result = []
        @bits.each_slice(8) do |slice|
          byte = 0
          slice.each_with_index { |b, i| byte |= (b << (7 - i)) }
          result << byte
        end
        result
      end

      # Return the raw bit array (for M1 packing).
      def bits
        @bits.dup
      end
    end

    # =========================================================================
    # Data encoding helpers
    # =========================================================================

    # Encode numeric string: groups of 3 digits → 10 bits,
    # remaining pair → 7 bits, single remaining digit → 4 bits.
    #
    # Example: "12345" → [123→10bits, 45→7bits] = 17 bits total.
    def encode_numeric(input, writer)
      digits = input.chars.map { |c| c.ord - "0".ord }
      i = 0
      while i + 2 < digits.length
        writer.write(digits[i] * 100 + digits[i + 1] * 10 + digits[i + 2], 10)
        i += 3
      end
      if i + 1 < digits.length
        writer.write(digits[i] * 10 + digits[i + 1], 7)
        i += 2
      end
      writer.write(digits[i], 4) if i < digits.length
    end

    # Encode alphanumeric string: pairs → 11 bits, single trailer → 6 bits.
    #
    # Each character maps to its index in ALPHANUM_CHARS (0..44).
    # A pair (first, second) encodes as first_index * 45 + second_index.
    def encode_alphanumeric(input, writer)
      indices = input.chars.map { |c| ALPHANUM_CHARS.index(c) || 0 }
      i = 0
      while i + 1 < indices.length
        writer.write(indices[i] * 45 + indices[i + 1], 11)
        i += 2
      end
      writer.write(indices[i], 6) if i < indices.length
    end

    # Encode byte mode: each UTF-8 byte as 8 bits.
    # Multi-byte UTF-8 sequences are treated as individual bytes.
    def encode_byte(input, writer)
      input.bytes.each { |b| writer.write(b, 8) }
    end

    # Mode indicator value for a given mode and config.
    #
    # M1 (0 bits): no indicator — only one mode available.
    # M2 (1 bit):  0 = numeric,  1 = alphanumeric
    # M3 (2 bits): 00 = numeric, 01 = alpha, 10 = byte
    # M4 (3 bits): 000 = numeric, 001 = alpha, 010 = byte, 011 = kanji
    def mode_indicator_value(mode, cfg)
      case cfg.mode_indicator_bits
      when 0 then 0
      when 1 then mode == MODE_NUMERIC ? 0 : 1
      when 2
        case mode
        when MODE_NUMERIC      then 0b00
        when MODE_ALPHANUMERIC then 0b01
        else                        0b10
        end
      when 3
        case mode
        when MODE_NUMERIC      then 0b000
        when MODE_ALPHANUMERIC then 0b001
        else                        0b010
        end
      else 0
      end
    end

    # Character count field width for a given mode and config.
    def cc_bits(mode, cfg)
      case mode
      when MODE_NUMERIC      then cfg.cc_bits_numeric
      when MODE_ALPHANUMERIC then cfg.cc_bits_alpha
      when MODE_BYTE         then cfg.cc_bits_byte
      else 0
      end
    end

    # =========================================================================
    # Build data codewords
    # =========================================================================
    #
    # Assembles the complete data codeword byte sequence.
    #
    # For all symbols except M1 (m1_half_cw = false):
    #   [mode indicator] [char count] [data bits] [terminator]
    #   [zero-pad to byte boundary] [0xEC/0x11 fill to cfg.data_cw bytes]
    #
    # For M1 (m1_half_cw = true):
    #   Total capacity = 20 bits = 2 full bytes + 4-bit nibble.
    #   The RS encoder receives 3 bytes where byte[2] has data in the
    #   upper 4 bits and zeros in the lower 4 bits.
    def build_data_codewords(input, cfg, mode)
      total_bits = cfg.m1_half_cw ? cfg.data_cw * 8 - 4 : cfg.data_cw * 8

      w = BitWriter.new

      # Mode indicator (0, 1, 2, or 3 bits).
      w.write(mode_indicator_value(mode, cfg), cfg.mode_indicator_bits) if cfg.mode_indicator_bits > 0

      # Character count.
      char_count = (mode == MODE_BYTE) ? input.bytesize : input.length
      w.write(char_count, cc_bits(mode, cfg))

      # Encoded data bits.
      case mode
      when MODE_NUMERIC      then encode_numeric(input, w)
      when MODE_ALPHANUMERIC then encode_alphanumeric(input, w)
      when MODE_BYTE         then encode_byte(input, w)
      end

      # Terminator: up to terminator_bits zeros (truncated if capacity full).
      remaining = total_bits - w.bit_length
      if remaining > 0
        w.write(0, [cfg.terminator_bits, remaining].min)
      end

      if cfg.m1_half_cw
        # M1 special case: pack exactly 20 bits into 3 bytes.
        # Byte 2 carries data in bits [7:4] and zeros in bits [3:0].
        bits = w.bits
        bits.fill(0, bits.length...20)
        b0 = bits[0..7].each_with_index.reduce(0) { |acc, (b, i)| acc | (b << (7 - i)) }
        b1 = bits[8..15].each_with_index.reduce(0) { |acc, (b, i)| acc | (b << (7 - i)) }
        b2 = bits[16..19].each_with_index.reduce(0) { |acc, (b, i)| acc | (b << (7 - i)) }
        return [b0, b1, b2]
      end

      # Pad to byte boundary.
      rem = w.bit_length % 8
      w.write(0, 8 - rem) if rem != 0

      # Fill remaining codewords with alternating 0xEC / 0x11.
      bytes = w.to_bytes
      pad = 0xec
      while bytes.length < cfg.data_cw
        bytes << pad
        pad = (pad == 0xec) ? 0x11 : 0xec
      end

      bytes
    end

    # =========================================================================
    # Reed-Solomon encoder
    # =========================================================================
    #
    # Computes ECC bytes using LFSR polynomial division over GF(256)/0x11D.
    # Returns the remainder of D(x)·x^n mod G(x) where n = generator.length - 1.
    #
    # Uses the b=0 convention (first root is α^0 = 1), same as regular QR Code.
    #
    # Algorithm (LFSR division):
    #   rem = [0] × n
    #   for each data byte b:
    #     feedback = b XOR rem[0]
    #     shift rem left by 1 (drop rem[0], append 0)
    #     for each i in 0..n-1:
    #       rem[i] ^= gf_mul(generator[i+1], feedback)
    def rs_encode(data, generator)
      n = generator.length - 1
      rem = Array.new(n, 0)
      data.each do |b|
        fb = b ^ rem[0]
        rem.rotate!(1)
        rem[n - 1] = 0
        next if fb == 0

        (0...n).each do |i|
          rem[i] ^= GF256.multiply(generator[i + 1], fb)
        end
      end
      rem
    end

    # =========================================================================
    # Grid construction
    # =========================================================================
    #
    # A "work grid" is a pair of 2D arrays:
    #   modules[row][col]  — boolean: true = dark, false = light
    #   reserved[row][col] — boolean: true = structural (do not overwrite)

    WorkGrid = Struct.new(:size, :modules, :reserved)

    # Create a blank (all-light, all-unreserved) work grid of given size.
    def new_work_grid(size)
      WorkGrid.new(
        size,
        Array.new(size) { Array.new(size, false) },
        Array.new(size) { Array.new(size, false) }
      )
    end

    # Set a module value and optionally mark it reserved.
    def grid_set(g, row, col, dark, reserve: false)
      g.modules[row][col]  = dark
      g.reserved[row][col] = true if reserve
    end

    # -------------------------------------------------------------------------
    # Place the 7×7 finder pattern at rows 0–6, cols 0–6.
    #
    # The finder pattern is the locator that lets scanners find and orient
    # the symbol.  Its concentric ring structure (outer ring dark, next ring
    # light, inner 3×3 dark) produces the 1:1:3:1:1 dark-light-dark ratio
    # that scanners detect.
    #
    #   row\col  0 1 2 3 4 5 6
    #         0: ■ ■ ■ ■ ■ ■ ■
    #         1: ■ □ □ □ □ □ ■
    #         2: ■ □ ■ ■ ■ □ ■
    #         3: ■ □ ■ ■ ■ □ ■
    #         4: ■ □ ■ ■ ■ □ ■
    #         5: ■ □ □ □ □ □ ■
    #         6: ■ ■ ■ ■ ■ ■ ■
    # -------------------------------------------------------------------------
    def place_finder(g)
      7.times do |dr|
        7.times do |dc|
          on_border = dr == 0 || dr == 6 || dc == 0 || dc == 6
          in_core   = dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4
          grid_set(g, dr, dc, on_border || in_core, reserve: true)
        end
      end
    end

    # -------------------------------------------------------------------------
    # Place the L-shaped separator.
    #
    # Unlike regular QR which surrounds all three finders, Micro QR's single
    # finder only needs separation on its bottom and right sides — the top and
    # left edges of the finder ARE the symbol boundary.
    #
    #   Row 7, cols 0–7 → light modules (bottom edge of finder area)
    #   Col 7, rows 0–7 → light modules (right edge of finder area)
    # -------------------------------------------------------------------------
    def place_separator(g)
      (0..7).each do |i|
        grid_set(g, 7, i, false, reserve: true)  # bottom row of separator
        grid_set(g, i, 7, false, reserve: true)  # right col of separator
      end
    end

    # -------------------------------------------------------------------------
    # Place timing pattern extensions along row 0 and col 0.
    #
    # Positions 0–6 are already set by the finder pattern (outer ring = all
    # dark, which happens to satisfy even=dark at those positions).
    # Position 7 is the separator (reserved light).
    # Position 8 onward: dark if index is even, light if odd.
    #
    # Why row 0 and col 0? Unlike regular QR (timing at row 6 / col 6),
    # Micro QR places timing along the outer symbol edges, forming the "spine"
    # from which scanners triangulate module positions.
    # -------------------------------------------------------------------------
    def place_timing(g)
      sz = g.size
      (8...sz).each do |c|
        grid_set(g, 0, c, c.even?, reserve: true)
      end
      (8...sz).each do |r|
        grid_set(g, r, 0, r.even?, reserve: true)
      end
    end

    # -------------------------------------------------------------------------
    # Reserve the 15 format information module positions.
    #
    # Format info is written AFTER mask selection; we just mark these as
    # reserved (temporarily light = 0) during grid initialization.
    #
    # Row 8, cols 1–8 → 8 modules (bits f14..f7, MSB first going right)
    # Col 8, rows 1–7 → 7 modules (bits f6..f0, f6 at row 7, f0 at row 1)
    #
    # Total: 15 modules = 15-bit format word.
    # -------------------------------------------------------------------------
    def reserve_format_info(g)
      (1..8).each { |c| grid_set(g, 8, c, false, reserve: true) }
      (1..7).each { |r| grid_set(g, r, 8, false, reserve: true) }
    end

    # -------------------------------------------------------------------------
    # Write the 15-bit format word into the reserved format positions.
    #
    # Bit f14 (MSB) → row 8 col 1
    # Bit f13       → row 8 col 2
    # ...
    # Bit f7        → row 8 col 8
    # Bit f6        → col 8 row 7
    # Bit f5        → col 8 row 6
    # ...
    # Bit f0 (LSB)  → col 8 row 1
    # -------------------------------------------------------------------------
    def write_format_info(g, fmt)
      # Row 8, cols 1–8: bits f14 down to f7 (8 bits, MSB first)
      8.times do |i|
        g.modules[8][1 + i] = ((fmt >> (14 - i)) & 1) == 1
      end
      # Col 8, rows 7 down to 1: bits f6 down to f0 (7 bits)
      7.times do |i|
        g.modules[7 - i][8] = ((fmt >> (6 - i)) & 1) == 1
      end
    end

    # -------------------------------------------------------------------------
    # Build the initial work grid with all structural modules.
    # -------------------------------------------------------------------------
    def build_grid(cfg)
      g = new_work_grid(cfg.size)
      place_finder(g)
      place_separator(g)
      place_timing(g)
      reserve_format_info(g)
      g
    end

    # =========================================================================
    # Data placement — two-column zigzag
    # =========================================================================
    #
    # Places bits from the final codeword stream into unreserved modules via
    # a two-column zigzag scan starting from the bottom-right corner.
    #
    # Algorithm:
    #   col = size - 1   (rightmost column)
    #   dir = :up        (start scanning upward)
    #   while col >= 1:
    #     scan the 2-column strip (col, col-1) in direction dir
    #     for each (row, subcol) in this strip:
    #       skip if reserved
    #       place next bit
    #     flip direction, move left 2 columns
    #
    # Note: Micro QR has NO timing column at col 6 to skip around (unlike
    # regular QR which hops from col 7 to col 5 to avoid timing col 6).
    # Micro QR timing is at col 0, which is reserved and auto-skipped.
    def place_bits(g, bits)
      sz = g.size
      bit_idx = 0
      upward = true

      col = sz - 1
      while col >= 1
        rows = upward ? (sz - 1).downto(0) : (0...sz)

        rows.each do |row|
          [col, col - 1].each do |c|
            next if g.reserved[row][c]

            g.modules[row][c] = bit_idx < bits.length ? bits[bit_idx] : false
            bit_idx += 1
          end
        end

        upward = !upward
        col -= 2
      end
    end

    # =========================================================================
    # Masking
    # =========================================================================
    #
    # Masking XORs each data/ECC module with the mask condition result,
    # avoiding problematic patterns that look like finder patterns to scanners.
    #
    # Micro QR uses only 4 of regular QR's 8 mask patterns:
    #
    #   Pattern | Condition (flip if true)
    #   --------|------------------------
    #       0   | (row + col) mod 2 == 0
    #       1   | row mod 2 == 0
    #       2   | col mod 3 == 0
    #       3   | (row + col) mod 3 == 0

    def mask_condition?(mask_idx, row, col)
      case mask_idx
      when 0 then (row + col) % 2 == 0
      when 1 then row % 2 == 0
      when 2 then col % 3 == 0
      when 3 then (row + col) % 3 == 0
      else false
      end
    end

    # Apply mask to all non-reserved modules; returns a NEW 2D array.
    def apply_mask(modules, reserved, sz, mask_idx)
      result = Array.new(sz) { |r| modules[r].dup }
      sz.times do |r|
        sz.times do |c|
          next if reserved[r][c]

          result[r][c] = modules[r][c] ^ mask_condition?(mask_idx, r, c)
        end
      end
      result
    end

    # =========================================================================
    # Penalty scoring
    # =========================================================================
    #
    # Four rules (same as regular QR Code), evaluated on the full symbol
    # including structural modules.  Lower total penalty = better mask.
    #
    # Rule 1 — Adjacent same-color runs of ≥5 → score += run - 2
    # Rule 2 — 2×2 same-color blocks → score += 3 each
    # Rule 3 — Finder-like sequences → score += 40 each
    # Rule 4 — Dark proportion deviation from 50% → scaled penalty

    PATTERN1 = [1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0].freeze
    PATTERN2 = [0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1].freeze

    def compute_penalty(modules, sz)
      penalty = 0

      # ── Rule 1: Adjacent same-color runs ──────────────────────────────────
      # A run of length L ≥ 5 contributes (L - 2) to the penalty.
      # Scan every row and column.
      sz.times do |a|
        [true, false].each do |horiz|
          run = 1
          prev = horiz ? modules[a][0] : modules[0][a]
          (1...sz).each do |i|
            cur = horiz ? modules[a][i] : modules[i][a]
            if cur == prev
              run += 1
            else
              penalty += run - 2 if run >= 5
              run = 1
              prev = cur
            end
          end
          penalty += run - 2 if run >= 5
        end
      end

      # ── Rule 2: 2×2 same-color blocks ─────────────────────────────────────
      # For each 2×2 square with all four cells the same color, add 3.
      (sz - 1).times do |r|
        (sz - 1).times do |c|
          d = modules[r][c]
          penalty += 3 if d == modules[r][c + 1] &&
                          d == modules[r + 1][c] &&
                          d == modules[r + 1][c + 1]
        end
      end

      # ── Rule 3: Finder-like sequences ─────────────────────────────────────
      # Scan for 11-module sequences matching the finder pattern or its reverse.
      # These sequences confuse scanners and must be minimized.
      if sz >= 11
        limit = sz - 11
        sz.times do |a|
          (0..limit).each do |b|
            mh1 = mh2 = mv1 = mv2 = true
            11.times do |k|
              bh = modules[a][b + k] ? 1 : 0
              bv = modules[b + k][a] ? 1 : 0
              mh1 = false if bh != PATTERN1[k]
              mh2 = false if bh != PATTERN2[k]
              mv1 = false if bv != PATTERN1[k]
              mv2 = false if bv != PATTERN2[k]
              break if !mh1 && !mh2 && !mv1 && !mv2
            end
            penalty += 40 if mh1
            penalty += 40 if mh2
            penalty += 40 if mv1
            penalty += 40 if mv2
          end
        end
      end

      # ── Rule 4: Dark proportion deviation from 50% ────────────────────────
      # Count dark modules, compute percentage, find distance from multiples
      # of 5% nearest to 50%.
      dark = modules.sum { |row| row.count(true) }
      total = sz * sz
      dark_pct = (dark * 100) / total
      prev5 = (dark_pct / 5) * 5
      next5 = prev5 + 5
      r4 = [(prev5 - 50).abs, (next5 - 50).abs].min
      penalty += (r4 / 5) * 10

      penalty
    end

    # =========================================================================
    # Public API
    # =========================================================================

    # -------------------------------------------------------------------------
    # encode — encode a string to a Micro QR Code ModuleGrid
    #
    # Automatically selects the smallest symbol (M1..M4) and ECC level that
    # can hold the input.  Pass version: and/or ecc: to override.
    #
    # Parameters:
    #   input   — the string to encode
    #   version — MicroQRVersion constant or nil (auto-select)
    #   ecc     — MicroQREccLevel constant or nil (auto-select; tries M then L)
    #
    # Returns a CodingAdventures::Barcode2D::ModuleGrid.
    #
    # Raises:
    #   InputTooLong    — input exceeds M4 capacity
    #   ECCNotAvailable — requested version+ECC combination does not exist
    #   UnsupportedMode — no encoding mode covers the input for chosen symbol
    #
    # Example:
    #   grid = CodingAdventures::MicroQR.encode("HELLO")
    #   grid.rows  # => 13  (M2 symbol)
    #
    #   m4 = CodingAdventures::MicroQR.encode(
    #     "https://a.b",
    #     version: MicroQRVersion::M4,
    #     ecc:     MicroQREccLevel::L
    #   )
    #   m4.rows  # => 17
    # -------------------------------------------------------------------------
    def encode(input, version: nil, ecc: nil)
      cfg = select_config(input, version, ecc)
      mode = select_mode_for(input, cfg)

      raise UnsupportedMode,
        "Cannot encode input in any mode for #{cfg.version}/#{cfg.ecc}" if mode.nil?

      # 1. Build data codewords
      data_cw = build_data_codewords(input, cfg, mode)

      # 2. Compute RS ECC
      gen     = GENERATORS[cfg.ecc_cw]
      ecc_cws = rs_encode(data_cw, gen)

      # 3. Flatten to bit stream
      #    M1: third data codeword contributes only 4 bits (upper nibble)
      final_cw = data_cw + ecc_cws
      bits = []
      final_cw.each_with_index do |cw, idx|
        bits_in_cw = (cfg.m1_half_cw && idx == cfg.data_cw - 1) ? 4 : 8
        (bits_in_cw - 1).downto(0) do |b|
          bits << (((cw >> b) & 1) == 1)
        end
      end

      # 4. Build grid with all structural modules
      g = build_grid(cfg)

      # 5. Place data bits into non-reserved modules
      place_bits(g, bits)

      # 6. Evaluate all 4 masks, pick the one with the lowest penalty score
      #    (ties broken by lower mask index)
      best_mask    = 0
      best_penalty = Float::INFINITY

      4.times do |m|
        masked = apply_mask(g.modules, g.reserved, cfg.size, m)
        fmt    = FORMAT_TABLE[cfg.symbol_indicator][m]

        # Write format info into a temporary copy to score the full grid.
        tmp = masked.map(&:dup)
        8.times { |i| tmp[8][1 + i] = ((fmt >> (14 - i)) & 1) == 1 }
        7.times { |i| tmp[7 - i][8] = ((fmt >> (6 - i)) & 1) == 1 }

        p = compute_penalty(tmp, cfg.size)
        if p < best_penalty
          best_penalty = p
          best_mask    = m
        end
      end

      # 7. Apply best mask and write final format information
      final_modules = apply_mask(g.modules, g.reserved, cfg.size, best_mask)
      final_fmt     = FORMAT_TABLE[cfg.symbol_indicator][best_mask]

      # Write format info directly into final_modules.
      8.times { |i| final_modules[8][1 + i] = ((final_fmt >> (14 - i)) & 1) == 1 }
      7.times { |i| final_modules[7 - i][8] = ((final_fmt >> (6 - i)) & 1) == 1 }

      # 8. Return a frozen ModuleGrid
      CodingAdventures::Barcode2D::ModuleGrid.new(
        cols: cfg.size,
        rows: cfg.size,
        modules: final_modules.map(&:freeze).freeze,
        module_shape: "square"
      ).freeze
    end

    # -------------------------------------------------------------------------
    # encode_at — encode to a specific symbol version and ECC level
    #
    # Raises InputTooLong if the input does not fit in the requested combo.
    # -------------------------------------------------------------------------
    def encode_at(input, version, ecc)
      encode(input, version: version, ecc: ecc)
    end

    # -------------------------------------------------------------------------
    # layout — convert a ModuleGrid to a PaintScene
    #
    # Defaults to quiet_zone_modules: 2 (Micro QR minimum — half of regular
    # QR's 4-module requirement).  The smaller quiet zone is safe because
    # Micro QR's single-corner detection is spatially unambiguous.
    # -------------------------------------------------------------------------
    def layout(grid, config = nil)
      cfg = { quiet_zone_modules: 2 }.merge(config || {})
      CodingAdventures::Barcode2D.layout(grid, cfg)
    end

    # -------------------------------------------------------------------------
    # encode_and_layout — convenience: encode + layout in one call
    # -------------------------------------------------------------------------
    def encode_and_layout(input, version: nil, ecc: nil, config: nil)
      grid = encode(input, version: version, ecc: ecc)
      layout(grid, config)
    end
  end
end
