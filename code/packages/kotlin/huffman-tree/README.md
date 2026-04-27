# kotlin/huffman-tree

A **Huffman tree** in idiomatic Kotlin — the theoretically optimal prefix-free
code for any symbol frequency distribution.  O(n log n) construction, O(n)
encoding/decoding.

## What is a Huffman Tree?

A Huffman tree assigns variable-length bit codes to symbols so that frequent
symbols get short codes and rare symbols get long codes.  The resulting codes are
**prefix-free**: no code is a prefix of another, so the bit stream can be decoded
unambiguously without separator characters.

Think of Morse code: 'E' is `.` (one dot) and 'Z' is `--..` (four symbols).
Huffman's algorithm builds this mapping automatically and optimally for any
given frequency distribution.

## Usage

```kotlin
import com.codingadventures.huffmantree.HuffmanTree

// Build from (symbol, frequency) pairs
val tree = HuffmanTree.build(listOf(
    intArrayOf(65, 3),  // 'A' → frequency 3
    intArrayOf(66, 2),  // 'B' → frequency 2
    intArrayOf(67, 1)   // 'C' → frequency 1
))

// Encode
val table = tree.codeTable()
// table[65] → "0"   (A gets the shortest code)
// table[66] → "10"
// table[67] → "11"

val bits = table.getValue(65) + table.getValue(66) + table.getValue(67) // "01011"

// Decode
val decoded = tree.decodeAll(bits, 3)  // → [65, 66, 67]

// Canonical codes (DEFLATE-style)
val canonical = tree.canonicalCodeTable()
// canonical[65] → "0"
// canonical[66] → "10"
// canonical[67] → "11"

// Inspection
tree.weight()       // 6    (sum of all frequencies)
tree.depth()        // 2    (max code length)
tree.symbolCount()  // 3
tree.isValid()      // true
```

## API

| Member | Description |
|---|---|
| `HuffmanTree.build(List<IntArray>)` | Build from `[symbol, freq]` pairs |
| `fun codeTable(): Map<Int, String>` | O(n) — all codes as bit strings |
| `fun codeFor(symbol: Int): String?` | O(n) — single symbol lookup |
| `fun canonicalCodeTable(): Map<Int, String>` | O(n log n) — DEFLATE-style canonical codes |
| `fun decodeAll(bits: String, count: Int): List<Int>` | Decode exactly count symbols |
| `fun weight(): Int` | O(1) — sum of all frequencies |
| `fun depth(): Int` | O(n) — maximum code length |
| `fun symbolCount(): Int` | O(1) — number of distinct symbols |
| `fun leavesWithCodes(): List<Pair<Int, String>>` | O(n) — in-order leaf pairs |
| `fun isValid(): Boolean` | Structural validator |

## Design

Uses a **sealed class** hierarchy:
- `sealed class Node(weight: Int)` — base
- `data class Leaf(symbol, weight)` — leaf with symbol
- `data class Internal(weight, left, right, order)` — internal branching node

Pattern-matching `when` expressions in all tree traversals keep code concise
and exhaustive.

## Running tests

```
gradle test
```

35 tests covering: construction, code table, codeFor, canonical codes,
encode/decode round-trips (including all 256 bytes), exhaustion error,
inspection methods (weight, depth, symbolCount, leavesWithCodes),
tie-breaking determinism, and isValid.
