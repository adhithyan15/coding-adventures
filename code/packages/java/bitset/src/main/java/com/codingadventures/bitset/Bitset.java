// ============================================================================
// Bitset.java — Compact Boolean Array Packed into 64-bit Words
// ============================================================================
//
// What is a Bitset?
// -----------------
// A bitset stores a sequence of bits (each either 0 or 1) packed into
// machine-word-sized integers (long, which is 64 bits in Java). Instead of
// using an entire byte to represent a single true/false value, a bitset packs
// 64 of them into a single word.
//
// Why does this matter?
//
//   1. Space: 10,000 booleans as boolean[] = 10,000 bytes (JVM typically uses
//      1 byte per boolean). As a bitset = ~1,250 bytes. That's an 8x improvement.
//
//   2. Speed: AND-ing two boolean arrays loops over 10,000 elements. AND-ing
//      two bitsets loops over ~157 words. The CPU performs a single 64-bit AND
//      instruction on each word, operating on 64 bits at once.
//
//   3. Ubiquity: Bitsets appear in Bloom filters, register allocators, graph
//      algorithms (visited sets), database bitmap indexes, filesystem free-block
//      bitmaps, network subnet masks, and garbage collectors.
//
// Bit Ordering: LSB-First
// -----------------------
// We use Least Significant Bit first ordering. Bit 0 is the least significant
// bit of word 0. Bit 63 is the most significant bit of word 0. Bit 64 is the
// least significant bit of word 1.
//
//   Word 0                              Word 1
//   ┌─────────────────────────────┐     ┌─────────────────────────────┐
//   │ bit 63  ...  bit 2  bit 1  bit 0│ │ bit 127 ... bit 65  bit 64 │
//   └─────────────────────────────┘     └─────────────────────────────┘
//   MSB ◄─────────────────── LSB        MSB ◄─────────────────── LSB
//
// The three fundamental formulas that drive every bitset operation:
//
//   word_index = i / 64         (which word contains bit i?)
//   bit_offset = i % 64         (which position within that word?)
//   bitmask    = 1L << (i % 64) (a mask with only bit i set)
//
// Java Note on Signed Long
// ------------------------
// Java's long is signed 64-bit. We treat the bits as unsigned storage —
// exactly as a C uint64_t would behave. Bitwise operations (&, |, ^, ~) and
// overflow arithmetic work identically in two's complement. Two important
// patterns to keep in mind:
//
//   • "All bits set" is written as -1L (0xFFFFFFFFFFFFFFFFL).
//   • Popcount uses Long.bitCount(w), which correctly counts bits regardless
//     of sign.
//   • Trailing-zero count uses Long.numberOfTrailingZeros(w).
//
// Spec: code/specs/rng.md (bitset section)
// ============================================================================

package com.codingadventures.bitset;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Objects;

/**
 * A compact data structure that packs boolean values into 64-bit words.
 *
 * <p>Provides O(n/64) bulk bitwise operations (AND, OR, XOR, NOT),
 * efficient iteration over set bits using trailing-zero-count, and
 * ArrayList-style automatic growth when you set bits beyond the current size.
 *
 * <h2>Internal Representation</h2>
 *
 * <p>Bits are stored in a {@code long[]} called {@code words}. Each
 * {@code long} holds 64 bits. We also track {@code length}, the logical
 * size — the number of bits the user considers "addressable".
 *
 * <pre>
 *   ┌──────────────────────────────────────────────────────────────────┐
 *   │                   capacity (256 bits = 4 words)                  │
 *   │                                                                  │
 *   │  ┌──────────────────────────────────────────┐                    │
 *   │  │              length (200 bits)            │ ··· unused ····   │
 *   │  │  (highest addressable bit index + 1)      │ (always zero)     │
 *   │  └──────────────────────────────────────────┘                    │
 *   └──────────────────────────────────────────────────────────────────┘
 * </pre>
 *
 * <p><b>Clean-trailing-bits invariant</b>: Bits beyond {@code length} in
 * the last word are always zero. This is critical for correctness of
 * {@link #popcount()}, {@link #any()}, {@link #all()}, {@link #none()},
 * {@link #equals(Object)}, and {@link #toInteger()}. Every operation
 * that modifies the last word must clean trailing bits afterwards.
 */
public final class Bitset {

    // =========================================================================
    // Constants
    // =========================================================================

