/// Micro QR Code encoder — ISO/IEC 18004:2015 Annex E compliant.
///
/// ## What is Micro QR?
///
/// Micro QR Code is the compact sibling of regular QR Code. It was designed
/// for applications where even the smallest standard QR (21×21 at version 1)
/// is too large. Common uses include surface-mount component labels, circuit
/// board markings, and miniature industrial tags.
///
/// ```
/// M1: 11×11 modules   M2: 13×13 modules
/// M3: 15×15 modules   M4: 17×17 modules
/// formula: size = 2 × version_number + 9
/// ```
///
/// ## Key differences from regular QR Code
///
/// - **Single finder pattern** at top-left only (one 7×7 square, not three).
/// - **Timing at row 0 / col 0** (not row 6 / col 6).
/// - **Only 4 mask patterns** (not 8).
/// - **Format XOR mask 0x4445** (not 0x5412).
/// - **Single copy of format info** (not two).
/// - **2-module quiet zone** (not 4).
/// - **Narrower mode indicators** (0–3 bits instead of 4).
/// - **Single block** (no interleaving).
///
/// ## Encoding pipeline
///
/// ```
/// input string
///   → auto-select smallest symbol (M1..M4) and mode
///   → build bit stream (mode indicator + char count + data + terminator + padding)
///   → Reed-Solomon ECC (GF(256)/0x11D, b=0, single block)
///   → initialize grid (finder, L-shaped separator, timing at row0/col0, format reserved)
///   → zigzag data placement (two-column snake from bottom-right)
///   → evaluate 4 mask patterns, pick lowest penalty
///   → write format information (15 bits, single copy, XOR 0x4445)
///   → ModuleGrid
/// ```
library micro_qr;

import 'package:coding_adventures_barcode_2d/coding_adventures_barcode_2d.dart';
import 'package:coding_adventures_gf256/coding_adventures_gf256.dart';

// ============================================================================
// Package version
// ============================================================================

/// Package version following Semantic Versioning 2.0.
const String microQrVersion = '0.1.0';

// ============================================================================
// Public types
// ============================================================================

/// Micro QR symbol designator.
///
/// Each step up adds two rows/columns (size = 2×version_number+9):
///   M1=11×11, M2=13×13, M3=15×15, M4=17×17.
enum MicroQRVersion {
  /// 11×11 modules — numeric only, detection-error-only ECC.
  m1,

  /// 13×13 modules — numeric and alphanumeric and byte, L or M ECC.
  m2,

  /// 15×15 modules — numeric, alphanumeric, byte; L or M ECC.
  m3,

  /// 17×17 modules — all modes; L, M, or Q ECC.
  m4,
}

/// Error correction level for Micro QR.
///
/// | Level     | Available in | Recovery |
/// |-----------|-------------|---------|
/// | detection | M1 only     | detects errors only (no correction) |
/// | l         | M2, M3, M4  | ~7% of codewords |
/// | m         | M2, M3, M4  | ~15% of codewords |
/// | q         | M4 only     | ~25% of codewords |
///
/// Level H is not available in any Micro QR symbol — the symbols are too small
/// to spare 30% redundancy.
enum MicroQREccLevel {
  /// Error detection only (M1 exclusive). Cannot correct; only detect.
  detection,

  /// Low error correction (~7% recovery). Available in M2, M3, M4.
  l,

  /// Medium error correction (~15% recovery). Available in M2, M3, M4.
  m,

  /// Quartile error correction (~25% recovery). Available in M4 only.
  q,
}

/// Base class for Micro QR encoder errors.
///
/// All encoder failures throw a subclass of [MicroQRError]. Catch this base
/// type to handle any encoder failure without caring about the specific cause.
abstract class MicroQRError implements Exception {
  /// Human-readable description of the error.
  final String message;

  const MicroQRError(this.message);

  @override
  String toString() => '$runtimeType: $message';
}

/// Thrown when the input string is too long for any M1–M4 symbol.
///
/// The maximum capacity is 35 numeric characters in M4-L. If the input exceeds
/// this, consider using regular QR Code which supports up to 7,089 numeric chars.
class InputTooLong extends MicroQRError {
  const InputTooLong(super.message);
}

/// Thrown when the requested encoding mode is not available for the chosen symbol.
///
/// For example, byte mode requires at least M3; requesting byte mode with M1
/// raises this error.
class UnsupportedMode extends MicroQRError {
  const UnsupportedMode(super.message);
}

/// Thrown when the requested ECC level is not available for the chosen symbol.
///
/// For example, M1 only supports [MicroQREccLevel.detection]; requesting L, M,
/// or Q raises this error. M4-Q is the only way to get Q-level correction.
class ECCNotAvailable extends MicroQRError {
  const ECCNotAvailable(super.message);
}

/// Thrown when a character cannot be encoded in the selected mode.
///
/// For example, lowercase letters cannot be encoded in alphanumeric mode —
/// the 45-char alphanumeric set only includes uppercase A–Z, digits, and
/// a small set of symbols.
class InvalidCharacter extends MicroQRError {
  const InvalidCharacter(super.message);
}

// ============================================================================
// Internal encoding mode
// ============================================================================

/// Internal representation of the encoding mode chosen for a symbol.
///
/// Selection priority: numeric > alphanumeric > byte (most to least compact).
/// Kanji (M4 only) is a future extension — not implemented here.
enum _EncodingMode {
  numeric,
  alphanumeric,
  byte,
}

// ============================================================================
// Symbol configurations
// ============================================================================

/// All the compile-time constants for one (version, ECC) combination.
///
/// There are exactly 8 valid combinations:
///   M1/detection, M2/L, M2/M, M3/L, M3/M, M4/L, M4/M, M4/Q.
///
/// These come directly from ISO/IEC 18004:2015 Annex E, Table E.1.
class _SymbolConfig {
  final MicroQRVersion version;
  final MicroQREccLevel ecc;

  /// 3-bit symbol indicator placed in format information (0..7).
  final int symbolIndicator;

  /// Symbol side length in modules (11, 13, 15, or 17).
  final int size;

  /// Number of data codewords (full 8-bit bytes, except M1 which uses 2.5 bytes).
  ///
  /// M1 is treated as having 3 data bytes where the last byte has data in its
  /// upper 4 bits and zeros in the lower 4 bits.
  final int dataCw;

  /// Number of ECC codewords appended after the data bytes.
  final int eccCw;

  /// Maximum numeric characters. 0 = numeric not supported for this symbol.
  final int numericCap;

  /// Maximum alphanumeric characters. 0 = alphanumeric not supported.
  final int alphaCap;

