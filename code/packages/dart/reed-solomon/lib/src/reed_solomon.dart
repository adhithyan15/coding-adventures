/// Reed-Solomon error-correcting codes over GF(256).
///
/// Reed-Solomon is a block error-correcting code: you add `nCheck` redundancy
/// bytes to a message, and the decoder can recover the original message even
/// if up to `t = nCheck / 2` bytes are corrupted.
///
/// ## Where RS Is Used
///
/// | System | How RS Helps |
/// |--------|-------------|
/// | QR codes | Up to 30% of a QR symbol can be damaged and still decode |
/// | CDs / DVDs | CIRC two-level RS corrects scratches |
/// | Hard drives | Firmware error correction for sector-level faults |
/// | Voyager probes | Transmit images across 20+ billion km |
/// | RAID-6 | Two parity drives are an (n, n-2) RS code over GF(256) |
///
/// ## Building Blocks
///
/// ```
/// MA00  polynomial   — coefficient-array polynomial arithmetic
/// MA01  gf256        — GF(2^8) field arithmetic (add=XOR, mul=table lookup)
/// MA02  reed-solomon ← THIS PACKAGE
/// ```
///
/// ## Quick Start
///
/// ```dart
/// import 'package:coding_adventures_reed_solomon/coding_adventures_reed_solomon.dart';
///
/// final message = Uint8List.fromList([72, 101, 108, 108, 111]); // "Hello"
/// const nCheck = 8; // t = 4 errors correctable
///
/// final codeword = rsEncode(message, nCheck);
/// // codeword[0..4] == message (systematic: message appears unchanged)
///
/// // Corrupt 3 bytes — still recoverable (3 ≤ t = 4)
/// codeword[0] ^= 0xFF;
/// codeword[2] ^= 0xAA;
/// codeword[4] ^= 0x55;
///
/// final recovered = rsDecode(codeword, nCheck);
/// // recovered == message
/// ```
///
/// ## Polynomial Convention
///
/// All codeword bytes are treated as a **big-endian** polynomial:
/// ```
/// codeword[0]·x^{n-1} + codeword[1]·x^{n-2} + ... + codeword[n-1]
/// ```
///
/// The systematic codeword layout:
/// ```
/// [ message[0] ... message[k-1] | check[0] ... check[nCheck-1] ]
///   degree n-1 ... degree nCheck    degree nCheck-1 ... degree 0
/// ```
library reed_solomon;

import 'dart:typed_data';
import 'package:coding_adventures_gf256/coding_adventures_gf256.dart';

/// Package version.
const String version = '0.1.0';

// =============================================================================
// Exceptions
// =============================================================================

/// Thrown when decoding fails because more than `t = nCheck/2` errors occurred.
///
/// The codeword is too badly corrupted to recover. The caller must handle this
/// by requesting a retransmission or treating the data as unrecoverable.
class TooManyErrorsException implements Exception {
  const TooManyErrorsException();

  @override
  String toString() =>
      'ReedSolomon: too many errors — codeword is unrecoverable';
}

/// Thrown when [rsEncode] or [rsDecode] receives invalid parameters.
class InvalidInputException implements Exception {
  final String message;
  const InvalidInputException(this.message);

  @override
  String toString() => 'ReedSolomon: invalid input — $message';
}

// =============================================================================
// Generator Polynomial
// =============================================================================

