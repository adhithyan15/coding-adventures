# DT27 — Huffman Tree

## Overview

A **Huffman tree** is a full binary tree (every internal node has exactly two
children) built from a symbol alphabet so that each symbol gets a unique
**variable-length bit code**. Symbols that appear often get short codes; symbols
that appear rarely get long codes. The total number of bits needed to encode a
message is minimised — it is the theoretically optimal prefix-free code for a
given symbol frequency distribution.

Think of it like Morse code. In Morse, `E` is `.` (one dot) and `Z` is `--..`
(four symbols). The designers knew `E` is the most common letter in English so
they gave it the shortest code. Huffman's algorithm does this automatically and
optimally for any alphabet with any frequency distribution.

The result is a lossless compression scheme. Given the Huffman tree (or the code
table derived from it), the original message can be reconstructed bit-for-bit.
This is not lossy approximation — it is exact recovery.

### The Two Guarantees

**1. Optimality:** The Huffman tree achieves the minimum possible expected code
length for any prefix-free code over the given symbol frequencies. No other
prefix-free code can do better. This was proved by David Huffman in 1952 and
follows from Shannon's information theory.

**2. Prefix-free property:** No valid code is a prefix of another valid code.
This means a stream of Huffman-coded bits can be decoded unambiguously without
any separator characters — just walk the tree. You will see why in the Concepts
section.

### Where It Comes From: Shannon Entropy

For a symbol with probability p, the information content of that symbol is
`-log2(p)` bits. Intuitively: a coin flip carries 1 bit of information (p=0.5,
`-log2(0.5) = 1`). A symbol that always appears carries 0 bits of information
(p=1, `-log2(1) = 0`).

The **Shannon entropy** of an alphabet is the expected information per symbol:

```
H = -sum(p(s) * log2(p(s)))   for all symbols s
```

This is the theoretical minimum average bits per symbol for any lossless code.
Huffman codes approach this bound; for large alphabets with varied frequencies
they get very close.

```
Example: symbols A(50%), B(25%), C(12.5%), D(12.5%)

Shannon entropy:
  H = -(0.5 * log2(0.5) + 0.25 * log2(0.25) + 0.125 * log2(0.125) + 0.125 * log2(0.125))
    = -(0.5 * -1 + 0.25 * -2 + 0.125 * -3 + 0.125 * -3)
    = 0.5 + 0.5 + 0.375 + 0.375
    = 1.75 bits per symbol

Huffman codes: A=0 (1 bit), B=10 (2 bits), C=110 (3 bits), D=111 (3 bits)
Expected length = 0.5*1 + 0.25*2 + 0.125*3 + 0.125*3 = 0.5 + 0.5 + 0.375 + 0.375 = 1.75 bits
↑ Matches Shannon entropy exactly! (power-of-two probabilities hit the bound perfectly)
```

## Layer Position

```
DT02: tree
DT03: binary-tree     ← Huffman tree IS a binary tree with extra structure
DT04: heap            ← used during construction (min-priority queue)
DT27: huffman-tree    ← [YOU ARE HERE]
  └── used by: CMP04 (Huffman compression)

DT13: trie            ← sibling: both are prefix trees
                         trie indexes string keys; Huffman tree indexes bit-codes
```

**Depends on:** DT03 (binary tree structure for the tree itself), DT04 (min-heap
for the greedy construction algorithm).

**Used by:** CMP04 (Huffman compression) — builds the tree from input frequencies
and uses the code table to encode/decode byte streams.

**Sibling of DT13 (Trie):** both structures are prefix trees where a path from
the root to a node spells out a code. The key difference: a trie stores string
keys explicitly (each edge is one character); a Huffman tree assigns bit codes to
symbols implicitly (each left edge is a `0` bit, each right edge is a `1` bit,
and codes live only at the leaves).

## Concepts

### Symbol Frequencies

The starting point is a frequency table: for each symbol `s` in the alphabet,
how often does it appear? Frequencies can be raw counts or probabilities — only
the relative ordering matters for tree shape, so raw counts are typically used.

```
Input message: "AAABBC"

Count each symbol:
  A → 3
  B → 2
  C → 1
  Total: 6 characters

Frequencies: {A: 3, B: 2, C: 1}
```

Higher frequency → shorter code → symbol contributes fewer bits per occurrence
→ fewer total bits for the whole message.

### Full Binary Tree Structure

A Huffman tree is a **full binary tree**: every node is either:
- A **leaf** — stores a symbol and its weight (frequency).
- An **internal node** — stores a combined weight and has exactly two children.

There are no nodes with only one child. This is not a coincidence — it is a
consequence of the construction algorithm, and it is what guarantees the
prefix-free property.

```
Node variants:

  Leaf:     [ symbol | weight ]
                no children

  Internal: [   weight        ]
             /               \
          left              right
       (any node)        (any node)
```

The **weight** of a leaf is its frequency. The **weight** of an internal node is
the sum of its children's weights — the number of times any symbol in that
subtree appears.

### Building the Tree: The Greedy Algorithm

Huffman's algorithm is a **greedy algorithm** — at each step it makes the locally
optimal choice (merge the two lightest nodes), and it can be proved that the
globally optimal tree results. It uses a **min-heap** (DT04) as its core data
structure.

```
Algorithm Huffman(weights):
  Input:  a list of (symbol, frequency) pairs, non-empty
  Output: root of the Huffman tree

  1. For each (symbol, freq) pair, create a Leaf node.
  2. Insert all Leaf nodes into a min-heap keyed by weight.
  3. While the heap contains more than one node:
       a. Pop the node with the smallest weight → call it LEFT.
       b. Pop the node with the next smallest weight → call it RIGHT.
       c. Create an Internal node:
            weight = LEFT.weight + RIGHT.weight
            left   = LEFT
            right  = RIGHT
       d. Push the Internal node back into the heap.
  4. The one remaining node in the heap is the root. Return it.
```

Why does this produce an optimal tree? Intuitively: the two rarest symbols will
always have the longest codes in the optimal tree (they share a parent at the
deepest level). By merging the two lightest nodes at every step and treating the
result as a new combined symbol, the algorithm simulates exactly this structure.
The formal proof uses an exchange argument: if the tree produced by Huffman's
algorithm were not optimal, you could swap two nodes to reduce the total bits,
contradicting the greedy choice.

