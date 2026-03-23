// bitset.ts -- Bitset: A Compact Boolean Array Packed into 32-bit Words
// =====================================================================
//
// A bitset stores a sequence of bits -- each one either 0 or 1 -- packed
// into machine-word-sized integers. Instead of using an entire byte (or
// a pointer-sized boolean) to represent a single true/false value, a
// bitset packs 32 of them into a single 32-bit word.
//
// Why 32-bit words instead of 64?
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// JavaScript has no native 64-bit integer type. `Number` is a 64-bit
// IEEE 754 float that can only represent integers exactly up to 2^53.
// `BigInt` exists but is slow and cannot be used with typed arrays.
// `Uint32Array` gives us fixed-size 32-bit unsigned integers with
// predictable bitwise behavior -- exactly what a bitset needs.
//
// Why does this matter?
//
// 1. **Space**: 10,000 booleans in a JS array = ~80,000 bytes (each
//    boolean is a heap-allocated value with a pointer in the array).
//    As a bitset using Uint32Array = ~1,250 bytes. A 64x improvement.
//
// 2. **Speed**: AND-ing two boolean arrays loops over 10,000 elements.
//    AND-ing two bitsets loops over ~313 words. The engine performs a
//    single 32-bit AND on each word, operating on 32 bits at once.
//
// 3. **Ubiquity**: Bitsets appear in Bloom filters, register allocators,
//    graph algorithms (visited sets), database bitmap indexes, filesystem
//    free-block bitmaps, and garbage collectors.
//
// Bit Ordering: LSB-First
// -----------------------
//
// We use Least Significant Bit first ordering. Bit 0 is the least
// significant bit of word 0. Bit 31 is the most significant bit of
// word 0. Bit 32 is the least significant bit of word 1. And so on.
//
//     Word 0                              Word 1
//     ┌─────────────────────────────┐     ┌─────────────────────────────┐
//     │ bit 31  ...  bit 2  bit 1  bit 0│ │ bit 63  ... bit 33  bit 32 │
//     └─────────────────────────────┘     └─────────────────────────────┘
//     MSB ◄─────────────────── LSB        MSB ◄─────────────────── LSB
//
// The three fundamental formulas that drive every bitset operation:
//
//     word_index = i >> 5        (which word contains bit i? i / 32)
//     bit_offset = i & 31       (which position within that word? i % 32)
//     bitmask    = 1 << (i & 31)  (a mask with only bit i set)
//
// These are the heart of the entire implementation.
//
// Why >> 5 and & 31 instead of / 32 and % 32?
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// For powers of two, division and modulo can be expressed as bit shifts
// and masks. Since 32 = 2^5:
//   - i / 32  ===  i >> 5    (shift right by 5 drops the bottom 5 bits)
//   - i % 32  ===  i & 31    (mask with 0b11111 keeps only bottom 5 bits)
//
// These bitwise forms are preferred because:
//   1. They make the power-of-two relationship explicit in the code
//   2. They're guaranteed to be fast (single CPU instructions)
//   3. They match the convention used in bitset implementations everywhere

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
//
// BITS_PER_WORD is 32 because we use Uint32Array as our word type.
// Every formula in this module uses this constant rather than a magic
// number, so if someone wanted to experiment with different word sizes,
// they'd only need to change this constant and the array type.

/** Number of bits stored in each word of the bitset. */
const BITS_PER_WORD = 32;

/** Mask for extracting the bit offset within a word: i & OFFSET_MASK === i % 32. */
const OFFSET_MASK = BITS_PER_WORD - 1; // 31 = 0b11111

/** Shift amount for computing the word index: i >> WORD_SHIFT === i / 32. */
const WORD_SHIFT = 5; // log2(32) = 5

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------
//
// We have exactly one error class: an invalid binary string was passed to
// `fromBinaryStr`. This keeps the error type minimal and focused.

/**
 * Error thrown when a bitset operation receives invalid input.
 *
 * Currently only produced by `fromBinaryStr` when the input string
 * contains characters other than '0' and '1'.
 */
export class BitsetError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "BitsetError";
  }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------
//
// These small utility functions compute the word index, bit offset, and
// number of words needed for a given bit count. They're used throughout
// the implementation.

/**
 * How many 32-bit words do we need to store `bitCount` bits?
 *
 * This is ceiling division: (bitCount + 31) / 32, using bit shifts.
 *
 * ```
 * wordsNeeded(0)   = 0   (no bits, no words)
 * wordsNeeded(1)   = 1   (1 bit needs 1 word)
 * wordsNeeded(32)  = 1   (32 bits fit exactly in 1 word)
 * wordsNeeded(33)  = 2   (33 bits need 2 words)
 * wordsNeeded(64)  = 2   (64 bits fit exactly in 2 words)
 * wordsNeeded(100) = 4   (100 bits need ceil(100/32) = 4 words)
 * ```
 */
function wordsNeeded(bitCount: number): number {
  return (bitCount + OFFSET_MASK) >>> WORD_SHIFT;
}

/**
 * Which word contains bit `i`? Simply i / 32, expressed as i >> 5.
 *
 * ```
 * wordIndex(0)   = 0   (bit 0 is in word 0)
 * wordIndex(31)  = 0   (bit 31 is the last bit of word 0)
 * wordIndex(32)  = 1   (bit 32 is the first bit of word 1)
 * wordIndex(100) = 3   (bit 100 is in word 3)
 * ```
 */
function wordIndex(i: number): number {
  return i >>> WORD_SHIFT;
}

