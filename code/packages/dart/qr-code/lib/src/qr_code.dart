/// QR Code encoder — ISO/IEC 18004:2015 compliant.
///
/// ## What is a QR Code?
///
/// QR Code (Quick Response Code) was invented in 1994 by Masahiro Hara at
/// Denso Wave to track automotive parts. A QR Code is a square grid of dark
/// and light modules (small squares). Scanners use the three distinctive
/// "finder patterns" at the corners to locate and orient the symbol, then read
/// the data from the remaining modules.
///
/// ## What this library does
///
/// Given any UTF-8 string and an error-correction level (L/M/Q/H), this library
/// produces a [ModuleGrid] — an abstract boolean grid, `true` = dark module.
/// Pass the grid to `barcode-2d`'s `layout()` for pixel rendering, or call
/// [encodeAndLayout] for a one-shot convenience function.
///
/// ## Encoding pipeline overview
///
/// ```
/// input string
///   → mode selection     (numeric / alphanumeric / byte — pick smallest)
///   → version selection  (v1–v40 — pick minimum that fits at chosen ECC)
///   → bit stream         (mode indicator + char count + data + padding)
///   → blocks + RS ECC    (GF(256) b=0 convention, polynomial 0x11D)
///   → interleave         (round-robin data CWs, then ECC CWs)
///   → grid init          (finder × 3, separators, timing, alignment, dark)
///   → zigzag placement   (two-column snake scan from bottom-right)
///   → mask evaluation    (8 patterns, 4-rule penalty score, pick lowest)
///   → finalize           (format info + version info for v7+)
///   → ModuleGrid
/// ```
///
/// ## Error correction levels
///
/// | Level | Recovery capacity | Typical use |
/// |-------|------------------|-------------|
/// | L     | ~7% of codewords  | Large symbols, good scan conditions |
/// | M     | ~15%             | General purpose (most common) |
/// | Q     | ~25%             | Moderate damage expected |
/// | H     | ~30%             | Small symbols, damage-prone surfaces |
///
/// ## Why Reed-Solomon ECC?
///
/// A QR Code printed on a package can be torn, smudged, or partially covered.
/// Reed-Solomon error correction lets a scanner reconstruct the original message
/// even when up to 30% of the modules are unreadable. The ECC works by
/// computing redundant "check bytes" using polynomial arithmetic over GF(256),
/// then interleaving them with the data so that burst damage (which destroys a
/// contiguous region) only wipes out a fraction of any one block.
library qr_code;

import 'package:coding_adventures_gf256/coding_adventures_gf256.dart' as gf;
import 'package:coding_adventures_barcode_2d/coding_adventures_barcode_2d.dart';

// Re-export barcode-2d types so callers can use them without a separate import.
export 'package:coding_adventures_barcode_2d/coding_adventures_barcode_2d.dart'
    show ModuleGrid, ModuleShape, PaintScene, Barcode2DLayoutConfig, defaultBarcode2DLayoutConfig;

/// Package version following Semantic Versioning 2.0.
const String version = '0.1.0';

// ============================================================================
// EccLevel
// ============================================================================

/// Error correction level. Higher levels add more redundancy at the cost of
/// reduced data capacity.
///
/// ISO/IEC 18004 uses 2-bit indicators encoded into the format information:
///
/// | Level | Indicator | Recovery |
/// |-------|-----------|----------|
/// | L     | 01        | ~7%      |
/// | M     | 00        | ~15%     |
/// | Q     | 11        | ~25%     |
/// | H     | 10        | ~30%     |
///
/// The non-obvious ordering (L=01, M=00) is a deliberate design choice in the
/// ISO standard — probably to give common cases (M) a low Hamming weight in
/// the format bits, reducing the chance of scanner misidentification.
enum EccLevel {
  /// ~7% codeword recovery — use when scanning conditions are ideal.
  l,

  /// ~15% codeword recovery — the most common default.
  m,

  /// ~25% codeword recovery — for labels that may be partially covered.
  q,

  /// ~30% codeword recovery — maximum redundancy at reduced capacity.
  h,
}

// ============================================================================
// QRCodeError
// ============================================================================

/// Base class for errors produced by the QR Code encoder.
///
/// Use `is QRCodeError` to catch any QR encoding error regardless of subtype.
abstract class QRCodeError implements Exception {
  /// Human-readable description of the error.
  final String message;
  const QRCodeError(this.message);
  @override
  String toString() => 'QRCodeError: $message';
}

/// Thrown when the input string is too long to fit in any version (1–40) at
/// the chosen ECC level.
///
/// The maximum capacity is ~7089 numeric chars or ~2953 bytes at level L.
final class InputTooLongError extends QRCodeError {
  const InputTooLongError(super.message);
  @override
  String toString() => 'InputTooLongError: $message';
}

/// Thrown when layout configuration is invalid (e.g. moduleSizePx <= 0).
final class QRLayoutError extends QRCodeError {
  const QRLayoutError(super.message);
  @override
  String toString() => 'QRLayoutError: $message';
}

// ============================================================================
// Internal: ECC level index helpers
// ============================================================================

/// 2-bit format information indicator for each ECC level.
///
/// These specific values are defined by ISO/IEC 18004 Annex C. The format
/// information word embeds these bits so scanners know which mask and ECC level
/// were used.
int _eccIndicator(EccLevel ecc) => switch (ecc) {
  EccLevel.l => 0x01,
  EccLevel.m => 0x00,
  EccLevel.q => 0x03,
  EccLevel.h => 0x02,
};

/// Zero-based index into the capacity tables (0=L, 1=M, 2=Q, 3=H).
int _eccIdx(EccLevel ecc) => switch (ecc) {
  EccLevel.l => 0,
  EccLevel.m => 1,
  EccLevel.q => 2,
  EccLevel.h => 3,
};

// ============================================================================
// ISO 18004:2015 — ECC codewords per block (Table 9)
// ============================================================================
//
// Each entry is the number of ECC codewords produced per block for a given
// version (index 1–40) and ECC level. Index 0 is a dummy (-1).
//
// These values come directly from Table 9 of ISO/IEC 18004:2015. They are
// NOT computed — they are fixed by the standard and embedded as a lookup table.
// Every QR Code scanner in the world uses the same values.

