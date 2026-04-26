/// # pdf417 — PDF417 stacked linear barcode encoder
///
/// PDF417 (Portable Data File 417) was invented by Ynjiun P. Wang at Symbol
/// Technologies in 1991. The name encodes the format's geometry: each codeword
/// has exactly **4** bars and **4** spaces (8 elements total), and every
/// codeword occupies exactly **17** modules of horizontal space.
///
/// ## Where PDF417 is deployed
///
/// | Application  | Detail                                              |
/// |--------------|-----------------------------------------------------|
/// | AAMVA        | North American driver's licences and government IDs |
/// | IATA BCBP    | Airline boarding passes                             |
/// | USPS         | Domestic shipping labels                            |
/// | US immigration | Form I-94, customs declarations                   |
/// | Healthcare   | Patient wristbands, medication labels               |
///
/// ## Architecture overview
///
/// This file contains everything needed to encode arbitrary bytes into a
/// scannable PDF417 `ModuleGrid`. The encoding pipeline:
///
/// ```
/// raw bytes
///   → byte compaction     (codeword 924 latch + 6-bytes→5-codewords, base-900)
///   → length descriptor   (codeword[0] = total codewords in symbol)
///   → RS ECC              (GF(929) Reed-Solomon, b=3 convention, α=3)
///   → dimension selection (auto: roughly square symbol)
///   → padding             (codeword 900 fills unused slots)
///   → row indicators      (LRI + RRI per row, encode R/C/ECC level)
///   → cluster table lookup (codeword → 17-module bar/space pattern)
///   → start/stop patterns (fixed per row)
///   → ModuleGrid          (abstract boolean grid)
/// ```
///
/// ## v0.1.0 scope
///
/// This release implements **byte compaction only**. All input bytes are
/// compacted using codeword 924 (latch to byte compaction). Text compaction
/// and numeric compaction are planned for v0.2.0.
library pdf417;

import 'dart:convert' show utf8;
import 'dart:math' as math;

import 'package:coding_adventures_barcode_2d/coding_adventures_barcode_2d.dart';

import 'cluster_tables.dart';

// ============================================================================
// Package version
// ============================================================================

/// Package version following Semantic Versioning 2.0.
const String pdf417Version = '0.1.0';

// ============================================================================
// Constants
// ============================================================================

/// GF(929) prime modulus. PDF417 Reed-Solomon operates over the integers
/// modulo 929. Since 929 is prime, every non-zero element has an inverse.
const int _gf929Prime = 929;

/// Generator element α = 3 (primitive root mod 929).
///
/// The powers 3^0, 3^1, ..., 3^927 cycle through all 928 non-zero elements
/// of GF(929) before repeating. This is specified in ISO/IEC 15438:2015 §A.4.
const int _gf929Alpha = 3;

/// Multiplicative group order = prime − 1 = 928.
const int _gf929Order = 928;

/// Latch-to-byte-compaction codeword (alternate form, usable for any length).
///
/// Emitted before a byte compaction segment. 6 bytes → 5 base-900 codewords;
/// remaining 1–5 bytes → 1 codeword each.
const int _latchByte = 924;

/// Padding codeword. Fills unused slots to complete the r×c grid.
///
/// Value 900 is the latch-to-text codeword, which is a neutral filler:
/// a scanner that encounters it after data will switch to text mode and
/// produce no output until actual character codewords follow.
const int _paddingCw = 900;

/// Minimum valid number of rows in a PDF417 symbol (ISO/IEC 15438:2015 §5).
const int _minRows = 3;

/// Maximum valid number of rows in a PDF417 symbol (ISO/IEC 15438:2015 §5).
const int _maxRows = 90;

/// Minimum valid number of data columns (ISO/IEC 15438:2015 §5).
const int _minCols = 1;

/// Maximum valid number of data columns (ISO/IEC 15438:2015 §5).
const int _maxCols = 30;

// ============================================================================
// Error types
// ============================================================================

/// Base class for all PDF417 encoding errors.
///
/// Catch this type to handle any error from this library without caring about
/// the specific subtype.
final class Pdf417Error implements Exception {
  /// Human-readable description of the error.
  final String message;

  const Pdf417Error(this.message);