/**
 * A bitmask with only bit `i` set within its word.
 *
 * This is `1 << (i & 31)`. We use this mask to isolate, set, clear,
 * or toggle a single bit within a word using bitwise operations:
 *
 * ```
 * To set bit i:    word |= bitmask(i)     (OR with mask turns bit on)
 * To clear bit i:  word &= ~bitmask(i)    (AND with inverted mask turns bit off)
 * To test bit i:   (word & bitmask(i)) !== 0  (AND with mask isolates the bit)
 * To toggle bit i: word ^= bitmask(i)     (XOR with mask flips the bit)
 * ```
 */
function bitmask(i: number): number {
  return 1 << (i & OFFSET_MASK);
}

/**
 * Count the number of set bits in a 32-bit integer.
 *
 * JavaScript has no built-in `popcnt` instruction, so we use the classic
 * Hamming weight algorithm that processes bits in parallel using a
 * divide-and-conquer approach:
 *
 * Step 1: Count bits in pairs of 1
 *   Mask with 0x55555555 (binary: 01010101...) to isolate even bits.
 *   Shift right by 1 and mask to isolate odd bits.
 *   Add them: each 2-bit field now holds the count of set bits in
 *   the original 2-bit field (0, 1, or 2).
 *
 * Step 2: Count bits in groups of 4
 *   Mask with 0x33333333 (binary: 00110011...) to isolate pairs.
 *   Shift right by 2 and mask to isolate other pairs.
 *   Add them: each 4-bit field now holds a count (0-4).
 *
 * Step 3: Count bits in groups of 8
 *   Add adjacent 4-bit fields and mask with 0x0F0F0F0F.
 *   Each byte now holds a count (0-8).
 *
 * Step 4: Sum all bytes
 *   Multiply by 0x01010101 which sums all bytes into the top byte.
 *   Shift right by 24 to extract the total count.
 *
 * This runs in constant time (no loops, no branches) and is the same
 * algorithm used by hardware POPCNT implementations.
 */
function popcount32(n: number): number {
  n = n - ((n >>> 1) & 0x55555555);
  n = (n & 0x33333333) + ((n >>> 2) & 0x33333333);
  n = (n + (n >>> 4)) & 0x0f0f0f0f;
  return (n * 0x01010101) >>> 24;
}

/**
 * Count trailing zeros in a 32-bit integer.
 *
 * JavaScript has `Math.clz32` (count leading zeros) but no built-in
 * count-trailing-zeros. We use the classic trick:
 *
 *   ctz(n) = 32 - clz32(n & -n)
 *
 * `n & -n` isolates the lowest set bit. Then `clz32` tells us how
 * many leading zeros that isolated bit has. Since there's exactly one
 * bit set, `32 - clz32` gives us its position from the right (i.e.,
 * the number of trailing zeros in the original number).
 *
 * Special case: if n === 0, there are 32 trailing zeros (no bits set).
 *
 * ```
 * ctz32(0b00101000) = 3   (three trailing zeros before bit 3)
 * ctz32(0b00000001) = 0   (no trailing zeros)
 * ctz32(0b10000000_00000000_00000000_00000000) = 31
 * ctz32(0) = 32
 * ```
 */
function ctz32(n: number): number {
  if (n === 0) return 32;
  // n & -n isolates the lowest set bit.
  // For example: 0b00101000 & 0b11011000 (two's complement) = 0b00001000
  // Math.clz32 counts leading zeros of that isolated bit.
  // 32 - clz32 gives us the position from the right.
  return 31 - Math.clz32(n & -n);
}

// ---------------------------------------------------------------------------
// The Bitset class
// ---------------------------------------------------------------------------
//
// Internal Representation
// ~~~~~~~~~~~~~~~~~~~~~~~
//
// We store bits in a Uint32Array called `_words`. Each element holds 32 bits.
// We also track `_len`, the logical size -- the number of bits the user
// considers "addressable". The capacity is always _words.length * 32.
//
//     ┌──────────────────────────────────────────────────────────────────┐
//     │                          capacity (128 bits = 4 words)           │
//     │                                                                  │
//     │  ┌──────────────────────────────────────────┐                    │
//     │  │              len (100 bits)                │ ··· unused ····  │
//     │  │  (highest addressable bit index + 1)       │ (always zero)   │
//     │  └──────────────────────────────────────────┘                    │
//     └──────────────────────────────────────────────────────────────────┘
//
// **Clean-trailing-bits invariant**: Bits beyond `_len` in the last word are
// always zero. This is critical for correctness of popcount, any, all, none,
// equality, and toInteger. Every operation that modifies the last word must
// clean trailing bits afterwards.
//
// **Uint32Array is fixed-size**: Unlike Rust's Vec<u64>, a Uint32Array cannot
// grow in place. When we need more capacity, we allocate a new Uint32Array
// and copy the old data. This is the same approach used by ArrayList in Java.

/**
 * A compact bitset that packs boolean values into 32-bit words.
 *
 * `Bitset` provides O(n/32) bulk bitwise operations (AND, OR, XOR, NOT),
 * efficient iteration over set bits using trailing-zero-count, and
 * ArrayList-style automatic growth when you set bits beyond the current size.
 *
 * @example
 * ```ts
 * const bs = new Bitset(100);
 * bs.set(0);
 * bs.set(42);
 * bs.set(99);
 * console.log(bs.popcount()); // 3
 *
 * // Iterate over set bits
 * for (const idx of bs.iterSetBits()) {
 *   console.log(idx); // 0, 42, 99
 * }
 *
 * // Bulk operations return new bitsets
 * const other = Bitset.fromInteger(42);
 * const intersection = bs.and(other);
 * ```
 */