  /// Maximum byte characters. 0 = byte mode not supported.
  final int byteCap;

  /// Terminator bit count (3 for M1, 5 for M2, 7 for M3, 9 for M4).
  ///
  /// Longer terminators ensure the final codeword aligns to a byte boundary
  /// even when the bit stream is nearly full.
  final int terminatorBits;

  /// Mode indicator bit width (0=M1, 1=M2, 2=M3, 3=M4).
  ///
  /// M1 has no mode indicator because it only supports numeric mode.
  final int modeIndicatorBits;

  /// Character count field width for numeric mode.
  final int ccBitsNumeric;

  /// Character count field width for alphanumeric mode (0 = not supported).
  final int ccBitsAlpha;

  /// Character count field width for byte mode (0 = not supported).
  final int ccBitsByte;

  /// True for M1 only: last data "codeword" is 4 bits → total = 20 data bits.
  ///
  /// M1's data capacity is 3 × 8 − 4 = 20 bits. The RS encoder receives 3 full
  /// bytes where byte[2]'s lower nibble is forced to zero.
  final bool m1HalfCw;

  const _SymbolConfig({
    required this.version,
    required this.ecc,
    required this.symbolIndicator,
    required this.size,
    required this.dataCw,
    required this.eccCw,
    required this.numericCap,
    required this.alphaCap,
    required this.byteCap,
    required this.terminatorBits,
    required this.modeIndicatorBits,
    required this.ccBitsNumeric,
    required this.ccBitsAlpha,
    required this.ccBitsByte,
    required this.m1HalfCw,
  });
}

/// All 8 valid Micro QR symbol configurations from ISO 18004:2015 Annex E.
///
/// Ordered from smallest to largest symbol, matching the auto-selection
/// priority: first fitting config wins.
const List<_SymbolConfig> _symbolConfigs = [
  // ── M1 / Detection ──────────────────────────────────────────────────────
  //
  // The tiniest symbol: 11×11 modules. Only numeric mode. The "ECC" here is
  // actually just error detection (2 check bytes), not correction.
  // Unique quirk: only 20 data bits (not 24) — the last "codeword" is 4 bits.
  _SymbolConfig(
    version: MicroQRVersion.m1,
    ecc: MicroQREccLevel.detection,
    symbolIndicator: 0,
    size: 11,
    dataCw: 3,
    eccCw: 2,
    numericCap: 5,
    alphaCap: 0,
    byteCap: 0,
    terminatorBits: 3,
    modeIndicatorBits: 0,
    ccBitsNumeric: 3,
    ccBitsAlpha: 0,
    ccBitsByte: 0,
    m1HalfCw: true,
  ),
  // ── M2 / L ──────────────────────────────────────────────────────────────
  _SymbolConfig(
    version: MicroQRVersion.m2,
    ecc: MicroQREccLevel.l,
    symbolIndicator: 1,
    size: 13,
    dataCw: 5,
    eccCw: 5,
    numericCap: 10,
    alphaCap: 6,
    byteCap: 4,
    terminatorBits: 5,
    modeIndicatorBits: 1,
    ccBitsNumeric: 4,
    ccBitsAlpha: 3,
    ccBitsByte: 4,
    m1HalfCw: false,
  ),
  // ── M2 / M ──────────────────────────────────────────────────────────────
  _SymbolConfig(
    version: MicroQRVersion.m2,
    ecc: MicroQREccLevel.m,
    symbolIndicator: 2,
    size: 13,
    dataCw: 4,
    eccCw: 6,
    numericCap: 8,
    alphaCap: 5,
    byteCap: 3,
    terminatorBits: 5,
    modeIndicatorBits: 1,
    ccBitsNumeric: 4,
    ccBitsAlpha: 3,
    ccBitsByte: 4,
    m1HalfCw: false,
  ),
  // ── M3 / L ──────────────────────────────────────────────────────────────
  _SymbolConfig(
    version: MicroQRVersion.m3,
    ecc: MicroQREccLevel.l,
    symbolIndicator: 3,
    size: 15,
    dataCw: 11,
    eccCw: 6,
    numericCap: 23,
    alphaCap: 14,
    byteCap: 9,
    terminatorBits: 7,
    modeIndicatorBits: 2,
    ccBitsNumeric: 5,
    ccBitsAlpha: 4,
    ccBitsByte: 4,
    m1HalfCw: false,
  ),
  // ── M3 / M ──────────────────────────────────────────────────────────────
  _SymbolConfig(
    version: MicroQRVersion.m3,
    ecc: MicroQREccLevel.m,
    symbolIndicator: 4,
    size: 15,
    dataCw: 9,
    eccCw: 8,
    numericCap: 18,
    alphaCap: 11,
    byteCap: 7,
    terminatorBits: 7,
    modeIndicatorBits: 2,
    ccBitsNumeric: 5,
    ccBitsAlpha: 4,
    ccBitsByte: 4,
    m1HalfCw: false,
  ),
  // ── M4 / L ──────────────────────────────────────────────────────────────
  _SymbolConfig(
    version: MicroQRVersion.m4,
    ecc: MicroQREccLevel.l,
    symbolIndicator: 5,
    size: 17,
    dataCw: 16,
    eccCw: 8,
    numericCap: 35,
    alphaCap: 21,
    byteCap: 15,
    terminatorBits: 9,
    modeIndicatorBits: 3,
    ccBitsNumeric: 6,
    ccBitsAlpha: 5,
    ccBitsByte: 5,
    m1HalfCw: false,
  ),
  // ── M4 / M ──────────────────────────────────────────────────────────────
  _SymbolConfig(
    version: MicroQRVersion.m4,
    ecc: MicroQREccLevel.m,
    symbolIndicator: 6,
    size: 17,
    dataCw: 14,
    eccCw: 10,
    numericCap: 30,
    alphaCap: 18,
    byteCap: 13,
    terminatorBits: 9,
    modeIndicatorBits: 3,
    ccBitsNumeric: 6,
    ccBitsAlpha: 5,
    ccBitsByte: 5,
    m1HalfCw: false,
  ),
  // ── M4 / Q ──────────────────────────────────────────────────────────────
  _SymbolConfig(
    version: MicroQRVersion.m4,
    ecc: MicroQREccLevel.q,
    symbolIndicator: 7,
    size: 17,
    dataCw: 10,
    eccCw: 14,
    numericCap: 21,
    alphaCap: 13,
    byteCap: 9,
    terminatorBits: 9,
    modeIndicatorBits: 3,
    ccBitsNumeric: 6,
    ccBitsAlpha: 5,
    ccBitsByte: 5,
    m1HalfCw: false,
  ),
];

