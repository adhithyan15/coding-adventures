package com.codingadventures.reedsolomon;

import com.codingadventures.gf256.GF256;

/**
 * Reed-Solomon error-correcting codes over GF(256).
 *
 * <p>Reed-Solomon is a block error-correcting code: given a message of {@code k}
 * bytes, the encoder appends {@code nCheck} redundancy bytes to form a
 * {@code k + nCheck} byte codeword. The decoder can recover the original message
 * even if up to {@code t = nCheck / 2} bytes are corrupted.
 *
 * <h2>Where RS is Used</h2>
 *
 * <table border="1">
 *   <tr><th>System</th><th>How RS Helps</th></tr>
 *   <tr><td>QR codes</td><td>Up to 30% damage survivable thanks to RS</td></tr>
 *   <tr><td>CDs / DVDs</td><td>CIRC two-level RS corrects scratches</td></tr>
 *   <tr><td>Hard drives</td><td>Firmware sector-level error correction</td></tr>
 *   <tr><td>Deep-space probes</td><td>Voyager transmits images across billions of km</td></tr>
 *   <tr><td>RAID-6</td><td>Two parity drives are exactly an (n, n-2) RS code</td></tr>
 * </table>
 *
 * <h2>Building Blocks</h2>
 *
 * <pre>
 *   MA00  polynomial   — coefficient-array polynomial arithmetic
 *   MA01  gf256        — GF(2^8) field arithmetic (add=XOR, mul=table lookup)
 *   MA02  reed-solomon ← THIS CLASS
 * </pre>
 *
 * <h2>Code Parameters</h2>
 *
 * <pre>
 *   nCheck = n - k = number of check bytes
 *   t      = nCheck / 2 = maximum errors correctable
 *   n      ≤ 255 (GF(256) block size limit)
 * </pre>
 *
 * <h2>Polynomial Conventions</h2>
 *
 * <p>All codeword bytes are treated internally as a <strong>big-endian</strong> polynomial:
 * <pre>
 *   codeword[0] · x^{n-1} + codeword[1] · x^{n-2} + … + codeword[n-1]
 * </pre>
 *
 * <p>The systematic codeword layout:
 * <pre>
 *   [ message[0] … message[k-1] | check[0] … check[nCheck-1] ]
 *     degree n-1 … degree nCheck    degree nCheck-1 … degree 0
 * </pre>
 *
 * <p>Internally, the generator polynomial and other scratch polynomials use
 * <strong>little-endian</strong> coefficient arrays (index = degree) for the
 * Berlekamp-Massey and Forney computations, matching the GF256 polynomial
 * operations from the polynomial package.
 */
public final class ReedSolomon {

    // =========================================================================
    // Generator Polynomial
    // =========================================================================

    /**
     * Build the RS generator polynomial for a given number of check bytes.
     *
     * <p>The generator is the product of {@code nCheck} linear factors:
     * <pre>
     *   g(x) = (x + α¹)(x + α²) … (x + α^{nCheck})
     * </pre>
     * where {@code α = 2} is the primitive element of GF(256).
     *
     * <p>All coefficient arithmetic is in GF(256): addition is XOR, multiplication
     * uses the log/antilog tables.
     *
     * <p>The result is a <strong>little-endian</strong> coefficient array of
     * length {@code nCheck + 1}. The last element (highest degree) is always 1
     * (the polynomial is monic).
     *
     * <p>Example for {@code nCheck = 2}:
     * <pre>
     *   Start: g = [1]
     *   i=1:   g = [2, 1]         (x + 2)
     *   i=2:   g = [8, 6, 1]      (x² + 6x + 8)
     *
     *   Verify root α=2: 8 XOR GF256.mul(6,2) XOR 4 = 8 XOR 12 XOR 4 = 0  ✓
     * </pre>
     *
     * @param nCheck number of check bytes; must be even and ≥ 2
     * @return little-endian generator polynomial of degree {@code nCheck}
     * @throws RsInvalidInputException if nCheck is 0 or odd
     */
    public static int[] buildGenerator(int nCheck) {
        if (nCheck == 0 || nCheck % 2 != 0) {
            throw new RsInvalidInputException(
                "nCheck must be a positive even number, got " + nCheck);
        }

        // g starts as the constant polynomial [1].
        int[] g = new int[]{1};

        // Multiply in each linear factor (x + α^i) for i = 1..nCheck.
        // In little-endian form, (x + α^i) has coefficients [α^i, 1].
        for (int i = 1; i <= nCheck; i++) {
            int alphaI = GF256.pow(2, i);      // α^i
            int[] newG = new int[g.length + 1]; // degree increases by 1

            // Multiply g by (x + alphaI).
            // new_g[j]   ^= g[j] * alphaI    (term from x-part of the factor)
            // new_g[j+1] ^= g[j]              (term from the x-coefficient 1)
            for (int j = 0; j < g.length; j++) {
                newG[j]     ^= GF256.mul(g[j], alphaI);
                newG[j + 1] ^= g[j];
            }
            g = newG;
        }

        return g;
    }

