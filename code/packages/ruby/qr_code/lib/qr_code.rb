# frozen_string_literal: true

# =============================================================================
# qr_code — QR Code encoder, ISO/IEC 18004:2015 compliant.
# =============================================================================
#
# QR Code (Quick Response) was invented by Masahiro Hara at Denso Wave in 1994
# to track automotive parts on assembly lines at 10× the speed of 1D barcodes.
# It was designed to survive physical damage to up to 30% of the symbol area.
# Today it is the most widely deployed 2D barcode on earth — on every product
# label, restaurant menu, bus stop, and business card.
#
# Understanding how to build a QR Code encoder from scratch teaches:
#   - how binary data is packed into Galois Field elements
#   - how Reed-Solomon erasure coding works in practice
#   - how a structured 2D layout is designed around invariant structural regions
#   - how masking defeats degenerate patterns that confuse scanners
#   - why error correction level and version selection matter for reliability
#
# ## Encoding pipeline (in order)
#
#   input string
#     → mode selection    (numeric / alphanumeric / byte — pick most compact)
#     → version selection (smallest version that fits at the chosen ECC level)
#     → bit stream        (mode indicator + char count + data + padding)
#     → blocks + RS ECC   (GF(256) b=0 convention, poly 0x11D)
#     → interleave        (data codewords interleaved, then ECC codewords)
#     → grid init         (finder, separator, timing, alignment, format, dark)
#     → zigzag placement  (two-column snake from bottom-right corner)
#     → mask evaluation   (8 patterns, lowest 4-rule penalty wins)
#     → finalize          (format info + version info for v7+)
#     → ModuleGrid        (abstract boolean grid: true = dark)
#
# ## Key bit ordering note (from lessons.md)
#
#   Format information is placed MSB-first in row 8 cols 0-5 (f14→f9 going
#   left-to-right). This is the most common source of bugs in QR encoders.
#   See write_format_info for detailed comments on the exact positions.
#
# ## Dependencies
#
#   gf256            — GF(2^8) arithmetic with primitive polynomial x^8+x^4+x^3+x^2+1
#   barcode_2d       — ModuleGrid type and layout() → PaintScene
#   paint_instructions — PaintScene, PaintRect (transitively via barcode_2d)
#
# =============================================================================

require_relative "../../gf256/lib/gf256"
require "coding_adventures_barcode_2d"
require_relative "qr_code/version"