// ============================================================================
// RS generator polynomials (compile-time constants)
// ============================================================================

/// Return the monic RS generator polynomial for [eccCount] ECC codewords.
///
/// g(x) = (x+α⁰)(x+α¹)···(x+α^{n-1}) over GF(256)/0x11D with b=0 convention.
///
/// The array length is n+1 (leading monic term 0x01 included). Only the ECC
/// codeword counts used by Micro QR are supported: {2, 5, 6, 8, 10, 14}.
///
/// These are compile-time constants derived from the standard; computing them
/// at runtime would be error-prone and wasteful.
List<int> _getGenerator(int eccCount) {
  switch (eccCount) {
    case 2:
      return [0x01, 0x03, 0x02];
    case 5:
      return [0x01, 0x1f, 0xf6, 0x44, 0xd9, 0x68];
    case 6:
      return [0x01, 0x3f, 0x4e, 0x17, 0x9b, 0x05, 0x37];
    case 8:
      return [0x01, 0x63, 0x0d, 0x60, 0x6d, 0x5b, 0x10, 0xa2, 0xa3];
    case 10:
      return [0x01, 0xf6, 0x75, 0xa8, 0xd0, 0xc3, 0xe3, 0x36, 0xe1, 0x3c, 0x45];
    case 14:
      return [
        0x01, 0xf6, 0x9a, 0x60, 0x97, 0x8a, 0xf1, 0xa4,
        0xa1, 0x8e, 0xfc, 0x7a, 0x52, 0xad, 0xac,
      ];
    default:
      throw ArgumentError('No RS generator for eccCount=$eccCount');
  }
}

// ============================================================================
// Pre-computed format information table
// ============================================================================

/// All 32 pre-computed format words (after XOR with 0x4445).
///
/// Indexed as `_formatTable[symbolIndicator][maskPattern]`.
///
/// The 15-bit format word structure:
///   [symbol_indicator (3b)] [mask_pattern (2b)] [BCH-10 remainder]
/// XOR-masked with 0x4445 (Micro QR specific, not 0x5412 like regular QR).
///
/// | Row | Symbol+ECC    |
/// |-----|--------------|
/// | 0   | M1/detection |
/// | 1   | M2-L         |
/// | 2   | M2-M         |
/// | 3   | M3-L         |
/// | 4   | M3-M         |
/// | 5   | M4-L         |
/// | 6   | M4-M         |
/// | 7   | M4-Q         |
const List<List<int>> _formatTable = [
  [0x4445, 0x4172, 0x4E2B, 0x4B1C], // M1/detection
  [0x5528, 0x501F, 0x5F46, 0x5A71], // M2-L
  [0x6649, 0x637E, 0x6C27, 0x6910], // M2-M
  [0x7764, 0x7253, 0x7D0A, 0x783D], // M3-L
  [0x06DE, 0x03E9, 0x0CB0, 0x0987], // M3-M
  [0x17F3, 0x12C4, 0x1D9D, 0x18AA], // M4-L
  [0x24B2, 0x2185, 0x2EDC, 0x2BEB], // M4-M
  [0x359F, 0x30A8, 0x3FF1, 0x3AC6], // M4-Q
];

// ============================================================================
// 45-character alphanumeric set
// ============================================================================

/// The 45-character alphanumeric set shared with regular QR Code.
///
/// Characters are assigned indices 0–44. Pairs are packed as:
///   `value = first_index × 45 + second_index`
/// into 11 bits. Trailing single characters use 6 bits.
const String _alphanumChars = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ \$%*+-./:';

// ============================================================================
// Mode selection
// ============================================================================

/// Select the most compact encoding mode supported by [cfg] for [input].
///
/// Priority: numeric > alphanumeric > byte. Kanji is not implemented.
///
/// Throws [UnsupportedMode] if no supported mode can encode the input in
/// the chosen symbol.
_EncodingMode _selectMode(String input, _SymbolConfig cfg) {
  // Numeric: all characters must be ASCII digits 0–9.
  // M1 only supports numeric, so if the symbol is M1 and input isn't all digits,
  // we fall through to an error (no other modes available).
  final isNumeric = input.isEmpty || input.runes.every((c) => c >= 0x30 && c <= 0x39);
  if (isNumeric && cfg.ccBitsNumeric > 0) {
    return _EncodingMode.numeric;
  }

  // Alphanumeric: all characters must be in the 45-char set.
  final isAlpha = input.runes.every((c) => _alphanumChars.contains(String.fromCharCode(c)));
  if (isAlpha && cfg.alphaCap > 0) {
    return _EncodingMode.alphanumeric;
  }

  // Byte: any byte-representable content. We encode as UTF-8 bytes.
  if (cfg.byteCap > 0) {
    return _EncodingMode.byte;
  }

  throw UnsupportedMode(
    'Input cannot be encoded in any mode supported by '
    '${cfg.version.name}-${cfg.ecc.name}',
  );
}

/// Return the mode indicator value for [mode] in a symbol with [cfg].
///
/// M1 has no indicator (0 bits → value is irrelevant).
/// M2: 1 bit → 0=numeric, 1=alphanumeric.
/// M3: 2 bits → 00=numeric, 01=alpha, 10=byte.
/// M4: 3 bits → 000=numeric, 001=alpha, 010=byte.
int _modeIndicatorValue(_EncodingMode mode, _SymbolConfig cfg) {
  switch (cfg.modeIndicatorBits) {
    case 0:
      return 0; // M1: implicit numeric, no bits written
    case 1:
      return mode == _EncodingMode.numeric ? 0 : 1;
    case 2:
      return switch (mode) {
        _EncodingMode.numeric => 0x00,
        _EncodingMode.alphanumeric => 0x01,
        _EncodingMode.byte => 0x02,
      };
    case 3:
      return switch (mode) {
        _EncodingMode.numeric => 0x000,
        _EncodingMode.alphanumeric => 0x001,
        _EncodingMode.byte => 0x010,
      };
    default:
      return 0;
  }
}

/// Return the character count field width for [mode] in [cfg].
int _charCountBits(_EncodingMode mode, _SymbolConfig cfg) {
  return switch (mode) {
    _EncodingMode.numeric => cfg.ccBitsNumeric,
    _EncodingMode.alphanumeric => cfg.ccBitsAlpha,
    _EncodingMode.byte => cfg.ccBitsByte,
  };
}

// ============================================================================
// Bit-writer
// ============================================================================

