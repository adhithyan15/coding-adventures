/// Galois Field GF(2^8) arithmetic.
///
/// GF(256) is the finite field with 256 elements. The elements are the
/// integers 0..255. Arithmetic uses the primitive polynomial:
///
///   p(x) = x^8 + x^4 + x^3 + x^2 + 1  =  0x11D  =  285
///
/// ## Why GF(256) Exists
///
/// Finite fields are the mathematical foundation of error-correcting codes
/// and some encryption algorithms:
///
///   - **Reed-Solomon** codes (QR codes, CDs, DVDs, hard drives) perform
///     polynomial arithmetic over GF(256). Each byte is a field element.
///   - **AES encryption** uses GF(2^8) with a different primitive polynomial
///     (0x11B) for the SubBytes and MixColumns steps.
///
/// ## Key Insight: Addition Is XOR
///
/// In GF(2^8) (characteristic 2), `1 + 1 = 0`. This means every element is
/// its own additive inverse, so **subtraction equals addition**, and both
/// equal **XOR**. No carry, no overflow, no tables needed for add/subtract.
///
/// ## Multiplication via Log/Antilog Tables
///
/// Multiplication is implemented using precomputed lookup tables:
///
///   a × b = ALOG[(LOG[a] + LOG[b]) mod 255]
///
/// This turns multiplication into two table lookups and one modular addition —
/// much faster than polynomial long division at every call.
///
/// ## The Primitive Polynomial
///
/// The 256 elements of GF(2^8) are polynomials over GF(2) of degree ≤ 7:
///
///   a₇x⁷ + a₆x⁶ + ... + a₁x + a₀
///
/// Multiplication of two such polynomials can produce degree > 7, so we
/// reduce modulo an irreducible polynomial of degree 8:
///
///   p(x) = x^8 + x^4 + x^3 + x^2 + 1 = 0x11D = 285
///
/// This polynomial is **primitive**: the element g = 2 (= x) generates the
/// entire multiplicative group. That is, g^0, g^1, ..., g^254 are exactly
/// the 255 non-zero elements of GF(256).
library gf256;

/// The primitive (irreducible) polynomial for Reed-Solomon GF(2^8).
///
/// p(x) = x^8 + x^4 + x^3 + x^2 + 1
/// Binary: 1_0001_1101 = 0x11D = 285
///
/// This is the standard polynomial for QR codes and most RS implementations.
/// AES uses 0x11B instead, but this library targets RS/QR.
const int primitivePolynomial = 0x11D;

/// Package version.
const String version = '0.1.0';

// =============================================================================
// Log/Antilog Table Construction
// =============================================================================
//
// Two lookup tables are built once at program start:
//
//   _alog[i] = 2^i mod p(x)   (antilogarithm: discrete exponentiation)
//   _log[x]  = i such that 2^i = x   (logarithm)
//
// Construction algorithm:
//   Start with value = 1. At each step, multiply by 2 (shift left 1 bit).
//   If bit 8 is set (overflow), XOR with 0x11D to reduce modulo p(x).
//
// Why does shift-left = multiply by 2?
//   In GF(2^8), the element "2" is the polynomial x (bit 1 set, all others 0).
//   Multiplying any polynomial f(x) by x shifts all coefficients up one degree.
//   That is exactly a 1-bit left shift. When degree 8 appears (overflow), we
//   reduce by XOR-ing with p(x) = 0x11D (since x^8 ≡ x^4 + x^3 + x^2 + 1).

final List<int> _alog = List<int>.filled(256, 0);
final List<int> _log = List<int>.filled(256, 0);

/// Whether the tables have been initialized.
bool _tablesBuilt = false;

/// Build the log and antilog tables.
///
/// This is called lazily on the first use of any field operation. Tables are
/// built exactly once and reused for all subsequent calls.
///
/// After building:
///   - _alog[0..254] maps exponent → field element
///   - _alog[255] = 1 (the multiplicative group wraps: g^255 = g^0 = 1)
///   - _log[1..255] maps field element → exponent
///   - _log[0] = 0 (unused; zero has no logarithm)
void _buildTables() {
  if (_tablesBuilt) return;

  int val = 1;
  for (int i = 0; i < 255; i++) {
    _alog[i] = val;
    _log[val] = i;

    // Multiply val by 2 (the generator g = x in GF(2^8)).
    val <<= 1;
    // If bit 8 is set, the product overflowed a byte. Reduce modulo p(x)
    // by XOR-ing with the primitive polynomial. This is equivalent to
    // subtracting x^8 (= x^8 mod p(x) = x^4 + x^3 + x^2 + 1 = 0x1D)
    // but in GF(2), subtraction = addition = XOR.
    if (val >= 256) {
      val ^= primitivePolynomial;
    }
  }

  // _alog[255] = 1: the multiplicative group has order 255, so g^255 = g^0 = 1.
  // This entry is needed so that inverse(1) = _alog[255 - _log[1]] = _alog[255] = 1.
  _alog[255] = 1;
  // _log[0] remains 0; it is never accessed for valid inputs.

  _tablesBuilt = true;
}

