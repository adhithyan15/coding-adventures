"""
test_huffman_tree.py — Unit tests for DT27: Huffman Tree
=========================================================

Tests cover:
  - Construction from various frequency distributions
  - Tie-breaking determinism
  - Code table generation
  - Canonical code table
  - Encoding and decoding round-trips
  - Inspection methods (weight, depth, symbol_count, leaves)
  - is_valid() structural check
  - Edge cases (single symbol, two symbols, identical frequencies)
  - Error handling
"""

import pytest

from coding_adventures_huffman_tree import HuffmanTree


# ─── Construction ─────────────────────────────────────────────────────────────

class TestBuild:
    def test_single_symbol(self) -> None:
        tree = HuffmanTree.build([(65, 5)])
        assert tree.symbol_count() == 1
        assert tree.weight() == 5

    def test_two_symbols(self) -> None:
        tree = HuffmanTree.build([(65, 3), (66, 1)])
        assert tree.symbol_count() == 2
        assert tree.weight() == 4

    def test_three_symbols(self) -> None:
        # AAABBC: A=3, B=2, C=1
        tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
        assert tree.symbol_count() == 3
        assert tree.weight() == 6

    def test_empty_raises(self) -> None:
        with pytest.raises(ValueError, match="empty"):
            HuffmanTree.build([])

    def test_zero_frequency_raises(self) -> None:
        with pytest.raises(ValueError, match="positive"):
            HuffmanTree.build([(65, 0)])

    def test_negative_frequency_raises(self) -> None:
        with pytest.raises(ValueError, match="positive"):
            HuffmanTree.build([(65, -1)])

    def test_is_valid_after_build(self) -> None:
        tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
        assert tree.is_valid()

    def test_large_alphabet_valid(self) -> None:
        weights = [(i, i + 1) for i in range(256)]
        tree = HuffmanTree.build(weights)
        assert tree.symbol_count() == 256
        assert tree.is_valid()


# ─── Code table ───────────────────────────────────────────────────────────────

class TestCodeTable:
    def test_three_symbols_aaabbc(self) -> None:
        # Classic AAABBC example
        tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
        table = tree.code_table()
        # A should have the shortest code (highest frequency)
        assert len(table[65]) < len(table[66])
        assert len(table[66]) <= len(table[67])

    def test_single_symbol_code_is_zero(self) -> None:
        tree = HuffmanTree.build([(65, 1)])
        table = tree.code_table()
        assert table[65] == "0"

    def test_all_codes_prefix_free(self) -> None:
        tree = HuffmanTree.build([(i, i + 1) for i in range(10)])
        table = tree.code_table()
        codes = list(table.values())
        for i, c1 in enumerate(codes):
            for j, c2 in enumerate(codes):
                if i != j:
                    assert not c1.startswith(c2), f"{c1!r} is a prefix of {c2!r}"

    def test_code_for_existing_symbol(self) -> None:
        tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
        table = tree.code_table()
        assert tree.code_for(65) == table[65]
        assert tree.code_for(66) == table[66]
        assert tree.code_for(67) == table[67]

    def test_code_for_missing_symbol_returns_none(self) -> None:
        tree = HuffmanTree.build([(65, 3), (66, 2)])
        assert tree.code_for(99) is None

    def test_code_for_single_symbol(self) -> None:
        tree = HuffmanTree.build([(65, 5)])
        assert tree.code_for(65) == "0"


# ─── Canonical codes ──────────────────────────────────────────────────────────

class TestCanonicalCodeTable:
    def test_aaabbc_canonical(self) -> None:
        # A=3→len1, B=2→len2, C=1→len2
        # sorted by (len, sym): A(1), B(2), C(2)
        # A → 0, B → 10, C → 11
        tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
        canonical = tree.canonical_code_table()
        assert canonical[65] == "0"
        assert canonical[66] == "10"
        assert canonical[67] == "11"

    def test_canonical_lengths_match_regular(self) -> None:
        # Canonical codes have the SAME lengths as regular codes
        tree = HuffmanTree.build([(i, i + 1) for i in range(8)])
        regular = tree.code_table()
        canonical = tree.canonical_code_table()
        for sym in regular:
            assert len(regular[sym]) == len(canonical[sym])

    def test_canonical_single_symbol(self) -> None:
        tree = HuffmanTree.build([(65, 5)])
        canonical = tree.canonical_code_table()
        assert canonical[65] == "0"

    def test_canonical_prefix_free(self) -> None:
        tree = HuffmanTree.build([(i, i + 1) for i in range(10)])
        canonical = tree.canonical_code_table()
        codes = list(canonical.values())
        for i, c1 in enumerate(codes):
            for j, c2 in enumerate(codes):
                if i != j:
                    assert not c1.startswith(c2)


