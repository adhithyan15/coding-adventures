// ============================================================================
// Md5.kt — MD5 message-digest algorithm (RFC 1321), from scratch
// ============================================================================
//
// MD5 maps any sequence of bytes to a fixed 16-byte (128-bit) digest. This is a
// from-scratch implementation (no java.security.MessageDigest), the Kotlin port
// of the `md5` package; it produces byte-identical digests to the
// Rust/Java/Dart ports.
//
// SECURITY: MD5 is cryptographically BROKEN — practical collision attacks exist.
// Never use it for signatures, certificates, or password storage. It remains a
// fast, non-adversarial integrity checksum.
//
// Little-endian & 32-bit arithmetic:
// ----------------------------------
// Unlike SHA-1/SHA-256, MD5 is little-endian: block words are read least-
// significant byte first, the length is appended little-endian, and the digest
// words are emitted least-significant byte first. Kotlin's `Int` is a 32-bit
// two's-complement value whose `+` and bitwise operators match unsigned 32-bit
// semantics, so no masking is needed; `Int.rotateLeft` performs the rotations.

package com.codingadventures.md5

object Md5 {

    // Per-round left-rotation amounts (four repeating groups of four).
    private val S = intArrayOf(
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
        5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
        4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
        6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    )

    // The 64 sine-derived constants: T[i] = floor(abs(sin(i+1)) * 2^32).
    // Constants above 0x7FFFFFFF are Long literals in Kotlin, so `.toInt()`
    // truncates each to its exact 32-bit pattern.
    private val T = intArrayOf(
        0xD76AA478.toInt(), 0xE8C7B756.toInt(), 0x242070DB.toInt(), 0xC1BDCEEE.toInt(),
        0xF57C0FAF.toInt(), 0x4787C62A.toInt(), 0xA8304613.toInt(), 0xFD469501.toInt(),
        0x698098D8.toInt(), 0x8B44F7AF.toInt(), 0xFFFF5BB1.toInt(), 0x895CD7BE.toInt(),
        0x6B901122.toInt(), 0xFD987193.toInt(), 0xA679438E.toInt(), 0x49B40821.toInt(),
        0xF61E2562.toInt(), 0xC040B340.toInt(), 0x265E5A51.toInt(), 0xE9B6C7AA.toInt(),
        0xD62F105D.toInt(), 0x02441453.toInt(), 0xD8A1E681.toInt(), 0xE7D3FBC8.toInt(),
        0x21E1CDE6.toInt(), 0xC33707D6.toInt(), 0xF4D50D87.toInt(), 0x455A14ED.toInt(),
        0xA9E3E905.toInt(), 0xFCEFA3F8.toInt(), 0x676F02D9.toInt(), 0x8D2A4C8A.toInt(),
        0xFFFA3942.toInt(), 0x8771F681.toInt(), 0x6D9D6122.toInt(), 0xFDE5380C.toInt(),
        0xA4BEEA44.toInt(), 0x4BDECFA9.toInt(), 0xF6BB4B60.toInt(), 0xBEBFBC70.toInt(),
        0x289B7EC6.toInt(), 0xEAA127FA.toInt(), 0xD4EF3085.toInt(), 0x04881D05.toInt(),
        0xD9D4D039.toInt(), 0xE6DB99E5.toInt(), 0x1FA27CF8.toInt(), 0xC4AC5665.toInt(),
        0xF4292244.toInt(), 0x432AFF97.toInt(), 0xAB9423A7.toInt(), 0xFC93A039.toInt(),
        0x655B59C3.toInt(), 0x8F0CCC92.toInt(), 0xFFEFF47D.toInt(), 0x85845DD1.toInt(),
        0x6FA87E4F.toInt(), 0xFE2CE6E0.toInt(), 0xA3014314.toInt(), 0x4E0811A1.toInt(),
        0xF7537E82.toInt(), 0xBD3AF235.toInt(), 0x2AD7D2BB.toInt(), 0xEB86D391.toInt(),
    )

    // Initial state (A, B, C, D).
    private val INIT = intArrayOf(0x67452301, 0xEFCDAB89.toInt(), 0x98BADCFE.toInt(), 0x10325476)

    // Fold one 64-byte block (at `offset`) into the four-word state over 64 rounds.
    private fun compress(state: IntArray, data: ByteArray, offset: Int) {
        val m = IntArray(16)
        for (i in 0 until 16) {
            val j = offset + i * 4
            // Little-endian: byte j is the least-significant byte of word i.
            m[i] = (data[j].toInt() and 0xff) or
                ((data[j + 1].toInt() and 0xff) shl 8) or
                ((data[j + 2].toInt() and 0xff) shl 16) or
                ((data[j + 3].toInt() and 0xff) shl 24)
        }

        var a = state[0]; var b = state[1]; var c = state[2]; var d = state[3]

        for (i in 0 until 64) {
            val f: Int
            val g: Int
            when {
                i < 16 -> { f = (b and c) or (b.inv() and d); g = i }
                i < 32 -> { f = (d and b) or (d.inv() and c); g = (5 * i + 1) and 15 }
                i < 48 -> { f = b xor c xor d; g = (3 * i + 5) and 15 }
                else -> { f = c xor (b or d.inv()); g = (7 * i) and 15 }
            }
            val sum = a + f + m[g] + T[i]
            val temp = b + sum.rotateLeft(S[i])
            a = d; d = c; c = b; b = temp
        }

        state[0] += a; state[1] += b; state[2] += c; state[3] += d
    }

    // Serialise the four state words into a 16-byte little-endian digest.
    private fun stateToDigest(state: IntArray): ByteArray {
        val out = ByteArray(16)
        for (i in 0 until 4) {
            out[i * 4] = state[i].toByte()
            out[i * 4 + 1] = (state[i] ushr 8).toByte()
            out[i * 4 + 2] = (state[i] ushr 16).toByte()
            out[i * 4 + 3] = (state[i] ushr 24).toByte()
        }
        return out
    }

    // Pad: append 0x80, zero-fill to 56 (mod 64), then the bit length as a
    // 64-bit LITTLE-endian integer (RFC 1321 §3).
    private fun pad(tail: ByteArray, totalBytes: Long): ByteArray {
        val bitLen = totalBytes * 8L
        var padLen = 1
        while ((tail.size + padLen) % 64 != 56) padLen++
        val padded = ByteArray(tail.size + padLen + 8)
        tail.copyInto(padded, 0)
        padded[tail.size] = 0x80.toByte()
        for (i in 0 until 8) {
            padded[tail.size + padLen + i] = (bitLen ushr (i * 8)).toByte() // little-endian
        }
        return padded
    }

    // ── Public one-shot API ─────────────────────────────────────────────────

    /** Compute the MD5 digest of [data] as a 16-byte array. */
    fun sumMd5(data: ByteArray): ByteArray {
        val padded = pad(data, data.size.toLong())
        val state = INIT.copyOf()
        var off = 0
        while (off < padded.size) {
            compress(state, padded, off)
            off += 64
        }
        return stateToDigest(state)
    }

    /** Compute MD5 and return the 32-character lowercase hex string. */
    fun hexString(data: ByteArray): String = toHex(sumMd5(data))

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
     * Incremental MD5. Feed data with [update]; [digest] is non-destructive so
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

        /** Return the 16-byte digest of all data fed so far (non-destructive). */
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

        /** Return the 32-character lowercase hex digest string. */
        fun hexDigest(): String = toHex(digest())

        /** Return an independent copy of this hasher's current state. */
        fun copy(): Digest = Digest(state.copyOf(), buf.copyOf(), byteCount)
    }
}
