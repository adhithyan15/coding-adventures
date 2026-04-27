/// Data Matrix ECC200 encoder — ISO/IEC 16022:2006 compliant.
///
/// ## What is Data Matrix?
///
/// Data Matrix is a 2-D matrix barcode invented in 1989 (originally "DataCode")
/// and standardised as ISO/IEC 16022:2006. ECC200 is the modern variant — it
/// uses Reed-Solomon error correction over GF(256) — and has displaced the
/// older ECC000–ECC140 lineage worldwide.
///
/// ```
/// ■ ■ ■ ■ ■ ■ ■ ■ ■ ■   ← row 0: timing clock (alternating dark/light)
/// ■ · ■ · · · · · · ■   ← outer border continues
/// ■ ■ · ■ ■ ■ · · · ■
/// ■ · ■ · ■ · ■ · · ■
/// ■ ■ ■ ■ ■ ■ ■ ■ ■ ■   ← bottom row: L-finder (all dark)
/// ```
///
/// The **L-shaped finder** (left column + bottom row, all dark) tells a scanner
/// which corner of the symbol is the origin and which 90° orientation it is in.
/// The **timing border** (top row + right column, alternating dark/light) gives
/// a spatial ruler for module size calibration.
///
/// ## Key differences from QR Code
///
/// | Property         | QR Code              | Data Matrix ECC200      |
/// |------------------|----------------------|-------------------------|
/// | GF(256) poly     | 0x11D                | 0x12D                   |
/// | RS root start    | b = 0 (α⁰ …)        | b = 1 (α¹ …)            |
/// | Finder           | three corner squares | L-shape (left + bottom) |
/// | Data placement   | column zigzag        | "Utah" diagonal         |
/// | Masking          | 8 patterns, scored   | NONE                    |
/// | Sizes            | 40 versions          | 24 square + 6 rect      |
///
/// ## Encoding pipeline
///
/// ```
/// input string
///   → ASCII encoding    (char+1; digit pairs → 130+pair)
///   → symbol selection  (smallest symbol whose data_cw ≥ codeword count)
///   → pad to capacity   (EOM=129, then scrambled pads)
///   → RS blocks + ECC   (GF(256)/0x12D, b=1 convention)
///   → interleave blocks (round-robin data then ECC)
///   → grid init         (L-finder + timing border + alignment borders)
///   → Utah placement    (diagonal codeword placement — no masking)
///   → ModuleGrid
/// ```
library data_matrix;

import 'package:coding_adventures_barcode_2d/coding_adventures_barcode_2d.dart';

// ============================================================================
// Package version
// ============================================================================

/// Package version following Semantic Versioning 2.0.
const String dataMatrixVersion = '0.1.0';

// ============================================================================
// GF(256) constants
// ============================================================================

/// The primitive polynomial used by Data Matrix ECC200 for GF(256) arithmetic.
///
/// p(x) = x⁸ + x⁵ + x⁴ + x² + x + 1 = 0x12D = 301
///
/// IMPORTANT: This is DIFFERENT from QR Code's 0x11D polynomial. Never mix
/// GF tables between QR Code and Data Matrix.
const int gf256Prime = 0x12D;

// ============================================================================
// Symbol size constants
// ============================================================================

/// Smallest supported square Data Matrix dimension (10×10).
const int minSize = 10;

/// Largest supported square Data Matrix dimension (144×144).
const int maxSize = 144;

// ============================================================================
// Public types — SymbolShape
// ============================================================================

/// Controls which symbol shapes are considered during auto-selection.
///
/// | Value       | Meaning                                                 |
/// |-------------|-------------------------------------------------------- |
/// | square      | Only the 24 square sizes (10×10 … 144×144). Default.    |
/// | rectangle   | Only the 6 rectangular sizes (8×18 … 16×48).            |
/// | any         | Both shapes, picks smallest by total module count.       |
enum SymbolShape {
  /// Only consider square symbols (default, most common in practice).
  square,

  /// Only consider rectangular symbols (for constrained print areas).
  rectangle,

  /// Consider both square and rectangular; pick the smallest that fits.
  any,
}

// ============================================================================
// Options
// ============================================================================

/// Encoding options for [encode].
///
/// ```dart
/// final grid = encode('HELLO', options: DataMatrixOptions(shape: SymbolShape.any));
/// ```
class DataMatrixOptions {
  /// Force a specific symbol size (rows dimension for square; null = auto).
  ///
  /// When non-null, the encoder selects the symbol with the matching
  /// dimension rather than picking the smallest fitting symbol. Throws
  /// [InvalidSizeError] if no ECC200 symbol matches.
  final int? size;

  /// Shape preference for auto-selection.
  ///
  /// Ignored when [size] is non-null. Defaults to [SymbolShape.square].
  final SymbolShape shape;

  const DataMatrixOptions({
    this.size,
    this.shape = SymbolShape.square,
  });
}

// ============================================================================
// Error hierarchy
// ============================================================================

/// Base class for all Data Matrix encoder errors.
///
/// Catch [DataMatrixError] to handle any encoder failure without caring about
/// the specific subtype.
///
/// ```dart
/// try {
///   encode(data);
/// } on DataMatrixError catch (e) {
///   print('Encoder failed: $e');
/// }
/// ```
abstract class DataMatrixError implements Exception {
  /// Human-readable description of what went wrong.
  final String message;

  const DataMatrixError(this.message);

  @override
  String toString() => '$runtimeType: $message';
}

/// Thrown when the input encodes to more codewords than any symbol can hold.
///
/// The largest ECC200 symbol (144×144) holds 1558 data codewords.
/// If the input exceeds this, consider splitting across multiple symbols
/// (Structured Append) or using a different format.
class InputTooLongError extends DataMatrixError {
  const InputTooLongError(super.message);
}

/// Thrown when the caller-specified [DataMatrixOptions.size] is invalid.
///
/// Valid square sizes: 10, 12, 14, 16, 18, 20, 22, 24, 26, 32, 36, 40, 44,
/// 48, 52, 64, 72, 80, 88, 96, 104, 120, 132, 144.
/// Valid rectangular sizes: 8×18, 8×32, 12×26, 12×36, 16×36, 16×48.
class InvalidSizeError extends DataMatrixError {
  const InvalidSizeError(super.message);
}

// ============================================================================
// Symbol size table — ISO/IEC 16022:2006 Table 7
// ============================================================================

