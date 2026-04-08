"""
test_radix_tree.py — Comprehensive tests for the RadixTree implementation.

Test strategy:
  - Each public method gets its own TestCase class.
  - Edge cases: empty tree, single key, overlapping prefixes, empty string key.
  - Structural invariants: compression ratio, edge-splitting correctness.
  - Property test: insert 100 random keys, verify all found, delete half,
    verify deletions are clean.
  - Coverage target: 95%+
"""

from __future__ import annotations

import random
import string

import pytest

from radix_tree import RadixNode, RadixTree


# ─── Helpers ──────────────────────────────────────────────────────────────────


def _count_nodes(node: RadixNode) -> int:
    """Recursively count all nodes in the subtree (including node itself)."""
    return 1 + sum(_count_nodes(child) for _, child in node.children.values())


def _tree_with(*keys: str) -> RadixTree[int]:
    """Build a RadixTree[int] from the given keys (value = index + 1)."""
    t: RadixTree[int] = RadixTree()
    for i, k in enumerate(keys):
        t.insert(k, i + 1)
    return t


# ─── Insert ───────────────────────────────────────────────────────────────────


class TestRadixTreeInsert:
    """Tests for RadixTree.insert()."""

    def test_insert_single_key(self) -> None:
        t = _tree_with("hello")
        assert t.search("hello") == 1
        assert len(t) == 1

    def test_insert_empty_string_key(self) -> None:
        t: RadixTree[str] = RadixTree()
        t.insert("", "empty")
        assert t.search("") == "empty"
        assert len(t) == 1

    def test_insert_updates_value_for_duplicate_key(self) -> None:
        t: RadixTree[int] = RadixTree()
        t.insert("apple", 1)
        t.insert("apple", 99)
        assert t.search("apple") == 99
        assert len(t) == 1  # still only one key

    def test_insert_keys_sharing_prefix_case2(self) -> None:
        """Case 2: new key extends an existing edge (descend)."""
        t = _tree_with("app", "apple")
        assert t.search("app") == 1
        assert t.search("apple") == 2
        assert len(t) == 2

    def test_insert_prefix_of_existing_key_case3(self) -> None:
        """Case 3: new key is a prefix of an existing edge (split)."""
        t: RadixTree[int] = RadixTree()
        t.insert("apple", 1)
        t.insert("app", 2)
        assert t.search("apple") == 1
        assert t.search("app") == 2
        assert len(t) == 2

    def test_insert_partial_overlap_case4(self) -> None:
        """Case 4: both new key and existing edge diverge (split)."""
        t: RadixTree[int] = RadixTree()
        t.insert("application", 1)
        t.insert("apple", 2)
        assert t.search("application") == 1
        assert t.search("apple") == 2
        # "appl" itself is not a key
        assert t.search("appl") is None

    def test_insert_no_common_prefix_case1(self) -> None:
        """Case 1: new key shares no prefix with any existing edge."""
        t = _tree_with("apple", "banana")
        assert t.search("apple") == 1
        assert t.search("banana") == 2

    def test_insert_many_keys_len_accurate(self) -> None:
        keys = ["a", "ab", "abc", "abcd", "b", "bc", "xyz"]
        t: RadixTree[int] = RadixTree()
        for i, k in enumerate(keys):
            t.insert(k, i)
        assert len(t) == len(keys)

    def test_insert_search_word_searcher_searching(self) -> None:
        """The canonical radix tree example from the spec."""
        t = _tree_with("search", "searcher", "searching")
        assert t.search("search") == 1
        assert t.search("searcher") == 2
        assert t.search("searching") == 3

    def test_insert_duplicate_does_not_increase_size(self) -> None:
        t: RadixTree[int] = RadixTree()
        t.insert("foo", 1)
        t.insert("foo", 2)
        t.insert("foo", 3)
        assert len(t) == 1


# ─── Search ───────────────────────────────────────────────────────────────────