export class Bitset {
  /** The packed bit storage. Each element holds 32 bits. */
  private _words: Uint32Array;

  /**
   * The logical size: the number of bits the user considers addressable.
   * Bits 0 through _len-1 are "real". Bits from _len to capacity-1 exist
   * in memory but are always zero (the clean-trailing-bits invariant).
   */
  private _len: number;

  // ------------------------------------------------------------------
  // Constructor
  // ------------------------------------------------------------------

  /**
   * Create a new bitset with all bits initially zero.
   *
   * The `size` parameter sets the logical length (`_len`). The capacity
   * is rounded up to the next multiple of 32.
   *
   * @example
   * ```ts
   * const bs = new Bitset(100);
   * console.log(bs.size);       // 100
   * console.log(bs.capacity);   // 128 (4 words * 32 bits/word)
   * console.log(bs.popcount()); // 0  (all bits start as zero)
   * ```
   *
   * `new Bitset(0)` is valid and creates an empty bitset:
   * ```ts
   * const bs = new Bitset(0);
   * console.log(bs.size);     // 0
   * console.log(bs.capacity); // 0
   * ```
   */
  constructor(size: number) {
    this._words = new Uint32Array(wordsNeeded(size));
    this._len = size;
  }

  // ------------------------------------------------------------------
  // Static factory methods (constructors)
  // ------------------------------------------------------------------

  /**
   * Create a bitset from a non-negative integer.
   *
   * Bit 0 of the bitset is the least significant bit of `value`.
   * The `len` of the result is the position of the highest set bit + 1.
   * If `value === 0`, then `len = 0`.
   *
   * Because JavaScript Numbers are 64-bit floats, only integers up to
   * 2^53 - 1 (Number.MAX_SAFE_INTEGER) can be represented exactly.
   * Values beyond this will lose precision. We use 32-bit words, so
   * we split the value into 32-bit chunks.
   *
   * @example
   * ```ts
   * const bs = Bitset.fromInteger(5);  // binary: 101
   * console.log(bs.size);    // 3  (highest bit at position 2)
   * console.log(bs.test(0)); // true   (bit 0 = 1)
   * console.log(bs.test(1)); // false  (bit 1 = 0)
   * console.log(bs.test(2)); // true   (bit 2 = 1)
   * ```
   */
  static fromInteger(value: number): Bitset {
    // Special case: zero produces an empty bitset.
    if (value === 0) {
      return new Bitset(0);
    }

    if (value < 0 || !Number.isInteger(value)) {
      throw new BitsetError(
        `fromInteger requires a non-negative integer, got: ${value}`
      );
    }

    // Determine how many bits we need. For a positive integer, the
    // highest set bit position is floor(log2(value)), and we need
    // that position + 1 bits.
    //
    // For values that fit in 32 bits, we can use: 32 - Math.clz32(value).
    // For larger values, we split into 32-bit words.
    //
    // We build the words array by extracting 32 bits at a time from
    // the value. Since JS Numbers are floats, we use division and
    // modulo with unsigned right shift to extract each word.
    const words: number[] = [];
    let remaining = value;

    while (remaining > 0) {
      // Extract the lowest 32 bits. We use `>>> 0` to convert to uint32.
      // For values > 2^32, `remaining & 0xFFFFFFFF` would not work because
      // JS bitwise operators truncate to 32 bits. Instead, use modulo.
      words.push(remaining % 0x100000000);

      // Shift right by 32 bits using division (since >>> only works on 32-bit).
      remaining = Math.floor(remaining / 0x100000000);
    }

    // The logical length is the position of the highest set bit + 1.
    // The highest set bit is in the last word. We find it using clz32.
    const lastWord = words[words.length - 1];
    const bitsInLastWord = 32 - Math.clz32(lastWord);
    const len = (words.length - 1) * BITS_PER_WORD + bitsInLastWord;

    const bs = new Bitset(0);
    bs._words = new Uint32Array(words);
    bs._len = len;
    return bs;
  }

  /**
   * Create a bitset from a string of '0' and '1' characters.
   *
   * The leftmost character is the highest-indexed bit (conventional binary
   * notation, matching how humans write numbers). The rightmost character
   * is bit 0.
   *
   * String-to-bits mapping:
   * ```
   * Input string: "1 0 1 0"
   * Position:      3 2 1 0    (leftmost = highest bit index)
   *
   * Bit 0 = '0' (rightmost char)
   * Bit 1 = '1'
   * Bit 2 = '0'
   * Bit 3 = '1' (leftmost char)
   *
   * This is the same as the integer 10 (binary 1010).
   * ```
   *
   * @throws {BitsetError} if the string contains any character other than '0' or '1'
   *
   * @example
   * ```ts
   * const bs = Bitset.fromBinaryStr("1010");
   * console.log(bs.size);    // 4
   * console.log(bs.test(1)); // true   (bit 1 = '1')
   * console.log(bs.test(3)); // true   (bit 3 = '1')
   * console.log(bs.test(0)); // false  (bit 0 = '0')
   * ```
   */
  static fromBinaryStr(s: string): Bitset {
    // Validate: every character must be '0' or '1'.
    for (let i = 0; i < s.length; i++) {
      if (s[i] !== "0" && s[i] !== "1") {
        throw new BitsetError(`invalid binary string: "${s}"`);
      }
    }

    // Empty string produces an empty bitset.
    if (s.length === 0) {
      return new Bitset(0);
    }

    // The string length is the logical len of the bitset.
    const len = s.length;
    const bs = new Bitset(len);

    // Walk the string from right to left (LSB to MSB).
    // The rightmost character (index s.length-1) is bit 0.
    // The leftmost character (index 0) is bit s.length-1.
    for (let charIdx = 0; charIdx < s.length; charIdx++) {
      const ch = s[s.length - 1 - charIdx];
      if (ch === "1") {
        // charIdx is the bit index (0 = rightmost = LSB).
        const wi = wordIndex(charIdx);
        bs._words[wi] |= bitmask(charIdx);
      }
    }

    // Clean trailing bits defensively.
    bs.cleanTrailingBits();

    return bs;
  }

