# coding_adventures_bitset (Java)

A compact boolean array packed into 64-bit words, implemented in Java 21.

See `code/specs/bitset.md` for the full specification.

## What is a Bitset?

A bitset stores a sequence of bits (each 0 or 1) packed into 64-bit `long`
words. Instead of using an entire byte per boolean, a bitset packs 64 of them
into a single word.

| Representation  | 10,000 booleans | Notes                        |
|-----------------|-----------------|------------------------------|
| `boolean[]`     | ~10,000 bytes   | 1 byte per entry (JVM)       |
| `Bitset`        | ~1,250 bytes    | 8× space saving              |
| `java.util.BitSet` | ~1,250 bytes | Standard library equivalent  |

Speed advantage: AND-ing two `boolean[]` of 10,000 elements requires 10,000
iterations. AND-ing two `Bitset` objects requires ~157 iterations (one 64-bit
AND per word).

## Bit Ordering: LSB-First

Bit 0 is the least significant bit of word 0. Bit 63 is the most significant
bit of word 0. Bit 64 is the least significant bit of word 1.

```
Word 0                              Word 1
┌─────────────────────────────┐     ┌─────────────────────────────┐
│ bit 63  ...  bit 2  bit 1  bit 0│ │ bit 127 ... bit 65  bit 64 │
└─────────────────────────────┘     └─────────────────────────────┘
MSB ◄─────────────────── LSB        MSB ◄─────────────────── LSB
```

## API

```java
// Construction
Bitset bs = new Bitset(100);          // 100 zero bits
Bitset bs = Bitset.fromInteger(5L);   // bits 0 and 2 set, length=3
Bitset bs = Bitset.fromBinaryStr("101"); // same as above

// Single-bit operations
bs.set(i);         // set bit i to 1; auto-grows if i >= length
bs.clear(i);       // set bit i to 0; no-op if i >= length
boolean b = bs.test(i);   // is bit i set? returns false if i >= length
bs.toggle(i);      // flip bit i; auto-grows if i >= length

// Bulk bitwise — each returns a NEW bitset
Bitset c = a.and(b);      // intersection
Bitset c = a.or(b);       // union
Bitset c = a.xor(b);      // symmetric difference
Bitset c = a.not();        // complement (within length)
Bitset c = a.andNot(b);   // set difference (a & ~b)

// Counting and query
int n = bs.popcount();    // number of set bits
int n = bs.length();      // logical size (addressable bits)
int n = bs.capacity();    // allocated size (multiple of 64)
boolean b = bs.any();     // at least one bit set?
boolean b = bs.all();     // all bits in [0,length) set?
boolean b = bs.none();    // no bits set?

// Iteration
List<Integer> indices = bs.iterSetBits();  // indices of set bits, ascending

// Conversion
long v = bs.toInteger();           // unsigned 64-bit value; throws if > 64 bits
String s = bs.toBinaryStr();       // "101" (MSB on left)
```

## Usage

```java
import com.codingadventures.bitset.Bitset;

// Sieve of Eratosthenes: mark composite numbers
Bitset composites = new Bitset(100);
for (int p = 2; p < 10; p++) {
    if (!composites.test(p)) { // p is prime
        for (int mult = p * p; mult < 100; mult += p) {
            composites.set(mult);
        }
    }
}

// Graph visited-node tracking
Bitset visited = new Bitset(numNodes);
visited.set(startNode);

// Bitwise set intersection
Bitset setA = Bitset.fromBinaryStr("11001010");
Bitset setB = Bitset.fromBinaryStr("10110110");
Bitset intersection = setA.and(setB);
```

## Clean-Trailing-Bits Invariant

Bits beyond `length` in the last word are **always zero**. This makes
`popcount()`, `any()`, `all()`, `none()`, `equals()`, and `toInteger()` correct
without special-casing the partial last word in most operations.

Example: length=3, capacity=64. The last word has bits 0–63 allocated, but
only bits 0–2 are logical. Bits 3–63 are always zero. `not()` correctly
produces a bitset with only bit 1 set (for input "101"), not 63 bits set.

## Development

```bash
cd code/packages/java/bitset
gradle test
```

Or use the repo build tool:

```bash
bash BUILD
```
