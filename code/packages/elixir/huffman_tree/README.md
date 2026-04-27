# huffman_tree — Huffman Tree Data Structure (Elixir)

Huffman tree (DT27) — a full binary tree that assigns optimal prefix-free
bit codes to symbols based on their frequency. Used by Huffman Coding (CMP04)
to achieve entropy-optimal compression. Part of the data-structures and
compression series in the coding-adventures monorepo.

## In the Series

| Spec  | Package          | Description                                          |
|-------|------------------|------------------------------------------------------|
| DT27  | **huffman_tree** | Huffman tree construction and encoding ← you are here |
| CMP04 | huffman          | Huffman Coding compression using DT27                |
| CMP05 | deflate          | DEFLATE = LZ77 + Huffman; ZIP/gzip/PNG/zlib          |

## What Is a Huffman Tree?

A Huffman tree is a full binary tree (every internal node has exactly two
children) built from a symbol alphabet so that each symbol gets a unique
variable-length bit code. Symbols that appear often get short codes; symbols
that appear rarely get long codes.

Think of it like Morse code: `E` is `.` (one dot) because it's the most common
letter in English. Huffman's algorithm does this automatically and optimally
for any alphabet with any frequency distribution.

## Algorithm

1. Create one leaf per distinct symbol, each with its frequency as weight.
2. Repeatedly pop the two lightest nodes from a min-heap, combine them into
   an internal node whose weight = sum of children, and push the result back.
3. The single remaining node is the root.

Tie-breaking rules for deterministic output:
1. Lowest weight pops first.
2. Leaf nodes have higher priority than internal nodes at equal weight.
3. Among leaves of equal weight, lower symbol value wins.
4. Among internal nodes of equal weight, earlier-created node wins (FIFO).

## Usage

```elixir
alias CodingAdventures.HuffmanTree

# Build a tree from symbol frequencies (A=3, B=2, C=1 — the "AAABBC" example)
tree = HuffmanTree.build([{65, 3}, {66, 2}, {67, 1}])

# Get the full code table
HuffmanTree.code_table(tree)
# => %{65 => "0", 66 => "10", 67 => "11"}

# Look up a single symbol's code
HuffmanTree.code_for(tree, 65)   # => "0"
HuffmanTree.code_for(tree, 99)   # => nil  (not in tree)

# Get canonical (DEFLATE-style) codes — only lengths need to be transmitted
HuffmanTree.canonical_code_table(tree)
# => %{65 => "0", 66 => "10", 67 => "11"}

# Decode a bit string (supply the exact number of symbols expected)
HuffmanTree.decode_all(tree, "001011", 4)
# => [65, 65, 66, 67]   (A, A, B, C)

# Tree inspection
HuffmanTree.weight(tree)        # => 6  (sum of all frequencies)
HuffmanTree.depth(tree)         # => 2  (max code length)
HuffmanTree.symbol_count(tree)  # => 3
HuffmanTree.leaves(tree)        # => [{65, "0"}, {66, "10"}, {67, "11"}]
HuffmanTree.is_valid(tree)      # => true
```

## Prefix-Free Property

In a Huffman tree, symbols live only at the leaves. The code for each symbol
is the path from root to leaf (left edge = `"0"`, right edge = `"1"`). Since
no leaf is an ancestor of another, no code is a prefix of another — so the
bit stream can be decoded without separators just by walking the tree.

## Canonical Codes

`canonical_code_table/1` returns DEFLATE-style canonical codes: given the code
lengths only, the exact same table can be reconstructed anywhere. The compressed
stream can transmit only the length table, not the tree structure, saving space.

## Dependencies

- `coding_adventures_heap` (path `../heap`) — provides `MinHeap` for the greedy
  construction algorithm.

## Running Tests

```bash
cd code/packages/elixir/huffman_tree
mise exec -- mix deps.get
mise exec -- mix test --cover
```
