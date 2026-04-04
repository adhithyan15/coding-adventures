/**
 * @module reed-solomon
 *
 * Reed-Solomon error-correcting codes over GF(256).
 *
 * Reed-Solomon is a block error-correcting code: you add `nCheck` redundancy
 * bytes to a message, and the decoder can recover the original even if up to
 * `t = nCheck / 2` bytes are corrupted.
 *
 * ## Where RS is Used
 *
 * | System | How RS Helps |
 * |--------|-------------|
 * | QR codes | Up to 30% of a QR symbol can be scratched and still decode |
 * | CDs / DVDs | CIRC two-level RS corrects scratches |
 * | Hard drives | Firmware error correction for sector-level faults |
 * | Voyager probes | Transmit images across 20+ billion km |
 * | RAID-6 | The two parity drives are exactly an (n, n-2) RS code |
 *
 * ## Building Blocks
 *
 * ```
 * MA00  polynomial   — f64 polynomial arithmetic
 * MA01  gf256        — GF(2^8) field arithmetic (add=XOR, mul=table lookup)
 * MA02  reed-solomon ← THIS PACKAGE
 * ```
 *
 * ## Quick Start
 *
 * ```ts
 * import { encode, decode } from "@coding-adventures/reed-solomon";
 *
 * const message = new Uint8Array([72, 101, 108, 108, 111]); // "Hello"
 * const nCheck = 8; // t = 4 errors correctable
 *
 * const codeword = encode(message, nCheck);
 * // codeword[0..4] === message (systematic)
 *
 * // Corrupt 4 bytes — still recoverable
 * codeword[0] ^= 0xff;
 * codeword[2] ^= 0xaa;
 *
 * const recovered = decode(codeword, nCheck);
 * // recovered deep-equals message
 * ```
 *
 * ## Polynomial Conventions
 *
 * All codeword bytes are treated as a **big-endian** polynomial:
 *
 * ```
 * codeword[0]·x^{n-1} + codeword[1]·x^{n-2} + … + codeword[n-1]
 * ```
 *
 * The systematic codeword layout:
 *
 * ```
 * [ message[0] … message[k-1] | check[0] … check[nCheck-1] ]
 *   degree n-1 … degree nCheck    degree nCheck-1 … degree 0
 * ```
 *
 * For error position `p` in a big-endian codeword of length `n`, the **locator
 * number** is `X_p = α^{n-1-p}` and its inverse is `X_p⁻¹ = α^{(p+256-n) mod 255}`.
 */

import {
  add,
  divide,
  multiply,
  power,
} from "@coding-adventures/gf256";

export { VERSION } from "./version.js";

// =============================================================================
// Types
// =============================================================================

/**
 * Error returned when decoding fails because more errors occurred than the
 * code's correction capacity `t = nCheck / 2`.
 */
export class TooManyErrorsError extends Error {
  constructor() {
    super("reed-solomon: too many errors — codeword is unrecoverable");
    this.name = "TooManyErrorsError";
  }
}

/**
 * Error returned when encode/decode receives invalid parameters.
 */
export class InvalidInputError extends Error {
  constructor(message: string) {
    super(`reed-solomon: invalid input — ${message}`);
    this.name = "InvalidInputError";
  }
}

// =============================================================================
// Generator Polynomial
// =============================================================================

/**
 * Build the RS generator polynomial for a given number of check bytes.
 *
 * The generator is the product of `nCheck` linear factors:
 *
 * ```
 * g(x) = (x + α¹)(x + α²)…(x + α^{nCheck})
 * ```
 *
 * where `α = 2` is the primitive element of GF(256).
 *
 * ## Return
 *
 * A **little-endian** coefficient array (index = degree), length `nCheck + 1`.
 * The last element is always `1` (monic polynomial).
 *
 * ## Algorithm
 *
 * Start with `g = [1]`. At each step multiply by `(αⁱ + x)`:
 *
 * ```
 * new_g[j] = GF256.mul(αⁱ, g[j]) XOR g[j-1]
 * ```
 *
 * ## Example: nCheck = 2
 *
 * ```
 * Start: g = [1]
 * i=1: g = [2, 1]           (2 + x)
 * i=2: g = [8, 6, 1]        (8 + 6x + x²)
 * ```
 *
 * Verify root α¹=2: g(2) = 8 XOR GF256.mul(6,2) XOR GF256.mul(1,4) = 8 XOR 12 XOR 4 = 0 ✓
 *
 * @throws {InvalidInputError} if nCheck is 0 or odd
 */