/// Accumulates bits MSB-first and converts to bytes.
///
/// Each [write] call appends [count] least-significant bits of [value] to the
/// stream, MSB first. This matches the QR/Micro-QR convention of big-endian
/// bit ordering within each codeword.
///
/// Think of this as a "bit FIFO" or "bit shift-register" growing to the right.
/// Writing `value=0b101, count=3` appends bits [1, 0, 1] to the stream.
class _BitWriter {
  final List<int> _bits = []; // each element is 0 or 1

  /// Append the [count] LSBs of [value], MSB first.
  ///
  /// ```
  /// writer.write(0b101, 3);  // appends 1, 0, 1
  /// ```
  void write(int value, int count) {
    for (var i = count - 1; i >= 0; i--) {
      _bits.add((value >> i) & 1);
    }
  }

  /// Current number of bits in the stream.
  int get bitLen => _bits.length;

  /// Return the raw bit list (each element is 0 or 1).
  List<int> toBitList() => List.unmodifiable(_bits);

  /// Pack bits into bytes (MSB-first, padding last byte with zero bits if needed).
  List<int> toBytes() {
    final result = <int>[];
    var i = 0;
    while (i < _bits.length) {
      var byte = 0;
      for (var j = 0; j < 8; j++) {
        byte = (byte << 1) | (_bits.length > i + j ? _bits[i + j] : 0);
      }
      result.add(byte);
      i += 8;
    }
    return result;
  }
}

// ============================================================================
// Data encoding helpers
// ============================================================================

/// Encode a numeric string: groups of 3 → 10 bits, pair → 7 bits, single → 4 bits.
///
/// Numeric mode packs decimal digits as decimal values (not BCD). Three digits
/// form a value 000–999, encoded in 10 bits. Remaining two form 00–99 in 7 bits.
/// A final single digit uses 4 bits.
///
/// Example: "12345" → [123 in 10 bits] + [45 in 7 bits] = 17 bits total.
void _encodeNumeric(String input, _BitWriter w) {
  final digits = input.codeUnits.map((c) => c - 0x30).toList();
  var i = 0;

  // Groups of three → 10 bits (decimal 0–999).
  while (i + 2 < digits.length) {
    w.write(digits[i] * 100 + digits[i + 1] * 10 + digits[i + 2], 10);
    i += 3;
  }

  // Remaining pair → 7 bits (decimal 0–99).
  if (i + 1 < digits.length) {
    w.write(digits[i] * 10 + digits[i + 1], 7);
    i += 2;
  }

  // Remaining single digit → 4 bits (decimal 0–9).
  if (i < digits.length) {
    w.write(digits[i], 4);
  }
}

/// Encode an alphanumeric string: pairs → 11 bits, trailing single → 6 bits.
///
/// Each character maps to an index 0–44 in the 45-char alphanumeric set.
/// Two adjacent characters are encoded as `index0 × 45 + index1`, fitting
/// in 11 bits (max value 44×45+44 = 2024 < 2048 = 2^11).
///
/// Example: "AC" = (10)(12) → 10×45+12 = 462 = 0b00111001110 (11 bits).
void _encodeAlphanumeric(String input, _BitWriter w) {
  final indices = input.runes.map((c) => _alphanumChars.indexOf(String.fromCharCode(c))).toList();
  var i = 0;

  // Pairs → 11 bits.
  while (i + 1 < indices.length) {
    w.write(indices[i] * 45 + indices[i + 1], 11);
    i += 2;
  }

  // Trailing single → 6 bits.
  if (i < indices.length) {
    w.write(indices[i], 6);
  }
}

/// Encode byte mode: each UTF-8 byte → 8 bits.
///
/// The ISO standard specifies ISO-8859-1, but in practice encoding as UTF-8
/// bytes works with modern scanners. Each byte in the UTF-8 representation
/// is a separate "character" in the count field.
void _encodeByteMode(String input, _BitWriter w) {
  for (final b in input.codeUnits) {
    w.write(b, 8);
  }
}

// ============================================================================
// Data codeword assembly
// ============================================================================

/// Build the complete data codeword byte sequence for [input] in [mode].
///
/// For all symbols except M1:
///   [mode indicator] [char count] [data bits] [terminator] [byte-align] [0xEC/0x11 fill]
///   → exactly cfg.dataCw bytes.
///
/// For M1 ([_SymbolConfig.m1HalfCw] = true):
///   Total capacity = 20 bits = 2 full bytes + 4-bit nibble.
///   The RS encoder receives 3 bytes where byte[2] has data in the upper 4 bits
///   and zeros in the lower 4 bits.
///
/// ## Terminator
///
/// After all data bits, up to [_SymbolConfig.terminatorBits] zero bits are
/// appended. If the data already fills the capacity, the terminator is skipped.
/// This guarantees any scanner reading the stream can find the end-of-data marker.
///
/// ## Padding
///
/// After the terminator, the stream is zero-padded to the next byte boundary,
/// then filled with alternating bytes 0xEC and 0x11 until all data codewords
/// are full. These are the "pad codewords" defined in ISO 18004.
List<int> _buildDataCodewords(String input, _SymbolConfig cfg, _EncodingMode mode) {
  // Total usable data bits.
  // M1 is special: 3 data codewords but only 20 bits (last CW is a nibble).
  final totalBits = cfg.m1HalfCw
      ? cfg.dataCw * 8 - 4 // M1: 3×8 − 4 = 20 bits
      : cfg.dataCw * 8;

  final w = _BitWriter();

  // ── Mode indicator ───────────────────────────────────────────────────────
  // M1 has no mode indicator (only one mode supported).
  // M2: 1 bit. M3: 2 bits. M4: 3 bits.
  if (cfg.modeIndicatorBits > 0) {
    w.write(_modeIndicatorValue(mode, cfg), cfg.modeIndicatorBits);
  }

  // ── Character count ──────────────────────────────────────────────────────
  // Byte mode counts bytes, not characters (multi-byte UTF-8 = multiple bytes).
  final charCount = mode == _EncodingMode.byte ? input.codeUnits.length : input.runes.length;
  w.write(charCount, _charCountBits(mode, cfg));

  // ── Encoded data ─────────────────────────────────────────────────────────
  switch (mode) {
    case _EncodingMode.numeric:
      _encodeNumeric(input, w);
    case _EncodingMode.alphanumeric:
      _encodeAlphanumeric(input, w);
    case _EncodingMode.byte:
      _encodeByteMode(input, w);
  }

  // ── Terminator ───────────────────────────────────────────────────────────
  // Append up to terminatorBits zero bits, truncated if capacity exhausted.
  final remaining = totalBits - w.bitLen;
  if (remaining > 0) {
    final termLen = remaining < cfg.terminatorBits ? remaining : cfg.terminatorBits;
    w.write(0, termLen);
  }

  // ── M1 special handling ──────────────────────────────────────────────────
  if (cfg.m1HalfCw) {
    // Pack into exactly 20 bits → 3 bytes.
    // Byte[2] holds data in upper 4 bits; lower 4 bits are zero.
    // This is what the RS encoder receives for the "half codeword".
    final bits = w.toBitList().toList();
    while (bits.length < 20) {
      bits.add(0);
    }
    final b0 = (bits[0] << 7) | (bits[1] << 6) | (bits[2] << 5) | (bits[3] << 4)
             | (bits[4] << 3) | (bits[5] << 2) | (bits[6] << 1) | bits[7];
    final b1 = (bits[8] << 7) | (bits[9] << 6) | (bits[10] << 5) | (bits[11] << 4)
             | (bits[12] << 3) | (bits[13] << 2) | (bits[14] << 1) | bits[15];
    final b2 = (bits[16] << 7) | (bits[17] << 6) | (bits[18] << 5) | (bits[19] << 4);
    return [b0, b1, b2];
  }

  // ── Pad to byte boundary ─────────────────────────────────────────────────
  // Add 0–7 zero bits to align to the next full byte boundary.
  final mod = w.bitLen % 8;
  if (mod != 0) {
    w.write(0, 8 - mod);
  }

  // ── Fill remaining codewords with pad bytes ──────────────────────────────
  // Alternate 0xEC and 0x11 (the QR/Micro-QR pad codewords) until all
  // data codewords are occupied.
  final bytes = w.toBytes();
  final mutableBytes = List<int>.from(bytes);
  var pad = 0xec;
  while (mutableBytes.length < cfg.dataCw) {
    mutableBytes.add(pad);
    pad = (pad == 0xec) ? 0x11 : 0xec;
  }
  return mutableBytes;
}

