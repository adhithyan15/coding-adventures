# huffman-tree — DT27

Huffman tree data structure for the coding-adventures monorepo.  Implements
the greedy min-heap construction algorithm and provides full encoding, decoding,
and canonical (DEFLATE-style) code generation.

## What it is

A Huffman tree is a full binary tree where each leaf represents a symbol and
each internal node combines two sub-trees.  By always merging the two
lowest-weight nodes first, the algorithm produces an optimal prefix-free code:
the most frequent symbols get the shortest bit strings and the total encoding
length is minimised.

Think of Morse code: `E` is `.` (one dot, very common) and `Z` is `--..` (four
symbols, rare).  Huffman's algorithm does this automatically and provably
optimally.

## Placement in the stack

```text
DT27 (Huffman tree, this crate)  ← data structure
      ↓
CMP04 (Huffman compression)      ← uses DT27 to compress/decompress data
      ↓
CMP05 (DEFLATE)                  ← uses both LZ77 (CMP00) and Huffman (CMP04)
```

The tree is also used for canonical DEFLATE-style codes, where only the code
lengths are transmitted (not the tree), saving space.

## Usage

```rust
use huffman_tree::HuffmanTree;

// Build from (symbol, frequency) pairs.
let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();

// Encode: get a symbol → bit-string map.
let table = tree.code_table();
println!("A = {}", table[&65]); // "0"

// Decode a bit string back to symbols.
let symbols = tree.decode_all("001011", 4).unwrap();
println!("{:?}", symbols); // [65, 65, 67, 66]

// Canonical codes (DEFLATE-style, transmissible by length table only).
let canonical = tree.canonical_code_table();
println!("B canonical = {}", canonical[&66]); // "10"
```

## API

| Method | Description |
|--------|-------------|
| `build(weights)` | Construct from `(symbol, frequency)` pairs |
| `code_table()` | Returns `HashMap<u16, String>` of tree-walk codes |
| `code_for(symbol)` | Lookup a single symbol's code without full table |
| `canonical_code_table()` | DEFLATE-style canonical codes |
| `decode_all(bits, count)` | Decode exactly `count` symbols from a bit string |
| `weight()` | Root weight = total frequency sum |
| `depth()` | Maximum code length = deepest leaf depth |
| `symbol_count()` | Number of distinct symbols |
| `leaves()` | In-order leaf traversal: `[(symbol, code), ...]` |
| `is_valid()` | Structural invariant check (for testing) |

## Tie-breaking rules

For deterministic output that matches all other language implementations in
this monorepo:

1. Lowest weight pops first.
2. Leaf nodes have higher priority than internal nodes at equal weight.
3. Among leaves of equal weight, lower symbol value wins.
4. Among internal nodes of equal weight, earlier-created node wins (FIFO).

## Dependencies

- [`heap`](../heap) — provides `MinHeap<T: Ord>` used during construction.

## Building and testing

```bash
# From code/packages/rust/
cargo test -p huffman-tree -- --nocapture
```

## Series context

```text
CMP00 (LZ77,     1977) — Sliding-window backreferences.
CMP01 (LZ78,     1978) — Explicit dictionary (trie).
CMP02 (LZSS,     1982) — LZ77 + flag bits; no wasted literals.
CMP03 (LZW,      1984) — LZ78 + pre-initialised alphabet; GIF.
DT27  (Huffman tree)   ← this crate
CMP04 (Huffman,  1952) — Entropy coding; prerequisite for DEFLATE.
CMP05 (DEFLATE,  1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
```