    // =========================================================================
    // Internal Helpers
    // =========================================================================

    /**
     * Evaluate a <strong>big-endian</strong> GF(256) polynomial at point {@code x}.
     *
     * <p>In big-endian form, {@code p[0]} is the highest-degree coefficient.
     * Horner's method iterates left-to-right:
     * <pre>
     *   acc = 0
     *   for each byte b in p (highest degree first):
     *     acc = acc · x  XOR  b
     * </pre>
     *
     * <p>Used for syndrome evaluation: {@code S_j = evalBE(codeword, α^j)}.
     */
    private static int evalBE(byte[] p, int x) {
        int acc = 0;
        for (byte b : p) {
            acc = GF256.add(GF256.mul(acc, x), b & 0xFF);
        }
        return acc;
    }

    /**
     * Evaluate a <strong>little-endian</strong> GF(256) polynomial at point {@code x}.
     *
     * <p>{@code p[i]} is the coefficient of {@code x^i}. Horner iterates
     * from high degree to low:
     * <pre>
     *   acc = 0
     *   for i from p.length-1 downto 0:
     *     acc = acc · x  XOR  p[i]
     * </pre>
     */
    private static int evalLE(int[] p, int x) {
        int acc = 0;
        for (int i = p.length - 1; i >= 0; i--) {
            acc = GF256.add(GF256.mul(acc, x), p[i]);
        }
        return acc;
    }

    /**
     * Multiply two little-endian GF(256) polynomials (convolution).
     *
     * <p>{@code result[i+j] ^= a[i] * b[j]} for all i, j.
     */
    private static int[] mulLE(int[] a, int[] b) {
        if (a.length == 0 || b.length == 0) return new int[0];
        int[] result = new int[a.length + b.length - 1];
        for (int i = 0; i < a.length; i++) {
            for (int j = 0; j < b.length; j++) {
                result[i + j] ^= GF256.mul(a[i], b[j]);
            }
        }
        return result;
    }

    /**
     * Compute the remainder of big-endian polynomial division in GF(256).
     *
     * <p>Both {@code dividend} and {@code divisor} are big-endian.
     * The divisor must be <strong>monic</strong> (leading coefficient = 1),
     * which is always the case for the generator polynomial.
     *
     * <p>Algorithm: for each position from left to right, eliminate the current
     * leading term by subtracting a scaled copy of the divisor:
     * <pre>
     *   for i = 0 .. (len(dividend) - len(divisor)):
     *     coeff = dividend[i]        (monic divisor: divide by 1)
     *     for j = 0 .. len(divisor):
     *       dividend[i+j] ^= coeff · divisor[j]
     * </pre>
     *
     * <p>The last {@code (len(divisor) - 1)} bytes are the remainder.
     *
     * @param dividend big-endian dividend
     * @param divisor  big-endian monic divisor
     * @return big-endian remainder of length {@code divisor.length - 1}
     */
    private static byte[] modBE(byte[] dividend, int[] divisor) {
        // Work on a mutable int copy to avoid byte overflow issues.
        int[] rem = new int[dividend.length];
        for (int i = 0; i < dividend.length; i++) {
            rem[i] = dividend[i] & 0xFF;
        }

        int divLen = divisor.length;
        if (rem.length < divLen) {
            // Dividend shorter than divisor — remainder is the dividend itself.
            byte[] r = new byte[rem.length];
            for (int i = 0; i < rem.length; i++) r[i] = (byte) rem[i];
            return r;
        }

        int steps = rem.length - divLen + 1;
        for (int i = 0; i < steps; i++) {
            int coeff = rem[i];
            if (coeff == 0) continue;
            for (int j = 0; j < divLen; j++) {
                rem[i + j] ^= GF256.mul(coeff, divisor[j]);
            }
        }

        // The last (divLen - 1) entries are the remainder.
        byte[] result = new byte[divLen - 1];
        for (int i = 0; i < result.length; i++) {
            result[i] = (byte) rem[rem.length - (divLen - 1) + i];
        }
        return result;
    }