export function buildGenerator(nCheck: number): Uint8Array {
  if (nCheck === 0 || nCheck % 2 !== 0) {
    throw new InvalidInputError(
      `nCheck must be a positive even number, got ${nCheck}`
    );
  }

  let g = new Uint8Array([1]);

  for (let i = 1; i <= nCheck; i++) {
    const alphaI = power(2, i);
    const newG = new Uint8Array(g.length + 1);
    for (let j = 0; j < g.length; j++) {
      newG[j] ^= multiply(g[j], alphaI);
      newG[j + 1] ^= g[j];
    }
    g = newG;
  }

  return g;
}

// =============================================================================
// Internal Polynomial Helpers
// =============================================================================

/**
 * Evaluate a **big-endian** GF(256) polynomial at `x`.
 *
 * `p[0]` is the highest-degree coefficient. Horner's method left-to-right:
 *
 * ```
 * acc = 0
 * for each byte b in p (highest degree first):
 *   acc = acc·x + b
 * ```
 *
 * Used for syndrome evaluation: `S_j = polyEvalBE(codeword, α^j)`.
 */
function polyEvalBE(p: Uint8Array, x: number): number {
  let acc = 0;
  for (const b of p) {
    acc = add(multiply(acc, x), b);
  }
  return acc;
}

/**
 * Evaluate a **little-endian** GF(256) polynomial at `x`.
 *
 * `p[i]` is the coefficient of `xⁱ`. Horner iterates from high to low degree.
 */
function polyEvalLE(p: Uint8Array, x: number): number {
  let acc = 0;
  for (let i = p.length - 1; i >= 0; i--) {
    acc = add(multiply(acc, x), p[i]);
  }
  return acc;
}

/**
 * Multiply two **little-endian** GF(256) polynomials (convolution).
 *
 * `result[i+j] ^= a[i] · b[j]`
 */
function polyMulLE(a: Uint8Array, b: Uint8Array): Uint8Array {
  if (a.length === 0 || b.length === 0) return new Uint8Array(0);
  const result = new Uint8Array(a.length + b.length - 1);
  for (let i = 0; i < a.length; i++) {
    for (let j = 0; j < b.length; j++) {
      result[i + j] ^= multiply(a[i], b[j]);
    }
  }
  return result;
}

/**
 * Compute the remainder of **big-endian** polynomial division in GF(256).
 *
 * `dividend` and `divisor` are both big-endian (first = highest degree).
 * The divisor must be **monic** (leading coefficient = 1).
 *
 * ## Algorithm
 *
 * At each step, eliminate the current leading term by subtracting a scaled
 * copy of the divisor:
 *
 * ```
 * for i = 0 .. (len(dividend) - len(divisor)):
 *   coeff = dividend[i]        // monic divisor: coeff = dividend[i] / 1
 *   for j = 0 .. len(divisor):
 *     dividend[i+j] ^= coeff · divisor[j]
 * ```
 *
 * The last `(len(divisor) - 1)` bytes are the remainder.
 */
function polyModBE(dividend: Uint8Array, divisor: Uint8Array): Uint8Array {
  const rem = new Uint8Array(dividend);
  const divLen = divisor.length;

  if (rem.length < divLen) {
    return rem;
  }

  const steps = rem.length - divLen + 1;
  for (let i = 0; i < steps; i++) {
    const coeff = rem[i];
    if (coeff === 0) continue;
    for (let j = 0; j < divLen; j++) {
      rem[i + j] ^= multiply(coeff, divisor[j]);
    }
  }

  return rem.slice(rem.length - (divLen - 1));
}