/// Build the RS generator polynomial for a given number of check bytes.
///
/// The generator is the product of `nCheck` linear factors:
/// ```
/// g(x) = (x + α¹)(x + α²)...(x + α^{nCheck})
/// ```
///
/// where `α = 2` is the primitive element of GF(256).
///
/// ## Return Format
///
/// A **little-endian** `Uint8List` of length `nCheck + 1`. Index `i` holds the
/// coefficient of `x^i`. The last element is always `1` (monic polynomial).
///
/// ## Construction Algorithm
///
/// Start with `g = [1]`. For each `i` from 1 to nCheck, multiply in the
/// factor `(x + αⁱ)`:
///
/// ```
/// new_g[j] = GF256.mul(αⁱ, g[j]) XOR g[j-1]
/// ```
///
/// This is the polynomial multiplication `[αⁱ, 1] × g` in GF(256):
///   - The `αⁱ` coefficient of the factor multiplies each `g[j]` → contributes to `new_g[j]`
///   - The `1` coefficient of the factor shifts `g[j]` up one → contributes to `new_g[j+1]`
///   - In GF(256), addition is XOR, so we XOR both contributions
///
/// ## Example (nCheck = 2)
///
/// ```
/// Start: g = [1]
/// i=1, α¹=2:  g = [mul(2,1) ^ 0, mul(2,0) ^ 1] wait...
///   new_g[0] = mul(2, g[0]) = mul(2, 1) = 2
///   new_g[1] = g[0] = 1
///   g = [2, 1]   (2 + x)
///
/// i=2, α²=4:
///   new_g[0] = mul(4, 2) = 8
///   new_g[1] = mul(4, 1) ^ 2 = 4 ^ 2 = 6
///   new_g[2] = 1
///   g = [8, 6, 1]   (8 + 6x + x²)
///
/// Verify root α¹=2: g(2) = 8 XOR mul(6,2) XOR mul(1,4) = 8 XOR 12 XOR 4 = 0 ✓
/// ```
///
/// Throws [InvalidInputException] if nCheck is 0 or odd.
Uint8List rsBuildGenerator(int nCheck) {
  if (nCheck == 0 || nCheck % 2 != 0) {
    throw InvalidInputException(
        'nCheck must be a positive even number, got $nCheck');
  }

  var g = Uint8List.fromList([1]);

  for (int i = 1; i <= nCheck; i++) {
    final int alphaI = gfPower(2, i);
    final newG = Uint8List(g.length + 1);
    for (int j = 0; j < g.length; j++) {
      newG[j] ^= gfMultiply(g[j], alphaI);
      newG[j + 1] ^= g[j];
    }
    g = newG;
  }

  return g;
}

// =============================================================================
// Internal Polynomial Helpers
// =============================================================================

/// Evaluate a **big-endian** GF(256) polynomial at `x`.
///
/// `p[0]` is the highest-degree coefficient. Horner's method reads left-to-right:
/// ```
/// acc = 0
/// for each byte b in p (highest degree first):
///   acc = acc·x XOR b
/// ```
///
/// Used for syndrome evaluation: `S_j = _polyEvalBE(codeword, α^j)`.
int _polyEvalBE(Uint8List p, int x) {
  int acc = 0;
  for (final int b in p) {
    acc = gfAdd(gfMultiply(acc, x), b);
  }
  return acc;
}

/// Evaluate a **little-endian** GF(256) polynomial at `x`.
///
/// `p[i]` is the coefficient of `x^i`. Horner iterates from high to low degree.
int _polyEvalLE(Uint8List p, int x) {
  int acc = 0;
  for (int i = p.length - 1; i >= 0; i--) {
    acc = gfAdd(gfMultiply(acc, x), p[i]);
  }
  return acc;
}

/// Multiply two **little-endian** GF(256) polynomials (convolution).
///
/// `result[i+j] ^= a[i] · b[j]` for all i, j.
/// This is the polynomial product in GF(256): addition is XOR, multiplication
/// uses the log/antilog tables.
Uint8List _polyMulLE(Uint8List a, Uint8List b) {
  if (a.isEmpty || b.isEmpty) return Uint8List(0);
  final result = Uint8List(a.length + b.length - 1);
  for (int i = 0; i < a.length; i++) {
    for (int j = 0; j < b.length; j++) {
      result[i + j] ^= gfMultiply(a[i], b[j]);
    }
  }
  return result;
}

/// Compute the remainder of **big-endian** polynomial division in GF(256).
///
/// `dividend` and `divisor` are both big-endian (index 0 = highest degree).
/// The divisor must be **monic** (leading coefficient = 1).
///
/// ## Algorithm
///
/// At each step, eliminate the leading term of the current remainder by
/// subtracting a scaled copy of the divisor:
///
/// ```
/// for i = 0 .. (len(dividend) - len(divisor)):
///   coeff = dividend[i]   // monic: coeff = dividend[i] / 1
///   for j = 0 .. len(divisor):
///     dividend[i+j] ^= coeff · divisor[j]
/// ```
///
/// The last `(len(divisor) - 1)` bytes are the remainder.
/// In GF(256), subtraction = addition = XOR.
Uint8List _polyModBE(Uint8List dividend, Uint8List divisor) {
  final rem = Uint8List.fromList(dividend);
  final int divLen = divisor.length;

  if (rem.length < divLen) return rem;

  final int steps = rem.length - divLen + 1;
  for (int i = 0; i < steps; i++) {
    final int coeff = rem[i];
    if (coeff == 0) continue;
    for (int j = 0; j < divLen; j++) {
      rem[i + j] ^= gfMultiply(coeff, divisor[j]);
    }
  }

  return rem.sublist(rem.length - (divLen - 1));
}