  // ------------------------------------------------------------------
  // Single-bit operations
  // ------------------------------------------------------------------
  //
  // These are the bread-and-butter operations: set a bit, clear a bit,
  // test whether a bit is set, toggle a bit. Each one translates to a
  // single bitwise operation on the containing word.
  //
  // Growth semantics:
  //   - set(i) and toggle(i) AUTO-GROW the bitset if i >= _len.
  //   - test(i) and clear(i) do NOT grow. They return false / do nothing
  //     for out-of-range indices. This is safe because unallocated bits
  //     are conceptually zero.

  /**
   * Set bit `i` to 1. Auto-grows the bitset if `i >= len`.
   *
   * How auto-growth works:
   *
   * If `i` is beyond the current capacity, we double the capacity
   * repeatedly until it's large enough (with a minimum of 32 bits).
   * This is the same amortized O(1) strategy used by ArrayList and
   * Python's list.
   *
   * ```
   * Before: len=50, capacity=64 (2 words)
   * set(100): 100 >= 64, so double: 64 -> 128. Now 100 < 128.
   * After: len=101, capacity=128 (4 words)
   * ```
   *
   * @example
   * ```ts
   * const bs = new Bitset(10);
   * bs.set(5);
   * console.log(bs.test(5)); // true
   *
   * // Auto-growth:
   * bs.set(100); // grows from len=10 to len=101
   * console.log(bs.size); // 101
   * console.log(bs.test(100)); // true
   * ```
   */
  set(i: number): void {
    this.ensureCapacity(i);
    // The core operation: OR the bitmask into the word.
    //
    //     _words[3] = 0b...0000_0000
    //     mask      = 0b...0010_0000   (bit 5 within the word)
    //     result    = 0b...0010_0000   (bit 5 is now set)
    //
    // OR is idempotent: setting an already-set bit is a no-op.
    this._words[wordIndex(i)] |= bitmask(i);
  }

  /**
   * Set bit `i` to 0. No-op if `i >= len` (does not grow).
   *
   * Clearing a bit that's already 0 is a no-op. Clearing a bit beyond
   * the bitset's length is also a no-op -- there's nothing to clear,
   * because unallocated bits are conceptually zero.
   *
   * How it works:
   *
   * We AND the word with the inverted bitmask. The inverted mask has all
   * bits set EXCEPT the target bit, so every other bit is preserved:
   *
   * ```
   * _words[2] = 0b...0010_0100   (bits 2 and 5 set)
   * mask      = 0b...0010_0000   (bit 5)
   * ~mask     = 0b...1101_1111   (everything except bit 5)
   * result    = 0b...0000_0100   (bit 5 cleared, bit 2 preserved)
   * ```
   *
   * @example
   * ```ts
   * const bs = new Bitset(10);
   * bs.set(5);
   * console.log(bs.test(5)); // true
   * bs.clear(5);
   * console.log(bs.test(5)); // false
   *
   * // Clearing beyond len is a no-op:
   * bs.clear(999); // no error, no growth
   * console.log(bs.size); // 10
   * ```
   */
  clear(i: number): void {
    if (i >= this._len) {
      return; // out of range: nothing to clear
    }
    this._words[wordIndex(i)] &= ~bitmask(i);
  }

  /**
   * Test whether bit `i` is set. Returns `false` if `i >= len`.
   *
   * This is a pure read operation -- it never modifies the bitset.
   * Testing a bit beyond the bitset's length returns false because
   * unallocated bits are conceptually zero.
   *
   * How it works:
   *
   * We AND the word with the bitmask. If the result is non-zero, the
   * bit is set:
   *
   * ```
   * _words[2] = 0b...0010_0100   (bits 2 and 5 set)
   * mask      = 0b...0010_0000   (bit 5)
   * result    = 0b...0010_0000   (non-zero -> bit 5 is set)
   *
   * mask      = 0b...0000_1000   (bit 3)
   * result    = 0b...0000_0000   (zero -> bit 3 is not set)
   * ```
   *
   * @example
   * ```ts
   * const bs = new Bitset(10);
   * bs.set(5);
   * console.log(bs.test(5));   // true
   * console.log(bs.test(3));   // false
   * console.log(bs.test(999)); // false (beyond len)
   * ```
   */
  test(i: number): boolean {
    if (i >= this._len) {
      return false; // out of range: conceptually zero
    }
    return (this._words[wordIndex(i)] & bitmask(i)) !== 0;
  }

  /**
   * Toggle (flip) bit `i`. Auto-grows if `i >= len`.
   *
   * If the bit is 0, it becomes 1. If it's 1, it becomes 0.
   *
   * How it works:
   *
   * XOR with the bitmask flips exactly one bit:
   *
   * ```
   * _words[2] = 0b...0010_0100   (bits 2 and 5 set)
   * mask      = 0b...0010_0000   (bit 5)
   * result    = 0b...0000_0100   (bit 5 flipped to 0)
   * ```
   *
   * @example
   * ```ts
   * const bs = new Bitset(10);
   * bs.toggle(5);           // 0 -> 1
   * console.log(bs.test(5)); // true
   * bs.toggle(5);           // 1 -> 0
   * console.log(bs.test(5)); // false
   * ```
   */
  toggle(i: number): void {
    this.ensureCapacity(i);
    this._words[wordIndex(i)] ^= bitmask(i);

    // Toggle might have cleared a bit in the last word's trailing region
    // (if the bit was previously set by growth). Clean trailing bits to
    // maintain the invariant.
    this.cleanTrailingBits();
  }

