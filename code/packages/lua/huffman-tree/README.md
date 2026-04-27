# coding-adventures-huffman-tree (Lua)

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

Think of it like Morse code. In Morse, `E` is `.` (one dot) and `Z` is `--..`
(four symbols). The designers knew `E` is the most common letter in English so
they gave it the shortest code. Huffman's algorithm does this automatically and
optimally for any alphabet with any frequency distribution.

## Installation

```bash
luarocks install coding-adventures-huffman-tree
```

## Usage

```lua
local HuffmanTree = require("coding_adventures.huffman_tree")

-- Build a tree from (symbol, frequency) pairs.
-- Symbols are integers; frequencies must be positive.
local tree = HuffmanTree.build({
    {65, 3},  -- 'A' appears 3 times
    {66, 2},  -- 'B' appears 2 times
    {67, 1},  -- 'C' appears 1 time
})

-- Get the code table: {[symbol] = bit_string}
local tbl = tree:code_table()
-- tbl[65] = "0"   (A gets the shortest code)
-- tbl[66] = "10"
-- tbl[67] = "11"

-- Encode a message
local message = {65, 65, 66, 67}  -- AABC
local bits = ""
for _, sym in ipairs(message) do
    bits = bits .. tbl[sym]
end
-- bits = "001011"

-- Decode
local decoded = tree:decode_all(bits, 4)
-- decoded = {65, 65, 66, 67}

-- Canonical codes (DEFLATE-style)
local canon = tree:canonical_code_table()
-- Same code lengths as regular, but assigned numerically for compactness.

-- Inspection
print(tree:weight())        -- 6  (total of all frequencies)
print(tree:depth())         -- 2  (max code length)
print(tree:symbol_count())  -- 3

-- In-order leaf traversal
for _, pair in ipairs(tree:leaves()) do
    print(pair[1], pair[2])  -- symbol, code
end

-- Validity check (for testing)
print(tree:is_valid())  -- true
```

## Algorithm

Greedy min-heap construction with deterministic tie-breaking:

1. Create one leaf per symbol, push all onto a min-heap.
2. Pop two smallest nodes, merge into an internal node, push back.
3. Repeat until one node remains — this is the root.

Tie-breaking (for identical output across all implementations):
1. Lowest weight pops first.
2. Leaf before internal at equal weight.
3. Lower symbol value among equal-weight leaves.
4. Earlier-created (FIFO) among equal-weight internal nodes.

## Dependency

This package depends on `coding-adventures-heap` for the standalone `MinHeap`
used during tree construction.

## API

| Function | Description |
|---|---|
| `HuffmanTree.build(weights)` | Build tree from `{{symbol, freq}, ...}` |
| `tree:code_table()` | Returns `{[symbol] = bitstring}` |
| `tree:code_for(symbol)` | Returns the bit string for one symbol, or nil |
| `tree:canonical_code_table()` | Returns DEFLATE-style canonical codes |
| `tree:decode_all(bits, count)` | Decode `count` symbols from a bit string |
| `tree:weight()` | Total weight (sum of all frequencies) |
| `tree:depth()` | Maximum code length |
| `tree:symbol_count()` | Number of distinct symbols |
| `tree:leaves()` | In-order leaf list: `{{symbol, code}, ...}` |
| `tree:is_valid()` | Check structural invariants |

## The Series

| Code | Algorithm | Year | Description |
|---|---|---|---|
| CMP00 | LZ77 | 1977 | Sliding-window backreferences |
| CMP01 | LZ78 | 1978 | Explicit dictionary (trie) |
| CMP02 | LZSS | 1982 | LZ77 + flag bits |
| CMP03 | LZW | 1984 | Pre-initialised dictionary; powers GIF |
| **DT27** | **Huffman** | **1952** | **Entropy coding; prerequisite for DEFLATE** |
| CMP05 | DEFLATE | 1996 | LZ77 + Huffman; ZIP/gzip/PNG/zlib |

## Running Tests

```bash
luarocks show coding-adventures-heap 1>/dev/null 2>/dev/null || (cd ../heap && luarocks make --local --deps-mode=none coding-adventures-heap-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-huffman-tree-0.1.0-1.rockspec
cd tests && LUA_PATH="../src/?.lua;../src/?/init.lua;../../heap/src/?.lua;../../heap/src/?/init.lua;;" busted . --verbose --pattern=test_
```

## License

MIT