// ============================================================================
// Reed-Solomon encoder
// ============================================================================

/// Compute RS ECC bytes using LFSR polynomial division over GF(256)/0x11D.
///
/// Returns the remainder of D(x)·x^n mod G(x), where:
/// - D(x) = the data polynomial (data bytes as coefficients)
/// - G(x) = the generator polynomial (degree n = number of ECC bytes)
/// - All arithmetic is over GF(256) with primitive polynomial 0x11D
///
/// This is the standard "systematic RS encoder" used by QR Code and Micro QR.
/// It uses the b=0 convention: the first root is α^0 = 1.
///
/// The LFSR (Linear Feedback Shift Register) approach:
///   1. Feed each data byte into the shift register.
///   2. The feedback byte is the XOR of the data byte and the current shift[0].
///   3. Shift the register left, insert 0 at the right.
///   4. XOR each position with generator[i+1] × feedback.
///   5. After all data bytes, the register holds the ECC bytes.
List<int> _rsEncode(List<int> data, List<int> generator) {
  final n = generator.length - 1; // number of ECC bytes
  final rem = List<int>.filled(n, 0);

  for (final b in data) {
    // Feedback: XOR the current top-of-register with this data byte.
    // If feedback is 0, the generator adds nothing (0 × anything = 0 in GF(256)).
    final fb = b ^ rem[0];

    // Shift register left: rem[0] is consumed, rem[n-1] gets 0.
    for (var i = 0; i < n - 1; i++) {
      rem[i] = rem[i + 1];
    }
    rem[n - 1] = 0;

    // XOR generator coefficients × feedback into the shifted register.
    if (fb != 0) {
      for (var i = 0; i < n; i++) {
        rem[i] ^= gfMultiply(generator[i + 1], fb);
      }
    }
  }

  return rem;
}

// ============================================================================
// Symbol selection
// ============================================================================

/// Find the smallest symbol configuration that can hold [input].
///
/// If [version] is specified, only that version is considered.
/// If [ecc] is specified, only that ECC level is considered.
///
/// Auto-selection tries configs in [_symbolConfigs] order (M1 → M4, within
/// each version L before M before Q), returning the first fitting config.
///
/// Throws [ECCNotAvailable] if the version/ECC combination doesn't exist.
/// Throws [InputTooLong] if the input doesn't fit in any matching config.
_SymbolConfig _selectConfig(
  String input,
  MicroQRVersion? version,
  MicroQREccLevel? ecc,
) {
  // Filter configs to the requested version and/or ECC level.
  final candidates = _symbolConfigs.where((c) {
    if (version != null && c.version != version) return false;
    if (ecc != null && c.ecc != ecc) return false;
    return true;
  }).toList();

  if (candidates.isEmpty) {
    throw ECCNotAvailable(
      'No symbol configuration matches version=${version?.name} ecc=${ecc?.name}. '
      'M1 only supports detection. M2/M3 support L and M. M4 supports L, M, Q.',
    );
  }

  // Walk candidates in order: try each, return the first that fits.
  for (final cfg in candidates) {
    try {
      final mode = _selectMode(input, cfg);
      final len = mode == _EncodingMode.byte ? input.codeUnits.length : input.runes.length;
      final cap = switch (mode) {
        _EncodingMode.numeric => cfg.numericCap,
        _EncodingMode.alphanumeric => cfg.alphaCap,
        _EncodingMode.byte => cfg.byteCap,
      };
      if (cap > 0 && len <= cap) {
        return cfg;
      }
    } on UnsupportedMode {
      // This config can't encode the input at all; try the next one.
      continue;
    }
  }

  throw InputTooLong(
    'Input (length ${input.length}) does not fit in any Micro QR symbol '
    '(version=${version?.name}, ecc=${ecc?.name}). '
    'Maximum is 35 numeric chars in M4-L. Consider using regular QR Code.',
  );
}

// ============================================================================
// Grid construction
// ============================================================================

/// Mutable working grid with module values and reservation flags.
///
/// The [modules] 2D list holds the dark/light state of each module.
/// The [reserved] 2D list marks which modules cannot be overwritten by data.
///
/// We use a mutable grid internally during construction, then freeze it into
/// an immutable [ModuleGrid] at the end.
class _WorkGrid {
  final int size;

  /// `true` = dark module (ink), `false` = light module (background).
  final List<List<bool>> modules;

  /// `true` = this module position is reserved for structural use.
  final List<List<bool>> reserved;

  _WorkGrid(this.size)
      : modules = List.generate(size, (_) => List<bool>.filled(size, false)),
        reserved = List.generate(size, (_) => List<bool>.filled(size, false));

