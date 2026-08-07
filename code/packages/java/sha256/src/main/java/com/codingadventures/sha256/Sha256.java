// ============================================================================
// Sha256.java — SHA-256 cryptographic hash (FIPS 180-4), from scratch
// ============================================================================
//
// SHA-256 maps any sequence of bytes to a fixed 32-byte (256-bit) digest. The
// same input always yields the same digest; flipping one input bit changes the
// digest completely (the "avalanche effect"); and the digest cannot be reversed
// to the input. It is the workhorse of TLS, git, Bitcoin, and code signing.
//
// This is a from-scratch implementation (no java.security.MessageDigest) so
// every step — padding, the message schedule, the 64-round compression — is
// visible. It is the Java port of the `sha256` package in the coding-adventures
// monorepo and produces byte-identical digests to the Rust/Python/Dart ports.
//
// 32-bit arithmetic:
// ------------------
// SHA-256 is defined over unsigned 32-bit words. Java's `int` is a 32-bit
// two's-complement value, and its `+`, `<<`, and `&`/`^`/`|` operators wrap and
// mix bits exactly as unsigned 32-bit arithmetic requires. We use `>>>` (the
// logical, zero-filling right shift) for SHR, and `Integer.rotateRight` for
// rotations. No explicit masking is needed — that is the advantage of a native
// 32-bit integer type.

package com.codingadventures.sha256;

public final class Sha256 {

    // ── Constants ───────────────────────────────────────────────────────────

    // Initial hash values: the first 32 bits of the fractional parts of the
    // square roots of the first 8 primes. "Nothing up my sleeve" numbers.
    private static final int[] INIT = {
        0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A,
        0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19
    };

    // Round constants: the first 32 bits of the fractional parts of the cube
    // roots of the first 64 primes. 64 distinct constants, one per round.
    private static final int[] K = {
        0x428A2F98, 0x71374491, 0xB5C0FBCF, 0xE9B5DBA5, 0x3956C25B, 0x59F111F1,
        0x923F82A4, 0xAB1C5ED5, 0xD807AA98, 0x12835B01, 0x243185BE, 0x550C7DC3,
        0x72BE5D74, 0x80DEB1FE, 0x9BDC06A7, 0xC19BF174, 0xE49B69C1, 0xEFBE4786,
        0x0FC19DC6, 0x240CA1CC, 0x2DE92C6F, 0x4A7484AA, 0x5CB0A9DC, 0x76F988DA,
        0x983E5152, 0xA831C66D, 0xB00327C8, 0xBF597FC7, 0xC6E00BF3, 0xD5A79147,
        0x06CA6351, 0x14292967, 0x27B70A85, 0x2E1B2138, 0x4D2C6DFC, 0x53380D13,
        0x650A7354, 0x766A0ABB, 0x81C2C92E, 0x92722C85, 0xA2BFE8A1, 0xA81A664B,
        0xC24B8B70, 0xC76C51A3, 0xD192E819, 0xD6990624, 0xF40E3585, 0x106AA070,
        0x19A4C116, 0x1E376C08, 0x2748774C, 0x34B0BCB5, 0x391C0CB3, 0x4ED8AA4A,
        0x5B9CCA4F, 0x682E6FF3, 0x748F82EE, 0x78A5636F, 0x84C87814, 0x8CC70208,
        0x90BEFFFA, 0xA4506CEB, 0xBEF9A3F7, 0xC67178F2
    };

    private Sha256() {} // static utility class; no instances

    // ── Auxiliary functions ─────────────────────────────────────────────────

    private static int ch(int x, int y, int z)  { return (x & y) ^ (~x & z); }
    private static int maj(int x, int y, int z)  { return (x & y) ^ (x & z) ^ (y & z); }
    private static int bigSigma0(int x)   { return rotr(x, 2)  ^ rotr(x, 13) ^ rotr(x, 22); }
    private static int bigSigma1(int x)   { return rotr(x, 6)  ^ rotr(x, 11) ^ rotr(x, 25); }
    private static int smallSigma0(int x) { return rotr(x, 7)  ^ rotr(x, 18) ^ (x >>> 3); }
    private static int smallSigma1(int x) { return rotr(x, 17) ^ rotr(x, 19) ^ (x >>> 10); }

    private static int rotr(int x, int n) { return Integer.rotateRight(x, n); }

