# huffman-tree — DT27

Go implementation of the Huffman Tree data structure (DT27).

A Huffman tree is a full binary tree built from symbol frequencies that assigns optimal prefix-free bit codes: frequent symbols get short codes, rare symbols get long codes.  It is the theoretical basis for DEFLATE, zlib, and PNG compression.

## Usage

```go
import huffmantree "github.com/adhithyan15/coding-adventures/code/packages/go/huffman-tree"

// Build from (symbol, frequency) pairs.
tree, err := huffmantree.Build([]huffmantree.WeightPair{
    {Symbol: 65, Frequency: 3}, // 'A'
    {Symbol: 66, Frequency: 2}, // 'B'
    {Symbol: 67, Frequency: 1}, // 'C'
})

// Encode: get code table.
table := huffmantree.CodeTable(tree)
// table[65] == "0", table[66] == "10", table[67] == "11"

// Decode bit stream.
symbols, _ := huffmantree.DecodeAll(tree, "010011", 4)
// symbols == [65, 65, 66, 67]

// DEFLATE-style canonical codes.
canonical := huffmantree.CanonicalCodeTable(tree)

// Inspection.
huffmantree.Weight(tree)      // 6
huffmantree.Depth(tree)       // 2
huffmantree.SymbolCount(tree) // 3
huffmantree.IsValid(tree)     // true
```

## Algorithm

Greedy min-heap construction with deterministic tie-breaking:
1. Lowest weight pops first.
2. Leaves before internal nodes at equal weight.
3. Lower symbol value wins among equal-weight leaves.
4. Earlier-created internal node wins among equal-weight internals (FIFO).

## Package in the stack

- Used by: `CMP04 huffman` compression algorithm.
- Depends on: `DT?? heap` (min-heap).
