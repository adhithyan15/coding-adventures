// ============================================================================
// BloomFilter.kt — Probabilistic set membership filter (Kotlin)
// ============================================================================
//
// A Bloom filter answers "Have I seen this element before?" with two possible
// answers:
//
//   "Definitely NO"  — zero false negatives.  If the filter says NO, the
//                      element was NEVER added.  Trust this completely.
//
//   "Probably YES"   — small, tunable probability of false positives.  The
//                      filter says YES, but occasionally the element was never
//                      added.  The false positive rate is controlled by the
//                      parameters at construction time.
//
// This asymmetry makes Bloom filters ideal as a fast pre-flight check before
// expensive operations (disk reads, network calls, cache lookups).
//
// ============================================================================
// How it works: bit array + multiple hash functions
// ============================================================================
//
//   The filter is a bit array of m bits, all initially 0.
//
//   To ADD an element:    compute k bit positions; set those k bits to 1.
//   To CHECK an element:  compute the same k positions; if ALL are 1, return
//                         "probably yes"; if ANY is 0, return "definitely no".
//
//   Example (m=16 bits, k=3 hash functions):
//
//   Empty:       0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0
//                0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15
//
//   Add "alice" (h1→3, h2→7, h3→11):
//                0  0  0  1  0  0  0  1  0  0  0  1  0  0  0  0
//
//   Add "bob"  (h1→1, h2→5, h3→11):
//                0  1  0  1  0  1  0  1  0  0  0  1  0  0  0  0
//                         (bit 11 is shared between alice and bob)
//
//   Check "carol" (h1→2, ...): bit 2 is 0 → "Definitely NO"
//   Check "dave"  (h1→1, h2→5, h3→11): all 1 → "Probably YES" — FALSE POSITIVE!
//
// ============================================================================
// Optimal parameters
// ============================================================================
//
//   Given expected item count n and desired false positive rate p:
//
//     m = ceil(-n × ln(p) / ln(2)²)   ← optimal number of bits
//     k = round((m / n) × ln(2))      ← optimal number of hash functions
//
// ============================================================================
// Double hashing: k positions from two hash functions
// ============================================================================
//
//   g_i(x) = (h1(x) + i × h2(x)) mod m   for i = 0, 1, ..., k-1
//
//   h1 = FNV-1a 32-bit with fmix32 finalizer
//   h2 = DJB2 64-bit, folded to 32 bits, with fmix32 finalizer
//
//   fmix32 (MurmurHash3, public domain) breaks prefix correlation between
//   h1 and h2 so probe positions are well-distributed.
//

package com.codingadventures.bloomfilter

import kotlin.math.ceil
import kotlin.math.ln
import kotlin.math.max
import kotlin.math.pow
import kotlin.math.roundToInt

/**
 * Space-efficient probabilistic set membership filter.
 *
 * Never has false negatives: if [contains] returns `false`, the element is
 * guaranteed not to be in the set.
 *
 * May have false positives: if [contains] returns `true`, the element is
 * probably in the set — but occasionally was never added. The false positive
 * rate is controlled by [expectedItems] and [falsePositiveRate].
 *
 * ```kotlin
 * val bf = BloomFilter<String>(1000, 0.01)
 * bf.add("hello")
 * bf.contains("hello")  // true  — definitely was added
 * bf.contains("world")  // false — definitely not added
 * ```
 *
 * @param T the type of elements. [Any.toString] is used as the key for hashing.
 */