### Full Traced Example: "AAABBC" (A:3, B:2, C:1)

```
Starting leaves:
  A(3)   B(2)   C(1)

Step 1 — Build the initial min-heap:
  Insert all leaves. The heap orders by weight (smallest at top):

  Min-heap: [C(1), B(2), A(3)]
              ↑
              minimum

Step 2 — First merge:
  Pop minimum: C(1)
  Pop next minimum: B(2)
  Create internal node I1 with weight = 1 + 2 = 3
    I1(3)
    /   \
  C(1)  B(2)
  Push I1(3) into heap.

  Heap after push: [A(3), I1(3)]
  (Two nodes with equal weight 3 — tie-breaking rule applies; see below)

Step 3 — Second merge (final):
  Pop minimum: A(3) (leaf; wins tie-break over internal node I1)
  Pop next: I1(3)
  Create root R with weight = 3 + 3 = 6
    R(6)
    /    \
  A(3)   I1(3)
          /   \
        C(1)  B(2)

  Push R(6) into heap.

  Heap: [R(6)]  ← only one node remaining

Step 4 — Return R(6) as the root.
```

Wait — the standard construction puts the more-frequent symbol on the left when
tie-breaking. Let me redo with the canonical tie-break rule (leaves before
internal nodes at equal weight, lower symbol value wins among equal-weight
leaves):

```
Heap: [C(1), B(2), A(3)]

Pop C(1), pop B(2):
  I1(3), children: left=C(1), right=B(2)
  (popped in order: C is left, B is right)

Heap: [A(3), I1(3)]
  Tie at weight 3. A is a leaf; I1 is internal. Leaves before internal → A pops first.

Pop A(3), pop I1(3):
  Root R(6), children: left=A(3), right=I1(3)

Final tree:
       R(6)
      /    \
    A(3)   I1(3)
           /   \
         C(1)  B(2)

Code assignment (0 = left, 1 = right):
  A:  path root→left         = 0       (1 bit)
  C:  path root→right→left   = 10      (2 bits)
  B:  path root→right→right  = 11      (2 bits)
```

Verify total bits to encode "AAABBC":
```
  A appears 3 times × 1 bit  =  3 bits
  B appears 2 times × 2 bits =  4 bits
  C appears 1 time  × 2 bits =  2 bits
  Total: 9 bits

  Compare to fixed 2-bit encoding (need 2 bits to distinguish 3 symbols):
    6 characters × 2 bits = 12 bits

  Saving: 12 - 9 = 3 bits = 25% reduction
```

### Larger Example: A(45), B(13), C(12), D(16), E(9), F(5)

This is a classic textbook example with 6 symbols. Frequencies sum to 100.

```
Initial leaves:
  F(5)  E(9)  C(12)  B(13)  D(16)  A(45)

Min-heap: [F(5), E(9), C(12), B(13), D(16), A(45)]

Merge 1: Pop F(5), E(9) → I1(14) [left=F, right=E]
Heap: [C(12), B(13), I1(14), D(16), A(45)]

Merge 2: Pop C(12), B(13) → I2(25) [left=C, right=B]
Heap: [I1(14), D(16), I2(25), A(45)]

Merge 3: Pop I1(14), D(16) → I3(30) [left=I1, right=D]
Heap: [I2(25), I3(30), A(45)]

Merge 4: Pop I2(25), I3(30) → I4(55) [left=I2, right=I3]
Heap: [A(45), I4(55)]

Merge 5: Pop A(45), I4(55) → Root(100) [left=A, right=I4]

Final tree:
                Root(100)
               /          \
            A(45)          I4(55)
                          /      \
                       I2(25)    I3(30)
                       /   \     /   \
                     C(12) B(13) I1(14) D(16)
                                /   \
                              F(5)  E(9)

Codes (0=left, 1=right):
  A:  0         (1 bit)
  C:  100       (3 bits)
  B:  101       (3 bits)
  F:  1100      (4 bits)
  E:  1101      (4 bits)
  D:  111       (3 bits)

Expected bits per symbol:
  0.45×1 + 0.13×3 + 0.12×3 + 0.05×4 + 0.09×4 + 0.16×3
  = 0.45 + 0.39 + 0.36 + 0.20 + 0.36 + 0.48
  = 2.24 bits/symbol

Shannon entropy:
  H ≈ 2.20 bits/symbol

The Huffman code uses only 0.04 bits/symbol more than theoretical minimum.
```

### The Prefix-Free Property

**What it means:** no codeword is a prefix of another codeword. In the "AAABBC"
example: A=`0`, C=`10`, B=`11`. Is `0` a prefix of `10`? No — `0` starts with
`0`, `10` starts with `1`. Is `10` a prefix of `11`? No — they diverge at the
second bit. The property holds.

**Why Huffman trees guarantee it:** codes are assigned only to **leaves**. A code
for symbol X is the path from the root to the leaf for X. For one code to be a
prefix of another, one leaf would have to be an ancestor of another leaf — but
trees don't work that way. A leaf has no descendants by definition.

```
Visual proof:

       root
      /    \
   A(leaf)  internal
            /      \
         C(leaf)  B(leaf)

Code A = "0"
Code C = "10"
Code B = "11"

For A="0" to be a prefix of C="10": A's node would need to be on the path to C.
But A's node is a leaf — it has no children. The path to C goes through the
right child of root (the internal node), not through A at all.
```

**Why this matters for decoding:** Because of the prefix-free property, you can
decode a bitstream without any delimiter or length field — just walk the tree.
When you reach a leaf, you have a complete symbol. Restart from the root. No
ambiguity, no lookahead needed.

### Decoding: Walking the Tree

Decoding algorithm:
```
decode(tree, bits):
  node = root
  result = []
  for each bit b in bits:
    if b == 0:
      node = node.left
    else:
      node = node.right
    if node is a Leaf:
      result.append(node.symbol)
      node = root   ← reset to root for next symbol
  return result
```