class TestRadixTreeSearch:
    """Tests for RadixTree.search()."""

    def test_search_existing_key(self) -> None:
        t = _tree_with("hello")
        assert t.search("hello") == 1

    def test_search_missing_key_returns_none(self) -> None:
        t = _tree_with("hello")
        assert t.search("world") is None

    def test_search_prefix_only_not_a_key(self) -> None:
        """A path prefix that is NOT marked as a key must return None."""
        t: RadixTree[int] = RadixTree()
        t.insert("apple", 1)
        assert t.search("app") is None
        assert t.search("appl") is None
        assert t.search("a") is None

    def test_search_extension_of_existing_key(self) -> None:
        t = _tree_with("app")
        assert t.search("apple") is None

    def test_search_empty_tree(self) -> None:
        t: RadixTree[int] = RadixTree()
        assert t.search("anything") is None

    def test_search_empty_key(self) -> None:
        t: RadixTree[int] = RadixTree()
        t.insert("", 42)
        assert t.search("") == 42

    def test_search_empty_key_not_present(self) -> None:
        t = _tree_with("apple")
        assert t.search("") is None

    def test_search_multiple_keys_independent(self) -> None:
        t = _tree_with("foo", "bar", "baz")
        assert t.search("foo") == 1
        assert t.search("bar") == 2
        assert t.search("baz") == 3
        assert t.search("ba") is None


# ─── Delete ───────────────────────────────────────────────────────────────────


class TestRadixTreeDelete:
    """Tests for RadixTree.delete()."""

    def test_delete_existing_key(self) -> None:
        t = _tree_with("apple")
        assert t.delete("apple") is True
        assert t.search("apple") is None
        assert len(t) == 0

    def test_delete_non_existent_key(self) -> None:
        t = _tree_with("apple")
        assert t.delete("banana") is False
        assert len(t) == 1

    def test_delete_prefix_not_a_key(self) -> None:
        """Deleting a prefix that was never inserted returns False."""
        t: RadixTree[int] = RadixTree()
        t.insert("apple", 1)
        assert t.delete("app") is False
        assert t.search("apple") == 1

    def test_delete_key_with_sibling_prefix(self) -> None:
        """Deleting one key must not affect another that shares a prefix."""
        t = _tree_with("app", "apple")
        assert t.delete("apple") is True
        assert t.search("app") == 1
        assert t.search("apple") is None
        assert len(t) == 1

    def test_delete_key_triggers_merge(self) -> None:
        """
        After deleting "app", the "app" node becomes a non-endpoint with one
        child ("le"). It should be merged with "le" into "apple".
        """
        t: RadixTree[int] = RadixTree()
        t.insert("app", 1)
        t.insert("apple", 2)
        assert t.delete("app") is True
        assert t.search("apple") == 2
        assert t.search("app") is None

    def test_delete_last_key_in_subtree(self) -> None:
        t = _tree_with("search", "searcher")
        t.delete("searcher")
        assert t.search("search") == 1
        assert t.search("searcher") is None

    def test_delete_all_keys(self) -> None:
        t = _tree_with("a", "b", "c")
        for k in ["a", "b", "c"]:
            t.delete(k)
        assert len(t) == 0

    def test_delete_decrements_size(self) -> None:
        t = _tree_with("foo", "bar")
        t.delete("foo")
        assert len(t) == 1

    def test_delete_empty_key(self) -> None:
        t: RadixTree[int] = RadixTree()
        t.insert("", 1)
        assert t.delete("") is True
        assert t.search("") is None
        assert len(t) == 0

    def test_delete_from_empty_tree(self) -> None:
        t: RadixTree[int] = RadixTree()
        assert t.delete("anything") is False


# ─── starts_with ──────────────────────────────────────────────────────────────