    /**
     * Number of bits per word. We use 64-bit longs.
     *
     * <p>Every formula in this class uses this constant rather than a magic
     * number, so if someone ever wanted to experiment with int words (32 bits),
     * they'd only need to change this constant and the word type.
     */
    private static final int BITS_PER_WORD = 64;

    /**
     * Maximum allowed bitset size, in bits.
     *
     * <p>67,108,864 bits = 8 MB of word storage. This cap prevents two
     * denial-of-service vectors:
     * <ul>
     *   <li><b>Direct over-allocation</b>: {@code new Bitset(Integer.MAX_VALUE)}
     *       would silently try to allocate ~256 MB of {@code long[]}.</li>
     *   <li><b>Integer overflow in ensureCapacity</b>: the doubling loop
     *       {@code while (newCap <= i) newCap *= 2} wraps to a negative value
     *       when {@code i} is near {@code Integer.MAX_VALUE}, causing an
     *       {@code ArrayIndexOutOfBoundsException} or silent corruption.</li>
     * </ul>
     *
     * <p>8 MB is generous for a single bitset. Adjust if your application
     * genuinely needs larger bitsets with untrusted size inputs.
     */
    static final int MAX_BITS = 1 << 26; // 67,108,864 bits ≈ 8 MB

    // =========================================================================
    // Fields
    // =========================================================================

    /**
     * Packed bit storage. Each {@code long} holds 64 bits.
     *
     * <p>{@code words[0]} holds bits 0–63, {@code words[1]} holds bits 64–127, etc.
     * Bits beyond {@link #length} in the last word are always zero.
     */
    private long[] words;

    /**
     * Logical size: the number of bits the user considers addressable.
     *
     * <p>Bits 0 through {@code length-1} are "real". Bits from {@code length}
     * to {@code capacity-1} exist in memory but are always zero.
     */
    private int length;

    // =========================================================================
    // Constructors
    // =========================================================================

    /**
     * Creates a new bitset with all bits initially zero.
     *
     * <p>The {@code size} parameter sets the logical length. The capacity is
     * rounded up to the next multiple of 64.
     *
     * <pre>
     *   Bitset bs = new Bitset(100);
     *   // bs.length() == 100
     *   // bs.capacity() == 128  (2 words × 64 bits/word)
     *   // bs.popcount() == 0    (all bits start as zero)
     * </pre>
     *
     * <p>{@code new Bitset(0)} is valid and creates an empty bitset with
     * length=0, capacity=0.
     *
     * @param size the number of addressable bits
     */
    public Bitset(int size) {
        if (size < 0) {
            throw new IllegalArgumentException(
                "size must be non-negative, got: " + size);
        }
        if (size > MAX_BITS) {
            throw new IllegalArgumentException(
                "size " + size + " exceeds maximum allowed bits (" + MAX_BITS + ")");
        }
        this.words = new long[wordsNeeded(size)];
        this.length = size;
    }

    // Private constructor used internally to build a Bitset from an existing
    // word array and length without copying (for bulk ops returning new Bitsets).
    private Bitset(long[] words, int length) {
        this.words = words;
        this.length = length;
    }

    // =========================================================================
    // Factory methods
    // =========================================================================

    /**
     * Creates a bitset from a non-negative integer (treated as unsigned 64-bit).
     *
     * <p>Bit 0 of the bitset is the least significant bit of {@code value}.
     * The length of the result is the position of the highest set bit + 1.
     * If {@code value == 0}, then length = 0 (empty bitset).
     *
     * <pre>
     *   Bitset bs = Bitset.fromInteger(5L);  // binary: 101
     *   // bs.length() == 3
     *   // bs.test(0) == true    (bit 0 = 1)
     *   // bs.test(1) == false   (bit 1 = 0)
     *   // bs.test(2) == true    (bit 2 = 1)
     * </pre>
     *
     * @param value the unsigned 64-bit value; treated as non-negative
     * @return a new bitset representing the given value
     */
    public static Bitset fromInteger(long value) {
        // Special case: zero produces an empty bitset.
        if (value == 0L) {
            return new Bitset(0);
        }

        // Long.SIZE - Long.numberOfLeadingZeros(v) gives the number of bits
        // needed to represent v (i.e., position of the highest set bit + 1).
        // This works correctly even for negative signed values because
        // numberOfLeadingZeros counts literal leading 0 bits.
        int length = Long.SIZE - Long.numberOfLeadingZeros(value);
        return new Bitset(new long[]{value}, length);
    }

