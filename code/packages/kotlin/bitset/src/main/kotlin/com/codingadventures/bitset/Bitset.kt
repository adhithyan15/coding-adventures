// ============================================================================
// Bitset.kt — Compact Boolean Array Packed into 64-bit Words
// ============================================================================
//
// What is a Bitset?
// -----------------
// A bitset stores a sequence of bits (each either 0 or 1) packed into
// machine-word-sized integers (Long, which is 64 bits in Kotlin/JVM). Instead
// of using an entire byte to represent a single true/false value, a bitset
// packs 64 of them into a single word.
//
// Why does this matter?
//
//   1. Space: 10,000 booleans as BooleanArray ≈ 10,000 bytes. As a bitset
//      ≈ 1,250 bytes — 8× more compact.
//
//   2. Speed: AND-ing two BooleanArray of 10,000 elements requires 10,000
//      iterations. AND-ing two Bitsets requires ~157 iterations, with a single
//      64-bit AND instruction per word.
//
//   3. Ubiquity: Bloom filters, register allocators, graph visited-sets,
//      database bitmap indexes, filesystem free-block maps, subnet masks.
//
// Bit Ordering: LSB-First
// -----------------------
// We use Least Significant Bit first ordering:
//
//   Word 0                              Word 1
//   ┌─────────────────────────────┐     ┌─────────────────────────────┐
//   │ bit 63  ...  bit 2  bit 1  bit 0│ │ bit 127 ... bit 65  bit 64 │
//   └─────────────────────────────┘     └─────────────────────────────┘
//   MSB ◄─────────────────── LSB        MSB ◄─────────────────── LSB
//
// The three fundamental formulas:
//   word_index = i / 64          (which word contains bit i?)
//   bit_offset = i % 64          (which position within that word?)
//   bitmask    = 1L shl (i % 64) (a mask with only bit i set)
//
// Kotlin Note on Long
// -------------------
// Kotlin's Long is signed 64-bit (JVM Long). Bitwise operations work in
// two's-complement, identical to unsigned 64-bit arithmetic for our purposes:
//
//   • "All bits set" is -1L (0xFFFFFFFFFFFFFFFFL).
//   • Popcount: Long.countOneBits() — Kotlin 1.5+ extension.
//   • Trailing zeros: Long.countTrailingZeroBits().
//   • Unsigned right shift: Long.ushr(n) (Kotlin's >>>).
//
// Spec: code/specs/bitset.md
// ============================================================================

package com.codingadventures.bitset

/**
 * A compact data structure that packs boolean values into 64-bit words.
 *
 * Provides O(n/64) bulk bitwise operations (AND, OR, XOR, NOT), efficient
 * iteration over set bits using trailing-zero-count, and ArrayList-style
 * automatic growth when bits are set beyond the current size.
 *
 * ## Internal Representation
 *
 * Bits are stored in `_words: LongArray`. Each `Long` holds 64 bits. [_size]
 * is the logical size — the number of bits the user considers "addressable".
 * Bits beyond [_size] in the last word are **always zero** (the
 * clean-trailing-bits invariant).
 *
 * ```
 * ┌──────────────────────────────────────────────────────────────────┐
 * │                   capacity (256 bits = 4 words)                  │
 * │                                                                  │
 * │  ┌──────────────────────────────────────────┐                    │
 * │  │              length (200 bits)            │ ··· unused ····   │
 * │  └──────────────────────────────────────────┘  (always zero)    │
 * └──────────────────────────────────────────────────────────────────┘
 * ```
 */
