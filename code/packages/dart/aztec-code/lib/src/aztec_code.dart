/// Aztec Code encoder — ISO/IEC 24778:2008 compliant.
///
/// Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995 and
/// published as a patent-free format. Unlike QR Code (which uses three square
/// finder patterns at three corners), Aztec Code places a single **bullseye
/// finder pattern at the center** of the symbol. The scanner finds the centre
/// first, then reads outward in a spiral — no large quiet zone is needed.
///
/// ## Where Aztec Code is used today
///
/// - **IATA boarding passes** — the barcode on every airline boarding pass.
/// - **Eurostar and Amtrak rail tickets** — printed and on-screen tickets.
/// - **PostNL, Deutsche Post, La Poste** — European postal routing.
/// - **US military ID cards.**
///
/// ## Symbol variants
///
/// ```
/// Compact: 1–4 layers,  size = 11 + 4×layers   (15×15 to 27×27)
/// Full:    1–32 layers, size = 15 + 4×layers   (19×19 to 143×143)
/// ```
///
/// ## Encoding pipeline (v0.1.0 — byte-mode only)
///
/// ```
/// input string / bytes
///   → Binary-Shift codewords from Upper mode
///   → symbol size selection (smallest compact then full that fits at 23% ECC)
///   → pad to exact codeword count
///   → GF(256)/0x12D Reed-Solomon ECC (b=1, roots α^1..α^n)
///   → bit stuffing (insert complement bit after 4 consecutive identical bits)
///   → GF(16) mode message (layers + codeword count + 5 or 6 RS nibbles)
///   → ModuleGrid  (bullseye → orientation marks → mode msg → data spiral)
/// ```
///
/// ## v0.1.0 simplifications
///
/// 1. **Byte-mode only** — all input encoded via Binary-Shift from Upper mode.
///    Multi-mode (Digit/Upper/Lower/Mixed/Punct) optimisation is v0.2.0.
/// 2. **8-bit codewords → GF(256) RS** (same polynomial as Data Matrix: 0x12D).
///    GF(16) and GF(32) RS for 4-bit/5-bit codewords are v0.2.0.
/// 3. **Default ECC = 23%.**
/// 4. **Auto-select compact vs full** (force-compact option is v0.2.0).
library aztec_code;

import 'package:coding_adventures_barcode_2d/coding_adventures_barcode_2d.dart';

// ============================================================================
// Package version
// ============================================================================

/// Package version following Semantic Versioning 2.0.
const String aztecCodeVersion = '0.1.0';

// ============================================================================
// Public types
// ============================================================================

/// Options for Aztec Code encoding.
///
/// The only option in v0.1.0 is [minEccPercent], which controls how many
/// symbol slots are reserved for error-correction codewords.  Higher values
/// make the symbol more resilient to physical damage at the cost of requiring
/// a larger symbol (or rejecting longer inputs).
///
/// ```dart
/// final strict = const AztecOptions(minEccPercent: 33);
/// final grid   = encode('hello', options: strict);
/// ```
class AztecOptions {
  /// Minimum error-correction percentage.  Default: 23.  Range: 10–90.
  ///
  /// At 23% the symbol can recover from the corruption of roughly one in four
  /// modules — well within IATA boarding pass scanning conditions.
  final int minEccPercent;

  const AztecOptions({this.minEccPercent = 23});
}

// ============================================================================
// Error hierarchy
// ============================================================================

/// Base class for all Aztec Code encoder errors.
///
/// Catch [AztecError] to handle any encoder error regardless of subclass.
///
/// ```dart
/// try {
///   encode(veryLongData);
/// } on AztecError catch (e) {
///   print('Aztec failed: $e');
/// }
/// ```
class AztecError implements Exception {
  /// Human-readable description of the error.
  final String message;

  const AztecError(this.message);

  @override
  String toString() => 'AztecError: $message';
}

/// Thrown when the input is too long for any Aztec Code symbol.
///
/// The maximum capacity is roughly 1914 bytes in a 32-layer full symbol at
/// 23% ECC.  For larger payloads, split the data or use a different format.
class InputTooLongError extends AztecError {
  const InputTooLongError(super.message);

  @override
  String toString() => 'InputTooLongError: $message';
}

// ============================================================================
// GF(16) arithmetic — for the mode message Reed-Solomon
// ============================================================================
//
// GF(16) is the finite field with 16 elements, built from the primitive
// polynomial:
//
//   p(x) = x^4 + x + 1   (binary: 10011 = 0x13)
//
// Every non-zero element can be written as a power of the primitive
// element alpha.  alpha is the root of p(x), so alpha^4 = alpha + 1.
//
// The log table maps a field element (1..15) to its discrete log (0..14).
// The antilog (exponentiation) table maps a log value to its element.
//
// alpha^0=1, alpha^1=2, alpha^2=4, alpha^3=8,
// alpha^4=3, alpha^5=6, alpha^6=12, alpha^7=11,
// alpha^8=5, alpha^9=10, alpha^10=7, alpha^11=14,
// alpha^12=15, alpha^13=13, alpha^14=9, alpha^15=1 (period=15)