/// ECC codewords per block, indexed [eccIdx][version], version range 1–40.
/// Index 0 is a placeholder (-1).
const List<List<int>> _eccCwPerBlock = [
  // L:  0    1    2    3    4    5    6    7    8    9   10   11   12   13   14   15   16   17   18   19   20   21   22   23   24   25   26   27   28   29   30   31   32   33   34   35   36   37   38   39   40
  [-1,  7,  10,  15,  20,  26,  18,  20,  24,  30,  18,  20,  24,  26,  30,  22,  24,  28,  30,  28,  28,  28,  28,  30,  30,  26,  28,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30],
  // M:  0    1    2    3    4    5    6    7    8    9   10   11   12   13   14   15   16   17   18   19   20   21   22   23   24   25   26   27   28   29   30   31   32   33   34   35   36   37   38   39   40
  [-1, 10,  16,  26,  18,  24,  16,  18,  22,  22,  26,  30,  22,  22,  24,  24,  28,  28,  26,  26,  26,  26,  28,  28,  28,  28,  28,  28,  28,  28,  28,  28,  28,  28,  28,  28,  28,  28,  28,  28,  28],
  // Q:  0    1    2    3    4    5    6    7    8    9   10   11   12   13   14   15   16   17   18   19   20   21   22   23   24   25   26   27   28   29   30   31   32   33   34   35   36   37   38   39   40
  [-1, 13,  22,  18,  26,  18,  24,  18,  22,  20,  24,  28,  26,  24,  20,  30,  24,  28,  28,  26,  30,  28,  30,  30,  30,  30,  28,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30],
  // H:  0    1    2    3    4    5    6    7    8    9   10   11   12   13   14   15   16   17   18   19   20   21   22   23   24   25   26   27   28   29   30   31   32   33   34   35   36   37   38   39   40
  [-1, 17,  28,  22,  16,  22,  28,  26,  26,  24,  28,  24,  28,  22,  24,  24,  30,  28,  28,  26,  28,  30,  24,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30],
];

/// Total number of error correction blocks, indexed [eccIdx][version].
const List<List<int>> _numBlocks = [
  // L:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
  [-1,  1,  1,  1,  1,  1,  2,  2,  2,  2,  4,  4,  4,  4,  4,  6,  6,  6,  6,  7,  8,  8,  9,  9, 10, 12, 12, 12, 13, 14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25],
  // M:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
  [-1,  1,  1,  1,  2,  2,  4,  4,  4,  5,  5,  5,  8,  9,  9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21, 23, 25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49],
  // Q:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
  [-1,  1,  1,  2,  2,  4,  4,  6,  6,  8,  8,  8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27, 29, 34, 34, 35, 38, 40, 43, 45, 48, 51, 53, 56, 59, 62, 65, 68],
  // H:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
  [-1,  1,  1,  2,  4,  4,  4,  5,  6,  8,  8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32, 35, 37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 80],
];

// ============================================================================
// Alignment pattern center positions (ISO 18004 Annex E)
// ============================================================================
//
// For version V, `_alignmentPositions[V-1]` gives the list of row/column
// center coordinates for alignment patterns. The full grid of alignment
// pattern centers is every combination of (row, col) from this list, EXCLUDING
// positions that overlap with a finder pattern.
//
// Version 1 has no alignment patterns (empty list).
// Version 2 has one pattern centered at (18, 18) — the single combination.
// Larger versions have more combinations.

const List<List<int>> _alignmentPositions = [
  [],                                   // v1  — no alignment patterns
  [6, 18],                              // v2
  [6, 22],                              // v3
  [6, 26],                              // v4
  [6, 30],                              // v5
  [6, 34],                              // v6
  [6, 22, 38],                          // v7
  [6, 24, 42],                          // v8
  [6, 26, 46],                          // v9
  [6, 28, 50],                          // v10
  [6, 30, 54],                          // v11
  [6, 32, 58],                          // v12
  [6, 34, 62],                          // v13
  [6, 26, 46, 66],                      // v14
  [6, 26, 48, 70],                      // v15
  [6, 26, 50, 74],                      // v16
  [6, 30, 54, 78],                      // v17
  [6, 30, 56, 82],                      // v18
  [6, 30, 58, 86],                      // v19
  [6, 34, 62, 90],                      // v20
  [6, 28, 50, 72, 94],                  // v21
  [6, 26, 50, 74, 98],                  // v22
  [6, 30, 54, 78, 102],                 // v23
  [6, 28, 54, 80, 106],                 // v24
  [6, 32, 58, 84, 110],                 // v25
  [6, 30, 58, 86, 114],                 // v26
  [6, 34, 62, 90, 118],                 // v27
  [6, 26, 50, 74, 98, 122],             // v28
  [6, 30, 54, 78, 102, 126],            // v29
  [6, 26, 52, 78, 104, 130],            // v30
  [6, 30, 56, 82, 108, 134],            // v31
  [6, 34, 60, 86, 112, 138],            // v32
  [6, 30, 58, 86, 114, 142],            // v33
  [6, 34, 62, 90, 118, 146],            // v34
  [6, 30, 54, 78, 102, 126, 150],       // v35
  [6, 24, 50, 76, 102, 128, 154],       // v36
  [6, 28, 54, 80, 106, 132, 158],       // v37
  [6, 32, 58, 84, 110, 136, 162],       // v38
  [6, 26, 54, 82, 110, 138, 166],       // v39
  [6, 30, 58, 86, 114, 142, 170],       // v40
];

// ============================================================================
// Grid geometry helpers
// ============================================================================

/// Symbol size in modules for the given version.
///
/// A QR Code version V has a (4V + 17) × (4V + 17) grid.
///
/// ```
/// Version 1:  21×21   (4×1 + 17 = 21)
/// Version 2:  25×25   (4×2 + 17 = 25)
/// Version 10: 57×57   (4×10 + 17 = 57)
/// Version 40: 177×177 (4×40 + 17 = 177)
/// ```
int _symbolSize(int version) => 4 * version + 17;

/// Total number of raw data + ECC bits available in a version-V symbol.
///
/// This is the number of non-reserved modules in the grid. It includes data
/// bits AND ECC bits — the caller subtracts ECC to get data capacity.
///
/// Formula from Nayuki's QR Code generator (public domain derivation from ISO):
///
/// ```
/// rawBits = (16V + 128)V + 64
///         - alignment_overhead   (if V >= 2)
///         - version_info_overhead (if V >= 7)
/// ```
int _numRawDataModules(int version) {
  var result = (16 * version + 128) * version + 64;
  if (version >= 2) {
    final numAlign = version ~/ 7 + 2;
    result -= (25 * numAlign - 10) * numAlign - 55;
    if (version >= 7) result -= 36;
  }
  return result;
}

/// Number of remainder bits for a given version.
///
/// After placing all codeword bits, the zigzag placement may have a few
/// leftover bit positions (0–7 remainder bits depending on version). These
/// are filled with zero-bit padding and do not carry data.
///
/// Remainder bits exist because the total module count is not always a
/// multiple of 8.
int _numRemainderBits(int version) => _numRawDataModules(version) % 8;

/// Number of data codewords (bytes) available at a given version and ECC level.
///
/// Total raw codewords = raw bits ÷ 8.
/// ECC codewords are reserved for error correction.
/// Data codewords = total - ECC.
int _numDataCodewords(int version, EccLevel ecc) {
  final e = _eccIdx(ecc);
  final rawCw = _numRawDataModules(version) ~/ 8;
  final eccCw = _numBlocks[e][version] * _eccCwPerBlock[e][version];
  return rawCw - eccCw;
}