  @override
  String toString() => 'Pdf417Error: $message';
}

/// Thrown when the input data is too long to fit in any valid PDF417 symbol.
///
/// The maximum capacity of a PDF417 symbol is 90 rows × 30 columns = 2700
/// data+ECC slots. Subtracting minimum ECC (level 2 = 8 codewords) and the
/// length descriptor leaves 2691 data codewords. Each byte takes at most
/// 1 codeword (direct mapping) after the 924 latch, so the practical maximum
/// is around 2690 bytes.
final class InputTooLongError extends Pdf417Error {
  const InputTooLongError(super.message);

  @override
  String toString() => 'InputTooLongError: $message';
}

/// Thrown when the user-supplied rows or columns are outside valid ranges.
///
/// Valid ranges: 3–90 rows, 1–30 columns.
final class InvalidDimensionsError extends Pdf417Error {
  const InvalidDimensionsError(super.message);

  @override
  String toString() => 'InvalidDimensionsError: $message';
}

/// Thrown when the specified ECC level is not in the range 0–8.
final class InvalidEccLevelError extends Pdf417Error {
  const InvalidEccLevelError(super.message);

  @override
  String toString() => 'InvalidEccLevelError: $message';
}

// ============================================================================
// PDF417Options
// ============================================================================

/// Options controlling how the PDF417 symbol is encoded.
final class Pdf417Options {
  /// Reed-Solomon error correction level (0–8).
  ///
  /// Higher levels use more ECC codewords, reducing data capacity but
  /// increasing resilience to damage.
  ///
  /// | Level | ECC codewords | Corrects errors |
  /// |-------|--------------|-----------------|
  /// |   0   |      2       |   detects only  |
  /// |   1   |      4       |       1         |
  /// |   2   |      8       |       3         |
  /// |   3   |     16       |       7         |
  /// |   4   |     32       |      15         |
  /// |   5   |     64       |      31         |
  /// |   6   |    128       |      63         |
  /// |   7   |    256       |     127         |
  /// |   8   |    512       |     255         |
  ///
  /// `null` → auto-selected based on data length (see [_autoEccLevel]).
  final int? eccLevel;

  /// Number of data columns (1–30).
  ///
  /// `null` → auto-selected to produce a roughly square symbol.
  final int? columns;

  /// Module-rows per logical PDF417 row (1–10).
  ///
  /// PDF417 rows are "tall" relative to their width (the 17-module codeword
  /// width is wide). Using `rowHeight >= 3` ensures scanners that need a
  /// minimum vertical extent per row can read the symbol reliably.
  ///
  /// Default: 3.
  final int rowHeight;

  const Pdf417Options({
    this.eccLevel,
    this.columns,
    this.rowHeight = 3,
  });
}

// ============================================================================
// GF(929) arithmetic
// ============================================================================
//
// GF(929) is the integers modulo 929. Since 929 is prime, every non-zero
// element has a multiplicative inverse, making it a field. We build log and
// antilog (exp) tables once at module load time for O(1) multiplication.
//
// Table sizes:
//   _gfExp: 929 ints  (the 928-element cycle plus one wrap entry)
//   _gfLog: 929 ints  (log[0] is unused / undefined)
// Total: ~3.7 KB — negligible startup cost.

/// GF(929) antilog table: _gfExp[i] = α^i mod 929.
final List<int> _gfExp = _buildGfExp();

/// GF(929) log table: _gfLog[v] = i such that α^i ≡ v (mod 929).
final List<int> _gfLog = _buildGfLog(_gfExp);

/// Build the antilog (exp) table.
///
/// We compute successive powers of α = 3 modulo 929, cycling through all 928
/// non-zero elements before returning to 1. The extra entry at index 928 is a
/// copy of index 0 for wrap-around convenience in [_gfMul].
List<int> _buildGfExp() {
  final exp = List<int>.filled(929, 0);
  var val = 1;
  for (var i = 0; i < _gf929Order; i++) {
    exp[i] = val;
    val = (val * _gf929Alpha) % _gf929Prime;
  }
  // Wrap-around copy: exp[928] = exp[0] = 1.
  exp[928] = exp[0];
  return exp;
}

