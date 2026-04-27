/// GF(256) Galois Field arithmetic for Reed-Solomon error correction.
///
/// This library implements GF(2^8) using the Reed-Solomon primitive polynomial
/// x^8 + x^4 + x^3 + x^2 + 1 = 0x11D. It provides:
///
///   - [gfAdd] / [gfSubtract] — XOR (same operation in characteristic 2)
///   - [gfMultiply] / [gfDivide] — log/antilog table lookups (O(1))
///   - [gfPower] — exponentiation using the log table
///   - [gfInverse] — multiplicative inverse
///   - [alog] / [log] — the raw lookup tables (for Reed-Solomon internals)
///
/// ## Usage
///
/// ```dart
/// import 'package:coding_adventures_gf256/coding_adventures_gf256.dart';
///
/// void main() {
///   // Addition in GF(256) is XOR
///   print(gfAdd(0x53, 0xCA));   // → 153 (0x99)
///
///   // Every element is its own additive inverse
///   print(gfAdd(42, 42));       // → 0
///
///   // Multiplicative inverse
///   print(gfMultiply(0x53, gfInverse(0x53)));  // → 1
/// }
/// ```
library coding_adventures_gf256;

export 'src/gf256.dart';
