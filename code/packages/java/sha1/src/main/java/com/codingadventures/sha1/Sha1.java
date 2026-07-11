// ============================================================================
// Sha1.java — SHA-1 cryptographic hash (FIPS 180-4), from scratch
// ============================================================================
//
// SHA-1 maps any sequence of bytes to a fixed 20-byte (160-bit) digest. Like
// MD5 and SHA-256 it uses the Merkle–Damgård construction over 64-byte blocks,
// but with five state words and 80 rounds. This is a from-scratch
// implementation (no java.security.MessageDigest), the Java port of the `sha1`
// package; it produces byte-identical digests to the Rust/Dart ports.
//
// SECURITY: SHA-1 is BROKEN for collision resistance (the SHAttered attack,
// 2017). Never use it for signatures or certificates. Legacy protocols and
// non-adversarial checksums (e.g. git object names) only.
//
// 32-bit arithmetic:
// ------------------
// SHA-1 is big-endian (like SHA-256, the opposite of MD5). Java's `int` is a
// 32-bit two's-complement value whose arithmetic and bitwise operators match
// unsigned 32-bit semantics, so no masking is needed; `Integer.rotateLeft`
// performs the rotations and block bytes are masked with `& 0xff` before shifting.

package com.codingadventures.sha1;

public final class Sha1 {

    // Initial state H0..H4 ("nothing up my sleeve" counting-sequence constants).
    private static final int[] INIT = {0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0};

    // Per-stage round constants: floor(sqrt(2,3,5,10) * 2^30).
    private static final int[] K = {0x5A827999, 0x6ED9EBA1, 0x8F1BBCDC, 0xCA62C1D6};

    private Sha1() {}

    // ── Compression ─────────────────────────────────────────────────────────
    //
    // Fold one 64-byte block (at `offset`) into the five-word state over 80
    // rounds. The 16 big-endian words of the block are expanded to an 80-word
    // schedule, then four stages of 20 rounds each mix them in.
    private static void compress(int[] state, byte[] data, int offset) {
        int[] w = new int[80];
        for (int i = 0; i < 16; i++) {
            int j = offset + i * 4;
            w[i] = ((data[j] & 0xff) << 24)
                 | ((data[j + 1] & 0xff) << 16)
                 | ((data[j + 2] & 0xff) << 8)
                 |  (data[j + 3] & 0xff);
        }
        for (int i = 16; i < 80; i++) {
            w[i] = Integer.rotateLeft(w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16], 1);
        }

        int a = state[0], b = state[1], c = state[2], d = state[3], e = state[4];

        for (int t = 0; t < 80; t++) {
            int f, k;
            if (t < 20) {
                f = (b & c) | (~b & d);
                k = K[0];
            } else if (t < 40) {
                f = b ^ c ^ d;
                k = K[1];
            } else if (t < 60) {
                f = (b & c) | (b & d) | (c & d);
                k = K[2];
            } else {
                f = b ^ c ^ d;
                k = K[3];
            }
            int temp = Integer.rotateLeft(a, 5) + f + e + k + w[t];
            e = d;
            d = c;
            c = Integer.rotateLeft(b, 30);
            b = a;
            a = temp;
        }

        state[0] += a; state[1] += b; state[2] += c; state[3] += d; state[4] += e;
    }

    // Serialise the five state words into a 20-byte big-endian digest.
    private static byte[] stateToDigest(int[] state) {
        byte[] out = new byte[20];
        for (int i = 0; i < 5; i++) {
            out[i * 4]     = (byte) (state[i] >>> 24);
            out[i * 4 + 1] = (byte) (state[i] >>> 16);
            out[i * 4 + 2] = (byte) (state[i] >>> 8);
            out[i * 4 + 3] = (byte) (state[i]);
        }
        return out;
    }

    // Pad: append 0x80, zero-fill to 56 (mod 64), then the bit length as a
    // 64-bit big-endian integer (FIPS 180-4).
    private static byte[] pad(byte[] tail, long totalBytes) {
        long bitLen = totalBytes * 8L;
        int padLen = 1;
        while ((tail.length + padLen) % 64 != 56) padLen++;
        byte[] padded = new byte[tail.length + padLen + 8];
        System.arraycopy(tail, 0, padded, 0, tail.length);
        padded[tail.length] = (byte) 0x80;
        for (int i = 0; i < 8; i++) {
            padded[padded.length - 1 - i] = (byte) (bitLen >>> (i * 8)); // big-endian
        }
        return padded;
    }

    // ── Public one-shot API ─────────────────────────────────────────────────

    /** Compute the SHA-1 digest of {@code data} as a 20-byte array. */
    public static byte[] sum1(byte[] data) {
        byte[] padded = pad(data, data.length);
        int[] state = INIT.clone();
        for (int off = 0; off < padded.length; off += 64) {
            compress(state, padded, off);
        }
        return stateToDigest(state);
    }

    /** Compute SHA-1 and return the 40-character lowercase hex string. */
    public static String hexString(byte[] data) {
        return toHex(sum1(data));
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
     * Incremental SHA-1. Feed data with {@link #update}; {@link #digest} is
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

        /** Return the 20-byte digest of all data fed so far (non-destructive). */
        public byte[] digest() {
            byte[] tail = pad(buf, byteCount);
            int[] s = state.clone();
            for (int off = 0; off < tail.length; off += 64) {
                compress(s, tail, off);
            }
            return stateToDigest(s);
        }

        /** Return the 40-character lowercase hex digest string. */
        public String hexDigest() {
            return toHex(digest());
        }

        /** Return an independent copy of this hasher's current state. */
        public Digest copy() {
            return new Digest(state, buf, byteCount);
        }
    }
}