/// Build the discrete logarithm table from the antilog table.
List<int> _buildGfLog(List<int> exp) {
  final log = List<int>.filled(929, 0);
  for (var i = 0; i < _gf929Order; i++) {
    log[exp[i]] = i;
  }
  return log;
}

/// GF(929) multiplication using log/antilog tables.
///
/// Uses the identity: `a × b = α^(log(a) + log(b))`.
/// Returns 0 if either operand is 0 (since 0 has no log).
///
/// ```dart
/// _gfMul(3, 310) // → 1   (3 and 310 are inverses mod 929)
/// _gfMul(0, 100) // → 0
/// ```
int _gfMul(int a, int b) {
  if (a == 0 || b == 0) return 0;
  return _gfExp[(_gfLog[a] + _gfLog[b]) % _gf929Order];
}

/// GF(929) addition: `(a + b) mod 929`.
///
/// In a prime field, addition is just modular addition — no XOR like in GF(2^k).
int _gfAdd(int a, int b) => (a + b) % _gf929Prime;

// ============================================================================
// Reed-Solomon generator polynomial
// ============================================================================
//
// PDF417 uses the b=3 convention: the generator polynomial for ECC level L
// has roots α^3, α^4, ..., α^(k+2) where k = 2^(L+1) is the number of ECC
// codewords.
//
//   g(x) = (x − α^3)(x − α^4) ··· (x − α^(k+2))
//
// This is distinct from QR Code (b=0) and Data Matrix / Aztec (b=1 or b=0).
// The b=3 offset is specified in ISO/IEC 15438:2015 §A.4.
//
// We build the polynomial iteratively: start with g = [1], then multiply in
// each linear factor (x − α^j) one at a time. Each multiplication shifts all
// coefficients left (multiply by x) and subtracts α^j × each coefficient.

/// Build the RS generator polynomial for ECC level [eccLevel].
///
/// Returns k+1 coefficients [g_k, g_{k-1}, ..., g_1, g_0] where
/// k = 2^(eccLevel+1) and g_k = 1 (the leading coefficient is always 1 for
/// a monic polynomial).
///
/// The polynomial is defined over GF(929), so all coefficients are in 0..928.
List<int> _buildGenerator(int eccLevel) {
  final k = 1 << (eccLevel + 1); // 2^(eccLevel+1) ECC codewords
  var g = [1]; // Start with the constant polynomial 1.

  // Multiply in each linear factor (x − α^j) for j = 3 to k+2.
  for (var j = 3; j <= k + 2; j++) {
    final root = _gfExp[j % _gf929Order]; // α^j mod 929
    // −root in GF(929) is (929 − root), since adding root gives 929 ≡ 0.
    final negRoot = _gf929Prime - root;

    final newG = List<int>.filled(g.length + 1, 0);
    for (var i = 0; i < g.length; i++) {
      // Coefficient of x^(degree−i) in the new polynomial:
      //   new[i]   += g[i]         (the x × g(x) term)
      //   new[i+1] += g[i] × −root (the −root × g(x) term)
      newG[i] = _gfAdd(newG[i], g[i]);
      newG[i + 1] = _gfAdd(newG[i + 1], _gfMul(g[i], negRoot));
    }
    g = newG;
  }

  return g;
}

// ============================================================================
// Reed-Solomon encoder
// ============================================================================
//
// Given data codewords D = [d_0, ..., d_{n-1}] and generator polynomial g(x)
// of degree k, the ECC codewords are the remainder of D(x)·x^k ÷ g(x).
//
// The standard way to compute this is the LFSR shift-register:
//   For each data symbol d:
//     feedback = d ⊕ ecc[0]   (⊕ = GF addition)
//     shift the ecc register left (ecc[0] drops out, ecc[k-1] becomes 0)
//     add feedback × g[k-i] to each ecc[i]
//
// No interleaving: unlike QR Code which splits data into multiple blocks,
// PDF417 feeds all data into a single RS encoder. This simplifies the
// encoder considerably.

