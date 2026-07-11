// ============================================================================
// Sha256.kt — SHA-256 cryptographic hash (FIPS 180-4), from scratch
// ============================================================================
//
// SHA-256 maps any sequence of bytes to a fixed 32-byte (256-bit) digest. The
// same input always yields the same digest; flipping one input bit changes the
// digest completely (the "avalanche effect"); and the digest cannot be reversed
// to the input. It is the workhorse of TLS, git, Bitcoin, and code signing.
//
// This is a from-scratch implementation (no java.security.MessageDigest), the
// Kotlin port of the `sha256` package; it produces byte-identical digests to
// the Rust/Java/Dart ports.
//
// 32-bit arithmetic:
// ------------------
// SHA-256 is defined over unsigned 32-bit words. Kotlin's `Int` is a 32-bit
// two's-complement value whose `+` and bitwise operators (`and`, `or`, `xor`,
// `inv`) wrap and mix exactly as unsigned 32-bit arithmetic requires. We use
// `ushr` (the logical, zero-filling right shift) for SHR and `rotateRight` for
// rotations. No explicit masking is needed.

package com.codingadventures.sha256

object Sha256 {

    // Initial hash values: first 32 bits of the fractional parts of the square
    // roots of the first 8 primes. Constants above 0x7FFFFFFF are Long literals
    // in Kotlin, so `.toInt()` truncates each to its exact 32-bit pattern.
    private val INIT = intArrayOf(
        0x6A09E667.toInt(), 0xBB67AE85.toInt(), 0x3C6EF372.toInt(), 0xA54FF53A.toInt(),
        0x510E527F.toInt(), 0x9B05688C.toInt(), 0x1F83D9AB.toInt(), 0x5BE0CD19.toInt(),
    )

    // Round constants: first 32 bits of the fractional parts of the cube roots
    // of the first 64 primes.
    private val K = intArrayOf(
        0x428A2F98.toInt(), 0x71374491.toInt(), 0xB5C0FBCF.toInt(), 0xE9B5DBA5.toInt(),
        0x3956C25B.toInt(), 0x59F111F1.toInt(), 0x923F82A4.toInt(), 0xAB1C5ED5.toInt(),
        0xD807AA98.toInt(), 0x12835B01.toInt(), 0x243185BE.toInt(), 0x550C7DC3.toInt(),
        0x72BE5D74.toInt(), 0x80DEB1FE.toInt(), 0x9BDC06A7.toInt(), 0xC19BF174.toInt(),
        0xE49B69C1.toInt(), 0xEFBE4786.toInt(), 0x0FC19DC6.toInt(), 0x240CA1CC.toInt(),
        0x2DE92C6F.toInt(), 0x4A7484AA.toInt(), 0x5CB0A9DC.toInt(), 0x76F988DA.toInt(),
        0x983E5152.toInt(), 0xA831C66D.toInt(), 0xB00327C8.toInt(), 0xBF597FC7.toInt(),
        0xC6E00BF3.toInt(), 0xD5A79147.toInt(), 0x06CA6351.toInt(), 0x14292967.toInt(),
        0x27B70A85.toInt(), 0x2E1B2138.toInt(), 0x4D2C6DFC.toInt(), 0x53380D13.toInt(),
        0x650A7354.toInt(), 0x766A0ABB.toInt(), 0x81C2C92E.toInt(), 0x92722C85.toInt(),
        0xA2BFE8A1.toInt(), 0xA81A664B.toInt(), 0xC24B8B70.toInt(), 0xC76C51A3.toInt(),
        0xD192E819.toInt(), 0xD6990624.toInt(), 0xF40E3585.toInt(), 0x106AA070.toInt(),
        0x19A4C116.toInt(), 0x1E376C08.toInt(), 0x2748774C.toInt(), 0x34B0BCB5.toInt(),
        0x391C0CB3.toInt(), 0x4ED8AA4A.toInt(), 0x5B9CCA4F.toInt(), 0x682E6FF3.toInt(),
        0x748F82EE.toInt(), 0x78A5636F.toInt(), 0x84C87814.toInt(), 0x8CC70208.toInt(),
        0x90BEFFFA.toInt(), 0xA4506CEB.toInt(), 0xBEF9A3F7.toInt(), 0xC67178F2.toInt(),
    )

    private fun ch(x: Int, y: Int, z: Int) = (x and y) xor (x.inv() and z)
    private fun maj(x: Int, y: Int, z: Int) = (x and y) xor (x and z) xor (y and z)
    private fun bigSigma0(x: Int) = x.rotateRight(2) xor x.rotateRight(13) xor x.rotateRight(22)
    private fun bigSigma1(x: Int) = x.rotateRight(6) xor x.rotateRight(11) xor x.rotateRight(25)
    private fun smallSigma0(x: Int) = x.rotateRight(7) xor x.rotateRight(18) xor (x ushr 3)
    private fun smallSigma1(x: Int) = x.rotateRight(17) xor x.rotateRight(19) xor (x ushr 10)