/// GF(16) discrete logarithm table: _log16[e] = i means alpha^i = e.
///
/// Index 0 is undefined (log(0) is undefined in any field) — we represent
/// it as -1 to catch accidental use.
const List<int> _log16 = [
  -1, // log(0) = undefined
  0,  // log(1)  = 0  → alpha^0 = 1
  1,  // log(2)  = 1  → alpha^1 = 2
  4,  // log(3)  = 4  → alpha^4 = 3
  2,  // log(4)  = 2  → alpha^2 = 4
  8,  // log(5)  = 8  → alpha^8 = 5
  5,  // log(6)  = 5  → alpha^5 = 6
  10, // log(7)  = 10
  3,  // log(8)  = 3
  14, // log(9)  = 14
  9,  // log(10) = 9
  7,  // log(11) = 7
  6,  // log(12) = 6
  13, // log(13) = 13
  11, // log(14) = 11
  12, // log(15) = 12
];

/// GF(16) antilogarithm table: _alog16[i] = alpha^i.
///
/// Period is 15, so index 15 wraps back to alpha^0 = 1.
/// We store 16 entries (0..15) for convenient modular indexing.
const List<int> _alog16 = [1, 2, 4, 8, 3, 6, 12, 11, 5, 10, 7, 14, 15, 13, 9, 1];

/// Multiply two GF(16) elements using the log/antilog shortcut.
///
/// For non-zero a, b: a × b = alpha^(log(a) + log(b)) mod 15.
/// Multiplication distributes over addition (XOR) in characteristic-2 fields.
int _gf16Mul(int a, int b) {
  if (a == 0 || b == 0) return 0;
  return _alog16[(_log16[a] + _log16[b]) % 15];
}

/// Build the GF(16) RS generator polynomial with roots alpha^1..alpha^n.
///
/// Returns [g₀, g₁, ..., gₙ] where gₙ = 1 (monic polynomial).
///
/// Built incrementally by multiplying one factor (x − alpha^i) at a time.
/// In a characteristic-2 field, subtraction is the same as addition (XOR),
/// so x − alpha^i = x + alpha^i.
List<int> _buildGf16Generator(int n) {
  var g = [1];
  for (var i = 1; i <= n; i++) {
    final ai = _alog16[i % 15];
    final nxt = List<int>.filled(g.length + 1, 0);
    for (var j = 0; j < g.length; j++) {
      nxt[j + 1] ^= g[j];
      nxt[j] ^= _gf16Mul(ai, g[j]);
    }
    g = nxt;
  }
  return g;
}

/// Compute [n] GF(16) RS check nibbles for the given data nibbles.
///
/// Uses the LFSR polynomial-division algorithm — the same shape as a
/// classic CRC computation but in GF(16) arithmetic.
List<int> _gf16RsEncode(List<int> data, int n) {
  final g = _buildGf16Generator(n);
  final rem = List<int>.filled(n, 0);
  for (final nibble in data) {
    final fb = nibble ^ rem[0];
    for (var i = 0; i < n - 1; i++) {
      rem[i] = rem[i + 1] ^ _gf16Mul(g[i + 1], fb);
    }
    rem[n - 1] = _gf16Mul(g[n], fb);
  }
  return rem;
}

// ============================================================================
// GF(256)/0x12D arithmetic — for 8-bit data codewords
// ============================================================================
//
// Aztec Code uses GF(256) with primitive polynomial:
//   p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  =  0x12D
//
// This is the SAME polynomial as Data Matrix ECC200, but DIFFERENT from
// QR Code (0x11D).  The barcode-2d repo's gf256 helper targets QR's 0x11D,
// so we build 0x12D tables inline here.
//
// Generator convention: b=1, roots alpha^1..alpha^n (the MA02 / Aztec style).

const int _gf256Poly = 0x12D;

/// Lazily-initialised GF(256)/0x12D antilogarithm table (doubled, length 512).
///
/// The primitive element is alpha = 2.  Repeatedly doubling modulo the
/// primitive polynomial enumerates all 255 non-zero elements before cycling
/// back to 1.
///
/// The doubled antilog table (index range 0..509) lets multiplication skip
/// the modulo-255 reduction: alpha^(i+j) = exp[i+j] for i, j in [0,254].
///
/// Dart does not support top-level record destructuring, so we compute both
/// tables in the same lazy getter and expose them via separate top-level
/// getters below.
List<int>? __exp12d;
List<int>? __log12d;

void _ensureGf256Tables() {
  if (__exp12d != null) return;
  final exp = List<int>.filled(512, 0);
  final log = List<int>.filled(256, 0);
  var x = 1;
  for (var i = 0; i < 255; i++) {
    exp[i] = x;
    exp[i + 255] = x;
    log[x] = i;
    x <<= 1;
    if (x & 0x100 != 0) x ^= _gf256Poly;
    x &= 0xFF;
  }
  exp[255] = 1;
  __exp12d = exp;
  __log12d = log;
}

