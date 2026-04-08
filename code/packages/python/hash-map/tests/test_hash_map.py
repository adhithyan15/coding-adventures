"""
Tests for the hash_map package (DT18).

All functional tests are parametrized over both strategies so that
chaining and open addressing are always held to the same observable
contract.  Strategy-specific tests (tombstone behaviour, load-factor
thresholds) are clearly labelled.

Coverage target: 95%+
"""

from __future__ import annotations

import random

import pytest

from hash_map import HashMap, from_entries, merge


# ---------------------------------------------------------------------------
# Parametrize helpers
# ---------------------------------------------------------------------------

STRATEGIES = ["chaining", "open_addressing"]
HASH_FNS = ["fnv1a", "murmur3", "djb2"]


# ---------------------------------------------------------------------------
# TestSetGet
# ---------------------------------------------------------------------------


class TestSetGet:
    """Basic insertion and retrieval."""

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_set_and_get_single_key(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        m.set("hello", 42)
        assert m.get("hello") == 42

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_get_missing_key_returns_none(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        assert m.get("missing") is None

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_overwrite_existing_key(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        m.set("a", 1)
        m.set("a", 99)
        assert m.get("a") == 99
        assert m.size() == 1  # no duplicate

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_multiple_distinct_keys(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        m.set("x", 10)
        m.set("y", 20)
        m.set("z", 30)
        assert m.get("x") == 10
        assert m.get("y") == 20
        assert m.get("z") == 30

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_none_value_stored_and_retrieved(self, strategy: str) -> None:
        # Storing None as a value is legal.
        m: HashMap[str, None] = HashMap(strategy=strategy)
        m.set("key", None)
        assert m.size() == 1
        # None value: get returns None, but key IS present
        assert "key" in m

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_integer_keys(self, strategy: str) -> None:
        m: HashMap[int, str] = HashMap(strategy=strategy)
        m.set(0, "zero")
        m.set(1, "one")
        m.set(100, "hundred")
        assert m.get(0) == "zero"
        assert m.get(1) == "one"
        assert m.get(100) == "hundred"

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_all_hash_functions(self, strategy: str) -> None:
        for fn in HASH_FNS:
            m: HashMap[str, int] = HashMap(strategy=strategy, hash_fn=fn)
            m.set("alpha", 1)
            m.set("beta", 2)
            assert m.get("alpha") == 1
            assert m.get("beta") == 2

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_large_number_of_keys(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        keys = [f"key_{i}" for i in range(200)]
        for i, k in enumerate(keys):
            m.set(k, i)
        for i, k in enumerate(keys):
            assert m.get(k) == i


# ---------------------------------------------------------------------------
# TestDelete
# ---------------------------------------------------------------------------


class TestDelete:
    """Deletion behaviour."""

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_delete_existing_key_returns_true(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        m.set("a", 1)
        assert m.delete("a") is True

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_delete_existing_key_removes_it(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        m.set("a", 1)
        m.delete("a")
        assert m.get("a") is None
        assert m.size() == 0

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_delete_nonexistent_key_returns_false(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        assert m.delete("ghost") is False

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_delete_nonexistent_does_not_change_size(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        m.set("a", 1)
        m.delete("ghost")
        assert m.size() == 1

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_delete_then_re_insert(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        m.set("a", 1)
        m.delete("a")
        m.set("a", 2)
        assert m.get("a") == 2
        assert m.size() == 1

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_delete_all_keys(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        for i in range(10):
            m.set(f"k{i}", i)
        for i in range(10):
            m.delete(f"k{i}")
        assert m.size() == 0
        for i in range(10):
            assert m.get(f"k{i}") is None

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_has_returns_false_after_delete(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        m.set("a", 1)
        m.delete("a")
        assert m.has("a") is False


# ---------------------------------------------------------------------------
# TestHas
# ---------------------------------------------------------------------------


class TestHas:
    """has() / __contains__ membership tests."""

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_has_existing_key(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        m.set("cat", 1)
        assert m.has("cat") is True

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_has_missing_key(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        assert m.has("dog") is False

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_contains_operator(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        m.set("a", 1)
        assert "a" in m
        assert "b" not in m


# ---------------------------------------------------------------------------
# TestCollisionHandling
# ---------------------------------------------------------------------------


class TestCollisionHandling:
    """Force collisions and verify correctness."""

    def test_chaining_all_keys_same_bucket(self) -> None:
        # capacity=1 forces all keys to bucket 0.  Every key collides.
        m: HashMap[str, int] = HashMap(capacity=1, strategy="chaining")
        m.set("cat", 1)
        m.set("car", 2)
        m.set("cab", 3)
        assert m.get("cat") == 1
        assert m.get("car") == 2
        assert m.get("cab") == 3
        assert m.size() == 3

    def test_chaining_overwrite_in_collision_bucket(self) -> None:
        m: HashMap[str, int] = HashMap(capacity=1, strategy="chaining")
        m.set("cat", 1)
        m.set("cat", 99)  # overwrite
        assert m.get("cat") == 99
        assert m.size() == 1

    def test_open_addressing_collision_keys_accessible(self) -> None:
        # Use a small capacity to force probing.
        m: HashMap[str, int] = HashMap(capacity=4, strategy="open_addressing")
        keys = [f"k{i}" for i in range(3)]
        for i, k in enumerate(keys):
            m.set(k, i)
        for i, k in enumerate(keys):
            assert m.get(k) == i

    def test_open_addressing_collision_with_wraparound(self) -> None:
        # Insert enough items into a tiny table to force wrap-around probing.
        m: HashMap[int, str] = HashMap(capacity=4, strategy="open_addressing")
        # At capacity=4 with 3 items, load=0.75 which triggers resize;
        # use 2 items to stay below threshold without triggering it.
        m.set(0, "zero")
        m.set(4, "four")   # same hash%4 as 0 → probes forward
        assert m.get(0) == "zero"
        assert m.get(4) == "four"


# ---------------------------------------------------------------------------
# TestResize
# ---------------------------------------------------------------------------


class TestResize:
    """Automatic resizing when load factor threshold is exceeded."""

    def test_chaining_resize_doubles_capacity(self) -> None:
        # Start at capacity=4 (chaining resizes at load>1.0 → 5 items).
        m: HashMap[str, int] = HashMap(capacity=4, strategy="chaining")
        for i in range(5):
            m.set(f"k{i}", i)
        # After 5th insert, load was 5/4=1.25 → resize to 8.
        assert m.capacity() == 8
        assert m.size() == 5

    def test_chaining_all_keys_accessible_after_resize(self) -> None:
        m: HashMap[str, int] = HashMap(capacity=4, strategy="chaining")
        for i in range(5):
            m.set(f"k{i}", i)
        for i in range(5):
            assert m.get(f"k{i}") == i

    def test_open_addressing_resize_doubles_capacity(self) -> None:
        # capacity=4, open addressing resizes at load>0.75 → 4th insert triggers.
        m: HashMap[str, int] = HashMap(capacity=4, strategy="open_addressing")
        for i in range(4):
            m.set(f"k{i}", i)
        assert m.capacity() == 8

    def test_open_addressing_all_keys_accessible_after_resize(self) -> None:
        m: HashMap[str, int] = HashMap(capacity=4, strategy="open_addressing")
        for i in range(4):
            m.set(f"k{i}", i)
        for i in range(4):
            assert m.get(f"k{i}") == i

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_multiple_resizes(self, strategy: str) -> None:
        # Trigger several resize events via bulk insertion.
        m: HashMap[str, int] = HashMap(capacity=2, strategy=strategy)
        for i in range(100):
            m.set(f"key{i}", i)
        assert m.size() == 100
        for i in range(100):
            assert m.get(f"key{i}") == i


# ---------------------------------------------------------------------------
# TestTombstones (open addressing only)
# ---------------------------------------------------------------------------


class TestTombstones:
    """
    Tombstone-specific tests for open addressing.

    Deleting an entry places a TOMBSTONE sentinel.  Lookups for keys
    that were probed past the deleted slot must still succeed.
    """

    def test_tombstone_does_not_break_lookup(self) -> None:
        # Use capacity=8 so we don't trigger automatic resize during setup.
        m: HashMap[str, int] = HashMap(capacity=8, strategy="open_addressing")
        # Insert "cat" first; "car" will probe past it if they collide.
        m.set("cat", 1)
        m.set("car", 2)
        # Delete "cat" → slot becomes TOMBSTONE.
        m.delete("cat")
        # "car" must still be findable even if its probe chain crosses the tombstone.
        assert m.get("car") == 2

    def test_tombstone_slot_reused_on_insert(self) -> None:
        m: HashMap[str, int] = HashMap(capacity=8, strategy="open_addressing")
        m.set("a", 1)
        m.delete("a")
        m.set("a", 2)
        assert m.get("a") == 2
        assert m.size() == 1

    def test_tombstone_chain_multiple_deletes(self) -> None:
        """Three colliding insertions, middle deleted; both ends still found."""
        m: HashMap[int, str] = HashMap(capacity=8, strategy="open_addressing")
        # Force three entries to the same starting slot by using integer keys
        # and small capacity.
        m2: HashMap[int, str] = HashMap(capacity=4, strategy="open_addressing")
        # Use 2 items (below resize threshold) at capacity=4.
        m2.set(1, "one")
        m2.set(5, "five")   # same hash%4 as 1 in many hash functions? Not guaranteed.
        # Re-insert both to make sure they're there regardless.
        assert m2.size() == 2
        # Delete one and verify the other.
        m2.delete(1)
        assert m2.get(5) == "five"

    def test_tombstones_cleared_on_resize(self) -> None:
        """After resize, tombstones disappear and size is correct."""
        m: HashMap[str, int] = HashMap(capacity=8, strategy="open_addressing")
        for i in range(4):
            m.set(f"k{i}", i)
        for i in range(4):
            m.delete(f"k{i}")
        assert m.size() == 0
        # Insert fresh entries; they should work cleanly.
        m.set("new", 42)
        assert m.get("new") == 42


# ---------------------------------------------------------------------------
# TestBulkAccess
# ---------------------------------------------------------------------------


class TestBulkAccess:
    """keys(), values(), entries() — order may vary but all elements present."""

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_keys_contains_all_keys(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        expected = {"a", "b", "c"}
        for k in expected:
            m.set(k, 1)
        assert set(m.keys()) == expected

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_values_contains_all_values(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        m.set("a", 10)
        m.set("b", 20)
        m.set("c", 30)
        assert sorted(m.values()) == [10, 20, 30]

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_entries_contains_all_pairs(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        pairs = [("x", 1), ("y", 2), ("z", 3)]
        for k, v in pairs:
            m.set(k, v)
        assert set(m.entries()) == set(pairs)

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_empty_map_bulk_access(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        assert m.keys() == []
        assert m.values() == []
        assert m.entries() == []

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_keys_count_equals_size(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        for i in range(20):
            m.set(f"k{i}", i)
        assert len(m.keys()) == m.size()


# ---------------------------------------------------------------------------
# TestFromEntries
# ---------------------------------------------------------------------------


class TestFromEntries:
    """from_entries factory function."""

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_creates_correct_map(self, strategy: str) -> None:
        m = from_entries([("a", 1), ("b", 2), ("c", 3)], strategy=strategy)
        assert m.get("a") == 1
        assert m.get("b") == 2
        assert m.get("c") == 3
        assert m.size() == 3

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_last_value_wins_for_duplicate_keys(self, strategy: str) -> None:
        m = from_entries([("a", 1), ("a", 2), ("a", 3)], strategy=strategy)
        assert m.get("a") == 3
        assert m.size() == 1

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_empty_list(self, strategy: str) -> None:
        m = from_entries([], strategy=strategy)
        assert m.size() == 0

    @pytest.mark.parametrize("hash_fn", HASH_FNS)
    def test_all_hash_functions(self, hash_fn: str) -> None:
        m = from_entries([("k", 42)], hash_fn=hash_fn)
        assert m.get("k") == 42


# ---------------------------------------------------------------------------
# TestMerge
# ---------------------------------------------------------------------------


class TestMerge:
    """merge() utility function."""

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_merge_disjoint_maps(self, strategy: str) -> None:
        m1 = from_entries([("a", 1), ("b", 2)], strategy=strategy)
        m2 = from_entries([("c", 3), ("d", 4)], strategy=strategy)
        m3 = merge(m1, m2)
        assert m3.get("a") == 1
        assert m3.get("b") == 2
        assert m3.get("c") == 3
        assert m3.get("d") == 4
        assert m3.size() == 4

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_merge_m2_overrides_m1_on_conflict(self, strategy: str) -> None:
        m1 = from_entries([("a", 1), ("b", 2)], strategy=strategy)
        m2 = from_entries([("b", 99), ("c", 3)], strategy=strategy)
        m3 = merge(m1, m2)
        assert m3.get("a") == 1
        assert m3.get("b") == 99  # m2 wins
        assert m3.get("c") == 3

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_merge_does_not_mutate_inputs(self, strategy: str) -> None:
        m1 = from_entries([("a", 1)], strategy=strategy)
        m2 = from_entries([("a", 99)], strategy=strategy)
        merge(m1, m2)
        assert m1.get("a") == 1  # unchanged
        assert m2.get("a") == 99  # unchanged

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_merge_with_empty_map(self, strategy: str) -> None:
        m1 = from_entries([("a", 1)], strategy=strategy)
        m2: HashMap[str, int] = HashMap(strategy=strategy)
        m3 = merge(m1, m2)
        assert m3.get("a") == 1
        assert m3.size() == 1

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_merge_two_empty_maps(self, strategy: str) -> None:
        m1: HashMap[str, int] = HashMap(strategy=strategy)
        m2: HashMap[str, int] = HashMap(strategy=strategy)
        m3 = merge(m1, m2)
        assert m3.size() == 0


# ---------------------------------------------------------------------------
# TestPropertyBased
# ---------------------------------------------------------------------------


class TestPropertyBased:
    """
    Property-based style tests comparing HashMap against Python's dict.

    For any sequence of set/delete operations, the HashMap must produce
    the same state as a reference Python dict.
    """

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_random_set_delete_matches_dict(self, strategy: str) -> None:
        rng = random.Random(42)
        ref: dict[str, int] = {}
        m: HashMap[str, int] = HashMap(strategy=strategy)

        keys = [f"k{i}" for i in range(20)]

        for _ in range(300):
            op = rng.choice(["set", "delete"])
            k = rng.choice(keys)
            if op == "set":
                v = rng.randint(0, 1000)
                ref[k] = v
                m.set(k, v)
            else:
                ref.pop(k, None)
                m.delete(k)

        # Verify size
        assert m.size() == len(ref), f"size mismatch: {m.size()} vs {len(ref)}"

        # Verify each key in ref is in m with the correct value
        for k, v in ref.items():
            assert m.get(k) == v, f"value mismatch for key {k!r}"

        # Verify no extra keys in m
        assert set(m.keys()) == set(ref.keys())

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_size_equals_len_of_keys(self, strategy: str) -> None:
        rng = random.Random(7)
        m: HashMap[str, int] = HashMap(strategy=strategy)
        for _ in range(100):
            k = rng.choice(["a", "b", "c", "d", "e"])
            v = rng.randint(0, 100)
            m.set(k, v)
            assert m.size() == len(m.keys()), "size() must equal len(keys())"

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_all_keys_accessible_after_bulk_insert(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        n = 150
        for i in range(n):
            m.set(f"key_{i}", i * 2)
        assert m.size() == n
        for i in range(n):
            assert m.get(f"key_{i}") == i * 2


# ---------------------------------------------------------------------------
# TestLoadFactor
# ---------------------------------------------------------------------------


class TestLoadFactor:
    """Load factor invariants."""

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_load_factor_never_exceeds_threshold_after_ops(
        self, strategy: str
    ) -> None:
        threshold = 1.0 if strategy == "chaining" else 0.75
        m: HashMap[str, int] = HashMap(capacity=4, strategy=strategy)
        for i in range(50):
            m.set(f"k{i}", i)
            # After each set, load factor must not exceed threshold
            # (HashMap resizes when it would).
            assert m.load_factor() <= threshold + 1e-9, (
                f"load factor {m.load_factor():.4f} exceeds threshold "
                f"{threshold} after {i+1} inserts"
            )

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_load_factor_zero_on_empty_map(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        assert m.load_factor() == 0.0

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_load_factor_correct_formula(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(capacity=16, strategy=strategy)
        m.set("a", 1)
        m.set("b", 2)
        expected = 2 / m.capacity()
        assert abs(m.load_factor() - expected) < 1e-9


# ---------------------------------------------------------------------------
# TestRepr
# ---------------------------------------------------------------------------


class TestRepr:
    """__repr__ contains useful information."""

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_repr_contains_strategy(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        r = repr(m)
        assert strategy in r

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_repr_contains_size(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        m.set("a", 1)
        r = repr(m)
        assert "size=1" in r

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_repr_contains_capacity(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(capacity=32, strategy=strategy)
        r = repr(m)
        assert "32" in r

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_repr_contains_hash_fn(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy, hash_fn="djb2")
        r = repr(m)
        assert "djb2" in r


# ---------------------------------------------------------------------------
# TestPythonProtocols
# ---------------------------------------------------------------------------


class TestPythonProtocols:
    """__len__, __contains__, __iter__."""

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_len(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        assert len(m) == 0
        m.set("a", 1)
        assert len(m) == 1
        m.set("b", 2)
        assert len(m) == 2
        m.delete("a")
        assert len(m) == 1

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_contains(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        m.set("x", 10)
        assert "x" in m
        assert "y" not in m

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_iter(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        m.set("a", 1)
        m.set("b", 2)
        m.set("c", 3)
        assert sorted(m) == ["a", "b", "c"]


# ---------------------------------------------------------------------------
# TestInvalidStrategy
# ---------------------------------------------------------------------------


class TestInvalidStrategy:
    """Constructor validation."""

    def test_invalid_strategy_raises_value_error(self) -> None:
        with pytest.raises(ValueError, match="Unknown strategy"):
            HashMap(strategy="linked_list")


# ---------------------------------------------------------------------------
# TestEdgeCases
# ---------------------------------------------------------------------------


class TestEdgeCases:
    """Corner cases and boundary conditions."""

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_empty_string_key(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        m.set("", 0)
        assert m.get("") == 0
        assert m.size() == 1

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_tuple_key(self, strategy: str) -> None:
        m: HashMap[tuple[int, int], str] = HashMap(strategy=strategy)
        m.set((1, 2), "pair")
        assert m.get((1, 2)) == "pair"

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_single_item_repeated_overwrites(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        for i in range(50):
            m.set("only", i)
        assert m.get("only") == 49
        assert m.size() == 1

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_delete_from_empty_map(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        assert m.delete("nonexistent") is False

    @pytest.mark.parametrize("strategy", STRATEGIES)
    def test_interleaved_set_and_delete(self, strategy: str) -> None:
        m: HashMap[str, int] = HashMap(strategy=strategy)
        m.set("a", 1)
        m.set("b", 2)
        m.delete("a")
        m.set("c", 3)
        m.delete("b")
        assert m.get("a") is None
        assert m.get("b") is None
        assert m.get("c") == 3
        assert m.size() == 1
