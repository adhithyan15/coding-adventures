"""
hash_map.py — HashMap implementation with chaining and open addressing.

This module provides the core ``HashMap[K, V]`` class together with two
internal strategy classes that handle collision resolution:

  - ``_ChainingStrategy``: buckets of (key, value) lists.
  - ``_OpenAddressingStrategy``: flat array with TOMBSTONE deletion.

The ``HashMap`` delegates every operation to its chosen strategy and calls
``resize`` whenever the load factor exceeds the strategy-specific threshold.

Design pattern: **Strategy** (GoF).  The ``HashMap`` class is the context;
``_ChainingStrategy`` and ``_OpenAddressingStrategy`` are the concrete
strategies.  Swapping strategy at construction time is the only configuration
point — the public API is identical regardless of which strategy is active.

───────────────────────────────────────────────────────────────────────────
Layer position (from spec DT18):

    DT17: hash-functions  ← direct dependency
      └── DT18: hash-map  ← YOU ARE HERE
            └── DT19: hash-set

───────────────────────────────────────────────────────────────────────────
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Any, Generic, Iterator, TypeVar

from hash_functions import djb2, fnv1a_32, murmur3_32

# ---------------------------------------------------------------------------
# Type variables
# ---------------------------------------------------------------------------

K = TypeVar("K")
V = TypeVar("V")

# ---------------------------------------------------------------------------
# Key serialisation
# ---------------------------------------------------------------------------

def _serialize_key(key: Any) -> bytes:
    """
    Convert any key to bytes for hashing.

    We use ``str(key).encode("utf-8")`` as the universal serialiser.  This
    works for strings, integers, tuples, and any other type that has a
    reasonable ``__str__`` implementation.

    A production implementation might use ``repr()`` for unambiguous
    round-tripping, but ``str()`` is fine for our purposes because we
    always compare keys with ``==`` after hashing — the hash only selects
    the bucket, equality confirms the match.

    Example::

        _serialize_key("hello")  →  b"hello"
        _serialize_key(42)       →  b"42"
        _serialize_key((1, 2))   →  b"(1, 2)"
    """
    return str(key).encode("utf-8")


# ---------------------------------------------------------------------------
# Hash function dispatch
# ---------------------------------------------------------------------------

def _apply_hash(data: bytes, hash_fn_name: str) -> int:
    """
    Apply the named hash function to *data* and return the raw integer.

    Supported names:
      ``"fnv1a"``   — FNV-1a 32-bit (default; fast, good distribution)
      ``"murmur3"`` — MurmurHash3 32-bit (high quality; processes 4 bytes at a time)
      ``"djb2"``    — DJB2 (simple shift-add; classic teaching hash)

    All three functions return non-negative integers, so ``% capacity``
    always produces a valid bucket index.
    """
    if hash_fn_name == "murmur3":
        return murmur3_32(data)
    elif hash_fn_name == "djb2":
        return djb2(data)
    else:  # default: fnv1a
        return fnv1a_32(data)


# ---------------------------------------------------------------------------
# Sentinel objects for open addressing
# ---------------------------------------------------------------------------

# ``_EMPTY`` marks a slot that has never been written to.
# Probe chains STOP at EMPTY — no entry with the searched key exists beyond.
_EMPTY: object = object()

# ``_TOMBSTONE`` marks a slot where an entry was deleted.
# Probe chains CONTINUE past TOMBSTONE — a matching key may lie further along.
# During insertion, TOMBSTONE slots can be reused.
_TOMBSTONE: object = object()


# ---------------------------------------------------------------------------
# Abstract base strategy
# ---------------------------------------------------------------------------

class _HashMapStrategy(ABC, Generic[K, V]):
    """
    Abstract base for collision-resolution strategies.

    Each strategy manages its own storage layout and must implement:
      - ``set``         — insert or update a key-value pair
      - ``get``         — look up a value by key (None if absent)
      - ``delete``      — remove a key (True if removed, False if absent)
      - ``entries``     — return all live (key, value) pairs
      - ``needs_resize``— return True when load factor exceeds threshold
      - ``resize``      — re-hash all entries into a larger storage array

    The ``HashMap`` wrapper calls ``needs_resize`` after every ``set`` and
    calls ``resize(capacity * 2)`` when the threshold is exceeded.
    """

    @property
    @abstractmethod
    def _capacity(self) -> int: ...

    @property
    @abstractmethod
    def _size(self) -> int: ...

    @abstractmethod
    def set(self, key: K, value: V) -> None: ...

    @abstractmethod
    def get(self, key: K) -> V | None: ...

    @abstractmethod
    def delete(self, key: K) -> bool: ...

    @abstractmethod
    def entries(self) -> list[tuple[K, V]]: ...

    @abstractmethod
    def needs_resize(self) -> bool: ...

    @abstractmethod
    def resize(self, new_capacity: int) -> None: ...


# ---------------------------------------------------------------------------
# Strategy 1: Separate chaining
# ---------------------------------------------------------------------------

class _ChainingStrategy(_HashMapStrategy[K, V]):
    """
    Separate chaining collision strategy.

    Storage layout::

        _buckets: list[list[tuple[K, V]]]
        index:    0      1      2      ...   capacity-1
                  []    [(k,v)] [(k1,v1),(k2,v2)]   []

    Each bucket is a plain Python list of (key, value) pairs.  When two
    keys hash to the same bucket (a collision), both pairs live in that
    bucket's list.  Lookup scans the list linearly — O(1) average when
    the load factor is low (chains are short).

    Resize threshold: load factor > 1.0
    (on average, more than one item per bucket)

    Example — forcing a collision with capacity=1::

        m = HashMap(capacity=1, strategy="chaining")
        m.set("cat", 1)
        m.set("car", 2)
        # Both go to bucket 0; bucket 0 → [("cat",1), ("car",2)]
        m.get("cat")  # → 1
        m.get("car")  # → 2
    """

    def __init__(self, capacity: int, hash_fn_name: str) -> None:
        # Each bucket starts as an empty list.  We use a list comprehension
        # (NOT [[]]*capacity) because the latter would create N references to
        # the SAME list object — mutating one bucket would mutate all.
        self.__buckets: list[list[tuple[K, V]]] = [[] for _ in range(capacity)]
        self.__capacity: int = capacity
        self.__size: int = 0
        self.__hash_fn_name: str = hash_fn_name

    # -- Internal helpers ----------------------------------------------------

    @property
    def _capacity(self) -> int:
        return self.__capacity

    @property
    def _size(self) -> int:
        return self.__size

    def _bucket_index(self, key: K) -> int:
        """
        Map *key* to a bucket index in [0, capacity).

        Algorithm:
          1. Serialise key to bytes with ``_serialize_key``
          2. Apply the chosen hash function → large integer
          3. Modulo by capacity → valid bucket index

        The modulo operation means many different hash values land in the
        same bucket as capacity grows smaller relative to the hash space.
        This is expected and handled by the chaining list.
        """
        raw = _serialize_key(key)
        h = _apply_hash(raw, self.__hash_fn_name)
        return h % self.__capacity

    # -- Core operations -----------------------------------------------------

    def set(self, key: K, value: V) -> None:
        """
        Insert or update a key-value pair.

        If *key* already exists in the bucket, overwrite its value in-place.
        This keeps ``size`` constant (no duplicate keys).

        If *key* is new, append ``(key, value)`` to the bucket list and
        increment ``size``.

        Time complexity: O(n/capacity) average (length of chain at the bucket).
        """
        idx = self._bucket_index(key)
        bucket = self.__buckets[idx]
        for i, (k, _) in enumerate(bucket):
            if k == key:
                # Key exists — overwrite value; do not increment size.
                bucket[i] = (key, value)
                return
        # Key is new — append to the chain.
        bucket.append((key, value))
        self.__size += 1

    def get(self, key: K) -> V | None:
        """
        Return the value for *key*, or ``None`` if absent.

        We hash to the bucket, then scan the chain linearly.
        If the chain has length 1 (no collision) this is a single comparison.
        """
        idx = self._bucket_index(key)
        for k, v in self.__buckets[idx]:
            if k == key:
                return v
        return None

    def delete(self, key: K) -> bool:
        """
        Remove *key* from the map.

        Returns ``True`` if the key existed and was removed, ``False`` if
        the key was not present (no error is raised).

        Implementation: rebuild the bucket list without the matching pair.
        The list comprehension is O(chain length).
        """
        idx = self._bucket_index(key)
        bucket = self.__buckets[idx]
        new_bucket = [(k, v) for k, v in bucket if k != key]
        if len(new_bucket) < len(bucket):
            self.__buckets[idx] = new_bucket
            self.__size -= 1
            return True
        return False

    def entries(self) -> list[tuple[K, V]]:
        """
        Return all live (key, value) pairs in arbitrary order.

        Iterates every bucket and flattens the chains.
        O(capacity + size).
        """
        result: list[tuple[K, V]] = []
        for bucket in self.__buckets:
            result.extend(bucket)
        return result

    def needs_resize(self) -> bool:
        """
        Return True when the load factor exceeds the chaining threshold of 1.0.

        At load factor 1.0, there is (on average) one item per bucket.
        Chains start growing, degrading average lookup from O(1) toward O(n).
        Doubling the capacity restores the load factor to ~0.5.
        """
        return self.__size / self.__capacity > 1.0

    def resize(self, new_capacity: int) -> None:
        """
        Re-hash all entries into a new bucket array of size *new_capacity*.

        Algorithm (O(capacity + size)):
          1. Collect all live (key, value) pairs.
          2. Allocate a fresh bucket array of size new_capacity.
          3. Re-insert each pair using the new capacity as the modulus.

        After resize, load factor ≈ old_size / new_capacity (usually ~0.5
        after a capacity-doubling resize).
        """
        old_entries = self.entries()
        self.__capacity = new_capacity
        self.__buckets = [[] for _ in range(new_capacity)]
        self.__size = 0
        for k, v in old_entries:
            self.set(k, v)


# ---------------------------------------------------------------------------
# Strategy 2: Open addressing (linear probing)
# ---------------------------------------------------------------------------

class _OpenAddressingStrategy(_HashMapStrategy[K, V]):
    """
    Open addressing with linear probing and tombstone deletion.

    Storage layout::

        _slots: list[_EMPTY | _TOMBSTONE | tuple[K, V]]
        index:  0         1           2           ...

    All entries live in a single flat array — no heap allocation per entry.
    This gives excellent cache locality at low load factors.

    **Linear probing**: when inserting key ``k`` and slot ``h(k) % cap``
    is occupied, try ``(h(k)+1) % cap``, then ``(h(k)+2) % cap``, etc.

    **Tombstone deletion**: clearing a slot on deletion would break existing
    probe chains.  Instead, we mark the slot ``_TOMBSTONE``.  Lookups skip
    tombstones; insertions reuse them.

    Resize threshold: load factor > 0.75.
    (Python's dict resizes at 2/3; Java's HashMap at 0.75)

    Worked example::

        capacity=4, insert "cat" (hash%4=3), insert "car" (hash%4=3)

        After "cat":  [EMPTY, EMPTY, EMPTY, ("cat",1)]
        After "car":  probe 3 → occupied, probe 0 (wrap) → empty
                      [("car",2), EMPTY, EMPTY, ("cat",1)]

        Delete "cat": slot 3 → TOMBSTONE
                      [("car",2), EMPTY, EMPTY, TOMBSTONE]

        Lookup "car": hash%4=3 → TOMBSTONE (continue) → wrap to 0 →
                      ("car",2) → found!
    """

    def __init__(self, capacity: int, hash_fn_name: str) -> None:
        self.__slots: list[Any] = [_EMPTY] * capacity
        self.__capacity: int = capacity
        self.__size: int = 0
        self.__hash_fn_name: str = hash_fn_name

    # -- Internal helpers ----------------------------------------------------

    @property
    def _capacity(self) -> int:
        return self.__capacity

    @property
    def _size(self) -> int:
        return self.__size

    def _start_index(self, key: K) -> int:
        """Compute the starting probe position for *key*."""
        raw = _serialize_key(key)
        h = _apply_hash(raw, self.__hash_fn_name)
        return h % self.__capacity

    # -- Core operations -----------------------------------------------------

    def set(self, key: K, value: V) -> None:
        """
        Insert or update a key-value pair using linear probing.

        Probe sequence:
          ``start``, ``start+1``, ..., ``start+capacity-1`` (all mod capacity)

        Decision at each probed slot:
          - **EMPTY**: key is new → insert here (or at first tombstone seen).
          - **TOMBSTONE**: remember this index as a candidate for insertion,
            continue probing (the key might exist further along).
          - **Occupied, same key**: update value in-place, return.
          - **Occupied, different key**: continue (collision).

        Why reuse the first tombstone?
          Placing the new entry at the earliest tombstone minimises probe
          distances for future lookups that start between the tombstone and
          the old empty slot.

        Raises ``RuntimeError`` if the table is full (should never happen
        because ``HashMap`` resizes before calling ``set``).
        """
        start = self._start_index(key)
        first_tombstone: int | None = None

        for probe in range(self.__capacity):
            i = (start + probe) % self.__capacity
            slot = self.__slots[i]

            if slot is _EMPTY:
                # Key does not exist — insert at tombstone (if seen) or here.
                insert_at = first_tombstone if first_tombstone is not None else i
                self.__slots[insert_at] = (key, value)
                self.__size += 1
                return

            if slot is _TOMBSTONE:
                # Mark as candidate but keep probing in case key exists ahead.
                if first_tombstone is None:
                    first_tombstone = i

            elif slot[0] == key:
                # Key found — overwrite value (size unchanged).
                self.__slots[i] = (key, value)
                return
            # else: occupied by a different key → continue probing.

        # All slots tried — if we saw a tombstone, insert there.
        if first_tombstone is not None:
            self.__slots[first_tombstone] = (key, value)
            self.__size += 1
            return

        raise RuntimeError(
            "Hash table is completely full — resize should have triggered earlier."
        )

    def get(self, key: K) -> V | None:
        """
        Return the value for *key*, or ``None`` if absent.

        Probe sequence is the same as ``set``.  We stop at:
          - **EMPTY**: key is definitely not in the table (probe chain ended).
          - **Occupied, matching key**: found.

        We continue past TOMBSTONES because a key may have been inserted
        *after* a deletion occurred at an intermediate slot.
        """
        start = self._start_index(key)
        for probe in range(self.__capacity):
            i = (start + probe) % self.__capacity
            slot = self.__slots[i]
            if slot is _EMPTY:
                return None
            if slot is not _TOMBSTONE and slot[0] == key:
                return slot[1]
        return None

    def delete(self, key: K) -> bool:
        """
        Remove *key* by placing a TOMBSTONE in its slot.

        Returns ``True`` if removed, ``False`` if not found.

        Why TOMBSTONE instead of EMPTY?
          If we wrote EMPTY, future lookups for keys that were probed past
          this slot would stop at the (now-empty) slot and incorrectly
          conclude the key does not exist.

          TOMBSTONE tells the lookup: "something used to be here; keep
          probing forward."
        """
        start = self._start_index(key)
        for probe in range(self.__capacity):
            i = (start + probe) % self.__capacity
            slot = self.__slots[i]
            if slot is _EMPTY:
                return False
            if slot is not _TOMBSTONE and slot[0] == key:
                self.__slots[i] = _TOMBSTONE
                self.__size -= 1
                return True
        return False

    def entries(self) -> list[tuple[K, V]]:
        """
        Return all live (key, value) pairs.

        Only slots that hold a tuple (not EMPTY or TOMBSTONE) are included.
        """
        return [
            slot
            for slot in self.__slots
            if slot is not _EMPTY and slot is not _TOMBSTONE
        ]

    def needs_resize(self) -> bool:
        """
        Return True when load factor exceeds the open-addressing threshold 0.75.

        At high load factors, probe chains grow longer and performance
        degrades toward O(n).  Resizing at 0.75 keeps average probe length
        under 2 for most distributions.

        Knuth's analysis (Introduction to Algorithms) shows that at
        α=0.75, the expected probe length is ~2.5.  At α=0.9 it's ~5.5.
        """
        return self.__size / self.__capacity > 0.75

    def resize(self, new_capacity: int) -> None:
        """
        Re-hash all live entries into a new slot array of size *new_capacity*.

        Tombstones are discarded during resize — they only serve to preserve
        probe chains in the current array, which is being replaced.

        This has the beneficial side effect of compacting the table after
        heavy deletion.
        """
        old_entries = self.entries()
        self.__capacity = new_capacity
        self.__slots = [_EMPTY] * new_capacity
        self.__size = 0
        for k, v in old_entries:
            self.set(k, v)


# ---------------------------------------------------------------------------
# HashMap: the public interface
# ---------------------------------------------------------------------------

class HashMap(Generic[K, V]):
    """
    A generic hash map with pluggable collision strategy and hash function.

    Parameters
    ----------
    capacity:
        Initial number of buckets/slots.  Grows automatically; this is
        only a hint.  Default: 16.
    strategy:
        ``"chaining"`` (default) or ``"open_addressing"``.
    hash_fn:
        ``"fnv1a"`` (default), ``"murmur3"``, or ``"djb2"``.

    Examples
    --------
    Basic usage::

        >>> m = HashMap()
        >>> m.set("key", "value")
        >>> m.get("key")
        'value'
        >>> m.has("key")
        True
        >>> m.delete("key")
        True
        >>> m.size()
        0

    Open addressing with murmur3::

        >>> m = HashMap(capacity=8, strategy="open_addressing", hash_fn="murmur3")
        >>> m.set(1, "one")
        >>> m.set(2, "two")
        >>> m.entries()  # order may vary
        [(1, 'one'), (2, 'two')]

    Iteration::

        >>> m = HashMap()
        >>> m.set("a", 1)
        >>> m.set("b", 2)
        >>> sorted(m)
        ['a', 'b']
        >>> len(m)
        2

    Implementation notes
    --------------------
    After every ``set``, we check ``_impl.needs_resize()``.  If True,
    we call ``_impl.resize(capacity * 2)``.  The resize doubles the
    capacity, which halves the load factor.

    This is O(n) work amortised over n inserts — each element is
    re-hashed at most O(log n) times across all resizes.

    We do NOT shrink the table on deletion.  Shrinking adds complexity
    and deletion-heavy workloads are uncommon compared to insertion.
    """

    def __init__(
        self,
        capacity: int = 16,
        strategy: str = "chaining",
        hash_fn: str = "fnv1a",
    ) -> None:
        self._strategy_name = strategy
        self._hash_fn_name = hash_fn
        self._impl: _ChainingStrategy[K, V] | _OpenAddressingStrategy[K, V]
        if strategy == "chaining":
            self._impl = _ChainingStrategy(capacity, hash_fn)
        elif strategy == "open_addressing":
            self._impl = _OpenAddressingStrategy(capacity, hash_fn)
        else:
            raise ValueError(
                f"Unknown strategy {strategy!r}. "
                "Choose 'chaining' or 'open_addressing'."
            )

    # -- Mutating operations -------------------------------------------------

    def set(self, key: K, value: V) -> None:
        """
        Insert or update *key* → *value*.

        If *key* already exists, its value is overwritten.
        If *key* is new, it is added; size increases by 1.

        After insertion, if the load factor exceeds the strategy threshold,
        the map is automatically resized (capacity doubled).

        Time: O(1) amortised.
        """
        self._impl.set(key, value)
        if self._impl.needs_resize():
            self._impl.resize(self._impl._capacity * 2)

    def delete(self, key: K) -> bool:
        """
        Remove *key* from the map.

        Returns ``True`` if the key existed and was deleted.
        Returns ``False`` if the key was not present (no error).

        Time: O(1) average.
        """
        return self._impl.delete(key)

    # -- Query operations ----------------------------------------------------

    def get(self, key: K) -> V | None:
        """
        Return the value for *key*, or ``None`` if absent.

        Time: O(1) average.
        """
        return self._impl.get(key)

    def has(self, key: K) -> bool:
        """
        Return ``True`` if *key* is present in the map.

        Delegates to ``__contains__`` so that keys mapped to ``None``
        are correctly reported as present (``get`` alone cannot
        distinguish "key maps to None" from "key absent").
        """
        return key in self

    # -- Bulk accessors ------------------------------------------------------

    def keys(self) -> list[K]:
        """Return a list of all keys in arbitrary order."""
        return [k for k, _ in self._impl.entries()]

    def values(self) -> list[V]:
        """Return a list of all values in arbitrary order."""
        return [v for _, v in self._impl.entries()]

    def entries(self) -> list[tuple[K, V]]:
        """Return a list of all (key, value) pairs in arbitrary order."""
        return self._impl.entries()

    # -- Introspection -------------------------------------------------------

    def size(self) -> int:
        """Return the number of key-value pairs currently stored."""
        return self._impl._size

    def load_factor(self) -> float:
        """
        Return the current load factor: ``size / capacity``.

        For chaining this is always ≤ 1.0 immediately after a resize.
        For open addressing this is always ≤ 0.75 immediately after a resize.
        """
        if self._impl._capacity == 0:
            return 0.0
        return self._impl._size / self._impl._capacity

    def capacity(self) -> int:
        """Return the current capacity (number of buckets/slots)."""
        return self._impl._capacity

    # -- Python protocol methods ---------------------------------------------

    def __len__(self) -> int:
        """Support ``len(m)``."""
        return self._impl._size

    def __contains__(self, key: object) -> bool:
        """
        Support ``key in m``.

        We look up via entries to correctly handle None values stored
        under a key (``get`` would return None for both "found, value=None"
        and "not found").
        """
        for k, _ in self._impl.entries():
            if k == key:
                return True
        return False

    def __iter__(self) -> Iterator[K]:
        """Support ``for key in m:``."""
        return iter(self.keys())

    def __repr__(self) -> str:
        """
        Human-readable representation.

        Example::

            HashMap(size=3, capacity=16, strategy='chaining', hash_fn='fnv1a',
                    entries={'a': 1, 'b': 2, 'c': 3})
        """
        pairs = ", ".join(
            f"{k!r}: {v!r}" for k, v in self._impl.entries()
        )
        return (
            f"HashMap(size={self._impl._size}, "
            f"capacity={self._impl._capacity}, "
            f"strategy={self._strategy_name!r}, "
            f"hash_fn={self._hash_fn_name!r}, "
            f"entries={{{pairs}}})"
        )


# ---------------------------------------------------------------------------
# Module-level utility functions
# ---------------------------------------------------------------------------

def from_entries(
    pairs: list[tuple[K, V]],
    strategy: str = "chaining",
    hash_fn: str = "fnv1a",
) -> HashMap[K, V]:
    """
    Construct a ``HashMap`` from a list of (key, value) pairs.

    If the same key appears more than once, the last value wins
    (consistent with Python dict behaviour).

    Example::

        >>> m = from_entries([("a", 1), ("b", 2), ("c", 3)])
        >>> m.get("b")
        2
        >>> m.size()
        3
    """
    m: HashMap[K, V] = HashMap(strategy=strategy, hash_fn=hash_fn)
    for k, v in pairs:
        m.set(k, v)
    return m


def merge(m1: HashMap[K, V], m2: HashMap[K, V]) -> HashMap[K, V]:
    """
    Return a new ``HashMap`` containing all entries from *m1* and *m2*.

    When the same key exists in both maps, the value from *m2* wins.
    Neither *m1* nor *m2* is modified.

    The result uses the same strategy and hash function as *m1*.

    Example::

        >>> m1 = from_entries([("a", 1), ("b", 2)])
        >>> m2 = from_entries([("b", 99), ("c", 3)])
        >>> m3 = merge(m1, m2)
        >>> m3.get("a"), m3.get("b"), m3.get("c")
        (1, 99, 3)
    """
    result: HashMap[K, V] = HashMap(
        strategy=m1._strategy_name,
        hash_fn=m1._hash_fn_name,
    )
    for k, v in m1.entries():
        result.set(k, v)
    for k, v in m2.entries():
        result.set(k, v)
    return result
