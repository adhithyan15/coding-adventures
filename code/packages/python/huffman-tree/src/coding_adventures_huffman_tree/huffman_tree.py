"""
huffman_tree.py — DT27: Huffman Tree
=====================================

A Huffman tree is a full binary tree (every internal node has exactly two
children) built from a symbol alphabet so that each symbol gets a unique
variable-length bit code.  Symbols that appear often get short codes;
symbols that appear rarely get long codes.  The total bits needed to
encode a message is minimised — it is the theoretically optimal prefix-free
code for a given symbol frequency distribution.

Think of it like Morse code.  In Morse, ``E`` is ``.`` (one dot) and
``Z`` is ``--..`` (four symbols).  The designers knew ``E`` is the most
common letter in English so they gave it the shortest code.  Huffman's
algorithm does this automatically and optimally for any alphabet with any
frequency distribution.

============================================================
Algorithm: Greedy construction via min-heap
============================================================

1. Create one leaf node per distinct symbol, each with its frequency as its
   weight.  Push all leaves onto a min-heap keyed by weight.

2. While the heap has more than one node:
     a. Pop the two nodes with the smallest weight.
     b. Create a new internal node whose weight = sum of the two children.
     c. Set left = the first popped node, right = the second popped node.
     d. Push the new internal node back onto the heap.

3. The one remaining node is the root of the Huffman tree.

Tie-breaking rules (for deterministic output across implementations):
  1. Lowest weight pops first.
  2. Leaf nodes have higher priority than internal nodes at equal weight
     ("leaf-before-internal" rule).
  3. Among leaves of equal weight, lower symbol value wins.
  4. Among internal nodes of equal weight, earlier-created node wins
     (insertion-order FIFO).

Why these rules?  Without tie-breaking, different implementations could
build structurally different trees from the same input — producing different
(but equally valid) code lengths.  Deterministic tie-breaking ensures the
canonical code table is identical everywhere.

============================================================
Prefix-free property: why it works
============================================================

In a Huffman tree:
  - Symbols live ONLY at the leaves, never at internal nodes.
  - The code for a symbol is the path from root to its leaf
    (left edge = '0', right edge = '1').

Since one leaf is never an ancestor of another leaf, no code can be a
prefix of another code.  This is the prefix-free property, and it means the
bit stream can be decoded unambiguously without separator characters: just
walk the tree bit by bit until you hit a leaf.

============================================================
Canonical codes (DEFLATE / zlib style)
============================================================

The standard tree-walk produces valid codes, but different tree shapes can
produce different codes for the same symbol lengths.  Canonical codes
normalise this: given only the code *lengths*, you can reconstruct the exact
canonical code table without transmitting the tree structure.

Algorithm:
  1. Collect (symbol, code_length) pairs from the tree.
  2. Sort by (code_length, symbol_value).
  3. Assign codes numerically:
       code[0] = 0 (left-padded to length[0] bits)
       code[i] = (code[i-1] + 1) << (length[i] - length[i-1])

This is exactly what DEFLATE uses: the compressed stream contains only the
length table, not the tree, saving space.

Example with AAABBC:
  A: weight=3, B: weight=2, C: weight=1
  Tree:      [6]
             / \\
            A   [3]
           (3)  / \\
               B   C
              (2) (1)
  Lengths: A=1, B=2, C=2
  Sorted by (length, symbol): A(1), B(2), C(2)
  Canonical codes:
    A → 0        (length 1,  code = 0)
    B → 10       (length 2,  code = 0+1=1, shifted 1 bit → 10)
    C → 11       (length 2,  code = 10+1 = 11)
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Optional, Union

from heap import MinHeap


# ─── Node types ──────────────────────────────────────────────────────────────

@dataclass(frozen=True)
class Leaf:
    """A leaf node representing a single symbol."""
    symbol: int
    weight: int


@dataclass(frozen=True)
class Internal:
    """An internal node combining two sub-trees."""
    weight: int
    left: "Node"
    right: "Node"
    # Insertion order for tie-breaking among internal nodes.
    # Not part of the logical tree; used only by the comparator.
    _order: int = 0


Node = Union[Leaf, Internal]


def _node_weight(node: Node) -> int:
    return node.weight


def _node_priority(node: Node) -> tuple[int, int, int, int]:
    """
    Returns a 4-tuple used as the heap key (lower = higher priority).

    Fields:
      [0] weight         — lower weight wins
      [1] is_internal    — 0=leaf (higher priority), 1=internal
      [2] symbol_or_neg1 — leaf: symbol value; internal: -1 (not used)
      [3] order          — internal: insertion order (FIFO); leaf: -1
    """
    if isinstance(node, Leaf):
        return (node.weight, 0, node.symbol, -1)
    else:
        return (node.weight, 1, -1, node._order)


# ─── HuffmanTree ─────────────────────────────────────────────────────────────

class HuffmanTree:
    """
    A full binary tree that assigns optimal prefix-free bit codes to symbols.

    Build the tree once from symbol frequencies; then:
      - Use ``code_table()`` to get a ``{symbol → bit_string}`` map for encoding.
      - Use ``decode_all()`` to decode a bit stream back to symbols.
      - Use ``canonical_code_table()`` for DEFLATE-style transmissible codes.

    All symbols are integers (typically 0..255 for byte-level coding, but any
    non-negative integer is valid).  Frequencies must be positive integers.

    The tree is immutable after construction.  Build a new tree if frequencies
    change.

    Example::

        >>> tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
        >>> table = tree.code_table()
        >>> table[65]  # 'A' gets the shortest code
        '0'
        >>> tree.decode_all('0', 1)
        [65]
    """

    def __init__(self, root: Node, _symbol_count: int) -> None:
        self._root = root
        self._symbol_count = _symbol_count

    @classmethod
    def build(cls, weights: list[tuple[int, int]]) -> "HuffmanTree":
        """
        Construct a Huffman tree from ``(symbol, frequency)`` pairs.

        The greedy algorithm uses a min-heap.  At each step it pops the two
        lowest-weight nodes, combines them into a new internal node, and pushes
        the internal node back.  The single remaining node is the root.

        Tie-breaking (for deterministic output across implementations):
          1. Lowest weight pops first.
          2. Leaves before internal nodes at equal weight.
          3. Lower symbol value wins among leaves of equal weight.
          4. Earlier-created internal node wins among internal nodes of equal
             weight (FIFO insertion order).

        Args:
            weights: A list of ``(symbol, frequency)`` pairs.  Each symbol
                     must be a non-negative integer; each frequency must be > 0.

        Returns:
            A ``HuffmanTree`` instance ready for encoding/decoding.

        Raises:
            ValueError: If ``weights`` is empty or any frequency is ≤ 0.

        Example::

            >>> tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
            >>> tree.symbol_count()
            3
        """
        if not weights:
            raise ValueError("weights must not be empty")
        for sym, freq in weights:
            if freq <= 0:
                raise ValueError(
                    f"frequency must be positive; got symbol={sym}, freq={freq}"
                )

        # Build the min-heap.  Elements are (priority_tuple, node).
        heap: MinHeap[tuple[tuple[int, int, int, int], Node]] = MinHeap()

        for sym, freq in weights:
            leaf: Node = Leaf(symbol=sym, weight=freq)
            heap.push((_node_priority(leaf), leaf))

        order_counter = 0  # monotonic counter for internal node insertion order

        while len(heap) > 1:
            _, left = heap.pop()
            _, right = heap.pop()
            combined_weight = _node_weight(left) + _node_weight(right)
            internal: Node = Internal(
                weight=combined_weight,
                left=left,
                right=right,
                _order=order_counter,
            )
            order_counter += 1
            heap.push((_node_priority(internal), internal))

        _, root = heap.pop()
        return cls(root, len(weights))

    # ─── Encoding helpers ────────────────────────────────────────────────────

    def code_table(self) -> dict[int, str]:
        """
        Return ``{symbol: bit_string}`` for all symbols in the tree.

        Left edges are ``'0'``, right edges are ``'1'``.  For a single-symbol
        tree the convention is ``{symbol: '0'}`` (one bit per occurrence).

        Time: O(n) where n = number of distinct symbols.

        Example::

            >>> tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
            >>> table = tree.code_table()
            >>> sorted(table.items())
            [(65, '0'), (66, '10'), (67, '11')]
        """
        table: dict[int, str] = {}
        _walk(self._root, "", table)
        return table

    def code_for(self, symbol: int) -> Optional[str]:
        """
        Return the bit string for a specific symbol, or ``None`` if not in
        the tree.

        Walks the tree searching for the leaf with ``symbol``; does NOT build
        the full code table.

        Time: O(n) worst case (full tree traversal).

        Example::

            >>> tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
            >>> tree.code_for(65)
            '0'
            >>> tree.code_for(99) is None
            True
        """
        return _find_code(self._root, symbol, "")

    def canonical_code_table(self) -> dict[int, str]:
        """
        Return canonical Huffman codes (DEFLATE-style).

        Sorted by ``(code_length, symbol_value)``; codes assigned numerically.
        Useful when you need to transmit only code lengths, not the tree.

        Time: O(n log n).

        Example::

            >>> tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
            >>> canonical = tree.canonical_code_table()
            >>> sorted(canonical.items())
            [(65, '0'), (66, '10'), (67, '11')]
        """
        # Step 1: collect lengths
        lengths: dict[int, int] = {}
        _collect_lengths(self._root, 0, lengths)

        # Single-leaf edge case: assign length 1 by convention
        if len(lengths) == 1:
            sym = next(iter(lengths))
            return {sym: "0"}

        # Step 2: sort by (length, symbol)
        sorted_syms = sorted(lengths.items(), key=lambda kv: (kv[1], kv[0]))

        # Step 3: assign canonical codes numerically
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

    # ─── Decoding ────────────────────────────────────────────────────────────

    def decode_all(self, bits: str, count: int) -> list[int]:
        """
        Decode exactly ``count`` symbols from a bit string by walking the tree.

        Args:
            bits:  A string of ``'0'`` and ``'1'`` characters.
            count: The exact number of symbols to decode.

        Returns:
            A list of decoded symbols of length == ``count``.

        Raises:
            ValueError: If the bit stream is exhausted before ``count``
                        symbols are decoded.

        For a single-leaf tree, each ``'0'`` bit decodes to that symbol.

        Time: O(total bits consumed).

        Example::

            >>> tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
            >>> tree.decode_all('010011', 4)
            [65, 65, 66, 67]
        """
        result: list[int] = []
        node = self._root
        i = 0
        # Single-leaf trees encode each symbol as a single '0' bit.
        # Multi-leaf trees: reaching a leaf means i is already past the last
        # consumed bit — no extra advance needed.
        single_leaf = isinstance(self._root, Leaf)

        while len(result) < count:
            if isinstance(node, Leaf):
                result.append(node.symbol)
                node = self._root
                if single_leaf:
                    # Consume the '0' bit for this symbol
                    if i < len(bits):
                        i += 1
                continue

            if i >= len(bits):
                raise ValueError(
                    f"Bit stream exhausted after {len(result)} symbols; "
                    f"expected {count}"
                )
            bit = bits[i]
            i += 1
            node = node.left if bit == "0" else node.right

        return result

    # ─── Inspection ──────────────────────────────────────────────────────────

    def weight(self) -> int:
        """
        Total weight of the tree = sum of all leaf frequencies = root weight.
        O(1) — stored at the root.

        Example::

            >>> tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
            >>> tree.weight()
            6
        """
        return self._root.weight

    def depth(self) -> int:
        """
        Maximum code length = depth of the deepest leaf.
        O(n) — must traverse the tree.

        Example::

            >>> tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
            >>> tree.depth()
            2
        """
        return _max_depth(self._root, 0)

    def symbol_count(self) -> int:
        """
        Number of distinct symbols (= number of leaf nodes).
        O(1) — stored at construction time.

        Example::

            >>> tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
            >>> tree.symbol_count()
            3
        """
        return self._symbol_count

    def leaves(self) -> list[tuple[int, str]]:
        """
        In-order traversal of leaves.

        Returns ``[(symbol, code), ...]``, left subtree before right subtree.
        Useful for visualisation and debugging.

        Time: O(n).

        Example::

            >>> tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
            >>> tree.leaves()
            [(65, '0'), (66, '10'), (67, '11')]
        """
        table = self.code_table()
        result: list[tuple[int, str]] = []
        _in_order_leaves(self._root, result, table)
        return result

    def is_valid(self) -> bool:
        """
        Check structural invariants.  For testing only.

          1. Every internal node has exactly 2 children (full binary tree).
          2. ``weight(internal) == weight(left) + weight(right)``.
          3. No symbol appears in more than one leaf.

        Returns ``True`` if all invariants hold.

        Example::

            >>> tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
            >>> tree.is_valid()
            True
        """
        seen: set[int] = set()
        return _check_invariants(self._root, seen)


# ─── Private helpers ─────────────────────────────────────────────────────────

def _walk(node: Node, prefix: str, table: dict[int, str]) -> None:
    """Recursively walk the tree building the code table."""
    if isinstance(node, Leaf):
        # Single-leaf edge case: no edges traversed; use "0" by convention
        table[node.symbol] = prefix if prefix else "0"
        return
    _walk(node.left,  prefix + "0", table)
    _walk(node.right, prefix + "1", table)


def _find_code(node: Node, symbol: int, prefix: str) -> Optional[str]:
    """Search the tree for a specific symbol, returning its code or None."""
    if isinstance(node, Leaf):
        if node.symbol == symbol:
            return prefix if prefix else "0"
        return None
    left_result = _find_code(node.left,  symbol, prefix + "0")
    if left_result is not None:
        return left_result
    return _find_code(node.right, symbol, prefix + "1")


def _collect_lengths(node: Node, d: int, lengths: dict[int, int]) -> None:
    """Collect code lengths for all leaves."""
    if isinstance(node, Leaf):
        lengths[node.symbol] = d if d > 0 else 1  # single-leaf: depth=0, length=1
        return
    _collect_lengths(node.left,  d + 1, lengths)
    _collect_lengths(node.right, d + 1, lengths)


def _max_depth(node: Node, d: int) -> int:
    """Return the maximum depth of any leaf."""
    if isinstance(node, Leaf):
        return d
    return max(_max_depth(node.left, d + 1), _max_depth(node.right, d + 1))


def _in_order_leaves(
    node: Node, result: list[tuple[int, str]], table: dict[int, str]
) -> None:
    """Collect leaves in left-to-right (in-order) traversal."""
    if isinstance(node, Leaf):
        result.append((node.symbol, table[node.symbol]))
        return
    _in_order_leaves(node.left,  result, table)
    _in_order_leaves(node.right, result, table)


def _check_invariants(node: Node, seen: set[int]) -> bool:
    """Recursively validate tree invariants."""
    if isinstance(node, Leaf):
        if node.symbol in seen:
            return False
        seen.add(node.symbol)
        return True
    # Internal node: check weight sum
    if node.weight != node.left.weight + node.right.weight:
        return False
    return _check_invariants(node.left, seen) and _check_invariants(node.right, seen)