  /// Set module [r,c] to [dark]; optionally mark it as [reserve]d.
  void set(int r, int c, bool dark, {bool reserve = false}) {
    modules[r][c] = dark;
    if (reserve) reserved[r][c] = true;
  }
}

/// Place the 7×7 finder pattern at the top-left corner (rows 0–6, cols 0–6).
///
/// ```
/// ■ ■ ■ ■ ■ ■ ■   ← row 0: all dark (outer border)
/// ■ □ □ □ □ □ ■   ← row 1: dark-light-light-light-light-light-dark
/// ■ □ ■ ■ ■ □ ■   ← row 2: border, gap, core (3×3), gap, border
/// ■ □ ■ ■ ■ □ ■   ← row 3
/// ■ □ ■ ■ ■ □ ■   ← row 4
/// ■ □ □ □ □ □ ■   ← row 5: dark-light-light-light-light-light-dark
/// ■ ■ ■ ■ ■ ■ ■   ← row 6: all dark (outer border)
/// ```
///
/// The 3×3 dark center is the "bull's-eye" that scanners locate first.
/// The 1-pixel-wide light ring and the dark outer border create the 1:1:3:1:1
/// ratio that scanners detect by scanning through the finder pattern.
void _placeFinder(_WorkGrid g) {
  for (var dr = 0; dr < 7; dr++) {
    for (var dc = 0; dc < 7; dc++) {
      // A module is dark if it's on the outer border OR in the 3×3 core.
      final onBorder = dr == 0 || dr == 6 || dc == 0 || dc == 6;
      final inCore = dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4;
      g.set(dr, dc, onBorder || inCore, reserve: true);
    }
  }
}

/// Place the L-shaped separator around the finder pattern.
///
/// Unlike regular QR which surrounds three finders on all four sides, Micro QR
/// has only one finder in the top-left corner. The top and left sides of the
/// finder are the symbol boundary itself, so we only need:
///
/// ```
/// Row 7, cols 0–7  (bottom of finder)  ← all light
/// Col 7, rows 0–7  (right of finder)   ← all light
/// ```
///
/// The corner module at (7,7) is covered by both rules and set to light once.
void _placeSeparator(_WorkGrid g) {
  for (var i = 0; i <= 7; i++) {
    g.set(7, i, false, reserve: true); // bottom edge
    g.set(i, 7, false, reserve: true); // right edge
  }
}

/// Place timing pattern extensions along row 0 and column 0.
///
/// Positions 0–6 in row 0 and col 0 are already covered by the finder pattern.
/// Position 7 is the separator (always light). Starting at position 8, the
/// timing pattern extends outward to the opposite edge of the symbol.
///
/// Timing rule: module at index k is dark if k is even, light if k is odd.
/// This starts dark at k=8 (even). Example for M4 (size=17):
///
/// ```
/// Row 0: col 8=dark, col 9=light, col 10=dark, ..., col 16=dark
/// Col 0: row 8=dark, row 9=light, row 10=dark, ..., row 16=dark
/// ```
///
/// The alternating pattern provides a spatial "ruler" that scanners use to
/// measure module size and compensate for distortion.
void _placeTiming(_WorkGrid g) {
  for (var c = 8; c < g.size; c++) {
    g.set(0, c, c % 2 == 0, reserve: true);
  }
  for (var r = 8; r < g.size; r++) {
    g.set(r, 0, r % 2 == 0, reserve: true);
  }
}

/// Reserve the 15 format information module positions.
///
/// Format information occupies 15 modules in an L-shape:
/// ```
/// Row 8, cols 1–8  → 8 modules  (bits f14 down to f7, MSB first)
/// Col 8, rows 1–7  → 7 modules  (bits f6 down to f0, f6 at row 7)
/// ```
///
/// These are marked reserved before data placement and filled in after mask
/// selection. There is only ONE copy of format info in Micro QR (not two as
/// in regular QR) — if these modules are damaged, the symbol cannot be decoded.
void _reserveFormatInfo(_WorkGrid g) {
  for (var c = 1; c <= 8; c++) {
    g.set(8, c, false, reserve: true);
  }
  for (var r = 1; r <= 7; r++) {
    g.set(r, 8, false, reserve: true);
  }
}

/// Write the 15-bit [fmt] word into the reserved format information positions.
///
/// Bit assignment (f14 = MSB, f0 = LSB):
/// ```
/// Row 8: col 1 ← f14, col 2 ← f13, ..., col 8 ← f7
/// Col 8: row 7 ← f6,  row 6 ← f5,  ..., row 1 ← f0
/// ```
///
/// The L-shape reading order: sweep right along row 8 (f14→f7), then sweep
/// up along col 8 (f6→f0). Row 7 holds f6, row 1 holds f0 (LSB nearest
/// the finder corner).
void _writeFormatInfo(_WorkGrid g, int fmt) {
  // Row 8, cols 1–8: bits f14 down to f7.
  for (var i = 0; i < 8; i++) {
    g.modules[8][1 + i] = ((fmt >> (14 - i)) & 1) == 1;
  }
  // Col 8, rows 7 down to 1: bits f6 down to f0.
  for (var i = 0; i < 7; i++) {
    g.modules[7 - i][8] = ((fmt >> (6 - i)) & 1) == 1;
  }
}

/// Initialize the grid with all structural modules for [cfg].
///
/// Runs the full structural setup in order:
///   1. Finder pattern (7×7 corner)
///   2. L-shaped separator
///   3. Timing pattern extensions (row 0 and col 0 from position 8 onward)
///   4. Format information reservation
_WorkGrid _buildGrid(_SymbolConfig cfg) {
  final g = _WorkGrid(cfg.size);
  _placeFinder(g);
  _placeSeparator(g);
  _placeTiming(g);
  _reserveFormatInfo(g);
  return g;
}

// ============================================================================
// Data placement (two-column zigzag)
// ============================================================================