/// Compute [k] RS ECC codewords for [data] over GF(929) with the b=3 convention.
///
/// [data] is the complete data-plus-length-descriptor sequence.
/// [eccLevel] determines the number of ECC codewords: k = 2^(eccLevel+1).
///
/// Returns exactly k codeword values in 0..928.
List<int> _rsEncode(List<int> data, int eccLevel) {
  final g = _buildGenerator(eccLevel);
  final k = g.length - 1; // degree of g = number of ECC codewords
  final ecc = List<int>.filled(k, 0);

  for (final d in data) {
    final feedback = _gfAdd(d, ecc[0]);

    // Shift the register left: ecc[0] is consumed, ecc[k-1] becomes 0.
    for (var i = 0; i < k - 1; i++) {
      ecc[i] = ecc[i + 1];
    }
    ecc[k - 1] = 0;

    // Add feedback × generator coefficient to each cell.
    // g[k-i] corresponds to the coefficient at position k-i in g.
    for (var i = 0; i < k; i++) {
      ecc[i] = _gfAdd(ecc[i], _gfMul(g[k - i], feedback));
    }
  }

  return ecc;
}

// ============================================================================
// Byte compaction
// ============================================================================
//
// Byte compaction encodes arbitrary binary data at approximately 1.2 bytes per
// codeword for long sequences (the "full group" encoding), falling back to
// 1 byte per codeword for any remainder.
//
// ## Full-group encoding: 6 bytes → 5 codewords
//
// Treat 6 bytes as a 48-bit big-endian integer, then express it in base 900:
//
//   n = b1×256^5 + b2×256^4 + b3×256^3 + b4×256^2 + b5×256 + b6
//   c5 = n mod 900;  n = n ÷ 900
//   c4 = n mod 900;  n = n ÷ 900
//   c3 = n mod 900;  n = n ÷ 900
//   c2 = n mod 900;  n = n ÷ 900
//   c1 = n
//
// The maximum n is 256^6 − 1 ≈ 2.81×10^14, which fits in a 64-bit integer.
// Dart's `int` on 64-bit platforms is a 64-bit signed integer (range up to
// ~9.2×10^18), so no BigInt is needed.
//
// ## Remainder encoding: 1–5 bytes → 1 codeword each
//
// Leftover bytes (after full groups) are mapped directly to their byte value,
// since 0..255 are all valid codeword values.
//
// ## Latch codeword
//
// Codeword 924 precedes the byte compaction segment. It is the "alternate"
// byte latch usable regardless of whether the byte count is divisible by 6.

/// Encode [bytes] using byte compaction mode (codeword 924 latch).
///
/// Returns the sequence `[924, c1, c2, ...]` where the codewords after 924
/// represent the byte-compacted data.
List<int> _byteCompact(List<int> bytes) {
  final codewords = <int>[_latchByte];

  var i = 0;
  final len = bytes.length;

  // Process full 6-byte groups: 6 bytes → 5 base-900 codewords.
  while (i + 6 <= len) {
    // Build the 48-bit big-endian integer from 6 consecutive bytes.
    // Dart `int` is 64-bit on all supported platforms; 256^6 < 2^48 < 2^63, safe.
    var n = 0;
    for (var j = 0; j < 6; j++) {
      n = n * 256 + bytes[i + j];
    }

    // Convert n to 5 base-900 codewords, most-significant first.
    final group = List<int>.filled(5, 0);
    for (var j = 4; j >= 0; j--) {
      group[j] = n % 900;
      n = n ~/ 900;
    }
    codewords.addAll(group);
    i += 6;
  }

  // Remaining bytes: 1 codeword per byte (direct mapping, values 0..255).
  while (i < len) {
    codewords.add(bytes[i]);
    i++;
  }

  return codewords;
}

// ============================================================================
// ECC level auto-selection
// ============================================================================
//
// The spec recommends higher ECC levels for longer data. The thresholds below
// are from ISO/IEC 15438:2015 and mirrored in the reference implementations.

/// Select the minimum recommended ECC level based on [dataCount].
///
/// | dataCount ≤ | Level | ECC codewords |
/// |-------------|-------|--------------|
/// | 40          |   2   |       8      |
/// | 160         |   3   |      16      |
/// | 320         |   4   |      32      |
/// | 863         |   5   |      64      |
/// | ∞           |   6   |     128      |
int _autoEccLevel(int dataCount) {
  if (dataCount <= 40) return 2;
  if (dataCount <= 160) return 3;
  if (dataCount <= 320) return 4;
  if (dataCount <= 863) return 5;
  return 6;
}

