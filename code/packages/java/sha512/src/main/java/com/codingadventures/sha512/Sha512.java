package com.codingadventures.sha512;

import java.util.Arrays;

/**
 * Pure Java implementation of the SHA-512 cryptographic hash function (FIPS 180-4).
 * Adheres to the HF04 streaming contract, zero-host crypto imports, and Standing Rule #20.
 */
public final class Sha512 {

    public static final int DIGEST_LENGTH = Sha512Constants.DIGEST_LENGTH;

    private static final char[] HEX_CHARS = new char[] {
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f'
    };

    private Sha512() {
    }

    /**
     * Compute the 64-byte SHA-512 digest of the provided byte array.
     */
    public static byte[] sha512(byte[] data) {
        return create().update(data).digest();
    }

    /**
     * Compute the 128-character lowercase hexadecimal SHA-512 digest of the provided byte array.
     */
    public static String sha512Hex(byte[] data) {
        return create().update(data).hexDigest();
    }

    /**
     * Factory for incremental, streaming Hasher instances.
     */
    public static Hasher create() {
        return new Hasher();
    }

    /**
     * Encodes 128-bit message length in big-endian bit format per FIPS 180-4 §5.1.2.
     * Enforces the 2^128-bit maximum domain bound before bit-shift transformation.
     */
    static byte[] encodeBitLength(long highU64, long lowU64) {
        // Enforce FIPS domain bound: bits 61..63 of high 64-bit counter must be 0
        if ((highU64 & Sha512Constants.OVERFLOW_MASK) != 0L) {
            throw new Sha512Exception(Sha512ErrorCode.MESSAGE_TOO_LONG);
        }
        // Multiply 128-bit byte count by 8 to produce bit length, carrying 3 bits from low to high
        long bitHigh = (highU64 << Sha512Constants.BITS_PER_BYTE_SHIFT)
            | (lowU64 >>> Sha512Constants.BYTE_TO_BIT_CARRY_SHIFT);
        long bitLow = lowU64 << Sha512Constants.BITS_PER_BYTE_SHIFT;

        byte[] result = new byte[Sha512Constants.LENGTH_FIELD_BYTES];
        putLongBigEndian(result, 0, bitHigh);
        putLongBigEndian(result, Sha512Constants.WORD_BYTES, bitLow);
        return result;
    }

    private static void putLongBigEndian(byte[] out, int offset, long value) {
        out[offset] = (byte) (value >>> 56);
        out[offset + 1] = (byte) (value >>> 48);
        out[offset + 2] = (byte) (value >>> 40);
        out[offset + 3] = (byte) (value >>> 32);
        out[offset + 4] = (byte) (value >>> 24);
        out[offset + 5] = (byte) (value >>> 16);
        out[offset + 6] = (byte) (value >>> 8);
        out[offset + 7] = (byte) value;
    }

    static String bytesToHex(byte[] bytes) {
        char[] chars = new char[bytes.length * 2];
        for (int i = 0; i < bytes.length; i++) {
            int v = bytes[i] & (int) Sha512Constants.BYTE_MASK;
            chars[i * 2] = HEX_CHARS[v >>> Sha512Constants.NIBBLE_BITS];
            chars[i * 2 + 1] = HEX_CHARS[v & Sha512Constants.NIBBLE_MASK];
        }
        return new String(chars);
    }

    /**
     * Incremental streaming SHA-512 hasher complying with the HF04 contract.
     */
    public static final class Hasher {
        private final long[] state = new long[Sha512Constants.STATE_WORDS];
        private final byte[] buffer = new byte[Sha512Constants.BLOCK_SIZE];
        private int bufferLength;
        private long byteCountHigh;
        private long byteCountLow;

        public Hasher() {
            System.arraycopy(Sha512Constants.INITIAL_STATE, 0, this.state, 0, Sha512Constants.STATE_WORDS);
        }