  // ------------------------------------------------------------------
  // Bulk bitwise operations
  // ------------------------------------------------------------------
  //
  // All bulk operations return a NEW bitset. They don't modify either
  // operand. The result has len = max(a.len, b.len).
  //
  // When two bitsets have different lengths, the shorter one is
  // "zero-extended" conceptually. In practice, we just stop reading
  // from the shorter one's words once they run out and treat missing
  // words as zero.
  //
  // Performance: each operation processes one 32-bit word per loop
  // iteration, so 32 bits are handled in a single CPU instruction.
  // This is the fundamental performance advantage of bitsets.

  /**
   * Bitwise AND: result bit is 1 only if BOTH input bits are 1.
   *
   * Truth table:
   * ```
   * A  B  A&B
   * 0  0   0
   * 0  1   0
   * 1  0   0
   * 1  1   1
   * ```
   *
   * AND is used for **intersection**: elements that are in both sets.
   *
   * @returns A new bitset with len = max(this.len, other.len)
   *
   * @example
   * ```ts
   * const a = Bitset.fromInteger(0b1100); // bits 2,3
   * const b = Bitset.fromInteger(0b1010); // bits 1,3
   * const c = a.and(b);
   * console.log(c.toInteger()); // 8 (0b1000, only bit 3)
   * ```
   */
  and(other: Bitset): Bitset {
    const resultLen = Math.max(this._len, other._len);
    const maxWords = Math.max(this._words.length, other._words.length);
    const result = new Bitset(0);
    result._words = new Uint32Array(maxWords);
    result._len = resultLen;

    for (let i = 0; i < maxWords; i++) {
      // If one bitset is shorter, its missing words are zero.
      // AND with zero produces zero, which is correct.
      const a = i < this._words.length ? this._words[i] : 0;
      const b = i < other._words.length ? other._words[i] : 0;
      result._words[i] = a & b;
    }

    result.cleanTrailingBits();
    return result;
  }

  /**
   * Bitwise OR: result bit is 1 if EITHER (or both) input bits are 1.
   *
   * Truth table:
   * ```
   * A  B  A|B
   * 0  0   0
   * 0  1   1
   * 1  0   1
   * 1  1   1
   * ```
   *
   * OR is used for **union**: elements that are in either set.
   *
   * @returns A new bitset with len = max(this.len, other.len)
   *
   * @example
   * ```ts
   * const a = Bitset.fromInteger(0b1100); // bits 2,3
   * const b = Bitset.fromInteger(0b1010); // bits 1,3
   * const c = a.or(b);
   * console.log(c.toInteger()); // 14 (0b1110, bits 1,2,3)
   * ```
   */
  or(other: Bitset): Bitset {
    const resultLen = Math.max(this._len, other._len);
    const maxWords = Math.max(this._words.length, other._words.length);
    const result = new Bitset(0);
    result._words = new Uint32Array(maxWords);
    result._len = resultLen;

    for (let i = 0; i < maxWords; i++) {
      const a = i < this._words.length ? this._words[i] : 0;
      const b = i < other._words.length ? other._words[i] : 0;
      result._words[i] = a | b;
    }

    result.cleanTrailingBits();
    return result;
  }

  /**
   * Bitwise XOR: result bit is 1 if the input bits DIFFER.
   *
   * Truth table:
   * ```
   * A  B  A^B
   * 0  0   0
   * 0  1   1
   * 1  0   1
   * 1  1   0
   * ```
   *
   * XOR is used for **symmetric difference**: elements in either set
   * but not both.
   *
   * @returns A new bitset with len = max(this.len, other.len)
   *
   * @example
   * ```ts
   * const a = Bitset.fromInteger(0b1100); // bits 2,3
   * const b = Bitset.fromInteger(0b1010); // bits 1,3
   * const c = a.xor(b);
   * console.log(c.toInteger()); // 6 (0b0110, bits 1,2)
   * ```
   */
  xor(other: Bitset): Bitset {
    const resultLen = Math.max(this._len, other._len);
    const maxWords = Math.max(this._words.length, other._words.length);
    const result = new Bitset(0);
    result._words = new Uint32Array(maxWords);
    result._len = resultLen;

    for (let i = 0; i < maxWords; i++) {
      const a = i < this._words.length ? this._words[i] : 0;
      const b = i < other._words.length ? other._words[i] : 0;
      result._words[i] = a ^ b;
    }

    result.cleanTrailingBits();
    return result;
  }