// ============================================================================
// Dimension selection
// ============================================================================
//
// We must choose c (data columns, 1–30) and r (rows, 3–90) such that:
//   r × c ≥ total_codewords
//
// Heuristic (from the TypeScript reference implementation):
//   c = clamp(ceil(sqrt(total / 3)), 1, 30)
//   r = clamp(ceil(total / c), 3, 90)
//
// The division by 3 accounts for the approximate 3:1 width-to-height aspect
// ratio of PDF417 rows: a 17-module codeword is roughly 3× wider than a
// 3-module-tall row, so dividing total by 3 gives a c that produces a
// roughly square symbol.

/// Choose the number of columns and rows for the symbol.
///
/// Returns a `(cols, rows)` record. Throws [InputTooLongError] if the data
/// cannot fit in any valid symbol.
({int cols, int rows}) _chooseDimensions(int total) {
  // Integer square root via simple sqrt; ceil to be safe.
  var c = _clampInt(_sqrtCeil(total ~/ 3 == 0 ? 1 : total ~/ 3), _minCols, _maxCols);

  var r = _clampInt((total / c).ceil(), _minRows, _maxRows);

  // If r is at maximum and we still can't fit, try increasing c.
  if (r * c < total) {
    for (var tryC = c + 1; tryC <= _maxCols; tryC++) {
      final tryR = _clampInt((total / tryC).ceil(), _minRows, _maxRows);
      if (tryR * tryC >= total) {
        c = tryC;
        r = tryR;
        break;
      }
    }
  }

  if (r * c < total) {
    throw InputTooLongError(
      'Cannot fit $total codewords in any valid PDF417 symbol '
      '(max ${_maxRows}×${_maxCols} = ${_maxRows * _maxCols}).',
    );
  }

  return (cols: c, rows: r);
}

/// Integer ceiling of sqrt(n).
///
/// For n ≤ 2700 (max PDF417 codewords), `math.sqrt` is exact in double
/// precision, so no correction is needed for typical inputs. We add a small
/// adjustment loop to be safe.
int _sqrtCeil(int n) {
  if (n <= 1) return 1;
  var x = math.sqrt(n.toDouble()).ceil();
  // Adjust for floating-point rounding.
  while (x * x < n) {
    x++;
  }
  while (x > 1 && (x - 1) * (x - 1) >= n) {
    x--;
  }
  return x;
}

/// Clamp [value] to the inclusive range [lo, hi].
int _clampInt(int value, int lo, int hi) {
  if (value < lo) return lo;
  if (value > hi) return hi;
  return value;
}

// ============================================================================
// Row indicator computation
// ============================================================================
//
// Every row in a PDF417 symbol has two row indicator codewords (LRI and RRI)
// that together encode three quantities:
//
//   R_info = (R − 1) / 3      where R = total rows (3..90) → R_info ∈ 0..29
//   C_info = C − 1            where C = data columns (1..30) → C_info ∈ 0..29
//   L_info = 3×L + (R-1) mod 3  encodes ECC level and row parity → 0..29
//
// The three quantities are distributed across the three clusters (one per row
// modulo 3), so that any three consecutive rows contain all three quantities.
//
// Formula (using cluster = r mod 3, row_group = r ÷ 3):
//
//   Cluster 0:  LRI = 30×row_group + R_info   RRI = 30×row_group + C_info
//   Cluster 1:  LRI = 30×row_group + L_info   RRI = 30×row_group + R_info
//   Cluster 2:  LRI = 30×row_group + C_info   RRI = 30×row_group + L_info
//
// Note: this formula follows the Python pdf417 library (verified scannable)
// rather than the original spec text which has a slight ambiguity in the RRI
// assignments.

/// Compute the Left Row Indicator codeword value for row [r].
///
/// [r] is the 0-indexed row number.
/// [rows] is the total number of rows R.
/// [cols] is the number of data columns C.
/// [eccLevel] is the ECC level L.
int computeLri(int r, int rows, int cols, int eccLevel) {
  final rInfo = (rows - 1) ~/ 3;
  final cInfo = cols - 1;
  final lInfo = 3 * eccLevel + (rows - 1) % 3;
  final rowGroup = r ~/ 3;
  final cluster = r % 3;

  if (cluster == 0) return 30 * rowGroup + rInfo;
  if (cluster == 1) return 30 * rowGroup + lInfo;
  return 30 * rowGroup + cInfo;
}

