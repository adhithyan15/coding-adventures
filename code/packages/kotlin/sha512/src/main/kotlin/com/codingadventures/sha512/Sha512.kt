package com.codingadventures.sha512

private val HEX_CHARS = charArrayOf(
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f'
)

fun sha512(data: ByteArray): ByteArray {
    return Sha512.create().update(data).digest()
}

fun sha512Hex(data: ByteArray): String {
    return Sha512.create().update(data).hexDigest()
}

/**
 * Encodes 128-bit message length in big-endian bit format per FIPS 180-4 §5.1.2.
 * Enforces the 2^128-bit maximum domain bound before bit-shift transformation.
 */
internal fun encodeBitLength(highU64: ULong, lowU64: ULong): ByteArray {
    // Enforce FIPS domain bound: bits 61..63 of high 64-bit counter must be 0
    if ((highU64 and Sha512Constants.OVERFLOW_MASK_ULONG) != 0uL) {
        throw Sha512Exception(Sha512ErrorCode.MESSAGE_TOO_LONG)
    }
    // Multiply 128-bit byte count by 8 to produce bit length, carrying 3 bits from low to high
    val bitHigh = (highU64 shl Sha512Constants.BITS_PER_BYTE_SHIFT) or
        (lowU64 shr Sha512Constants.BYTE_TO_BIT_CARRY_SHIFT)
    val bitLow = lowU64 shl Sha512Constants.BITS_PER_BYTE_SHIFT

    val result = ByteArray(Sha512Constants.LENGTH_FIELD_BYTES)
    putLongBigEndian(result, 0, bitHigh.toLong())
    putLongBigEndian(result, Sha512Constants.WORD_BYTES, bitLow.toLong())
    return result
}

private fun putLongBigEndian(out: ByteArray, offset: Int, value: Long) {
    out[offset] = (value ushr 56).toByte()
    out[offset + 1] = (value ushr 48).toByte()
    out[offset + 2] = (value ushr 40).toByte()
    out[offset + 3] = (value ushr 32).toByte()
    out[offset + 4] = (value ushr 24).toByte()
    out[offset + 5] = (value ushr 16).toByte()
    out[offset + 6] = (value ushr 8).toByte()
    out[offset + 7] = value.toByte()
}

internal fun bytesToHex(bytes: ByteArray): String {
    val chars = CharArray(bytes.size * 2)
    for (i in bytes.indices) {
        val v = bytes[i].toInt() and 0xFF
        chars[i * 2] = HEX_CHARS[v ushr Sha512Constants.NIBBLE_BITS]
        chars[i * 2 + 1] = HEX_CHARS[v and Sha512Constants.NIBBLE_MASK]
    }
    return String(chars)
}

object Sha512 {
    const val DIGEST_LENGTH: Int = Sha512Constants.DIGEST_LENGTH

    fun create(): Hasher = Hasher()

    fun sha512(data: ByteArray): ByteArray = com.codingadventures.sha512.sha512(data)

    fun sha512Hex(data: ByteArray): String = com.codingadventures.sha512.sha512Hex(data)

    /**
     * Incremental streaming SHA-512 hasher complying with the HF04 contract.
     */
    class Hasher() {
        private val state = LongArray(Sha512Constants.STATE_WORDS)
        private val buffer = ByteArray(Sha512Constants.BLOCK_SIZE)
        private var bufferLength = 0
        private var byteCountHigh: ULong = 0uL
        private var byteCountLow: ULong = 0uL

        init {
            Sha512Constants.INITIAL_STATE.copyInto(state)
        }

        constructor(other: Hasher) : this() {
            other.state.copyInto(this.state)
            other.buffer.copyInto(this.buffer, 0, 0, other.bufferLength)
            this.bufferLength = other.bufferLength
            this.byteCountHigh = other.byteCountHigh
            this.byteCountLow = other.byteCountLow
        }

        internal fun setByteCountersForTesting(high: ULong, low: ULong) {
            this.byteCountHigh = high
            this.byteCountLow = low
        }

        fun update(data: ByteArray): Hasher {
            return update(data, 0, data.size)
        }

        fun update(data: ByteArray, offset: Int, length: Int): Hasher {
            // Overflow-safe bounds checking prevents integer wrapping attacks
            if (offset < 0 || length < 0 || offset > data.size || length > data.size - offset) {
                throw IndexOutOfBoundsException(Sha512Messages.ERR_INDEX_OUT_OF_BOUNDS)
            }
            if (length == 0) {
                return this
            }

            // Failure-atomic 128-bit domain check before mutating internal state
            val added = length.toULong()
            val nextLow = byteCountLow + added
            var nextHigh = byteCountHigh
            if (nextLow < byteCountLow) {
                nextHigh++
            }
            if ((nextHigh and Sha512Constants.OVERFLOW_MASK_ULONG) != 0uL) {
                throw Sha512Exception(Sha512ErrorCode.MESSAGE_TOO_LONG)
            }

            // Commit counter advances only after domain bound validation succeeds
            byteCountLow = nextLow
            byteCountHigh = nextHigh

            var currentOffset = offset
            var remaining = length

            // Drain partial buffer if present
            if (bufferLength > 0) {
                val needed = Sha512Constants.BLOCK_SIZE - bufferLength
                val toCopy = minOf(needed, remaining)
                System.arraycopy(data, currentOffset, buffer, bufferLength, toCopy)
                bufferLength += toCopy
                currentOffset += toCopy
                remaining -= toCopy

                if (bufferLength == Sha512Constants.BLOCK_SIZE) {
                    compress(state, buffer, 0)
                    bufferLength = 0
                }
            }

            // Process full 128-byte blocks directly from caller array without copying
            while (remaining >= Sha512Constants.BLOCK_SIZE) {
                compress(state, data, currentOffset)
                currentOffset += Sha512Constants.BLOCK_SIZE
                remaining -= Sha512Constants.BLOCK_SIZE
            }

            // Stash remaining partial block in internal buffer
            if (remaining > 0) {
                System.arraycopy(data, currentOffset, buffer, 0, remaining)
                bufferLength = remaining
            }

            return this
        }