/**
 * Compute the inverse locator `X_p⁻¹` for byte position `p` in a codeword
 * of length `n`.
 *
 * In big-endian convention, position `p` has degree `n-1-p`.
 * The locator is `X_p = α^{n-1-p}`, so `X_p⁻¹ = α^{(p+256-n) mod 255}`.
 *
 * Special cases:
 * - `p = n-1` (last byte): `X_p⁻¹ = α^{255 mod 255} = α⁰ = 1`
 * - `p = 0` (first byte): `X_p⁻¹ = α^{256-n mod 255}`
 */
function invLocator(p: number, n: number): number {
  const exp = (p + 256 - n) % 255;
  return power(2, exp);
}

// =============================================================================
// Encoding
// =============================================================================

/**
 * Encode a message with Reed-Solomon, producing a systematic codeword.
 *
 * **Systematic** means the message bytes are unchanged in the output:
 *
 * ```
 * output = [ message bytes | check bytes ]
 *            degree n-1 … nCheck    degree nCheck-1 … 0
 * ```
 *
 * ## Algorithm
 *
 * 1. Build generator `g` (little-endian), then reverse to big-endian `g_BE`.
 * 2. Append `nCheck` zero bytes: `shifted = message || 000…0`
 *    (represents `M(x)·x^{nCheck}` in big-endian).
 * 3. Remainder `R = shifted mod g_BE`.
 * 4. Output `message || R` (padded to exactly `nCheck` bytes).
 *
 * ## Why it works
 *
 * `C(x) = M(x)·x^{nCheck} + R(x) = Q(x)·g(x)` (by division algorithm),
 * so `C(αⁱ) = Q(αⁱ)·g(αⁱ) = 0` for `i = 1…nCheck`.
 *
 * @param message - raw data bytes
 * @param nCheck  - number of check bytes to add (must be even ≥ 2)
 * @returns systematic codeword of length `message.length + nCheck`
 * @throws {InvalidInputError} if nCheck is 0/odd, or total length > 255
 */
export function encode(message: Uint8Array, nCheck: number): Uint8Array {
  if (nCheck === 0 || nCheck % 2 !== 0) {
    throw new InvalidInputError(
      `nCheck must be a positive even number, got ${nCheck}`
    );
  }
  const n = message.length + nCheck;
  if (n > 255) {
    throw new InvalidInputError(
      `total codeword length ${n} exceeds GF(256) block size limit of 255`
    );
  }

  const gLE = buildGenerator(nCheck);
  // Reverse to big-endian for division: g_LE[last] = 1 becomes g_BE[0] = 1
  const gBE = gLE.slice().reverse();

  // shifted = message || zeros (big-endian representation of M(x)·x^{nCheck})
  const shifted = new Uint8Array(n);
  shifted.set(message, 0);
  // trailing nCheck bytes stay 0

  const remainder = polyModBE(shifted, gBE);

  // Codeword = message || check bytes (padded to nCheck bytes)
  const codeword = new Uint8Array(n);
  codeword.set(message, 0);
  const pad = nCheck - remainder.length;
  codeword.set(remainder, message.length + pad);

  return codeword;
}

// =============================================================================
// Decoding
// =============================================================================

/**
 * Compute the `nCheck` syndromes of a received codeword.
 *
 * `S_j = received(α^j)` for `j = 1, …, nCheck`.
 *
 * If all syndromes are zero, the codeword has no errors. Any non-zero
 * syndrome reveals corruption.
 *
 * ## Convention
 *
 * The codeword is evaluated as a **big-endian** polynomial: `received[0]`
 * is the highest-degree coefficient. An error at position `p` contributes
 * `e · (α^j)^{n-1-p} = e · X_p^j` where `X_p = α^{n-1-p}`.
 *
 * @param received - received codeword bytes (possibly corrupted)
 * @param nCheck   - number of check bytes in the codeword
 * @returns array of `nCheck` syndrome values
 */
export function syndromes(received: Uint8Array, nCheck: number): Uint8Array {
  const s = new Uint8Array(nCheck);
  for (let i = 1; i <= nCheck; i++) {
    s[i - 1] = polyEvalBE(received, power(2, i));
  }
  return s;
}