List<int> get _exp12d {
  _ensureGf256Tables();
  return __exp12d!;
}

List<int> get _log12d {
  _ensureGf256Tables();
  return __log12d!;
}

/// Multiply two GF(256)/0x12D elements via log/antilog lookup.
///
/// Returns 0 if either operand is 0 (GF multiplication always maps 0 to 0).
int _gf256Mul(int a, int b) {
  if (a == 0 || b == 0) return 0;
  return _exp12d[_log12d[a] + _log12d[b]];
}

/// Build the GF(256)/0x12D RS generator polynomial.
///
/// Roots are alpha^1..alpha^n.  Returns big-endian coefficients (highest
/// degree first) — this matches the iteration order used in [_gf256RsEncode].
List<int> _buildGf256Generator(int n) {
  var g = [1];
  for (var i = 1; i <= n; i++) {
    final ai = _exp12d[i];
    final nxt = List<int>.filled(g.length + 1, 0);
    for (var j = 0; j < g.length; j++) {
      nxt[j] ^= g[j];
      nxt[j + 1] ^= _gf256Mul(g[j], ai);
    }
    g = nxt;
  }
  return g;
}

/// Compute [nCheck] GF(256)/0x12D RS check bytes for the given data bytes.
///
/// Standard LFSR polynomial division.  [rem] holds the partial remainder
/// coefficients in big-endian order.
List<int> _gf256RsEncode(List<int> data, int nCheck) {
  final g = _buildGf256Generator(nCheck);
  final n = g.length - 1;
  final rem = List<int>.filled(n, 0);
  for (final byte in data) {
    final fb = byte ^ rem[0];
    for (var i = 0; i < n - 1; i++) {
      rem[i] = rem[i + 1] ^ _gf256Mul(g[i + 1], fb);
    }
    rem[n - 1] = _gf256Mul(g[n], fb);
  }
  return rem;
}

// ============================================================================
// Aztec Code capacity tables
// ============================================================================
//
// Derived from ISO/IEC 24778:2008 Table 1.
// Each entry holds:
//   totalBits  — total data+ECC bit positions in the symbol layers.
//   maxBytes8  — number of 8-bit codeword slots (data + ECC combined).

/// One row of the Aztec capacity table.
class _Capacity {
  /// Total data+ECC bit positions in the symbol layers.
  final int totalBits;

  /// Number of 8-bit codeword slots (data + ECC combined).
  final int maxBytes8;

  const _Capacity(this.totalBits, this.maxBytes8);
}

/// Compact symbol capacities for layers 1–4 (index 0 is unused).
///
/// Compact symbols (11+4×layers modules square):
///   Layer 1: 15×15 →   72 bits,  9 codewords
///   Layer 2: 19×19 →  200 bits, 25 codewords
///   Layer 3: 23×23 →  392 bits, 49 codewords
///   Layer 4: 27×27 →  648 bits, 81 codewords
const List<_Capacity> _compactCapacity = [
  _Capacity(0, 0),     // index 0 unused
  _Capacity(72, 9),    // 1 layer, 15×15
  _Capacity(200, 25),  // 2 layers, 19×19
  _Capacity(392, 49),  // 3 layers, 23×23
  _Capacity(648, 81),  // 4 layers, 27×27
];

/// Full symbol capacities for layers 1–32 (index 0 is unused).
///
/// Full symbols (15+4×layers modules square).
const List<_Capacity> _fullCapacity = [
  _Capacity(0, 0),           // index 0 unused
  _Capacity(88, 11),         //  1 layer,  19×19
  _Capacity(216, 27),        //  2 layers, 23×23
  _Capacity(360, 45),        //  3 layers, 27×27
  _Capacity(520, 65),        //  4 layers, 31×31
  _Capacity(696, 87),        //  5 layers, 35×35
  _Capacity(888, 111),       //  6 layers, 39×39
  _Capacity(1096, 137),      //  7 layers, 43×43
  _Capacity(1320, 165),      //  8 layers, 47×47
  _Capacity(1560, 195),      //  9 layers, 51×51
  _Capacity(1816, 227),      // 10 layers, 55×55
  _Capacity(2088, 261),      // 11 layers, 59×59
  _Capacity(2376, 297),      // 12 layers, 63×63
  _Capacity(2680, 335),      // 13 layers, 67×67
  _Capacity(3000, 375),      // 14 layers, 71×71
  _Capacity(3336, 417),      // 15 layers, 75×75
  _Capacity(3688, 461),      // 16 layers, 79×79
  _Capacity(4056, 507),      // 17 layers, 83×83
  _Capacity(4440, 555),      // 18 layers, 87×87
  _Capacity(4840, 605),      // 19 layers, 91×91
  _Capacity(5256, 657),      // 20 layers, 95×95
  _Capacity(5688, 711),      // 21 layers, 99×99
  _Capacity(6136, 767),      // 22 layers, 103×103
  _Capacity(6600, 825),      // 23 layers, 107×107
  _Capacity(7080, 885),      // 24 layers, 111×111
  _Capacity(7576, 947),      // 25 layers, 115×115
  _Capacity(8088, 1011),     // 26 layers, 119×119
  _Capacity(8616, 1077),     // 27 layers, 123×123
  _Capacity(9160, 1145),     // 28 layers, 127×127
  _Capacity(9720, 1215),     // 29 layers, 131×131
  _Capacity(10296, 1287),    // 30 layers, 135×135
  _Capacity(10888, 1361),    // 31 layers, 139×139
  _Capacity(11496, 1437),    // 32 layers, 143×143
];

