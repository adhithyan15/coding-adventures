// ============================================================================
// UUID.kt — Universally Unique Identifiers (RFC 4122 + RFC 9562)
// ============================================================================
//
// A UUID is a 128-bit label formatted as 32 lowercase hex digits separated by
// hyphens into five groups: 8-4-4-4-12:
//
//   550e8400-e29b-41d4-a716-446655440000
//   ^^^^^^^^ ^^^^ ^    ^^^^  ^^^^^^^^^^^^
//   time-low  mid  ver  clk   node (48 bits)
//
// The version nibble (position 13 in the string, letter M) identifies the
// generation algorithm. The variant field (first nibble of the 4th group)
// is always 8, 9, a, or b for standard RFC 4122 UUIDs (high 2 bits = 10xx).
//
// UUID versions implemented here:
// --------------------------------
// v1 — Time-based: encodes current time as 100-ns intervals since 1582-10-15.
// v3 — Name-based (MD5): deterministic hash of namespace UUID + name string.
// v4 — Random: 122 bits from SecureRandom; most commonly used.
// v5 — Name-based (SHA-1): deterministic hash; prefer over v3 for new code.
// v7 — Time-ordered random (RFC 9562): millisecond timestamp in high bits.
//
// Internal representation:
// ------------------------
// Stored as two Long fields: msb (bytes 0–7) and lsb (bytes 8–15), big-endian.
// This mirrors the JDK's java.util.UUID and is the most efficient JVM layout.
//

package com.codingadventures.uuid

import java.security.MessageDigest
import java.security.SecureRandom
import java.util.regex.Pattern

/**
 * A 128-bit Universally Unique Identifier (UUID) per RFC 4122 and RFC 9562.
 *
 * Stored as two [Long] fields: [msb] (most significant bits, bytes 0–7) and
 * [lsb] (least significant bits, bytes 8–15).
 *
 * ```kotlin
 * val u = UUID.v4()
 * println(u)           // e.g. "550e8400-e29b-41d4-..."
 * println(u.version)   // 4
 * println(u.variant)   // "rfc4122"
 *
 * // Name-based (deterministic)
 * val dns = UUID.v5(UUID.NAMESPACE_DNS, "python.org")
 * assertEquals("886313e1-3b8a-5372-9b90-0c9aee199e5d", dns.toString())
 * ```
 */
