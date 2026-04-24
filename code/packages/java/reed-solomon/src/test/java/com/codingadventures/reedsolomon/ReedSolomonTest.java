package com.codingadventures.reedsolomon;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

import java.util.Arrays;
import java.util.Random;

/**
 * Unit tests for Reed-Solomon encoding and decoding.
 *
 * <p>Tests are organized by concern:
 * <ol>
 *   <li>Generator polynomial construction — verify structure and roots.</li>
 *   <li>Encoding — systematic structure, round-trip sanity.</li>
 *   <li>Syndrome computation — zero syndromes for valid codewords.</li>
 *   <li>Decoding — no errors, single error, multiple errors, too many errors.</li>
 *   <li>Input validation — nCheck constraints, length constraints.</li>
 *   <li>Standard QR test vector — the Version 1-L standard vector.</li>
 * </ol>
 *
 * <p>All test vectors are cross-validated against the TypeScript reference implementation
 * (code/packages/typescript/reed-solomon).
 */
class ReedSolomonTest {

    // =========================================================================
    // buildGenerator
    // =========================================================================

    /**
     * Generator for nCheck=2 must be [8, 6, 1] (= x² + 6x + 8 over GF(256)).
     *
     * From spec MA02 worked example: g(x) = (x+2)(x+4).
     * Coefficients: constant = GF256.mul(2,4) = 8, linear = 2 XOR 4 = 6, leading = 1.
     */
    @Test
    void buildGenerator_nCheck2() {
        int[] g = ReedSolomon.buildGenerator(2);
        assertArrayEquals(new int[]{8, 6, 1}, g);
    }

    /**
     * Generator polynomial must be monic (leading coefficient = 1).
     */
    @Test
    void buildGenerator_isMonic() {
        for (int nCheck = 2; nCheck <= 16; nCheck += 2) {
            int[] g = ReedSolomon.buildGenerator(nCheck);
            assertEquals(nCheck + 1, g.length,
                "generator length should be nCheck + 1 for nCheck=" + nCheck);
            assertEquals(1, g[g.length - 1],
                "generator must be monic for nCheck=" + nCheck);
        }
    }

    /**
     * g(α^i) = 0 for i = 1..nCheck: the generator's roots are the required powers of α.
     *
     * This is the fundamental property of the RS generator polynomial.
     */
    @Test
    void buildGenerator_correctRoots() {
        int[] g = ReedSolomon.buildGenerator(4);
        int alpha = 2;
        // Evaluate g at each of its 4 roots: α¹, α², α³, α⁴.
        for (int i = 1; i <= 4; i++) {
            int x = pow(alpha, i);
            int val = evalLE(g, x);
            assertEquals(0, val,
                "g(α^" + i + ") must be 0 for nCheck=4 generator");
        }
    }

    /**
     * buildGenerator throws on odd nCheck.
     */
    @Test
    void buildGenerator_oddThrows() {
        assertThrows(RsInvalidInputException.class, () -> ReedSolomon.buildGenerator(3));
        assertThrows(RsInvalidInputException.class, () -> ReedSolomon.buildGenerator(1));
    }

    /**
     * buildGenerator throws on zero nCheck.
     */
    @Test
    void buildGenerator_zeroThrows() {
        assertThrows(RsInvalidInputException.class, () -> ReedSolomon.buildGenerator(0));
    }

    // =========================================================================
    // encode
    // =========================================================================

    /**
     * Encoding is systematic: the first k bytes of the codeword equal the message.
     */
    @Test
    void encode_systematic() {
        byte[] message = new byte[]{4, 3, 2, 1};
        int nCheck = 2;
        byte[] codeword = ReedSolomon.encode(message, nCheck);

        assertEquals(message.length + nCheck, codeword.length,
            "codeword length must be message.length + nCheck");
        // First k bytes are the original message unchanged.
        for (int i = 0; i < message.length; i++) {
            assertEquals(message[i], codeword[i],
                "codeword[" + i + "] must equal message[" + i + "]");
        }
    }