// ============================================================================
// Reed-Solomon encoder (b=0 convention)
// ============================================================================
//
// QR Code RS uses the b=0 convention: the generator polynomial has roots
// α^0, α^1, ..., α^{n-1}, where α = 2 (the primitive element of GF(256)).
//
//   g(x) = ∏(x + α^i)  for i in 0..n−1
//
// This differs from the b=1 convention (used in some other RS implementations)
// by one shift. Using the wrong convention produces ECC bytes that appear
// structurally valid but cause decoding failures.
//
// The RS encoder runs a polynomial long-division LFSR (Linear Feedback Shift
// Register). The input data polynomial D(x) is divided by g(x), and the
// n-byte remainder R(x) = D(x)·x^n mod g(x) forms the ECC codewords.
//
// LFSR algorithm:
//   rem = [0, 0, ..., 0]   (n zeros)
//   for each data byte b:
//     feedback = b XOR rem[0]
//     shift rem left (drop rem[0], shift 1..n-1 to 0..n-2, append 0)
//     for i in 0..n-1:
//       rem[i] ^= gf_mul(generator[i+1], feedback)
//   result = rem

/// Build the monic RS generator polynomial of degree n with roots
/// α^0, α^1, ..., α^{n-1}.
///
/// Returns a list of n+1 coefficients (index 0 = leading coefficient = 1).
///
/// Construction: start with g = [1], then multiply by (x + α^i) for each i:
///
/// ```
/// g · (x + α^i)  =  [g₀, g₁, ..., gₖ] · [1, α^i]
///                 =  [g₀, g₀·α^i + g₁, g₁·α^i + g₂, ..., gₖ·α^i]
/// ```
List<int> _buildGenerator(int n) {
  // Start with the degree-0 polynomial "1".
  var g = [1];
  for (var i = 0; i < n; i++) {
    // α^i in GF(256) — primitive element α = 2.
    final ai = gf.gfPower(2, i); // 2^i mod 0x11D
    final next = List<int>.filled(g.length + 1, 0);
    for (var j = 0; j < g.length; j++) {
      next[j] ^= g[j];                        // multiply by x
      next[j + 1] ^= gf.gfMultiply(g[j], ai); // multiply by α^i
    }
    g = next;
  }
  return g;
}

/// Compute n ECC bytes using LFSR polynomial division.
///
/// Returns the n-byte remainder of D(x)·x^n mod G(x), where G is the
/// generator polynomial built by [_buildGenerator].
List<int> _rsEncode(List<int> data, List<int> generator) {
  final n = generator.length - 1;
  final rem = List<int>.filled(n, 0);
  for (final b in data) {
    final fb = b ^ rem[0];
    // Shift remainder left: discard rem[0], shift all left by 1.
    for (var i = 0; i < n - 1; i++) rem[i] = rem[i + 1];
    rem[n - 1] = 0;
    if (fb != 0) {
      for (var i = 0; i < n; i++) {
        rem[i] ^= gf.gfMultiply(generator[i + 1], fb);
      }
    }
  }
  return rem;
}

// ============================================================================
// Data encoding modes
// ============================================================================
//
// QR supports three data encoding modes (this implementation does not cover
// the rarer Kanji or ECI modes):
//
// NUMERIC mode:       Only digits 0–9. Packs three digits into 10 bits.
// ALPHANUMERIC mode:  Digits, A–Z, space, and $ % * + - . / :
//                     Packs two characters into 11 bits via a 45-char alphabet.
// BYTE mode:          Any bytes (UTF-8). Each byte uses 8 bits.
//
// Mode selection heuristic (v0.1.0):
//   1. If all chars are digits → numeric (most compact)
//   2. Else if all chars are in the 45-char alphanumeric set → alphanumeric
//   3. Otherwise → byte (always valid)

/// The 45-character alphanumeric alphabet, ordered by index.
///
/// Character c gets index `_alphanumChars.indexOf(c)`. Pair (c1, c2) encodes
/// to `index(c1) * 45 + index(c2)` packed into 11 bits.
const String _alphanumChars = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ \$%*+-./:';

/// The three data encoding modes.
enum _EncodingMode {
  numeric,
  alphanumeric,
  byte,
}

/// Select the most compact mode that can represent [input].
_EncodingMode _selectMode(String input) {
  if (input.isEmpty) return _EncodingMode.byte; // empty → byte

  // Numeric: every character is a digit.
  if (input.runes.every((r) => r >= 0x30 && r <= 0x39)) {
    return _EncodingMode.numeric;
  }
  // Alphanumeric: every character is in the 45-char set.
  if (input.runes.every((r) => _alphanumChars.contains(String.fromCharCode(r)))) {
    return _EncodingMode.alphanumeric;
  }
  // Byte: universal fallback.
  return _EncodingMode.byte;
}

/// 4-bit mode indicator for each mode.
///
/// These values are defined by ISO/IEC 18004 Table 2.
int _modeIndicator(_EncodingMode mode) => switch (mode) {
  _EncodingMode.numeric     => 0x1, // 0001
  _EncodingMode.alphanumeric => 0x2, // 0010
  _EncodingMode.byte        => 0x4, // 0100
};

/// Width of the character count field in bits.
///
/// The count field width varies by mode and version group:
///
/// | Mode         | V1–9  | V10–26 | V27–40 |
/// |--------------|-------|--------|--------|
/// | Numeric      | 10    | 12     | 14     |
/// | Alphanumeric | 9     | 11     | 13     |
/// | Byte         | 8     | 16     | 16     |
int _charCountBits(_EncodingMode mode, int version) {
  return switch (mode) {
    _EncodingMode.numeric => version <= 9 ? 10 : (version <= 26 ? 12 : 14),
    _EncodingMode.alphanumeric => version <= 9 ? 9 : (version <= 26 ? 11 : 13),
    _EncodingMode.byte => version <= 9 ? 8 : 16,
  };
}

// ============================================================================
// BitWriter — accumulates bits, flushes to bytes
// ============================================================================

/// A write-only bit buffer.
///
/// Bits are packed MSB-first into bytes. The standard requires all multi-bit
/// values (mode indicator, character count, encoded data) to appear with the
/// most-significant bit first. The [write] method handles this by emitting
/// bits from high to low.
///
/// After encoding the data, [toBytes] pads the last partial byte with zeros
/// and returns the complete byte list.
class _BitWriter {
  final List<int> _bits = []; // each element is 0 or 1

  /// Total number of bits written so far.
  int get bitLength => _bits.length;

  /// Write [count] bits from [value], MSB first.
  ///
  /// Example: `write(0b10110, 5)` appends bits [1, 0, 1, 1, 0].
  void write(int value, int count) {
    for (var i = count - 1; i >= 0; i--) {
      _bits.add((value >> i) & 1);
    }
  }

  /// Pack bits into bytes (MSB-first within each byte).
  ///
  /// If the bit count is not a multiple of 8, the final byte is zero-padded
  /// on the right (low-order bits).
  List<int> toBytes() {
    final result = <int>[];
    for (var i = 0; i < _bits.length; i += 8) {
      var byte = 0;
      for (var j = 0; j < 8; j++) {
        byte = (byte << 1) | (_bits.length > i + j ? _bits[i + j] : 0);
      }
      result.add(byte);
    }
    return result;
  }
}

/// Encode digits in numeric mode.
///
/// Groups of 3 digits → 10 bits (range 0–999)
/// Pairs of 2 digits  → 7 bits  (range 0–99)
/// Single digit       → 4 bits  (range 0–9)
void _encodeNumeric(String input, _BitWriter w) {
  final digits = input.codeUnits.map((c) => c - 0x30).toList();
  var i = 0;
  while (i + 2 < digits.length) {
    w.write(digits[i] * 100 + digits[i + 1] * 10 + digits[i + 2], 10);
    i += 3;
  }
  if (i + 1 < digits.length) {
    w.write(digits[i] * 10 + digits[i + 1], 7);
    i += 2;
  }
  if (i < digits.length) {
    w.write(digits[i], 4);
  }
}

