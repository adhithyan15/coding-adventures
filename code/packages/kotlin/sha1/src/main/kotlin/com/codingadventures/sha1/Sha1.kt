// ============================================================================
// Sha1.kt — SHA-1 cryptographic hash (FIPS 180-4), from scratch
// ============================================================================
//
// SHA-1 maps any sequence of bytes to a fixed 20-byte (160-bit) digest. Like
// MD5 and SHA-256 it uses the Merkle–Damgård construction over 64-byte blocks,
// but with five state words and 80 rounds. This is a from-scratch
// implementation (no java.security.MessageDigest), the Kotlin port of the `sha1`
// package; it produces byte-identical digests to the Rust/Java/Dart ports.
//
// SECURITY: SHA-1 is BROKEN for collision resistance (the SHAttered attack,
// 2017). Never use it for signatures or certificates. Legacy protocols and
// non-adversarial checksums (e.g. git object names) only.
//
// 32-bit arithmetic:
// ------------------
// SHA-1 is big-endian (like SHA-256, the opposite of MD5). Kotlin's `Int` is a
// 32-bit two's-complement value whose arithmetic and bitwise operators match
// unsigned 32-bit semantics, so no masking is needed; `Int.rotateLeft` performs
// the rotations and block bytes are masked with `and 0xff` before shifting.

package com.codingadventures.sha1

object Sha1 {

    // Initial state H0..H4.
    private val INIT = intArrayOf(
        0x67452301, 0xEFCDAB89.toInt(), 0x98BADCFE.toInt(), 0x10325476, 0xC3D2E1F0.toInt(),
    )

    // Per-stage round constants: floor(sqrt(2,3,5,10) * 2^30).
    private val K = intArrayOf(0x5A827999, 0x6ED9EBA1, 0x8F1BBCDC.toInt(), 0xCA62C1D6.toInt())

    // Fold one 64-byte block (at `offset`) into the five-word state over 80 rounds.
    private fun compress(state: IntArray, data: ByteArray, offset: Int) {
        val w = IntArray(80)
        for (i in 0 until 16) {
            val j = offset + i * 4
            w[i] = ((data[j].toInt() and 0xff) shl 24) or
                ((data[j + 1].toInt() and 0xff) shl 16) or
                ((data[j + 2].toInt() and 0xff) shl 8) or
                (data[j + 3].toInt() and 0xff)
        }
        for (i in 16 until 80) {
            w[i] = (w[i - 3] xor w[i - 8] xor w[i - 14] xor w[i - 16]).rotateLeft(1)
        }

        var a = state[0]; var b = state[1]; var c = state[2]; var d = state[3]; var e = state[4]

        for (t in 0 until 80) {
            val f: Int
            val k: Int
            when {
                t < 20 -> { f = (b and c) or (b.inv() and d); k = K[0] }
                t < 40 -> { f = b xor c xor d; k = K[1] }
                t < 60 -> { f = (b and c) or (b and d) or (c and d); k = K[2] }
                else -> { f = b xor c xor d; k = K[3] }
            }
            val temp = a.rotateLeft(5) + f + e + k + w[t]
            e = d; d = c; c = b.rotateLeft(30); b = a; a = temp
        }

        state[0] += a; state[1] += b; state[2] += c; state[3] += d; state[4] += e
    }

    // Serialise the five state words into a 20-byte big-endian digest.
    private fun stateToDigest(state: IntArray): ByteArray {
        val out = ByteArray(20)
        for (i in 0 until 5) {
            out[i * 4] = (state[i] ushr 24).toByte()
            out[i * 4 + 1] = (state[i] ushr 16).toByte()
            out[i * 4 + 2] = (state[i] ushr 8).toByte()
            out[i * 4 + 3] = state[i].toByte()
        }
        return out
    }

    // Pad: append 0x80, zero-fill to 56 (mod 64), then the bit length as a
    // 64-bit big-endian integer (FIPS 180-4).
    private fun pad(tail: ByteArray, totalBytes: Long): ByteArray {
        val bitLen = totalBytes * 8L
        var padLen = 1
        while ((tail.size + padLen) % 64 != 56) padLen++
        val padded = ByteArray(tail.size + padLen + 8)
        tail.copyInto(padded, 0)
        padded[tail.size] = 0x80.toByte()
        for (i in 0 until 8) {
            padded[padded.size - 1 - i] = (bitLen ushr (i * 8)).toByte() // big-endian
        }
        return padded
    }

    // ── Public one-shot API ─────────────────────────────────────────────────

    /** Compute the SHA-1 digest of [data] as a 20-byte array. */
    fun sum1(data: ByteArray): ByteArray {
        val padded = pad(data, data.size.toLong())
        val state = INIT.copyOf()
        var off = 0
        while (off < padded.size) {
            compress(state, padded, off)
            off += 64
        }
        return stateToDigest(state)
    }

    /** Compute SHA-1 and return the 40-character lowercase hex string. */
    fun hexString(data: ByteArray): String = toHex(sum1(data))

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
     * Incremental SHA-1. Feed data with [update]; [digest] is non-destructive so
     * the hasher can keep receiving updates afterwards.
     */
    class Digest private constructor(
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

        /** Return the 20-byte digest of all data fed so far (non-destructive). */
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

        /** Return the 40-character lowercase hex digest string. */
        fun hexDigest(): String = toHex(digest())

        /** Return an independent copy of this hasher's current state. */
        fun copy(): Digest = Digest(state.copyOf(), buf.copyOf(), byteCount)
    }
}