/**
 * Berlekamp-Massey algorithm: find the shortest LFSR generating the syndrome
 * sequence.
 *
 * Returns `[Λ, L]` where `Λ` is the **error locator polynomial** in
 * **little-endian** form (`Λ[0] = 1`) and `L` is the number of errors.
 *
 * The LFSR connection polynomial `Λ(x)` satisfies:
 *
 * ```
 * Λ(x) = ∏_{k=1}^{v} (1 - X_k · x)
 * ```
 *
 * where `X_k` are the error locator numbers. The roots of `Λ` are `X_k⁻¹`,
 * found via Chien search.
 *
 * ## Algorithm
 *
 * ```
 * C = [1], B = [1], L = 0, xShift = 1, b = 1
 *
 * for n = 0 to 2t-1:
 *   d = S[n] XOR ∑_{j=1}^{L} C[j]·S[n-j]   ← discrepancy
 *
 *   if d == 0:
 *     xShift++
 *   elif 2L ≤ n:
 *     T = C.clone()
 *     C = C XOR (d/b)·x^{xShift}·B
 *     L = n+1-L;  B = T;  b = d;  xShift = 1
 *   else:
 *     C = C XOR (d/b)·x^{xShift}·B
 *     xShift++
 * ```
 */
function berlekampMassey(synds: Uint8Array): [Uint8Array, number] {
  const twoT = synds.length;

  let c = new Uint8Array([1]);
  let b = new Uint8Array([1]);
  let bigL = 0;
  let xShift = 1;
  let bScale = 1;

  for (let n = 0; n < twoT; n++) {
    // Compute discrepancy d = S[n] + ∑_{j=1}^{L} C[j]·S[n-j]
    let d = synds[n];
    for (let j = 1; j <= bigL; j++) {
      if (j < c.length && n >= j) {
        d ^= multiply(c[j], synds[n - j]);
      }
    }

    if (d === 0) {
      xShift++;
    } else if (2 * bigL <= n) {
      const tSave = c.slice();

      const scale = divide(d, bScale);
      const shiftedLen = xShift + b.length;
      if (c.length < shiftedLen) {
        const cNew = new Uint8Array(shiftedLen);
        cNew.set(c);
        c = cNew;
      }
      for (let k = 0; k < b.length; k++) {
        c[xShift + k] ^= multiply(scale, b[k]);
      }

      bigL = n + 1 - bigL;
      b = tSave;
      bScale = d;
      xShift = 1;
    } else {
      const scale = divide(d, bScale);
      const shiftedLen = xShift + b.length;
      if (c.length < shiftedLen) {
        const cNew = new Uint8Array(shiftedLen);
        cNew.set(c);
        c = cNew;
      }
      for (let k = 0; k < b.length; k++) {
        c[xShift + k] ^= multiply(scale, b[k]);
      }
      xShift++;
    }
  }

  return [c, bigL];
}

/**
 * Chien Search: find which byte positions are error locations.
 *
 * Position `p` is an error location if `Λ(X_p⁻¹) = 0`, where
 * `X_p⁻¹ = α^{(p+256-n) mod 255}` for a codeword of length `n`.
 *
 * @returns sorted array of error positions (0-indexed, big-endian)
 */
function chienSearch(lambda: Uint8Array, n: number): number[] {
  const positions: number[] = [];
  for (let p = 0; p < n; p++) {
    const xiInv = invLocator(p, n);
    if (polyEvalLE(lambda, xiInv) === 0) {
      positions.push(p);
    }
  }
  return positions;
}

/**
 * Forney Algorithm: compute error magnitudes from positions.
 *
 * For each error at position `p`:
 *
 * ```
 * e_p = Ω(X_p⁻¹) / Λ'(X_p⁻¹)
 * ```
 *
 * where:
 * - `Ω(x) = (S(x) · Λ(x)) mod x^{2t}` — error evaluator polynomial
 * - `S(x) = S₁ + S₂x + … + S_{2t}x^{2t-1}` — syndrome polynomial (LE)
 * - `Λ'(x)` — formal derivative of Λ in GF(2^8)
 *
 * ## Formal derivative in characteristic 2
 *
 * Only odd-indexed coefficients of Λ survive (even terms vanish because 2 = 0):
 *
 * ```
 * Λ'(x) = Λ₁ + Λ₃x² + Λ₅x⁴ + …
 * ```
 *
 * @throws {TooManyErrorsError} if the denominator evaluates to zero
 */