class TestRadixTreeStartsWith:
    """Tests for RadixTree.starts_with()."""

    def test_starts_with_exact_key(self) -> None:
        t = _tree_with("apple")
        assert t.starts_with("apple") is True

    def test_starts_with_prefix_of_key(self) -> None:
        t = _tree_with("apple")
        assert t.starts_with("app") is True
        assert t.starts_with("a") is True
        assert t.starts_with("appl") is True

    def test_starts_with_prefix_not_present(self) -> None:
        t = _tree_with("apple")
        assert t.starts_with("apz") is False
        assert t.starts_with("b") is False

    def test_starts_with_empty_prefix_non_empty_tree(self) -> None:
        t = _tree_with("anything")
        assert t.starts_with("") is True

    def test_starts_with_empty_prefix_empty_tree(self) -> None:
        t: RadixTree[int] = RadixTree()
        assert t.starts_with("") is False

    def test_starts_with_extension_of_all_keys(self) -> None:
        """Prefix that extends beyond any stored key — no key starts with it."""
        t = _tree_with("app")
        assert t.starts_with("apple") is False

    def test_starts_with_mid_edge(self) -> None:
        """Prefix that ends in the middle of an edge label still returns True."""
        t: RadixTree[int] = RadixTree()
        t.insert("searching", 1)
        # "search" is in the middle of the "searching" edge from root.
        assert t.starts_with("sear") is True
        assert t.starts_with("search") is True
        assert t.starts_with("searchin") is True


# ─── words_with_prefix ────────────────────────────────────────────────────────


class TestWordsWithPrefix:
    """Tests for RadixTree.words_with_prefix()."""

    def test_no_matches(self) -> None:
        t = _tree_with("apple", "application")
        assert t.words_with_prefix("xyz") == []

    def test_exact_key_match(self) -> None:
        t = _tree_with("apple")
        assert t.words_with_prefix("apple") == ["apple"]

    def test_multiple_matches(self) -> None:
        t = _tree_with("search", "searcher", "searching")
        result = t.words_with_prefix("search")
        assert result == ["search", "searcher", "searching"]

    def test_empty_prefix_returns_all(self) -> None:
        t = _tree_with("banana", "apple", "application")
        result = t.words_with_prefix("")
        # Should be sorted lexicographically
        assert result == sorted(["banana", "apple", "application"])

    def test_prefix_matches_subset(self) -> None:
        t = _tree_with("app", "apple", "application", "banana")
        result = t.words_with_prefix("app")
        assert result == ["app", "apple", "application"]

    def test_prefix_mid_edge(self) -> None:
        """Prefix that ends in the middle of an edge label."""
        t: RadixTree[int] = RadixTree()
        t.insert("searching", 1)
        result = t.words_with_prefix("sear")
        assert result == ["searching"]

    def test_empty_tree(self) -> None:
        t: RadixTree[int] = RadixTree()
        assert t.words_with_prefix("x") == []

    def test_sorted_output(self) -> None:
        t = _tree_with("bbb", "aaa", "aab", "baa")
        result = t.words_with_prefix("")
        assert result == sorted(["bbb", "aaa", "aab", "baa"])

    def test_single_char_prefix(self) -> None:
        t = _tree_with("apple", "ant", "banana")
        result = t.words_with_prefix("a")
        assert result == ["ant", "apple"]


# ─── longest_prefix_match ─────────────────────────────────────────────────────


class TestLongestPrefixMatch:
    """Tests for RadixTree.longest_prefix_match()."""

    def test_full_key_match(self) -> None:
        t = _tree_with("abc")
        assert t.longest_prefix_match("abc") == "abc"

    def test_partial_match_stops_at_longest(self) -> None:
        t: RadixTree[int] = RadixTree()
        t.insert("a", 1)
        t.insert("ab", 2)
        t.insert("abc", 3)
        assert t.longest_prefix_match("abcdef") == "abc"

    def test_no_match(self) -> None:
        t = _tree_with("apple")
        assert t.longest_prefix_match("xyz") is None

    def test_partial_match_shorter_key(self) -> None:
        t: RadixTree[int] = RadixTree()
        t.insert("a", 1)
        t.insert("ab", 2)
        assert t.longest_prefix_match("abc") == "ab"

    def test_exact_match_is_the_prefix(self) -> None:
        t: RadixTree[int] = RadixTree()
        t.insert("a", 1)
        assert t.longest_prefix_match("a") == "a"

    def test_empty_key_stored(self) -> None:
        t: RadixTree[int] = RadixTree()
        t.insert("", 1)
        t.insert("a", 2)
        assert t.longest_prefix_match("xyz") == ""

    def test_no_prefix_at_all(self) -> None:
        t: RadixTree[int] = RadixTree()
        t.insert("abc", 1)
        assert t.longest_prefix_match("xyz") is None

    def test_longest_among_multiple_candidates(self) -> None:
        t: RadixTree[int] = RadixTree()
        t.insert("app", 1)
        t.insert("apple", 2)
        t.insert("application", 3)
        assert t.longest_prefix_match("application") == "application"
        assert t.longest_prefix_match("applications") == "application"
        assert t.longest_prefix_match("apple") == "apple"
        assert t.longest_prefix_match("apples") == "apple"
        assert t.longest_prefix_match("app") == "app"
        assert t.longest_prefix_match("apply") == "app"