/// One ECC200 symbol size entry and its capacity parameters.
///
/// A Data Matrix ECC200 symbol consists of an outer border (1 module wide)
/// surrounding one or more rectangular data regions separated by 2-module
/// alignment borders.
///
/// ```
/// Physical layout for a 32×32 symbol (2×2 regions, each 14×14 data modules):
///
///   1 module  ← outer border (finder + timing)
///   14 modules ← data region 0,0
///   2 modules  ← alignment border
///   14 modules ← data region 0,1
///   1 module  ← outer border
/// ```
class _SymbolEntry {
  /// Total symbol size in modules (including outer border).
  final int symbolRows;
  final int symbolCols;

  /// Number of data region rows / columns.
  ///
  /// For single-region symbols: regionRows = regionCols = 1.
  /// For 144×144: regionRows = regionCols = 6.
  final int regionRows;
  final int regionCols;

  /// Interior size of each data region (excluding the alignment border).
  final int dataRegionHeight;
  final int dataRegionWidth;

  /// Total data codeword capacity.
  final int dataCw;

  /// Total ECC codewords appended after data.
  final int eccCw;

  /// Number of interleaved Reed-Solomon blocks.
  final int numBlocks;

  /// ECC codewords per block (same for all blocks in one symbol).
  final int eccPerBlock;

  const _SymbolEntry({
    required this.symbolRows,
    required this.symbolCols,
    required this.regionRows,
    required this.regionCols,
    required this.dataRegionHeight,
    required this.dataRegionWidth,
    required this.dataCw,
    required this.eccCw,
    required this.numBlocks,
    required this.eccPerBlock,
  });
}

/// All 24 square ECC200 symbol sizes, from ISO/IEC 16022:2006 Table 7.
///
/// Ordered by data capacity ascending. Each entry corresponds to one physical
/// symbol size, from the tiny 10×10 (3 data codewords) up to the 144×144
/// (1558 data codewords). Encoders walk this list and return the first entry
/// whose `dataCw` meets or exceeds the encoded codeword count.
const List<_SymbolEntry> _squareSizes = [
  _SymbolEntry(symbolRows: 10,  symbolCols: 10,  regionRows: 1, regionCols: 1, dataRegionHeight: 8,  dataRegionWidth: 8,  dataCw: 3,    eccCw: 5,   numBlocks: 1,  eccPerBlock: 5),
  _SymbolEntry(symbolRows: 12,  symbolCols: 12,  regionRows: 1, regionCols: 1, dataRegionHeight: 10, dataRegionWidth: 10, dataCw: 5,    eccCw: 7,   numBlocks: 1,  eccPerBlock: 7),
  _SymbolEntry(symbolRows: 14,  symbolCols: 14,  regionRows: 1, regionCols: 1, dataRegionHeight: 12, dataRegionWidth: 12, dataCw: 8,    eccCw: 10,  numBlocks: 1,  eccPerBlock: 10),
  _SymbolEntry(symbolRows: 16,  symbolCols: 16,  regionRows: 1, regionCols: 1, dataRegionHeight: 14, dataRegionWidth: 14, dataCw: 12,   eccCw: 12,  numBlocks: 1,  eccPerBlock: 12),
  _SymbolEntry(symbolRows: 18,  symbolCols: 18,  regionRows: 1, regionCols: 1, dataRegionHeight: 16, dataRegionWidth: 16, dataCw: 18,   eccCw: 14,  numBlocks: 1,  eccPerBlock: 14),
  _SymbolEntry(symbolRows: 20,  symbolCols: 20,  regionRows: 1, regionCols: 1, dataRegionHeight: 18, dataRegionWidth: 18, dataCw: 22,   eccCw: 18,  numBlocks: 1,  eccPerBlock: 18),
  _SymbolEntry(symbolRows: 22,  symbolCols: 22,  regionRows: 1, regionCols: 1, dataRegionHeight: 20, dataRegionWidth: 20, dataCw: 30,   eccCw: 20,  numBlocks: 1,  eccPerBlock: 20),
  _SymbolEntry(symbolRows: 24,  symbolCols: 24,  regionRows: 1, regionCols: 1, dataRegionHeight: 22, dataRegionWidth: 22, dataCw: 36,   eccCw: 24,  numBlocks: 1,  eccPerBlock: 24),
  _SymbolEntry(symbolRows: 26,  symbolCols: 26,  regionRows: 1, regionCols: 1, dataRegionHeight: 24, dataRegionWidth: 24, dataCw: 44,   eccCw: 28,  numBlocks: 1,  eccPerBlock: 28),
  _SymbolEntry(symbolRows: 32,  symbolCols: 32,  regionRows: 2, regionCols: 2, dataRegionHeight: 14, dataRegionWidth: 14, dataCw: 62,   eccCw: 36,  numBlocks: 2,  eccPerBlock: 18),
  _SymbolEntry(symbolRows: 36,  symbolCols: 36,  regionRows: 2, regionCols: 2, dataRegionHeight: 16, dataRegionWidth: 16, dataCw: 86,   eccCw: 42,  numBlocks: 2,  eccPerBlock: 21),
  _SymbolEntry(symbolRows: 40,  symbolCols: 40,  regionRows: 2, regionCols: 2, dataRegionHeight: 18, dataRegionWidth: 18, dataCw: 114,  eccCw: 48,  numBlocks: 2,  eccPerBlock: 24),
  _SymbolEntry(symbolRows: 44,  symbolCols: 44,  regionRows: 2, regionCols: 2, dataRegionHeight: 20, dataRegionWidth: 20, dataCw: 144,  eccCw: 56,  numBlocks: 4,  eccPerBlock: 14),
  _SymbolEntry(symbolRows: 48,  symbolCols: 48,  regionRows: 2, regionCols: 2, dataRegionHeight: 22, dataRegionWidth: 22, dataCw: 174,  eccCw: 68,  numBlocks: 4,  eccPerBlock: 17),
  _SymbolEntry(symbolRows: 52,  symbolCols: 52,  regionRows: 2, regionCols: 2, dataRegionHeight: 24, dataRegionWidth: 24, dataCw: 204,  eccCw: 84,  numBlocks: 4,  eccPerBlock: 21),
  _SymbolEntry(symbolRows: 64,  symbolCols: 64,  regionRows: 4, regionCols: 4, dataRegionHeight: 14, dataRegionWidth: 14, dataCw: 280,  eccCw: 112, numBlocks: 4,  eccPerBlock: 28),
  _SymbolEntry(symbolRows: 72,  symbolCols: 72,  regionRows: 4, regionCols: 4, dataRegionHeight: 16, dataRegionWidth: 16, dataCw: 368,  eccCw: 144, numBlocks: 4,  eccPerBlock: 36),
  _SymbolEntry(symbolRows: 80,  symbolCols: 80,  regionRows: 4, regionCols: 4, dataRegionHeight: 18, dataRegionWidth: 18, dataCw: 456,  eccCw: 192, numBlocks: 4,  eccPerBlock: 48),
  _SymbolEntry(symbolRows: 88,  symbolCols: 88,  regionRows: 4, regionCols: 4, dataRegionHeight: 20, dataRegionWidth: 20, dataCw: 576,  eccCw: 224, numBlocks: 4,  eccPerBlock: 56),
  _SymbolEntry(symbolRows: 96,  symbolCols: 96,  regionRows: 4, regionCols: 4, dataRegionHeight: 22, dataRegionWidth: 22, dataCw: 696,  eccCw: 272, numBlocks: 4,  eccPerBlock: 68),
  _SymbolEntry(symbolRows: 104, symbolCols: 104, regionRows: 4, regionCols: 4, dataRegionHeight: 24, dataRegionWidth: 24, dataCw: 816,  eccCw: 336, numBlocks: 6,  eccPerBlock: 56),
  _SymbolEntry(symbolRows: 120, symbolCols: 120, regionRows: 6, regionCols: 6, dataRegionHeight: 18, dataRegionWidth: 18, dataCw: 1050, eccCw: 408, numBlocks: 6,  eccPerBlock: 68),
  _SymbolEntry(symbolRows: 132, symbolCols: 132, regionRows: 6, regionCols: 6, dataRegionHeight: 20, dataRegionWidth: 20, dataCw: 1304, eccCw: 496, numBlocks: 8,  eccPerBlock: 62),
  _SymbolEntry(symbolRows: 144, symbolCols: 144, regionRows: 6, regionCols: 6, dataRegionHeight: 22, dataRegionWidth: 22, dataCw: 1558, eccCw: 620, numBlocks: 10, eccPerBlock: 62),
];

