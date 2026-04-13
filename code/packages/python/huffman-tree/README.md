# huffman-tree — DT27

Huffman tree data structure: optimal prefix-free codes via greedy min-heap construction.

## What it does

A Huffman tree is a full binary tree that assigns variable-length bit codes to symbols.
Symbols with higher frequencies receive shorter codes; rare symbols receive longer codes.
The result is the theoretically optimal prefix-free code for any symbol distribution.

This package implements the tree data structure (DT27). The compression algorithm that
uses it is `coding-adventures-huffman` (CMP04).

## Layer position

```
DT04: heap            ← used during construction
DT27: huffman-tree    ← [YOU ARE HERE]
  └── used by: CMP04 (Huffman compression)
```

## Usage

```python
from coding_adventures_huffman_tree import HuffmanTree

# Build from (symbol, frequency) pairs
tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])  # A=3, B=2, C=1

# Get code table: {symbol → bit_string}
table = tree.code_table()
# {65: '0', 66: '10', 67: '11'}

# Encode
bits = "".join(table[s] for s in [65, 65, 66, 67])
# "001011"

# Decode
symbols = tree.decode_all(bits, 4)
# [65, 65, 66, 67]

# Canonical codes (DEFLATE-style, from lengths only)
canonical = tree.canonical_code_table()
# {65: '0', 66: '10', 67: '11'}
```

## Inspection

```python
tree.weight()        # 6 (sum of all frequencies)
tree.depth()         # 2 (max code length)
tree.symbol_count()  # 3 (number of distinct symbols)
tree.leaves()        # [(65, '0'), (66, '10'), (67, '11')]
tree.is_valid()      # True
```

## Algorithm

Greedy construction via min-heap:

1. Create one leaf per symbol with its frequency as weight.
2. While heap has >1 node: pop two lowest-weight nodes, create internal node
   with combined weight, push back.
3. The remaining node is the root.

Tie-breaking for determinism:
1. Lowest weight first.
2. Leaves before internal nodes at equal weight.
3. Lower symbol value among equal-weight leaves.
4. Earlier-created internal node among equal-weight internal nodes (FIFO).

## Dependencies

- `coding-adventures-heap` (DT04) — min-heap used during construction