  /**
   * Bitwise NOT: flip every bit within `len`.
   *
   * Truth table:
   * ```
   * A  ~A
   * 0   1
   * 1   0
   * ```
   *
   * NOT is used for **complement**: elements NOT in the set.
   *
   * **Important**: NOT flips bits within `len`, NOT within `capacity`.
   * Bits beyond `len` remain zero (clean-trailing-bits invariant).
   * The result has the same `len` as the input.
   *
   * @example
   * ```ts
   * const a = Bitset.fromInteger(0b1010); // len=4, bits 1,3 set
   * const b = a.not();
   * console.log(b.toInteger()); // 5 (0b0101, bits 0,2 set)
   * ```
   */
  not(): Bitset {
    const result = new Bitset(0);
    result._words = new Uint32Array(this._words.length);
    result._len = this._len;

    for (let i = 0; i < this._words.length; i++) {
      // The `~` operator in JavaScript returns a signed 32-bit integer.
      // `>>> 0` converts it back to an unsigned 32-bit integer, which
      // is what Uint32Array stores. However, since we're assigning to
      // a Uint32Array element, the conversion happens automatically.
      result._words[i] = ~this._words[i];
    }

    // Critical: clean trailing bits! The NOT operation flipped ALL bits
    // in every word, including the trailing bits beyond `len` that were
    // zero. We must zero them out again to maintain the invariant.
    //
    //     Before NOT: word[last] = 0b00000000_XXXXXXXX  (trailing bits are 0)
    //     After  NOT: word[last] = 0b11111111_xxxxxxxx  (trailing bits are 1!)
    //     After clean: word[last] = 0b00000000_xxxxxxxx  (trailing bits zeroed)
    result.cleanTrailingBits();
    return result;
  }

  /**
   * AND-NOT (set difference): bits in `this` that are NOT in `other`.
   *
   * This is equivalent to `this.and(other.not())`, but more efficient
   * because we don't need to create an intermediate NOT result.
   *
   * Truth table:
   * ```
   * A  B  A & ~B
   * 0  0    0
   * 0  1    0
   * 1  0    1
   * 1  1    0
   * ```
   *
   * AND-NOT is used for **set difference**: elements in A but not in B.
   *
   * @returns A new bitset with len = max(this.len, other.len)
   *
   * @example
   * ```ts
   * const a = Bitset.fromInteger(0b1110); // bits 1,2,3
   * const b = Bitset.fromInteger(0b1010); // bits 1,3
   * const c = a.andNot(b);
   * console.log(c.toInteger()); // 4 (0b0100, only bit 2)
   * ```
   */
  andNot(other: Bitset): Bitset {
    const resultLen = Math.max(this._len, other._len);
    const maxWords = Math.max(this._words.length, other._words.length);
    const result = new Bitset(0);
    result._words = new Uint32Array(maxWords);
    result._len = resultLen;

    for (let i = 0; i < maxWords; i++) {
      const a = i < this._words.length ? this._words[i] : 0;
      const b = i < other._words.length ? other._words[i] : 0;
      // a & ~b: keep bits from a that are NOT in b.
      // The `~b` produces a signed int, but `&` with unsigned `a` and
      // assignment to Uint32Array handles the conversion correctly.
      result._words[i] = a & ~b;
    }

    result.cleanTrailingBits();
    return result;
  }

  // ------------------------------------------------------------------
  // Counting and query operations
  // ------------------------------------------------------------------

  /**
   * Count the number of set (1) bits. Named after the CPU instruction
   * `POPCNT` (population count) that does this for a single word.
   *
   * How it works:
   *
   * We call our `popcount32` helper on each word and sum the results.
   * For a bitset with N bits, this runs in O(N/32) time -- we process
   * 32 bits per loop iteration.
   *
   * @example
   * ```ts
   * const bs = Bitset.fromInteger(0b10110); // bits 1,2,4 set
   * console.log(bs.popcount()); // 3
   * ```
   */
  popcount(): number {
    let count = 0;
    for (let i = 0; i < this._words.length; i++) {
      count += popcount32(this._words[i]);
    }
    return count;
  }

  /**
   * Returns the logical length: the number of addressable bits.
   *
   * This is the value passed to `new Bitset(size)`, or the highest
   * bit index + 1 after any auto-growth operations.
   */
  get size(): number {
    return this._len;
  }

  /**
   * Returns the capacity: the total allocated bits (always a multiple
   * of 32).
   *
   * Capacity >= size. The difference (capacity - size) is "slack space" --
   * bits that exist in memory but are always zero.
   */
  get capacity(): number {
    return this._words.length * BITS_PER_WORD;
  }

  /**
   * Returns `true` if at least one bit is set.
   *
   * Short-circuits: returns as soon as it finds a non-zero word,
   * without scanning the rest. This is O(1) in the best case
   * (first word is non-zero) and O(N/32) in the worst case.
   *
   * @example
   * ```ts
   * const bs = new Bitset(100);
   * console.log(bs.any()); // false
   * bs.set(50);
   * console.log(bs.any()); // true
   * ```
   */
  any(): boolean {
    for (let i = 0; i < this._words.length; i++) {
      if (this._words[i] !== 0) return true;
    }
    return false;
  }

  /**
   * Returns `true` if ALL bits in `0..len` are set.
   *
   * For an empty bitset (`len = 0`), returns `true` -- this is
   * **vacuous truth**, the same convention used by Python's `all([])`,
   * Rust's `Iterator::all`, and mathematical logic.
   *
   * How it works:
   *
   * For each full word (all except possibly the last), we check if
   * every bit is set (word === 0xFFFFFFFF, i.e., all 32 bits are 1).
   *
   * For the last word, we only check the bits within `len`. We create
   * a mask of the valid bits and check that all valid bits are set.
   *
   * @example
   * ```ts
   * const bs = new Bitset(0);
   * console.log(bs.all()); // true (vacuous truth)
   *
   * const bs2 = Bitset.fromBinaryStr("1111");
   * console.log(bs2.all()); // true
   *
   * const bs3 = Bitset.fromBinaryStr("1110");
   * console.log(bs3.all()); // false
   * ```
   */
  all(): boolean {
    // Vacuous truth: all bits of nothing are set.
    if (this._len === 0) {
      return true;
    }

    const numWords = this._words.length;

    // Check all full words (all bits must be 1 = 0xFFFFFFFF).
    for (let i = 0; i < numWords - 1; i++) {
      if (this._words[i] !== 0xffffffff) {
        return false;
      }
    }

    // Check the last word: only the bits within `len` matter.
    const remaining = this._len & OFFSET_MASK;
    if (remaining === 0) {
      // len is a multiple of 32, so the last word is a full word.
      return this._words[numWords - 1] === 0xffffffff;
    } else {
      // Create a mask for the valid bits: (1 << remaining) - 1
      // Example: remaining = 8 -> mask = 0xFF (bits 0-7)
      const mask = (1 << remaining) - 1;
      return (this._words[numWords - 1] & mask) === mask;
    }
  }