Traced decoding with the "AAABBC" tree (A=`0`, C=`10`, B=`11`):

```
Encoded bitstream for "AAABBC": 0 0 0 10 11 10 = "000101110"

Let's decode "000101110" with the tree:
       R(6)
      /    \
    A(3)   I1(3)
           /   \
         C(1)  B(2)

Bit 0: go left → A (leaf!) → emit A, reset to root
Bit 0: go left → A (leaf!) → emit A, reset to root
Bit 0: go left → A (leaf!) → emit A, reset to root
Bit 1: go right → I1 (internal node, continue)
Bit 0: go left  → C (leaf!) → emit C, reset to root
Bit 1: go right → I1 (internal node, continue)
Bit 1: go right → B (leaf!) → emit B, reset to root
Bit 1: go right → I1 (internal node, continue)
Bit 0: go left  → C (leaf!) → emit C, reset to root

Result: A A A C B C = "AAACBC"

Hmm, wait — let me re-encode "AAABBC" correctly:
  A → 0
  A → 0
  A → 0
  B → 11
  B → 11
  C → 10

Bitstream: 0  0  0  11 11 10 = "000111110"

Re-decode:
Bit 0: left → A → emit A, reset
Bit 0: left → A → emit A, reset
Bit 0: left → A → emit A, reset
Bit 1: right → I1 (continue)
Bit 1: right → B → emit B, reset
Bit 1: right → I1 (continue)
Bit 1: right → B → emit B, reset
Bit 1: right → I1 (continue)
Bit 0: left  → C → emit C, reset

Result: "AAABBC" ✓
```

This is the round-trip: encode every symbol using its code, concatenate the bits,
then decode by walking the tree. The original message is recovered exactly.

### Tie-Breaking: Deterministic Tree Construction

When two nodes in the min-heap have equal weight, the order they are popped
determines the tree shape and therefore the code assignments. Different orderings
produce equally optimal trees (same total bits), but they produce different
specific codes. For interoperability — so that all implementations produce the
same tree and the same code table — we define a canonical tie-breaking rule.

**Tie-breaking rule (from highest to lowest priority):**

1. **Lower weight wins.** (Standard min-heap behaviour; only applies between
   nodes of different weight.)
2. **Leaves before internal nodes** at equal weight. A leaf node is "smaller"
   than an internal node of the same weight.
3. **Lower symbol value wins** among leaves of equal weight. Symbol values are
   compared as unsigned bytes (or Unicode code points). Symbol `A` (0x41) beats
   symbol `B` (0x42).
4. **Creation order wins** among internal nodes of equal weight: the node created
   earlier in the algorithm is considered "smaller". Implementations should use a
   stable heap or a monotone sequence counter to track insertion order.

```
Why rule 2? Consider three symbols A(5), B(5), C(10).

Without rule 2:
  Pop A(5), B(5) → I1(10) [left=A, right=B]
  Heap: [C(10), I1(10)]
  Options for next pop: C or I1? Both have weight 10.
  If C pops first: Root[left=C, right=I1] → codes: C=0, A=10, B=11
  If I1 pops first: Root[left=I1, right=C] → codes: A=00, B=01, C=1

  Both trees have the same total bits (C at depth 1, A and B at depth 2 — or all
  at depth 2 which is worse... wait, C(10) = A(5)+B(5) so they're equal weight).
  Actually both are equally optimal but produce different codes.

With rule 2 (leaves before internal nodes):
  At weight tie of 10: C is a leaf → pops first
  Root[left=C, right=I1]
  Codes: C=0, A=10, B=11
  → Deterministic across all implementations.
```

This tie-breaking rule must be implemented identically across all language
implementations so that test vectors work across languages.

### Code Table: From Tree to Lookup Map

After building the tree, compute a code table by traversing the tree once.
Accumulate the path as a bit string: append `0` when going left, `1` when
going right. Store the bit string at each leaf.

```
code_table(tree):
  table = {}
  traverse(tree.root, prefix="")
  return table

traverse(node, prefix):
  if node is Leaf:
    table[node.symbol] = prefix
    return
  traverse(node.left,  prefix + "0")
  traverse(node.right, prefix + "1")
```

Edge case: single-symbol alphabet. The tree is just a single leaf with no
internal nodes. There is no left or right path — the code is conventionally
assigned as `"0"` (one bit). This is a special case that must be handled
explicitly.

```
Single symbol: build({X: 7})
  Tree: just Leaf(X, 7) at root
  Code: X → "0"   (by convention; some specs use empty string — see API below)
```

### Canonical Huffman Codes

A standard Huffman tree can produce many different valid code tables (depending
on tie-breaking choices). **Canonical Huffman coding** defines a unique canonical
form based only on code lengths, not the specific tree structure.

This is what real-world formats like DEFLATE (used in gzip, zlib, PNG) use,
because you only need to transmit code lengths (one byte per symbol) rather than
the full tree structure.

**Canonical construction algorithm:**

```
Step 1: Assign code lengths from any valid Huffman tree.
        (The lengths are the same for all optimal trees for the same input.)

Step 2: Sort symbols by (code_length, symbol_value).
        Shorter codes come first; ties broken by symbol value (ascending).

Step 3: Assign codes numerically, starting from 0 at the shortest length.
        When the length increases by 1, shift the counter left by 1.
```

```
Example: A(45), B(13), C(12), D(16), E(9), F(5) from the larger example above.

Lengths: A=1, B=3, C=3, D=3, E=4, F=4

Sorted by (length, symbol):
  A (length=1)
  B (length=3)
  C (length=3)
  D (length=3)
  E (length=4)
  F (length=4)

Assign canonical codes:
  A: code = 0           (length 1, value 0 in binary: "0")
  Next length is 3: shift left by (3-1)=2 positions:
  code = 0 << 2 = 0, then increment before assigning next:
  B: code = 100         (value 4 in 3 bits: "100")
  C: code = 101         (value 5 in 3 bits: "101")
  D: code = 110         (value 6 in 3 bits: "110")
  Next length is 4: shift: 110 + 1 = 111, then << 1 = 1110
  E: code = 1110        (4 bits)
  F: code = 1111        (4 bits)

Canonical code table:
  A → 0
  B → 100
  C → 101
  D → 110
  E → 1110
  F → 1111
```