/// Antilogarithm table: alog[i] = 2^i in GF(256).
///
/// Maps discrete exponent i in {0..255} to the field element 2^i.
/// Notable entries:
///   alog[0]  = 1    (2^0 = 1)
///   alog[1]  = 2    (2^1 = 2)
///   alog[7]  = 128  (2^7 = 0x80)
///   alog[8]  = 29   (256 XOR 0x11D = 0x1D = 29; first reduction)
///   alog[255]= 1    (multiplicative group wraps: 2^255 = 1)
List<int> get alog {
  _buildTables();
  return List<int>.unmodifiable(_alog);
}

/// Logarithm table: log[x] = i such that 2^i = x in GF(256).
///
/// The inverse of alog. log[0] is undefined (0 is not a power of any element).
/// For x in 1..255: alog[log[x]] = x.
List<int> get log {
  _buildTables();
  return List<int>.unmodifiable(_log);
}

// =============================================================================
// Field Operations
// =============================================================================

/// Add two GF(256) elements.
///
/// In characteristic-2 fields, addition is XOR. Each bit represents a GF(2)
/// coefficient, and GF(2) addition satisfies 1 + 1 = 0 (mod 2).
///
/// No overflow, no carry, no tables needed.
///
/// Examples:
///   add(0x53, 0xCA) = 0x53 ^ 0xCA = 0x99
///   add(x, x) = 0 for all x  (every element is its own additive inverse)
///
/// This is also subtraction — in characteristic 2, -1 = 1, so x - y = x + y.
int gfAdd(int a, int b) => a ^ b;

/// Subtract two GF(256) elements.
///
/// Subtraction equals addition in characteristic 2 (since -1 = 1 means every
/// element is its own negation). Both operations are XOR.
///
/// This simplifies error-correction algorithms: syndrome computation via
/// subtraction uses the same operation as addition.
int gfSubtract(int a, int b) => a ^ b;

/// Multiply two GF(256) elements using log/antilog tables.
///
/// Uses the identity: a × b = g^(log_g(a) + log_g(b))
/// where g = 2 is the generator.
///
/// Special case: if either operand is 0, the result is 0. Zero has no
/// logarithm (it is not reachable as a power of any element), so we must
/// handle it explicitly.
///
/// The modular addition (log[a] + log[b]) % 255 keeps the exponent within
/// the cyclic group of order 255.
///
/// Time complexity: O(1) — two table lookups and one addition.
///
/// Example:
///   multiply(2, 4) = alog[(log[2] + log[4]) % 255] = alog[(1 + 2) % 255]
///                  = alog[3] = 8   ✓ (2 × 4 = 8 in ordinary arithmetic too,
///                                     since neither value overflows a byte)
int gfMultiply(int a, int b) {
  _buildTables();
  // The product of anything with zero is zero.
  if (a == 0 || b == 0) return 0;
  return _alog[(_log[a] + _log[b]) % 255];
}

/// Divide a by b in GF(256).
///
/// a / b = g^(log_g(a) - log_g(b)) = alog[(log[a] - log[b] + 255) % 255]
///
/// The `+ 255` before `% 255` ensures a non-negative result when
/// log[a] < log[b]. Without it, Dart's `%` could return a negative number.
///
/// Special case: a = 0 → result is 0 (0 / anything = 0).
///
/// Throws [ArgumentError] if b is 0 (division by zero is undefined).
int gfDivide(int a, int b) {
  _buildTables();
  if (b == 0) throw ArgumentError('GF256: division by zero');
  if (a == 0) return 0;
  return _alog[(_log[a] - _log[b] + 255) % 255];
}

/// Raise a GF(256) element to a non-negative integer power.
///
/// Uses the log table: base^exp = alog[(log[base] * exp) % 255]
///
/// The modulo 255 reflects the order of the multiplicative group: every
/// non-zero element satisfies g^255 = 1 (Fermat's little theorem for
/// finite fields, since the group has order 255).
///
/// Special cases:
///   0^0 = 1 by convention (consistent with most numeric libraries)
///   0^n = 0 for n > 0
///
/// Throws [ArgumentError] if exp is negative.
int gfPower(int base, int exp) {
  _buildTables();
  if (exp < 0) throw ArgumentError('GF256: exponent must be non-negative');
  if (base == 0) return exp == 0 ? 1 : 0;
  if (exp == 0) return 1;
  // Use modular arithmetic since the multiplicative group has order 255.
  // We also add 255 before modulo to handle the case where log[base] * exp
  // is a multiple of 255 (which would give 0, mapping to _alog[0] = 1 — correct).
  return _alog[((_log[base] * exp) % 255 + 255) % 255];
}

/// Compute the multiplicative inverse of a GF(256) element.
///
/// The inverse of a satisfies: a × inverse(a) = 1.
///
/// Derivation using the cyclic group:
///   a × a^(-1) = 1 = g^0 = g^255
///   So log(a) + log(a^(-1)) ≡ 0 (mod 255)
///   Therefore log(a^(-1)) = 255 - log(a)
///   And a^(-1) = alog[255 - log[a]]
///
/// This operation is fundamental to Reed-Solomon decoding (Forney's algorithm)
/// and AES SubBytes.
///
/// Throws [ArgumentError] if a is 0 (zero has no multiplicative inverse).
int gfInverse(int a) {
  _buildTables();
  if (a == 0) throw ArgumentError('GF256: zero has no multiplicative inverse');
  return _alog[255 - _log[a]];
}

/// Return the additive identity (zero element).
int gfZero() => 0;

/// Return the multiplicative identity (one element).
int gfOne() => 1;