    /**
     * Compute the inverse locator X_p⁻¹ for byte position {@code p} in a
     * codeword of length {@code n}.
     *
     * <p>In big-endian convention, position {@code p} has degree {@code n-1-p}.
     * The error locator is {@code X_p = α^{n-1-p}}, so
     * {@code X_p⁻¹ = α^{(p+256-n) mod 255}}.
     */
    private static int invLocator(int p, int n) {
        int exp = (p + 256 - n) % 255;
        return GF256.pow(2, exp);
    }

    // =========================================================================
    // Encoding
    // =========================================================================

    /**
     * Encode a message with Reed-Solomon, producing a <strong>systematic</strong> codeword.
     *
     * <p>Systematic means the original message bytes appear unchanged at the start
     * of the output, followed by the computed check bytes:
     * <pre>
     *   output = [ message[0] … message[k-1] | check[0] … check[nCheck-1] ]
     * </pre>
     *
     * <p>Algorithm:
     * <ol>
     *   <li>Build the generator polynomial {@code g} (little-endian), then
     *       reverse to big-endian {@code g_BE} for division.</li>
     *   <li>Form the shifted message: {@code shifted = message || 000…0}
     *       (represents {@code M(x) · x^{nCheck}} in big-endian).</li>
     *   <li>Remainder {@code R = shifted mod g_BE} has exactly {@code nCheck} bytes.</li>
     *   <li>Output {@code message || R}.</li>
     * </ol>
     *
     * <p>Why this works: {@code C(x) = M(x)·x^{nCheck} + R(x) = Q(x)·g(x)}, so
     * {@code C(α^i) = 0} for all {@code i = 1..nCheck}.
     *
     * @param message input data bytes
     * @param nCheck  number of check bytes to add (must be even and ≥ 2)
     * @return systematic codeword of length {@code message.length + nCheck}
     * @throws RsInvalidInputException if nCheck is 0/odd or total length &gt; 255
     */
    public static byte[] encode(byte[] message, int nCheck) {
        if (nCheck == 0 || nCheck % 2 != 0) {
            throw new RsInvalidInputException(
                "nCheck must be a positive even number, got " + nCheck);
        }
        int n = message.length + nCheck;
        if (n > 255) {
            throw new RsInvalidInputException(
                "total codeword length " + n + " exceeds GF(256) block size limit of 255");
        }

        int[] gLE = buildGenerator(nCheck);

        // Reverse to big-endian: the leading (highest-degree) coefficient moves to index 0.
        // gLE[nCheck] = 1 (monic) → gBE[0] = 1.
        int[] gBE = new int[gLE.length];
        for (int i = 0; i < gLE.length; i++) {
            gBE[i] = gLE[gLE.length - 1 - i];
        }

        // shifted = message bytes followed by nCheck zero bytes.
        // Represents M(x) · x^{nCheck} in big-endian form.
        byte[] shifted = new byte[n];
        System.arraycopy(message, 0, shifted, 0, message.length);
        // trailing nCheck bytes are already 0 by array initialization.

        // Compute the remainder R = shifted mod g_BE.
        byte[] remainder = modBE(shifted, gBE);

        // Assemble the codeword: message || remainder (pad remainder to nCheck bytes).
        byte[] codeword = new byte[n];
        System.arraycopy(message, 0, codeword, 0, message.length);
        int pad = nCheck - remainder.length;
        System.arraycopy(remainder, 0, codeword, message.length + pad, remainder.length);

        return codeword;
    }