// ============================================================================
// Data encoding — Binary-Shift from Upper mode (byte-mode path)
// ============================================================================
//
// All input is wrapped in a single Binary-Shift block from Upper mode:
//
//   1. Emit 5 bits = 0b11111  (Binary-Shift escape in Upper mode)
//   2. If len <= 31: 5 bits for length
//      If len > 31:  5 bits = 0b00000, then 11 bits for length
//   3. Each byte as 8 bits, MSB first
//
// Why Binary-Shift from Upper?  It is the simplest possible encoding path.
// Upper mode is the default start mode, and the Binary-Shift escape code
// (11111) lets us escape directly to raw 8-bit bytes without changing modes.
// Multi-mode optimization (e.g. using Digit mode for "12345") is v0.2.0.

/// Encode input bytes as a flat bit list using the Binary-Shift escape.
///
/// Returns a List<int> of 0/1 values, MSB first.
/// This is the entire data bit stream consumed by the symbol-selection step.
List<int> _encodeBytesAsBits(List<int> bytes) {
  final bits = <int>[];

  // Write [count] bits from [value], MSB first.
  void writeBits(int value, int count) {
    for (var i = count - 1; i >= 0; i--) {
      bits.add((value >> i) & 1);
    }
  }

  final length = bytes.length;

  // Binary-Shift escape: 5 bits all-ones in Upper mode.
  writeBits(0x1F, 5);

  // Length field:
  //   Short form (5 bits): for 1–31 bytes.
  //   Long form (5 zero bits + 11 bits): for 32–2047 bytes.
  if (length <= 31) {
    writeBits(length, 5);
  } else {
    writeBits(0, 5);
    writeBits(length, 11);
  }

  // Each byte, 8 bits MSB first.
  for (final byte in bytes) {
    writeBits(byte, 8);
  }

  return bits;
}

// ============================================================================
// Symbol size selection
// ============================================================================

/// Resolved symbol parameters chosen by [_selectSymbol].
class _SymbolSpec {
  final bool compact;
  final int layers;
  final int dataCwCount;
  final int eccCwCount;
  final int totalBits;

  const _SymbolSpec({
    required this.compact,
    required this.layers,
    required this.dataCwCount,
    required this.eccCwCount,
    required this.totalBits,
  });
}

/// Select the smallest symbol that fits [dataBitCount] bits.
///
/// Tries compact 1–4 first, then full 1–32.
///
/// ## Stuffing overhead
///
/// Bit stuffing (see [_stuffBits]) inserts a complement bit after every
/// 4 consecutive identical bits.  In the worst case every 5th bit is
/// overhead — a 20% increase.  We multiply by 12/10 (ceiling division)
/// before sizing.
///
/// Throws [InputTooLongError] if no symbol can hold the data.
_SymbolSpec _selectSymbol(int dataBitCount, int minEccPct) {
  // Stuffing inserts at most 1 bit per 4 input bits → 20% overhead (safe upper bound).
  // ceil(x * 1.2) = ceil(x * 12 / 10) = (-(-x * 12) ~/ 10)
  final stuffedBitCount = (-(-dataBitCount * 12) ~/ 10);

  // Try compact symbols (1–4 layers) first — they are smaller and simpler.
  for (var layers = 1; layers <= 4; layers++) {
    final cap = _compactCapacity[layers];
    final totalBytes = cap.maxBytes8;
    final eccCwCount = (-(-minEccPct * totalBytes) ~/ 100); // ceil
    final dataCwCount = totalBytes - eccCwCount;
    if (dataCwCount <= 0) continue;
    // ceil(stuffedBitCount / 8) <= dataCwCount  ↔  stuffedBitCount <= dataCwCount * 8
    if ((-(-stuffedBitCount) ~/ 8) <= dataCwCount) {
      return _SymbolSpec(
        compact: true,
        layers: layers,
        dataCwCount: dataCwCount,
        eccCwCount: eccCwCount,
        totalBits: cap.totalBits,
      );
    }
  }

  // Fall back to full symbols (1–32 layers).
  for (var layers = 1; layers <= 32; layers++) {
    final cap = _fullCapacity[layers];
    final totalBytes = cap.maxBytes8;
    final eccCwCount = (-(-minEccPct * totalBytes) ~/ 100);
    final dataCwCount = totalBytes - eccCwCount;
    if (dataCwCount <= 0) continue;
    if ((-(-stuffedBitCount) ~/ 8) <= dataCwCount) {
      return _SymbolSpec(
        compact: false,
        layers: layers,
        dataCwCount: dataCwCount,
        eccCwCount: eccCwCount,
        totalBits: cap.totalBits,
      );
    }
  }

  throw InputTooLongError(
    'Input is too long to fit in any Aztec Code symbol '
    '($dataBitCount bits needed, max ~91648 stuffed bits at 23% ECC)',
  );
}