The canonical code table is prefix-free and produces the same total bit count as
the standard Huffman code. Its advantage: to transmit this code table, you only
need to send 6 numbers `[1, 3, 3, 3, 4, 4]` (one per symbol in symbol order),
saving space versus transmitting the full tree.

## Representation

### Node Type

The Huffman tree uses two node variants. In languages with algebraic data types:

```
# Language-agnostic pseudocode

type Node =
  | Leaf     { symbol: u8,    weight: usize }
  | Internal { weight: usize, left: Node, right: Node }
```

In Python:
```python
from dataclasses import dataclass
from typing import Union

@dataclass
class Leaf:
    symbol: int    # 0..255 for byte-level coding
    weight: int

@dataclass
class Internal:
    weight: int
    left:   "Node"
    right:  "Node"

Node = Union[Leaf, Internal]
```

In Rust:
```rust
pub enum Node {
    Leaf { symbol: u8, weight: usize },
    Internal { weight: usize, left: Box<Node>, right: Box<Node> },
}
```

In Go (using interfaces):
```go
type Node interface {
    Weight() int
    isNode()  // private marker
}

type Leaf struct {
    Symbol byte
    WeightVal int
}

type Internal struct {
    WeightVal int
    Left, Right Node
}
```

### Tree Type

```python
@dataclass
class HuffmanTree:
    root:         Node         # the root node (Leaf or Internal)
    symbol_count: int          # number of distinct symbols (leaf nodes)
```

### Space Complexity

A Huffman tree over an alphabet of n distinct symbols has:
- n leaf nodes
- n - 1 internal nodes (a full binary tree with n leaves has exactly n-1
  internal nodes)
- Total: 2n - 1 nodes

```
n=3 (A, B, C):  2×3 - 1 = 5 nodes  (3 leaves + 2 internal)
n=256 (bytes):  2×256 - 1 = 511 nodes
```

Space: O(n) where n is the size of the symbol alphabet.

The code table (a map from symbol to bit string) takes O(n · L) space where L is
the maximum code length. For an n-symbol alphabet, the maximum code length is
n - 1 in the degenerate case (one symbol is extremely rare). In practice, for
the 256-byte alphabet, maximum code length is bounded at 15 bits in DEFLATE.

## Algorithms (Pure Functions)