function forney(
  lambda: Uint8Array,
  synds: Uint8Array,
  positions: number[],
  n: number
): number[] {
  const twoT = synds.length;

  // Ω = S(x) · Λ(x) mod x^{2t}: truncate to first 2t terms
  const sFull = synds; // LE: S[0]=S₁, S[1]=S₂, …
  const omegaFull = polyMulLE(sFull, lambda);
  const omega = omegaFull.slice(0, twoT);

  // Formal derivative Λ'(x): Λ'[k] = Λ[k+1] if k+1 is odd (k is even), else 0
  const lambdaPrime = new Uint8Array(Math.max(0, lambda.length - 1));
  for (let j = 1; j < lambda.length; j++) {
    if (j % 2 === 1) {
      // Odd index j → contributes to Λ'[j-1]
      lambdaPrime[j - 1] ^= lambda[j];
    }
  }

  return positions.map((pos) => {
    const xiInv = invLocator(pos, n);
    const omegaVal = polyEvalLE(omega, xiInv);
    const lpVal = polyEvalLE(lambdaPrime, xiInv);
    if (lpVal === 0) throw new TooManyErrorsError();
    return divide(omegaVal, lpVal);
  });
}

/**
 * Decode a received Reed-Solomon codeword, correcting up to `t = nCheck/2` errors.
 *
 * ## Pipeline
 *
 * ```
 * received
 *   │
 *   ▼ Step 1: Compute syndromes S₁…S_{nCheck}
 *   │         all zero? → return message directly
 *   │
 *   ▼ Step 2: Berlekamp-Massey → Λ(x), error count L
 *   │         L > t? → TooManyErrorsError
 *   │
 *   ▼ Step 3: Chien search → error positions {p₁…pᵥ}
 *   │         |positions| ≠ L? → TooManyErrorsError
 *   │
 *   ▼ Step 4: Forney → error magnitudes {e₁…eᵥ}
 *   │
 *   ▼ Step 5: Correct: received[pₖ] ^= eₖ
 *   │
 *   ▼ Return first k = len - nCheck bytes
 * ```
 *
 * @param received - received codeword bytes (possibly corrupted)
 * @param nCheck   - number of check bytes (must be even ≥ 2)
 * @returns recovered message bytes (length = received.length - nCheck)
 * @throws {InvalidInputError}   if nCheck is 0/odd or received is too short
 * @throws {TooManyErrorsError}  if more than t errors are present
 */
export function decode(received: Uint8Array, nCheck: number): Uint8Array {
  if (nCheck === 0 || nCheck % 2 !== 0) {
    throw new InvalidInputError(
      `nCheck must be a positive even number, got ${nCheck}`
    );
  }
  if (received.length < nCheck) {
    throw new InvalidInputError(
      `received length ${received.length} < nCheck ${nCheck}`
    );
  }

  const t = nCheck / 2;
  const n = received.length;
  const k = n - nCheck;

  // Step 1: Syndromes
  const synds = syndromes(received, nCheck);
  if (synds.every((s) => s === 0)) {
    return received.slice(0, k);
  }

  // Step 2: Berlekamp-Massey
  const [lambda, numErrors] = berlekampMassey(synds);
  if (numErrors > t) throw new TooManyErrorsError();

  // Step 3: Chien Search
  const positions = chienSearch(lambda, n);
  if (positions.length !== numErrors) throw new TooManyErrorsError();

  // Step 4: Forney
  const magnitudes = forney(lambda, synds, positions, n);

  // Step 5: Apply corrections
  const corrected = received.slice();
  for (let i = 0; i < positions.length; i++) {
    corrected[positions[i]] ^= magnitudes[i];
  }

  return corrected.slice(0, k);
}

/**
 * Compute the error locator polynomial from a syndrome array.
 *
 * Exposed for advanced use cases (QR decoders, diagnostics).
 * Returns `Λ(x)` in **little-endian** form with `Λ[0] = 1`.
 *
 * @param synds - syndrome array (length = nCheck)
 */
export function errorLocator(synds: Uint8Array): Uint8Array {
  const [lambda] = berlekampMassey(synds);
  return lambda;
}