module QrCode
  # ===========================================================================
  # Public error types
  # ===========================================================================

  # Base error for all QR Code failures.
  class QrCodeError < StandardError; end

  # Raised when the input is too long to fit in any version/ECC combination.
  # QR v40 byte mode holds at most 2953 bytes; numeric mode up to 7089 digits.
  class InputTooLongError < QrCodeError; end

  # Raised when mode selection explicitly fails (reserved for future use;
  # currently mode selection always succeeds by falling back to byte mode).
  class InvalidInputError < QrCodeError; end

  # ===========================================================================
  # ECC level constants
  # ===========================================================================
  #
  # There are four error correction levels. Higher levels recover from more
  # damage but reduce the data capacity of the symbol.
  #
  # | Level | Approx. recovery | Use case                              |
  # |-------|-----------------|---------------------------------------|
  # | L     | ~7%             | Maximum data density                  |
  # | M     | ~15%            | General-purpose (common default)      |
  # | Q     | ~25%            | Moderate noise / damage expected      |
  # | H     | ~30%            | High damage risk, logo overlaid       |
  #
  # The 2-bit indicators below (L=01, M=00, Q=11, H=10) are deliberately NOT
  # in alphabetical order — this is an ISO 18004 oddity.
  ECC_INDICATOR = {L: 0b01, M: 0b00, Q: 0b11, H: 0b10}.freeze

  # Index into the capacity/block tables: L=0, M=1, Q=2, H=3.
  ECC_IDX = {L: 0, M: 1, Q: 2, H: 3}.freeze

  # ===========================================================================
  # ISO 18004:2015 — ECC codewords per block (Table 9)
  # ===========================================================================
  #
  # Indexed by [ecc_idx][version]. Index 0 is a placeholder; versions run 1–40.
  # These constants come directly from ISO 18004:2015 Table 9 and must not be
  # computed — they are normative data.
  ECC_CODEWORDS_PER_BLOCK = [
    # L:    0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1, 7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28, 30, 30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
    # M:    0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28],
    # Q:    0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1, 13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30, 24, 28, 28, 26, 30, 28, 30, 30, 30, 30, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
    # H:    0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 30, 24, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30]
  ].freeze

  # ===========================================================================
  # ISO 18004:2015 — Number of error correction blocks (Table 9)
  # ===========================================================================
  #
  # Indexed by [ecc_idx][version].
  # Multiple blocks improve damage resilience: a burst error that destroys a
  # contiguous region only wipes out one or two blocks, leaving others intact.
  NUM_BLOCKS = [
    # L:    0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 4, 4, 4, 4, 4, 6, 6, 6, 6, 7, 8, 8, 9, 9, 10, 12, 12, 12, 13, 14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25],
    # M:    0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1, 1, 1, 1, 2, 2, 4, 4, 4, 5, 5, 5, 8, 9, 9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21, 23, 25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49],
    # Q:    0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1, 1, 1, 2, 2, 4, 4, 6, 6, 8, 8, 8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27, 29, 34, 34, 35, 38, 40, 43, 45, 48, 51, 53, 56, 59, 62, 65, 68],
    # H:    0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1, 1, 1, 2, 4, 4, 4, 5, 6, 8, 8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32, 35, 37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 80]
  ].freeze

  # ===========================================================================
  # Alignment pattern center coordinates (ISO 18004:2015 Annex E)
  # ===========================================================================
  #
  # Indexed by [version - 1]. The cross product of this list (with duplicates)
  # gives all alignment pattern center positions; those that overlap finder
  # patterns or the timing strips are skipped during placement.
  #
  # Alignment patterns are 5×5 finder-like squares. They appear in version 2+
  # and help scanners correct for perspective distortion in large symbols.
  ALIGNMENT_POSITIONS = [
    [],                              # v1  — no alignment patterns
    [6, 18],                         # v2
    [6, 22],                         # v3
    [6, 26],                         # v4
    [6, 30],                         # v5
    [6, 34],                         # v6
    [6, 22, 38],                     # v7
    [6, 24, 42],                     # v8
    [6, 26, 46],                     # v9
    [6, 28, 50],                     # v10
    [6, 30, 54],                     # v11
    [6, 32, 58],                     # v12
    [6, 34, 62],                     # v13
    [6, 26, 46, 66],                 # v14
    [6, 26, 48, 70],                 # v15
    [6, 26, 50, 74],                 # v16
    [6, 30, 54, 78],                 # v17
    [6, 30, 56, 82],                 # v18
    [6, 30, 58, 86],                 # v19
    [6, 34, 62, 90],                 # v20
    [6, 28, 50, 72, 94],             # v21
    [6, 26, 50, 74, 98],             # v22
    [6, 30, 54, 78, 102],            # v23
    [6, 28, 54, 80, 106],            # v24
    [6, 32, 58, 84, 110],            # v25
    [6, 30, 58, 86, 114],            # v26
    [6, 34, 62, 90, 118],            # v27
    [6, 26, 50, 74, 98, 122],        # v28
    [6, 30, 54, 78, 102, 126],       # v29
    [6, 26, 52, 78, 104, 130],       # v30
    [6, 30, 56, 82, 108, 134],       # v31
    [6, 34, 60, 86, 112, 138],       # v32
    [6, 30, 58, 86, 114, 142],       # v33
    [6, 34, 62, 90, 118, 146],       # v34
    [6, 30, 54, 78, 102, 126, 150],  # v35
    [6, 24, 50, 76, 102, 128, 154],  # v36
    [6, 28, 54, 80, 106, 132, 158],  # v37
    [6, 32, 58, 84, 110, 136, 162],  # v38
    [6, 26, 54, 82, 110, 138, 166],  # v39
    [6, 30, 58, 86, 114, 142, 170]   # v40
  ].freeze

  # ===========================================================================
  # Alphanumeric character set
  # ===========================================================================
  #
  # 45 characters total (0–44 indices). Pairs of characters are packed into
  # 11 bits: (first_index * 45 + second_index). A trailing single character
  # uses 6 bits.
  #
  # 0–9 → indices 0–9
  # A–Z → indices 10–35
  # SP  → 36,  $→37,  %→38,  *→39,  +→40,  -→41,  .→42,  /→43,  :→44
  ALPHANUM_CHARS = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:"

  # Pre-build the lookup hash for O(1) character → index mapping.
  # Without this, String#index in a hot loop would be O(n) per character.
  ALPHANUM_INDEX = ALPHANUM_CHARS.chars.each_with_index.to_h.freeze

  # Mode indicator bits (4-bit values placed at the start of the bit stream).
  #
  # | Mode         | Indicator | Notes                                  |
  # |--------------|-----------|----------------------------------------|
  # | Numeric      | 0001      | Digits 0–9 only                        |
  # | Alphanumeric | 0010      | Digits + uppercase A–Z + 9 symbols     |
  # | Byte         | 0100      | Any byte (UTF-8 encoded)               |
  # | Kanji        | 1000      | Shift-JIS double-byte chars (v0.2.0)   |
  # | Terminator   | 0000      | Ends the data segment                  |
  MODE_INDICATOR = {numeric: 0b0001, alphanumeric: 0b0010, byte: 0b0100}.freeze

  # ===========================================================================
  # Grid geometry helpers
  # ===========================================================================
  #
  # A QR Code symbol of version V is exactly (4V + 17) modules on each side.
  #
  #   Version 1:  21×21 modules
  #   Version 2:  25×25 modules
  #   Version 10: 57×57 modules
  #   Version 40: 177×177 modules
  def self.symbol_size(version)
    4 * version + 17
  end

  # Calculate how many raw data + ECC bits fit in the symbol after reserving
  # all structural modules.
  #
  # Formula from Nayuki's QR Code generator (public domain). The formula
  # subtracts:
  #   - Finder patterns (3 × 8×8 corners including separators)
  #   - Timing strips (row 6 and col 6)
  #   - Alignment patterns (5×5, starting in v2)
  #   - Version info blocks (6×3×2, starting in v7)
  #   - Format info (15 modules × 2 copies)
  def self.num_raw_data_modules(version)
    result = (16 * version + 128) * version + 64
    if version >= 2
      num_align = (version / 7) + 2
      result -= (25 * num_align - 10) * num_align - 55
      result -= 36 if version >= 7
    end
    result
  end

  # Total data codewords (message + padding, excluding ECC codewords).
  # This is the capacity of the symbol in bytes for the given version and ECC.
  def self.num_data_codewords(version, ecc)
    e = ECC_IDX[ecc]
    (num_raw_data_modules(version) / 8) -
      (NUM_BLOCKS[e][version] * ECC_CODEWORDS_PER_BLOCK[e][version])
  end

  # Remainder bits: zero padding appended after all codewords are placed.
  # Needed because the total raw module count may not be divisible by 8.
  # Possible values: 0, 3, 4, or 7 (depending on version).
  def self.num_remainder_bits(version)
    num_raw_data_modules(version) % 8
  end

  # ===========================================================================
  # Reed-Solomon ECC (b=0 convention)
  # ===========================================================================
  #
  # QR Code uses a specific variant of Reed-Solomon over GF(256).
  # The key difference from some other RS implementations is the b=0 convention:
  #
  #   g(x) = (x + α^0)(x + α^1)(x + α^2)···(x + α^{n-1})
  #
  # Here α = 2 (the primitive element) and n is the number of ECC codewords.
  # The first root is α^0 = 1, NOT α^1 = 2. This makes the roots α^0..α^{n-1}.
  #
  # The QR RS encoder only needs polynomial remainder (encoding), not decoding.
  # Given:
  #   D(x) = data polynomial of degree k-1 (k data codewords)
  #   G(x) = generator polynomial of degree n (n ECC codewords)
  # Compute:
  #   R(x) = D(x) × x^n mod G(x)
  # The ECC codewords are the coefficients of R(x).
  #
  # This is implemented as an LFSR (linear feedback shift register) division.

  # Build the monic RS generator polynomial of degree n using b=0 convention.
  #
  # Algorithm:
  #   Start with g = [1]
  #   For i = 0..n-1: multiply g by the linear factor [1, α^i]
  #
  # The multiplication [1, α^i] means:
  #   new_g[j] = old_g[j-1] XOR (α^i × old_g[j])
  #
  # Since GF(256) uses log/antilog tables, each multiplication is O(1).
  # Building the generator takes O(n^2) time total.
  def self.build_generator(n)
    g = [1]
    alog = GF256.alog_table
    n.times do |i|
      ai = alog[i]  # α^i
      next_g = Array.new(g.length + 1, 0)
      g.length.times do |j|
        next_g[j] ^= g[j]
        next_g[j + 1] ^= GF256.multiply(g[j], ai)
      end
      g = next_g
    end
    g
  end

  # Pre-build all generator polynomials used by the QR capacity table.
  # These 13 values are the only ECC codeword counts that appear in
  # ISO 18004:2015 Table 9 across all 40 versions × 4 ECC levels.
  GENERATORS = (
    generators = {}
    [7, 10, 13, 15, 16, 17, 18, 20, 22, 24, 26, 28, 30].each do |n|
      generators[n] = build_generator(n).freeze
    end
    generators.freeze
  )

  # Look up the generator polynomial for n ECC codewords.
  # Builds lazily if not pre-built (handles edge cases not in the standard table).
  def self.get_generator(n)
    GENERATORS[n] || build_generator(n)
  end

  # Encode data bytes to produce n ECC codewords using LFSR polynomial division.
  #
  # LFSR division algorithm:
  #   Initialize shift register rem[0..n-1] = 0
  #   For each data byte b:
  #     feedback = b XOR rem[0]        ← tap at the leading position
  #     Shift rem left (rem[i] ← rem[i+1] for i < n-1; rem[n-1] = 0)
  #     For i = 0..n-1: rem[i] ^= G[i+1] × feedback
  #
  # The shift register implements R(x) = D(x) × x^n mod G(x).
  # After processing all data bytes, rem contains the ECC codewords.
  def self.rs_encode(data, generator)
    n = generator.length - 1
    rem = Array.new(n, 0)
    data.each do |b|
      fb = b ^ rem[0]  # feedback = data byte XOR leading register tap
      (n - 1).times { |i| rem[i] = rem[i + 1] }
      rem[n - 1] = 0
      if fb != 0
        n.times { |i| rem[i] ^= GF256.multiply(generator[i + 1], fb) }
      end
    end
    rem
  end

  # ===========================================================================
  # Encoding mode selection
  # ===========================================================================
  #
  # Three modes are supported in v0.1.0 (Kanji is v0.2.0):
  #
  # NUMERIC: digits 0–9 only. Three digits → 10 bits (range 0–999).
  #   Remaining pair → 7 bits (0–99). Single digit → 4 bits (0–9).
  #   ~3.3 bits per character. Best for long digit strings like phone numbers.
  #
  # ALPHANUMERIC: 45-char set (digits, A–Z, SP, $%*+-./:). Pairs → 11 bits.
  #   Single trailing char → 6 bits. ~5.5 bits per character. Good for URLs,
  #   uppercase text.
  #
  # BYTE: raw UTF-8 bytes. Each byte → 8 bits. Always valid. Modern decoders
  #   interpret the bytes as UTF-8 by default.
  #
  # Mode selection heuristic: choose the most compact mode that covers the
  # entire input as a single segment. Mixed-mode (segmented) encoding is v0.2.0.

  # Select the most compact encoding mode for the input string.
  # Returns one of: :numeric, :alphanumeric, :byte
  def self.select_mode(input)
    if input.match?(/\A\d*\z/)
      :numeric
    elsif input.chars.all? { |c| ALPHANUM_INDEX.key?(c) }
      :alphanumeric
    else
      :byte
    end
  end

  # Width of the character-count indicator field (bits).
  # This field follows the mode indicator and tells the decoder how many
  # characters (not bytes) follow. Its width depends on both mode and version
  # because larger versions support more data so need a wider count.
  #
  # | Mode         | Versions 1–9 | Versions 10–26 | Versions 27–40 |
  # |--------------|-------------|----------------|----------------|
  # | Numeric      | 10 bits     | 12 bits        | 14 bits        |
  # | Alphanumeric | 9 bits      | 11 bits        | 13 bits        |
  # | Byte         | 8 bits      | 16 bits        | 16 bits        |
  def self.char_count_bits(mode, version)
    case mode
    when :numeric
      if version <= 9
        10
      else
        ((version <= 26) ? 12 : 14)
      end
    when :alphanumeric
      if version <= 9
        9
      else
        ((version <= 26) ? 11 : 13)
      end
    else # :byte
      (version <= 9) ? 8 : 16
    end
  end

  # ===========================================================================
  # BitWriter — accumulate bits, flush as bytes
  # ===========================================================================
  #
  # A simple bit-packing accumulator. Bits are written MSB-first within each
  # byte (the QR standard convention). When to_bytes is called, any trailing
  # partial byte is zero-padded on the right.
  #
  # Example:
  #   w = BitWriter.new
  #   w.write(0b101, 3)    # writes bits: 1 0 1
  #   w.write(0b11001, 5)  # writes bits: 1 1 0 0 1
  #   w.to_bytes           # => [0b10111001] = [0xB9] (8 bits)
  class BitWriter
    def initialize
      @bits = []
    end

    # Append `count` bits from `value`, MSB first.
    # Only the lower `count` bits of value are written.
    def write(value, count)
      (count - 1).downto(0) { |i| @bits << ((value >> i) & 1) }
    end

    # Current number of bits written.
    def bit_length
      @bits.length
    end

    # Convert accumulated bits to a byte array (MSB-first, zero-padded).
    def to_bytes
      bytes = []
      i = 0
      while i < @bits.length
        byte = 0
        8.times { |j| byte = (byte << 1) | (@bits[i + j] || 0) }
        bytes << byte
        i += 8
      end
      bytes
    end
  end

  # ===========================================================================
  # Numeric encoding
  # ===========================================================================
  #
  # Groups of 3 digits → 10 bits (values 0–999, so 10 bits suffice: 2^10=1024)
  # Remaining pair of 2 → 7 bits (values 0–99, 2^7=128)
  # Single remaining digit → 4 bits (values 0–9, 2^4=16)
  #
  # Example: "01234567"
  #   "012" → 12 → 10 bits: 0000001100
  #   "345" → 345 → 10 bits: 0101011001
  #   "67"  → 67 → 7 bits:  1000011
  def self.encode_numeric(input, writer)
    i = 0
    len = input.length
    while i + 2 < len
      writer.write(input[i, 3].to_i, 10)
      i += 3
    end
    if i + 1 < len
      writer.write(input[i, 2].to_i, 7)
      i += 2
    end
    writer.write(input[i].to_i, 4) if i < len
  end

  # ===========================================================================
  # Alphanumeric encoding
  # ===========================================================================
  #
  # Character pairs are encoded together for better compression.
  # The index of each character in ALPHANUM_CHARS is combined:
  #   combined = first_index × 45 + second_index
  # This fits in 11 bits (max: 44×45+44 = 2024 < 2048 = 2^11).
  # A single trailing character just uses its index directly in 6 bits.
  #
  # Example: "AC-3"
  #   'A'=10, 'C'=12 → 10×45+12=462 → 11 bits: 00111001110
  #   '-'=41, '3'=3  → 41×45+3=1848 → 11 bits: 11100111000
  def self.encode_alphanumeric(input, writer)
    i = 0
    len = input.length
    while i + 1 < len
      idx0 = ALPHANUM_INDEX[input[i]]
      idx1 = ALPHANUM_INDEX[input[i + 1]]
      # Precondition: select_mode() must have confirmed every char is in
      # the alphanumeric set. Fail fast here rather than silently corrupt.
      raise QrCodeError, "char '#{input[i]}' not in QR alphanumeric set" if idx0.nil?
      raise QrCodeError, "char '#{input[i + 1]}' not in QR alphanumeric set" if idx1.nil?
      writer.write(idx0 * 45 + idx1, 11)
      i += 2
    end
    if i < len
      idx = ALPHANUM_INDEX[input[i]]
      raise QrCodeError, "char '#{input[i]}' not in QR alphanumeric set" if idx.nil?
      writer.write(idx, 6)
    end
  end

  # ===========================================================================
  # Byte encoding
  # ===========================================================================
  #
  # Each byte of the UTF-8 representation is written as-is (8 bits each).
  # Most modern decoders assume UTF-8 for byte-mode QR codes. To explicitly
  # signal UTF-8, an ECI segment (mode 0111, assignment 26) would precede the
  # data — this is v0.2.0 work; for now we rely on decoder defaults.
  def self.encode_byte(input, writer)
    input.encode("UTF-8").bytes.each { |b| writer.write(b, 8) }
  end

  # ===========================================================================
  # Build data codeword sequence
  # ===========================================================================
  #
  # Assembles the full bit stream and pads it to exactly num_data_codewords bytes.
  #
  # Bit stream format:
  #   [4-bit mode indicator]
  #   [character count indicator — width depends on mode and version]
  #   [encoded data bits]
  #   [terminator: up to 4 zero bits (fewer if near capacity)]
  #   [zero padding to byte boundary]
  #   [pad bytes 0xEC 0x11 alternating until capacity is filled]
  #
  # 0xEC = 11101100 and 0x11 = 00010001 are specified by ISO 18004.
  # Alternating these fills any remaining space in a visually balanced way
  # (roughly equal dark and light modules in the padding area).
  def self.build_data_codewords(input, version, ecc)
    mode = select_mode(input)
    capacity = num_data_codewords(version, ecc)
    w = BitWriter.new

    # Mode indicator (4 bits)
    w.write(MODE_INDICATOR[mode], 4)

    # Character count indicator
    char_count = if mode == :byte
      input.encode("UTF-8").bytesize
    else
      input.length
    end
    w.write(char_count, char_count_bits(mode, version))

    # Encoded data bits
    case mode
    when :numeric then encode_numeric(input, w)
    when :alphanumeric then encode_alphanumeric(input, w)
    else encode_byte(input, w)
    end

    # Terminator: write up to 4 zero bits (fewer if at capacity)
    term_len = [4, capacity * 8 - w.bit_length].min
    w.write(0, term_len) if term_len > 0

    # Pad to byte boundary (write zeros to complete current byte)
    rem = w.bit_length % 8
    w.write(0, 8 - rem) if rem != 0

    # Pad remaining bytes with alternating 0xEC / 0x11
    bytes = w.to_bytes
    pad = 0xEC
    while bytes.length < capacity
      bytes << pad
      pad = (pad == 0xEC) ? 0x11 : 0xEC
    end

    bytes
  end

  # ===========================================================================
  # Block splitting and ECC computation
  # ===========================================================================
  #
  # For most versions, the message codewords are split across multiple blocks.
  # This improves damage resilience: a burst error (e.g., a scratch) that wipes
  # out a contiguous region only destroys one or two blocks. Each block's RS
  # decoder handles its few lost codewords independently.
  #
  # Block layout:
  #   total_blocks = NUM_BLOCKS[ecc_idx][version]
  #   ecc_per_block = ECC_CODEWORDS_PER_BLOCK[ecc_idx][version]
  #   short_len = total_data / total_blocks  (floor division)
  #   num_long  = total_data % total_blocks  (remainder blocks get one extra byte)
  #
  # Group 1: (total_blocks - num_long) blocks, each with short_len data bytes
  # Group 2: num_long blocks, each with (short_len + 1) data bytes
  # (ECC codewords per block is the same for both groups)

  Block = Struct.new(:data, :ecc, keyword_init: true)

  def self.compute_blocks(data, version, ecc)
    e = ECC_IDX[ecc]
    total_blocks = NUM_BLOCKS[e][version]
    ecc_len = ECC_CODEWORDS_PER_BLOCK[e][version]
    total_data = num_data_codewords(version, ecc)
    short_len = total_data / total_blocks
    num_long = total_data % total_blocks
    gen = get_generator(ecc_len)

    blocks = []
    offset = 0

    # Group 1 blocks: short_len data bytes each
    g1_count = total_blocks - num_long
    g1_count.times do
      d = data[offset, short_len]
      blocks << Block.new(data: d, ecc: rs_encode(d, gen))
      offset += short_len
    end

    # Group 2 blocks: (short_len + 1) data bytes each
    num_long.times do
      d = data[offset, short_len + 1]
      blocks << Block.new(data: d, ecc: rs_encode(d, gen))
      offset += short_len + 1
    end

    blocks
  end

  # ===========================================================================
  # Interleaving
  # ===========================================================================
  #
  # After ECC is computed, codewords are interleaved before placement:
  #
  #   1. First data codeword from block 1, then block 2, then block 3, ...
  #   2. Second data codeword from block 1, then block 2, then block 3, ...
  #   ... until all data codewords are placed ...
  #   n. First ECC codeword from block 1, then block 2, then block 3, ...
  #   ... until all ECC codewords are placed ...
  #
  # Why interleave? If data is NOT interleaved and a burst error destroys 10
  # consecutive codewords, all 10 are in one block — which may exceed that
  # block's correction capacity. After interleaving, those 10 codewords come
  # from 10 different blocks, so each block only loses 1 codeword — well within
  # the correction budget.
  def self.interleave_blocks(blocks)
    result = []
    max_data = blocks.map { |b| b.data.length }.max
    max_ecc = blocks.map { |b| b.ecc.length }.max
    max_data.times { |i| blocks.each { |b| result << b.data[i] if i < b.data.length } }
    max_ecc.times { |i| blocks.each { |b| result << b.ecc[i] if i < b.ecc.length } }
    result
  end

  # ===========================================================================
  # Work grid — mutable grid for construction phase
  # ===========================================================================
  #
  # Unlike the immutable ModuleGrid returned by the public API, the work grid
  # is a pair of mutable 2D arrays:
  #   modules[row][col]  — boolean: true = dark
  #   reserved[row][col] — boolean: true = structural (finder/timing/format etc)
  #
  # The reserved array prevents the data/mask placement phase from overwriting
  # structural modules. After construction, the work grid is converted to the
  # immutable public ModuleGrid.
  WorkGrid = Struct.new(:size, :modules, :reserved, keyword_init: true)

  def self.make_work_grid(size)
    WorkGrid.new(
      size: size,
      modules: Array.new(size) { Array.new(size, false) },
      reserved: Array.new(size) { Array.new(size, false) }
    )
  end

  # Set a module value and optionally mark it as reserved.
  def self.set_mod(g, row, col, dark, reserve: false)
    g.modules[row][col] = dark
    g.reserved[row][col] = true if reserve
  end

  # ===========================================================================
  # Finder patterns
  # ===========================================================================
  #
  # Each finder pattern is a 7×7 square with this structure:
  #
  #   ■ ■ ■ ■ ■ ■ ■
  #   ■ □ □ □ □ □ ■
  #   ■ □ ■ ■ ■ □ ■
  #   ■ □ ■ ■ ■ □ ■
  #   ■ □ ■ ■ ■ □ ■
  #   ■ □ □ □ □ □ ■
  #   ■ ■ ■ ■ ■ ■ ■
  #
  # The 1:1:3:1:1 dark:light ratio in every scan direction lets any decoder
  # locate and orient the symbol even under partial occlusion or rotation.
  # Three corners (not four) means a scanner always knows which corner is which:
  # the data corner is the one without a finder pattern.
  def self.place_finder(g, top_row, top_col)
    7.times do |dr|
      7.times do |dc|
        on_border = dr == 0 || dr == 6 || dc == 0 || dc == 6
        in_core = dr.between?(2, 4) && dc.between?(2, 4)
        set_mod(g, top_row + dr, top_col + dc, on_border || in_core, reserve: true)
      end
    end
  end

  # ===========================================================================
  # Alignment patterns
  # ===========================================================================
  #
  # A 5×5 pattern with this structure (dark outer border, light ring, dark center):
  #
  #   ■ ■ ■ ■ ■
  #   ■ □ □ □ ■
  #   ■ □ ■ □ ■
  #   ■ □ □ □ ■
  #   ■ ■ ■ ■ ■
  #
  # Alignment patterns appear in version 2+ at positions defined in
  # ALIGNMENT_POSITIONS. They help decoders correct for perspective distortion
  # and barrel/pincushion warping in larger symbols.
  def self.place_alignment(g, row, col)
    (-2..2).each do |dr|
      (-2..2).each do |dc|
        on_border = dr.abs == 2 || dc.abs == 2
        is_center = dr == 0 && dc == 0
        set_mod(g, row + dr, col + dc, on_border || is_center, reserve: true)
      end
    end
  end

  # Place all alignment patterns for the given version.
  # Uses the cross-product of ALIGNMENT_POSITIONS[version-1] with itself.
  # Skips any center that falls on an already-reserved module (finder or timing).
  def self.place_all_alignments(g, version)
    positions = ALIGNMENT_POSITIONS[version - 1]
    positions.each do |row|
      positions.each do |col|
        next if g.reserved[row][col]  # overlaps finder or timing strip
        place_alignment(g, row, col)
      end
    end
  end

  # ===========================================================================
  # Timing patterns
  # ===========================================================================
  #
  # Two alternating dark/light strips:
  #   Horizontal: row 6, columns 8 to (size-9)
  #   Vertical:   column 6, rows 8 to (size-9)
  #
  # The strips start and end dark (at even indices). They let scanners
  # determine the module size and grid alignment, especially in larger symbols
  # where perspective distortion may cause the module pitch to vary slightly.
  def self.place_timing_strips(g)
    sz = g.size
    (8..sz - 9).each { |c| set_mod(g, 6, c, c.even?, reserve: true) }
    (8..sz - 9).each { |r| set_mod(g, r, 6, r.even?, reserve: true) }
  end

  # ===========================================================================
  # Format information module reservation
  # ===========================================================================
  #
  # The format information is placed in two copies in the symbol, adjacent to
  # the three finder patterns. The positions are reserved now with placeholder
  # false values; actual bits are written after mask selection.
  #
  # Copy 1 (adjacent to top-left finder):
  #   Row 8, cols 0..8 (skip col 6 = timing strip)
  #   Col 8, rows 0..8 (skip row 6 = timing strip)
  #
  # Copy 2:
  #   Col 8, rows (size-7)..(size-1)   [adjacent to bottom-left finder]
  #   Row 8, cols (size-8)..(size-1)  [adjacent to top-right finder]
  def self.reserve_format_info(g)
    sz = g.size
    (0..8).each { |c| g.reserved[8][c] = true unless c == 6 }   # row 8 horizontal
    (0..8).each { |r| g.reserved[r][8] = true unless r == 6 }   # col 8 vertical
    ((sz - 7)..sz - 1).each { |r| g.reserved[r][8] = true }     # copy 2 bottom-left
    ((sz - 8)..sz - 1).each { |c| g.reserved[8][c] = true }     # copy 2 top-right
  end

  # ===========================================================================
  # Format information computation
  # ===========================================================================
  #
  # The 15-bit format string encodes the ECC level and mask pattern index.
  # Construction:
  #   1. 5-bit data = [ECC_indicator (2b)] [mask_pattern (3b)]
  #   2. Multiply by x^10 (left-shift by 10)
  #   3. Divide by G(x) = x^10+x^8+x^5+x^4+x^2+x+1 = 0x537
  #   4. The 10-bit remainder forms the BCH error correction bits
  #   5. Concatenate: 5-bit data || 10-bit remainder = 15 bits
  #   6. XOR with mask 0x5412 to prevent all-zero format info
  #
  # The XOR mask ensures the format information always has some dark modules,
  # preventing ambiguity with the reserved (all-light) state of the positions.
  def self.compute_format_bits(ecc, mask)
    data = (ECC_INDICATOR[ecc] << 3) | mask
    rem = data << 10
    # BCH polynomial division: G(x) = 0x537 = 10100110111 binary
    14.downto(10) do |i|
      rem ^= (0x537 << (i - 10)) if (rem >> i) & 1 != 0
    end
    ((data << 10) | (rem & 0x3FF)) ^ 0x5412
  end

  # ===========================================================================
  # Format information placement (CRITICAL: MSB-first in row 8, left to right)
  # ===========================================================================
  #
  # IMPORTANT BUG WARNING (from lessons.md, 2026-04-23):
  # ISO 18004 places format bits MSB-first (f14→f0) reading left-to-right
  # across row 8. The naive reading order (f0 at left) produces a reversed
  # 15-bit word that appears valid by some naive checks but is rejected by
  # every standard decoder (zbarimg, iPhone camera, ZXing).
  #
  # The CORRECT placement (verified against zbarimg):
  #
  # Copy 1 — adjacent to top-left finder:
  #   Row 8, col 0: bit 14 (MSB)
  #   Row 8, col 1: bit 13
  #   Row 8, col 2: bit 12
  #   Row 8, col 3: bit 11
  #   Row 8, col 4: bit 10
  #   Row 8, col 5: bit 9
  #   Row 8, col 7: bit 8    ← col 6 is timing strip, skip to 7
  #   Row 8, col 8: bit 7    ← corner module
  #   Row 7, col 8: bit 6    ← row 6 is timing strip, start at row 7 going up → 5..0
  #   Row 5, col 8: bit 5    }
  #   Row 4, col 8: bit 4    } going upward (decreasing row index)
  #   Row 3, col 8: bit 3    }
  #   Row 2, col 8: bit 2    }
  #   Row 1, col 8: bit 1    }
  #   Row 0, col 8: bit 0 (LSB)
  #
  # Copy 2:
  #   Row 8, cols (size-8)..(size-1): bits 7→0 going left to right
  #   Col 8, rows (size-7)..(size-1): bits 8→14 going top to bottom
  def self.write_format_info(g, fmt_bits)
    sz = g.size

    # Copy 1 — horizontal strip: row 8, cols 0..5 (MSB first: f14..f9)
    6.times { |i| g.modules[8][i] = ((fmt_bits >> (14 - i)) & 1) == 1 }
    # col 7 gets bit 8
    g.modules[8][7] = ((fmt_bits >> 8) & 1) == 1
    # col 8 gets bit 7 (the corner)
    g.modules[8][8] = ((fmt_bits >> 7) & 1) == 1

    # Copy 1 — vertical strip: col 8, rows 7..0 (bit 6 at row 7, bit 0 at row 0)
    # Row 6 is the timing strip — skip from row 7 directly to row 5
    g.modules[7][8] = ((fmt_bits >> 6) & 1) == 1
    6.times { |i| g.modules[5 - i][8] = ((fmt_bits >> (5 - i)) & 1) == 1 }

    # Copy 2 — bottom-left: col 8, rows (size-7)..(size-1) (bits 8..14)
    7.times { |i| g.modules[sz - 7 + i][8] = ((fmt_bits >> (8 + i)) & 1) == 1 }
    # Copy 2 — top-right: row 8, cols (size-8)..(size-1) (bits 7..0)
    8.times { |i| g.modules[8][sz - 8 + i] = ((fmt_bits >> (7 - i)) & 1) == 1 }
  end

  # ===========================================================================
  # Version information (v7+)
  # ===========================================================================
  #
  # Versions 7–40 embed an 18-bit version number in two 6×3 blocks.
  # Construction:
  #   1. 6-bit version number
  #   2. BCH(18,6): G(x) = x^12+x^11+x^10+x^9+x^8+x^5+x^2+1 = 0x1F25
  #   3. Concatenate: 6-bit version || 12-bit BCH = 18 bits
  #
  # Two copies:
  #   Top-right:   rows 0..5, cols (size-11)..(size-9)
  #   Bottom-left: rows (size-11)..(size-9), cols 0..5
  #
  # The two copies are transposed relative to each other.

  def self.reserve_version_info(g, version)
    return if version < 7
    sz = g.size
    6.times { |r| 3.times { |dc| g.reserved[r][sz - 11 + dc] = true } }
    3.times { |dr| 6.times { |c| g.reserved[sz - 11 + dr][c] = true } }
  end

  def self.compute_version_bits(version)
    rem = version << 12
    # G(x) = 0x1F25 = 001111100100101 binary (degree 12)
    17.downto(12) { |i| rem ^= (0x1F25 << (i - 12)) if (rem >> i) & 1 != 0 }
    (version << 12) | (rem & 0xFFF)
  end

  # Write version information into both 6×3 blocks.
  # Bit i → top-right block: (5 - i/3, size-9 - i%3)
  #        bottom-left block (transposed): (size-9 - i%3, 5 - i/3)
  def self.write_version_info(g, version)
    return if version < 7
    sz = g.size
    bits = compute_version_bits(version)
    18.times do |i|
      dark = ((bits >> i) & 1) == 1
      a = 5 - (i / 3)
      b = sz - 9 - (i % 3)
      g.modules[a][b] = dark
      g.modules[b][a] = dark
    end
  end

  # ===========================================================================
  # Dark module
  # ===========================================================================
  #
  # A single always-dark module at position (4V+9, 8). This module is always
  # set to dark, never masked, and is not part of any data. It ensures that
  # the format information region always has at least one dark module, which
  # helps decoders distinguish an initialized symbol from an unformatted grid.
  def self.place_dark_module(g, version)
    set_mod(g, 4 * version + 9, 8, true, reserve: true)
  end

  # ===========================================================================
  # Grid initialization
  # ===========================================================================
  #
  # Assembles all structural elements into the work grid. The order matters:
  #   1. Finder patterns (7×7 at three corners)
  #   2. Separators (1-module light border around each finder)
  #   3. Timing strips (alternating on row 6 and col 6)
  #   4. Alignment patterns (5×5, version-specific)
  #   5. Reserve format information positions (2 copies, 15 bits each)
  #   6. Reserve version information positions (v7+, 2 copies, 6×3 each)
  #   7. Dark module at (4V+9, 8)
  def self.build_grid(version)
    sz = symbol_size(version)
    g = make_work_grid(sz)

    # Three finder patterns at the top-left, top-right, and bottom-left corners.
    place_finder(g, 0, 0)          # top-left
    place_finder(g, 0, sz - 7)     # top-right
    place_finder(g, sz - 7, 0)     # bottom-left

    # Separators: 1-module-wide light border just outside each finder pattern.
    # These isolate the finder from the data area so decoders don't confuse
    # data modules for part of the finder pattern.
    #
    # Top-left separator: row 7 (horizontal) and col 7 (vertical)
    8.times do |i|
      set_mod(g, 7, i, false, reserve: true)        # row 7, cols 0..7
      set_mod(g, i, 7, false, reserve: true)        # col 7, rows 0..7
    end
    # Top-right separator: row 7 (horizontal) and col sz-8 (vertical)
    8.times do |i|
      set_mod(g, 7, sz - 1 - i, false, reserve: true)    # row 7, right side
      set_mod(g, i, sz - 8, false, reserve: true)         # col sz-8, rows 0..7
    end
    # Bottom-left separator: row sz-8 (horizontal) and col 7 (vertical)
    8.times do |i|
      set_mod(g, sz - 8, i, false, reserve: true)         # row sz-8, cols 0..7
      set_mod(g, sz - 1 - i, 7, false, reserve: true)     # col 7, bottom side
    end

    # Timing strips must come before alignment patterns (row/col 6 reserved first)
    place_timing_strips(g)
    place_all_alignments(g, version)

    reserve_format_info(g)
    reserve_version_info(g, version)
    place_dark_module(g, version)

    g
  end

  # ===========================================================================
  # Zigzag data placement
  # ===========================================================================
  #
  # The interleaved codeword bit stream is placed into the non-reserved modules
  # using a two-column zigzag scan, starting from the bottom-right corner and
  # moving upward.
  #
  # Algorithm:
  #   current_col = size - 1     (start from rightmost column)
  #   direction = upward         (alternates after each 2-column strip)
  #
  #   Loop:
  #     For each row in current direction (bottom→top or top→bottom):
  #       Place bit at (row, current_col) if not reserved
  #       Place bit at (row, current_col - 1) if not reserved and col != 6
  #     Flip direction; move current_col left by 2
  #     If current_col == 6: skip by 1 more (the vertical timing strip is col 6)
  #     Stop when current_col < 1
  #
  # This zigzag pattern ensures adjacent bits in the stream are placed in
  # adjacent modules, maximizing locality for burst-error correction.
  #
  # IMPORTANT: col 6 (the vertical timing strip) is skipped globally.
  # When the sweep column index lands on 7, the paired column would be 6;
  # we skip col 6 but still process col 7. When moving left past col 6,
  # we jump from col 7 to col 5 directly.
  def self.place_bits(g, codewords, version)
    sz = g.size

    # Flatten codewords to a bit array (MSB first per codeword)
    bits = []
    codewords.each { |cw| 7.downto(0) { |b| bits << (((cw >> b) & 1) == 1) } }
    # Append remainder bits (zeros)
    num_remainder_bits(version).times { bits << false }

    bit_idx = 0
    up = true         # true = bottom-to-top scan
    col = sz - 1      # leading column (the rightmost one of each pair)

    while col >= 1
      sz.times do |vi|
        row = up ? (sz - 1 - vi) : vi
        [0, 1].each do |dc|
          c = col - dc
          next if c == 6          # skip the vertical timing strip column
          next if g.reserved[row][c]
          g.modules[row][c] = (bit_idx < bits.length) ? bits[bit_idx] : false
          bit_idx += 1
        end
      end
      up = !up
      col -= 2
      col -= 1 if col == 6  # hop over the vertical timing strip
    end
  end

  # ===========================================================================
  # Mask patterns (ISO 18004:2015 Table 10)
  # ===========================================================================
  #
  # Each mask pattern is a condition on (row, col). If the condition is true,
  # that module's bit is flipped (dark ↔ light). Only data/ECC modules are
  # masked; structural modules are never flipped.
  #
  # The purpose of masking is to prevent degenerate patterns:
  #   - Large solid areas (all-dark or all-light) confuse scanners
  #   - Finder-pattern-like sequences in data confuse the locator
  #   - Long runs of one color confuse the scan calibration
  #
  # Pattern 0: checkerboard (every other module)
  # Pattern 1: every other row
  # Pattern 2: every third column
  # Pattern 3: alternating diagonals
  # Pattern 4: 2×3 rectangular blocks
  # Pattern 5: based on row*col product
  # Pattern 6: like pattern 5 but modulo 2
  # Pattern 7: mixed diagonal and product
  MASK_CONDS = [
    ->(r, c) { (r + c) % 2 == 0 },
    ->(r, _c) { r % 2 == 0 },
    ->(_r, c) { c % 3 == 0 },
    ->(r, c) { (r + c) % 3 == 0 },
    ->(r, c) { (r / 2 + c / 3) % 2 == 0 },
    ->(r, c) { (r * c) % 2 + (r * c) % 3 == 0 },
    ->(r, c) { ((r * c) % 2 + (r * c) % 3) % 2 == 0 },
    ->(r, c) { ((r + c) % 2 + (r * c) % 3) % 2 == 0 }
  ].freeze

  # Apply a mask pattern to the module grid.
  # Returns a NEW array (does not modify the work grid's modules in place).
  # Only non-reserved modules are flipped.
  def self.apply_mask(modules, reserved, sz, mask_idx)
    cond = MASK_CONDS[mask_idx]
    result = Array.new(sz) { |r| Array.new(sz) { |c| modules[r][c] } }
    sz.times do |r|
      sz.times do |c|
        next if reserved[r][c]
        result[r][c] = result[r][c] ^ cond.call(r, c)
      end
    end
    result
  end

  # ===========================================================================
  # Penalty scoring (ISO 18004:2015 Section 7.8.3)
  # ===========================================================================
  #
  # After applying each mask, we score the result with four penalty rules.
  # The mask with the LOWEST total penalty is selected for the final symbol.
  # This ensures the encoded data avoids patterns that confuse scanners.
  #
  # The four rules and their rationale:
  #
  # RULE 1 — Long runs of the same color:
  #   A run of ≥5 same-color modules in a row or column is penalized.
  #   Score += (run_length - 2) for each such run.
  #   Why: long runs resemble timing strip patterns and confuse scan calibration.
  #
  # RULE 2 — 2×2 solid blocks:
  #   Any 2×2 block where all four modules are the same color: score += 3.
  #   Why: large solid areas confuse edge detection in the scanner.
  #
  # RULE 3 — Finder-pattern-like sequences:
  #   The pattern 1,0,1,1,1,0,1,0,0,0,0 (or its reverse) in any row or column.
  #   Each occurrence: score += 40.
  #   Why: this 11-module pattern resembles a finder pattern scanning sequence
  #   and could cause the locator to find false positives.
  #
  # RULE 4 — Dark module ratio:
  #   The proportion of dark modules should be close to 50% for maximum contrast.
  #   Penalty is proportional to the deviation from 50% in 5% steps.
  #   Why: heavy imbalance (e.g., 70% dark) reduces contrast for scanning.
  def self.compute_penalty(modules, sz)
    penalty = 0

    # Rule 1 — runs of ≥5 consecutive same-color modules
    sz.times do |r|
      # Check both horizontal (horiz=true) and vertical (horiz=false)
      [true, false].each do |horiz|
        run = 1
        prev = horiz ? modules[r][0] : modules[0][r]
        (1...sz).each do |i|
          cur = horiz ? modules[r][i] : modules[i][r]
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

    # Rule 2 — 2×2 same-color blocks
    (sz - 1).times do |r|
      (sz - 1).times do |c|
        d = modules[r][c]
        if d == modules[r][c + 1] && d == modules[r + 1][c] && d == modules[r + 1][c + 1]
          penalty += 3
        end
      end
    end

    # Rule 3 — finder-pattern-like sequences
    # The pattern and its reverse both penalize. Both horizontal and vertical.
    p1 = [1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0]
    p2 = [0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1]
    sz.times do |a|
      (sz - 11 + 1).times do |b|
        mh1 = mh2 = mv1 = mv2 = true
        11.times do |k|
          bh = modules[a][b + k] ? 1 : 0
          bv = modules[b + k][a] ? 1 : 0
          mh1 = false unless bh == p1[k]
          mh2 = false unless bh == p2[k]
          mv1 = false unless bv == p1[k]
          mv2 = false unless bv == p2[k]
        end
        penalty += 40 if mh1
        penalty += 40 if mh2
        penalty += 40 if mv1
        penalty += 40 if mv2
      end
    end

    # Rule 4 — dark module ratio
    dark = 0
    sz.times { |r| sz.times { |c| dark += 1 if modules[r][c] } }
    ratio = dark.to_f / (sz * sz) * 100.0
    prev5 = (ratio / 5).floor * 5
    penalty += [(prev5 - 50).abs, (prev5 + 5 - 50).abs].min / 5 * 10

    penalty
  end

  # ===========================================================================
  # Version selection
  # ===========================================================================
  #
  # Find the minimum version (1–40) whose data codeword capacity fits the input.
  # The capacity check uses the exact bit count including mode indicator and
  # character-count field width (which varies with version).
  #
  # For byte mode with UTF-8: uses the encoded byte length, not the string length.
  # For numeric mode: 10 bits per group of 3, 7 bits for pair, 4 for single.
  # For alphanumeric mode: 11 bits per pair, 6 bits for single.
  #
  # The simplified formula below slightly overestimates capacity for boundary
  # cases but is exact in practice for typical inputs.
  def self.select_version(input, ecc)
    mode = select_mode(input)
    byte_len = input.encode("UTF-8").bytesize

    (1..40).each do |v|
      capacity = num_data_codewords(v, ecc)
      data_bits = case mode
      when :numeric
        n3 = input.length / 3
        n2 = (input.length % 3 >= 2) ? 1 : 0
        n1 = (input.length % 3 == 1) ? 1 : 0
        n3 * 10 + n2 * 7 + n1 * 4
      when :alphanumeric
        (input.length / 2) * 11 + (input.length.odd? ? 6 : 0)
      else
        byte_len * 8
      end
      bits_needed = 4 + char_count_bits(mode, v) + data_bits
      return v if (bits_needed.to_f / 8).ceil <= capacity
    end

    raise InputTooLongError,
      "Input (#{input.length} chars, ECC=#{ecc}) exceeds version 40 capacity."
  end

  # ===========================================================================
  # Public API
  # ===========================================================================

  # Encode a string into a QR Code and return a ModuleGrid.
  #
  # The grid is a (4V+17) × (4V+17) boolean grid: true = dark module.
  # Use Barcode2D.layout(grid) to convert to a pixel-resolved PaintScene.
  #
  # Parameters:
  #   data    — input string (UTF-8); any length up to QR v40 capacity
  #   level:  — ECC level (:L, :M, :Q, :H). Default :M
  #   version: — specific version (1–40), or 0 for auto-select. Default 0
  #   mode:   — specific mode (:numeric, :alphanumeric, :byte), or nil for auto. Default nil
  #
  # Raises InputTooLongError if data exceeds version 40 capacity.
  #
  # Example:
  #   grid = QrCode.encode("https://example.com", level: :M)
  #   grid.rows  # => 29 (version 3, 29×29)
  #   grid.cols  # => 29
  def self.encode(data, level: :M, version: 0, mode: nil)
    # Early-exit guard to prevent O(40n) memory allocation on huge inputs
    # before throwing InputTooLongError. QR v40 byte mode max: 2953 bytes.
    if data.encode("UTF-8").bytesize > 7089
      raise InputTooLongError,
        "Input exceeds 7089 bytes (QR Code v40 maximum in numeric mode)."
    end

    v = if version > 0
      version
    else
      select_version(data, level)
    end

    sz = symbol_size(v)

    data_cw = build_data_codewords(data, v, level)
    blocks = compute_blocks(data_cw, v, level)
    interleaved = interleave_blocks(blocks)

    grid = build_grid(v)
    place_bits(grid, interleaved, v)

    # Evaluate all 8 masks; pick the one with the lowest penalty score.
    best_mask = 0
    best_penalty = Float::INFINITY

    8.times do |m|
      masked = apply_mask(grid.modules, grid.reserved, sz, m)
      fmt_bits = compute_format_bits(level, m)
      # Write format info into a temporary copy to include in penalty scoring
      temp = make_work_grid(sz)
      temp.modules = masked.map(&:dup)
      temp.reserved = grid.reserved
      write_format_info(temp, fmt_bits)
      p = compute_penalty(temp.modules, sz)
      if p < best_penalty
        best_penalty = p
        best_mask = m
      end
    end

    # Finalize: apply best mask and write final format + version information
    final_mods = apply_mask(grid.modules, grid.reserved, sz, best_mask)
    final_g = make_work_grid(sz)
    final_g.modules = final_mods
    final_g.reserved = grid.reserved

    write_format_info(final_g, compute_format_bits(level, best_mask))
    write_version_info(final_g, v)

    # Convert mutable work grid to immutable public ModuleGrid
    frozen_mods = final_g.modules.map { |row| row.freeze }.freeze
    CodingAdventures::Barcode2D::ModuleGrid.new(
      rows: sz,
      cols: sz,
      modules: frozen_mods,
      module_shape: "square"
    ).freeze
  end

  # Encode a string and convert to a pixel-resolved PaintScene.
  #
  # Delegates pixel geometry (module size, quiet zone, colours) to
  # Barcode2D.layout(). Accepts the same parameters as encode() plus an
  # optional config hash for layout settings.
  #
  # Example:
  #   scene = QrCode.encode_to_scene("HELLO", level: :H)
  #   # scene is a PaintScene ready for PaintVM (SVG, Metal, etc.)
  def self.encode_to_scene(data, level: :M, version: 0, mode: nil, config: nil)
    grid = encode(data, level: level, version: version, mode: mode)
    CodingAdventures::Barcode2D.layout(grid, config)
  end
end
