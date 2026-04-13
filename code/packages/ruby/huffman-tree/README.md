# coding-adventures-huffman-tree

**DT27** — Huffman Tree data structure for the `coding-adventures` monorepo.

## What is this?

A Huffman tree is a full binary tree built from a symbol alphabet so that each
symbol gets a unique variable-length bit code.  Symbols that appear often get
short codes; symbols that appear rarely get long codes.  The total bits needed
to encode a message is minimised — it is the theoretically optimal prefix-free
code for a given symbol frequency distribution.

This package implements the tree data structure.  The companion CMP04 Huffman
compression package uses it to compress and decompress byte streams.

## Where does it fit?

```
CMP00 (LZ77,    1977) — Sliding-window backreferences
CMP01 (LZ78,    1978) — Explicit dictionary (trie)
CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals
CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; GIF
CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE  ← uses DT27
CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib
```

The Huffman tree (DT27) is a dependency of CMP04 and through it of CMP05
(DEFLATE).

## Algorithm

Greedy construction via a min-heap (from the `coding_adventures_heap` package):

1. Create one leaf node per distinct symbol, weighted by frequency.
2. While there are more than one node on the heap:
   a. Pop the two lowest-weight nodes.
   b. Combine them into a new internal node (weight = sum).
   c. Push the new internal node back.
3. The remaining node is the root.

### Tie-breaking (for deterministic output)

| Priority | Rule |
|----------|------|
| 1st | Lower weight pops first |
| 2nd | Leaves before internal nodes at equal weight |
| 3rd | Lower symbol value among equal-weight leaves |
| 4th | Earlier-created internal node (FIFO) among equal-weight internals |

### Canonical codes (DEFLATE style)

`canonical_code_table` produces DEFLATE-compatible codes: given only code
*lengths*, the same code table can be reconstructed anywhere without
transmitting the tree structure.

## Usage

```ruby
require "coding_adventures_huffman_tree"

# Build from (symbol, frequency) pairs
tree = CodingAdventures::HuffmanTree.build([[65, 3], [66, 2], [67, 1]])

# Inspect
tree.symbol_count   # => 3
tree.weight         # => 6   (total frequency = 3+2+1)
tree.depth          # => 2   (max code length)

# Encode
table = tree.code_table          # => {65=>"0", 66=>"10", 67=>"11"}
bits  = [65, 65, 66, 67].map { |s| table[s] }.join  # => "010011"

# Decode
tree.decode_all("010011", 4)     # => [65, 65, 66, 67]

# Canonical codes (DEFLATE style)
canon = tree.canonical_code_table  # => {65=>"0", 66=>"10", 67=>"11"}

# Inspection
tree.leaves     # => [[65, "0"], [66, "10"], [67, "11"]]  (in-order)
tree.valid?     # => true
```

## Testing

```sh
bundle install
bundle exec rspec
```

## Dependencies

- `coding_adventures_heap` — min-heap used for greedy tree construction
