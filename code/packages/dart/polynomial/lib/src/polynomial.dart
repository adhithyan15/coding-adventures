/// Coefficient-array polynomial arithmetic.
///
/// A polynomial is represented as a `List<num>` where **the index equals the
/// degree** of the corresponding term:
///
/// ```
/// [3, 0, 2]  →  3 + 0·x + 2·x²  =  3 + 2x²
/// [1, 2, 3]  →  1 + 2x + 3x²
/// []         →  the zero polynomial
/// ```
///
/// This **little-endian** representation (constant term first) makes addition
/// trivially position-aligned and keeps Horner's method natural to implement.
///
/// ## Normalization
///
/// All operations produce **normalized** polynomials — trailing zeros (high-
/// degree coefficients that are zero) are stripped:
///
/// ```
/// normalize([1, 0, 0]) → [1]   (constant polynomial 1)
/// normalize([0])       → []    (zero polynomial)
/// ```
///
/// ## Why This Package Exists
///
/// Polynomial arithmetic underpins three layers in the coding-adventures stack:
///
///   1. **GF(2^8) arithmetic (MA01)** — Every element of GF(256) is a
///      polynomial over GF(2). The field is the polynomial ring modulo an
///      irreducible degree-8 polynomial.
///
///   2. **Reed-Solomon error correction (MA02)** — RS encoding is polynomial
///      multiplication; decoding uses the Euclidean GCD algorithm.
///
///   3. **CRCs** — A CRC is the remainder after polynomial division modulo a
///      generator polynomial over GF(2).
library polynomial;

/// Polynomial type: a list where index `i` holds the coefficient of `x^i`.
///
/// The zero polynomial is the empty list `[]`.
typedef Polynomial = List<num>;

/// Package version.
const String version = '0.1.0';

// =============================================================================
// Fundamentals
// =============================================================================

/// Remove trailing zeros from a polynomial.
///
/// Trailing zeros represent zero-coefficient high-degree terms. Stripping them
/// ensures that:
///   - Two polynomials that are mathematically equal compare as equal lists.
///   - `degree(p)` correctly finds the highest *non-zero* term.
///   - The polynomial long division loop terminates correctly.
///
/// Examples:
/// ```dart
/// normalize([1, 0, 0]) // → [1]
/// normalize([0])       // → []
/// normalize([1, 2, 3]) // → [1, 2, 3] (unchanged)
/// ```
Polynomial normalize(Polynomial p) {
  int len = p.length;
  // Walk backwards until we find a non-zero coefficient.
  while (len > 0 && p[len - 1] == 0) {
    len--;
  }
  return p.sublist(0, len);
}

/// Return the degree of a polynomial.
///
/// The degree is the index of the highest non-zero coefficient.
///
/// By convention, the **zero polynomial has degree -1**. This sentinel value
/// makes polynomial long division terminate cleanly: the loop condition
/// `degree(remainder) >= degree(divisor)` becomes false when remainder is zero.
///
/// Examples:
/// ```dart
/// degree([3, 0, 2]) // → 2   (highest non-zero: index 2)
/// degree([7])       // → 0   (constant polynomial)
/// degree([])        // → -1  (zero polynomial)
/// degree([0, 0])    // → -1  (normalizes to []; same as zero polynomial)
/// ```
int polynomialDegree(Polynomial p) {
  final n = normalize(p);
  return n.length - 1; // -1 when n is empty (zero polynomial)
}

/// Return the zero polynomial [].
///
/// Zero is the additive identity: add(zero(), p) = p for any polynomial p.
/// It has degree -1 by convention.
Polynomial polynomialZero() => <num>[];

/// Return the multiplicative identity polynomial [1].
///
/// Multiplying any polynomial by one() returns that polynomial unchanged.
Polynomial polynomialOne() => <num>[1];

// =============================================================================
// Addition and Subtraction
// =============================================================================

/// Add two polynomials term-by-term.
///
/// Addition adds matching coefficients. If the polynomials have different
/// lengths, the shorter one is padded with implicit zeros.
///
/// Visual example:
/// ```
///   [1, 2, 3]   =  1 + 2x + 3x²
/// + [4, 5]      =  4 + 5x
/// ─────────────
///   [5, 7, 3]   =  5 + 7x + 3x²
/// ```
///
/// The result is normalized (trailing zeros stripped).
Polynomial polynomialAdd(Polynomial a, Polynomial b) {
  final int len = a.length > b.length ? a.length : b.length;
  final result = List<num>.filled(len, 0);

  for (int i = 0; i < len; i++) {
    final num ai = i < a.length ? a[i] : 0;
    final num bi = i < b.length ? b[i] : 0;
    result[i] = ai + bi;
  }

  return normalize(result);
}