/// Compute the inverse locator `X_p⁻¹` for byte position `p` in a codeword
/// of length `n`.
///
/// In big-endian convention, position `p` has degree `n-1-p`.
/// The locator is `X_p = α^{n-1-p}`, so `X_p⁻¹ = α^{(p+256-n) mod 255}`.
///
/// Special cases:
///   - `p = n-1` (last byte): `X_p⁻¹ = α^{255 mod 255} = α^0 = 1`
///   - `p = 0` (first byte): `X_p⁻¹ = α^{(256-n) mod 255}`
int _invLocator(int p, int n) {
  final int exp = (p + 256 - n) % 255;
  return gfPower(2, exp);
}

// =============================================================================
// Encoding
// =============================================================================

/// Encode a message with Reed-Solomon, producing a systematic codeword.
///
/// **Systematic** means the original message bytes are unchanged in the output:
/// ```
/// output = [ message bytes | check bytes ]
///            degree n-1 ... nCheck    degree nCheck-1 ... 0
/// ```
///
/// ## Algorithm
///
/// 1. Build generator `g` (little-endian), reverse to big-endian `gBE`.
/// 2. Append `nCheck` zero bytes: `shifted = message || 000...0`
///    (represents `M(x)·x^{nCheck}` in big-endian).
/// 3. Remainder `R = shifted mod gBE`.
/// 4. Output `message || R` (padded to exactly `nCheck` bytes).
///
/// ## Why It Works
///
/// `C(x) = M(x)·x^{nCheck} XOR R(x)` is exactly divisible by `g(x)` (we
/// subtracted — equivalently XOR'd — the remainder). In GF(256), subtraction
/// is XOR, so:
/// ```
/// C(αⁱ) = Q(αⁱ)·g(αⁱ) = 0   for i = 1..nCheck
/// ```
/// Every valid codeword evaluates to zero at the roots of `g`. The decoder
/// uses this property to detect and correct errors.
///
/// @param message raw data bytes
/// @param nCheck number of check bytes to add (must be even ≥ 2)
/// @returns systematic codeword of length `message.length + nCheck`
/// @throws [InvalidInputException] if nCheck is 0/odd, or total length > 255
Uint8List rsEncode(Uint8List message, int nCheck) {
  if (nCheck == 0 || nCheck % 2 != 0) {
    throw InvalidInputException(
        'nCheck must be a positive even number, got $nCheck');
  }
  final int n = message.length + nCheck;
  if (n > 255) {
    throw InvalidInputException(
        'total codeword length $n exceeds GF(256) block size limit of 255');
  }

  final gLE = rsBuildGenerator(nCheck);
  // Reverse to big-endian for division: g_LE[last]=1 becomes g_BE[0]=1 (monic).
  final gBE = Uint8List.fromList(gLE.reversed.toList());

  // shifted = message || zeros (big-endian representation of M(x)·x^{nCheck})
  final shifted = Uint8List(n);
  shifted.setAll(0, message);
  // Trailing nCheck bytes stay 0.

  final remainder = _polyModBE(shifted, gBE);

  // Codeword = message || check bytes (padded to nCheck bytes).
  final codeword = Uint8List(n);
  codeword.setAll(0, message);
  final int pad = nCheck - remainder.length;
  codeword.setAll(message.length + pad, remainder);

  return codeword;
}

// =============================================================================
// Decoding
// =============================================================================

/// Compute the `nCheck` syndromes of a received codeword.
///
/// `S_j = received(α^j)` for `j = 1, ..., nCheck`.
///
/// If all syndromes are zero, the codeword has no errors. Any non-zero
/// syndrome indicates corruption.
///
/// ## Convention
///
/// The codeword is evaluated as a **big-endian** polynomial: `received[0]`
/// is the highest-degree coefficient. An error at position `p` contributes
/// `e · (α^j)^{n-1-p} = e · X_p^j` where `X_p = α^{n-1-p}`.
///
/// @param received received codeword bytes (possibly corrupted)
/// @param nCheck number of check bytes in the codeword
/// @returns `Uint8List` of `nCheck` syndrome values
Uint8List rsSyndromes(Uint8List received, int nCheck) {
  final s = Uint8List(nCheck);
  for (int i = 1; i <= nCheck; i++) {
    s[i - 1] = _polyEvalBE(received, gfPower(2, i));
  }
  return s;
}