    /**
     * Encoding a single-byte message produces the right number of check bytes.
     */
    @Test
    void encode_singleByte() {
        byte[] message = new byte[]{42};
        int nCheck = 4;
        byte[] codeword = ReedSolomon.encode(message, nCheck);
        assertEquals(1 + nCheck, codeword.length);
        assertEquals(42, codeword[0] & 0xFF, "message byte must be preserved");
    }

    /**
     * Encoding with nCheck=0 throws.
     */
    @Test
    void encode_nCheckZeroThrows() {
        assertThrows(RsInvalidInputException.class,
            () -> ReedSolomon.encode(new byte[]{1, 2, 3}, 0));
    }

    /**
     * Encoding with odd nCheck throws.
     */
    @Test
    void encode_nCheckOddThrows() {
        assertThrows(RsInvalidInputException.class,
            () -> ReedSolomon.encode(new byte[]{1, 2, 3}, 3));
    }

    /**
     * Encoding with total length > 255 throws.
     */
    @Test
    void encode_tooLongThrows() {
        byte[] msg = new byte[252];
        assertThrows(RsInvalidInputException.class,
            () -> ReedSolomon.encode(msg, 4));  // 252 + 4 = 256 > 255
    }

    // =========================================================================
    // syndromes
    // =========================================================================

    /**
     * All syndromes of a valid codeword must be zero.
     */
    @Test
    void syndromes_zeroForValidCodeword() {
        byte[] message = new byte[]{1, 2, 3, 4, 5};
        int nCheck = 8;
        byte[] codeword = ReedSolomon.encode(message, nCheck);
        int[] s = ReedSolomon.syndromes(codeword, nCheck);
        for (int i = 0; i < s.length; i++) {
            assertEquals(0, s[i], "syndrome[" + i + "] must be 0 for valid codeword");
        }
    }

    /**
     * Syndromes of a corrupted codeword must have at least one non-zero entry.
     */
    @Test
    void syndromes_nonZeroForCorrupted() {
        byte[] message = new byte[]{10, 20, 30};
        int nCheck = 4;
        byte[] codeword = ReedSolomon.encode(message, nCheck);
        codeword[0] ^= 0xFF;   // corrupt first byte
        int[] s = ReedSolomon.syndromes(codeword, nCheck);
        boolean anyNonZero = false;
        for (int v : s) { if (v != 0) anyNonZero = true; }
        assertTrue(anyNonZero, "corrupted codeword must have non-zero syndrome");
    }

    // =========================================================================
    // decode — no errors
    // =========================================================================

    /**
     * decode(encode(message)) == message for various sizes and nCheck values.
     */
    @Test
    void decode_roundTrip_noErrors() {
        int[][] cases = {{4, 2}, {4, 4}, {4, 8}, {10, 4}, {20, 8}, {50, 16}};
        for (int[] c : cases) {
            int k = c[0], nCheck = c[1];
            byte[] message = new byte[k];
            for (int i = 0; i < k; i++) message[i] = (byte) (i + 1);
            byte[] codeword = ReedSolomon.encode(message, nCheck);
            byte[] decoded  = ReedSolomon.decode(codeword, nCheck);
            assertArrayEquals(message, decoded,
                "round-trip failed for k=" + k + ", nCheck=" + nCheck);
        }
    }

    // =========================================================================
    // decode — error correction
    // =========================================================================

    /**
     * A single error at position 0 is corrected.
     */
    @Test
    void decode_singleErrorAtStart() {
        byte[] message  = new byte[]{1, 2, 3, 4, 5};
        int    nCheck   = 4;
        byte[] codeword = ReedSolomon.encode(message, nCheck);

        byte[] corrupted = Arrays.copyOf(codeword, codeword.length);
        corrupted[0] ^= (byte) 0xAB;

        byte[] decoded = ReedSolomon.decode(corrupted, nCheck);
        assertArrayEquals(message, decoded, "single error at position 0 must be corrected");
    }