// ============================================================================
// Padding
// ============================================================================

/// Pad the bit stream up to exactly [targetBytes] * 8 bits with zeroes.
///
/// First pad to the next byte boundary (so the bit stream is whole bytes),
/// then keep appending zero bits until reaching [targetBytes] * 8 bits.
/// Truncates if already longer (the selector verified capacity, so this
/// should never happen in practice).
List<int> _padToBytes(List<int> bits, int targetBytes) {
  final out = List<int>.from(bits);
  while (out.length % 8 != 0) {
    out.add(0);
  }
  while (out.length < targetBytes * 8) {
    out.add(0);
  }
  return out.sublist(0, targetBytes * 8);
}

// ============================================================================
// Bit stuffing
// ============================================================================
//
// After every 4 consecutive identical bits (all 0 or all 1), insert one
// complement bit.  Applies only to the data+ECC bit stream — not the mode
// message, which is placed around the bullseye without stuffing.
//
// Example:
//   Input:  1 1 1 1 0 0 0 0
//   After 4 ones: insert 0  →  1 1 1 1 0 ...
//   After 4 zeros: insert 1 →  0 0 0 0 1 ...
//
// This rule prevents long runs of identical bits, which the scanner needs to
// distinguish from the bullseye/orientation patterns.

/// Apply Aztec bit stuffing to the data+ECC bit stream.
///
/// Inserts a complement bit after every run of 4 identical bits.  After
/// inserting, the run resets so the inserted complement counts as a run-of-1.
List<int> _stuffBits(List<int> bits) {
  final stuffed = <int>[];
  var runVal = -1;
  var runLen = 0;

  for (final bit in bits) {
    if (bit == runVal) {
      runLen++;
    } else {
      runVal = bit;
      runLen = 1;
    }

    stuffed.add(bit);

    if (runLen == 4) {
      // Insert the complement of the repeated bit.
      final stuffBit = 1 - bit;
      stuffed.add(stuffBit);
      runVal = stuffBit;
      runLen = 1;
    }
  }

  return stuffed;
}

// ============================================================================
// Mode message encoding
// ============================================================================
//
// The mode message encodes the layer count and data codeword count,
// protected by a GF(16) Reed-Solomon code.
//
// Compact (28 bits = 7 nibbles):
//   m = ((layers-1) << 6) | (dataCwCount-1)
//   2 data nibbles + 5 ECC nibbles
//
// Full (40 bits = 10 nibbles):
//   m = ((layers-1) << 11) | (dataCwCount-1)
//   4 data nibbles + 6 ECC nibbles
//
// Note: data nibbles are emitted little-endian (LSB nibble first), but bits
// within each nibble are MSB-first.  This matches the wire-format convention
// in ISO/IEC 24778:2008 Annex G.

/// Encode the mode message as a flat bit list.
///
/// Returns 28 bits for compact symbols, 40 bits for full symbols.
List<int> _encodeModeMessage(bool compact, int layers, int dataCwCount) {
  final List<int> dataNibbles;
  final int numEcc;

  if (compact) {
    // Compact: 8-bit word → 2 nibbles (little-endian nibble order).
    final m = ((layers - 1) << 6) | (dataCwCount - 1);
    dataNibbles = [m & 0xF, (m >> 4) & 0xF];
    numEcc = 5;
  } else {
    // Full: 15-bit word → 4 nibbles (little-endian nibble order).
    final m = ((layers - 1) << 11) | (dataCwCount - 1);
    dataNibbles = [
      m & 0xF,
      (m >> 4) & 0xF,
      (m >> 8) & 0xF,
      (m >> 12) & 0xF,
    ];
    numEcc = 6;
  }

  final eccNibbles = _gf16RsEncode(dataNibbles, numEcc);
  final allNibbles = [...dataNibbles, ...eccNibbles];

  final bits = <int>[];
  for (final nibble in allNibbles) {
    for (var i = 3; i >= 0; i--) {
      bits.add((nibble >> i) & 1);
    }
  }

  return bits;
}

// ============================================================================
// Grid construction helpers
// ============================================================================

/// Side length in modules.
///
/// Compact: 11 + 4×layers.  Full: 15 + 4×layers.
int _symbolSize(bool compact, int layers) {
  return compact ? (11 + 4 * layers) : (15 + 4 * layers);
}

/// Bullseye Chebyshev radius.
///
/// Compact: 5 (bullseye spans 11×11 modules at the centre).
/// Full:    7 (bullseye spans 15×15 modules at the centre).
///
/// The Chebyshev (chessboard) distance `d = max(|dx|, |dy|)` is the right
/// metric because the bullseye is composed of square rings, not circular rings.
int _bullseyeRadius(bool compact) => compact ? 5 : 7;