    // =========================================================================
    // Syndrome Computation
    // =========================================================================

    /**
     * Compute the {@code nCheck} syndromes of a received codeword.
     *
     * <p>{@code S_j = received(α^j)} for {@code j = 1, …, nCheck}.
     *
     * <p>If all syndromes are zero, the codeword has no detectable errors.
     * Any non-zero syndrome reveals corruption.
     *
     * <p>Evaluation uses the big-endian polynomial convention: {@code received[0]}
     * is the highest-degree coefficient. An error at position {@code p} contributes
     * {@code e · (α^j)^{n-1-p} = e · X_p^j} to syndrome {@code S_j}.
     *
     * @param received codeword bytes (possibly corrupted)
     * @param nCheck   number of check bytes in the codeword
     * @return array of {@code nCheck} syndrome values
     */
    public static int[] syndromes(byte[] received, int nCheck) {
        int[] s = new int[nCheck];
        for (int i = 1; i <= nCheck; i++) {
            s[i - 1] = evalBE(received, GF256.pow(2, i));
        }
        return s;
    }

    // =========================================================================
    // Berlekamp-Massey Algorithm
    // =========================================================================

    /**
     * Find the error locator polynomial Λ(x) via the Berlekamp-Massey algorithm.
     *
     * <p>Given the syndrome array, finds the shortest linear feedback shift register
     * (LFSR) that generates the syndrome sequence. The LFSR connection polynomial
     * is Λ(x), the error locator polynomial.
     *
     * <p>If errors occurred at positions {@code i₁, i₂, …, iᵥ}, the error locators
     * are {@code X_k = α^{i_k}} and:
     * <pre>
     *   Λ(x) = ∏_k (1 - X_k · x)   with  Λ(0) = 1
     * </pre>
     *
     * <p>Algorithm (0-indexed syndromes, following the TypeScript reference):
     * <pre>
     *   C = [1], B = [1], bigL = 0, xShift = 1, bScale = 1
     *
     *   for n = 0 to 2t-1:
     *     d = synds[n] XOR ∑_{j=1}^{bigL} C[j] · synds[n-j]
     *
     *     if d == 0:
     *       xShift++
     *     elif 2*bigL ≤ n:
     *       T = C.clone()
     *       C = C XOR (d/bScale) · x^{xShift} · B
     *       bigL = n+1-bigL;  B = T;  bScale = d;  xShift = 1
     *     else:
     *       C = C XOR (d/bScale) · x^{xShift} · B
     *       xShift++
     * </pre>
     *
     * @param synds syndrome array (length = nCheck = 2t)
     * @return two-element array {@code {Λ, numErrors}} where Λ is the error locator
     *         in little-endian form ({@code Λ[0] = 1}) and numErrors = degree of Λ
     */
    static int[][] berlekampMassey(int[] synds) {
        int twoT = synds.length;

        int[] c = new int[]{1};
        int[] b = new int[]{1};
        int bigL  = 0;
        int xShift = 1;
        int bScale = 1;

        for (int n = 0; n < twoT; n++) {
            // Compute the discrepancy: d = synds[n] + ∑_{j=1}^{L} C[j] · synds[n-j]
            int d = synds[n];
            for (int j = 1; j <= bigL; j++) {
                if (j < c.length && n >= j) {
                    d ^= GF256.mul(c[j], synds[n - j]);
                }
            }

            if (d == 0) {
                xShift++;
            } else if (2 * bigL <= n) {
                // Save a copy of C, then update C.
                int[] tSave = c.clone();

                int scale = GF256.div(d, bScale);
                int shiftedLen = xShift + b.length;
                if (c.length < shiftedLen) {
                    int[] cNew = new int[shiftedLen];
                    System.arraycopy(c, 0, cNew, 0, c.length);
                    c = cNew;
                }
                for (int k = 0; k < b.length; k++) {
                    c[xShift + k] ^= GF256.mul(scale, b[k]);
                }

                bigL   = n + 1 - bigL;
                b      = tSave;
                bScale = d;
                xShift = 1;
            } else {
                int scale = GF256.div(d, bScale);
                int shiftedLen = xShift + b.length;
                if (c.length < shiftedLen) {
                    int[] cNew = new int[shiftedLen];
                    System.arraycopy(c, 0, cNew, 0, c.length);
                    c = cNew;
                }
                for (int k = 0; k < b.length; k++) {
                    c[xShift + k] ^= GF256.mul(scale, b[k]);
                }
                xShift++;
            }
        }

        return new int[][]{c, new int[]{bigL}};
    }