data class UUID(
    /** Bytes 0–7 (time-low, time-mid, version nibble + time-hi in v1; random in v4). */
    val msb: Long,
    /** Bytes 8–15 (variant + clock-seq + node). */
    val lsb: Long,
) : Comparable<UUID> {

    // =========================================================================
    // Properties
    // =========================================================================

    /**
     * The version field (bits 48–51 of the UUID, high nibble of byte 6).
     * Returns 0 for NIL and MAX which have no meaningful version.
     */
    val version: Int get() = ((msb shr 12) and 0xF).toInt()

    /**
     * The variant field as a human-readable string.
     *
     * - `"rfc4122"` — standard (10xx)
     * - `"microsoft"` — legacy GUID (110x)
     * - `"ncs"` — NCS backward-compatible (0xxx)
     * - `"reserved"` — future use (1111)
     */
    val variant: String get() {
        val top = ((lsb ushr 62) and 0x3).toInt()
        return when {
            top <= 1 -> "ncs"
            top == 2 -> "rfc4122"
            else -> if (((lsb ushr 61) and 0x7).toInt() == 7) "reserved" else "microsoft"
        }
    }

    /** True if this is the nil UUID (all zeros). */
    val isNil: Boolean get() = msb == 0L && lsb == 0L

    /** True if this is the max UUID (all ones). */
    val isMax: Boolean get() = msb == -1L && lsb == -1L

    /**
     * Return the 16 bytes of this UUID in network (big-endian) byte order.
     * bytes[0] is the most significant byte; bytes[15] is the least.
     */
    fun toBytes(): ByteArray {
        val b = ByteArray(16)
        longToBytes(msb, b, 0)
        longToBytes(lsb, b, 8)
        return b
    }

    // =========================================================================
    // String representation
    // =========================================================================

    /**
     * Return the standard 8-4-4-4-12 lowercase hyphenated UUID string.
     * Example: `"550e8400-e29b-41d4-a716-446655440000"`
     */
    override fun toString(): String {
        val hex = "%016x%016x".format(msb, lsb)
        return "${hex.substring(0, 8)}-${hex.substring(8, 12)}-" +
               "${hex.substring(12, 16)}-${hex.substring(16, 20)}-${hex.substring(20)}"
    }

    // =========================================================================
    // compareTo
    // =========================================================================

    /**
     * Lexicographic order by bytes (unsigned comparison of msb then lsb).
     * Corresponds to temporal order for v7 UUIDs.
     */
    override fun compareTo(other: UUID): Int {
        val c = java.lang.Long.compareUnsigned(msb, other.msb)
        return if (c != 0) c else java.lang.Long.compareUnsigned(lsb, other.lsb)
    }

    // =========================================================================
    // Companion object
    // =========================================================================

    companion object {

        // UUID_PATTERN must be initialized BEFORE any UUID constants that call
        // fromString(), because Kotlin companion objects initialize in source order.
        private val UUID_PATTERN: Pattern = Pattern.compile(
            "^\\s*(?:urn:uuid:)?\\{?" +
            "([0-9a-fA-F]{8})-?([0-9a-fA-F]{4})-?([0-9a-fA-F]{4})-?([0-9a-fA-F]{4})-?([0-9a-fA-F]{12})" +
            "\\}?\\s*$",
            Pattern.CASE_INSENSITIVE,
        )

        /** The nil UUID: all 128 bits are zero. */
        val NIL = UUID(0L, 0L)

        /** The max UUID: all 128 bits are one. */
        val MAX = UUID(-1L, -1L)

        /** RFC 4122 namespace for fully-qualified domain names. */
        val NAMESPACE_DNS  = fromString("6ba7b810-9dad-11d1-80b4-00c04fd430c8")
        /** RFC 4122 namespace for URLs. */
        val NAMESPACE_URL  = fromString("6ba7b811-9dad-11d1-80b4-00c04fd430c8")
        /** RFC 4122 namespace for ISO OIDs. */
        val NAMESPACE_OID  = fromString("6ba7b812-9dad-11d1-80b4-00c04fd430c8")
        /** RFC 4122 namespace for X.500 distinguished names. */
        val NAMESPACE_X500 = fromString("6ba7b814-9dad-11d1-80b4-00c04fd430c8")

        private val SECURE_RANDOM = SecureRandom()

        // =====================================================================
        // Factory: parsing
        // =====================================================================

        /**
         * Parse a UUID from its string representation.
         *
         * Accepts standard (hyphenated), compact (no hyphens), braced, and URN
         * forms. Case-insensitive.
         *
         * @throws UUIDException if the string is not a valid UUID
         */
        fun fromString(text: String?): UUID {
            if (text == null) throw UUIDException("UUID string must not be null")
            val m = UUID_PATTERN.matcher(text.trim())
            if (!m.matches()) throw UUIDException("Invalid UUID string: '$text'")
            val hex = m.group(1) + m.group(2) + m.group(3) + m.group(4) + m.group(5)
            val msbVal = java.lang.Long.parseUnsignedLong(hex.substring(0, 16), 16)
            val lsbVal = java.lang.Long.parseUnsignedLong(hex.substring(16, 32), 16)
            return UUID(msbVal, lsbVal)
        }

        /**
         * Construct a UUID from exactly 16 bytes in network (big-endian) byte order.
         *
         * @throws UUIDException if the array is not exactly 16 bytes
         */
        fun fromBytes(bytes: ByteArray?): UUID {
            if (bytes == null || bytes.size != 16) {
                throw UUIDException(
                    "UUID bytes must be exactly 16, got ${bytes?.size ?: "null"}")
            }
            return UUID(bytesToLong(bytes, 0), bytesToLong(bytes, 8))
        }

        /** Return true if [text] is a valid UUID string in any supported format. */
        fun isValid(text: String?): Boolean {
            if (text == null) return false
            return UUID_PATTERN.matcher(text.trim()).matches()
        }

        // =====================================================================
        // Factory: UUID v4 — Random
        // =====================================================================

        /**
         * Generate a UUID v4 (random).
         *
         * Uses [SecureRandom] (backed by the OS CSPRNG) for 122 bits of entropy.
         * The remaining 6 bits encode the version (4) and variant (10xx).
         */
        fun v4(): UUID {
            val b = ByteArray(16)
            SECURE_RANDOM.nextBytes(b)
            return stampVersionVariant(b, 4)
        }

        // =====================================================================
        // Factory: UUID v7 — Time-Ordered Random (RFC 9562)
        // =====================================================================

        /**
         * Generate a UUID v7 (time-ordered random, RFC 9562).
         *
         * The first 48 bits are the current Unix timestamp in milliseconds,
         * ensuring lexicographic sort order roughly matches creation time.
         * Ideal for database primary keys.
         */
        fun v7(): UUID {
            val tsMs = System.currentTimeMillis()
            val rand = ByteArray(10)
            SECURE_RANDOM.nextBytes(rand)
            val raw = ByteArray(16)
            raw[0] = ((tsMs shr 40) and 0xFF).toByte()
            raw[1] = ((tsMs shr 32) and 0xFF).toByte()
            raw[2] = ((tsMs shr 24) and 0xFF).toByte()
            raw[3] = ((tsMs shr 16) and 0xFF).toByte()
            raw[4] = ((tsMs shr  8) and 0xFF).toByte()
            raw[5] = ( tsMs         and 0xFF).toByte()
            raw[6] = (0x70 or (rand[0].toInt() and 0x0F)).toByte()
            raw[7] = rand[1]
            raw[8] = (0x80 or (rand[2].toInt() and 0x3F)).toByte()
            rand.copyInto(raw, destinationOffset = 9, startIndex = 3, endIndex = 10)
            return fromBytes(raw)
        }

        // =====================================================================
        // Factory: UUID v1 — Time-Based
        // =====================================================================

        // Number of 100-ns intervals between 1582-10-15 and 1970-01-01
        private const val GREGORIAN_OFFSET = 122_192_928_000_000_000L

        private val CLOCK_SEQ: Int = run {
            val csb = ByteArray(2)
            SECURE_RANDOM.nextBytes(csb)
            ((csb[0].toInt() and 0xFF) shl 8 or (csb[1].toInt() and 0xFF)) and 0x3FFF
        }

        /**
         * Generate a UUID v1 (time-based).
         *
         * Encodes the current UTC time as 100-ns intervals since 1582-10-15.
         * Uses a random 48-bit node ID (multicast bit set per RFC 4122 §4.5).
         */
        fun v1(): UUID {
            val timestamp = System.nanoTime() / 100 + GREGORIAN_OFFSET
            val timeLow  = timestamp and 0xFFFFFFFFL
            val timeMid  = (timestamp shr 32) and 0xFFFFL
            val timeHi   = (timestamp shr 48) and 0x0FFFL
            val timeHiAndVersion = (1L shl 12) or timeHi
            val msbVal = (timeLow shl 32) or (timeMid shl 16) or timeHiAndVersion

            val clockSeqHi = 0x80L or (CLOCK_SEQ shr 8).toLong()
            val clockSeqLow = (CLOCK_SEQ and 0xFF).toLong()
            val nodeBytes = ByteArray(6)
            SECURE_RANDOM.nextBytes(nodeBytes)
            nodeBytes[0] = (nodeBytes[0].toInt() or 0x01).toByte()
            var node = 0L
            for (nb in nodeBytes) node = (node shl 8) or (nb.toLong() and 0xFF)
            val lsbVal = (clockSeqHi shl 56) or (clockSeqLow shl 48) or node
            return UUID(msbVal, lsbVal)
        }

        // =====================================================================
        // Factory: UUID v5 — Name-Based (SHA-1)
        // =====================================================================

        /**
         * Generate a UUID v5 (name-based, SHA-1).
         *
         * Deterministic: same [namespace] and [name] always produce the same UUID.
         *
         * RFC test vector: `v5(NAMESPACE_DNS, "python.org")` →
         * `"886313e1-3b8a-5372-9b90-0c9aee199e5d"`
         */
        fun v5(namespace: UUID, name: String): UUID {
            val data = namespace.toBytes() + name.toByteArray(Charsets.UTF_8)
            val digest = digest(data, "SHA-1")
            val raw = digest.copyOf(16)
            return stampVersionVariant(raw, 5)
        }

        // =====================================================================
        // Factory: UUID v3 — Name-Based (MD5)
        // =====================================================================

        /**
         * Generate a UUID v3 (name-based, MD5).
         *
         * Deterministic. Prefer [v5] for new code.
         *
         * RFC test vector: `v3(NAMESPACE_DNS, "python.org")` →
         * `"6fa459ea-ee8a-3ca4-894e-db77e160355e"`
         */
        fun v3(namespace: UUID, name: String): UUID {
            val data = namespace.toBytes() + name.toByteArray(Charsets.UTF_8)
            val digest = digest(data, "MD5")
            return stampVersionVariant(digest, 3)
        }

        // =====================================================================
        // Internal helpers
        // =====================================================================

        private fun stampVersionVariant(b: ByteArray, version: Int): UUID {
            b[6] = ((b[6].toInt() and 0x0F) or (version shl 4)).toByte()
            b[8] = ((b[8].toInt() and 0x3F) or 0x80).toByte()
            return fromBytes(b)
        }

        private fun digest(data: ByteArray, algorithm: String): ByteArray =
            try {
                MessageDigest.getInstance(algorithm).digest(data)
            } catch (e: java.security.NoSuchAlgorithmException) {
                throw UUIDException("Hash algorithm not available: $algorithm")
            }

        private fun bytesToLong(b: ByteArray, offset: Int): Long {
            var v = 0L
            for (i in 0 until 8) v = (v shl 8) or (b[offset + i].toLong() and 0xFF)
            return v
        }

        private fun longToBytes(v: Long, b: ByteArray, offset: Int) {
            var value = v
            for (i in 7 downTo 0) {
                b[offset + i] = (value and 0xFF).toByte()
                value = value ushr 8
            }
        }
    }
}
