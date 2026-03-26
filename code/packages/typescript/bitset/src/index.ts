/**
 * Bitset Library
 * ==============
 *
 * A compact bitset data structure that packs boolean values into 32-bit
 * words using `Uint32Array`. Provides O(n/32) bulk bitwise operations
 * (AND, OR, XOR, NOT), efficient iteration over set bits using
 * trailing-zero-count, and ArrayList-style automatic growth when you
 * set bits beyond the current size.
 *
 * Quick start:
 *
 *     import { Bitset } from "@coding-adventures/bitset";
 *
 *     const bs = new Bitset(100);
 *     bs.set(0);
 *     bs.set(42);
 *     bs.set(99);
 *
 *     console.log(bs.popcount());        // 3
 *     console.log([...bs.iterSetBits()]); // [0, 42, 99]
 *
 *     const other = Bitset.fromInteger(42);
 *     const intersection = bs.and(other);
 *
 * Error classes are available at the top level too:
 *
 *     import { BitsetError } from "@coding-adventures/bitset";
 */

export { Bitset, BitsetError } from "./bitset.js";