/// Paint the bullseye finder pattern at the centre.
///
/// Module colour at Chebyshev distance d from centre:
/// - d ≤ 1 → DARK (the solid 3×3 inner core)
/// - d > 1, d even → LIGHT (gap ring)
/// - d > 1, d odd → DARK (ring)
///
/// Also marks every painted cell as reserved so the data spiral skips them.
///
/// ```
/// Distance map (compact, br=5):
///   5 5 5 5 5 5 5 5 5 5 5
///   5 4 4 4 4 4 4 4 4 4 5
///   5 4 3 3 3 3 3 3 3 4 5
///   5 4 3 2 2 2 2 2 3 4 5
///   5 4 3 2 1 1 1 2 3 4 5
///   5 4 3 2 1 0 1 2 3 4 5   ← centre
///   ...
///
/// DARK if d≤1 or d is odd → d=0,1,3,5 are dark; d=2,4 are light.
/// ```
void _drawBullseye(
  List<List<bool>> modules,
  List<List<bool>> reserved,
  int cx,
  int cy,
  bool compact,
) {
  final br = _bullseyeRadius(compact);
  for (var row = cy - br; row <= cy + br; row++) {
    for (var col = cx - br; col <= cx + br; col++) {
      final d = (col - cx).abs() > (row - cy).abs()
          ? (col - cx).abs()
          : (row - cy).abs();
      final dark = d <= 1 || d % 2 == 1;
      modules[row][col] = dark;
      reserved[row][col] = true;
    }
  }
}

/// Paint the reference grid for full Aztec symbols.
///
/// Reference grid lines lie at rows/cols whose offset from the centre is a
/// multiple of 16.  The module value alternates dark/light along each line
/// based on the distance from the centre.
///
/// This grid is later partially overwritten by the bullseye and mode-message
/// rings near the centre — those reservations are written by subsequent calls.
void _drawReferenceGrid(
  List<List<bool>> modules,
  List<List<bool>> reserved,
  int cx,
  int cy,
  int size,
) {
  for (var row = 0; row < size; row++) {
    for (var col = 0; col < size; col++) {
      final onH = (cy - row) % 16 == 0;
      final onV = (cx - col) % 16 == 0;
      if (!onH && !onV) continue;

      final bool dark;
      if (onH && onV) {
        dark = true;
      } else if (onH) {
        dark = (cx - col) % 2 == 0;
      } else {
        dark = (cy - row) % 2 == 0;
      }

      modules[row][col] = dark;
      reserved[row][col] = true;
    }
  }
}

/// Paint orientation marks and place mode-message bits around the centre.
///
/// The mode-message ring is the perimeter at Chebyshev radius
/// (bullseyeRadius + 1).  The 4 corners of that ring are orientation marks
/// (always DARK — they help scanners determine symbol rotation).  The remaining
/// non-corner positions carry the mode-message bits clockwise starting from
/// the top edge (top-left corner + 1).
///
/// Returns the leftover ring positions (after mode-message bits are placed) as
/// (col, row) pairs.  The caller fills these from the data spiral.
List<(int col, int row)> _drawOrientationAndModeMessage(
  List<List<bool>> modules,
  List<List<bool>> reserved,
  int cx,
  int cy,
  bool compact,
  List<int> modeMsgBits,
) {
  final r = _bullseyeRadius(compact) + 1;

  // Enumerate non-corner perimeter positions clockwise, starting from the
  // cell immediately right of the top-left corner.
  final nonCorner = <(int col, int row)>[];

  // Top edge: left to right (skip both corners).
  for (var col = cx - r + 1; col < cx + r; col++) {
    nonCorner.add((col, cy - r));
  }
  // Right edge: top to bottom (skip both corners).
  for (var row = cy - r + 1; row < cy + r; row++) {
    nonCorner.add((cx + r, row));
  }
  // Bottom edge: right to left (skip both corners).
  for (var col = cx + r - 1; col > cx - r; col--) {
    nonCorner.add((col, cy + r));
  }
  // Left edge: bottom to top (skip both corners).
  for (var row = cy + r - 1; row > cy - r; row--) {
    nonCorner.add((cx - r, row));
  }

  // Place the 4 orientation-mark corners as DARK.
  final corners = [
    (cx - r, cy - r),
    (cx + r, cy - r),
    (cx + r, cy + r),
    (cx - r, cy + r),
  ];
  for (final (col, row) in corners) {
    modules[row][col] = true;
    reserved[row][col] = true;
  }

  // Place mode-message bits.
  final limit = modeMsgBits.length < nonCorner.length
      ? modeMsgBits.length
      : nonCorner.length;
  for (var i = 0; i < limit; i++) {
    final (col, row) = nonCorner[i];
    modules[row][col] = modeMsgBits[i] == 1;
    reserved[row][col] = true;
  }

  // Return leftover ring positions (the data spiral fills these first).
  return nonCorner.sublist(modeMsgBits.length);
}