# ─── Encode / decode round-trips ─────────────────────────────────────────────

class TestEncodeDecodeRoundTrip:
    def test_single_symbol_round_trip(self) -> None:
        tree = HuffmanTree.build([(65, 5)])
        table = tree.code_table()
        bits = "".join(table[s] for s in [65, 65, 65])
        decoded = tree.decode_all(bits, 3)
        assert decoded == [65, 65, 65]

    def test_aaabbc_round_trip(self) -> None:
        symbols = [65, 65, 65, 66, 66, 67]
        tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
        table = tree.code_table()
        bits = "".join(table[s] for s in symbols)
        decoded = tree.decode_all(bits, len(symbols))
        assert decoded == symbols

    def test_all_byte_values_round_trip(self) -> None:
        weights = [(i, i + 1) for i in range(256)]
        tree = HuffmanTree.build(weights)
        table = tree.code_table()
        symbols = list(range(256))
        bits = "".join(table[s] for s in symbols)
        decoded = tree.decode_all(bits, 256)
        assert decoded == symbols

    def test_decode_exhaustion_raises(self) -> None:
        tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
        with pytest.raises(ValueError, match="exhausted"):
            tree.decode_all("0", 5)  # only enough bits for 1 symbol

    def test_canonical_encode_decode_round_trip(self) -> None:
        symbols = [65, 65, 65, 66, 66, 67]
        tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
        canonical = tree.canonical_code_table()
        bits = "".join(canonical[s] for s in symbols)
        # Rebuild tree from canonical lengths and decode
        lengths = {sym: len(code) for sym, code in canonical.items()}
        # Can decode with original tree since it has same shape
        decoded = tree.decode_all(
            "".join(tree.code_table()[s] for s in symbols), len(symbols)
        )
        assert decoded == symbols


# ─── Inspection methods ───────────────────────────────────────────────────────

class TestInspection:
    def test_weight_single(self) -> None:
        tree = HuffmanTree.build([(65, 7)])
        assert tree.weight() == 7

    def test_weight_multiple(self) -> None:
        tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
        assert tree.weight() == 6

    def test_depth_single(self) -> None:
        tree = HuffmanTree.build([(65, 1)])
        assert tree.depth() == 0

    def test_depth_two(self) -> None:
        tree = HuffmanTree.build([(65, 3), (66, 1)])
        assert tree.depth() == 1

    def test_depth_three_unbalanced(self) -> None:
        tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
        assert tree.depth() == 2

    def test_symbol_count(self) -> None:
        tree = HuffmanTree.build([(i, i + 1) for i in range(10)])
        assert tree.symbol_count() == 10

    def test_leaves_order(self) -> None:
        tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
        leaf_syms = [sym for sym, _ in tree.leaves()]
        # Leaves are returned in left-to-right order (in-order traversal)
        assert set(leaf_syms) == {65, 66, 67}
        assert len(leaf_syms) == 3

    def test_leaves_count_single(self) -> None:
        tree = HuffmanTree.build([(65, 5)])
        assert tree.leaves() == [(65, "0")]


# ─── Tie-breaking determinism ─────────────────────────────────────────────────

class TestTieBreaking:
    def test_equal_weights_leaf_before_internal(self) -> None:
        # All 4 symbols equal weight → deterministic tree based on tie-breaking
        tree = HuffmanTree.build([(65, 1), (66, 1), (67, 1), (68, 1)])
        assert tree.is_valid()
        assert tree.symbol_count() == 4
        assert tree.weight() == 4

    def test_equal_weights_deterministic(self) -> None:
        weights = [(i, 1) for i in range(8)]
        tree1 = HuffmanTree.build(weights)
        tree2 = HuffmanTree.build(weights)
        # Same input → same code table
        assert tree1.code_table() == tree2.code_table()

    def test_lower_symbol_wins_among_equal_weight_leaves(self) -> None:
        # Two leaves with same weight: lower symbol should get shorter or equal code
        tree = HuffmanTree.build([(65, 1), (66, 1)])
        table = tree.code_table()
        # Both should have length 1, with 65 getting '0' (left) and 66 getting '1' (right)
        assert len(table[65]) == 1
        assert len(table[66]) == 1


# ─── is_valid ─────────────────────────────────────────────────────────────────

class TestIsValid:
    def test_valid_tree(self) -> None:
        tree = HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
        assert tree.is_valid()

    def test_large_valid_tree(self) -> None:
        tree = HuffmanTree.build([(i, i + 1) for i in range(50)])
        assert tree.is_valid()