# ─── Compression invariant ────────────────────────────────────────────────────


class TestRadixTreeCompression:
    """
    Verify that the radix tree actually compresses nodes.

    A naive trie for ["search", "searcher", "searching"] would use 14 nodes.
    The radix tree should use only 4: root + "search" + "er" + "ing".
    """

    def test_compression_search_words(self) -> None:
        t = _tree_with("search", "searcher", "searching")
        # root(1) + "search" node(1) + "er" node(1) + "ing" node(1) = 4
        node_count = _count_nodes(t._root)
        assert node_count == 4, f"Expected 4 nodes, got {node_count}"

    def test_root_has_one_child_for_common_prefix(self) -> None:
        t = _tree_with("search", "searcher", "searching")
        # All three words share "search" — root should have exactly 1 child.
        assert len(t._root.children) == 1

    def test_compression_single_long_key(self) -> None:
        """A single long key should produce exactly 2 nodes: root + leaf."""
        t: RadixTree[int] = RadixTree()
        t.insert("superlongkeyhere", 1)
        assert _count_nodes(t._root) == 2

    def test_compression_after_delete_merge(self) -> None:
        """After deleting "app", "apple" node should merge back with root child."""
        t: RadixTree[int] = RadixTree()
        t.insert("app", 1)
        t.insert("apple", 2)
        # 3 nodes: root + "app" inner + "le" leaf
        assert _count_nodes(t._root) == 3
        t.delete("app")
        # 2 nodes: root + "apple" leaf (merged)
        assert _count_nodes(t._root) == 2


# ─── Edge splitting ───────────────────────────────────────────────────────────


class TestEdgeSplitting:
    """
    Verify correctness of the four insertion cases, especially splits.

    The edge split is the hardest part of radix tree insertion. A bug in
    splitting causes previously inserted keys to become unreachable.
    """

    def test_split_case3_apple_then_app(self) -> None:
        """Insert 'apple', then 'app' — Case 3 split."""
        t: RadixTree[int] = RadixTree()
        t.insert("apple", 1)
        t.insert("app", 2)
        assert t.search("apple") == 1
        assert t.search("app") == 2
        assert t.search("ap") is None

    def test_split_case4_application_then_apple(self) -> None:
        """Insert 'application', then 'apple' — Case 4 split."""
        t: RadixTree[int] = RadixTree()
        t.insert("application", 1)
        t.insert("apple", 2)
        assert t.search("application") == 1
        assert t.search("apple") == 2
        assert t.search("appl") is None
        assert t.search("appi") is None

    def test_split_preserves_all_existing_keys(self) -> None:
        """Each split must keep all previously-inserted keys accessible."""
        keys = ["apple", "app", "application", "apt", "apply"]
        t: RadixTree[int] = RadixTree()
        for i, k in enumerate(keys):
            t.insert(k, i)
            # Verify ALL previously inserted keys still accessible.
            for j in range(i + 1):
                assert t.search(keys[j]) == j, (
                    f"After inserting '{k}', key '{keys[j]}' became inaccessible"
                )

    def test_split_case3_then_further_descent(self) -> None:
        """
        After splitting "application" into "app" + "lication", insert "apple"
        which must navigate "app" and then split "lication".
        """
        t: RadixTree[int] = RadixTree()
        t.insert("application", 1)
        t.insert("app", 2)
        t.insert("apple", 3)
        assert t.search("application") == 1
        assert t.search("app") == 2
        assert t.search("apple") == 3

    def test_split_single_char_difference(self) -> None:
        """Keys that differ in only the last character."""
        t: RadixTree[int] = RadixTree()
        t.insert("abc", 1)
        t.insert("abd", 2)
        assert t.search("abc") == 1
        assert t.search("abd") == 2
        assert t.search("ab") is None