    /**
     * A single error in the check bytes is corrected.
     */
    @Test
    void decode_singleErrorInCheckBytes() {
        byte[] message  = new byte[]{10, 20, 30};
        int    nCheck   = 4;
        byte[] codeword = ReedSolomon.encode(message, nCheck);

        byte[] corrupted = Arrays.copyOf(codeword, codeword.length);
        corrupted[codeword.length - 1] ^= (byte) 0x55;  // last check byte

        byte[] decoded = ReedSolomon.decode(corrupted, nCheck);
        assertArrayEquals(message, decoded, "single error in check bytes must be corrected");
    }

    /**
     * t errors are corrected (capacity test): with nCheck=8, t=4 errors.
     */
    @Test
    void decode_atCapacity() {
        byte[] message  = new byte[]{1, 2, 3, 4, 5};
        int    nCheck   = 8;   // t = 4
        byte[] codeword = ReedSolomon.encode(message, nCheck);

        byte[] corrupted = Arrays.copyOf(codeword, codeword.length);
        // Corrupt exactly 4 positions (capacity limit).
        corrupted[0] ^= 0x11;
        corrupted[2] ^= 0x22;
        corrupted[5] ^= 0x33;
        corrupted[8] ^= 0x44;

        byte[] decoded = ReedSolomon.decode(corrupted, nCheck);
        assertArrayEquals(message, decoded, "t=4 errors must be corrected with nCheck=8");
    }

    /**
     * Two errors with nCheck=4 (t=2) are corrected.
     */
    @Test
    void decode_twoErrors() {
        byte[] message  = new byte[]{100, 101, 102, 103, 104};
        int    nCheck   = 4;   // t = 2
        byte[] codeword = ReedSolomon.encode(message, nCheck);

        byte[] corrupted = Arrays.copyOf(codeword, codeword.length);
        corrupted[1] ^= 0xFF;
        corrupted[3] ^= 0xAA;

        byte[] decoded = ReedSolomon.decode(corrupted, nCheck);
        assertArrayEquals(message, decoded, "two errors with nCheck=4 must be corrected");
    }

    /**
     * Round-trip with random messages and errors (fuzz test with fixed seed).
     */
    @Test
    void decode_randomRoundTrip() {
        Random rng = new Random(42);
        int nCheck = 8;
        int t = nCheck / 2;

        for (int trial = 0; trial < 20; trial++) {
            int k = 5 + rng.nextInt(15);
            byte[] message = new byte[k];
            rng.nextBytes(message);

            byte[] codeword  = ReedSolomon.encode(message, nCheck);
            byte[] corrupted = Arrays.copyOf(codeword, codeword.length);

            // Inject exactly t random errors at distinct positions.
            int n = corrupted.length;
            java.util.Set<Integer> positions = new java.util.HashSet<>();
            while (positions.size() < t) {
                positions.add(rng.nextInt(n));
            }
            for (int p : positions) {
                int errVal = 1 + rng.nextInt(255);  // non-zero error
                corrupted[p] ^= (byte) errVal;
            }

            byte[] decoded = ReedSolomon.decode(corrupted, nCheck);
            assertArrayEquals(message, decoded,
                "random trial " + trial + " failed");
        }
    }

    // =========================================================================
    // decode — too many errors
    // =========================================================================

    /**
     * t+1 errors must throw TooManyErrorsException (not silently return wrong data).
     */
    @Test
    void decode_tooManyErrors() {
        byte[] message  = new byte[]{1, 2, 3, 4, 5};
        int    nCheck   = 4;   // t = 2
        byte[] codeword = ReedSolomon.encode(message, nCheck);

        byte[] corrupted = Arrays.copyOf(codeword, codeword.length);
        // Corrupt t+1 = 3 positions.
        corrupted[0] ^= 0x01;
        corrupted[2] ^= 0x02;
        corrupted[4] ^= 0x03;

        assertThrows(RsTooManyErrorsException.class,
            () -> ReedSolomon.decode(corrupted, nCheck),
            "t+1 errors must throw TooManyErrorsException");
    }

    // =========================================================================
    // decode — invalid input
    // =========================================================================

    /**
     * decode with nCheck=0 or odd throws RsInvalidInputException.
     */
    @Test
    void decode_invalidNCheck() {
        byte[] codeword = new byte[10];
        assertThrows(RsInvalidInputException.class,
            () -> ReedSolomon.decode(codeword, 0));
        assertThrows(RsInvalidInputException.class,
            () -> ReedSolomon.decode(codeword, 3));
    }