/// Encode characters in alphanumeric mode.
///
/// Pairs of characters are encoded as `(idx1 * 45 + idx2)` in 11 bits.
/// A single trailing character is encoded as `idx` in 6 bits.
void _encodeAlphanumeric(String input, _BitWriter w) {
  final indices = input.runes
      .map((r) => _alphanumChars.indexOf(String.fromCharCode(r)))
      .toList();
  var i = 0;
  while (i + 1 < indices.length) {
    w.write(indices[i] * 45 + indices[i + 1], 11);
    i += 2;
  }
  if (i < indices.length) {
    w.write(indices[i], 6);
  }
}

/// Encode a string in byte mode (UTF-8 bytes, 8 bits each).
void _encodeByte(String input, _BitWriter w) {
  for (final b in input.codeUnits) {
    // For ASCII strings this is just the ASCII value.
    // For multi-byte UTF-8, runes are split across multiple codeUnits.
    // We use encodeUtf8 approach: write each UTF-8 byte.
    w.write(b, 8);
  }
}

/// Build the complete data codeword byte sequence.
///
/// Assembles:
///   [4-bit mode indicator]
///   [character count indicator]
///   [encoded data bits]
///   [terminator: up to 4 zero bits]
///   [byte-boundary padding: 0 bits to reach a byte boundary]
///   [0xEC 0x11 ... fill padding to reach `capacity` bytes]
///
/// The alternating 0xEC/0x11 pattern is specified by ISO/IEC 18004 §7.4.10.
/// It was chosen because it produces a medium-density module pattern that
/// is easy for scanners to handle when the data region is mostly padding.
List<int> _buildDataCodewords(String input, int version, EccLevel ecc) {
  final mode = _selectMode(input);
  final capacity = _numDataCodewords(version, ecc);
  final w = _BitWriter();

  // Mode indicator: 4 bits telling the scanner which decoding mode to use.
  w.write(_modeIndicator(mode), 4);

  // Character count: number of characters (or bytes in byte mode).
  // Use UTF-8 byte length for byte mode, character count otherwise.
  final charCount = mode == _EncodingMode.byte
      ? _utf8Bytes(input).length
      : input.length; // character count for numeric/alphanumeric
  w.write(charCount, _charCountBits(mode, version));

  // Data payload.
  switch (mode) {
    case _EncodingMode.numeric:
      _encodeNumeric(input, w);
    case _EncodingMode.alphanumeric:
      _encodeAlphanumeric(input, w);
    case _EncodingMode.byte:
      for (final b in _utf8Bytes(input)) {
        w.write(b, 8);
      }
  }

  // Terminator: up to 4 zero bits (fewer if capacity is already reached).
  final available = capacity * 8;
  final termLen = (available - w.bitLength).clamp(0, 4);
  if (termLen > 0) w.write(0, termLen);

  // Byte-boundary padding: zero bits to reach the next byte boundary.
  final rem = w.bitLength % 8;
  if (rem != 0) w.write(0, 8 - rem);

  // Fill padding: alternate 0xEC and 0x11 to fill remaining data codewords.
  // 0xEC = 1110 1100 = patterns that avoid solid-color runs.
  // 0x11 = 0001 0001 = complements 0xEC well for the module density target.
  final bytes = w.toBytes();
  final result = List<int>.from(bytes);
  var pad = 0xEC;
  while (result.length < capacity) {
    result.add(pad);
    pad = (pad == 0xEC) ? 0x11 : 0xEC;
  }
  return result;
}

/// Encode [input] as UTF-8 bytes.
///
/// For ASCII text this is just the code units. For non-ASCII, Dart's string
/// codeUnits are already UTF-16, so we need to handle surrogate pairs.
/// For v0.1.0 we use a simple approach: encode each rune via UTF-8.
List<int> _utf8Bytes(String input) {
  // Dart strings are UTF-16. We need UTF-8 bytes for byte-mode QR.
  // Use the built-in string encoding.
  final bytes = <int>[];
  for (final rune in input.runes) {
    if (rune < 0x80) {
      bytes.add(rune);
    } else if (rune < 0x800) {
      bytes.add(0xC0 | (rune >> 6));
      bytes.add(0x80 | (rune & 0x3F));
    } else if (rune < 0x10000) {
      bytes.add(0xE0 | (rune >> 12));
      bytes.add(0x80 | ((rune >> 6) & 0x3F));
      bytes.add(0x80 | (rune & 0x3F));
    } else {
      bytes.add(0xF0 | (rune >> 18));
      bytes.add(0x80 | ((rune >> 12) & 0x3F));
      bytes.add(0x80 | ((rune >> 6) & 0x3F));
      bytes.add(0x80 | (rune & 0x3F));
    }
  }
  return bytes;
}

// ============================================================================
// Block splitting and interleaving
// ============================================================================
//
// For most versions, the data stream is split into multiple blocks, each with
// its own RS ECC computation. This improves damage resilience: a burst error
// that wipes out a contiguous region of the symbol affects only a fraction of
// each block's data.
//
// Block structure: two groups (G1 and G2).
//   G1: `g1Count` blocks, each with `shortLen` data codewords.
//   G2: `g2Count` blocks, each with `shortLen + 1` data codewords.
// All blocks share the same ECC codeword count.
//
// G1 count = totalBlocks - (totalData % totalBlocks)   (number of short blocks)
// G2 count = totalData % totalBlocks                   (number of long blocks)
//
// After computing ECC for each block, the codewords are interleaved:
//   Take codeword 0 from block 0, block 1, block 2, …
//   Take codeword 1 from block 0, block 1, block 2, …
//   … (data codewords) …
//   Take ECC codeword 0 from block 0, block 1, …
//   … (ECC codewords) …

/// One RS-encoded block: raw data codewords + computed ECC codewords.
class _Block {
  final List<int> data;
  final List<int> ecc;
  const _Block({required this.data, required this.ecc});
}

/// Split [data] into blocks and compute RS ECC for each.
List<_Block> _computeBlocks(List<int> data, int version, EccLevel ecc) {
  final e = _eccIdx(ecc);
  final totalBlocks = _numBlocks[e][version];
  final eccLen = _eccCwPerBlock[e][version];
  final totalData = _numDataCodewords(version, ecc);
  final shortLen = totalData ~/ totalBlocks;
  final numLong = totalData % totalBlocks; // number of "long" blocks (shortLen+1)
  final gen = _buildGenerator(eccLen);
  final blocks = <_Block>[];
  var offset = 0;

  // G1 blocks (short): totalBlocks - numLong blocks of `shortLen` bytes.
  final g1Count = totalBlocks - numLong;
  for (var i = 0; i < g1Count; i++) {
    final d = data.sublist(offset, offset + shortLen);
    final eccCw = _rsEncode(d, gen);
    blocks.add(_Block(data: d, ecc: eccCw));
    offset += shortLen;
  }

  // G2 blocks (long): numLong blocks of `shortLen + 1` bytes.
  for (var i = 0; i < numLong; i++) {
    final d = data.sublist(offset, offset + shortLen + 1);
    final eccCw = _rsEncode(d, gen);
    blocks.add(_Block(data: d, ecc: eccCw));
    offset += shortLen + 1;
  }

  return blocks;
}

