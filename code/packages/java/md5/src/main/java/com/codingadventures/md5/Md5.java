// ============================================================================
// Md5.java — MD5 message-digest algorithm (RFC 1321), from scratch
// ============================================================================
//
// MD5 maps any sequence of bytes to a fixed 16-byte (128-bit) digest. This is a
// from-scratch implementation (no java.security.MessageDigest), the Java port of
// the `md5` package in the coding-adventures monorepo; it produces
// byte-identical digests to the Rust/Python/Dart ports.
//
// SECURITY: MD5 is cryptographically BROKEN — practical collision attacks exist.
// Never use it for signatures, certificates, or password storage. It remains a
// fast, non-adversarial integrity checksum.
//
// Little-endian:
// -------------
// Unlike SHA-1/SHA-256, MD5 is little-endian: block words are read with the
// first byte as the least-significant, the length is appended little-endian, and
// the digest words are emitted least-significant byte first.
//
// 32-bit arithmetic:
// ------------------
// MD5 is defined over unsigned 32-bit words. Java's `int` is a 32-bit
// two's-complement value whose `+` and bitwise operators wrap and mix exactly as
// unsigned 32-bit arithmetic requires, so no masking is needed;
// `Integer.rotateLeft` performs the circular rotations.

package com.codingadventures.md5;

public final class Md5 {

    // Per-round left-rotation amounts (four repeating groups of four).
    private static final int[] S = {
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
        5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
        4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
        6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21
    };

    // The 64 sine-derived constants: T[i] = floor(abs(sin(i+1)) * 2^32).
    private static final int[] T = {
        0xD76AA478, 0xE8C7B756, 0x242070DB, 0xC1BDCEEE, 0xF57C0FAF, 0x4787C62A,
        0xA8304613, 0xFD469501, 0x698098D8, 0x8B44F7AF, 0xFFFF5BB1, 0x895CD7BE,
        0x6B901122, 0xFD987193, 0xA679438E, 0x49B40821, 0xF61E2562, 0xC040B340,
        0x265E5A51, 0xE9B6C7AA, 0xD62F105D, 0x02441453, 0xD8A1E681, 0xE7D3FBC8,
        0x21E1CDE6, 0xC33707D6, 0xF4D50D87, 0x455A14ED, 0xA9E3E905, 0xFCEFA3F8,
        0x676F02D9, 0x8D2A4C8A, 0xFFFA3942, 0x8771F681, 0x6D9D6122, 0xFDE5380C,
        0xA4BEEA44, 0x4BDECFA9, 0xF6BB4B60, 0xBEBFBC70, 0x289B7EC6, 0xEAA127FA,
        0xD4EF3085, 0x04881D05, 0xD9D4D039, 0xE6DB99E5, 0x1FA27CF8, 0xC4AC5665,
        0xF4292244, 0x432AFF97, 0xAB9423A7, 0xFC93A039, 0x655B59C3, 0x8F0CCC92,
        0xFFEFF47D, 0x85845DD1, 0x6FA87E4F, 0xFE2CE6E0, 0xA3014314, 0x4E0811A1,
        0xF7537E82, 0xBD3AF235, 0x2AD7D2BB, 0xEB86D391
    };

    // Initial state (A, B, C, D). Little-endian bytes spell 01 23 45 67 …
    private static final int[] INIT = {0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476};

    private Md5() {}