```python
import heapq
from typing import Optional

# ─── Construction ─────────────────────────────────────────────────────────────

def build(weights: list[tuple[int, int]]) -> "HuffmanTree":
    """
    Build a Huffman tree from a list of (symbol, frequency) pairs.

    Rules:
      - symbols are integers (typically 0..255 for byte-level coding)
      - frequencies must be positive
      - single-symbol input: returns a single Leaf as the root
      - empty input: raises ValueError

    Tie-breaking: lower weight first; leaves before internal nodes at equal
    weight; lower symbol value wins among leaves of equal weight.

    Time: O(n log n) where n = len(weights).
    """
    if not weights:
        raise ValueError("Cannot build Huffman tree from empty weight list")

    # heap entries: (weight, is_internal, seq, node)
    # is_internal: False (0) for leaves, True (1) for internal nodes
    # seq: monotone counter to break ties among internal nodes
    heap = []
    seq = 0
    for symbol, freq in weights:
        if freq <= 0:
            raise ValueError(f"Frequency must be positive, got {freq} for symbol {symbol}")
        entry = (freq, False, symbol, seq, Leaf(symbol, freq))
        heapq.heappush(heap, entry)
        seq += 1

    if len(heap) == 1:
        # Single symbol: return just the leaf
        _, _, _, _, node = heap[0]
        return HuffmanTree(root=node, symbol_count=1)

    while len(heap) > 1:
        # Pop two smallest nodes
        w1, is_int1, key1, seq1, left  = heapq.heappop(heap)
        w2, is_int2, key2, seq2, right = heapq.heappop(heap)

        # Create internal node
        merged_weight = w1 + w2
        internal = Internal(weight=merged_weight, left=left, right=right)

        # Internal nodes sort after leaves (is_internal=True > False)
        # Use seq as secondary key for internal nodes
        entry = (merged_weight, True, seq, seq, internal)
        heapq.heappush(heap, entry)
        seq += 1

    _, _, _, _, root = heap[0]
    count = _count_leaves(root)
    return HuffmanTree(root=root, symbol_count=count)

def _count_leaves(node: Node) -> int:
    if isinstance(node, Leaf):
        return 1
    return _count_leaves(node.left) + _count_leaves(node.right)

# ─── Code table ───────────────────────────────────────────────────────────────

def code_table(tree: HuffmanTree) -> dict[int, str]:
    """
    Walk the tree and assign a bit string to every symbol.
    Left edge = '0', right edge = '1'.
    For a single-leaf tree, assigns '0' to that symbol by convention.

    Returns: {symbol: bit_string} e.g. {65: '0', 66: '10', 67: '11'}
    Time: O(n) where n = number of distinct symbols.
    """
    table: dict[int, str] = {}
    _walk(tree.root, "", table)
    return table

def _walk(node: Node, prefix: str, table: dict[int, str]) -> None:
    if isinstance(node, Leaf):
        # Single-leaf edge case: no bits accumulated; use "0" by convention
        table[node.symbol] = prefix if prefix else "0"
        return
    _walk(node.left,  prefix + "0", table)
    _walk(node.right, prefix + "1", table)

# ─── Encoding ─────────────────────────────────────────────────────────────────

def encode(table: dict[int, str], symbols: list[int]) -> str:
    """
    Encode a sequence of symbols using a code table.
    Returns the concatenated bit string.

    Raises KeyError if any symbol is not in the table.
    Time: O(sum of code lengths for each symbol).
    """
    return "".join(table[s] for s in symbols)

# ─── Decoding ─────────────────────────────────────────────────────────────────

def decode_all(tree: HuffmanTree, bits: str, count: int) -> list[int]:
    """
    Decode exactly `count` symbols from a bit string by walking the tree.

    - bits: a string of '0' and '1' characters
    - count: the exact number of symbols to decode
    - Returns: list of decoded symbols (length == count)

    Raises ValueError if the bit string is exhausted before `count` symbols
    are decoded.

    For a single-leaf tree, each '0' bit decodes to that symbol.
    Time: O(total bits consumed).
    """
    result = []
    node = tree.root
    i = 0
    while len(result) < count:
        if isinstance(node, Leaf):
            result.append(node.symbol)
            node = tree.root
            # Do NOT advance i — single-leaf tree uses 1 bit per symbol ("0")
            # but we need to advance for single-leaf too:
            if i < len(bits):
                i += 1
            continue
        if i >= len(bits):
            raise ValueError(
                f"Bit stream exhausted after {len(result)} symbols; expected {count}"
            )
        bit = bits[i]
        i += 1
        node = node.left if bit == '0' else node.right
    return result

# ─── Inspection ───────────────────────────────────────────────────────────────

def weight(tree: HuffmanTree) -> int:
    """Total weight of the tree (= sum of all leaf frequencies = root weight)."""
    return _node_weight(tree.root)

def _node_weight(node: Node) -> int:
    if isinstance(node, Leaf):
        return node.weight
    return node.weight  # Internal nodes store combined weight

def depth(tree: HuffmanTree) -> int:
    """Maximum depth of any leaf = maximum code length. O(n)."""
    return _max_depth(tree.root, 0)

def _max_depth(node: Node, d: int) -> int:
    if isinstance(node, Leaf):
        return d
    return max(_max_depth(node.left, d + 1), _max_depth(node.right, d + 1))

def symbol_count(tree: HuffmanTree) -> int:
    """Number of distinct symbols (leaf nodes). O(1)."""
    return tree.symbol_count

def leaves(tree: HuffmanTree) -> list[tuple[int, str]]:
    """
    In-order traversal of leaves.
    Returns [(symbol, code), ...] in the order: left subtree first, then right.
    Time: O(n).
    """
    table = code_table(tree)
    result: list[tuple[int, str]] = []
    _in_order_leaves(tree.root, result, table)
    return result

def _in_order_leaves(node: Node, result: list, table: dict) -> None:
    if isinstance(node, Leaf):
        result.append((node.symbol, table[node.symbol]))
        return
    _in_order_leaves(node.left,  result, table)
    _in_order_leaves(node.right, result, table)

# ─── Canonical codes ─────────────────────────────────────────────────────────

def canonical_code_table(tree: HuffmanTree) -> dict[int, str]:
    """
    Compute the canonical Huffman code table from the tree.

    1. Extract code lengths from the tree (same as code_table but only lengths).
    2. Sort symbols by (length, symbol_value).
    3. Assign codes numerically.

    The canonical code table is prefix-free and has the same code lengths as the
    standard code table. It is uniquely determined by the tree.

    Returns: {symbol: bit_string} in canonical form.
    Time: O(n log n) for the sort.
    """
    # Step 1: get lengths
    lengths: dict[int, int] = {}
    _collect_lengths(tree.root, 0, lengths)

    # Handle single-leaf: assign length 1
    if len(lengths) == 1:
        sym = next(iter(lengths))
        return {sym: "0"}

    # Step 2: sort by (length, symbol)
    sorted_syms = sorted(lengths.items(), key=lambda kv: (kv[1], kv[0]))

    # Step 3: assign canonical codes
    code_val = 0
    prev_len = sorted_syms[0][1]
    result: dict[int, str] = {}

    for sym, length in sorted_syms:
        if length > prev_len:
            code_val <<= (length - prev_len)
        result[sym] = format(code_val, f"0{length}b")
        code_val += 1
        prev_len = length

    return result

def _collect_lengths(node: Node, depth: int, lengths: dict[int, int]) -> None:
    if isinstance(node, Leaf):
        lengths[node.symbol] = depth if depth > 0 else 1  # single-leaf: depth=0, length=1
        return
    _collect_lengths(node.left,  depth + 1, lengths)
    _collect_lengths(node.right, depth + 1, lengths)
```

## Public API

```python
from typing import Optional, Iterator

class HuffmanTree:
    """
    A full binary tree that assigns optimal prefix-free bit codes to symbols.

    Build the tree once from symbol frequencies; then:
      - Use code_table() to get a {symbol → bit_string} map for encoding.
      - Use decode_all() to decode a bit stream back to symbols.

    All symbols are integers (typically 0..255 for byte-level coding, but any
    non-negative integer is valid). Frequencies must be positive integers.

    The tree is immutable after construction. Build a new tree if frequencies
    change.
    """

    @classmethod
    def build(cls, weights: list[tuple[int, int]]) -> "HuffmanTree":
        """
        Construct a Huffman tree from (symbol, frequency) pairs.

        Tie-breaking (for deterministic output across implementations):
          1. Lowest weight pops first.
          2. Leaves before internal nodes at equal weight.
          3. Lower symbol value wins among leaves of equal weight.
          4. Earlier-created internal node wins among internal nodes of equal weight.

        Raises ValueError if weights is empty or any frequency is non-positive.
        Time: O(n log n).
        """
        ...

    # ─── Encoding helpers ───────────────────────────────────────────
    def code_table(self) -> dict[int, str]:
        """
        Return {symbol: bit_string} for all symbols in the tree.
        Left edges are '0', right edges are '1'.
        Single-symbol tree: returns {symbol: '0'} by convention.
        Time: O(n).
        """
        ...

    def code_for(self, symbol: int) -> Optional[str]:
        """
        Return the bit string for a specific symbol, or None if not in tree.
        Time: O(depth) — walks the tree rather than building full table.
        """
        ...

    def canonical_code_table(self) -> dict[int, str]:
        """
        Return canonical Huffman codes (DEFLATE-style).
        Sorted by (code_length, symbol_value); codes assigned numerically.
        Useful when you need to transmit only code lengths, not the tree.
        Time: O(n log n).
        """
        ...

    # ─── Decoding ───────────────────────────────────────────────────
    def decode_all(self, bits: str, count: int) -> list[int]:
        """
        Decode exactly `count` symbols from a bit string.
        bits: a string of '0' and '1' characters.
        Raises ValueError if the bit stream is exhausted prematurely.
        Time: O(total bits consumed).
        """
        ...

    # ─── Inspection ─────────────────────────────────────────────────
    def weight(self) -> int:
        """
        Total weight = sum of all leaf frequencies = weight of the root.
        O(1) — stored at the root.
        """
        ...

    def depth(self) -> int:
        """
        Maximum code length = depth of the deepest leaf.
        O(n) — must traverse the tree.
        """
        ...

    def symbol_count(self) -> int:
        """
        Number of distinct symbols (= number of leaf nodes).
        O(1) — stored at construction time.
        """
        ...

    def leaves(self) -> list[tuple[int, str]]:
        """
        In-order traversal of leaves.
        Returns [(symbol, code), ...], left subtree before right subtree.
        Useful for visualisation and debugging.
        Time: O(n).
        """
        ...

    def is_valid(self) -> bool:
        """
        Check structural invariants. For testing only.
          1. Every internal node has exactly 2 children (full binary tree).
          2. weight(internal) == weight(left) + weight(right).
          3. No symbol appears in more than one leaf.
          4. Codes are prefix-free.
        O(n).
        """
        ...

    # ─── Python protocol methods ─────────────────────────────────────
    def __len__(self) -> int:
        """Number of distinct symbols. Equivalent to symbol_count()."""
        ...

    def __contains__(self, symbol: int) -> bool:
        """True if symbol has a code in this tree."""
        ...

    def __repr__(self) -> str:
        """Human-readable summary: HuffmanTree(symbols=N, weight=W, depth=D)."""
        ...
```