/// Interleave codewords from multiple blocks.
///
/// ISO 18004 §8.6: interleave data codewords first (round-robin across
/// blocks), then ECC codewords (round-robin across blocks).
///
/// Example with 2 blocks of 3 data + 2 ECC:
///   Block 0 data: [A0, A1, A2]   ECC: [E0_0, E0_1]
///   Block 1 data: [B0, B1, B2]   ECC: [E1_0, E1_1]
///   Interleaved: A0 B0 A1 B1 A2 B2 E0_0 E1_0 E0_1 E1_1
List<int> _interleaveBlocks(List<_Block> blocks) {
  final result = <int>[];
  final maxData = blocks.map((b) => b.data.length).reduce((a, b) => a > b ? a : b);
  final maxEcc = blocks.map((b) => b.ecc.length).reduce((a, b) => a > b ? a : b);

  // Interleave data codewords.
  for (var i = 0; i < maxData; i++) {
    for (final b in blocks) {
      if (i < b.data.length) result.add(b.data[i]);
    }
  }
  // Interleave ECC codewords.
  for (var i = 0; i < maxEcc; i++) {
    for (final b in blocks) {
      if (i < b.ecc.length) result.add(b.ecc[i]);
    }
  }

  return result;
}

// ============================================================================
// Working grid
// ============================================================================
//
// The QR Code is assembled on a mutable working grid. Two parallel boolean
// matrices are maintained:
//
//   modules[row][col]  — the actual module value (dark/light)
//   reserved[row][col] — true for structural modules (finder, timing, format,
//                        etc.) that the data placement algorithm must skip
//
// After the working grid is fully populated, the final ModuleGrid is produced
// from the `modules` array.

/// Mutable grid used during QR Code construction.
///
/// Maintains two matrices of the same size: [modules] for module values
/// and [reserved] for the set of modules that must not be overwritten by data.
class _WorkGrid {
  final int size;
  final List<List<bool>> modules;
  final List<List<bool>> reserved;

  _WorkGrid(this.size)
      : modules = List.generate(size, (_) => List<bool>.filled(size, false)),
        reserved = List.generate(size, (_) => List<bool>.filled(size, false));

  /// Set a module and optionally mark it as reserved.
  void set(int row, int col, {required bool dark, bool reserve = false}) {
    modules[row][col] = dark;
    if (reserve) reserved[row][col] = true;
  }
}

// ============================================================================
// Structural element placement
// ============================================================================

/// Place a 7×7 finder pattern at the top-left corner (top, left).
///
/// Finder pattern layout (1 = dark, 0 = light):
///
/// ```
/// 1 1 1 1 1 1 1
/// 1 0 0 0 0 0 1
/// 1 0 1 1 1 0 1
/// 1 0 1 1 1 0 1
/// 1 0 1 1 1 0 1
/// 1 0 0 0 0 0 1
/// 1 1 1 1 1 1 1
/// ```
///
/// The 1:1:3:1:1 ratio of the dark-light-dark-light-dark sequences in any
/// row or column is uniquely recognizable at any orientation and scale.
void _placeFinder(_WorkGrid g, int top, int left) {
  for (var dr = 0; dr < 7; dr++) {
    for (var dc = 0; dc < 7; dc++) {
      final onBorder = dr == 0 || dr == 6 || dc == 0 || dc == 6;
      final inCore = dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4;
      g.set(top + dr, left + dc, dark: onBorder || inCore, reserve: true);
    }
  }
}

/// Place a 5×5 alignment pattern centered at (row, col).
///
/// Alignment pattern (1 = dark, 0 = light):
///
/// ```
/// 1 1 1 1 1
/// 1 0 0 0 1
/// 1 0 1 0 1
/// 1 0 0 0 1
/// 1 1 1 1 1
/// ```
///
/// The center is always dark. The pattern is a smaller version of the finder
/// pattern, placed at predetermined positions in the data area to help scanners
/// compensate for perspective distortion in large symbols.
void _placeAlignment(_WorkGrid g, int row, int col) {
  for (var dr = -2; dr <= 2; dr++) {
    for (var dc = -2; dc <= 2; dc++) {
      final onBorder = dr.abs() == 2 || dc.abs() == 2;
      final isCenter = dr == 0 && dc == 0;
      g.set(row + dr, col + dc, dark: onBorder || isCenter, reserve: true);
    }
  }
}

/// Place all alignment patterns for [version].
///
/// Alignment patterns are placed at every combination (r, c) of the version's
/// alignment positions, EXCEPT positions that overlap with the finder patterns
/// (detected by checking if the module is already reserved).
void _placeAllAlignments(_WorkGrid g, int version) {
  final positions = _alignmentPositions[version - 1];
  for (final row in positions) {
    for (final col in positions) {
      // Skip positions that overlap with finder patterns (already reserved).
      if (g.reserved[row][col]) continue;
      _placeAlignment(g, row, col);
    }
  }
}

/// Place timing strips (alternating dark/light strips between finders).
///
/// Horizontal timing: row 6, columns 8 to size-9, starting dark at col 8.
/// Vertical timing:   col 6, rows 8 to size-9, starting dark at row 8.
///
/// Timing strips let scanners determine the module grid spacing, especially
/// useful for higher-version symbols where the modules are very small.
void _placeTiming(_WorkGrid g) {
  final sz = g.size;
  for (var c = 8; c <= sz - 9; c++) {
    g.set(6, c, dark: c % 2 == 0, reserve: true);
  }
  for (var r = 8; r <= sz - 9; r++) {
    g.set(r, 6, dark: r % 2 == 0, reserve: true);
  }
}

/// Reserve format information modules (without setting values yet).
///
/// Format information occupies 15 modules in two copies:
///
/// Copy 1 (around top-left finder):
///   Row 8, columns 0–8  (skipping column 6 = timing)
///   Column 8, rows 0–8  (skipping row 6 = timing)
///
/// Copy 2:
///   Row 8, columns n-8 to n-1
///   Column 8, rows n-7 to n-1
///
/// We reserve them early so the data placement algorithm skips them.
void _reserveFormatInfo(_WorkGrid g) {
  final sz = g.size;
  // Copy 1: row 8 (cols 0-8) and col 8 (rows 0-8), skipping timing col/row.
  for (var c = 0; c <= 8; c++) {
    if (c != 6) g.reserved[8][c] = true;
  }
  for (var r = 0; r <= 8; r++) {
    if (r != 6) g.reserved[r][8] = true;
  }
  // Copy 2: bottom-left and top-right regions.
  for (var r = sz - 7; r < sz; r++) {
    g.reserved[r][8] = true;
  }
  for (var c = sz - 8; c < sz; c++) {
    g.reserved[8][c] = true;
  }
}