/// The 6 rectangular ECC200 symbol sizes from ISO/IEC 16022:2006 Table 7.
///
/// Rectangles are useful for constrained print areas (long thin labels, cable
/// wraps, etc.). All 6 are single-region symbols (regionRows = regionCols = 1).
const List<_SymbolEntry> _rectSizes = [
  _SymbolEntry(symbolRows: 8,  symbolCols: 18, regionRows: 1, regionCols: 1, dataRegionHeight: 6,  dataRegionWidth: 16, dataCw: 5,  eccCw: 7,  numBlocks: 1, eccPerBlock: 7),
  _SymbolEntry(symbolRows: 8,  symbolCols: 32, regionRows: 1, regionCols: 2, dataRegionHeight: 6,  dataRegionWidth: 14, dataCw: 10, eccCw: 11, numBlocks: 1, eccPerBlock: 11),
  _SymbolEntry(symbolRows: 12, symbolCols: 26, regionRows: 1, regionCols: 1, dataRegionHeight: 10, dataRegionWidth: 24, dataCw: 16, eccCw: 14, numBlocks: 1, eccPerBlock: 14),
  _SymbolEntry(symbolRows: 12, symbolCols: 36, regionRows: 1, regionCols: 2, dataRegionHeight: 10, dataRegionWidth: 16, dataCw: 22, eccCw: 18, numBlocks: 1, eccPerBlock: 18),
  _SymbolEntry(symbolRows: 16, symbolCols: 36, regionRows: 1, regionCols: 2, dataRegionHeight: 14, dataRegionWidth: 16, dataCw: 32, eccCw: 24, numBlocks: 1, eccPerBlock: 24),
  _SymbolEntry(symbolRows: 16, symbolCols: 48, regionRows: 1, regionCols: 2, dataRegionHeight: 14, dataRegionWidth: 22, dataCw: 49, eccCw: 28, numBlocks: 1, eccPerBlock: 28),
];

/// Maximum data codeword count across all symbols (for error messages).
const int _maxDataCw = 1558;

// ============================================================================
// GF(256) arithmetic over 0x12D — Data Matrix field
// ============================================================================
//
// Data Matrix ECC200 uses GF(256) with primitive polynomial 0x12D:
//
//   p(x) = x⁸ + x⁵ + x⁴ + x² + x + 1  =  0x12D  =  301
//
// This is DIFFERENT from QR Code's 0x11D. We build our own exp/log tables
// here to use the correct field. The generator g = 2 (polynomial x) is
// primitive in this field — it produces all 255 non-zero elements as
// successive powers α⁰, α¹, …, α²⁵⁴.

/// Antilog table: _dmExp[i] = α^i in GF(256)/0x12D.
///
/// Computed once at startup. _dmExp[255] = _dmExp[0] = 1 (group order 255).
final List<int> _dmExp = List<int>.filled(256, 0);

/// Log table: _dmLog[v] = i such that α^i = v in GF(256)/0x12D.
///
/// _dmLog[0] is undefined (0 is not a power of any element).
final List<int> _dmLog = List<int>.filled(256, 0);

/// Whether the Data Matrix GF tables have been built.
bool _dmTablesBuilt = false;

/// Build the exp/log tables for GF(256)/0x12D.
///
/// Algorithm: start with val = 1 (= α⁰). At each step, shift left by one bit
/// (multiply by α = x). If bit 8 overflows, XOR with 0x12D to reduce modulo
/// the primitive polynomial. After 255 steps every non-zero element appears
/// exactly once — confirming that α = 2 is primitive in this field.
void _buildDmTables() {
  if (_dmTablesBuilt) return;

  var val = 1;
  for (var i = 0; i < 255; i++) {
    _dmExp[i] = val;
    _dmLog[val] = i;
    val <<= 1;
    if (val & 0x100 != 0) {
      val ^= 0x12D; // reduce modulo p(x)
    }
  }
  // α^255 = α^0 = 1: the multiplicative group has order 255.
  _dmExp[255] = _dmExp[0];

  _dmTablesBuilt = true;
}

/// Multiply two GF(256)/0x12D field elements using log/antilog tables.
///
/// For a, b ≠ 0: a × b = α^{(log[a] + log[b]) mod 255}.
/// If either operand is 0, the product is 0 (zero absorbs multiplication).
int _gfMul(int a, int b) {
  _buildDmTables();
  if (a == 0 || b == 0) return 0;
  return _dmExp[(_dmLog[a] + _dmLog[b]) % 255];
}