/// Subtract polynomial b from polynomial a term-by-term.
///
/// Equivalent to add(a, negate(b)), but done in one pass without allocating
/// an intermediate negated polynomial.
///
/// Visual example:
/// ```
///   [5, 7, 3]   =  5 + 7x + 3x²
/// - [1, 2, 3]   =  1 + 2x + 3x²
/// ─────────────
///   [4, 5, 0]   →  normalize  →  [4, 5]
/// ```
///
/// Note: 3x² - 3x² = 0; normalize strips the trailing zero.
Polynomial polynomialSubtract(Polynomial a, Polynomial b) {
  final int len = a.length > b.length ? a.length : b.length;
  final result = List<num>.filled(len, 0);

  for (int i = 0; i < len; i++) {
    final num ai = i < a.length ? a[i] : 0;
    final num bi = i < b.length ? b[i] : 0;
    result[i] = ai - bi;
  }

  return normalize(result);
}

// =============================================================================
// Multiplication
// =============================================================================

/// Multiply two polynomials using polynomial convolution.
///
/// Each term `a[i]·xⁱ` of a multiplies each term `b[j]·xʲ` of b, adding
/// `a[i]·b[j]` to the result at index `i+j`.
///
/// If a has degree m and b has degree n, the result has degree m+n, so the
/// result array has length `a.length + b.length - 1`.
///
/// Visual example:
/// ```
///   [1, 2]  =  1 + 2x
/// × [3, 4]  =  3 + 4x
/// ──────────────────────────
/// result = [0, 0, 0] (length 3)
///   i=0, j=0: result[0] += 1·3 = 3   → [3, 0, 0]
///   i=0, j=1: result[1] += 1·4 = 4   → [3, 4, 0]
///   i=1, j=0: result[1] += 2·3 = 6   → [3, 10, 0]
///   i=1, j=1: result[2] += 2·4 = 8   → [3, 10, 8]
///
/// Result: [3, 10, 8]  =  3 + 10x + 8x²
/// Verify: (1+2x)(3+4x) = 3+4x+6x+8x² = 3+10x+8x²  ✓
/// ```
Polynomial polynomialMultiply(Polynomial a, Polynomial b) {
  // Multiplying by the zero polynomial yields zero.
  if (a.isEmpty || b.isEmpty) return <num>[];

  // Result degree = deg(a) + deg(b), length = a.length + b.length - 1.
  final resultLen = a.length + b.length - 1;
  final result = List<num>.filled(resultLen, 0);

  for (int i = 0; i < a.length; i++) {
    for (int j = 0; j < b.length; j++) {
      result[i + j] += a[i] * b[j];
    }
  }

  return normalize(result);
}

// =============================================================================
// Division
// =============================================================================

/// Perform polynomial long division, returning `[quotient, remainder]`.
///
/// Given polynomials a and b (b ≠ zero), finds q and r such that:
/// ```
/// a = b × q + r   and   degree(r) < degree(b)
/// ```
///
/// The algorithm is the polynomial analog of school long division:
///   1. Find the leading term of the current remainder.
///   2. Divide it by the leading term of b to get the next quotient term.
///   3. Subtract (quotient term) × b from the remainder.
///   4. Repeat until degree(remainder) < degree(b).
///
/// Detailed example: divide `[5, 1, 3, 2]` (5 + x + 3x² + 2x³) by `[2, 1]` (2 + x):
/// ```
/// Step 1: remainder = [5,1,3,2], deg=3.  Quotient term: 2x³/x = 2x² → q[2]=2
///         Subtract 2x²·(2+x) = 4x²+2x³ = [0,0,4,2]:
///         [5,1,3-4,2-2] = [5,1,-1]
///
/// Step 2: remainder = [5,1,-1], deg=2.  Quotient term: -x²/x = -x → q[1]=-1
///         Subtract -x·(2+x) = -2x-x² = [0,-2,-1]:
///         [5,3,0] → [5,3]
///
/// Step 3: remainder = [5,3], deg=1.  Quotient term: 3x/x = 3 → q[0]=3
///         Subtract 3·(2+x) = 6+3x = [6,3]:
///         [-1,0] → [-1]
///
/// Step 4: degree([-1]) = 0 < 1 = degree(b). STOP.
/// Result: quotient=[3,-1,2], remainder=[-1]
/// Verify: (x+2)(3-x+2x²)+(-1) = 3x-x²+2x³+6-2x+4x²-1 = 5+x+3x²+2x³ ✓
/// ```
///
/// Throws [ArgumentError] if b is the zero polynomial.
(Polynomial, Polynomial) polynomialDivmod(Polynomial a, Polynomial b) {
  final nb = normalize(b);
  if (nb.isEmpty) {
    throw ArgumentError('polynomial division by zero');
  }

  final na = normalize(a);
  final int degA = na.length - 1;
  final int degB = nb.length - 1;

  // If a has lower degree than b, quotient is 0 and remainder is a.
  if (degA < degB) {
    return (<num>[], List<num>.from(na));
  }

  // Work on a mutable copy of the remainder.
  final rem = List<num>.from(na);
  // Allocate the quotient array with the right size.
  final quot = List<num>.filled(degA - degB + 1, 0);

  // Leading coefficient of the divisor.
  final num leadB = nb[degB];

  // Current degree of the remainder (walks down as we subtract terms).
  int degRem = degA;

  while (degRem >= degB) {
    // Leading coefficient and power of the current quotient term.
    final num leadRem = rem[degRem];
    final num coeff = leadRem / leadB;
    final int power = degRem - degB;
    quot[power] = coeff;

    // Subtract coeff·x^power·b from rem.
    for (int j = 0; j <= degB; j++) {
      rem[power + j] -= coeff * nb[j];
    }

    // The leading term is now zero by construction. Move to next non-zero.
    degRem--;
    while (degRem >= 0 && rem[degRem] == 0) {
      degRem--;
    }
  }

  return (normalize(quot), normalize(rem));
}