/// Compute the Right Row Indicator codeword value for row [r].
///
/// See [computeLri] for parameter documentation.
int computeRri(int r, int rows, int cols, int eccLevel) {
  final rInfo = (rows - 1) ~/ 3;
  final cInfo = cols - 1;
  final lInfo = 3 * eccLevel + (rows - 1) % 3;
  final rowGroup = r ~/ 3;
  final cluster = r % 3;

  if (cluster == 0) return 30 * rowGroup + cInfo;
  if (cluster == 1) return 30 * rowGroup + rInfo;
  return 30 * rowGroup + lInfo;
}

// ============================================================================
// Codeword → module expansion
// ============================================================================
//
// Each codeword value (0–928) maps to a different 17-module bar/space pattern
// in each of the three clusters (0, 3, 6 in the spec; 0, 1, 2 as 0-indexed).
//
// The pattern is packed into a 32-bit integer:
//   bits 31..28 = b1, bits 27..24 = s1, ..., bits 3..0 = s4
// where b_i and s_i are bar and space widths in modules (1..6).
// The 8 widths sum to exactly 17.

/// Expand a packed bar/space pattern into 17 boolean module values and add
/// them to [modules].
///
/// The input [packed] is a 32-bit integer encoding 8 element widths,
/// 4 bits each, from MSB to LSB: b1, s1, b2, s2, b3, s3, b4, s4.
///
/// The first element is always a bar (dark = true). Elements alternate:
/// bar, space, bar, space, bar, space, bar, space.
void _expandPattern(int packed, List<bool> modules) {
  final b1 = (packed >>> 28) & 0xf;
  final s1 = (packed >>> 24) & 0xf;
  final b2 = (packed >>> 20) & 0xf;
  final s2 = (packed >>> 16) & 0xf;
  final b3 = (packed >>> 12) & 0xf;
  final s3 = (packed >>> 8) & 0xf;
  final b4 = (packed >>> 4) & 0xf;
  final s4 = packed & 0xf;

  for (var i = 0; i < b1; i++) modules.add(true);
  for (var i = 0; i < s1; i++) modules.add(false);
  for (var i = 0; i < b2; i++) modules.add(true);
  for (var i = 0; i < s2; i++) modules.add(false);
  for (var i = 0; i < b3; i++) modules.add(true);
  for (var i = 0; i < s3; i++) modules.add(false);
  for (var i = 0; i < b4; i++) modules.add(true);
  for (var i = 0; i < s4; i++) modules.add(false);
}

/// Expand a bar/space width array into boolean module values and add them
/// to [modules].
///
/// The first width is a bar (dark). Subsequent widths alternate dark/light.
/// Used for the fixed start and stop patterns.
void _expandWidths(List<int> widths, List<bool> modules) {
  var dark = true;
  for (final w in widths) {
    for (var i = 0; i < w; i++) modules.add(dark);
    dark = !dark;
  }
}

// ============================================================================
// Rasterization
// ============================================================================
//
// After all codewords have been assembled (including padding and ECC), we
// rasterize each logical row into a sequence of dark/light modules:
//
//   [START 17] [LRI 17] [data×c, each 17] [RRI 17] [STOP 18]
//
// Total modules per row: 17 + 17 + 17c + 17 + 18 = 69 + 17c
//
// Each logical row is then repeated `rowHeight` times vertically to produce
// the final ModuleGrid.

