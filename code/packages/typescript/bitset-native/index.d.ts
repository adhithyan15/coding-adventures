// index.d.ts -- TypeScript type definitions for @coding-adventures/bitset-native
// ================================================================================
//
// These type definitions describe the native Bitset class exposed by the
// Rust addon. The actual implementation is in src/lib.rs; these types exist
// so TypeScript consumers get full IntelliSense and type checking.

/**
 * A compact bitset that packs boolean values into 64-bit machine words.
 *
 * Wraps the Rust `bitset::Bitset` crate via N-API for native performance.
 * All bitwise operations (AND, OR, XOR, NOT) operate on 64 bits at a time,
 * making them dramatically faster than equivalent JavaScript boolean arrays.
 *
 * ## Construction
 *
 * ```typescript
 * // Create a zero-filled bitset with 100 addressable bits
 * const bs = new Bitset(100);
 *
 * // Create from an integer (bit 0 = LSB)
 * const bs2 = new Bitset(42, "integer");
 *
 * // Create from a binary string (leftmost = highest bit)
 * const bs3 = new Bitset("1010", "binary");
 * ```
 */
export class Bitset {
  /**
   * Create a new bitset with `size` bits, all initially zero.
   * Capacity is rounded up to the next multiple of 64.
   */
  constructor(size: number);

  /**
   * Create a bitset from an integer value or binary string.
   *
   * @param value - The integer value or binary string
   * @param mode - "integer" to interpret value as a number, "binary" to
   *               interpret as a string of '0' and '1' characters
   */
  constructor(value: number | string, mode: "integer" | "binary");

  // -- Single-bit operations -----------------------------------------------

  /** Set bit `i` to 1. Auto-grows the bitset if `i >= len`. */
  set(i: number): void;

  /** Set bit `i` to 0. No-op if `i >= len`. */
  clear(i: number): void;

  /** Returns true if bit `i` is set. Returns false if `i >= len`. */
  test(i: number): boolean;

  /** Flip bit `i`. Auto-grows the bitset if `i >= len`. */
  toggle(i: number): void;

  // -- Bulk bitwise operations ---------------------------------------------

  /** Returns a new bitset = this AND other (intersection). */
  and(other: Bitset): Bitset;

  /** Returns a new bitset = this OR other (union). */
  or(other: Bitset): Bitset;

  /** Returns a new bitset = this XOR other (symmetric difference). */
  xor(other: Bitset): Bitset;

  /** Returns a new bitset with all bits flipped (within len). */
  not(): Bitset;

  /** Returns a new bitset = this AND (NOT other) (difference). */
  andNot(other: Bitset): Bitset;

  // -- Query operations ----------------------------------------------------

  /** Returns the number of set bits (population count / Hamming weight). */
  popcount(): number;

  /** Returns the logical length: the number of addressable bits. */
  len(): number;

  /** Returns the allocated capacity in bits (always a multiple of 64). */
  capacity(): number;

  /** Returns true if at least one bit is set. */
  any(): boolean;

  /** Returns true if ALL bits in 0..len are set. */
  all(): boolean;

  /** Returns true if no bits are set. */
  none(): boolean;

  /** Returns true if len is 0 (the bitset has no addressable bits). */
  isEmpty(): boolean;

  // -- Iteration and conversion --------------------------------------------

  /**
   * Returns an array of indices where bits are set, in ascending order.
   *
   * ```typescript
   * const bs = new Bitset(42, "integer"); // binary: 101010
   * bs.iterSetBits(); // [1, 3, 5]
   * ```
   */
  iterSetBits(): number[];

  /**
   * Convert the bitset to a number, or null if it doesn't fit in 64 bits.
   *
   * Returns 0 for an empty bitset.
   */
  toInteger(): number | null;

  /**
   * Convert the bitset to a binary string with the highest bit on the left.
   *
   * ```typescript
   * const bs = new Bitset(5, "integer"); // binary: 101
   * bs.toBinaryStr(); // "101"
   * ```
   */
  toBinaryStr(): string;
}