/// Place [bits] from the final codeword stream into [g] via two-column zigzag.
///
/// The zigzag scans from the bottom-right corner, moving left two columns at
/// a time, alternating upward and downward directions. Reserved modules are
/// skipped; any bits that run out before all free modules are filled set the
/// remaining modules to false (light = 0 = remainder bits).
///
/// Unlike regular QR Code, there is no timing column at col 6 to jump over.
/// Micro QR's timing is at col 0, which is reserved and auto-skipped.
///
/// ```
/// Starting state (17×17 symbol, showing col scan order):
///
///   ← col scan moves right-to-left, 2 columns at a time
///   ↑↓ direction alternates: first strip goes up, next goes down, etc.
///
///   Strips (right → left):
///     cols 16-15: up
///     cols 14-13: down
///     cols 12-11: up
///     ...
///     cols 2-1:   up or down
///     (col 0 is all timing/separator — fully reserved, last strip auto-skips)
/// ```
void _placeBits(_WorkGrid g, List<bool> bits) {
  final sz = g.size;
  var bitIdx = 0;
  var goingUp = true;

  var col = sz - 1;
  while (col >= 1) {
    // Scan this two-column strip in the current direction.
    for (var vi = 0; vi < sz; vi++) {
      final row = goingUp ? (sz - 1 - vi) : vi;

      // Right column first, then left column of the pair.
      for (var dc = 0; dc <= 1; dc++) {
        final c = col - dc;
        if (g.reserved[row][c]) continue;

        // Place the next bit, or 0 (remainder) if bits exhausted.
        g.modules[row][c] = bitIdx < bits.length ? bits[bitIdx++] : false;
      }
    }

    goingUp = !goingUp;
    col -= 2;
  }
}

// ============================================================================
// Masking
// ============================================================================

/// Return true if mask pattern [maskIdx] applies to module at ([row], [col]).
///
/// Micro QR uses only 4 mask patterns (0–3), which are the first four of
/// regular QR's eight patterns. The more complex patterns (4–7) are absent
/// because Micro QR symbols are small enough that simpler patterns suffice.
///
/// | Pattern | Condition (flip if true)     |
/// |---------|------------------------------|
/// | 0       | (row + col) mod 2 == 0       |
/// | 1       | row mod 2 == 0               |
/// | 2       | col mod 3 == 0               |
/// | 3       | (row + col) mod 3 == 0       |
bool _maskCondition(int maskIdx, int row, int col) {
  return switch (maskIdx) {
    0 => (row + col) % 2 == 0,
    1 => row % 2 == 0,
    2 => col % 3 == 0,
    3 => (row + col) % 3 == 0,
    _ => false,
  };
}

/// Apply mask pattern [maskIdx] to non-reserved modules. Returns a new grid.
///
/// Masking XORs data/ECC modules with a geometric pattern to break up
/// degenerate sequences (all-dark rows, checkerboards, finder-like patterns)
/// that could confuse scanner pattern recognition.
///
/// Structural modules (finder, separator, timing, format) are never masked.
List<List<bool>> _applyMask(
  List<List<bool>> modules,
  List<List<bool>> reserved,
  int sz,
  int maskIdx,
) {
  final result = List.generate(sz, (r) => List<bool>.from(modules[r]));
  for (var r = 0; r < sz; r++) {
    for (var c = 0; c < sz; c++) {
      if (!reserved[r][c]) {
        // XOR with mask condition: if condition is true, flip the module.
        result[r][c] = modules[r][c] != _maskCondition(maskIdx, r, c);
      }
    }
  }
  return result;
}

// ============================================================================
// Penalty scoring
// ============================================================================

