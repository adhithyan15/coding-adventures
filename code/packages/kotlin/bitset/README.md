# coding_adventures_bitset (Kotlin)

A compact boolean array packed into 64-bit words, implemented in Kotlin.

See `code/specs/bitset.md` for the full specification.

## What is a Bitset?

A bitset stores a sequence of bits (each 0 or 1) packed into 64-bit `Long`
words. Instead of using an entire byte per boolean, a bitset packs 64 of them
into a single word.

| Representation  | 10,000 booleans | Notes                           |
|-----------------|-----------------|----------------------------------|
| `BooleanArray`  | ~10,000 bytes   | 1 byte per entry (JVM)          |
| `Bitset`        | ~1,250 bytes    | 8× space saving                 |

## Bit Ordering: LSB-First

Bit 0 is the least significant bit of word 0. Bit 64 is the least significant
bit of word 1.

```
Word 0                              Word 1
┌─────────────────────────────┐     ┌─────────────────────────────┐
│ bit 63  ...  bit 2  bit 1  bit 0│ │ bit 127 ... bit 65  bit 64 │
└─────────────────────────────┘     └─────────────────────────────┘
MSB ◄─────────────────── LSB        MSB ◄─────────────────── LSB
```

## API

```kotlin
// Construction
val bs = Bitset(100)                    // 100 zero bits
val bs = Bitset.fromInteger(5L)         // bits 0 and 2 set, length=3
val bs = Bitset.fromBinaryStr("101")    // same as above

// Single-bit operations
bs.set(i)           // set bit i to 1; auto-grows if i >= length()
bs.clear(i)         // set bit i to 0; no-op if i >= length()
val b = bs.test(i)  // is bit i set? false if i >= length()
bs.toggle(i)        // flip bit i; auto-grows if i >= length()

// Bulk bitwise — each returns a NEW bitset
val c = a.and(b)     // intersection
val c = a.or(b)      // union
val c = a.xor(b)     // symmetric difference
val c = a.not()      // complement (within length())
val c = a.andNot(b)  // set difference (a & ~b)

// Counting and query
val n = bs.popcount()  // number of set bits
val n = bs.length()    // logical size (addressable bits)
val n = bs.capacity()  // allocated size (multiple of 64)
val b = bs.any()       // at least one bit set?
val b = bs.all()       // all bits in [0, length()) set?
val b = bs.none()      // no bits set?

// Iteration
val indices = bs.iterSetBits()  // List<Int> of set bit indices, ascending

// Conversion
val v = bs.toInteger()     // Long (unsigned); throws if > 64 bits
val s = bs.toBinaryStr()   // "101" (MSB on left)
```

## Usage

```kotlin
import com.codingadventures.bitset.Bitset

// Sieve of Eratosthenes: mark composite numbers
val composites = Bitset(100)
for (p in 2 until 10) {
    if (!composites.test(p)) { // p is prime
        var mult = p * p
        while (mult < 100) {
            composites.set(mult)
            mult += p
        }
    }
}

// Bitwise set intersection
val setA = Bitset.fromBinaryStr("11001010")
val setB = Bitset.fromBinaryStr("10110110")
val intersection = setA.and(setB)
```

## Clean-Trailing-Bits Invariant

Bits beyond `length()` in the last word are **always zero**. This makes
`popcount()`, `any()`, `all()`, `none()`, `equals()`, and `toInteger()`
correct without special-casing the partial last word.

Example: `Bitset.fromBinaryStr("101").not()` gives `"010"` (1 bit set), not
63 bits set — because trailing bits are cleaned after `not()`.

## Kotlin Note

Internal fields are named `_words` and `_size` to avoid JVM signature clashes
with the public API methods `length()` and the word-array usage. This is a
Kotlin-specific pattern when you need both a private backing field and a public
method with a conceptually similar name.

## Development

```bash
cd code/packages/kotlin/bitset
gradle test
```

Or use the repo build tool:

```bash
bash BUILD
```