// ============================================================================
// RS generator polynomials (GF(256)/0x12D, b=1 convention)
// ============================================================================
//
// Data Matrix uses the b=1 convention: the generator's roots are α¹, α², …, α^n
// (not α⁰, α¹, …, α^{n-1} like QR Code). This shifts every coefficient.
//
// Generator: g(x) = (x + α¹)(x + α²) ··· (x + α^{n_ecc})
//
// We cache built generators to avoid recomputation for the same block size.

final Map<int, List<int>> _genCache = {};

/// Build the RS generator polynomial for [nEcc] ECC codewords.
///
/// Starts with g = [1], then for each root α^i (i from 1 to nEcc):
///   Multiply g by the linear factor (x + α^i).
///
/// The result has degree nEcc, with nEcc+1 coefficients (highest-degree first).
List<int> _buildGenerator(int nEcc) {
  _buildDmTables();

  List<int> g = [1];
  for (var i = 1; i <= nEcc; i++) {
    final ai = _dmExp[i]; // α^i root of this factor
    final newG = List<int>.filled(g.length + 1, 0);
    for (var j = 0; j < g.length; j++) {
      newG[j] ^= g[j];                  // coeff × x term
      newG[j + 1] ^= _gfMul(g[j], ai); // coeff × α^i constant term
    }
    g = newG;
  }
  return g;
}

/// Return the cached generator polynomial for [nEcc] ECC codewords.
List<int> _getGenerator(int nEcc) {
  return _genCache.putIfAbsent(nEcc, () => _buildGenerator(nEcc));
}

// ============================================================================
// Reed-Solomon encoding
// ============================================================================

/// Compute ECC codewords for one data block using LFSR polynomial division.
///
/// Computes R(x) = D(x) · x^n_ecc mod G(x) over GF(256)/0x12D.
///
/// The LFSR (shift-register) approach:
///   For each input byte d:
///     feedback = d XOR rem[0]
///     shift rem left (drop rem[0], append 0)
///     for each position i: rem[i] ^= gen[i+1] × feedback
///
/// After processing all data bytes, rem holds the ECC codewords.
/// This is the standard systematic RS encoder used by ISO 16022.
List<int> _rsEncodeBlock(List<int> data, List<int> generator) {
  final nEcc = generator.length - 1;
  final rem = List<int>.filled(nEcc, 0);

  for (final d in data) {
    final fb = d ^ rem[0];
    // Shift register left: rem[0] is consumed, rem[n-1] gets 0.
    for (var i = 0; i < nEcc - 1; i++) {
      rem[i] = rem[i + 1];
    }
    rem[nEcc - 1] = 0;
    // XOR in generator × feedback if feedback is non-zero.
    if (fb != 0) {
      for (var i = 0; i < nEcc; i++) {
        rem[i] ^= _gfMul(generator[i + 1], fb);
      }
    }
  }

  return rem;
}

// ============================================================================
// ASCII data encoding
// ============================================================================

/// Encode input bytes in Data Matrix ASCII mode.
///
/// ASCII mode rules (ISO/IEC 16022:2006 §5.2.4):
///
/// 1. **Digit pair**: two consecutive ASCII digit bytes (0x30–0x39) are
///    packed into a single codeword = 130 + (d1 × 10 + d2).
///    Example: "12" → codeword 142 (130 + 12). This halves the codeword
///    budget for numeric content.
///
/// 2. **Single ASCII char** (0–127): one codeword = ASCII value + 1.
///    The +1 shift reserves codeword 0 as "end of data".
///    Example: 'A' (65) → 66; space (32) → 33.
///
/// 3. **Extended ASCII** (128–255): two codewords: UPPER_SHIFT (235) then
///    value − 127. Enables Latin-1 characters, though rare in practice.
///
/// | Input   | Codewords    | Why                        |
/// |---------|--------------|----------------------------|
/// | "A"     | [66]         | 65 + 1                     |
/// | " "     | [33]         | 32 + 1                     |
/// | "12"    | [142]        | 130 + 12 (digit pair)      |
/// | "1234"  | [142, 174]   | two digit pairs             |
/// | "00"    | [130]        | 130 + 0                    |
/// | "99"    | [229]        | 130 + 99                   |
List<int> _encodeAscii(List<int> inputBytes) {
  final codewords = <int>[];
  var i = 0;
  final n = inputBytes.length;

  while (i < n) {
    final c = inputBytes[i];
    final isDigit = c >= 0x30 && c <= 0x39;
    final nextIsDigit = i + 1 < n && inputBytes[i + 1] >= 0x30 && inputBytes[i + 1] <= 0x39;

    if (isDigit && nextIsDigit) {
      // Digit pair: pack two digits into one codeword.
      final d1 = c - 0x30;
      final d2 = inputBytes[i + 1] - 0x30;
      codewords.add(130 + d1 * 10 + d2);
      i += 2;
    } else if (c <= 127) {
      // Standard single ASCII character.
      codewords.add(c + 1);
      i += 1;
    } else {
      // Extended ASCII: UPPER_SHIFT + (value - 127).
      codewords.add(235); // UPPER_SHIFT
      codewords.add(c - 127);
      i += 1;
    }
  }

  return codewords;
}

// ============================================================================
// Pad codewords (ISO/IEC 16022:2006 §5.2.3)
// ============================================================================

/// Pad [codewords] to exactly [dataCw] bytes using the ECC200 scrambling rule.
///
/// Padding rules:
///
/// 1. The first pad codeword is always the literal value 129 (EOM —
///    "End of Message").
///
/// 2. Subsequent pad codewords use a scrambled value to prevent a long run of
///    identical bytes from creating degenerate Utah placement patterns:
///
///    ```
///    k = 1-indexed position within the full codeword stream
///    scrambled = 129 + (149 × k mod 253) + 1
///    if scrambled > 254: scrambled -= 254
///    ```
///
/// Example — encoding "A" (codewords [66]) into 10×10 (dataCw = 3):
///   Position 2 (k=2): 129 (first pad — always literal)
///   Position 3 (k=3): 129 + (149×3 mod 253) + 1 = 129 + 194 + 1 = 324 → 70
///   Result: [66, 129, 70]
List<int> _padCodewords(List<int> codewords, int dataCw) {
  final padded = List<int>.from(codewords);
  var isFirst = true;
  var k = codewords.length + 1; // 1-indexed position of first pad byte

  while (padded.length < dataCw) {
    if (isFirst) {
      padded.add(129);
      isFirst = false;
    } else {
      var scrambled = 129 + (149 * k) % 253 + 1;
      if (scrambled > 254) scrambled -= 254;
      padded.add(scrambled);
    }
    k++;
  }

  return padded;
}