# ─── __iter__ and __len__ ─────────────────────────────────────────────────────


class TestIterAndLen:
    """Tests for __iter__, __len__, and __contains__."""

    def test_len_empty(self) -> None:
        t: RadixTree[int] = RadixTree()
        assert len(t) == 0

    def test_len_after_inserts(self) -> None:
        t = _tree_with("a", "b", "c", "d")
        assert len(t) == 4

    def test_iter_sorted_order(self) -> None:
        t = _tree_with("banana", "apple", "cherry", "apricot")
        result = list(t)
        assert result == sorted(["banana", "apple", "cherry", "apricot"])

    def test_iter_empty(self) -> None:
        t: RadixTree[int] = RadixTree()
        assert list(t) == []

    def test_contains_present(self) -> None:
        t = _tree_with("hello")
        assert "hello" in t

    def test_contains_absent(self) -> None:
        t = _tree_with("hello")
        assert "world" not in t

    def test_contains_prefix_only(self) -> None:
        t: RadixTree[int] = RadixTree()
        t.insert("hello", 1)
        assert "hel" not in t

    def test_contains_non_string(self) -> None:
        t = _tree_with("foo")
        assert 42 not in t  # type: ignore[operator]
        assert None not in t  # type: ignore[operator]

    def test_iter_with_common_prefixes(self) -> None:
        t = _tree_with("app", "apple", "application", "apt")
        result = list(t)
        assert result == ["app", "apple", "application", "apt"]


# ─── to_dict ──────────────────────────────────────────────────────────────────


class TestToDict:
    """Tests for RadixTree.to_dict()."""

    def test_to_dict_basic(self) -> None:
        t: RadixTree[int] = RadixTree()
        t.insert("foo", 1)
        t.insert("bar", 2)
        assert t.to_dict() == {"foo": 1, "bar": 2}

    def test_to_dict_empty(self) -> None:
        t: RadixTree[int] = RadixTree()
        assert t.to_dict() == {}

    def test_to_dict_after_delete(self) -> None:
        t: RadixTree[int] = RadixTree()
        t.insert("a", 1)
        t.insert("b", 2)
        t.delete("a")
        assert t.to_dict() == {"b": 2}

    def test_to_dict_values_accurate(self) -> None:
        keys = ["search", "searcher", "searching"]
        t: RadixTree[int] = RadixTree()
        for i, k in enumerate(keys):
            t.insert(k, i * 10)
        d = t.to_dict()
        assert d == {"search": 0, "searcher": 10, "searching": 20}

    def test_to_dict_round_trip(self) -> None:
        """Insert from dict, export to dict, should be identical."""
        original = {"alpha": 1, "beta": 2, "gamma": 3, "delta": 4}
        t: RadixTree[int] = RadixTree()
        for k, v in original.items():
            t.insert(k, v)
        assert t.to_dict() == original


# ─── __repr__ ─────────────────────────────────────────────────────────────────


class TestRepr:
    """Tests for RadixTree.__repr__()."""

    def test_repr_empty(self) -> None:
        t: RadixTree[int] = RadixTree()
        r = repr(t)
        assert "0 keys" in r

    def test_repr_few_keys(self) -> None:
        t = _tree_with("a", "b", "c")
        r = repr(t)
        assert "3 keys" in r

    def test_repr_many_keys_truncated(self) -> None:
        t: RadixTree[int] = RadixTree()
        for i in range(10):
            t.insert(str(i), i)
        r = repr(t)
        assert "...+" in r