  /**
   * Returns `true` if no bits are set. Equivalent to `!this.any()`.
   *
   * @example
   * ```ts
   * const bs = new Bitset(100);
   * console.log(bs.none()); // true
   * ```
   */
  none(): boolean {
    return !this.any();
  }

  /**
   * Returns `true` if the bitset has zero length.
   */
  isEmpty(): boolean {
    return this._len === 0;
  }

  // ------------------------------------------------------------------
  // Iteration
  // ------------------------------------------------------------------

  /**
   * Iterate over the indices of all set bits in ascending order.
   *
   * This is a generator function that yields bit indices lazily, one
   * at a time. It efficiently skips zero words (if a word is zero,
   * all 32 bits in it are zero and we jump to the next word).
   *
   * Within a non-zero word, we use the trailing-zero-count trick to
   * find the lowest set bit, yield its index, then clear it:
   *
   * ```
   * word = 0b10100100   (bits 2, 5, 7 are set)
   *
   * Step 1: trailing_zeros = 2  -> yield base + 2
   *         word &= word - 1   -> 0b10100000  (clear lowest set bit)
   *
   * Step 2: trailing_zeros = 5  -> yield base + 5
   *         word &= word - 1   -> 0b10000000  (clear lowest set bit)
   *
   * Step 3: trailing_zeros = 7  -> yield base + 7
   *         word &= word - 1   -> 0b00000000  (clear lowest set bit)
   *
   * word === 0, move to next word.
   * ```
   *
   * The trick `word &= word - 1` clears the lowest set bit. Here's why:
   *
   * ```
   * word     = 0b10100100
   * word - 1 = 0b10100011  (borrow propagates through trailing zeros)
   * AND      = 0b10100000  (lowest set bit is cleared)
   * ```
   *
   * This is O(k) where k is the number of set bits, and it skips zero
   * words entirely, making it very efficient for sparse bitsets.
   *
   * @example
   * ```ts
   * const bs = Bitset.fromInteger(0b10100101);
   * const bits = [...bs.iterSetBits()];
   * console.log(bits); // [0, 2, 5, 7]
   * ```
   */
  *iterSetBits(): Generator<number, void, undefined> {
    for (let wordIdx = 0; wordIdx < this._words.length; wordIdx++) {
      let word = this._words[wordIdx];

      // Skip zero words -- they have no set bits.
      if (word === 0) continue;

      const base = wordIdx * BITS_PER_WORD;

      while (word !== 0) {
        // Find the lowest set bit using trailing zeros.
        const bitPos = ctz32(word);
        const index = base + bitPos;

        // Only yield bits within len (don't yield trailing garbage).
        if (index >= this._len) break;

        yield index;

        // Clear the lowest set bit: word &= word - 1
        //
        //     0b00101000 & 0b00100111 = 0b00100000
        //     (bit 3 is cleared, bit 5 remains)
        //
        // We use `>>> 0` to ensure the result stays as unsigned 32-bit.
        word = (word & (word - 1)) >>> 0;
      }
    }
  }

  // ------------------------------------------------------------------
  // Conversion operations
  // ------------------------------------------------------------------

  /**
   * Convert the bitset to a non-negative integer.
   *
   * Returns the integer value represented by the bitset's set bits.
   * Returns 0 for an empty bitset.
   *
   * **Overflow behavior**: If the bitset represents a value larger than
   * `Number.MAX_SAFE_INTEGER` (2^53 - 1), this method throws a
   * `BitsetError` because JavaScript Numbers cannot represent it exactly.
   *
   * How it works:
   *
   * We reconstruct the integer by combining each word, multiplied by
   * its positional weight (2^0 for word 0, 2^32 for word 1, etc.).
   * Since we're working with 32-bit words and JS Numbers are 64-bit
   * floats, we can safely combine up to about 1.6 words (53 bits).
   *
   * @example
   * ```ts
   * const bs = Bitset.fromInteger(42);
   * console.log(bs.toInteger()); // 42
   *
   * const bs2 = new Bitset(0);
   * console.log(bs2.toInteger()); // 0
   * ```
   */
  toInteger(): number {
    if (this._words.length === 0) {
      return 0;
    }

    // Check that the value fits in Number.MAX_SAFE_INTEGER (2^53 - 1).
    // With 32-bit words, word 0 covers bits 0-31, word 1 covers bits 32-63.
    // We can safely represent up to bit 52 (value 2^53 - 1).
    // So if any word beyond index 1 is non-zero, or if word 1 has bits
    // above position 20 (bit 52 overall), it overflows.
    for (let i = 2; i < this._words.length; i++) {
      if (this._words[i] !== 0) {
        throw new BitsetError(
          "bitset value exceeds Number.MAX_SAFE_INTEGER (2^53 - 1)"
        );
      }
    }

    let value = this._words[0];

    if (this._words.length > 1 && this._words[1] !== 0) {
      // word 1 contributes bits 32-63. But we can only safely use bits
      // 32-52 (21 bits of word 1). Check that bits 53+ are not set.
      // Bit 53 is bit 21 of word 1. So word 1 must be < 2^21.
      const word1 = this._words[1];
      if (word1 >= 1 << 21) {
        throw new BitsetError(
          "bitset value exceeds Number.MAX_SAFE_INTEGER (2^53 - 1)"
        );
      }
      value += word1 * 0x100000000; // word1 * 2^32
    }

    return value;
  }