/// Berlekamp-Massey algorithm: find the shortest LFSR generating the syndrome
/// sequence.
///
/// Returns `(lambda, L)` where `lambda` is the **error locator polynomial** in
/// **little-endian** form (lambda[0] = 1) and `L` is the number of errors.
///
/// The LFSR connection polynomial Λ(x) satisfies:
/// ```
/// Λ(x) = ∏_{k=1}^{v} (1 - X_k · x)
/// ```
/// where `X_k` are the error locator numbers. The roots of Λ are `X_k⁻¹`,
/// found via Chien search.
///
/// ## Algorithm
///
/// ```
/// C = [1], B = [1], L = 0, xShift = 1, bScale = 1
///
/// for n = 0 to 2t-1:
///   d = S[n] XOR ∑_{j=1}^{L} C[j]·S[n-j]   ← discrepancy
///
///   if d == 0:
///     xShift++
///   elif 2L ≤ n:          ← more errors than currently modeled: grow Λ
///     T = C.clone()
///     C = C XOR (d/bScale)·x^{xShift}·B
///     L = n+1-L;  B = T;  bScale = d;  xShift = 1
///   else:                  ← consistent update: adjust Λ without growing
///     C = C XOR (d/bScale)·x^{xShift}·B
///     xShift++
/// ```
(Uint8List, int) _berlekampMassey(Uint8List synds) {
  final int twoT = synds.length;

  var c = Uint8List.fromList([1]);
  var b = Uint8List.fromList([1]);
  int bigL = 0;
  int xShift = 1;
  int bScale = 1;

  for (int n = 0; n < twoT; n++) {
    // Compute discrepancy: d = S[n] XOR ∑_{j=1}^{L} C[j]·S[n-j]
    int d = synds[n];
    for (int j = 1; j <= bigL; j++) {
      if (j < c.length && n >= j) {
        d ^= gfMultiply(c[j], synds[n - j]);
      }
    }

    if (d == 0) {
      // Syndrome consistent with current Λ — no update needed.
      xShift++;
    } else if (2 * bigL <= n) {
      // Found more errors than currently modeled: grow Λ.
      final tSave = Uint8List.fromList(c);

      final int scale = gfDivide(d, bScale);
      final int shiftedLen = xShift + b.length;
      if (c.length < shiftedLen) {
        final cNew = Uint8List(shiftedLen);
        cNew.setAll(0, c);
        c = cNew;
      }
      for (int k = 0; k < b.length; k++) {
        c[xShift + k] ^= gfMultiply(scale, b[k]);
      }

      bigL = n + 1 - bigL;
      b = tSave;
      bScale = d;
      xShift = 1;
    } else {
      // Consistent update — adjust Λ without growing.
      final int scale = gfDivide(d, bScale);
      final int shiftedLen = xShift + b.length;
      if (c.length < shiftedLen) {
        final cNew = Uint8List(shiftedLen);
        cNew.setAll(0, c);
        c = cNew;
      }
      for (int k = 0; k < b.length; k++) {
        c[xShift + k] ^= gfMultiply(scale, b[k]);
      }
      xShift++;
    }
  }

  return (c, bigL);
}

/// Chien Search: find which byte positions are error locations.
///
/// Position `p` is an error location if `Λ(X_p⁻¹) = 0`, where
/// `X_p⁻¹ = α^{(p+256-n) mod 255}` for a codeword of length `n`.
///
/// We test every position 0..n-1 and collect the ones where Λ evaluates to zero.
///
/// @returns sorted list of error positions (0-indexed, big-endian)
List<int> _chienSearch(Uint8List lambda, int n) {
  final positions = <int>[];
  for (int p = 0; p < n; p++) {
    final int xiInv = _invLocator(p, n);
    if (_polyEvalLE(lambda, xiInv) == 0) {
      positions.add(p);
    }
  }
  return positions;
}