/// Compute the 15-bit format information word.
///
/// The format word encodes the ECC level (2 bits) and mask pattern (3 bits),
/// protected by a 10-bit BCH code, XORed with 0x5412 to prevent all-zero
/// output.
///
/// Construction:
///   1. 5-bit data = [ecc_indicator (2 bits)][mask (3 bits)]
///   2. Shift left 10: data × x^10
///   3. Polynomial remainder mod G(x), where G = 0x537 (x^10+x^8+x^5+x^4+x^2+x+1)
///   4. Append 10-bit remainder to 5-bit data → 15-bit raw format word
///   5. XOR with 0x5412 (101_0100_0001_0010) to prevent all-zero/all-one
int _computeFormatBits(EccLevel ecc, int mask) {
  final data = (_eccIndicator(ecc) << 3) | mask;
  var rem = data << 10;
  // BCH remainder: divide by G = 0x537.
  for (var i = 14; i >= 10; i--) {
    if ((rem >> i) & 1 == 1) rem ^= 0x537 << (i - 10);
  }
  return ((data << 10) | (rem & 0x3FF)) ^ 0x5412;
}

/// Write format information into the grid.
///
/// See [lessons.md] for the critical bit-ordering details that tripped up
/// multiple implementations:
///
/// Copy 1 (top-left corner region):
///   Row 8, cols 0-5: f14, f13, f12, f11, f10, f9   (MSB-first, left-to-right)
///   Row 8, col 7:    f8   (skipping timing column 6)
///   Row 8, col 8:    f7
///   Col 8, row 7:    f6   (skipping timing row 6)
///   Col 8, rows 0-5: f0, f1, f2, f3, f4, f5   (LSB at row 0, ascending)
///
/// Copy 2 (top-right / bottom-left):
///   Row 8, cols n-1 down to n-8: f0, f1, ..., f7  (LSB at rightmost col)
///   Col 8, rows n-7 to n-1:      f8, f9, ..., f14 (ascending from bottom)
void _writeFormatInfo(_WorkGrid g, int fmt) {
  final sz = g.size;

  // Copy 1 — row 8, MSB-first left-to-right (f14 at col 0 ... f9 at col 5)
  for (var i = 0; i <= 5; i++) {
    g.modules[8][i] = (fmt >> (14 - i)) & 1 == 1;
  }
  g.modules[8][7] = (fmt >> 8) & 1 == 1; // f8 (col 7, skipping timing at col 6)
  g.modules[8][8] = (fmt >> 7) & 1 == 1; // f7
  g.modules[7][8] = (fmt >> 6) & 1 == 1; // f6 (row 7, skipping timing at row 6)
  // Col 8, rows 0-5: LSB-first ascending (f0 at row 0, f5 at row 5)
  for (var i = 0; i <= 5; i++) {
    g.modules[i][8] = (fmt >> i) & 1 == 1;
  }

  // Copy 2 — row 8, LSB-first right-to-left (f0 at col n-1, f7 at col n-8)
  for (var i = 0; i <= 7; i++) {
    g.modules[8][sz - 1 - i] = (fmt >> i) & 1 == 1;
  }
  // Col 8, rows n-7 to n-1: f8 at row n-7, f14 at row n-1 (ascending)
  for (var i = 8; i <= 14; i++) {
    g.modules[sz - 15 + i][8] = (fmt >> i) & 1 == 1;
  }
}

/// Reserve version information modules (for versions 7+).
///
/// Version information occupies two 6×3 blocks:
///   Near top-right finder: rows 0–5, columns n-11 to n-9
///   Near bottom-left finder: rows n-11 to n-9, columns 0–5
void _reserveVersionInfo(_WorkGrid g, int version) {
  if (version < 7) return;
  final sz = g.size;
  for (var r = 0; r < 6; r++) {
    for (var dc = 0; dc < 3; dc++) {
      g.reserved[r][sz - 11 + dc] = true;
    }
  }
  for (var dr = 0; dr < 3; dr++) {
    for (var c = 0; c < 6; c++) {
      g.reserved[sz - 11 + dr][c] = true;
    }
  }
}

/// Compute the 18-bit version information word.
///
/// The 6-bit version number is protected by a 12-bit BCH code.
///
/// Construction:
///   1. 6-bit version number
///   2. Shift left 12: version × x^12
///   3. Polynomial remainder mod G(x), G = 0x1F25
///   4. Append 12-bit remainder → 18-bit version word
int _computeVersionBits(int version) {
  final v = version;
  var rem = v << 12;
  for (var i = 17; i >= 12; i--) {
    if ((rem >> i) & 1 == 1) rem ^= 0x1F25 << (i - 12);
  }
  return (v << 12) | (rem & 0xFFF);
}

/// Write version information for versions 7+.
///
/// The 18-bit version word is written to a 6×3 block near the top-right
/// finder and a 3×6 block (transposed) near the bottom-left finder.
///
/// Bit i of the version word maps to:
///   Top-right block:  row = 5 - (i / 3),  col = n - 9 - (i % 3)
///   Bottom-left block: row = n - 9 - (i % 3),  col = 5 - (i / 3)
///   (symmetric transpose)
void _writeVersionInfo(_WorkGrid g, int version) {
  if (version < 7) return;
  final sz = g.size;
  final bits = _computeVersionBits(version);
  for (var i = 0; i < 18; i++) {
    final dark = (bits >> i) & 1 == 1;
    final a = 5 - (i ~/ 3);
    final b = sz - 9 - (i % 3);
    g.modules[a][b] = dark;
    g.modules[b][a] = dark;
  }
}

/// Place the "dark module" — always-dark, always-reserved.
///
/// ISO 18004 §7.9: There is a dark module at position (4V+9, 8). It is always
/// dark, regardless of mask. It is not part of any data. It exists to satisfy
/// a technical constraint on the synchronization signal.
void _placeDarkModule(_WorkGrid g, int version) {
  g.set(4 * version + 9, 8, dark: true, reserve: true);
}

// ============================================================================
// Data module placement — zigzag scan
// ============================================================================
//
// After all structural modules are placed and reserved, the interleaved message
// stream is placed into the remaining (non-reserved) modules using a two-column
// zigzag scan.
//
// The scan proceeds from the bottom-right corner:
//   - Process two columns at a time (col and col-1).
//   - Within each two-column strip, scan all rows in the current direction
//     (upward on the first pass, then alternating).
//   - Skip timing column 6 (handle as "column 5" when we reach it).
//   - Skip reserved modules.
//   - Place data bits MSB-first from each codeword.
//   - After placing all codewords, fill remaining positions with zeros
//     (remainder bits).