    /**
     * Creates a bitset from a string of {@code '0'} and {@code '1'} characters.
     *
     * <p>The leftmost character is the highest-indexed bit (conventional binary
     * notation, matching how humans write numbers). The rightmost character is
     * bit 0.
     *
     * <pre>
     *   Input string: "1 0 1 0"
     *   Position:      3 2 1 0   (leftmost = highest bit index)
     *
     *   This is the same as the integer 10 (binary 1010).
     * </pre>
     *
     * <p>An empty string produces an empty bitset with length=0.
     *
     * @param s a string of '0' and '1' characters; must not be null
     * @return a new bitset
     * @throws IllegalArgumentException if the string contains any character
     *                                  other than '0' or '1'
     */
    public static Bitset fromBinaryStr(String s) {
        Objects.requireNonNull(s, "s must not be null");
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            if (c != '0' && c != '1') {
                throw new IllegalArgumentException(
                    "invalid character '" + c + "' at index " + i + " in binary string");
            }
        }

        if (s.isEmpty()) {
            return new Bitset(0);
        }

        int length = s.length();
        Bitset bs = new Bitset(length);

        // Walk the string from right to left (bit 0 = rightmost char).
        // s.charAt(length-1-i) is the character for bit index i.
        for (int i = 0; i < length; i++) {
            int charIdx = length - 1 - i; // bit index i → string position charIdx
            if (s.charAt(charIdx) == '1') {
                bs.words[wordIndex(i)] |= bitmask(i);
            }
        }