    // =========================================================================
    // Chien Search
    // =========================================================================

    /**
     * Chien Search: find which byte positions are error locations.
     *
     * <p>Position {@code p} is an error location if {@code Λ(X_p⁻¹) = 0}, where
     * {@code X_p⁻¹ = α^{(p+256-n) mod 255}} for a codeword of length {@code n}.
     *
     * <p>Iterates over all {@code n} positions and collects those where
     * the error locator polynomial evaluates to zero.
     *
     * @param lambda the error locator polynomial (little-endian)
     * @param n      codeword length
     * @return sorted array of error positions (0-indexed, big-endian byte order)
     */
    private static int[] chienSearch(int[] lambda, int n) {
        int[] positions = new int[n];
        int count = 0;
        for (int p = 0; p < n; p++) {
            int xiInv = invLocator(p, n);
            if (evalLE(lambda, xiInv) == 0) {
                positions[count++] = p;
            }
        }
        return java.util.Arrays.copyOf(positions, count);
    }

    // =========================================================================
    // Forney Algorithm
    // =========================================================================

    /**
     * Forney Algorithm: compute error magnitudes from error positions.
     *
     * <p>For each error at position {@code p}:
     * <pre>
     *   e_p = Ω(X_p⁻¹) / Λ'(X_p⁻¹)
     * </pre>
     *
     * <p>Where:
     * <ul>
     *   <li>{@code Ω(x) = (S(x) · Λ(x)) mod x^{2t}} — error evaluator polynomial</li>
     *   <li>{@code S(x) = S₁ + S₂x + … + S_{2t}x^{2t-1}} — syndrome polynomial (LE)</li>
     *   <li>{@code Λ'(x)} — formal derivative of Λ in GF(2^8)</li>
     * </ul>
     *
     * <h3>Formal derivative in characteristic 2</h3>
     *
     * <p>In GF(2^8), multiplying by 2 = 0, so even-degree terms vanish under
     * differentiation. Only odd-indexed coefficients of Λ survive:
     * <pre>
     *   Λ'(x) = Λ₁ + Λ₃x² + Λ₅x⁴ + …
     * </pre>
     *
     * @param lambda    error locator polynomial (little-endian)
     * @param synds     syndrome array (length = 2t)
     * @param positions error positions
     * @param n         codeword length
     * @return error magnitudes (one per position)
     * @throws RsTooManyErrorsException if the Forney denominator evaluates to zero
     */
    private static int[] forney(int[] lambda, int[] synds, int[] positions, int n) {
        int twoT = synds.length;

        // Ω = (S(x) · Λ(x)) mod x^{2t}: keep only the first 2t coefficients.
        // synds is already in LE form (synds[0] = S₁, synds[1] = S₂, …).
        int[] omegaFull = mulLE(synds, lambda);
        int[] omega = java.util.Arrays.copyOf(omegaFull, Math.min(omegaFull.length, twoT));

        // Formal derivative Λ'(x): take only odd-indexed coefficients, shifted down by 1.
        // Λ'[j-1] = Λ[j] when j is odd (1, 3, 5, …).
        int[] lambdaPrime = new int[Math.max(0, lambda.length - 1)];
        for (int j = 1; j < lambda.length; j++) {
            if (j % 2 == 1) {
                lambdaPrime[j - 1] ^= lambda[j];
            }
        }

        int[] magnitudes = new int[positions.length];
        for (int idx = 0; idx < positions.length; idx++) {
            int pos = positions[idx];
            int xiInv = invLocator(pos, n);
            int omegaVal = evalLE(omega, xiInv);
            int lpVal    = evalLE(lambdaPrime, xiInv);
            if (lpVal == 0) throw new RsTooManyErrorsException();
            magnitudes[idx] = GF256.div(omegaVal, lpVal);
        }

        return magnitudes;
    }