/// Forney Algorithm: compute error magnitudes from error positions.
///
/// For each error at position `p`:
/// ```
/// e_p = Ω(X_p⁻¹) / Λ'(X_p⁻¹)
/// ```
///
/// where:
///   - `Ω(x) = (S(x) · Λ(x)) mod x^{2t}` — error evaluator polynomial
///   - `S(x) = S₁ + S₂x + ... + S_{2t}x^{2t-1}` — syndrome polynomial (LE)
///   - `Λ'(x)` — formal derivative of Λ in GF(2^8) characteristic 2
///
/// ## Formal Derivative in Characteristic 2
///
/// Only **odd-indexed** coefficients of Λ survive (even terms vanish because
/// 2 = 0 in characteristic 2: the derivative of x^{2k} is 2k·x^{2k-1} = 0):
/// ```
/// Λ'(x) = Λ₁ + Λ₃x² + Λ₅x⁴ + ...
/// ```
///
/// In code: copy Λ[j] for all odd j into Λ'[j-1], set rest to 0.
///
/// @throws [TooManyErrorsException] if the denominator Λ'(X_p⁻¹) = 0
List<int> _forney(
    Uint8List lambda, Uint8List synds, List<int> positions, int n) {
  final int twoT = synds.length;

  // Ω = S(x) · Λ(x) mod x^{2t}: multiply, then truncate to first 2t terms.
  final omegaFull = _polyMulLE(synds, lambda);
  final omega = omegaFull.length > twoT
      ? Uint8List.fromList(omegaFull.sublist(0, twoT))
      : omegaFull;

  // Formal derivative Λ'(x) in GF(2^8):
  // Λ'[j-1] = Λ[j] for odd j; 0 for even j.
  final lambdaPrime = Uint8List(lambda.length > 1 ? lambda.length - 1 : 0);
  for (int j = 1; j < lambda.length; j++) {
    if (j % 2 == 1) {
      // Odd index: this coefficient survives differentiation.
      lambdaPrime[j - 1] ^= lambda[j];
    }
    // Even index: 2·something = 0 in characteristic 2, so it vanishes.
  }

  return positions.map((int pos) {
    final int xiInv = _invLocator(pos, n);
    final int omegaVal = _polyEvalLE(omega, xiInv);
    final int lpVal = _polyEvalLE(lambdaPrime, xiInv);
    if (lpVal == 0) throw const TooManyErrorsException();
    return gfDivide(omegaVal, lpVal);
  }).toList();
}

/// Decode a received Reed-Solomon codeword, correcting up to `t = nCheck/2` errors.
///
/// ## Decoding Pipeline
///
/// ```
/// received bytes
///   │
///   ▼ [1] Compute syndromes S₁...S_{nCheck}
///   │     All zero? → return message directly (no errors)
///   │
///   ▼ [2] Berlekamp-Massey → Λ(x), error count L
///   │     L > t? → TooManyErrorsException
///   │
///   ▼ [3] Chien search → error positions {p₁...pᵥ}
///   │     |positions| ≠ L? → TooManyErrorsException
///   │
///   ▼ [4] Forney → error magnitudes {e₁...eᵥ}
///   │
///   ▼ [5] Correct: received[pₖ] ^= eₖ for each k
///   │
///   ▼ Return first k = len - nCheck bytes
/// ```
///
/// @param received received codeword bytes (possibly corrupted)
/// @param nCheck number of check bytes (must be even ≥ 2)
/// @returns recovered message bytes (length = received.length - nCheck)
/// @throws [InvalidInputException] if nCheck is 0/odd or received is too short
/// @throws [TooManyErrorsException] if more than t errors are present
Uint8List rsDecode(Uint8List received, int nCheck) {
  if (nCheck == 0 || nCheck % 2 != 0) {
    throw InvalidInputException(
        'nCheck must be a positive even number, got $nCheck');
  }
  if (received.length < nCheck) {
    throw InvalidInputException(
        'received length ${received.length} < nCheck $nCheck');
  }

  final int t = nCheck ~/ 2;
  final int n = received.length;
  final int k = n - nCheck;

  // Step 1: Syndromes
  final synds = rsSyndromes(received, nCheck);
  if (synds.every((int s) => s == 0)) {
    // No errors: return the message bytes directly.
    return received.sublist(0, k);
  }

  // Step 2: Berlekamp-Massey
  final (lambda, numErrors) = _berlekampMassey(synds);
  if (numErrors > t) throw const TooManyErrorsException();

  // Step 3: Chien Search
  final positions = _chienSearch(lambda, n);
  if (positions.length != numErrors) throw const TooManyErrorsException();

  // Step 4: Forney
  final magnitudes = _forney(lambda, synds, positions, n);

  // Step 5: Apply corrections (XOR in GF(256))
  final corrected = Uint8List.fromList(received);
  for (int i = 0; i < positions.length; i++) {
    corrected[positions[i]] ^= magnitudes[i];
  }

  return corrected.sublist(0, k);
}

/// Compute the error locator polynomial from a syndrome array.
///
/// Exposed for advanced use (QR decoders, diagnostics).
/// Returns Λ(x) in **little-endian** form with Λ[0] = 1.
///
/// @param synds syndrome array (length = nCheck)
Uint8List rsErrorLocator(Uint8List synds) {
  final (lambda, _) = _berlekampMassey(synds);
  return lambda;
}