    // ── Compression ─────────────────────────────────────────────────────────
    //
    // Fold one 64-byte block (at `offset` in `data`) into the eight-word state.
    // The 16 big-endian words of the block are expanded to a 64-word schedule,
    // then 64 rounds mix them in, and the Davies–Meyer feed-forward adds the
    // result back onto the input state.
    private static void compress(int[] state, byte[] data, int offset) {
        int[] w = new int[64];
        for (int i = 0; i < 16; i++) {
            int j = offset + i * 4;
            w[i] = ((data[j] & 0xff) << 24)
                 | ((data[j + 1] & 0xff) << 16)
                 | ((data[j + 2] & 0xff) << 8)
                 |  (data[j + 3] & 0xff);
        }
        for (int t = 16; t < 64; t++) {
            w[t] = smallSigma1(w[t - 2]) + w[t - 7] + smallSigma0(w[t - 15]) + w[t - 16];
        }

        int a = state[0], b = state[1], c = state[2], d = state[3];
        int e = state[4], f = state[5], g = state[6], h = state[7];

        for (int t = 0; t < 64; t++) {
            int t1 = h + bigSigma1(e) + ch(e, f, g) + K[t] + w[t];
            int t2 = bigSigma0(a) + maj(a, b, c);
            h = g; g = f; f = e; e = d + t1;
            d = c; c = b; b = a; a = t1 + t2;
        }

        state[0] += a; state[1] += b; state[2] += c; state[3] += d;
        state[4] += e; state[5] += f; state[6] += g; state[7] += h;
    }

    // Serialise eight 32-bit state words into a big-endian 32-byte digest.
    private static byte[] stateToDigest(int[] state) {
        byte[] out = new byte[32];
        for (int i = 0; i < 8; i++) {
            out[i * 4]     = (byte) (state[i] >>> 24);
            out[i * 4 + 1] = (byte) (state[i] >>> 16);
            out[i * 4 + 2] = (byte) (state[i] >>> 8);
            out[i * 4 + 3] = (byte) (state[i]);
        }
        return out;
    }

    // Build the padded message: append 0x80, zero-fill to 56 (mod 64), then the
    // original bit length as a 64-bit big-endian integer (FIPS 180-4 §5.1.1).
    private static byte[] pad(byte[] tail, long totalBytes) {
        long bitLen = totalBytes * 8L;
        int padLen = 1;
        while ((tail.length + padLen) % 64 != 56) padLen++;
        byte[] padded = new byte[tail.length + padLen + 8];
        System.arraycopy(tail, 0, padded, 0, tail.length);
        padded[tail.length] = (byte) 0x80;
        for (int i = 0; i < 8; i++) {
            padded[padded.length - 1 - i] = (byte) (bitLen >>> (i * 8));
        }
        return padded;
    }

    // ── Public one-shot API ─────────────────────────────────────────────────

    /** Compute the SHA-256 digest of {@code data} as a 32-byte array. */
    public static byte[] sha256(byte[] data) {
        byte[] padded = pad(data, data.length);
        int[] state = INIT.clone();
        for (int off = 0; off < padded.length; off += 64) {
            compress(state, padded, off);
        }
        return stateToDigest(state);
    }

    /** Compute SHA-256 and return the 64-character lowercase hex string. */
    public static String sha256Hex(byte[] data) {
        return toHex(sha256(data));
    }

    static String toHex(byte[] bytes) {
        StringBuilder sb = new StringBuilder(bytes.length * 2);
        for (byte b : bytes) {
            sb.append(Character.forDigit((b >>> 4) & 0xf, 16));
            sb.append(Character.forDigit(b & 0xf, 16));
        }
        return sb.toString();
    }

    // ── Streaming hasher ────────────────────────────────────────────────────

    /**
     * Incremental SHA-256. Feed data with {@link #update}; {@link #digest} is
     * non-destructive so the hasher can keep receiving updates afterwards.
     */
    public static final class Hasher {
        private final int[] state;
        private byte[] buf;      // unprocessed bytes (< 64)
        private long byteCount;  // total bytes fed

        public Hasher() {
            this.state = INIT.clone();
            this.buf = new byte[0];
            this.byteCount = 0;
        }

        private Hasher(int[] state, byte[] buf, long byteCount) {
            this.state = state.clone();
            this.buf = buf.clone();
            this.byteCount = byteCount;
        }

        /** Feed more bytes into the hash. */
        public void update(byte[] data) {
            byteCount += data.length;
            byte[] combined = new byte[buf.length + data.length];
            System.arraycopy(buf, 0, combined, 0, buf.length);
            System.arraycopy(data, 0, combined, buf.length, data.length);
            int full = combined.length - (combined.length % 64);
            for (int off = 0; off < full; off += 64) {
                compress(state, combined, off);
            }
            buf = new byte[combined.length - full];
            System.arraycopy(combined, full, buf, 0, buf.length);
        }

        /** Return the 32-byte digest of all data fed so far (non-destructive). */
        public byte[] digest() {
            byte[] tail = pad(buf, byteCount);
            int[] s = state.clone();
            for (int off = 0; off < tail.length; off += 64) {
                compress(s, tail, off);
            }
            return stateToDigest(s);
        }

        /** Return the 64-character lowercase hex digest string. */
        public String hexDigest() {
            return toHex(digest());
        }

        /** Return an independent copy of this hasher's current state. */
        public Hasher copy() {
            return new Hasher(state, buf, byteCount);
        }
    }
}