    // ── Compression ─────────────────────────────────────────────────────────
    //
    // Fold one 64-byte block (at `offset`) into the four-word state over 64
    // rounds. Four stages use different auxiliary functions and message-word
    // schedules; each round rotates the register set (a,b,c,d) ← (d,temp,b,c).
    private static void compress(int[] state, byte[] data, int offset) {
        int[] m = new int[16];
        for (int i = 0; i < 16; i++) {
            int j = offset + i * 4;
            // Little-endian: byte j is the least-significant byte of word i.
            m[i] = (data[j] & 0xff)
                 | ((data[j + 1] & 0xff) << 8)
                 | ((data[j + 2] & 0xff) << 16)
                 | ((data[j + 3] & 0xff) << 24);
        }

        int a = state[0], b = state[1], c = state[2], d = state[3];

        for (int i = 0; i < 64; i++) {
            int f, g;
            if (i < 16) {
                f = (b & c) | (~b & d);
                g = i;
            } else if (i < 32) {
                f = (d & b) | (~d & c);
                g = (5 * i + 1) & 15;
            } else if (i < 48) {
                f = b ^ c ^ d;
                g = (3 * i + 5) & 15;
            } else {
                f = c ^ (b | ~d);
                g = (7 * i) & 15;
            }
            int sum = a + f + m[g] + T[i];
            int temp = b + Integer.rotateLeft(sum, S[i]);
            a = d;
            d = c;
            c = b;
            b = temp;
        }

        state[0] += a; state[1] += b; state[2] += c; state[3] += d;
    }

    // Serialise the four state words into a 16-byte little-endian digest.
    private static byte[] stateToDigest(int[] state) {
        byte[] out = new byte[16];
        for (int i = 0; i < 4; i++) {
            out[i * 4]     = (byte) (state[i]);
            out[i * 4 + 1] = (byte) (state[i] >>> 8);
            out[i * 4 + 2] = (byte) (state[i] >>> 16);
            out[i * 4 + 3] = (byte) (state[i] >>> 24);
        }
        return out;
    }

    // Pad: append 0x80, zero-fill to 56 (mod 64), then the bit length as a
    // 64-bit LITTLE-endian integer (RFC 1321 §3).
    private static byte[] pad(byte[] tail, long totalBytes) {
        long bitLen = totalBytes * 8L;
        int padLen = 1;
        while ((tail.length + padLen) % 64 != 56) padLen++;
        byte[] padded = new byte[tail.length + padLen + 8];
        System.arraycopy(tail, 0, padded, 0, tail.length);
        padded[tail.length] = (byte) 0x80;
        for (int i = 0; i < 8; i++) {
            padded[tail.length + padLen + i] = (byte) (bitLen >>> (i * 8)); // little-endian
        }
        return padded;
    }

    // ── Public one-shot API ─────────────────────────────────────────────────

    /** Compute the MD5 digest of {@code data} as a 16-byte array. */
    public static byte[] sumMd5(byte[] data) {
        byte[] padded = pad(data, data.length);
        int[] state = INIT.clone();
        for (int off = 0; off < padded.length; off += 64) {
            compress(state, padded, off);
        }
        return stateToDigest(state);
    }

    /** Compute MD5 and return the 32-character lowercase hex string. */
    public static String hexString(byte[] data) {
        return toHex(sumMd5(data));
    }

    static String toHex(byte[] bytes) {
        StringBuilder sb = new StringBuilder(bytes.length * 2);
        for (byte b : bytes) {
            sb.append(Character.forDigit((b >>> 4) & 0xf, 16));
            sb.append(Character.forDigit(b & 0xf, 16));
        }
        return sb.toString();
    }

    // ── Streaming digest ────────────────────────────────────────────────────

    /**
     * Incremental MD5. Feed data with {@link #update}; {@link #digest} is
     * non-destructive so the hasher can keep receiving updates afterwards.
     */
    public static final class Digest {
        private final int[] state;
        private byte[] buf;
        private long byteCount;

        public Digest() {
            this.state = INIT.clone();
            this.buf = new byte[0];
            this.byteCount = 0;
        }

        private Digest(int[] state, byte[] buf, long byteCount) {
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

        /** Return the 16-byte digest of all data fed so far (non-destructive). */
        public byte[] digest() {
            byte[] tail = pad(buf, byteCount);
            int[] s = state.clone();
            for (int off = 0; off < tail.length; off += 64) {
                compress(s, tail, off);
            }
            return stateToDigest(s);
        }

        /** Return the 32-character lowercase hex digest string. */
        public String hexDigest() {
            return toHex(digest());
        }

        /** Return an independent copy of this hasher's current state. */
        public Digest copy() {
            return new Digest(state, buf, byteCount);
        }
    }
}