# ─── Property / randomised test ───────────────────────────────────────────────


class TestRandomProperty:
    """
    Property-based tests using random data.

    These tests use a fixed seed for reproducibility, but cover large numbers
    of random inputs to expose edge cases that targeted tests might miss.
    """

    def _random_keys(self, n: int, seed: int = 42) -> list[str]:
        rng = random.Random(seed)
        alphabet = string.ascii_lowercase
        return [
            "".join(rng.choices(alphabet, k=rng.randint(1, 12))) for _ in range(n)
        ]

    def test_insert_100_random_keys_all_found(self) -> None:
        keys = self._random_keys(100)
        unique_keys = list(dict.fromkeys(keys))  # deduplicate, preserve order

        t: RadixTree[int] = RadixTree()
        for i, k in enumerate(unique_keys):
            t.insert(k, i)

        for i, k in enumerate(unique_keys):
            assert t.search(k) == i, f"Key '{k}' not found after insert"

        assert len(t) == len(unique_keys)

    def test_delete_half_of_random_keys(self) -> None:
        keys = self._random_keys(100, seed=7)
        unique_keys = list(dict.fromkeys(keys))

        t: RadixTree[int] = RadixTree()
        for i, k in enumerate(unique_keys):
            t.insert(k, i)

        to_delete = unique_keys[: len(unique_keys) // 2]
        to_keep = unique_keys[len(unique_keys) // 2 :]

        for k in to_delete:
            result = t.delete(k)
            assert result is True, f"delete('{k}') returned False"

        for k in to_delete:
            assert t.search(k) is None, f"Deleted key '{k}' still found"

        for i_offset, k in enumerate(to_keep):
            idx = len(unique_keys) // 2 + i_offset
            assert t.search(k) == idx, f"Key '{k}' lost after deleting others"

        assert len(t) == len(to_keep)

    def test_iter_matches_sorted_insert_order(self) -> None:
        keys = self._random_keys(50, seed=13)
        unique_keys = list(dict.fromkeys(keys))

        t: RadixTree[int] = RadixTree()
        for i, k in enumerate(unique_keys):
            t.insert(k, i)

        result = list(t)
        assert result == sorted(unique_keys)

    def test_to_dict_matches_direct_lookup(self) -> None:
        keys = self._random_keys(60, seed=17)
        unique_keys = list(dict.fromkeys(keys))

        t: RadixTree[int] = RadixTree()
        expected: dict[str, int] = {}
        for i, k in enumerate(unique_keys):
            t.insert(k, i)
            expected[k] = i

        assert t.to_dict() == expected

    def test_words_with_prefix_consistency(self) -> None:
        """
        words_with_prefix("") must return the same set as list(t).
        """
        keys = self._random_keys(40, seed=21)
        unique_keys = list(dict.fromkeys(keys))

        t: RadixTree[int] = RadixTree()
        for i, k in enumerate(unique_keys):
            t.insert(k, i)

        all_via_iter = sorted(list(t))
        all_via_prefix = t.words_with_prefix("")
        assert all_via_prefix == all_via_iter

    def test_starts_with_consistent_with_words_with_prefix(self) -> None:
        """starts_with(p) must be True iff words_with_prefix(p) is non-empty."""
        keys = self._random_keys(30, seed=31)
        unique_keys = list(dict.fromkeys(keys))

        t: RadixTree[int] = RadixTree()
        for i, k in enumerate(unique_keys):
            t.insert(k, i)

        prefixes = [k[:i] for k in unique_keys for i in range(len(k) + 1)]
        # Also test some random strings not necessarily in tree.
        rng = random.Random(31)
        prefixes += ["".join(rng.choices(string.ascii_lowercase, k=3)) for _ in range(20)]

        for p in prefixes:
            sw = t.starts_with(p)
            wwp = t.words_with_prefix(p)
            assert sw == (len(wwp) > 0), (
                f"starts_with({p!r})={sw} but words_with_prefix returned {wwp}"
            )