    // Fold one 64-byte block (at `offset`) into the eight-word state.
    private fun compress(state: IntArray, data: ByteArray, offset: Int) {
        val w = IntArray(64)
        for (i in 0 until 16) {
            val j = offset + i * 4
            w[i] = ((data[j].toInt() and 0xff) shl 24) or
                ((data[j + 1].toInt() and 0xff) shl 16) or
                ((data[j + 2].toInt() and 0xff) shl 8) or
                (data[j + 3].toInt() and 0xff)
        }
        for (t in 16 until 64) {
            w[t] = smallSigma1(w[t - 2]) + w[t - 7] + smallSigma0(w[t - 15]) + w[t - 16]
        }

        var a = state[0]; var b = state[1]; var c = state[2]; var d = state[3]
        var e = state[4]; var f = state[5]; var g = state[6]; var h = state[7]

        for (t in 0 until 64) {
            val t1 = h + bigSigma1(e) + ch(e, f, g) + K[t] + w[t]
            val t2 = bigSigma0(a) + maj(a, b, c)
            h = g; g = f; f = e; e = d + t1
            d = c; c = b; b = a; a = t1 + t2
        }

        state[0] += a; state[1] += b; state[2] += c; state[3] += d
        state[4] += e; state[5] += f; state[6] += g; state[7] += h
    }

    // Serialise eight 32-bit state words into a big-endian 32-byte digest.
    private fun stateToDigest(state: IntArray): ByteArray {
        val out = ByteArray(32)
        for (i in 0 until 8) {
            out[i * 4] = (state[i] ushr 24).toByte()
            out[i * 4 + 1] = (state[i] ushr 16).toByte()
            out[i * 4 + 2] = (state[i] ushr 8).toByte()
            out[i * 4 + 3] = state[i].toByte()
        }
        return out
    }

    // Pad: append 0x80, zero-fill to 56 (mod 64), then the bit length as a
    // 64-bit big-endian integer (FIPS 180-4 §5.1.1).
    private fun pad(tail: ByteArray, totalBytes: Long): ByteArray {
        val bitLen = totalBytes * 8L
        var padLen = 1
        while ((tail.size + padLen) % 64 != 56) padLen++
        val padded = ByteArray(tail.size + padLen + 8)
        tail.copyInto(padded, 0)
        padded[tail.size] = 0x80.toByte()
        for (i in 0 until 8) {
            padded[padded.size - 1 - i] = (bitLen ushr (i * 8)).toByte()
        }
        return padded
    }

    // ── Public one-shot API ─────────────────────────────────────────────────

    /** Compute the SHA-256 digest of [data] as a 32-byte array. */
    fun sha256(data: ByteArray): ByteArray {
        val padded = pad(data, data.size.toLong())
        val state = INIT.copyOf()
        var off = 0
        while (off < padded.size) {
            compress(state, padded, off)
            off += 64
        }
        return stateToDigest(state)
    }

    /** Compute SHA-256 and return the 64-character lowercase hex string. */
    fun sha256Hex(data: ByteArray): String = toHex(sha256(data))

    internal fun toHex(bytes: ByteArray): String {
        val sb = StringBuilder(bytes.size * 2)
        for (b in bytes) {
            val v = b.toInt() and 0xff
            sb.append("0123456789abcdef"[v ushr 4])
            sb.append("0123456789abcdef"[v and 0xf])
        }
        return sb.toString()
    }

    /**
     * Incremental SHA-256. Feed data with [update]; [digest] is non-destructive
     * so the hasher can keep receiving updates afterwards.
     */
    class Hasher private constructor(
        private val state: IntArray,
        private var buf: ByteArray,
        private var byteCount: Long,
    ) {
        constructor() : this(INIT.copyOf(), ByteArray(0), 0L)

        /** Feed more bytes into the hash. */
        fun update(data: ByteArray) {
            byteCount += data.size
            val combined = buf + data
            val full = combined.size - (combined.size % 64)
            var off = 0
            while (off < full) {
                compress(state, combined, off)
                off += 64
            }
            buf = combined.copyOfRange(full, combined.size)
        }

        /** Return the 32-byte digest of all data fed so far (non-destructive). */
        fun digest(): ByteArray {
            val tail = pad(buf, byteCount)
            val s = state.copyOf()
            var off = 0
            while (off < tail.size) {
                compress(s, tail, off)
                off += 64
            }
            return stateToDigest(s)
        }

        /** Return the 64-character lowercase hex digest string. */
        fun hexDigest(): String = toHex(digest())

        /** Return an independent copy of this hasher's current state. */
        fun copy(): Hasher = Hasher(state.copyOf(), buf.copyOf(), byteCount)
    }
}