// ============================================================================
// Data layer spiral placement
// ============================================================================
//
// Bits are placed in a clockwise spiral starting from the innermost data
// layer.  Each layer band is 2 modules wide.  Within a band, pairs of cells
// are written outer-row/col first, then inner.
//
// For compact: d_inner of the first layer = bullseyeRadius + 2 = 7.
// For full:    d_inner of the first layer = bullseyeRadius + 2 = 9.
//
// Clockwise order within a layer band (d_i = inner radius, d_o = outer radius):
//   Top edge:    left-to-right, writing [outer row, inner row]
//   Right edge:  top-to-bottom, writing [outer col, inner col]
//   Bottom edge: right-to-left, writing [outer row, inner row]
//   Left edge:   bottom-to-top, writing [outer col, inner col]

/// Place all data bits using the clockwise layer spiral.
///
/// Fills the leftover mode-ring positions first, then spirals outward
/// layer by layer.  Cells already reserved by the bullseye, orientation
/// marks, mode message, or reference grid are silently skipped.
void _placeDataBits(
  List<List<bool>> modules,
  List<List<bool>> reserved,
  List<int> bits,
  int cx,
  int cy,
  bool compact,
  int layers,
  List<(int col, int row)> modeRingRemainingPositions,
) {
  final size = modules.length;
  var bitIndex = 0;

  void placeBit(int col, int row) {
    if (row < 0 || row >= size || col < 0 || col >= size) return;
    if (!reserved[row][col]) {
      final value = bitIndex < bits.length ? bits[bitIndex] : 0;
      modules[row][col] = value == 1;
      bitIndex++;
    }
  }

  // Fill remaining mode ring positions first (these are not yet reserved,
  // but we track them so the spiral doesn't overwrite them).
  for (final (col, row) in modeRingRemainingPositions) {
    final value = bitIndex < bits.length ? bits[bitIndex] : 0;
    modules[row][col] = value == 1;
    bitIndex++;
  }

  // Spiral through data layers, innermost first.
  final br = _bullseyeRadius(compact);
  final dStart = br + 2; // mode-msg ring sits at br+1; first data layer at br+2

  for (var layer = 0; layer < layers; layer++) {
    final dI = dStart + 2 * layer; // inner radius of the layer band
    final dO = dI + 1;             // outer radius of the layer band

    // Top edge: left to right.
    for (var col = cx - dI + 1; col <= cx + dI; col++) {
      placeBit(col, cy - dO);
      placeBit(col, cy - dI);
    }
    // Right edge: top to bottom.
    for (var row = cy - dI + 1; row <= cy + dI; row++) {
      placeBit(cx + dO, row);
      placeBit(cx + dI, row);
    }
    // Bottom edge: right to left.
    for (var col = cx + dI; col > cx - dI; col--) {
      placeBit(col, cy + dO);
      placeBit(col, cy + dI);
    }
    // Left edge: bottom to top.
    for (var row = cy + dI; row > cy - dI; row--) {
      placeBit(cx - dO, row);
      placeBit(cx - dI, row);
    }
  }
}

// ============================================================================
// Main encode function
// ============================================================================