/// Convert the flat codeword [sequence] into a [ModuleGrid].
///
/// [sequence] has exactly `rows × cols` entries (data+padding+ECC, no
/// row indicators).
///
/// The result has dimensions:
///   width  = 69 + 17 × cols  modules
///   height = rows × rowHeight  modules
ModuleGrid _rasterize(
  List<int> sequence,
  int rows,
  int cols,
  int eccLevel,
  int rowHeight,
) {
  // Each row: start(17) + LRI(17) + data×cols(17 each) + RRI(17) + stop(18)
  final moduleWidth = 69 + 17 * cols;
  final moduleHeight = rows * rowHeight;

  // Precompute start and stop modules (same for every row).
  final startModules = <bool>[];
  _expandWidths(kStartPattern, startModules);
  assert(startModules.length == 17, 'Start pattern must be 17 modules');

  final stopModules = <bool>[];
  _expandWidths(kStopPattern, stopModules);
  assert(stopModules.length == 18, 'Stop pattern must be 18 modules');

  // Build the grid as a mutable list-of-rows, then wrap in a ModuleGrid.
  // We use a flat representation initially for efficiency, then reshape.
  final rawModules = List.generate(
    moduleHeight,
    (_) => List<bool>.filled(moduleWidth, false),
    growable: false,
  );

  for (var r = 0; r < rows; r++) {
    final cluster = r % 3;
    final clusterTable = kClusterTables[cluster];

    // Build the module sequence for this logical row.
    final rowModules = <bool>[];

    // 1. Start pattern (17 modules, same for all rows).
    rowModules.addAll(startModules);

    // 2. Left Row Indicator (17 modules).
    final lri = computeLri(r, rows, cols, eccLevel);
    _expandPattern(clusterTable[lri], rowModules);

    // 3. Data codewords (17 modules each, c codewords).
    for (var j = 0; j < cols; j++) {
      final cw = sequence[r * cols + j];
      _expandPattern(clusterTable[cw], rowModules);
    }

    // 4. Right Row Indicator (17 modules).
    final rri = computeRri(r, rows, cols, eccLevel);
    _expandPattern(clusterTable[rri], rowModules);

    // 5. Stop pattern (18 modules, same for all rows).
    rowModules.addAll(stopModules);

    assert(
      rowModules.length == moduleWidth,
      'Row $r: expected $moduleWidth modules, got ${rowModules.length}',
    );

    // Write this module row rowHeight times into the grid.
    final moduleRowBase = r * rowHeight;
    for (var h = 0; h < rowHeight; h++) {
      final targetRow = rawModules[moduleRowBase + h];
      for (var col = 0; col < moduleWidth; col++) {
        targetRow[col] = rowModules[col];
      }
    }
  }

  return ModuleGrid(
    rows: moduleHeight,
    cols: moduleWidth,
    modules: rawModules,
    moduleShape: ModuleShape.square,
  );
}

// ============================================================================
// Main encoder: encode()
// ============================================================================