// ============================================================================
// Symbol selection
// ============================================================================

/// Find the smallest symbol entry that can hold [codewordCount] codewords.
///
/// Candidates are filtered by [shape], then sorted by capacity ascending
/// (ties broken by total module area). Returns the first symbol whose
/// dataCw >= codewordCount.
///
/// Throws [InputTooLongError] if no symbol is large enough.
_SymbolEntry _selectSymbol(int codewordCount, SymbolShape shape) {
  List<_SymbolEntry> candidates;
  switch (shape) {
    case SymbolShape.square:
      candidates = List<_SymbolEntry>.from(_squareSizes);
    case SymbolShape.rectangle:
      candidates = List<_SymbolEntry>.from(_rectSizes);
    case SymbolShape.any:
      candidates = [..._squareSizes, ..._rectSizes];
  }

  // Sort by capacity ascending; ties broken by area (smaller symbol first).
  candidates.sort((a, b) {
    final byCap = a.dataCw.compareTo(b.dataCw);
    if (byCap != 0) return byCap;
    return (a.symbolRows * a.symbolCols).compareTo(b.symbolRows * b.symbolCols);
  });

  for (final e in candidates) {
    if (e.dataCw >= codewordCount) return e;
  }

  throw InputTooLongError(
    'data-matrix: input too long — encoded $codewordCount codewords, '
    'maximum is $_maxDataCw (144×144 symbol).',
  );
}

/// Find a symbol entry matching [dimension] (used as the rows value for square,
/// or the rows value when searching all sizes).
///
/// Throws [InvalidSizeError] if no ECC200 symbol matches.
_SymbolEntry _findEntryBySize(int dimension) {
  // Check squares first: the dimension is the side length.
  for (final e in _squareSizes) {
    if (e.symbolRows == dimension) return e;
  }
  // Check rectangles: the dimension could match either rows.
  for (final e in _rectSizes) {
    if (e.symbolRows == dimension) return e;
  }
  throw InvalidSizeError(
    'data-matrix: $dimension is not a valid ECC200 symbol dimension. '
    'Valid square sizes: 10, 12, 14, 16, 18, 20, 22, 24, 26, 32, 36, 40, '
    '44, 48, 52, 64, 72, 80, 88, 96, 104, 120, 132, 144. '
    'Valid rectangular rows: 8 (8×18, 8×32), 12 (12×26, 12×36), 16 (16×36, 16×48).',
  );
}

// ============================================================================
// Block splitting, ECC computation, and interleaving
// ============================================================================

/// Split data, compute RS ECC per block, and interleave all blocks round-robin.
///
/// Block splitting — ISO 16022 convention:
///   base_len     = dataCw ÷ numBlocks   (integer division)
///   extra_blocks = dataCw mod numBlocks
///   Blocks 0..extra_blocks-1   get base_len + 1 data codewords.
///   Blocks extra_blocks..end-1 get base_len     data codewords.
///
/// Interleaving distributes burst errors across blocks. A scratch that
/// destroys N consecutive modules damages at most ⌈N/numBlocks⌉ codewords
/// per block — much more likely to be within each block's correction capacity.
///
/// Output order:
///   data[0][0], data[1][0], …, data[B-1][0],
///   data[0][1], data[1][1], …,
///   …
///   ecc[0][0], ecc[1][0], …, ecc[B-1][0],
///   ecc[0][1], …
List<int> _computeInterleaved(List<int> data, _SymbolEntry entry) {
  final numBlocks = entry.numBlocks;
  final eccPerBlock = entry.eccPerBlock;
  final dataCw = entry.dataCw;
  final gen = _getGenerator(eccPerBlock);

  // ── Split data into blocks ─────────────────────────────────────────────────
  final baseLen = dataCw ~/ numBlocks;
  final extraBlocks = dataCw % numBlocks;

  final dataBlocks = <List<int>>[];
  var offset = 0;
  for (var b = 0; b < numBlocks; b++) {
    final len = b < extraBlocks ? baseLen + 1 : baseLen;
    dataBlocks.add(data.sublist(offset, offset + len));
    offset += len;
  }

  // ── Compute ECC for each block ─────────────────────────────────────────────
  final eccBlocks = dataBlocks.map((blk) => _rsEncodeBlock(blk, gen)).toList();

  // ── Interleave data round-robin ────────────────────────────────────────────
  final interleaved = <int>[];
  final maxDataLen = dataBlocks.fold(0, (m, blk) => blk.length > m ? blk.length : m);
  for (var pos = 0; pos < maxDataLen; pos++) {
    for (var b = 0; b < numBlocks; b++) {
      if (pos < dataBlocks[b].length) {
        interleaved.add(dataBlocks[b][pos]);
      }
    }
  }

  // ── Interleave ECC round-robin ─────────────────────────────────────────────
  for (var pos = 0; pos < eccPerBlock; pos++) {
    for (var b = 0; b < numBlocks; b++) {
      interleaved.add(eccBlocks[b][pos]);
    }
  }

  return interleaved;
}

// ============================================================================
// Grid initialization (border + alignment borders)
// ============================================================================