/// Place the interleaved codeword stream into data modules.
void _placeBits(_WorkGrid g, List<int> codewords, int version) {
  final sz = g.size;

  // Expand codewords to a flat list of bits, MSB-first.
  final bits = <bool>[];
  for (final cw in codewords) {
    for (var b = 7; b >= 0; b--) {
      bits.add((cw >> b) & 1 == 1);
    }
  }
  // Append remainder bits (zeros) to pad to the full module count.
  for (var i = 0; i < _numRemainderBits(version); i++) {
    bits.add(false);
  }

  var bitIdx = 0;
  var goingUp = true; // direction: true=upward, false=downward
  var col = sz - 1;  // start from the rightmost column

  while (col >= 1) {
    for (var vi = 0; vi < sz; vi++) {
      // Row index depends on direction.
      final row = goingUp ? sz - 1 - vi : vi;

      // Process two columns: current and current-1.
      for (var dc = 0; dc <= 1; dc++) {
        final c = col - dc;
        if (c == 6) continue; // skip timing column
        if (g.reserved[row][c]) continue; // skip structural modules

        g.modules[row][c] = bitIdx < bits.length && bits[bitIdx];
        bitIdx++;
      }
    }

    goingUp = !goingUp;
    col -= 2;
    if (col == 6) col = 5; // jump over timing column
  }
}

// ============================================================================
// Mask patterns and penalty scoring
// ============================================================================
//
// After placing data bits, a mask is applied to flip non-reserved modules
// where a condition holds. The goal is to break up patterns that could confuse
// a scanner: solid-colored regions, long uniform runs, finder-pattern look-alikes.
//
// Eight mask patterns are defined (ISO 18004 Table 10). All eight are tried.
// The one with the lowest penalty score is used. Penalty rules are designed to
// discourage exactly the patterns that trouble real scanners.

/// Whether mask [m] applies to the module at (row, col).
///
/// If this returns true, the module's value is flipped.
bool _maskCondition(int m, int row, int col) {
  return switch (m) {
    0 => (row + col) % 2 == 0,
    1 => row % 2 == 0,
    2 => col % 3 == 0,
    3 => (row + col) % 3 == 0,
    4 => (row ~/ 2 + col ~/ 3) % 2 == 0,
    5 => (row * col) % 2 + (row * col) % 3 == 0,
    6 => ((row * col) % 2 + (row * col) % 3) % 2 == 0,
    7 => ((row + col) % 2 + (row * col) % 3) % 2 == 0,
    _ => false,
  };
}

/// Apply mask [m] to data/ECC modules (non-reserved).
///
/// Returns a new modules matrix — the original is not modified.
List<List<bool>> _applyMask(
  List<List<bool>> modules,
  List<List<bool>> reserved,
  int sz,
  int m,
) {
  // Deep copy of modules.
  final result = List.generate(sz, (r) => List<bool>.from(modules[r]));
  for (var r = 0; r < sz; r++) {
    for (var c = 0; c < sz; c++) {
      if (!reserved[r][c] && _maskCondition(m, r, c)) {
        result[r][c] = !result[r][c];
      }
    }
  }
  return result;
}

/// Compute penalty score for a masked grid.
///
/// Four rules contribute to the penalty:
///
/// Rule 1 — Runs of same color ≥ 5:
///   Each run of length L in a row or column adds (L - 2) to the penalty.
///   Run of 5 → +3, run of 6 → +4, run of 7 → +5, etc.
///
/// Rule 2 — 2×2 same-color blocks:
///   Each non-overlapping (in practice, counted at every position) 2×2 square
///   where all four modules are the same color adds +3.
///
/// Rule 3 — Finder-pattern-like sequences:
///   Checks for 1-0-1-1-1-0-1-0-0-0-0 or its reverse in rows and columns.
///   Each occurrence adds +40.
///
/// Rule 4 — Dark module proportion:
///   Counts the percentage of dark modules. The further from 50%, the higher
///   the penalty. Calculated as (deviation from 50% in steps of 5%) × 10.
int _computePenalty(List<List<bool>> modules, int sz) {
  var penalty = 0;

  // Rule 1: runs of same color ≥ 5.
  for (var a = 0; a < sz; a++) {
    for (final horiz in [true, false]) {
      var run = 1;
      var prev = horiz ? modules[a][0] : modules[0][a];
      for (var i = 1; i < sz; i++) {
        final cur = horiz ? modules[a][i] : modules[i][a];
        if (cur == prev) {
          run++;
        } else {
          if (run >= 5) penalty += run - 2;
          run = 1;
          prev = cur;
        }
      }
      if (run >= 5) penalty += run - 2;
    }
  }

  // Rule 2: 2×2 same-color blocks.
  for (var r = 0; r < sz - 1; r++) {
    for (var c = 0; c < sz - 1; c++) {
      final d = modules[r][c];
      if (d == modules[r][c + 1] &&
          d == modules[r + 1][c] &&
          d == modules[r + 1][c + 1]) {
        penalty += 3;
      }
    }
  }

  // Rule 3: finder-pattern-like sequences.
  // Pattern A: 1 0 1 1 1 0 1 0 0 0 0 (looks like a finder row with quiet zone).
  // Pattern B: 0 0 0 0 1 0 1 1 1 0 1 (reversed pattern A).
  const p1 = [1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0];
  const p2 = [0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1];
  for (var a = 0; a < sz; a++) {
    for (var b = 0; b <= sz - 11; b++) {
      var mh1 = true, mh2 = true, mv1 = true, mv2 = true;
      for (var k = 0; k < 11; k++) {
        final bh = modules[a][b + k] ? 1 : 0;
        final bv = modules[b + k][a] ? 1 : 0;
        if (bh != p1[k]) mh1 = false;
        if (bh != p2[k]) mh2 = false;
        if (bv != p1[k]) mv1 = false;
        if (bv != p2[k]) mv2 = false;
      }
      if (mh1) penalty += 40;
      if (mh2) penalty += 40;
      if (mv1) penalty += 40;
      if (mv2) penalty += 40;
    }
  }

  // Rule 4: dark module proportion deviation.
  var dark = 0;
  for (var r = 0; r < sz; r++) {
    for (var c = 0; c < sz; c++) {
      if (modules[r][c]) dark++;
    }
  }
  final total = sz * sz;
  final ratio = (dark * 100.0) / total;
  final prev5 = (ratio / 5).floor() * 5;
  final a = (prev5 - 50).abs();
  final b = (prev5 + 5 - 50).abs();
  penalty += (a < b ? a : b) ~/ 5 * 10;

  return penalty;
}

// ============================================================================
// Version selection
// ============================================================================

/// Estimate the number of bits needed to encode [input] in [mode] at [version].
///
/// This is used for version selection before building the full bit stream.
int _bitsNeeded(String input, _EncodingMode mode, int version) {
  final ccBits = _charCountBits(mode, version);
  final dataBits = switch (mode) {
    _EncodingMode.byte => _utf8Bytes(input).length * 8,
    _EncodingMode.numeric => () {
      final n = input.length;
      return (n ~/ 3) * 10 + (n % 3 == 2 ? 7 : (n % 3 == 1 ? 4 : 0));
    }(),
    _EncodingMode.alphanumeric => () {
      final n = input.length;
      return (n ~/ 2) * 11 + (n % 2 == 1 ? 6 : 0);
    }(),
  };
  return 4 + ccBits + dataBits; // mode indicator + count + data
}

/// Select the smallest version (1–40) that fits [input] at [ecc].
///
/// Throws [InputTooLongError] if no version can accommodate the input.
int _selectVersion(String input, EccLevel ecc) {
  final mode = _selectMode(input);
  for (var v = 1; v <= 40; v++) {
    final capacity = _numDataCodewords(v, ecc);
    final bitsRequired = _bitsNeeded(input, mode, v);
    final cwRequired = (bitsRequired + 7) ~/ 8;
    if (cwRequired <= capacity) return v;
  }
  throw InputTooLongError(
    'Input (${_utf8Bytes(input).length} bytes, ECC=$ecc) exceeds version-40 capacity.',
  );
}