    /**
     * decode with received shorter than nCheck throws.
     */
    @Test
    void decode_receivedTooShort() {
        assertThrows(RsInvalidInputException.class,
            () -> ReedSolomon.decode(new byte[3], 4));
    }

    // =========================================================================
    // Standard QR test vector
    // =========================================================================

    /**
     * QR Version 1-L standard encoding vector.
     *
     * From spec MA02: encode the 19-byte data sequence with nCheck=7.
     * Note: nCheck=7 is odd — the spec's QR example uses nCheck=7 EC bytes.
     * QR code RS actually uses any nCheck (odd or even); the even constraint
     * exists in this implementation for simplicity.  To test the actual QR
     * vector we use nCheck=8 (which the TypeScript reference also supports).
     *
     * Here we verify the core invariants with nCheck=8 against a known input.
     */
    @Test
    void encode_qrLike_roundTrip() {
        // Simulated QR-like data bytes (19 bytes, similar to Version 1-L).
        byte[] message = {
            32, 91, 11, 120, (byte)209, 114, (byte)220, 77, 67, 64,
            (byte)236, 17, (byte)236, 17, (byte)236, 17, (byte)236, 17, (byte)236
        };
        int nCheck = 8;
        byte[] codeword = ReedSolomon.encode(message, nCheck);

        // Systematic: first 19 bytes = message.
        for (int i = 0; i < message.length; i++) {
            assertEquals(message[i], codeword[i],
                "message byte " + i + " not preserved");
        }
        // Total length = 27 bytes.
        assertEquals(message.length + nCheck, codeword.length);

        // All syndromes must be zero.
        int[] s = ReedSolomon.syndromes(codeword, nCheck);
        for (int i = 0; i < s.length; i++) {
            assertEquals(0, s[i], "syndrome[" + i + "] != 0");
        }

        // Round-trip decode must recover the message.
        byte[] decoded = ReedSolomon.decode(codeword, nCheck);
        assertArrayEquals(message, decoded, "QR-like round-trip failed");
    }

    /**
     * Error correction with the QR-like vector: corrupt up to 4 bytes and recover.
     */
    @Test
    void decode_qrLike_errorCorrection() {
        byte[] message = {
            32, 91, 11, 120, (byte)209, 114, (byte)220, 77, 67, 64,
            (byte)236, 17, (byte)236, 17, (byte)236, 17, (byte)236, 17, (byte)236
        };
        int nCheck = 8;   // t = 4
        byte[] codeword  = ReedSolomon.encode(message, nCheck);
        byte[] corrupted = Arrays.copyOf(codeword, codeword.length);

        // Corrupt 4 bytes (at capacity).
        corrupted[0]  ^= 0xFF;
        corrupted[7]  ^= 0x42;
        corrupted[14] ^= 0x99;
        corrupted[20] ^= 0x11;

        byte[] decoded = ReedSolomon.decode(corrupted, nCheck);
        assertArrayEquals(message, decoded, "QR-like 4-error correction failed");
    }

    // =========================================================================
    // errorLocator
    // =========================================================================

    /**
     * errorLocator on zero syndromes returns [1] (no errors → trivial LFSR).
     */
    @Test
    void errorLocator_noErrors() {
        int[] zeroSynds = new int[4];
        int[] lambda = ReedSolomon.errorLocator(zeroSynds);
        assertArrayEquals(new int[]{1}, lambda,
            "errorLocator with no errors must return [1]");
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    /** Evaluate a little-endian GF(256) polynomial at x (for test use). */
    private static int evalLE(int[] p, int x) {
        int acc = 0;
        for (int i = p.length - 1; i >= 0; i--) {
            acc = com.codingadventures.gf256.GF256.add(
                com.codingadventures.gf256.GF256.mul(acc, x), p[i]);
        }
        return acc;
    }

    /** GF(256) power: α^n. */
    private static int pow(int a, int n) {
        return com.codingadventures.gf256.GF256.pow(a, n);
    }
}