/// Allocate the physical grid and draw all fixed structural elements.
///
/// Outer "finder + clock" border
/// ------------------------------
/// - **Left column** (col 0): all dark — the vertical leg of the L-finder.
/// - **Bottom row** (row R−1): all dark — the horizontal leg of the L-finder.
/// - **Top row** (row 0): alternating dark/light starting dark at col 0 —
///   the timing clock for the top edge.
/// - **Right column** (col C−1): alternating dark/light starting dark at row 0 —
///   the timing clock for the right edge.
///
/// Writing order: alignment borders FIRST, then top + right timing, then
/// left col, then bottom row LAST. The L-finder bottom row always wins at
/// intersections because it is drawn last.
///
/// Alignment borders (multi-region symbols)
/// -----------------------------------------
/// Symbols with regionRows × regionCols > 1 have 2-module alignment borders
/// separating adjacent data regions:
///   - First row/col of the border: all dark.
///   - Second row/col: alternating (starts dark at col/row 0).
List<List<bool>> _initGrid(_SymbolEntry entry) {
  final R = entry.symbolRows;
  final C = entry.symbolCols;

  final grid = List.generate(R, (_) => List<bool>.filled(C, false));

  // ── Alignment borders (multi-region symbols only) ──────────────────────────
  // Written FIRST so the outer border can override at intersections.
  for (var rr = 0; rr < entry.regionRows - 1; rr++) {
    // Physical row of the first alignment border row after data region rr+1.
    // 1 (outer border) + (rr+1) × dataRegionHeight + rr × 2 (prev ABs).
    final abRow0 = 1 + (rr + 1) * entry.dataRegionHeight + rr * 2;
    final abRow1 = abRow0 + 1;
    for (var c = 0; c < C; c++) {
      grid[abRow0][c] = true;             // all dark
      grid[abRow1][c] = (c % 2 == 0);    // alternating, starts dark
    }
  }

  for (var rc = 0; rc < entry.regionCols - 1; rc++) {
    final abCol0 = 1 + (rc + 1) * entry.dataRegionWidth + rc * 2;
    final abCol1 = abCol0 + 1;
    for (var r = 0; r < R; r++) {
      grid[r][abCol0] = true;             // all dark
      grid[r][abCol1] = (r % 2 == 0);    // alternating, starts dark
    }
  }

  // ── Top row: timing clock — alternating dark/light starting dark ───────────
  for (var c = 0; c < C; c++) {
    grid[0][c] = (c % 2 == 0);
  }

  // ── Right column: timing clock — alternating, starts dark ─────────────────
  for (var r = 0; r < R; r++) {
    grid[r][C - 1] = (r % 2 == 0);
  }

  // ── Left column: L-finder left leg — all dark ─────────────────────────────
  // Written after timing to override the timing value at (0, 0).
  for (var r = 0; r < R; r++) {
    grid[r][0] = true;
  }

  // ── Bottom row: L-finder bottom leg — all dark ────────────────────────────
  // Written LAST so it wins at any intersection (alignment borders, right col).
  for (var c = 0; c < C; c++) {
    grid[R - 1][c] = true;
  }

  return grid;
}

// ============================================================================
// Utah placement algorithm
// ============================================================================
//
// The "Utah" algorithm is Data Matrix's most distinctive feature. Its name
// comes from the 8-module codeword shape, which resembles the US state of
// Utah — a rectangle with a notch cut from the top-left corner:
//
//                col-2  col-1   col
//
//        row-2 :   .   [bit1]  [bit2]
//        row-1 : [bit3] [bit4] [bit5]
//        row   : [bit6] [bit7] [bit8]
//
// The algorithm scans the LOGICAL grid (all data region interiors concatenated)
// in a diagonal zigzag. It alternates upward-right and downward-left legs.
// Special corner patterns handle positions near the boundary.
//
// There is NO masking step — the diagonal traversal naturally distributes bits
// across the symbol without the clustering that QR Code's masking addresses.

/// Apply the boundary wrap rules from ISO/IEC 16022:2006 Annex F.
///
/// When the standard Utah shape extends beyond the logical grid edge, fold
/// coordinates back into range. The four rules are applied in order:
///
/// 1. row < 0 AND col == 0          → (1, 3)          top-left singularity
/// 2. row < 0 AND col == nCols      → (0, col-2)      wrapped past right
/// 3. row < 0                        → (row+nRows, col-4) wrap top→bottom
/// 4. col < 0                        → (row-4, col+nCols) wrap left→right
(int, int) _applyWrap(int row, int col, int nRows, int nCols) {
  if (row < 0 && col == 0) return (1, 3);
  if (row < 0 && col == nCols) return (0, col - 2);
  if (row < 0) return (row + nRows, col - 4);
  if (col < 0) return (row - 4, col + nCols);
  return (row, col);
}

/// Place one codeword using the standard "Utah" 8-module pattern.
///
/// Bit 8 (MSB) goes to (row, col), bit 1 (LSB) to (row-2, col-1).
/// Out-of-bounds positions are wrapped using [_applyWrap]. Already-used
/// positions are silently skipped (prevents overwriting fixed modules).
void _placeUtah(
  int cw,
  int row,
  int col,
  int nRows,
  int nCols,
  List<List<bool>> grid,
  List<List<bool>> used,
) {
  // (rawRow, rawCol, bitShift) where bitShift 7 = MSB, 0 = LSB.
  final placements = [
    (row,     col,     7), // bit 8 (MSB)
    (row,     col - 1, 6), // bit 7
    (row,     col - 2, 5), // bit 6
    (row - 1, col,     4), // bit 5
    (row - 1, col - 1, 3), // bit 4
    (row - 1, col - 2, 2), // bit 3
    (row - 2, col,     1), // bit 2
    (row - 2, col - 1, 0), // bit 1 (LSB)
  ];

  for (final (rawR, rawC, bit) in placements) {
    final (r, c) = _applyWrap(rawR, rawC, nRows, nCols);
    if (r >= 0 && r < nRows && c >= 0 && c < nCols && !used[r][c]) {
      grid[r][c] = ((cw >> bit) & 1) == 1;
      used[r][c] = true;
    }
  }
}

/// Place a codeword at explicitly specified (row, col, bit) positions.
///
/// Used by the four corner patterns which cannot use the standard Utah shape.
void _placeWithPositions(
  int cw,
  List<(int, int, int)> positions,
  int nRows,
  int nCols,
  List<List<bool>> grid,
  List<List<bool>> used,
) {
  for (final (r, c, bit) in positions) {
    if (r >= 0 && r < nRows && c >= 0 && c < nCols && !used[r][c]) {
      grid[r][c] = ((cw >> bit) & 1) == 1;
      used[r][c] = true;
    }
  }
}

/// Corner pattern 1 — triggered at the top-left boundary.
///
/// Fires when row == nRows AND col == 0 AND (nRows mod 4 == 0 OR nCols mod 4 == 0).
void _placeCorner1(int cw, int nRows, int nCols, List<List<bool>> grid, List<List<bool>> used) {
  _placeWithPositions(cw, [
    (0,          nCols - 2, 7),
    (0,          nCols - 1, 6),
    (1,          0,         5),
    (2,          0,         4),
    (nRows - 2,  0,         3),
    (nRows - 1,  0,         2),
    (nRows - 1,  1,         1),
    (nRows - 1,  2,         0),
  ], nRows, nCols, grid, used);
}