        /**
         * Non-destructive snapshot of the hash state at the current byte position.
         */
        fun digest(): ByteArray {
            // Snapshot working state so caller can continue streaming afterward
            val snapshotState = state.copyOf()
            val tail = ByteArray(Sha512Constants.BLOCK_SIZE * 2)
            System.arraycopy(buffer, 0, tail, 0, bufferLength)
            tail[bufferLength] = Sha512Constants.PADDING_MARKER

            // Zero-pad until tail length reaches 112 mod 128
            var padLen = bufferLength + 1
            while ((padLen % Sha512Constants.BLOCK_SIZE) != Sha512Constants.PADDING_OFFSET) {
                tail[padLen++] = 0
            }

            // Append 128-bit big-endian bit length
            val lenBytes = encodeBitLength(byteCountHigh, byteCountLow)
            System.arraycopy(lenBytes, 0, tail, padLen, Sha512Constants.LENGTH_FIELD_BYTES)
            padLen += Sha512Constants.LENGTH_FIELD_BYTES

            // Compress 1 or 2 final padded blocks
            var i = 0
            while (i < padLen) {
                compress(snapshotState, tail, i)
                i += Sha512Constants.BLOCK_SIZE
            }

            // Serialize 8 words into 64-byte big-endian digest
            val digest = ByteArray(Sha512Constants.DIGEST_LENGTH)
            for (j in 0 until Sha512Constants.STATE_WORDS) {
                putLongBigEndian(digest, j * Sha512Constants.WORD_BYTES, snapshotState[j])
            }
            return digest
        }

        fun hexDigest(): String = bytesToHex(digest())

        fun copy(): Hasher = Hasher(this)

        private fun rotr(x: Long, n: Int): Long {
            return (x ushr n) or (x shl (Sha512Constants.WORD_BITS - n))
        }

        private fun bigSigma0(x: Long): Long {
            return rotr(x, Sha512Constants.BS0_R1) xor
                rotr(x, Sha512Constants.BS0_R2) xor
                rotr(x, Sha512Constants.BS0_R3)
        }

        private fun bigSigma1(x: Long): Long {
            return rotr(x, Sha512Constants.BS1_R1) xor
                rotr(x, Sha512Constants.BS1_R2) xor
                rotr(x, Sha512Constants.BS1_R3)
        }

        private fun smallSigma0(x: Long): Long {
            return rotr(x, Sha512Constants.SS0_R1) xor
                rotr(x, Sha512Constants.SS0_R2) xor
                (x ushr Sha512Constants.SS0_S)
        }

        private fun smallSigma1(x: Long): Long {
            return rotr(x, Sha512Constants.SS1_R1) xor
                rotr(x, Sha512Constants.SS1_R2) xor
                (x ushr Sha512Constants.SS1_S)
        }

        private fun ch(x: Long, y: Long, z: Long): Long = (x and y) xor (x.inv() and z)

        private fun maj(x: Long, y: Long, z: Long): Long = (x and y) xor (x and z) xor (y and z)

        private fun compress(s: LongArray, block: ByteArray, offset: Int) {
            val w = LongArray(Sha512Constants.ROUNDS)

            // Initialize first 16 words from big-endian block bytes
            for (i in 0 until Sha512Constants.SCHEDULE_INIT_WORDS) {
                val idx = offset + i * Sha512Constants.WORD_BYTES
                w[i] = ((block[idx].toLong() and Sha512Constants.BYTE_MASK_LONG) shl 56) or
                    ((block[idx + 1].toLong() and Sha512Constants.BYTE_MASK_LONG) shl 48) or
                    ((block[idx + 2].toLong() and Sha512Constants.BYTE_MASK_LONG) shl 40) or
                    ((block[idx + 3].toLong() and Sha512Constants.BYTE_MASK_LONG) shl 32) or
                    ((block[idx + 4].toLong() and Sha512Constants.BYTE_MASK_LONG) shl 24) or
                    ((block[idx + 5].toLong() and Sha512Constants.BYTE_MASK_LONG) shl 16) or
                    ((block[idx + 6].toLong() and Sha512Constants.BYTE_MASK_LONG) shl 8) or
                    (block[idx + 7].toLong() and Sha512Constants.BYTE_MASK_LONG)
            }

            // Expand 16-word message schedule to 80 rounds
            for (t in Sha512Constants.SCHEDULE_INIT_WORDS until Sha512Constants.ROUNDS) {
                w[t] = smallSigma1(w[t - 2]) + w[t - 7] + smallSigma0(w[t - 15]) + w[t - 16]
            }

            // Load working variables
            var a = s[0]
            var b = s[1]
            var c = s[2]
            var d = s[3]
            var e = s[4]
            var f = s[5]
            var g = s[6]
            var h = s[7]

            // Execute 80 mixing rounds
            for (t in 0 until Sha512Constants.ROUNDS) {
                val t1 = h + bigSigma1(e) + ch(e, f, g) + Sha512Constants.K[t] + w[t]
                val t2 = bigSigma0(a) + maj(a, b, c)
                h = g
                g = f
                f = e
                e = d + t1
                d = c
                c = b
                b = a
                a = t1 + t2
            }

            // Feed-forward accumulation into state
            s[0] += a
            s[1] += b
            s[2] += c
            s[3] += d
            s[4] += e
            s[5] += f
            s[6] += g
            s[7] += h
        }
    }
}