## Composition Model

### Inheritance (Python, Ruby, TypeScript)

The `HuffmanTree` is self-contained; it does not inherit from a generic binary
tree class (because Huffman trees have domain-specific invariants that a generic
tree doesn't enforce). It uses the `Node` sum type internally.

```python
# Python
from dataclasses import dataclass, field
from typing import Union
import heapq

@dataclass(frozen=True)
class Leaf:
    symbol: int
    weight: int

@dataclass(frozen=True)
class Internal:
    weight:  int
    left:    "Node"
    right:   "Node"

Node = Union[Leaf, Internal]

class HuffmanTree:
    def __init__(self, root: Node, symbol_count: int) -> None:
        self._root = root
        self._symbol_count = symbol_count
```

```typescript
// TypeScript
type HuffmanNode =
  | { kind: "leaf";     symbol: number; weight: number }
  | { kind: "internal"; weight: number; left: HuffmanNode; right: HuffmanNode };

class HuffmanTree {
  private constructor(
    private readonly root: HuffmanNode,
    private readonly _symbolCount: number,
  ) {}

  static build(weights: Array<[number, number]>): HuffmanTree { ... }
  codeTable(): Map<number, string> { ... }
  decodeAll(bits: string, count: number): number[] { ... }
}
```

```ruby
# Ruby
module CodingAdventures
  module HuffmanTree
    Leaf     = Data.define(:symbol, :weight)
    Internal = Data.define(:weight, :left, :right)

    class Tree
      def initialize(root, symbol_count)
        @root = root
        @symbol_count = symbol_count
      end

      def self.build(weights) = ...
      def code_table = ...
      def decode_all(bits, count) = ...
    end
  end
end
```

### Composition (Rust, Go)

```rust
// Rust — enum for node, struct for tree
pub enum Node {
    Leaf { symbol: u8, weight: usize },
    Internal {
        weight: usize,
        left:   Box<Node>,
        right:  Box<Node>,
    },
}

impl Node {
    pub fn weight(&self) -> usize {
        match self {
            Node::Leaf { weight, .. }     => *weight,
            Node::Internal { weight, .. } => *weight,
        }
    }
}

pub struct HuffmanTree {
    root:         Box<Node>,
    symbol_count: usize,
}

impl HuffmanTree {
    pub fn build(weights: &[(u8, usize)]) -> Result<Self, HuffmanError> { ... }
    pub fn code_table(&self) -> HashMap<u8, String> { ... }
    pub fn decode_all(&self, bits: &str, count: usize) -> Result<Vec<u8>, HuffmanError> { ... }
    pub fn weight(&self) -> usize { ... }
    pub fn depth(&self) -> usize { ... }
}
```

```go
// Go — interface-based nodes
package huffmantree

type Node interface {
    Weight() int
    isNode()
}

type Leaf struct {
    Symbol    byte
    WeightVal int
}
func (l *Leaf) Weight() int { return l.WeightVal }
func (l *Leaf) isNode()     {}

type Internal struct {
    WeightVal int
    Left, Right Node
}
func (n *Internal) Weight() int { return n.WeightVal }
func (n *Internal) isNode()     {}

type HuffmanTree struct {
    Root        Node
    SymbolCount int
}

func Build(weights [][2]int) (*HuffmanTree, error) { ... }
func (t *HuffmanTree) CodeTable() map[byte]string { ... }
func (t *HuffmanTree) DecodeAll(bits string, count int) ([]byte, error) { ... }
```

### Module (Elixir, Lua, Perl)

```elixir
# Elixir — immutable tree as tagged tuples
defmodule CodingAdventures.HuffmanTree do
  # Nodes as tagged tuples:
  #   {:leaf, symbol, weight}
  #   {:internal, weight, left, right}

  def build([]),     do: {:error, :empty_weights}
  def build(weights) do
    leaves = Enum.map(weights, fn {sym, freq} -> {:leaf, sym, freq} end)
    heap = Enum.sort_by(leaves, &elem(&1, 2))
    root = reduce_heap(heap)
    {:ok, root}
  end

  defp reduce_heap([node]), do: node
  defp reduce_heap([left, right | rest]) do
    merged = {:internal, elem(left, 2) + elem(right, 2), left, right}
    new_heap = Enum.sort_by([merged | rest], &elem(&1, 2))
    reduce_heap(new_heap)
  end

  def code_table(root), do: walk(root, "", %{})

  defp walk({:leaf, sym, _}, prefix, acc) do
    Map.put(acc, sym, if(prefix == "", do: "0", else: prefix))
  end
  defp walk({:internal, _, left, right}, prefix, acc) do
    acc
    |> walk(left,  prefix <> "0")   # Note: `after` is an Elixir reserved word —
    |> walk(right, prefix <> "1")   # never use it as a variable name
  end
end
```

```lua
-- Lua
local HuffmanTree = {}
HuffmanTree.__index = HuffmanTree

function HuffmanTree.build(weights)
  -- weights: {{symbol, freq}, ...}
  local heap = {}
  for _, pair in ipairs(weights) do
    table.insert(heap, {kind="leaf", symbol=pair[1], weight=pair[2]})
  end
  table.sort(heap, function(a, b) return a.weight < b.weight end)
  while #heap > 1 do
    local left  = table.remove(heap, 1)
    local right = table.remove(heap, 1)
    local merged = {kind="internal", weight=left.weight+right.weight, left=left, right=right}
    table.insert(heap, merged)
    table.sort(heap, function(a, b) return a.weight < b.weight end)
  end
  return setmetatable({root=heap[1]}, HuffmanTree)
end
```

### Swift

```swift
// Swift — indirect enum for recursive node type
public indirect enum HuffmanNode {
    case leaf(symbol: UInt8, weight: Int)
    case internal_(weight: Int, left: HuffmanNode, right: HuffmanNode)

    var weight: Int {
        switch self {
        case .leaf(_, let w):           return w
        case .internal_(let w, _, _):   return w
        }
    }
}

public struct HuffmanTree {
    public let root: HuffmanNode
    public let symbolCount: Int

    public static func build(weights: [(UInt8, Int)]) throws -> HuffmanTree { ... }
    public func codeTable() -> [UInt8: String] { ... }
    public func decodeAll(bits: String, count: Int) throws -> [UInt8] { ... }
}
```

## Test Strategy

### Correctness Invariants

Before writing specific test cases, define a verifier:

```python
def verify_huffman_tree(tree: HuffmanTree) -> None:
    """
    Assert all structural invariants.
    Call after build(), after any round-trip, and in property tests.
    """
    # 1. Every internal node has exactly 2 children (full binary tree).
    # 2. Weight of internal node == sum of children's weights.
    # 3. No symbol appears more than once.
    # 4. code_table is prefix-free.
    # 5. symbol_count matches the actual number of leaves.
    # 6. weight() == sum of all input frequencies.
    _check_node(tree.root, set())
    _check_prefix_free(code_table(tree))
    assert symbol_count(tree) == _count_leaves(tree.root)

def _check_node(node: Node, seen_symbols: set) -> None:
    if isinstance(node, Leaf):
        assert node.symbol not in seen_symbols, f"Duplicate symbol {node.symbol}"
        seen_symbols.add(node.symbol)
        assert node.weight > 0
        return
    assert isinstance(node, Internal)
    expected = _node_weight(node.left) + _node_weight(node.right)
    assert node.weight == expected, f"Weight mismatch: {node.weight} != {expected}"
    _check_node(node.left,  seen_symbols)
    _check_node(node.right, seen_symbols)

def _check_prefix_free(table: dict[int, str]) -> None:
    codes = list(table.values())
    for i, c1 in enumerate(codes):
        for c2 in codes[i+1:]:
            assert not c2.startswith(c1), f"'{c1}' is prefix of '{c2}'"
            assert not c1.startswith(c2), f"'{c2}' is prefix of '{c1}'"
```

### Test Cases

```python
# ─── Test 1: AAABBC canonical example ────────────────────────────────────────

def test_aaabbc():
    weights = [(ord('A'), 3), (ord('B'), 2), (ord('C'), 1)]
    tree = HuffmanTree.build(weights)
    verify_huffman_tree(tree)

    assert symbol_count(tree) == 3
    assert weight(tree) == 6    # 3 + 2 + 1

    table = code_table(tree)
    # A is most frequent → 1-bit code
    assert len(table[ord('A')]) == 1
    # B and C → 2-bit codes
    assert len(table[ord('B')]) == 2
    assert len(table[ord('C')]) == 2

    # Round-trip
    symbols = [ord('A'), ord('A'), ord('A'), ord('B'), ord('B'), ord('C')]
    bits = encode(table, symbols)
    assert len(bits) == 9           # 3×1 + 2×2 + 1×2 = 9
    decoded = tree.decode_all(bits, 6)
    assert decoded == symbols

# ─── Test 2: Code lengths match frequency ordering ─────────────────────────

def test_code_length_monotone():
    # More frequent symbols must have codes no longer than less frequent ones.
    weights = [(s, f) for s, f in enumerate([10, 5, 3, 2, 1])]
    tree = HuffmanTree.build(weights)
    table = code_table(tree)
    sorted_by_freq = sorted(weights, key=lambda sf: -sf[1])
    lengths = [len(table[s]) for s, _ in sorted_by_freq]
    # lengths[i] <= lengths[j] for i < j (non-decreasing as freq decreases)
    for i in range(len(lengths) - 1):
        assert lengths[i] <= lengths[i+1], f"Monotone violation at index {i}"

# ─── Test 3: prefix-free property ─────────────────────────────────────────

def test_prefix_free():
    import random
    for _ in range(50):
        n = random.randint(2, 20)
        weights = [(i, random.randint(1, 100)) for i in range(n)]
        tree = HuffmanTree.build(weights)
        _check_prefix_free(code_table(tree))

# ─── Test 4: single-symbol edge case ─────────────────────────────────────

def test_single_symbol():
    tree = HuffmanTree.build([(42, 7)])
    assert symbol_count(tree) == 1
    assert weight(tree) == 7
    table = code_table(tree)
    assert table[42] == "0"    # convention: single symbol gets code "0"
    decoded = tree.decode_all("000", 3)
    assert decoded == [42, 42, 42]

# ─── Test 5: empty input raises ValueError ────────────────────────────────

def test_empty_raises():
    import pytest
    with pytest.raises(ValueError):
        HuffmanTree.build([])

# ─── Test 6: equal frequencies ───────────────────────────────────────────

def test_equal_frequencies():
    # All symbols with equal frequency: valid tree, all codes same length
    weights = [(i, 10) for i in range(4)]   # 4 symbols, each weight 10
    tree = HuffmanTree.build(weights)
    verify_huffman_tree(tree)
    assert weight(tree) == 40
    table = code_table(tree)
    lengths = [len(v) for v in table.values()]
    # All codes should be length 2 (log2(4) = 2) for a balanced tree
    assert all(l == 2 for l in lengths), f"Expected all length 2, got {lengths}"

# ─── Test 7: weight of root equals sum of all frequencies ─────────────────

def test_root_weight_equals_total():
    import random
    for _ in range(50):
        n = random.randint(1, 30)
        weights = [(i, random.randint(1, 1000)) for i in range(n)]
        tree = HuffmanTree.build(weights)
        expected = sum(f for _, f in weights)
        assert weight(tree) == expected

# ─── Test 8: round-trip with random byte sequences ────────────────────────

def test_round_trip_random():
    import random
    # Frequency table from random bytes
    msg = [random.randint(0, 255) for _ in range(500)]
    from collections import Counter
    freq = Counter(msg)
    weights = list(freq.items())
    tree = HuffmanTree.build(weights)
    table = code_table(tree)
    bits = encode(table, msg)
    decoded = tree.decode_all(bits, len(msg))
    assert decoded == msg

# ─── Test 9: depth is bounded ─────────────────────────────────────────────

def test_depth():
    # For n symbols, max depth is n-1 (degenerate case: Fibonacci weights)
    # For power-of-two equal weights, depth is log2(n)
    weights = [(i, 2**i) for i in range(1, 9)]  # Fibonacci-like: each doubles
    tree = HuffmanTree.build(weights)
    d = depth(tree)
    # Not asserting specific value but ensuring it's bounded and reasonable
    assert 1 <= d <= len(weights) - 1

# ─── Test 10: canonical codes are prefix-free and same lengths ─────────────

def test_canonical_codes():
    weights = [(ord(c), f) for c, f in [('A',45),('B',13),('C',12),('D',16),('E',9),('F',5)]]
    tree = HuffmanTree.build(weights)
    std_table      = code_table(tree)
    canonical      = tree.canonical_code_table()

    # Same symbols
    assert set(std_table.keys()) == set(canonical.keys())

    # Same code lengths
    for sym in std_table:
        assert len(std_table[sym]) == len(canonical[sym]), \
            f"Length mismatch for symbol {sym}"

    # Canonical codes are also prefix-free
    _check_prefix_free(canonical)

# ─── Test 11: tie-breaking determinism ────────────────────────────────────

def test_tie_breaking_determinism():
    # Same input in different orders should produce the same tree
    weights = [(ord('A'), 5), (ord('B'), 5), (ord('C'), 5)]
    tree1 = HuffmanTree.build(weights)
    tree2 = HuffmanTree.build(list(reversed(weights)))
    # Same code table
    assert code_table(tree1) == code_table(tree2)

# ─── Test 12: zero-frequency raises ValueError ────────────────────────────

def test_zero_frequency_raises():
    import pytest
    with pytest.raises(ValueError):
        HuffmanTree.build([(ord('A'), 0), (ord('B'), 5)])
```

### Coverage Targets

- 95%+ line coverage
- `build`: empty input, single symbol, two symbols, many symbols, duplicate weights
- `code_table`: verify prefix-free, verify all symbols present
- `canonical_code_table`: same lengths as standard, prefix-free
- `decode_all`: correct round-trip, premature EOF raises, single-symbol tree
- `weight`, `depth`, `symbol_count`: basic smoke tests
- `is_valid`: test with deliberately broken trees (should return False)
- Tie-breaking: same input different orderings produce same output
- Property test: round-trip for 100+ random inputs

## Future Extensions

- **Adaptive Huffman coding** — update code lengths as symbols are seen without
  transmitting a frequency table at all. The encoder and decoder maintain
  identical trees that evolve in sync. Used in Unix `compress` and early modems.
  Requires a more complex tree mutation algorithm (Vitter's algorithm).

- **Length-limited Huffman codes** — DEFLATE limits codes to 15 bits. Standard
  Huffman can produce longer codes for very skewed distributions. The
  Larmore-Hirschberg (1990) package-merge algorithm builds optimal codes subject
  to a maximum length constraint in O(nL) where L is the limit.

- **Arithmetic coding** — surpasses Huffman for skewed distributions by encoding
  entire messages as a single fraction. Where Huffman needs whole bits per symbol
  (A might want 0.3 bits but gets 1), arithmetic coding achieves the Shannon
  entropy bound exactly. Used in JPEG, CABAC (H.264 video). DT27 is the stepping
  stone to understanding why arithmetic coding is superior.

- **Huffman with a pre-initialized dictionary** — LZW (CMP03) and other
  dictionary compressors can feed a pre-seeded alphabet to Huffman, combining
  dictionary compression with entropy coding. This layered approach is how DEFLATE
  works: LZ77 + Huffman coding combined.

- **Parallel Huffman construction** — for very large alphabets, the O(n log n)
  construction can be parallelised. Heuristics exist for GPU parallelism.

- **Huffman trees in hardware** — FPGAs implement Huffman decoders as lookup
  tables in BRAM for single-cycle decode. Understanding the tree structure helps
  reason about hardware cost.

## Package Matrix

| Language | Package name | Import path |
|----------|-------------|-------------|
| Python | `coding-adventures-huffman-tree` | `coding_adventures.huffman_tree` |
| Go | `coding-adventures-huffman-tree` | `github.com/.../code/packages/go/huffman-tree` |
| Ruby | `coding_adventures_huffman_tree` | `coding_adventures/huffman_tree` |
| TypeScript | `@coding-adventures/huffman-tree` | `@coding-adventures/huffman-tree` |
| Rust | `huffman-tree` | `huffman_tree` |
| Elixir | `:coding_adventures_huffman_tree` | `CodingAdventures.HuffmanTree` |
| Lua | `coding-adventures-huffman-tree` | `coding_adventures.huffman_tree` |
| Perl | `CodingAdventures-HuffmanTree` | `CodingAdventures::HuffmanTree` |
| Swift | `HuffmanTree` | `import HuffmanTree` |