/// Corner pattern 2 — triggered at the top-right boundary.
///
/// Fires when row == nRows-2 AND col == 0 AND nCols mod 4 != 0.
void _placeCorner2(int cw, int nRows, int nCols, List<List<bool>> grid, List<List<bool>> used) {
  _placeWithPositions(cw, [
    (0,          nCols - 2, 7),
    (0,          nCols - 1, 6),
    (1,          nCols - 1, 5),
    (2,          nCols - 1, 4),
    (nRows - 1,  0,         3),
    (nRows - 1,  1,         2),
    (nRows - 1,  2,         1),
    (nRows - 1,  3,         0),
  ], nRows, nCols, grid, used);
}

/// Corner pattern 3 — triggered at the bottom-left boundary.
///
/// Fires when row == nRows-2 AND col == 0 AND nCols mod 8 == 4.
void _placeCorner3(int cw, int nRows, int nCols, List<List<bool>> grid, List<List<bool>> used) {
  _placeWithPositions(cw, [
    (0,          nCols - 1, 7),
    (1,          0,         6),
    (2,          0,         5),
    (nRows - 2,  0,         4),
    (nRows - 1,  0,         3),
    (nRows - 1,  1,         2),
    (nRows - 1,  2,         1),
    (nRows - 1,  3,         0),
  ], nRows, nCols, grid, used);
}

/// Corner pattern 4 — triggered for nCols mod 8 == 0.
///
/// Fires when row == nRows+4 AND col == 2 AND nCols mod 8 == 0.
void _placeCorner4(int cw, int nRows, int nCols, List<List<bool>> grid, List<List<bool>> used) {
  _placeWithPositions(cw, [
    (nRows - 3, nCols - 1, 7),
    (nRows - 2, nCols - 1, 6),
    (nRows - 1, nCols - 3, 5),
    (nRows - 1, nCols - 2, 4),
    (nRows - 1, nCols - 1, 3),
    (0,         0,         2),
    (1,         0,         1),
    (2,         0,         0),
  ], nRows, nCols, grid, used);
}

/// Run the Utah diagonal placement algorithm on the logical data matrix.
///
/// The reference position (row, col) starts at (4, 0) and zigzags diagonally.
/// Each outer loop iteration has two legs:
///
/// 1. **Upward-right leg**: place a codeword at (row, col), then move
///    row -= 2, col += 2, repeat until out of bounds. Then step:
///    row += 1, col += 3.
///
/// 2. **Downward-left leg**: place a codeword at (row, col), then move
///    row += 2, col -= 2, repeat until out of bounds. Then step:
///    row += 3, col += 1.
///
/// Corner patterns fire before each pair of legs at specific trigger positions.
///
/// Termination: when both row >= nRows AND col >= nCols, or all codewords are
/// placed. Any unvisited modules receive the ISO fill pattern:
/// (r + c) mod 2 == 1 (dark).
List<List<bool>> _utahPlacement(List<int> codewords, int nRows, int nCols) {
  final grid = List.generate(nRows, (_) => List<bool>.filled(nCols, false));
  final used = List.generate(nRows, (_) => List<bool>.filled(nCols, false));

  var cwIdx = 0;
  var row = 4;
  var col = 0;

  while (true) {
    // ── Corner special cases ───────────────────────────────────────────────
    if (row == nRows && col == 0 && (nRows % 4 == 0 || nCols % 4 == 0)) {
      if (cwIdx < codewords.length) {
        _placeCorner1(codewords[cwIdx], nRows, nCols, grid, used);
        cwIdx++;
      }
    }
    if (row == nRows - 2 && col == 0 && nCols % 4 != 0) {
      if (cwIdx < codewords.length) {
        _placeCorner2(codewords[cwIdx], nRows, nCols, grid, used);
        cwIdx++;
      }
    }
    if (row == nRows - 2 && col == 0 && nCols % 8 == 4) {
      if (cwIdx < codewords.length) {
        _placeCorner3(codewords[cwIdx], nRows, nCols, grid, used);
        cwIdx++;
      }
    }
    if (row == nRows + 4 && col == 2 && nCols % 8 == 0) {
      if (cwIdx < codewords.length) {
        _placeCorner4(codewords[cwIdx], nRows, nCols, grid, used);
        cwIdx++;
      }
    }

    // ── Upward-right diagonal leg (row -= 2, col += 2) ────────────────────
    while (true) {
      if (row >= 0 && row < nRows && col >= 0 && col < nCols &&
          !used[row][col] && cwIdx < codewords.length) {
        _placeUtah(codewords[cwIdx], row, col, nRows, nCols, grid, used);
        cwIdx++;
      }
      row -= 2;
      col += 2;
      if (row < 0 || col >= nCols) break;
    }

    // Step to next diagonal start.
    row += 1;
    col += 3;

    // ── Downward-left diagonal leg (row += 2, col -= 2) ───────────────────
    while (true) {
      if (row >= 0 && row < nRows && col >= 0 && col < nCols &&
          !used[row][col] && cwIdx < codewords.length) {
        _placeUtah(codewords[cwIdx], row, col, nRows, nCols, grid, used);
        cwIdx++;
      }
      row += 2;
      col -= 2;
      if (row >= nRows || col < 0) break;
    }

    // Step to next diagonal start.
    row += 3;
    col += 1;

    // ── Termination ────────────────────────────────────────────────────────
    if (row >= nRows && col >= nCols) break;
    if (cwIdx >= codewords.length) break;
  }

  // ── Fill remaining unvisited modules ──────────────────────────────────────
  // ISO/IEC 16022 §10: residual modules get (r+c) mod 2 == 1 (dark).
  for (var r = 0; r < nRows; r++) {
    for (var c = 0; c < nCols; c++) {
      if (!used[r][c]) {
        grid[r][c] = (r + c) % 2 == 1;
      }
    }
  }

  return grid;
}

// ============================================================================
// Logical → physical coordinate mapping
// ============================================================================

/// Map a logical data-matrix coordinate to its physical symbol coordinate.
///
/// The logical data matrix is the concatenation of all data region interiors
/// treated as one flat grid. Utah placement works in logical space. After
/// placement we map back to the physical grid, which adds:
///
/// - 1-module outer border (finder + timing) on all four sides.
/// - 2-module alignment border between adjacent data regions.
///
/// For region interior size (rh × rw):
///   phys_row = ⌊r / rh⌋ × (rh + 2) + (r mod rh) + 1
///   phys_col = ⌊c / rw⌋ × (rw + 2) + (c mod rw) + 1
///
/// For single-region symbols: phys_row = r + 1, phys_col = c + 1.
(int, int) _logicalToPhysical(int r, int c, _SymbolEntry entry) {
  final rh = entry.dataRegionHeight;
  final rw = entry.dataRegionWidth;
  final physRow = (r ~/ rh) * (rh + 2) + (r % rh) + 1;
  final physCol = (c ~/ rw) * (rw + 2) + (c % rw) + 1;
  return (physRow, physCol);
}