/// Encode [bytes] as a PDF417 symbol and return the [ModuleGrid].
///
/// [bytes] is a list of byte values (0–255). Arbitrary binary data is
/// supported; there is no restriction on content.
///
/// [options] controls ECC level, column count, and row height. All fields
/// are optional with sensible defaults.
///
/// ## Encoding pipeline
///
/// 1. Byte compaction: `[924, c1, c2, ...]` where groups of 6 bytes become
///    5 base-900 codewords; leftover bytes are mapped directly.
/// 2. Length descriptor: `codeword[0] = 1 + len(dataCwords) + eccCount`
/// 3. RS ECC: k = 2^(eccLevel+1) ECC codewords appended after data.
/// 4. Dimension selection: rows and cols chosen for roughly square aspect.
/// 5. Padding: codeword 900 fills unused slots.
/// 6. Rasterization: each row gets start/LRI/data/RRI/stop patterns.
///
/// @throws [InvalidEccLevelError] if `options.eccLevel` is not in 0–8.
/// @throws [InvalidDimensionsError] if `options.columns` is out of range.
/// @throws [InputTooLongError] if [bytes] exceeds symbol capacity.
ModuleGrid encode(List<int> bytes, {Pdf417Options options = const Pdf417Options()}) {
  // ── Validate ECC level ────────────────────────────────────────────────────
  if (options.eccLevel != null &&
      (options.eccLevel! < 0 || options.eccLevel! > 8)) {
    throw InvalidEccLevelError(
      'ECC level must be 0–8, got ${options.eccLevel}',
    );
  }

  // ── Validate columns ──────────────────────────────────────────────────────
  if (options.columns != null &&
      (options.columns! < _minCols || options.columns! > _maxCols)) {
    throw InvalidDimensionsError(
      'columns must be 1–30, got ${options.columns}',
    );
  }

  // ── Step 1: Byte compaction ────────────────────────────────────────────────
  final dataCwords = _byteCompact(bytes);

  // ── Step 2: Auto-select ECC level ─────────────────────────────────────────
  // The auto-level is based on the full data codeword count (including the
  // length descriptor we're about to prepend).
  final eccLevel = options.eccLevel ?? _autoEccLevel(dataCwords.length + 1);
  final eccCount = 1 << (eccLevel + 1); // 2^(eccLevel+1)

  // ── Step 3: Length descriptor ──────────────────────────────────────────────
  // The length descriptor is the first codeword. It counts itself + all data
  // codewords + all ECC codewords (but NOT padding, which is invisible to the
  // decoder). This lets a decoder know how many data codewords to expect.
  final lengthDesc = 1 + dataCwords.length + eccCount;
  final fullData = [lengthDesc, ...dataCwords];

  // ── Step 4: RS ECC ─────────────────────────────────────────────────────────
  final eccCwords = _rsEncode(fullData, eccLevel);

  // ── Step 5: Choose dimensions ──────────────────────────────────────────────
  final totalCwords = fullData.length + eccCwords.length;

  int cols;
  int rows;

  if (options.columns != null) {
    cols = options.columns!;
    rows = _clampInt((totalCwords / cols).ceil(), _minRows, _maxRows);
    if (rows * cols < totalCwords) {
      throw InputTooLongError(
        'Data requires more than $rows rows with $cols columns.',
      );
    }
  } else {
    final dims = _chooseDimensions(totalCwords);
    cols = dims.cols;
    rows = dims.rows;
  }

  // Final capacity check.
  if (cols * rows < totalCwords) {
    throw InputTooLongError(
      'Cannot fit $totalCwords codewords in ${rows}×$cols grid.',
    );
  }

  // ── Step 6: Pad to fill grid exactly ──────────────────────────────────────
  // Padding goes between data and ECC in the final sequence. The decoder
  // uses the length descriptor to skip over padding codewords.
  final paddingCount = cols * rows - totalCwords;
  final paddedData = [...fullData, ...List<int>.filled(paddingCount, _paddingCw)];

  // Full codeword sequence: [data+padding, ecc] — exactly rows×cols entries.
  final fullSequence = [...paddedData, ...eccCwords];
  assert(
    fullSequence.length == rows * cols,
    'fullSequence.length = ${fullSequence.length}, expected ${rows * cols}',
  );

  // ── Step 7: Rasterize ─────────────────────────────────────────────────────
  final rowHeight = options.rowHeight < 1 ? 1 : options.rowHeight;
  return _rasterize(fullSequence, rows, cols, eccLevel, rowHeight);
}

/// Encode a UTF-8 [string] as a PDF417 symbol and return the [ModuleGrid].
///
/// The string is first encoded to UTF-8 bytes, then byte-compacted as usual.
/// For ASCII text, each character produces exactly 1 byte; multi-byte
/// characters (e.g. emoji, accented letters) produce 2–4 bytes each.
///
/// If you want to use text compaction (v0.2.0) or numeric compaction (v0.2.0),
/// convert the string to bytes manually and pass them to [encode].
ModuleGrid encodeString(String s, {Pdf417Options options = const Pdf417Options()}) {
  return encode(utf8.encode(s), options: options);
}

/// Encode [bytes] and pass the result through the layout pipeline, producing a
/// [PaintScene] ready for a render backend (SVG, Canvas, Metal, etc.).
///
/// Uses quiet-zone = 2 (PDF417 minimum per ISO/IEC 15438:2015 §6.3) and
/// module size = 10 px by default. Pass [layoutConfig] to override.
PaintScene encodeAndLayout(
  List<int> bytes, {
  Pdf417Options options = const Pdf417Options(),
  Barcode2DLayoutConfig? layoutConfig,
}) {
  final grid = encode(bytes, options: options);
  final cfg = layoutConfig ??
      const Barcode2DLayoutConfig(
        moduleSizePx: 10,
        quietZoneModules: 2,
        foreground: '#000000',
        background: '#ffffff',
        moduleShape: ModuleShape.square,
      );
  return layout(grid, config: cfg);
}