/// Return the quotient of [polynomialDivmod](a, b).
///
/// Throws [ArgumentError] if b is the zero polynomial.
Polynomial polynomialDivide(Polynomial a, Polynomial b) =>
    polynomialDivmod(a, b).$1;

/// Return the remainder of [polynomialDivmod](a, b).
///
/// This is the polynomial modulo operation. In GF(2^8) construction, we
/// reduce a high-degree polynomial modulo the primitive polynomial using this.
///
/// Throws [ArgumentError] if b is the zero polynomial.
Polynomial polynomialMod(Polynomial a, Polynomial b) =>
    polynomialDivmod(a, b).$2;

// =============================================================================
// Evaluation
// =============================================================================

/// Evaluate a polynomial at `x` using Horner's method.
///
/// Horner's method rewrites the polynomial in nested form:
/// ```
/// a₀ + x(a₁ + x(a₂ + ... + x·aₙ))
/// ```
///
/// This requires only n additions and n multiplications — no powers of x.
/// It is more numerically stable and faster than the naive approach.
///
/// Algorithm (reading from high degree to low):
/// ```
/// acc = 0
/// for i from n downto 0:
///     acc = acc * x + p[i]
/// return acc
/// ```
///
/// Example: evaluate `[3, 1, 2]` = 3 + x + 2x² at x = 4:
/// ```
/// acc = 0
/// i=2: acc = 0*4 + 2 = 2
/// i=1: acc = 2*4 + 1 = 9
/// i=0: acc = 9*4 + 3 = 39
/// Verify: 3 + 4 + 2·16 = 39  ✓
/// ```
num polynomialEvaluate(Polynomial p, num x) {
  final n = normalize(p);
  if (n.isEmpty) return 0; // Zero polynomial evaluates to 0 everywhere.

  num acc = 0;
  // Iterate from high-degree term down to the constant term.
  for (int i = n.length - 1; i >= 0; i--) {
    acc = acc * x + n[i];
  }
  return acc;
}

// =============================================================================
// Greatest Common Divisor
// =============================================================================

/// Compute the greatest common divisor of two polynomials.
///
/// Uses the Euclidean algorithm: repeatedly replace (a, b) with (b, a mod b)
/// until b is the zero polynomial. The last non-zero remainder is the GCD.
///
/// This is identical to the integer GCD algorithm, with polynomial mod in place
/// of integer mod.
///
/// ```
/// while b ≠ zero:
///     a, b = b, a mod b
/// return normalize(a)
/// ```
///
/// Use case: GCD is used in Reed-Solomon decoding (extended Euclidean
/// algorithm) to find the error-locator and error-evaluator polynomials.
Polynomial polynomialGcd(Polynomial a, Polynomial b) {
  Polynomial u = normalize(a);
  Polynomial v = normalize(b);

  while (v.isNotEmpty) {
    final r = polynomialMod(u, v);
    u = v;
    v = r;
  }

  return normalize(u);
}