// ============================================================================
// Public API — encode
// ============================================================================

/// Encode [data] into a Data Matrix ECC200 [ModuleGrid].
///
/// ## Auto-selection (default)
///
/// When [options.size] is null, the encoder automatically selects the smallest
/// symbol that can hold the encoded data. The shape preference in [options.shape]
/// controls which symbol families are considered.
///
/// ## Forced size
///
/// Set [options.size] to force a specific symbol dimension (e.g. 18 for an 18×18
/// square symbol). Throws [InvalidSizeError] if the dimension is not a valid
/// ECC200 size. Throws [InputTooLongError] if the input does not fit.
///
/// ## Encoding pipeline
///
/// 1. ASCII-encode the input (UTF-8 bytes; digit-pairs packed).
/// 2. Select symbol (auto or forced).
/// 3. Pad to data capacity with ECC200 scrambled-pad sequence.
/// 4. Compute Reed-Solomon ECC per block (GF(256)/0x12D, b=1).
/// 5. Interleave data + ECC blocks round-robin.
/// 6. Initialize physical grid (finder + timing + alignment borders).
/// 7. Run Utah diagonal placement on the logical data matrix.
/// 8. Map logical → physical coordinates.
/// 9. Return immutable [ModuleGrid].
///
/// ## Errors
///
/// - [InputTooLongError] — input exceeds the largest fitting symbol.
/// - [InvalidSizeError] — forced size does not match any ECC200 symbol.
///
/// ## Example
///
/// ```dart
/// // Auto-select: "A" → 10×10 (smallest square symbol)
/// final grid = encode('A');
/// print(grid.rows);  // 10
///
/// // Force 18×18:
/// final big = encode('HELLO', options: DataMatrixOptions(size: 18));
/// print(big.rows);   // 18
/// ```
ModuleGrid encode(
  String data, {
  DataMatrixOptions options = const DataMatrixOptions(),
}) {
  // Step 1: ASCII-encode (uses UTF-8 bytes for non-ASCII characters).
  final inputBytes = data.codeUnits; // UTF-16 code units, works for ASCII range
  final codewords = _encodeAscii(inputBytes);

  // Step 2: Select symbol — explicit size or auto-pick smallest.
  late final _SymbolEntry entry;
  if (options.size != null) {
    entry = _findEntryBySize(options.size!);
    if (codewords.length > entry.dataCw) {
      throw InputTooLongError(
        'data-matrix: input encodes to ${codewords.length} codewords '
        'but ${entry.symbolRows}×${entry.symbolCols} symbol holds '
        'only ${entry.dataCw}.',
      );
    }
  } else {
    entry = _selectSymbol(codewords.length, options.shape);
  }

  // Step 3: Pad to data capacity.
  final padded = _padCodewords(codewords, entry.dataCw);

  // Steps 4–5: Compute ECC and interleave all blocks.
  final interleaved = _computeInterleaved(padded, entry);

  // Step 6: Initialize physical grid with finder + timing + alignment borders.
  final physGrid = _initGrid(entry);

  // Step 7: Run Utah placement on the logical data matrix.
  final nRows = entry.regionRows * entry.dataRegionHeight;
  final nCols = entry.regionCols * entry.dataRegionWidth;
  final logicalGrid = _utahPlacement(interleaved, nRows, nCols);

  // Step 8: Map logical → physical coordinates.
  for (var r = 0; r < nRows; r++) {
    for (var c = 0; c < nCols; c++) {
      final (pr, pc) = _logicalToPhysical(r, c, entry);
      physGrid[pr][pc] = logicalGrid[r][c];
    }
  }

  // Step 9: Build immutable ModuleGrid from physGrid.
  //
  // We use makeModuleGrid + setModule from barcode-2d to build the grid
  // immutably, but for performance we construct it in one shot by building
  // the modules list directly.
  final modules = List<List<bool>>.generate(
    entry.symbolRows,
    (r) => List<bool>.from(physGrid[r]),
    growable: false,
  );

  return ModuleGrid(
    rows: entry.symbolRows,
    cols: entry.symbolCols,
    modules: modules,
    moduleShape: ModuleShape.square,
  );
}

// ============================================================================
// Public API — layout helpers
// ============================================================================

/// Convert a [ModuleGrid] to a [PaintScene] via barcode-2d's [layout] function.
///
/// Defaults to a 1-module quiet zone (Data Matrix minimum — the L-finder is
/// inherently self-delimiting, so a narrower quiet zone is acceptable compared
/// to QR Code's required 4-module zone).
///
/// Pass [config] to override any layout settings.
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
        quietZoneModules: 1, // Data Matrix minimum (not QR's 4)
        foreground: '#000000',
        background: '#ffffff',
        moduleShape: ModuleShape.square,
      );
  return layout(grid, config: cfg);
}

/// Encode [data] and immediately convert to a [PaintScene].
///
/// Convenience wrapper: calls [encode] then [layoutGrid] in one step.
///
/// ## Example
///
/// ```dart
/// final scene = encodeAndLayout('HELLO');
/// ```
PaintScene encodeAndLayout(
  String data, {
  DataMatrixOptions options = const DataMatrixOptions(),
  Barcode2DLayoutConfig? config,
}) {
  final grid = encode(data, options: options);
  return layoutGrid(grid, config: config);
}

// ============================================================================
// Debug utility
// ============================================================================

/// Render a [ModuleGrid] as a multi-line '0'/'1' string for debugging.
///
/// Each row is one line (no trailing newline). Used for snapshot comparison
/// and cross-language corpus verification: all Data Matrix implementations
/// must produce bit-for-bit identical output for the same input.
///
/// ## Example
///
/// ```dart
/// final grid = encode('A');
/// print(gridToString(grid));
/// // 1010101010
/// // 1000001001
/// // ...
/// ```
String gridToString(ModuleGrid grid) {
  return grid.modules
      .map((row) => row.map((d) => d ? '1' : '0').join())
      .join('\n');
}