class BloomFilter<T> private constructor(
    val bitCount: Int,
    val hashCount: Int,
    private val nExpected: Int,
) {

    // =========================================================================
    // Companion object — construction and static utilities
    // =========================================================================

    companion object {

        // ─── FNV-1a 32-bit constants ──────────────────────────────────────────
        private const val FNV32_OFFSET_BASIS: Int = 0x811C9DC5.toInt()
        private const val FNV32_PRIME:        Int = 0x01000193

        private const val MASK64: Long = -1L              // 0xFFFFFFFFFFFFFFFFL
        private const val MASK32: Long = 0xFFFFFFFFL

        // ─── FNV-1a 32-bit ────────────────────────────────────────────────────
        //
        // Processes one byte at a time:
        //   1. XOR the current hash with the byte.
        //   2. Multiply by the FNV prime (avalanches the change across 32 bits).

        internal fun fnv1a32(data: ByteArray): Int {
            var h = FNV32_OFFSET_BASIS
            for (b in data) {
                h = h xor (b.toInt() and 0xFF)
                h *= FNV32_PRIME
            }
            return h
        }

        // ─── DJB2 (Dan Bernstein), folded to 32 bits ─────────────────────────
        //
        // hash = 5381; for each byte: hash = hash * 33 + byte
        // Written as ((hash << 5) + hash + byte) to use shift-add instead of multiply.
        // Folded: (h XOR (h ushr 32)) and 0xFFFFFFFF

        internal fun djb2_32(data: ByteArray): Int {
            var h = 5381L
            for (b in data) {
                h = ((h shl 5) + h + (b.toLong() and 0xFFL)) and MASK64
            }
            return ((h xor (h ushr 32)) and MASK32).toInt()
        }

        // ─── MurmurHash3 fmix32 finalizer ─────────────────────────────────────
        //
        // Bijective bit mixer: every output bit depends on every input bit.
        // Applied to both h1 and h2 to break prefix correlation between FNV1a
        // and DJB2 for strings that share a common prefix.
        // Constants from Austin Appleby's MurmurHash3 (public domain).

        internal fun fmix32(h: Int): Int {
            var x = h
            x = x xor (x ushr 16)
            x *= 0x85EBCA6B.toInt()
            x = x xor (x ushr 13)
            x *= 0xC2B2AE35.toInt()
            x = x xor (x ushr 16)
            return x
        }

        // ─── Constructor: auto-sized ──────────────────────────────────────────

        /**
         * Create a Bloom filter auto-sized for the given parameters.
         *
         * Computes optimal bit count m and hash count k using:
         * ```
         * m = ceil(-n × ln(p) / ln(2)²)
         * k = max(1, round((m / n) × ln(2)))
         * ```
         *
         * @param expectedItems how many distinct elements you plan to add (n)
         * @param falsePositiveRate target false positive rate, e.g. 0.01 = 1%
         */
        operator fun <T> invoke(expectedItems: Int, falsePositiveRate: Double): BloomFilter<T> {
            require(expectedItems > 0) {
                "expectedItems must be positive, got: $expectedItems"
            }
            require(falsePositiveRate > 0.0 && falsePositiveRate < 1.0) {
                "falsePositiveRate must be in the open interval (0, 1), got: $falsePositiveRate"
            }

            val n = expectedItems.toDouble()
            val p = falsePositiveRate
            val ln2 = ln(2.0)

            val mLong = ceil(-n * ln(p) / (ln2 * ln2)).toLong()
            require(mLong <= Int.MAX_VALUE) {
                "Required bit array ($mLong bits) exceeds Int.MAX_VALUE. " +
                "Reduce expectedItems or increase falsePositiveRate."
            }

            val m = mLong.toInt()
            val k = max(1, ((m / n) * ln2).roundToInt())

            return BloomFilter<T>(m, k, expectedItems)
        }

        /**
         * Create a filter with explicit bit count and hash count, bypassing
         * the auto-sizing formula.
         *
         * @param bitCount  total number of bits m
         * @param hashCount number of hash functions k
         */
        fun <T> explicit(bitCount: Int, hashCount: Int): BloomFilter<T> {
            require(bitCount > 0)  { "bitCount must be positive, got: $bitCount" }
            require(hashCount > 0) { "hashCount must be positive, got: $hashCount" }
            return BloomFilter<T>(bitCount, hashCount, nExpected = 0)
        }

        // ─── Static utilities ─────────────────────────────────────────────────

        /**
         * Optimal bit array size for [n] elements and false positive rate [p].
         *
         * Formula: `m = ceil(-n × ln(p) / ln(2)²)`
         */
        fun optimalM(n: Long, p: Double): Long {
            require(n > 0) { "n must be positive" }
            require(p > 0.0 && p < 1.0) { "p must be in (0, 1)" }
            val ln2 = ln(2.0)
            return ceil(-n * ln(p) / (ln2 * ln2)).toLong()
        }

        /**
         * Optimal number of hash functions for [m] bits and [n] elements.
         *
         * Formula: `k = max(1, round((m / n) × ln(2)))`
         */
        fun optimalK(m: Long, n: Long): Int {
            require(n > 0) { "n must be positive" }
            return max(1, ((m.toDouble() / n.toDouble()) * ln(2.0)).roundToInt())
        }

        /**
         * How many elements can be stored in [memoryBytes] at rate [p]?
         *
         * Inverse of [optimalM]: `n = -m × ln(2)² / ln(p)`
         */
        fun capacityForMemory(memoryBytes: Long, p: Double): Long {
            require(p > 0.0 && p < 1.0) { "p must be in (0, 1)" }
            val m = memoryBytes * 8L
            val ln2 = ln(2.0)
            return (-m * (ln2 * ln2) / ln(p)).toLong()
        }
    }

    // =========================================================================
    // Mutable state
    // =========================================================================

    private val bits: ByteArray = ByteArray((bitCount + 7) / 8)
    private var _bitsSet: Int = 0
    private var _size: Int = 0

    // =========================================================================
    // Internal hash computation
    // =========================================================================

    /**
     * Compute k bit indices for [element] using double hashing.
     *
     * ```
     * g_i(x) = (h1(x) + i × h2(x)) mod m
     * ```
     *
     * h2 is forced odd to maximise bit position spread when m is odd-sized.
     */
    private fun hashIndices(element: T): IntArray {
        val raw = element.toString().toByteArray(Charsets.UTF_8)

        val h1 = fmix32(fnv1a32(raw)).toLong() and MASK32
        val h2 = (fmix32(djb2_32(raw)).toLong() and MASK32) or 1L  // force odd

        val m = bitCount.toLong()
        return IntArray(hashCount) { i ->
            ((h1 + i.toLong() * h2) % m).toInt()
        }
    }

    // =========================================================================
    // Core operations
    // =========================================================================

    /**
     * Add [element] to the filter.
     *
     * Sets up to k bits. Bits already set are not double-counted.
     *
     * O(k) time.
     */
    fun add(element: T) {
        for (idx in hashIndices(element)) {
            val byteIdx = idx ushr 3       // idx / 8
            val bitMask = 1 shl (idx and 7) // bit at position idx % 8
            if ((bits[byteIdx].toInt() and bitMask) == 0) {
                bits[byteIdx] = (bits[byteIdx].toInt() or bitMask).toByte()
                _bitsSet++
            }
        }
        _size++
    }

    /**
     * Check if [element] might be in the filter.
     *
     * - Returns `false` → element is **definitely not** in the filter (zero false negatives).
     * - Returns `true`  → element is **probably** in the filter (bounded false positive rate).
     *
     * O(k) time.
     */
    fun contains(element: T): Boolean {
        for (idx in hashIndices(element)) {
            val byteIdx = idx ushr 3
            val bitMask = 1 shl (idx and 7)
            if ((bits[byteIdx].toInt() and bitMask) == 0) return false
        }
        return true
    }

    // =========================================================================
    // Properties and statistics
    // =========================================================================

    /** Number of bits currently set to 1. */
    val bitsSet: Int get() = _bitsSet

    /** Number of elements added via [add]. */
    val size: Int get() = _size

    /**
     * Fraction of bits currently set to 1: `bitsSet / bitCount`.
     *
     * At fill ratio ≈ 0.5 the filter is at its optimal operating point.
     */
    val fillRatio: Double get() = _bitsSet.toDouble() / bitCount.toDouble()

    /**
     * Estimated current false positive rate based on fill ratio.
     *
     * Formula: `fillRatio^k`. Returns 0.0 for an empty filter.
     */
    val estimatedFalsePositiveRate: Double
        get() = if (_bitsSet == 0) 0.0 else fillRatio.pow(hashCount.toDouble())

    /**
     * True if more elements than the original capacity have been added.
     *
     * Always `false` for filters created with [BloomFilter.explicit] (no capacity target).
     */
    val isOverCapacity: Boolean
        get() = nExpected > 0 && _size > nExpected

    /**
     * Memory usage of the bit array in bytes: `ceil(m / 8)`.
     */
    val sizeBytes: Int get() = bits.size

    // =========================================================================
    // toString
    // =========================================================================

    override fun toString(): String {
        val pctSet = fillRatio * 100.0
        val estFp  = estimatedFalsePositiveRate * 100.0
        return "BloomFilter(m=$bitCount, k=$hashCount, " +
               "bitsSet=$_bitsSet/$bitCount (%.2f%%), ~fp=%.4f%%)".format(pctSet, estFp)
    }
}