/// Encode [data] as an Aztec Code symbol.
///
/// Returns a [ModuleGrid] where `modules[row][col]` is `true` for a dark
/// module.  The grid origin (0, 0) is the top-left corner.
///
/// ## Encoding steps
///
/// 1. Encode the input via Binary-Shift from Upper mode (5-bit escape +
///    length + 8-bit bytes).
/// 2. Select the smallest symbol satisfying the requested ECC level
///    (compact 1→4, then full 1→32).
/// 3. Pad the data codeword sequence to exactly [dataCwCount] bytes.
/// 4. Compute GF(256)/0x12D Reed-Solomon ECC bytes.
/// 5. Apply bit stuffing to the combined data+ECC bit stream.
/// 6. Compute the GF(16) mode message (28 or 40 bits).
/// 7. Initialise the grid: reference grid (full only) → bullseye →
///    orientation marks + mode message.
/// 8. Place data+ECC bits in the clockwise layer spiral.
///
/// ## Parameters
///
/// - [data] — Input string (encoded as UTF-8).
/// - [options] — Optional [AztecOptions].  Defaults to `minEccPercent=23`.
///
/// ## Errors
///
/// Throws [InputTooLongError] if the data exceeds the maximum symbol
/// capacity (~1914 bytes at 23% ECC in a 32-layer full symbol).
///
/// ## Example
///
/// ```dart
/// final grid = encode('IATA BP DATA');
/// print(grid.rows);  // 19 (compact-2 symbol)
///
/// final opts = const AztecOptions(minEccPercent: 33);
/// final grid2 = encode('HELLO', options: opts);
/// ```
ModuleGrid encode(String data, {AztecOptions options = const AztecOptions()}) {
  final minEccPct = options.minEccPercent;
  final inputBytes = data.codeUnits; // UTF-16 code units (ASCII-safe for typical data)

  // Step 1: encode data into a bit stream via Binary-Shift.
  final dataBits = _encodeBytesAsBits(inputBytes);

  // Step 2: pick the smallest symbol that fits.
  final spec = _selectSymbol(dataBits.length, minEccPct);
  final compact = spec.compact;
  final layers = spec.layers;
  final dataCwCount = spec.dataCwCount;
  final eccCwCount = spec.eccCwCount;

  // Step 3: pad the bit stream up to dataCwCount whole bytes.
  final paddedBits = _padToBytes(dataBits, dataCwCount);

  final dataBytes = <int>[];
  for (var i = 0; i < dataCwCount; i++) {
    var byte = 0;
    for (var b = 0; b < 8; b++) {
      byte = (byte << 1) | paddedBits[i * 8 + b];
    }
    // All-zero codeword avoidance: per ISO/IEC 24778:2008 §7.3.1.1 the last
    // data codeword cannot be 0x00 (it would collide with the RS padding
    // sentinel).  Substitute 0xFF in that case.
    if (byte == 0 && i == dataCwCount - 1) byte = 0xFF;
    dataBytes.add(byte);
  }

  // Step 4: compute Reed-Solomon ECC bytes.
  final eccBytes = _gf256RsEncode(dataBytes, eccCwCount);

  // Step 5: build the combined bit stream and apply bit stuffing.
  final allBytes = [...dataBytes, ...eccBytes];
  final rawBits = <int>[];
  for (final byte in allBytes) {
    for (var i = 7; i >= 0; i--) {
      rawBits.add((byte >> i) & 1);
    }
  }
  final stuffedBits = _stuffBits(rawBits);

  // Step 6: build the mode message.
  final modeMsg = _encodeModeMessage(compact, layers, dataCwCount);

  // Step 7: initialise the grid.
  final size = _symbolSize(compact, layers);
  final cx = size ~/ 2;
  final cy = size ~/ 2;

  final modules = List.generate(size, (_) => List<bool>.filled(size, false));
  final reserved = List.generate(size, (_) => List<bool>.filled(size, false));

  // Reference grid first (full symbols only).  The bullseye overwrites the
  // central section, but outer reference modules survive.
  if (!compact) {
    _drawReferenceGrid(modules, reserved, cx, cy, size);
  }
  _drawBullseye(modules, reserved, cx, cy, compact);

  final modeRingRemaining = _drawOrientationAndModeMessage(
    modules, reserved, cx, cy, compact, modeMsg,
  );

  // Step 8: place the data spiral.
  _placeDataBits(
    modules, reserved, stuffedBits, cx, cy, compact, layers, modeRingRemaining,
  );

  // Wrap in an immutable ModuleGrid and return.
  return ModuleGrid(
    rows: size,
    cols: size,
    modules: modules,
    moduleShape: ModuleShape.square,
  );
}

// ============================================================================
// Convenience wrappers
// ============================================================================

/// Convert a [ModuleGrid] to a [PaintScene] via barcode-2d's [layout] function.
///
/// Aztec needs no large quiet zone (the bullseye serves as a self-contained
/// locator), but a small quiet zone improves scanner ergonomics.
/// The default [Barcode2DLayoutConfig] uses a 4-module quiet zone.
PaintScene layoutGrid(ModuleGrid grid, {Barcode2DLayoutConfig? config}) {
  final cfg = config ??
      const Barcode2DLayoutConfig(
        moduleSizePx: 10,
        quietZoneModules: 4,
        foreground: '#000000',
        background: '#ffffff',
        moduleShape: ModuleShape.square,
      );
  return layout(grid, config: cfg);
}

/// Encode [data] and convert the resulting grid to a [PaintScene].
///
/// Convenience wrapper: calls [encode] then [layoutGrid] in one step.
///
/// ```dart
/// final scene = encodeAndLayout('HELLO');
/// ```
PaintScene encodeAndLayout(
  String data, {
  AztecOptions options = const AztecOptions(),
  Barcode2DLayoutConfig? config,
}) {
  final grid = encode(data, options: options);
  return layoutGrid(grid, config: config);
}

/// Encode [data] and return an [AnnotatedModuleGrid].
///
/// In v0.1.0 annotations are not yet populated; the function returns the grid
/// wrapped in an annotated container with no per-module roles.  Full
/// annotation support (highlighting bullseye, mode message, data spiral, ECC
/// regions) is planned for v0.2.0.
AnnotatedModuleGrid explain(
  String data, {
  AztecOptions options = const AztecOptions(),
}) {
  final grid = encode(data, options: options);
  // Build a rows×cols matrix of null annotations (no roles populated yet).
  final annotations = List.generate(
    grid.rows,
    (_) => List<ModuleAnnotation?>.filled(grid.cols, null),
  );
  return AnnotatedModuleGrid(
    rows: grid.rows,
    cols: grid.cols,
    modules: grid.modules,
    moduleShape: grid.moduleShape,
    annotations: annotations,
  );
}