        public Hasher(Hasher other) {
            if (other == null) {
                throw new NullPointerException(Sha512Messages.ERR_NULL_DATA);
            }
            System.arraycopy(other.state, 0, this.state, 0, Sha512Constants.STATE_WORDS);
            System.arraycopy(other.buffer, 0, this.buffer, 0, other.bufferLength);
            this.bufferLength = other.bufferLength;
            this.byteCountHigh = other.byteCountHigh;
            this.byteCountLow = other.byteCountLow;
        }

        void setByteCountersForTesting(long high, long low) {
            this.byteCountHigh = high;
            this.byteCountLow = low;
        }

        public Hasher update(byte[] data) {
            if (data == null) {
                throw new NullPointerException(Sha512Messages.ERR_NULL_DATA);
            }
            return update(data, 0, data.length);
        }

        public Hasher update(byte[] data, int offset, int length) {
            // Null check fail-closed
            if (data == null) {
                throw new NullPointerException(Sha512Messages.ERR_NULL_DATA);
            }
            // Overflow-safe bounds checking prevents integer wrapping attacks
            if (offset < 0 || length < 0 || offset > data.length || length > data.length - offset) {
                throw new IndexOutOfBoundsException(Sha512Messages.ERR_INDEX_OUT_OF_BOUNDS);
            }
            if (length == 0) {
                return this;
            }

            // Failure-atomic 128-bit domain check before mutating internal state
            long nextLow = byteCountLow + length;
            long nextHigh = byteCountHigh;
            if (Long.compareUnsigned(nextLow, byteCountLow) < 0) {
                nextHigh++;
            }
            if ((nextHigh & Sha512Constants.OVERFLOW_MASK) != 0L) {
                throw new Sha512Exception(Sha512ErrorCode.MESSAGE_TOO_LONG);
            }

            // Commit counter advances only after domain bound validation succeeds
            byteCountLow = nextLow;
            byteCountHigh = nextHigh;

            int currentOffset = offset;
            int remaining = length;

            // Drain partial buffer if present
            if (bufferLength > 0) {
                int needed = Sha512Constants.BLOCK_SIZE - bufferLength;
                int toCopy = Math.min(needed, remaining);
                System.arraycopy(data, currentOffset, buffer, bufferLength, toCopy);
                bufferLength += toCopy;
                currentOffset += toCopy;
                remaining -= toCopy;

                if (bufferLength == Sha512Constants.BLOCK_SIZE) {
                    compress(state, buffer, 0);
                    bufferLength = 0;
                }
            }

            // Process full 128-byte blocks directly from caller array without copying
            while (remaining >= Sha512Constants.BLOCK_SIZE) {
                compress(state, data, currentOffset);
                currentOffset += Sha512Constants.BLOCK_SIZE;
                remaining -= Sha512Constants.BLOCK_SIZE;
            }

            // Stash remaining partial block in internal buffer
            if (remaining > 0) {
                System.arraycopy(data, currentOffset, buffer, 0, remaining);
                bufferLength = remaining;
            }

            return this;
        }

        /**
         * Non-destructive snapshot of the hash state at the current byte position.
         */
        public byte[] digest() {
            // Snapshot working state so caller can continue streaming afterward
            long[] snapshotState = Arrays.copyOf(state, Sha512Constants.STATE_WORDS);
            byte[] tail = new byte[Sha512Constants.BLOCK_SIZE * 2];
            System.arraycopy(buffer, 0, tail, 0, bufferLength);
            tail[bufferLength] = Sha512Constants.PADDING_MARKER;

            // Zero-pad until tail length reaches 112 mod 128
            int padLen = bufferLength + 1;
            while ((padLen % Sha512Constants.BLOCK_SIZE) != Sha512Constants.PADDING_OFFSET) {
                tail[padLen++] = 0;
            }

            // Append 128-bit big-endian bit length
            byte[] lenBytes = encodeBitLength(byteCountHigh, byteCountLow);
            System.arraycopy(lenBytes, 0, tail, padLen, Sha512Constants.LENGTH_FIELD_BYTES);
            padLen += Sha512Constants.LENGTH_FIELD_BYTES;

            // Compress 1 or 2 final padded blocks
            for (int i = 0; i < padLen; i += Sha512Constants.BLOCK_SIZE) {
                compress(snapshotState, tail, i);
            }

            // Serialize 8 words into 64-byte big-endian digest
            byte[] digest = new byte[Sha512Constants.DIGEST_LENGTH];
            for (int j = 0; j < Sha512Constants.STATE_WORDS; j++) {
                putLongBigEndian(digest, j * Sha512Constants.WORD_BYTES, snapshotState[j]);
            }
            return digest;
        }