/// Compute the 4-rule penalty score for [modules] (same rules as regular QR).
///
/// The scorer evaluates how "bad" a given masked grid looks to a scanner.
/// Lower scores are better — the mask with the lowest penalty is selected.
///
/// ## Rule 1 — Adjacent run penalty
///
/// Scan rows and columns for runs of ≥ 5 consecutive same-color modules.
/// Add `run_length − 2` for each qualifying run:
///   run of 5 → +3, run of 6 → +4, etc.
///
/// ## Rule 2 — 2×2 block penalty
///
/// For each 2×2 square with all four modules the same color, add 3.
///
/// ## Rule 3 — Finder-pattern-like sequences
///
/// For each occurrence of [1,0,1,1,1,0,1,0,0,0,0] or its reverse in any
/// row or column, add 40. These look like a finder pattern to scanners.
///
/// ## Rule 4 — Dark proportion
///
/// ```
/// dark_pct = dark_count * 100 / total
/// prev5 = largest multiple of 5 ≤ dark_pct
/// next5 = prev5 + 5
/// penalty = min(|prev5 - 50|, |next5 - 50|) / 5 * 10
/// ```
///
/// Penalty is 0 at exactly 50% dark, increasing as the ratio tilts.
int _computePenalty(List<List<bool>> modules, int sz) {
  var penalty = 0;

  // ── Rule 1: Adjacent same-color runs ≥ 5 ─────────────────────────────────
  for (var a = 0; a < sz; a++) {
    for (var horiz in [true, false]) {
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

  // ── Rule 2: 2×2 same-color blocks ────────────────────────────────────────
  for (var r = 0; r < sz - 1; r++) {
    for (var c = 0; c < sz - 1; c++) {
      final d = modules[r][c];
      if (d == modules[r][c + 1] && d == modules[r + 1][c] && d == modules[r + 1][c + 1]) {
        penalty += 3;
      }
    }
  }

  // ── Rule 3: Finder-pattern-like sequences ─────────────────────────────────
  // Pattern P1: 1 0 1 1 1 0 1 0 0 0 0  (looks like a finder + quiet zone)
  // Pattern P2: 0 0 0 0 1 0 1 1 1 0 1  (reverse of P1)
  const p1 = [1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0];
  const p2 = [0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1];
  if (sz >= 11) {
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
  }

  // ── Rule 4: Dark proportion deviation from 50% ───────────────────────────
  var dark = 0;
  for (var r = 0; r < sz; r++) {
    for (var c = 0; c < sz; c++) {
      if (modules[r][c]) dark++;
    }
  }
  final total = sz * sz;
  final darkPct = (dark * 100) ~/ total;
  final prev5 = (darkPct ~/ 5) * 5;
  final next5 = prev5 + 5;
  final r4 = (prev5 - 50).abs() < (next5 - 50).abs()
      ? (prev5 - 50).abs()
      : (next5 - 50).abs();
  penalty += (r4 ~/ 5) * 10;

  return penalty;
}

// ============================================================================
// Public API
// ============================================================================

/// Encode [input] to a Micro QR Code [ModuleGrid].
///
/// Automatically selects the smallest symbol (M1..M4) and ECC level that can
/// hold the input. Pass [version] and/or [ecc] to override auto-selection.
///
/// ## Auto-selection
///
/// Symbols are tried in order M1 → M4, within each version L before M before Q.
/// The first configuration where the input fits is used. Encoding mode is also
/// auto-selected: numeric > alphanumeric > byte.
///
/// ## ECC notes
///
/// - If [ecc] is omitted, the default is M (medium) for M2–M4, or detection
///   for M1 (the only option).
/// - M1 only supports [MicroQREccLevel.detection].
/// - M4 is the only symbol with [MicroQREccLevel.q].
///
/// ## Errors
///
/// - [InputTooLong] if the input exceeds M4 capacity (35 numeric chars).
/// - [ECCNotAvailable] if the version+ECC combination doesn't exist.
/// - [UnsupportedMode] if no encoding mode is available in the selected symbol.
///
/// ## Example
///
/// ```dart
/// // Auto-select: "HELLO" → M2-L alphanumeric (13×13 symbol)
/// final grid = encode('HELLO');
/// print(grid.rows);  // 13
///
/// // Force M4 for maximum capacity:
/// final m4 = encode('https://a.b', version: MicroQRVersion.m4, ecc: MicroQREccLevel.l);
/// print(m4.rows);  // 17
/// ```
ModuleGrid encode(
  String input, {
  MicroQRVersion? version,
  MicroQREccLevel? ecc,
}) {
  // ── Step 1: Select symbol configuration ───────────────────────────────────
  final cfg = _selectConfig(input, version, ecc);
  final mode = _selectMode(input, cfg);

  // ── Step 2: Build data codewords ─────────────────────────────────────────
  final dataCw = _buildDataCodewords(input, cfg, mode);

  // ── Step 3: Compute Reed-Solomon ECC ─────────────────────────────────────
  final generator = _getGenerator(cfg.eccCw);
  final eccCw = _rsEncode(dataCw, generator);

  // ── Step 4: Flatten to bit stream ─────────────────────────────────────────
  // Concatenate data and ECC codewords.
  // For M1: the last data codeword contributes only 4 bits (upper nibble).
  // All ECC codewords are always full 8-bit bytes.
  final finalCw = [...dataCw, ...eccCw];
  final bits = <bool>[];
  for (var cwIdx = 0; cwIdx < finalCw.length; cwIdx++) {
    final cw = finalCw[cwIdx];
    // M1's last data CW is a 4-bit nibble (high 4 bits only).
    final bitsInCw = (cfg.m1HalfCw && cwIdx == cfg.dataCw - 1) ? 4 : 8;
    for (var b = bitsInCw - 1; b >= 0; b--) {
      bits.add(((cw >> (b + (8 - bitsInCw))) & 1) == 1);
    }
  }

  // ── Step 5: Build grid with structural modules ────────────────────────────
  final grid = _buildGrid(cfg);

  // ── Step 6: Place data bits ───────────────────────────────────────────────
  _placeBits(grid, bits);

  // ── Step 7: Evaluate all 4 masks, pick lowest penalty ────────────────────
  var bestMask = 0;
  var bestPenalty = 0x7fffffff;

  for (var maskIdx = 0; maskIdx < 4; maskIdx++) {
    // Apply mask to non-reserved modules.
    final masked = _applyMask(grid.modules, grid.reserved, cfg.size, maskIdx);

    // Write format information for this mask (into a temporary grid copy).
    // We inline the write rather than creating a full _WorkGrid copy.
    final fmt = _formatTable[cfg.symbolIndicator][maskIdx];
    final tmpModules = List.generate(cfg.size, (r) => List<bool>.from(masked[r]));
    for (var i = 0; i < 8; i++) {
      tmpModules[8][1 + i] = ((fmt >> (14 - i)) & 1) == 1;
    }
    for (var i = 0; i < 7; i++) {
      tmpModules[7 - i][8] = ((fmt >> (6 - i)) & 1) == 1;
    }

    final p = _computePenalty(tmpModules, cfg.size);
    if (p < bestPenalty) {
      bestPenalty = p;
      bestMask = maskIdx;
    }
  }

  // ── Step 8: Apply best mask and write final format information ─────────────
  final finalModules = _applyMask(grid.modules, grid.reserved, cfg.size, bestMask);
  final finalFmt = _formatTable[cfg.symbolIndicator][bestMask];

  // Write format info directly into the final modules list.
  for (var i = 0; i < 8; i++) {
    finalModules[8][1 + i] = ((finalFmt >> (14 - i)) & 1) == 1;
  }
  for (var i = 0; i < 7; i++) {
    finalModules[7 - i][8] = ((finalFmt >> (6 - i)) & 1) == 1;
  }

  // ── Step 9: Wrap in immutable ModuleGrid ──────────────────────────────────
  return ModuleGrid(
    rows: cfg.size,
    cols: cfg.size,
    modules: finalModules,
    moduleShape: ModuleShape.square,
  );
}

/// Encode [input] to a specific symbol [version] and [ecc] level.
///
/// Unlike [encode], this never auto-selects — it uses exactly the version and
/// ECC level you provide. Throws [InputTooLong] if the input doesn't fit.
///
/// ## Example
///
/// ```dart
/// // Explicitly request M4-L (17×17, low ECC)
/// final grid = encodeAt('HELLO', MicroQRVersion.m4, MicroQREccLevel.l);
/// print(grid.rows);  // 17
/// ```
ModuleGrid encodeAt(
  String input,
  MicroQRVersion version,
  MicroQREccLevel ecc,
) {
  return encode(input, version: version, ecc: ecc);
}

/// Convert a [ModuleGrid] to a [PaintScene] via barcode-2d's [layout] function.
///
/// Defaults to a 2-module quiet zone (Micro QR's minimum — half of regular QR's
/// required 4-module zone). Pass [config] to override any layout settings.
///
/// ## Example
///
/// ```dart
/// final grid = encode('HELLO');
/// final scene = layoutGrid(grid);
/// // scene is ready for an SVG or canvas paint backend
/// ```
PaintScene layoutGrid(
  ModuleGrid grid, {
  Barcode2DLayoutConfig? config,
}) {
  final cfg = config ??
      const Barcode2DLayoutConfig(
        moduleSizePx: 10,
        quietZoneModules: 2, // Micro QR: 2 modules (not the regular QR 4)
        foreground: '#000000',
        background: '#ffffff',
        moduleShape: ModuleShape.square,
      );
  return layout(grid, config: cfg);
}

/// Encode [input] and immediately lay out into a [PaintScene].
///
/// Convenience wrapper: calls [encode] then [layoutGrid] in one step.
///
/// ## Example
///
/// ```dart
/// final scene = encodeAndLayout('HELLO');
/// ```
PaintScene encodeAndLayout(
  String input, {
  MicroQRVersion? version,
  MicroQREccLevel? ecc,
  Barcode2DLayoutConfig? config,
}) {
  final grid = encode(input, version: version, ecc: ecc);
  return layoutGrid(grid, config: config);
}