        bs.cleanTrailingBits();
        return bs;
    }

    // =========================================================================
    // Single-bit operations
    // =========================================================================
    //
    // Growth semantics:
    //   • set(i) and toggle(i) AUTO-GROW the bitset if i >= length.
    //   • test(i) and clear(i) do NOT grow. They return false / do nothing
    //     for out-of-range indices. This is safe because unallocated bits are
    //     conceptually zero.

    /**
     * Sets bit {@code i} to 1. Auto-grows the bitset if {@code i >= length}.
     *
     * <p>How auto-growth works: if {@code i} is beyond the current capacity,
     * we double the capacity repeatedly until it's large enough (minimum 64
     * bits). This is the same amortised O(1) strategy as Java's ArrayList.
     *
     * <p>The core operation uses OR to turn on the target bit:
     * <pre>
     *   words[w] = 0b...0000_0000
     *   mask     = 0b...0010_0000   (bit 5 within the word)
     *   result   = 0b...0010_0000   (bit 5 is now set)
     * </pre>
     *
     * <p>OR is idempotent: setting an already-set bit is a no-op.
     *
     * @param i the bit index (0-based); must be non-negative
     */
    public void set(int i) {
        ensureCapacity(i);
        words[wordIndex(i)] |= bitmask(i);
    }

    /**
     * Sets bit {@code i} to 0. No-op if {@code i >= length} (does not grow).
     *
     * <p>Clearing a bit that's already 0 is a no-op. Clearing a bit beyond the
     * bitset's length is also a no-op — there's nothing to clear, because
     * unallocated bits are conceptually zero.
     *
     * <p>How it works: AND with the inverted mask. The inverted mask has all
     * bits set EXCEPT the target bit, so every other bit is preserved:
     * <pre>
     *   words[w] = 0b...0010_0100   (bits 2 and 5 set)
     *   mask     = 0b...0010_0000   (bit 5)
     *   ~mask    = 0b...1101_1111   (everything except bit 5)
     *   result   = 0b...0000_0100   (bit 5 cleared, bit 2 preserved)
     * </pre>
     *
     * @param i the bit index (0-based)
     */
    public void clear(int i) {
        if (i < 0) {
            throw new IllegalArgumentException(
                "bit index must be non-negative, got: " + i);
        }
        if (i >= length) {
            return; // out of range: nothing to clear
        }
        words[wordIndex(i)] &= ~bitmask(i);
    }

    /**
     * Returns whether bit {@code i} is set. Returns {@code false} if
     * {@code i >= length}.
     *
     * <p>This is a pure read operation — it never modifies the bitset. Testing
     * a bit beyond the bitset's length returns false because unallocated bits
     * are conceptually zero.
     *
     * <p>How it works: AND with the mask isolates the target bit:
     * <pre>
     *   words[w] = 0b...0010_0100   (bits 2 and 5 set)
     *   mask     = 0b...0010_0000   (bit 5)
     *   result   = 0b...0010_0000   (non-zero → bit 5 is set)
     *
     *   mask     = 0b...0000_1000   (bit 3)
     *   result   = 0b...0000_0000   (zero → bit 3 is not set)
     * </pre>
     *
     * @param i the bit index (0-based)
     * @return {@code true} if bit i is set
     */
    public boolean test(int i) {
        if (i < 0) {
            throw new IllegalArgumentException(
                "bit index must be non-negative, got: " + i);
        }
        if (i >= length) {
            return false; // out of range: conceptually zero
        }
        return (words[wordIndex(i)] & bitmask(i)) != 0L;
    }

    /**
     * Flips bit {@code i} (0 becomes 1, 1 becomes 0). Auto-grows if
     * {@code i >= length}.
     *
     * <p>How it works: XOR with the bitmask flips exactly one bit:
     * <pre>
     *   words[w] = 0b...0010_0100   (bits 2 and 5 set)
     *   mask     = 0b...0010_0000   (bit 5)
     *   result   = 0b...0000_0100   (bit 5 flipped to 0)
     * </pre>
     *
     * @param i the bit index (0-based); must be non-negative
     */
    public void toggle(int i) {
        ensureCapacity(i);
        words[wordIndex(i)] ^= bitmask(i);
        // Toggle might have set a bit in the last word's trailing region.
        // Clean trailing bits to maintain the invariant.
        cleanTrailingBits();
    }

    // =========================================================================
    // Bulk bitwise operations
    // =========================================================================
    //
    // All bulk operations return a NEW bitset. They don't modify either
    // operand. The result has length = max(a.length, b.length).
    //
    // When two bitsets have different lengths, the shorter one is
    // "zero-extended" conceptually. In practice we just stop reading from the
    // shorter one's words once they run out and treat missing words as zero.
    //
    // Performance: each operation processes one 64-bit word per loop iteration,
    // so 64 bits are handled in a single CPU instruction.

    /**
     * Returns a new bitset where each bit is 1 only if BOTH corresponding
     * input bits are 1.
     *
     * <pre>
     *   A  B  A&amp;B
     *   0  0   0
     *   0  1   0
     *   1  0   0
     *   1  1   1
     * </pre>
     *
     * <p>AND is used for intersection: elements that are in both sets.
     *
     * @param other the other bitset; must not be null
     * @return a new bitset representing the intersection
     */
    public Bitset and(Bitset other) {
        Objects.requireNonNull(other, "other must not be null");
        int resultLen = Math.max(this.length, other.length);
        int maxWords = Math.max(this.words.length, other.words.length);
        long[] resultWords = new long[maxWords];

        for (int i = 0; i < maxWords; i++) {
            resultWords[i] = wordAt(this.words, i) & wordAt(other.words, i);
        }

        Bitset result = new Bitset(resultWords, resultLen);
        result.cleanTrailingBits();
        return result;
    }

    /**
     * Returns a new bitset where each bit is 1 if EITHER (or both)
     * corresponding input bits are 1.
     *
     * <pre>
     *   A  B  A|B
     *   0  0   0
     *   0  1   1
     *   1  0   1
     *   1  1   1
     * </pre>
     *
     * <p>OR is used for union: elements that are in either set.
     *
     * @param other the other bitset; must not be null
     * @return a new bitset representing the union
     */
    public Bitset or(Bitset other) {
        Objects.requireNonNull(other, "other must not be null");
        int resultLen = Math.max(this.length, other.length);
        int maxWords = Math.max(this.words.length, other.words.length);
        long[] resultWords = new long[maxWords];

        for (int i = 0; i < maxWords; i++) {
            resultWords[i] = wordAt(this.words, i) | wordAt(other.words, i);
        }

        Bitset result = new Bitset(resultWords, resultLen);
        result.cleanTrailingBits();
        return result;
    }

    /**
     * Returns a new bitset where each bit is 1 if the corresponding input bits
     * DIFFER.
     *
     * <pre>
     *   A  B  A^B
     *   0  0   0
     *   0  1   1
     *   1  0   1
     *   1  1   0
     * </pre>
     *
     * <p>XOR is used for symmetric difference: elements in either set but not
     * both.
     *
     * @param other the other bitset; must not be null
     * @return a new bitset representing the symmetric difference
     */
    public Bitset xor(Bitset other) {
        Objects.requireNonNull(other, "other must not be null");
        int resultLen = Math.max(this.length, other.length);
        int maxWords = Math.max(this.words.length, other.words.length);
        long[] resultWords = new long[maxWords];

        for (int i = 0; i < maxWords; i++) {
            resultWords[i] = wordAt(this.words, i) ^ wordAt(other.words, i);
        }

        Bitset result = new Bitset(resultWords, resultLen);
        result.cleanTrailingBits();
        return result;
    }

    /**
     * Returns a new bitset with every bit flipped within {@code length}.
     *
     * <pre>
     *   A  ~A
     *   0   1
     *   1   0
     * </pre>
     *
     * <p>NOT is used for complement: elements NOT in the set.
     *
     * <p><b>Important</b>: NOT flips bits within length, NOT within capacity.
     * Bits beyond length remain zero (clean-trailing-bits invariant). The
     * result has the same length as the input.
     *
     * @return a new bitset representing the complement
     */
    public Bitset not() {
        long[] resultWords = new long[words.length];
        for (int i = 0; i < words.length; i++) {
            resultWords[i] = ~words[i];
        }

        // Critical: clean trailing bits! NOT flipped ALL bits including the
        // trailing bits beyond length that were zero. We must zero them out
        // again to maintain the clean-trailing-bits invariant.
        //
        //   Before NOT: word[3] = 0b00000000_XXXXXXXX  (trailing bits are 0)
        //   After  NOT: word[3] = 0b11111111_xxxxxxxx  (trailing bits are 1!)
        //   After clean: word[3] = 0b00000000_xxxxxxxx  (trailing bits zeroed)
        Bitset result = new Bitset(resultWords, this.length);
        result.cleanTrailingBits();
        return result;
    }

    /**
     * Returns a new bitset with bits in {@code this} that are NOT in
     * {@code other} (set difference).
     *
     * <p>Equivalent to {@code this.and(other.not())}, but more efficient
     * because we don't need to allocate an intermediate NOT result.
     *
     * <pre>
     *   A  B  A &amp; ~B
     *   0  0    0
     *   0  1    0
     *   1  0    1
     *   1  1    0
     * </pre>
     *
     * <p>AND-NOT is used for set difference: elements in A but not in B.
     *
     * @param other the other bitset; must not be null
     * @return a new bitset representing the set difference
     */
    public Bitset andNot(Bitset other) {
        Objects.requireNonNull(other, "other must not be null");
        int resultLen = Math.max(this.length, other.length);
        int maxWords = Math.max(this.words.length, other.words.length);
        long[] resultWords = new long[maxWords];

        for (int i = 0; i < maxWords; i++) {
            // a & ~b: keep bits from a that are NOT in b
            resultWords[i] = wordAt(this.words, i) & ~wordAt(other.words, i);
        }

        Bitset result = new Bitset(resultWords, resultLen);
        result.cleanTrailingBits();
        return result;
    }

    // =========================================================================
    // Counting and query operations
    // =========================================================================

    /**
     * Returns the number of set (1) bits.
     *
     * <p>Named after the CPU instruction POPCNT (population count) that does
     * this for a single word. We call {@link Long#bitCount(long)} on each word
     * and sum the results. On modern JVMs this often compiles to the hardware
     * POPCNT instruction.
     *
     * <p>For a bitset with N bits, this runs in O(N/64) time.
     *
     * @return the population count
     */
    public int popcount() {
        int count = 0;
        for (long w : words) {
            count += Long.bitCount(w);
        }
        return count;
    }

    /**
     * Returns the logical length: the number of addressable bits.
     *
     * <p>This is the value passed to {@link #Bitset(int)}, or the highest bit
     * index + 1 after any auto-growth operations.
     *
     * @return the logical length
     */
    public int length() {
        return length;
    }

    /**
     * Returns the allocated size in bits (always a multiple of 64).
     *
     * <p>Capacity &ge; {@link #length()}. The difference (capacity - length)
     * is "slack space" — bits that exist in memory but are always zero.
     *
     * @return the capacity in bits
     */
    public int capacity() {
        return words.length * BITS_PER_WORD;
    }

    /**
     * Returns {@code true} if at least one bit is set.
     *
     * <p>Short-circuits: returns as soon as it finds a non-zero word, without
     * scanning the rest. This is O(1) in the best case and O(N/64) worst case.
     *
     * @return {@code true} if any bit is set
     */
    public boolean any() {
        for (long w : words) {
            if (w != 0L) {
                return true;
            }
        }
        return false;
    }

    /**
     * Returns {@code true} if ALL bits in 0..length are set.
     *
     * <p>For an empty bitset (length = 0), returns {@code true} — this is
     * vacuous truth, the same convention used by Java's
     * {@code Stream.allMatch()} on empty streams.
     *
     * <p>How it works: for each full word check {@code word == -1L} (all 64
     * bits are 1). For the last word, check only the bits within length using
     * a mask.
     *
     * @return {@code true} if all bits are set
     */
    public boolean all() {
        if (length == 0) {
            return true; // vacuous truth
        }

        int numWords = words.length;

        // Check all full words: every bit must be set (-1L = all 1s).
        for (int i = 0; i < numWords - 1; i++) {
            if (words[i] != -1L) {
                return false;
            }
        }

        // Check the last word: only bits within length matter.
        int remaining = bitOffset(length);
        if (remaining == 0) {
            // length is a multiple of 64 — the last word is a full word.
            return words[numWords - 1] == -1L;
        }

        // Mask for valid bits: (1L << remaining) - 1
        // Example: remaining=8 → mask=0xFF (bits 0-7 only)
        long mask = (1L << remaining) - 1L;
        return words[numWords - 1] == mask;
    }

    /**
     * Returns {@code true} if no bits are set. Equivalent to {@code !any()}.
     *
     * @return {@code true} if no bit is set
     */
    public boolean none() {
        return !any();
    }

    // =========================================================================
    // Iteration
    // =========================================================================

    /**
     * Returns the indices of all set bits in ascending order.
     *
     * <p>Uses the trailing-zero-count trick for efficiency. For each non-zero
     * word, we find the lowest set bit with
     * {@link Long#numberOfTrailingZeros(long)}, record its index, then clear
     * it with {@code word &= word - 1}:
     *
     * <pre>
     *   word = 0b10100100   (bits 2, 5, 7 are set)
     *
     *   Step 1: trailing_zeros = 2  → record base + 2
     *           word &amp;= word - 1   → 0b10100000  (bit 2 cleared)
     *
     *   Step 2: trailing_zeros = 5  → record base + 5
     *           word &amp;= word - 1   → 0b10000000  (bit 5 cleared)
     *
     *   Step 3: trailing_zeros = 7  → record base + 7
     *           word &amp;= word - 1   → 0b00000000  (bit 7 cleared)
     * </pre>
     *
     * <p>The trick {@code word &= word - 1} clears the lowest set bit. Here's
     * why:
     * <pre>
     *   word     = 0b10100100
     *   word - 1 = 0b10100011  (borrow propagates through trailing zeros)
     *   AND      = 0b10100000  (lowest set bit is cleared)
     * </pre>
     *
     * <p>This is O(k) where k is the number of set bits, and it skips zero
     * words entirely, making it very efficient for sparse bitsets.
     *
     * @return a list of bit indices of all set bits, in ascending order
     */
    public List<Integer> iterSetBits() {
        List<Integer> indices = new ArrayList<>();

        for (int wordIdx = 0; wordIdx < words.length; wordIdx++) {
            long w = words[wordIdx];
            int baseIndex = wordIdx * BITS_PER_WORD;

            while (w != 0L) {
                // Find the lowest set bit position within this word.
                int bitPos = Long.numberOfTrailingZeros(w);
                int index = baseIndex + bitPos;

                // Only include bits within length (don't return trailing garbage).
                if (index >= length) {
                    break;
                }

                indices.add(index);

                // Clear the lowest set bit: w &= w - 1
                w &= w - 1L;
            }
        }

        return indices;
    }

    // =========================================================================
    // Conversion operations
    // =========================================================================

    /**
     * Converts the bitset to an unsigned 64-bit integer (returned as a
     * signed {@code long} in Java).
     *
     * <p>Returns 0 for an empty bitset. Throws if the bitset has set bits
     * beyond position 63 (i.e., it requires more than one word).
     *
     * @return the unsigned 64-bit value
     * @throws ArithmeticException if the bitset value exceeds 64 bits
     */
    public long toInteger() {
        if (words.length == 0) {
            return 0L;
        }

        // Any word beyond the first must be zero for a valid conversion.
        for (int i = 1; i < words.length; i++) {
            if (words[i] != 0L) {
                throw new ArithmeticException("bitset value exceeds 64-bit range");
            }
        }

        return words[0];
    }

    /**
     * Converts the bitset to a string of {@code '0'} and {@code '1'}
     * characters with the highest bit on the left (conventional binary
     * notation).
     *
     * <p>This is the inverse of {@link #fromBinaryStr(String)}. An empty
     * bitset produces an empty string {@code ""}.
     *
     * <pre>
     *   Bitset.fromInteger(5L).toBinaryStr()  // returns "101"
     * </pre>
     *
     * @return a binary string representing the bitset
     */
    public String toBinaryStr() {
        if (length == 0) {
            return "";
        }

        // Build the string from the highest bit (length-1) down to bit 0.
        // This produces conventional binary notation: MSB on the left.
        StringBuilder sb = new StringBuilder(length);
        for (int i = length - 1; i >= 0; i--) {
            sb.append(test(i) ? '1' : '0');
        }
        return sb.toString();
    }

    // =========================================================================
    // Equality and Object overrides
    // =========================================================================

    /**
     * Returns {@code true} if two bitsets have the same length and the same
     * bits set.
     *
     * <p>Capacity is irrelevant to equality — a bitset with capacity=128 can
     * equal one with capacity=256 if their length and set bits match.
     *
     * <p>Thanks to the clean-trailing-bits invariant we can compare words
     * directly — trailing bits are always zero, so two bitsets with the same
     * logical content will have identical word vectors.
     *
     * @param o the object to compare with
     * @return {@code true} if equal
     */
    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (!(o instanceof Bitset)) return false;
        Bitset other = (Bitset) o;

        if (this.length != other.length) return false;

        int maxWords = Math.max(this.words.length, other.words.length);
        for (int i = 0; i < maxWords; i++) {
            if (wordAt(this.words, i) != wordAt(other.words, i)) {
                return false;
            }
        }
        return true;
    }

    /** {@inheritDoc} */
    @Override
    public int hashCode() {
        int result = length;
        result = 31 * result + Arrays.hashCode(words);
        return result;
    }

    /**
     * Returns a human-readable representation like {@code "Bitset(101)"}.
     *
     * @return a string representation
     */
    @Override
    public String toString() {
        return "Bitset(" + toBinaryStr() + ")";
    }

    // =========================================================================
    // Internal helpers
    // =========================================================================

    /**
     * Computes how many {@code long} words we need to store {@code bitCount}
     * bits. This is ceiling division: {@code (bitCount + 63) / 64}.
     *
     * <pre>
     *   wordsNeeded(0)   = 0
     *   wordsNeeded(1)   = 1
     *   wordsNeeded(64)  = 1
     *   wordsNeeded(65)  = 2
     *   wordsNeeded(200) = 4   (ceil(200/64) = 4)
     * </pre>
     */
    private static int wordsNeeded(int bitCount) {
        return (bitCount + BITS_PER_WORD - 1) / BITS_PER_WORD;
    }

    /**
     * Computes which word contains bit {@code i}. Simply {@code i / 64}.
     *
     * <pre>
     *   wordIndex(0)   = 0   (bit 0 is in word 0)
     *   wordIndex(63)  = 0   (bit 63 is the last bit of word 0)
     *   wordIndex(64)  = 1   (bit 64 is the first bit of word 1)
     * </pre>
     */
    private static int wordIndex(int i) {
        return i / BITS_PER_WORD;
    }

    /**
     * Computes which bit position within its word bit {@code i} occupies.
     * Simply {@code i % 64}.
     *
     * <pre>
     *   bitOffset(0)   = 0
     *   bitOffset(63)  = 63
     *   bitOffset(64)  = 0   (first bit of the next word)
     * </pre>
     */
    private static int bitOffset(int i) {
        return i % BITS_PER_WORD;
    }

    /**
     * Returns a {@code long} mask with only bit {@code i} set within its word.
     *
     * <p>This is {@code 1L << (i % 64)}. We use this mask to isolate, set,
     * clear, or toggle a single bit:
     * <pre>
     *   To set bit i:    word |= bitmask(i)       (OR turns bit on)
     *   To clear bit i:  word &amp;= ~bitmask(i)    (AND with ~mask turns bit off)
     *   To test bit i:   (word &amp; bitmask(i)) != 0 (AND isolates the bit)
     *   To toggle bit i: word ^= bitmask(i)       (XOR flips the bit)
     * </pre>
     */
    private static long bitmask(int i) {
        return 1L << bitOffset(i);
    }

    /**
     * Safely gets a word from a slice, returning 0L if the index is out of
     * bounds. Simplifies bulk operations between bitsets of different sizes —
     * missing words are treated as zero.
     */
    private static long wordAt(long[] words, int i) {
        return i < words.length ? words[i] : 0L;
    }

    /**
     * Ensures the bitset has capacity for bit {@code i}. If not, grows by
     * doubling.
     *
     * <p>After this call, {@code i < capacity()} and {@code length >= i + 1}.
     *
     * <p>Growth strategy: we double capacity repeatedly until it exceeds
     * {@code i}. The minimum capacity after growth is 64 (one word). This
     * doubling gives amortised O(1) growth, just like {@code ArrayList}.
     *
     * <pre>
     *   Example: capacity=128, set(500)
     *   128 → 256 → 512 → 1024  (stop: 500 &lt; 1024)
     * </pre>
     *
     * <p><b>Bounds guards</b>:
     * <ul>
     *   <li>Negative indices are rejected immediately.</li>
     *   <li>Indices at or above {@link #MAX_BITS} are rejected to prevent
     *       the doubling loop from overflowing {@code int} arithmetic and
     *       causing a silent wrap-around into negative values.</li>
     * </ul>
     */
    private void ensureCapacity(int i) {
        if (i < 0) {
            throw new IllegalArgumentException(
                "bit index must be non-negative, got: " + i);
        }
        if (i >= MAX_BITS) {
            throw new IllegalArgumentException(
                "bit index " + i + " exceeds maximum allowed bits (" + MAX_BITS + ")");
        }

        if (i < capacity()) {
            // Already have room. But we might need to extend length.
            if (i >= length) {
                length = i + 1;
            }
            return;
        }

        // Need to grow. Start with current capacity (or 64 as minimum).
        int newCap = capacity();
        if (newCap < BITS_PER_WORD) {
            newCap = BITS_PER_WORD;
        }
        // Double until we have room. The i >= MAX_BITS guard above means
        // newCap can never overflow: MAX_BITS (1<<26) fits comfortably in int,
        // and doubling from MAX_BITS/2 still fits (1<<27 < Integer.MAX_VALUE).
        while (newCap <= i) {
            newCap *= 2;
        }

        // Extend the word slice with zeros.
        int newWordCount = newCap / BITS_PER_WORD;
        words = Arrays.copyOf(words, newWordCount); // Arrays.copyOf zero-fills

        length = i + 1;
    }

    /**
     * Zeroes out any bits beyond {@code length} in the last word.
     *
     * <p>This maintains the clean-trailing-bits invariant. It must be called
     * after any operation that might set bits beyond length:
     * <ul>
     *   <li>{@link #not()} flips all bits, including trailing ones</li>
     *   <li>{@link #toggle(int)} on the last word</li>
     *   <li>Bulk operations (AND, OR, XOR) when operands have different sizes</li>
     * </ul>
     *
     * <p>How it works:
     * <pre>
     *   length = 200, capacity = 256
     *   The last word holds bits 192–255, but only 192–199 are "real".
     *   remaining = 200 % 64 = 8
     *   mask = (1L &lt;&lt; 8) - 1 = 0xFF  (bits 0-7)
     *   words[3] &amp;= 0xFF  → zeroes out bits 8–63 of word 3
     * </pre>
     *
     * <p>If length is a multiple of 64, there are no trailing bits to clean.
     */
    private void cleanTrailingBits() {
        if (length == 0 || words.length == 0) {
            return;
        }

        int remaining = bitOffset(length);
        if (remaining != 0) {
            int lastIdx = words.length - 1;
            long mask = (1L << remaining) - 1L;
            words[lastIdx] &= mask;
        }
    }
}
