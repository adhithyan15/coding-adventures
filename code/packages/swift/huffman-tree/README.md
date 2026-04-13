# HuffmanTree (Swift)

DT27: Huffman Tree — Optimal prefix-free entropy coding.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
educational computing stack.

## What Is a Huffman Tree?

A Huffman tree is a full binary tree (every internal node has exactly two
children) built from a symbol alphabet so that each symbol gets a unique
variable-length bit code. Symbols that appear often get short codes; symbols
that appear rarely get long codes. The total bits needed to encode a message is
minimised — it is the theoretically optimal prefix-free code for a given symbol
frequency distribution.

## Installation

Add to your `Package.swift`:

```swift
.package(url: "https://github.com/adhithyan15/coding-adventures.git", from: "0.1.0")
```

This package depends on the standalone `Heap` package for the shared min-heap
implementation used during tree construction.

## Usage

```swift
import HuffmanTree

// Build a tree from (symbol, frequency) pairs.
let tree = try HuffmanTree.build([
    (symbol: 65, frequency: 3),   // 'A' appears 3 times
    (symbol: 66, frequency: 2),   // 'B' appears 2 times
    (symbol: 67, frequency: 1),   // 'C' appears 1 time
])

// Get the code table: [symbol: bitString]
let table = tree.codeTable()
// table[65] == "0"   (A gets the shortest code)
// table[67] == "10"  (C)
// table[66] == "11"  (B)

// Encode a message
let message = [65, 65, 66, 67]   // AABC
let bits = message.map { table[$0]! }.joined()
// bits == "001110"

// Decode
let decoded = try tree.decodeAll(bits, count: 4)
// decoded == [65, 65, 66, 67]

// Canonical codes (DEFLATE-style)
let canon = tree.canonicalCodeTable()

// Inspection
print(tree.weight)        // 6
print(tree.depth)         // 2
print(tree.symbolCount)   // 3

// In-order leaf traversal
for (symbol, code) in tree.leaves() {
    print("\(symbol) => \(code)")
}

// Validity check
print(tree.isValid())  // true
```

## API

| Method / Property | Description |
|---|---|
| `HuffmanTree.build(_ weights:)` | Build tree; throws on empty/invalid input |
| `codeTable()` | Returns `[Int: String]` code dictionary |
| `codeFor(_ symbol:)` | Returns code for one symbol, or `nil` |
| `canonicalCodeTable()` | Returns DEFLATE-style canonical codes |
| `decodeAll(_ bits:count:)` | Decode `count` symbols; throws on exhaustion |
| `weight` | Total weight (sum of all frequencies) |
| `depth` | Maximum code length |
| `symbolCount` | Number of distinct symbols |
| `leaves()` | In-order leaf list: `[(symbol, code)]` |
| `isValid()` | Check structural invariants |

## Running Tests

```bash
swift test --enable-code-coverage --verbose
```

## License

MIT