// ============================================================================
// Grid builder — full QR Code assembly
// ============================================================================

/// Build an initialized [_WorkGrid] with all structural elements placed.
///
/// Structural elements are placed in this order:
///   1. Finder patterns (three 7×7 squares at corners)
///   2. Separators (1-module border around each finder, always light)
///   3. Timing patterns (alternating strips between finders)
///   4. Alignment patterns (version-specific 5×5 squares)
///   5. Format information reservation (without actual values yet)
///   6. Version information reservation (for v7+)
///   7. Dark module (single always-dark module at (4V+9, 8))
_WorkGrid _buildGrid(int version) {
  final sz = _symbolSize(version);
  final g = _WorkGrid(sz);

  // ── Finder patterns ────────────────────────────────────────────────────────
  // Top-left finder at (0, 0).
  _placeFinder(g, 0, 0);
  // Top-right finder at (0, sz-7).
  _placeFinder(g, 0, sz - 7);
  // Bottom-left finder at (sz-7, 0).
  _placeFinder(g, sz - 7, 0);

  // ── Separators ─────────────────────────────────────────────────────────────
  // Each finder is surrounded by a 1-module-wide light border (separator) that
  // isolates it from the data area. The separator is always light (false).
  //
  // Top-left separator:
  for (var i = 0; i <= 7; i++) {
    g.set(7, i, dark: false, reserve: true);        // bottom edge
    g.set(i, 7, dark: false, reserve: true);        // right edge
  }
  // Top-right separator:
  for (var i = 0; i <= 7; i++) {
    g.set(7, sz - 1 - i, dark: false, reserve: true); // bottom edge
    g.set(i, sz - 8, dark: false, reserve: true);     // left edge
  }
  // Bottom-left separator:
  for (var i = 0; i <= 7; i++) {
    g.set(sz - 8, i, dark: false, reserve: true);    // top edge
    g.set(sz - 1 - i, 7, dark: false, reserve: true); // right edge
  }

  // ── Timing patterns ────────────────────────────────────────────────────────
  _placeTiming(g);

  // ── Alignment patterns ────────────────────────────────────────────────────
  _placeAllAlignments(g, version);

  // ── Format information reservation ────────────────────────────────────────
  _reserveFormatInfo(g);

  // ── Version information reservation ───────────────────────────────────────
  _reserveVersionInfo(g, version);

  // ── Dark module ───────────────────────────────────────────────────────────
  _placeDarkModule(g, version);

  return g;
}

// ============================================================================
// Public API
// ============================================================================

/// Encode a UTF-8 string into a QR Code [ModuleGrid].
///
/// Selects the minimum version (1–40) that fits [input] at [ecc]. Returns a
/// `(4V+17) × (4V+17)` boolean grid where `true` = dark module.
///
/// ## Example
///
/// ```dart
/// import 'package:coding_adventures_qr_code/coding_adventures_qr_code.dart';
///
/// final grid = encode('HELLO WORLD', EccLevel.m);
/// print(grid.rows); // 21 (version 1, 21×21)
/// ```
///
/// ## Throws
///
/// [InputTooLongError] if [input] exceeds version-40 capacity.
///
/// ## Capacity limits
///
/// | Mode         | ECC L  | ECC H |
/// |--------------|--------|-------|
/// | Numeric      | 7089   | 3057  |
/// | Alphanumeric | 4296   | 1852  |
/// | Byte (UTF-8) | 2953   | 1273  |
ModuleGrid encode(String input, EccLevel ecc) {
  // Guard against obviously oversized inputs before doing any real work.
  // The maximum numeric capacity at ECC L (v40) is 7089 characters.
  if (input.length > 7089) {
    throw InputTooLongError(
      'Input length ${input.length} exceeds 7089 (QR v40 numeric maximum).',
    );
  }

  final version = _selectVersion(input, ecc);
  final sz = _symbolSize(version);

  // Step 1: Build data codewords (mode + char count + data + padding).
  final dataCw = _buildDataCodewords(input, version, ecc);

  // Step 2: Split into blocks and compute RS ECC for each block.
  final blocks = _computeBlocks(dataCw, version, ecc);

  // Step 3: Interleave codewords from all blocks.
  final interleaved = _interleaveBlocks(blocks);

  // Step 4: Initialize the grid with structural elements.
  final grid = _buildGrid(version);

  // Step 5: Place the interleaved bit stream into data modules (zigzag scan).
  _placeBits(grid, interleaved, version);

  // Step 6: Evaluate all 8 mask patterns; pick the one with lowest penalty.
  var bestMask = 0;
  var bestPenalty = 0x7FFFFFFF; // large initial value

  for (var m = 0; m < 8; m++) {
    final masked = _applyMask(grid.modules, grid.reserved, sz, m);
    final fmt = _computeFormatBits(ecc, m);

    // Build a temporary grid with this mask applied + format info written.
    final tempG = _WorkGrid(sz);
    for (var r = 0; r < sz; r++) {
      for (var c = 0; c < sz; c++) {
        tempG.modules[r][c] = masked[r][c];
        tempG.reserved[r][c] = grid.reserved[r][c];
      }
    }
    _writeFormatInfo(tempG, fmt);

    final p = _computePenalty(tempG.modules, sz);
    if (p < bestPenalty) {
      bestPenalty = p;
      bestMask = m;
    }
  }

  // Step 7: Finalize with the best mask.
  final finalMods = _applyMask(grid.modules, grid.reserved, sz, bestMask);

  // Build the finalized working grid.
  final finalGrid = _WorkGrid(sz);
  for (var r = 0; r < sz; r++) {
    for (var c = 0; c < sz; c++) {
      finalGrid.modules[r][c] = finalMods[r][c];
      finalGrid.reserved[r][c] = grid.reserved[r][c];
    }
  }

  // Write final format information.
  _writeFormatInfo(finalGrid, _computeFormatBits(ecc, bestMask));

  // Write version information (for v7+).
  _writeVersionInfo(finalGrid, version);

  // Convert to an immutable ModuleGrid.
  return ModuleGrid(
    rows: sz,
    cols: sz,
    modules: List.generate(
      sz,
      (r) => List<bool>.unmodifiable(finalGrid.modules[r]),
      growable: false,
    ),
    moduleShape: ModuleShape.square,
  );
}

/// Encode and convert directly to a pixel-resolved [PaintScene].
///
/// Convenience wrapper: [encode] then `barcode-2d`'s `layout()`.
///
/// ## Throws
///
/// [InputTooLongError] if input is too long.
/// [QRLayoutError] if the layout configuration is invalid.
PaintScene encodeAndLayout(
  String input,
  EccLevel ecc, {
  Barcode2DLayoutConfig? config,
}) {
  final grid = encode(input, ecc);
  try {
    return layout(grid, config: config);
  } catch (e) {
    throw QRLayoutError(
      'barcode-2d layout failed: $e',
    );
  }
}
