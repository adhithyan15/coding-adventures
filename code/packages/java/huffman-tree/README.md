# java/huffman-tree

A **Huffman tree** in Java — the theoretically optimal prefix-free code for any
symbol frequency distribution.  O(n log n) construction, O(n) encoding/decoding.

## What is a Huffman Tree?

A Huffman tree assigns variable-length bit codes to symbols so that frequent
symbols get short codes and rare symbols get long codes.  The resulting codes are
**prefix-free**: no code is a prefix of another, so the bit stream can be decoded
unambiguously without separator characters.

Think of Morse code: 'E' is `.` (one dot) and 'Z' is `--..` (four symbols).
Huffman's algorithm builds this mapping automatically and optimally for any
given frequency distribution.

## Usage

```java
import com.codingadventures.huffmantree.HuffmanTree;

// Build from (symbol, frequency) pairs
HuffmanTree tree = HuffmanTree.build(List.of(
    new int[]{65, 3},  // 'A' → frequency 3
    new int[]{66, 2},  // 'B' → frequency 2
    new int[]{67, 1}   // 'C' → frequency 1
));

// Encode
Map<Integer, String> table = tree.codeTable();
// table.get(65) → "0"   (A gets the shortest code)
// table.get(66) → "10"
// table.get(67) → "11"

String bits = table.get(65) + table.get(66) + table.get(67); // "01011"

// Decode
List<Integer> decoded = tree.decodeAll(bits, 3);  // → [65, 66, 67]

// Canonical codes (DEFLATE-style)
Map<Integer, String> canonical = tree.canonicalCodeTable();
// canonical.get(65) → "0"
// canonical.get(66) → "10"
// canonical.get(67) → "11"

// Inspection
tree.weight();       // 6    (sum of all frequencies)
tree.depth();        // 2    (max code length)
tree.symbolCount();  // 3
tree.isValid();      // true
```

## API

| Method | Description |
|---|---|
| `HuffmanTree.build(List<int[]> weights)` | Build from `[symbol, freq]` pairs |
| `Map<Integer,String> codeTable()` | O(n) — all codes as bit strings |
| `String codeFor(int symbol)` | O(n) — single symbol lookup (null if absent) |
| `Map<Integer,String> canonicalCodeTable()` | O(n log n) — DEFLATE-style canonical codes |
| `List<Integer> decodeAll(String bits, int count)` | Decode exactly count symbols |
| `int weight()` | O(1) — sum of all frequencies |
| `int depth()` | O(n) — maximum code length |
| `int symbolCount()` | O(1) — number of distinct symbols |
| `List<Object[]> leavesWithCodes()` | O(n) — in-order `[symbol, code]` pairs |
| `boolean isValid()` | Structural validator |

## Algorithm

1. Push all symbols as leaf nodes into a min-heap, keyed by (weight, isInternal, symbol, order)
2. While heap has > 1 node: pop two lightest, merge into an internal node, push back
3. The last node is the root

Tie-breaking for determinism:
- Leaf nodes beat internal nodes at equal weight
- Lower symbol value wins among leaves of equal weight
- Earlier-created internal node wins among internals of equal weight (FIFO)

## Running tests

```
gradle test
```

36 tests covering: construction, code table, codeFor, canonical codes,
encode/decode round-trips (including all 256 bytes), exhaustion error,
inspection methods (weight, depth, symbolCount, leavesWithCodes),
tie-breaking determinism, and isValid.