  /**
   * Convert the bitset to a binary string with the highest bit on the left.
   *
   * This is the inverse of `fromBinaryStr`. An empty bitset produces
   * an empty string `""`.
   *
   * @example
   * ```ts
   * const bs = Bitset.fromInteger(5); // binary 101
   * console.log(bs.toBinaryStr()); // "101"
   *
   * const bs2 = new Bitset(0);
   * console.log(bs2.toBinaryStr()); // ""
   * ```
   */
  toBinaryStr(): string {
    if (this._len === 0) {
      return "";
    }

    // Build the string from the highest bit (len-1) down to bit 0.
    // This produces conventional binary notation: MSB on the left.
    const chars: string[] = [];
    for (let i = this._len - 1; i >= 0; i--) {
      chars.push(this.test(i) ? "1" : "0");
    }
    return chars.join("");
  }

  /**
   * Human-readable debug representation like "Bitset(101)".
   *
   * @example
   * ```ts
   * const bs = Bitset.fromInteger(5);
   * console.log(bs.toString()); // "Bitset(101)"
   *
   * const bs2 = new Bitset(0);
   * console.log(bs2.toString()); // "Bitset()"
   * ```
   */
  toString(): string {
    return `Bitset(${this.toBinaryStr()})`;
  }

  // ------------------------------------------------------------------
  // Equality
  // ------------------------------------------------------------------

  /**
   * Check whether two bitsets are equal.
   *
   * Two bitsets are equal if and only if they have the same `len` and
   * the same bits set. Capacity is irrelevant to equality -- a bitset
   * with `capacity = 64` can equal one with `capacity = 128` if their
   * `len` and set bits match.
   *
   * Thanks to the clean-trailing-bits invariant, we can compare words
   * directly -- trailing bits are always zero, so two bitsets with the
   * same logical content will have identical word vectors (up to the
   * number of words needed for the longer one).
   *
   * @example
   * ```ts
   * const a = Bitset.fromInteger(5);
   * const b = Bitset.fromBinaryStr("101");
   * console.log(a.equals(b)); // true
   * ```
   */
  equals(other: Bitset): boolean {
    if (this._len !== other._len) {
      return false;
    }

    // Compare word-by-word. If one has more words allocated, the
    // extra words must all be zero (due to clean-trailing-bits).
    const maxWords = Math.max(this._words.length, other._words.length);
    for (let i = 0; i < maxWords; i++) {
      const a = i < this._words.length ? this._words[i] : 0;
      const b = i < other._words.length ? other._words[i] : 0;
      if (a !== b) {
        return false;
      }
    }
    return true;
  }

  // ------------------------------------------------------------------
  // Internal helpers
  // ------------------------------------------------------------------

  /**
   * Ensure the bitset has capacity for bit `i`. If not, grow by doubling.
   *
   * After this call, `i < capacity` and `_len >= i + 1`.
   *
   * Growth strategy:
   *
   * We double the capacity repeatedly until it exceeds `i`. The minimum
   * capacity after growth is 32 (one word). This doubling strategy gives
   * amortized O(1) growth, just like ArrayList and Python's list.
   *
   * ```
   * Example: capacity=64, set(200)
   *   64 -> 128 -> 256  (stop: 200 < 256)
   * ```
   *
   * Because Uint32Array has a fixed size, growing requires allocating a
   * new array and copying the old data. This is the same approach used
   * by Java's ArrayList.
   */
  private ensureCapacity(i: number): void {
    if (i < this.capacity) {
      // Already have room. But we might need to update _len.
      if (i >= this._len) {
        this._len = i + 1;
      }
      return;
    }

    // Need to grow. Start with current capacity (or 32 as minimum).
    let newCap = Math.max(this.capacity, BITS_PER_WORD);
    while (newCap <= i) {
      newCap *= 2;
    }

    // Allocate a new Uint32Array with the new size.
    const newWordCount = newCap / BITS_PER_WORD;
    const newWords = new Uint32Array(newWordCount);

    // Copy the old data.
    newWords.set(this._words);

    this._words = newWords;

    // Update len to include the new bit.
    this._len = i + 1;
  }

  /**
   * Zero out any bits beyond `_len` in the last word.
   *
   * This maintains the clean-trailing-bits invariant. It must be called
   * after any operation that might set bits beyond `_len`:
   *   - not() flips all bits, including trailing ones
   *   - toggle() on the last word
   *   - bulk operations (AND, OR, XOR) when operands have different sizes
   *
   * How it works:
   *
   * ```
   * len = 100, capacity = 128
   * The last word holds bits 96-127, but only bits 96-99 are "real".
   * remaining = 100 % 32 = 4
   * mask = (1 << 4) - 1 = 0xF  (bits 0-3)
   * _words[3] &= 0xF  -> zeroes out bits 4-31 of word 3
   * ```
   *
   * If `_len` is a multiple of 32, there are no trailing bits to clean.
   */
  private cleanTrailingBits(): void {
    if (this._len === 0 || this._words.length === 0) {
      return;
    }

    const remaining = this._len & OFFSET_MASK;
    if (remaining !== 0) {
      const lastIdx = this._words.length - 1;
      const mask = (1 << remaining) - 1;
      this._words[lastIdx] &= mask;
    }
  }
}