    // =========================================================================
    // Decoding
    // =========================================================================

    /**
     * Decode a received Reed-Solomon codeword, correcting up to {@code t = nCheck/2} errors.
     *
     * <p>Decoding pipeline:
     * <pre>
     *   received bytes
     *     │
     *     ▼ Step 1: Compute syndromes S₁…S_{nCheck}
     *     │         all zero? → return message directly
     *     │
     *     ▼ Step 2: Berlekamp-Massey → Λ(x), error count numErrors
     *     │         numErrors > t? → TooManyErrors
     *     │
     *     ▼ Step 3: Chien search → error positions {p₁…pᵥ}
     *     │         |positions| ≠ numErrors? → TooManyErrors
     *     │
     *     ▼ Step 4: Forney → error magnitudes {e₁…eᵥ}
     *     │
     *     ▼ Step 5: Correct: received[p_k] ^= e_k for each k
     *     │
     *     ▼ Return first k = len - nCheck bytes
     * </pre>
     *
     * @param received codeword bytes (possibly corrupted)
     * @param nCheck   number of check bytes (must be even and ≥ 2)
     * @return recovered message bytes (length = received.length - nCheck)
     * @throws RsInvalidInputException   if nCheck is 0/odd or received is too short
     * @throws RsTooManyErrorsException  if more than t errors are present
     */
    public static byte[] decode(byte[] received, int nCheck) {
        if (nCheck == 0 || nCheck % 2 != 0) {
            throw new RsInvalidInputException(
                "nCheck must be a positive even number, got " + nCheck);
        }
        if (received.length < nCheck) {
            throw new RsInvalidInputException(
                "received length " + received.length + " < nCheck " + nCheck);
        }

        int t = nCheck / 2;
        int n = received.length;
        int k = n - nCheck;

        // Step 1: Syndromes
        int[] synds = syndromes(received, nCheck);
        boolean allZero = true;
        for (int s : synds) {
            if (s != 0) { allZero = false; break; }
        }
        if (allZero) {
            return java.util.Arrays.copyOf(received, k);
        }

        // Step 2: Berlekamp-Massey
        int[][] bmResult = berlekampMassey(synds);
        int[] lambda     = bmResult[0];
        int numErrors    = bmResult[1][0];
        if (numErrors > t) throw new RsTooManyErrorsException();

        // Step 3: Chien Search
        int[] positions = chienSearch(lambda, n);
        if (positions.length != numErrors) throw new RsTooManyErrorsException();

        // Step 4: Forney — compute error magnitudes
        int[] magnitudes = forney(lambda, synds, positions, n);

        // Step 5: Apply corrections
        byte[] corrected = java.util.Arrays.copyOf(received, received.length);
        for (int i = 0; i < positions.length; i++) {
            corrected[positions[i]] ^= (byte) magnitudes[i];
        }

        return java.util.Arrays.copyOf(corrected, k);
    }

    /**
     * Compute the error locator polynomial from a syndrome array.
     *
     * <p>Exposed for advanced use cases (QR decoders, diagnostics).
     * Returns Λ(x) in little-endian form with {@code Λ[0] = 1}.
     *
     * @param synds syndrome array (length = nCheck)
     * @return error locator polynomial in little-endian form
     */
    public static int[] errorLocator(int[] synds) {
        return berlekampMassey(synds)[0];
    }
}