class Bitset private constructor(
    // We use underscore-prefixed names for internal fields to avoid
    // name clashes with the public API functions (e.g., length() and words).
    private var _words: LongArray,
    private var _size: Int
) {

    // =========================================================================
    // Public constructor
    // =========================================================================

    /**
     * Creates a new bitset with all bits initially zero.
     *
     * The [size] parameter sets the logical length. The capacity is rounded
     * up to the next multiple of 64.
     *
     * ```kotlin
     * val bs = Bitset(100)
     * // bs.length() == 100
     * // bs.capacity() == 128  (2 words × 64 bits/word)
     * // bs.popcount() == 0
     * ```
     *
     * `Bitset(0)` is valid and creates an empty bitset.
     *
     * @param size the number of addressable bits
     */
    constructor(size: Int) : this(LongArray(wordsNeeded(size)), size)

    // =========================================================================
    // Validation
    // =========================================================================

    init {
        // Validate _size here (runs for every constructor path, including the
        // public `constructor(size: Int)` and the private primary constructor
        // used internally by fromInteger / fromBinaryStr).
        require(_size >= 0) {
            "size must be non-negative, got: $_size"
        }
        require(_size <= MAX_BITS) {
            "size $_size exceeds maximum allowed bits ($MAX_BITS)"
        }
    }

    companion object {
        // =====================================================================
        // Constants
        // =====================================================================

        /**
         * Number of bits per word. We use 64-bit Longs.
         *
         * Every formula in this class uses this constant rather than a magic
         * number.
         */
        const val BITS_PER_WORD = 64

        /**
         * Maximum allowed bitset size, in bits.
         *
         * 67,108,864 bits = 8 MB of word storage. This cap prevents two
         * denial-of-service vectors:
         *
         * - **Direct over-allocation**: `Bitset(Int.MAX_VALUE)` would try to
         *   allocate ~256 MB of `LongArray`.
         * - **Integer overflow in ensureCapacity**: the doubling loop
         *   `while (newCap <= i) newCap *= 2` wraps to a negative value when
         *   `i` is near `Int.MAX_VALUE`, causing an `ArrayIndexOutOfBoundsException`
         *   or silent corruption.
         *
         * Adjust if your application genuinely needs larger bitsets with
         * untrusted size inputs.
         */
        const val MAX_BITS = 1 shl 26 // 67,108,864 bits ≈ 8 MB

        // =====================================================================
        // Factory methods
        // =====================================================================

        /**
         * Creates a bitset from a non-negative integer (treated as unsigned
         * 64-bit Long).
         *
         * Bit 0 of the bitset is the least significant bit of [value]. The
         * length of the result is the position of the highest set bit + 1.
         * If [value] == 0, produces an empty bitset.
         *
         * ```kotlin
         * val bs = Bitset.fromInteger(5L)  // binary: 101
         * // bs.length() == 3
         * // bs.test(0) == true    (bit 0 = 1)
         * // bs.test(1) == false   (bit 1 = 0)
         * // bs.test(2) == true    (bit 2 = 1)
         * ```
         *
         * @param value the unsigned 64-bit value
         * @return a new bitset
         */
        fun fromInteger(value: Long): Bitset {
            if (value == 0L) return Bitset(0)

            // 64 - value.countLeadingZeroBits() gives the number of bits
            // needed to represent value (i.e., position of highest set bit+1).
            // Works for "negative" signed longs because countLeadingZeroBits
            // counts literal leading zero bits.
            val sz = 64 - value.countLeadingZeroBits()
            return Bitset(longArrayOf(value), sz)
        }

        /**
         * Creates a bitset from a string of `'0'` and `'1'` characters.
         *
         * The leftmost character is the highest-indexed bit (conventional
         * binary notation). The rightmost character is bit 0.
         *
         * ```
         * Input: "1010"
         * Bit 3 = 1 (leftmost), bit 2 = 0, bit 1 = 1, bit 0 = 0 (rightmost)
         * Same as integer 10.
         * ```
         *
         * An empty string produces an empty bitset.
         *
         * @param s a string of '0' and '1' characters
         * @return a new bitset
         * @throws IllegalArgumentException if any character is not '0' or '1'
         */
        fun fromBinaryStr(s: String): Bitset {
            for ((i, c) in s.withIndex()) {
                require(c == '0' || c == '1') {
                    "invalid character '$c' at index $i in binary string"
                }
            }

            if (s.isEmpty()) return Bitset(0)

            val sz = s.length
            val bs = Bitset(sz)

            // Walk from right to left: rightmost char is bit 0.
            for (i in 0 until sz) {
                val charIdx = sz - 1 - i // bit index i → string position
                if (s[charIdx] == '1') {
                    bs._words[wordIndex(i)] = bs._words[wordIndex(i)] or bitmask(i)
                }
            }

            bs.cleanTrailingBits()
            return bs
        }

        // =====================================================================
        // Shared static helpers
        // =====================================================================

        /**
         * Ceiling division: how many Long words to store [bitCount] bits.
         *
         * ```
         * wordsNeeded(0)   = 0
         * wordsNeeded(1)   = 1
         * wordsNeeded(64)  = 1
         * wordsNeeded(65)  = 2
         * wordsNeeded(200) = 4
         * ```
         *
         * Validates [bitCount] is in `0..MAX_BITS` to prevent denial-of-service
         * via unbounded allocation. This is the earliest point where we can
         * reject bad inputs from the public `constructor(size: Int)` delegating
         * constructor — that path calls `wordsNeeded(size)` before the class
         * `init` block gets a chance to run.
         */
        fun wordsNeeded(bitCount: Int): Int {
            require(bitCount >= 0) {
                "size must be non-negative, got: $bitCount"
            }
            require(bitCount <= MAX_BITS) {
                "size $bitCount exceeds maximum allowed bits ($MAX_BITS)"
            }
            return (bitCount + BITS_PER_WORD - 1) / BITS_PER_WORD
        }

        /**
         * Which word contains bit [i]? Simply `i / 64`.
         *
         * ```
         * wordIndex(0)   = 0   (bit 0 is in word 0)
         * wordIndex(63)  = 0   (bit 63 is the last bit of word 0)
         * wordIndex(64)  = 1   (bit 64 is the first bit of word 1)
         * ```
         */
        fun wordIndex(i: Int): Int = i / BITS_PER_WORD

        /**
         * Which bit position within its word? Simply `i % 64`.
         *
         * ```
         * bitOffset(0)  = 0
         * bitOffset(63) = 63
         * bitOffset(64) = 0   (first bit of the next word)
         * ```
         */
        fun bitOffset(i: Int): Int = i % BITS_PER_WORD

        /**
         * A Long mask with only bit [i] set within its word.
         *
         * This is `1L shl (i % 64)`. Used to isolate, set, clear, or toggle:
         * ```
         * set:    _words[w] = _words[w] or bitmask(i)
         * clear:  _words[w] = _words[w] and bitmask(i).inv()
         * test:   (_words[w] and bitmask(i)) != 0L
         * toggle: _words[w] = _words[w] xor bitmask(i)
         * ```
         */
        fun bitmask(i: Int): Long = 1L shl bitOffset(i)

        /**
         * Safely reads a word, returning 0L if the index is out of bounds.
         * Simplifies bulk operations between bitsets of different sizes.
         */
        fun wordAt(words: LongArray, i: Int): Long =
            if (i < words.size) words[i] else 0L
    }

    // =========================================================================
    // Single-bit operations
    // =========================================================================
    //
    // Growth semantics:
    //   • set(i) and toggle(i) AUTO-GROW the bitset if i >= _size.
    //   • test(i) and clear(i) do NOT grow. They return false / do nothing.

    /**
     * Sets bit [i] to 1. Auto-grows the bitset if `i >= length()`.
     *
     * OR is idempotent: setting an already-set bit is a no-op.
     *
     * @param i the bit index (0-based); must be non-negative
     */
    fun set(i: Int) {
        ensureCapacity(i)
        _words[wordIndex(i)] = _words[wordIndex(i)] or bitmask(i)
    }

    /**
     * Sets bit [i] to 0. No-op if `i >= length()` (does not grow).
     *
     * AND with the inverted mask clears exactly one bit while preserving all
     * others:
     * ```
     * _words[w] = 0b...0010_0100   (bits 2 and 5 set)
     * mask      = 0b...0010_0000   (bit 5)
     * ~mask     = 0b...1101_1111
     * result    = 0b...0000_0100   (bit 5 cleared)
     * ```
     *
     * @param i the bit index (0-based)
     */
    fun clear(i: Int) {
        if (i >= _size) return
        _words[wordIndex(i)] = _words[wordIndex(i)] and bitmask(i).inv()
    }

    /**
     * Returns whether bit [i] is set. Returns `false` if `i >= length()`.
     *
     * Unallocated bits are conceptually zero — this never grows the bitset.
     *
     * @param i the bit index (0-based)
     * @return `true` if bit i is set
     */
    fun test(i: Int): Boolean {
        if (i >= _size) return false
        return (_words[wordIndex(i)] and bitmask(i)) != 0L
    }

    /**
     * Flips bit [i] (0 → 1, 1 → 0). Auto-grows if `i >= length()`.
     *
     * XOR with the bitmask flips exactly one bit:
     * ```
     * _words[w] = 0b...0010_0100   (bits 2 and 5 set)
     * mask      = 0b...0010_0000   (bit 5)
     * result    = 0b...0000_0100   (bit 5 flipped to 0)
     * ```
     *
     * @param i the bit index (0-based); must be non-negative
     */
    fun toggle(i: Int) {
        ensureCapacity(i)
        _words[wordIndex(i)] = _words[wordIndex(i)] xor bitmask(i)
        cleanTrailingBits()
    }

    // =========================================================================
    // Bulk bitwise operations
    // =========================================================================
    //
    // All bulk operations return a NEW Bitset. Neither operand is modified.
    // Result length = max(a.length(), b.length()).
    // Missing words in the shorter bitset are treated as zero.

    /**
     * Returns a new bitset where each bit is 1 only if BOTH corresponding
     * input bits are 1 (intersection).
     *
     * ```
     * A  B  A and B
     * 0  0     0
     * 0  1     0
     * 1  0     0
     * 1  1     1
     * ```
     *
     * @param other the other bitset
     * @return the intersection
     */
    fun and(other: Bitset): Bitset {
        val resultLen = maxOf(this._size, other._size)
        val maxWords = maxOf(this._words.size, other._words.size)
        val resultWords = LongArray(maxWords) { i ->
            wordAt(this._words, i) and wordAt(other._words, i)
        }
        return Bitset(resultWords, resultLen).also { it.cleanTrailingBits() }
    }

    /**
     * Returns a new bitset where each bit is 1 if EITHER input bit is 1
     * (union).
     *
     * ```
     * A  B  A or B
     * 0  0    0
     * 0  1    1
     * 1  0    1
     * 1  1    1
     * ```
     *
     * @param other the other bitset
     * @return the union
     */
    fun or(other: Bitset): Bitset {
        val resultLen = maxOf(this._size, other._size)
        val maxWords = maxOf(this._words.size, other._words.size)
        val resultWords = LongArray(maxWords) { i ->
            wordAt(this._words, i) or wordAt(other._words, i)
        }
        return Bitset(resultWords, resultLen).also { it.cleanTrailingBits() }
    }

    /**
     * Returns a new bitset where each bit is 1 if the corresponding input bits
     * DIFFER (symmetric difference).
     *
     * ```
     * A  B  A xor B
     * 0  0     0
     * 0  1     1
     * 1  0     1
     * 1  1     0
     * ```
     *
     * @param other the other bitset
     * @return the symmetric difference
     */
    fun xor(other: Bitset): Bitset {
        val resultLen = maxOf(this._size, other._size)
        val maxWords = maxOf(this._words.size, other._words.size)
        val resultWords = LongArray(maxWords) { i ->
            wordAt(this._words, i) xor wordAt(other._words, i)
        }
        return Bitset(resultWords, resultLen).also { it.cleanTrailingBits() }
    }

    /**
     * Returns a new bitset with every bit flipped within `length()`.
     *
     * ```
     * A  not A
     * 0    1
     * 1    0
     * ```
     *
     * **Important**: bits beyond `length()` remain zero (clean-trailing-bits
     * invariant). The result has the same length as the input.
     *
     * @return the complement
     */
    fun not(): Bitset {
        val resultWords = LongArray(_words.size) { i -> _words[i].inv() }
        // Critical: NOT flipped ALL bits including the trailing bits beyond
        // _size. We must clean them to maintain the clean-trailing-bits
        // invariant:
        //   Before NOT: word[k] = 0b00000000_XXXXXXXX  (trailing bits zero)
        //   After  NOT: word[k] = 0b11111111_xxxxxxxx  (trailing bits now 1!)
        //   After clean: zeros the trailing region again
        return Bitset(resultWords, this._size).also { it.cleanTrailingBits() }
    }

    /**
     * Returns a new bitset with bits in `this` that are NOT in [other]
     * (set difference: `this and other.not()`).
     *
     * ```
     * A  B  A andNot B
     * 0  0      0
     * 0  1      0
     * 1  0      1
     * 1  1      0
     * ```
     *
     * @param other the other bitset
     * @return the set difference
     */
    fun andNot(other: Bitset): Bitset {
        val resultLen = maxOf(this._size, other._size)
        val maxWords = maxOf(this._words.size, other._words.size)
        val resultWords = LongArray(maxWords) { i ->
            wordAt(this._words, i) and wordAt(other._words, i).inv()
        }
        return Bitset(resultWords, resultLen).also { it.cleanTrailingBits() }
    }

    // =========================================================================
    // Counting and query operations
    // =========================================================================

    /**
     * Returns the number of set (1) bits.
     *
     * Uses [Long.countOneBits] (Kotlin 1.5+) which compiles to the hardware
     * POPCNT instruction on modern JVMs. O(N/64) time.
     *
     * @return the population count
     */
    fun popcount(): Int = _words.sumOf { it.countOneBits() }

    /**
     * Returns the logical length: the number of addressable bits.
     *
     * @return the logical length
     */
    fun length(): Int = _size

    /**
     * Returns the allocated size in bits (always a multiple of 64).
     *
     * @return the capacity in bits
     */
    fun capacity(): Int = _words.size * BITS_PER_WORD

    /**
     * Returns `true` if at least one bit is set.
     *
     * Short-circuits as soon as a non-zero word is found.
     *
     * @return `true` if any bit is set
     */
    fun any(): Boolean = _words.any { it != 0L }

    /**
     * Returns `true` if ALL bits in `0 until length()` are set.
     *
     * For an empty bitset (length=0), returns `true` (vacuous truth).
     *
     * @return `true` if all bits are set
     */
    fun all(): Boolean {
        if (_size == 0) return true // vacuous truth

        val numWords = _words.size

        // All full words must have every bit set (-1L = all 1s in two's
        // complement).
        for (i in 0 until numWords - 1) {
            if (_words[i] != -1L) return false
        }

        // Last word: check only bits within _size.
        val remaining = bitOffset(_size)
        return if (remaining == 0) {
            _words[numWords - 1] == -1L
        } else {
            val mask = (1L shl remaining) - 1L
            _words[numWords - 1] == mask
        }
    }

    /**
     * Returns `true` if no bits are set. Equivalent to `!any()`.
     *
     * @return `true` if no bit is set
     */
    fun none(): Boolean = !any()

    // =========================================================================
    // Iteration
    // =========================================================================

    /**
     * Returns the indices of all set bits in ascending order.
     *
     * Uses the trailing-zero-count trick:
     * ```
     * word = 0b10100100  (bits 2, 5, 7 set)
     *
     * Step 1: countTrailingZeroBits = 2 → record base + 2
     *         word = word and (word - 1)  → 0b10100000 (bit 2 cleared)
     *
     * Step 2: countTrailingZeroBits = 5 → record base + 5
     *         word = word and (word - 1)  → 0b10000000
     *
     * Step 3: countTrailingZeroBits = 7 → record base + 7
     *         word = word and (word - 1)  → 0b00000000
     * ```
     *
     * `word and (word - 1)` clears the lowest set bit:
     * ```
     * word     = 0b10100100
     * word - 1 = 0b10100011  (borrow propagates through trailing zeros)
     * AND      = 0b10100000  (lowest bit cleared)
     * ```
     *
     * O(k) where k is the number of set bits. Skips zero words entirely.
     *
     * @return list of bit indices of all set bits, in ascending order
     */
    fun iterSetBits(): List<Int> {
        val indices = mutableListOf<Int>()

        for (wordIdx in _words.indices) {
            var w = _words[wordIdx]
            val baseIndex = wordIdx * BITS_PER_WORD

            while (w != 0L) {
                val bitPos = w.countTrailingZeroBits()
                val index = baseIndex + bitPos

                if (index >= _size) break

                indices.add(index)

                // Clear the lowest set bit.
                w = w and (w - 1L)
            }
        }

        return indices
    }

    // =========================================================================
    // Conversion operations
    // =========================================================================

    /**
     * Converts the bitset to an unsigned 64-bit integer (returned as Long).
     *
     * Returns 0 for an empty bitset. Throws [ArithmeticException] if bits
     * beyond position 63 are set.
     *
     * @return the unsigned 64-bit value
     * @throws ArithmeticException if the bitset value exceeds 64 bits
     */
    fun toInteger(): Long {
        if (_words.isEmpty()) return 0L
        for (i in 1 until _words.size) {
            if (_words[i] != 0L) throw ArithmeticException(
                "bitset value exceeds 64-bit range"
            )
        }
        return _words[0]
    }

    /**
     * Converts the bitset to a binary string with the highest bit on the left.
     *
     * Inverse of [Bitset.fromBinaryStr]. Empty bitset returns `""`.
     *
     * ```kotlin
     * Bitset.fromInteger(5L).toBinaryStr() == "101"
     * ```
     *
     * @return a binary string (MSB left, LSB right)
     */
    fun toBinaryStr(): String {
        if (_size == 0) return ""
        return buildString(_size) {
            for (i in _size - 1 downTo 0) {
                append(if (test(i)) '1' else '0')
            }
        }
    }

    // =========================================================================
    // Equality and Any overrides
    // =========================================================================

    /**
     * Returns `true` if two bitsets have the same length and the same bits set.
     *
     * Capacity is irrelevant. Thanks to the clean-trailing-bits invariant,
     * we can compare words directly.
     *
     * @param other the object to compare with
     * @return `true` if equal
     */
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is Bitset) return false
        if (this._size != other._size) return false

        val maxWords = maxOf(this._words.size, other._words.size)
        for (i in 0 until maxWords) {
            if (wordAt(this._words, i) != wordAt(other._words, i)) return false
        }
        return true
    }

    /** @suppress */
    override fun hashCode(): Int {
        var result = _size
        result = 31 * result + _words.contentHashCode()
        return result
    }

    /**
     * Returns a human-readable representation like `"Bitset(101)"`.
     *
     * @return a string representation
     */
    override fun toString(): String = "Bitset(${toBinaryStr()})"

    // =========================================================================
    // Internal helpers
    // =========================================================================

    /**
     * Ensures the bitset has capacity for bit [i]. Grows by doubling if needed.
     *
     * After this call, `i < capacity()` and `_size >= i + 1`.
     *
     * Growth: double capacity until it exceeds [i], starting from max(capacity,
     * 64). This gives amortised O(1) growth.
     *
     * **Bounds guards**:
     * - Negative indices are rejected immediately.
     * - Indices at or above [MAX_BITS] are rejected to prevent the doubling
     *   loop from overflowing `Int` arithmetic (wrap-around to negative).
     */
    private fun ensureCapacity(i: Int) {
        require(i >= 0) { "bit index must be non-negative, got: $i" }
        require(i < MAX_BITS) {
            "bit index $i exceeds maximum allowed bits ($MAX_BITS)"
        }

        if (i < capacity()) {
            if (i >= _size) _size = i + 1
            return
        }

        // Double until we have room. The i < MAX_BITS guard above means
        // newCap can never overflow: MAX_BITS (1 shl 26) fits in Int, and
        // doubling from MAX_BITS/2 still fits (1 shl 27 < Int.MAX_VALUE).
        var newCap = capacity().coerceAtLeast(BITS_PER_WORD)
        while (newCap <= i) newCap *= 2

        val newWords = LongArray(newCap / BITS_PER_WORD)
        _words.copyInto(newWords)
        _words = newWords
        _size = i + 1
    }

    /**
     * Zeroes out any bits beyond [_size] in the last word.
     *
     * Maintains the clean-trailing-bits invariant. Must be called after any
     * operation that might set bits beyond [_size]: [not], [toggle], and bulk
     * ops with different-length operands.
     *
     * ```
     * _size = 200, capacity = 256
     * Last word holds bits 192–255, but only 192–199 are "real".
     * remaining = 200 % 64 = 8
     * mask = (1L shl 8) - 1 = 0xFF  (bits 0–7)
     * _words[3] = _words[3] and 0xFF  → zeroes bits 8–63 of word 3
     * ```
     *
     * If _size is a multiple of 64, no trailing bits need cleaning.
     */
    private fun cleanTrailingBits() {
        if (_size == 0 || _words.isEmpty()) return
        val remaining = bitOffset(_size)
        if (remaining != 0) {
            val lastIdx = _words.size - 1
            val mask = (1L shl remaining) - 1L
            _words[lastIdx] = _words[lastIdx] and mask
        }
    }
}