        public String hexDigest() {
            return bytesToHex(digest());
        }

        public Hasher copy() {
            return new Hasher(this);
        }

        private static long rotr(long x, int n) {
            return (x >>> n) | (x << (Sha512Constants.WORD_BITS - n));
        }

        private static long bigSigma0(long x) {
            return rotr(x, Sha512Constants.BS0_R1)
                ^ rotr(x, Sha512Constants.BS0_R2)
                ^ rotr(x, Sha512Constants.BS0_R3);
        }

        private static long bigSigma1(long x) {
            return rotr(x, Sha512Constants.BS1_R1)
                ^ rotr(x, Sha512Constants.BS1_R2)
                ^ rotr(x, Sha512Constants.BS1_R3);
        }

        private static long smallSigma0(long x) {
            return rotr(x, Sha512Constants.SS0_R1)
                ^ rotr(x, Sha512Constants.SS0_R2)
                ^ (x >>> Sha512Constants.SS0_S);
        }

        private static long smallSigma1(long x) {
            return rotr(x, Sha512Constants.SS1_R1)
                ^ rotr(x, Sha512Constants.SS1_R2)
                ^ (x >>> Sha512Constants.SS1_S);
        }

        private static long ch(long x, long y, long z) {
            return (x & y) ^ (~x & z);
        }

        private static long maj(long x, long y, long z) {
            return (x & y) ^ (x & z) ^ (y & z);
        }

        private static void compress(long[] s, byte[] block, int offset) {
            long[] w = new long[Sha512Constants.ROUNDS];

            // Initialize first 16 words from big-endian block bytes
            for (int i = 0; i < Sha512Constants.SCHEDULE_INIT_WORDS; i++) {
                int idx = offset + i * Sha512Constants.WORD_BYTES;
                w[i] = ((block[idx] & Sha512Constants.BYTE_MASK) << 56)
                    | ((block[idx + 1] & Sha512Constants.BYTE_MASK) << 48)
                    | ((block[idx + 2] & Sha512Constants.BYTE_MASK) << 40)
                    | ((block[idx + 3] & Sha512Constants.BYTE_MASK) << 32)
                    | ((block[idx + 4] & Sha512Constants.BYTE_MASK) << 24)
                    | ((block[idx + 5] & Sha512Constants.BYTE_MASK) << 16)
                    | ((block[idx + 6] & Sha512Constants.BYTE_MASK) << 8)
                    | (block[idx + 7] & Sha512Constants.BYTE_MASK);
            }

            // Expand 16-word message schedule to 80 rounds
            for (int t = Sha512Constants.SCHEDULE_INIT_WORDS; t < Sha512Constants.ROUNDS; t++) {
                w[t] = smallSigma1(w[t - 2]) + w[t - 7] + smallSigma0(w[t - 15]) + w[t - 16];
            }

            // Load working variables (one declaration per line)
            long a = s[0];
            long b = s[1];
            long c = s[2];
            long d = s[3];
            long e = s[4];
            long f = s[5];
            long g = s[6];
            long h = s[7];

            // Execute 80 mixing rounds
            for (int t = 0; t < Sha512Constants.ROUNDS; t++) {
                long t1 = h + bigSigma1(e) + ch(e, f, g) + Sha512Constants.K[t] + w[t];
                long t2 = bigSigma0(a) + maj(a, b, c);
                h = g;
                g = f;
                f = e;
                e = d + t1;
                d = c;
                c = b;
                b = a;
                a = t1 + t2;
            }

            // Feed-forward accumulation into state
            s[0] += a;
            s[1] += b;
            s[2] += c;
            s[3] += d;
            s[4] += e;
            s[5] += f;
            s[6] += g;
            s[7] += h;
        }
    }
}
