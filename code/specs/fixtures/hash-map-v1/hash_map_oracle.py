"""Independent DT18 association-list and physical-layout oracle."""

from __future__ import annotations

from dataclasses import dataclass
from functools import lru_cache
from importlib.util import module_from_spec, spec_from_file_location
from itertools import permutations, product
from pathlib import Path
from typing import Any, Iterable


_DT17_PATH = Path(__file__).resolve().parent.parent / "hash-functions-v1" / "oracle.py"
_DT17_SPEC = spec_from_file_location("dt17_hash_map_reference", _DT17_PATH)
if _DT17_SPEC is None or _DT17_SPEC.loader is None:
    raise RuntimeError(f"cannot load independent DT17 oracle at {_DT17_PATH}")
_DT17 = module_from_spec(_DT17_SPEC)
_DT17_SPEC.loader.exec_module(_DT17)

HISTORY_KEYS = ("k0", "k1", "k2")
HISTORY_VALUES = ("v0", "v1")
HISTORY_ALPHABET = "0123456789abcdefghijklmnopq"
PHYSICAL_ALPHABET = "0123456789abcdef"
CAPACITY_ALPHABET = "0123456789abcdefghijklmnopqrstuvwxyz"
HISTORY_MAX_LENGTH = 5
HISTORY_COUNT = sum(9**length for length in range(HISTORY_MAX_LENGTH + 1))
INITIAL_CAPACITY = 4
MAXIMUM_CAPACITY = 1_048_576
MAXIMUM_KEY_BYTES = 1_048_576
TOMBSTONE = object()


@dataclass(frozen=True)
class Key:
    """Fixture key identity is tag and raw payload, independent of package APIs."""

    key_id: str
    kind: str
    payload: bytes

    @property
    def tag(self) -> int:
        if self.kind == "bytes":
            return 0
        if self.kind == "string":
            return 1
        raise ValueError(f"unsupported key kind: {self.kind}")

    @property
    def identity(self) -> tuple[int, bytes]:
        return (self.tag, self.payload)

    @property
    def encoded(self) -> bytes:
        return (
            bytes((self.tag,)) + len(self.payload).to_bytes(8, "little") + self.payload
        )


@dataclass(frozen=True)
class Entry:
    identity: tuple[int, bytes]
    key_id: str
    value_id: str
    value: Any


def parse_key(record: dict[str, Any]) -> Key:
    kind = record["kind"]
    if kind == "bytes":
        payload = bytes.fromhex(record["payload_hex"])
    elif kind == "string":
        payload = record["text"].encode("utf-8", errors="strict")
    else:
        raise ValueError(f"unsupported key kind: {kind}")
    return Key(record["id"], kind, payload)


def hash_bytes(data: bytes, configuration: dict[str, Any]) -> int:
    kind = configuration["kind"]
    if kind == "fnv1a32":
        return _DT17.fnv1a32(data)
    if kind == "murmur3":
        return _DT17.murmur3_32(data, int(configuration["seed_hex"], 16))
    if kind == "sipHash24":
        return _DT17.siphash24(data, bytes.fromhex(configuration["key_hex"]))
    if kind == "defaultSipHash24":
        return _DT17.siphash24(data, b"codex-dt18-key!!")
    raise ValueError(f"unsupported hash configuration: {kind}")


def bucket_index(key: Key, capacity: int, configuration: dict[str, Any]) -> int:
    return hash_bytes(key.encoded, configuration) % capacity


@lru_cache(maxsize=1)
def _physical_order_rows() -> tuple[tuple[str, ...], ...]:
    rows: list[tuple[str, ...]] = [()]
    for length in range(1, len(HISTORY_KEYS) + 1):
        rows.extend(
            tuple(HISTORY_KEYS[index] for index in order)
            for order in permutations(range(len(HISTORY_KEYS)), length)
        )
    return tuple(rows)


def physical_order_table() -> list[list[str]]:
    return [list(row) for row in _physical_order_rows()]


def physical_order_id(key_ids: Iterable[str]) -> int:
    normalized = list(key_ids)
    try:
        return _physical_order_rows().index(tuple(normalized))
    except ValueError as error:
        raise ValueError(
            f"not a physical order for the history keys: {normalized}"
        ) from error


def semantic_state_id(entries: list[Entry]) -> int:
    values_by_identity = {entry.identity: entry.value_id for entry in entries}
    key_by_id = HISTORY_KEY_MAP
    digits: list[int] = []
    for key_id in HISTORY_KEYS:
        value_id = values_by_identity.get(key_by_id[key_id].identity)
        digits.append(0 if value_id is None else HISTORY_VALUES.index(value_id) + 1)
    return digits[0] + 3 * digits[1] + 9 * digits[2]


def history_ordinal(operation_indices: tuple[int, ...]) -> int:
    length = len(operation_indices)
    if length > HISTORY_MAX_LENGTH or any(
        not 0 <= item < 9 for item in operation_indices
    ):
        raise ValueError("history is outside the frozen operation domain")
    rank = 0
    for index in operation_indices:
        rank = rank * 9 + index
    return sum(9**shorter for shorter in range(length)) + rank


def decode_history_ordinal(ordinal: int) -> tuple[int, ...]:
    if not 0 <= ordinal < HISTORY_COUNT:
        raise ValueError("history ordinal is outside the frozen domain")
    remaining = ordinal
    length = 0
    while remaining >= 9**length:
        remaining -= 9**length
        length += 1
    if length > HISTORY_MAX_LENGTH:
        raise AssertionError("ordinal decoder exceeded maximum history length")
    digits = [0] * length
    for position in range(length - 1, -1, -1):
        remaining, digits[position] = divmod(remaining, 9)
    return tuple(digits)


def iter_histories() -> Iterable[tuple[int, ...]]:
    for length in range(HISTORY_MAX_LENGTH + 1):
        yield from product(range(9), repeat=length)


class AssociationListModel:
    """Logical map semantics: ordered key/value pairs only, no bucket state."""

    def __init__(self) -> None:
        self.entries: list[Entry] = []

    def set(self, key: Key, value_id: str, value: Any) -> None:
        for index, entry in enumerate(self.entries):
            if entry.identity == key.identity:
                self.entries[index] = Entry(key.identity, key.key_id, value_id, value)
                return
        self.entries.append(Entry(key.identity, key.key_id, value_id, value))

    def remove(self, key: Key) -> bool:
        for index, entry in enumerate(self.entries):
            if entry.identity == key.identity:
                del self.entries[index]
                return True
        return False

    def has(self, key: Key) -> bool:
        return any(entry.identity == key.identity for entry in self.entries)

    def value(self, key: Key) -> tuple[bool, Any, str | None]:
        for entry in self.entries:
            if entry.identity == key.identity:
                return True, entry.value, entry.value_id
        return False, None, None


class PhysicalLayoutModel:
    """Separate strategy-specific layout model used only for order/capacity."""

    def __init__(
        self,
        strategy: str,
        capacity: int,
        hash_configuration: dict[str, Any],
        keys: dict[str, Key],
    ) -> None:
        if strategy not in ("separateChaining", "linearProbing"):
            raise ValueError(f"unsupported strategy: {strategy}")
        self.strategy = strategy
        self.capacity = capacity
        self.hash_configuration = hash_configuration
        self.keys = keys
        self.count = 0
        self.buckets: list[list[Entry]] = [[] for _ in range(capacity)]
        self.slots: list[Entry | object | None] = [None] * capacity

    def entries(self) -> list[Entry]:
        if self.strategy == "separateChaining":
            return [entry for bucket in self.buckets for entry in bucket]
        return [slot for slot in self.slots if isinstance(slot, Entry)]

    def _find(self, key: Key) -> tuple[str, int] | None:
        start = bucket_index(key, self.capacity, self.hash_configuration)
        if self.strategy == "separateChaining":
            for index, entry in enumerate(self.buckets[start]):
                if entry.identity == key.identity:
                    return "bucket", index
            return None
        for offset in range(self.capacity):
            index = (start + offset) % self.capacity
            slot = self.slots[index]
            if slot is None:
                return None
            if isinstance(slot, Entry) and slot.identity == key.identity:
                return "slot", index
        return None

    def set(self, entry: Entry) -> None:
        found = self._find(self.keys[entry.key_id])
        if found is not None:
            kind, index = found
            if kind == "bucket":
                self.buckets[
                    bucket_index(
                        self.keys[entry.key_id], self.capacity, self.hash_configuration
                    )
                ][index] = entry
            else:
                self.slots[index] = entry
            return

        should_grow = (
            self.count == self.capacity
            if self.strategy == "separateChaining"
            else (self.count + 1) * 4 > self.capacity * 3
        )
        if should_grow:
            new_capacity = min(self.capacity * 2, MAXIMUM_CAPACITY)
            if new_capacity <= self.capacity:
                raise OverflowError("capacity_limit_exceeded")
            previous = self.entries()
            self.capacity = new_capacity
            self.count = 0
            self.buckets = [[] for _ in range(new_capacity)]
            self.slots = [None] * new_capacity
            for old_entry in previous:
                self._insert_new(old_entry)
        self._insert_new(entry)

    def _insert_new(self, entry: Entry) -> None:
        key = self.keys[entry.key_id]
        start = bucket_index(key, self.capacity, self.hash_configuration)
        if self.strategy == "separateChaining":
            self.buckets[start].append(entry)
            self.count += 1
            return

        first_tombstone: int | None = None
        for offset in range(self.capacity):
            index = (start + offset) % self.capacity
            slot = self.slots[index]
            if slot is None:
                self.slots[
                    first_tombstone if first_tombstone is not None else index
                ] = entry
                self.count += 1
                return
            if slot is TOMBSTONE and first_tombstone is None:
                first_tombstone = index
        if first_tombstone is not None:
            self.slots[first_tombstone] = entry
            self.count += 1
            return
        raise AssertionError("load threshold allowed insertion into a full table")

    def remove(self, key: Key) -> bool:
        found = self._find(key)
        if found is None:
            return False
        kind, index = found
        if kind == "bucket":
            bucket = self.buckets[
                bucket_index(key, self.capacity, self.hash_configuration)
            ]
            del bucket[index]
        else:
            self.slots[index] = TOMBSTONE
        self.count -= 1
        return True


def assert_logical_matches_physical(
    logical: AssociationListModel, physical: PhysicalLayoutModel, label: str
) -> None:
    logical_entries = {entry.identity: entry.value_id for entry in logical.entries}
    physical_entries = {entry.identity: entry.value_id for entry in physical.entries()}
    if logical_entries != physical_entries:
        raise AssertionError(
            f"association-list and physical models disagree in {label}"
        )


def history_operations() -> list[dict[str, str]]:
    return [
        {"op": op, "key_id": key_id, **({"value_id": value_id} if value_id else {})}
        for op, key_id, value_id in _history_operation_rows()
    ]


@lru_cache(maxsize=1)
def _history_operation_rows() -> tuple[tuple[str, str, str | None], ...]:
    return tuple(
        ("set", key_id, value_id)
        for key_id in HISTORY_KEYS
        for value_id in HISTORY_VALUES
    ) + tuple(("remove", key_id, None) for key_id in HISTORY_KEYS)


def run_history(
    operation_indices: tuple[int, ...],
    strategy: str,
    keys: dict[str, Key],
    values: dict[str, Any],
) -> tuple[int, int, int]:
    operations = _history_operation_rows()
    logical = AssociationListModel()
    physical = PhysicalLayoutModel(
        strategy, INITIAL_CAPACITY, {"kind": "fnv1a32"}, keys
    )
    for op_index in operation_indices:
        operation, key_id, value_id = operations[op_index]
        key = keys[key_id]
        if operation == "set":
            assert value_id is not None
            logical.set(key, value_id, values[value_id])
            physical.set(Entry(key.identity, key.key_id, value_id, values[value_id]))
        else:
            logical.remove(key)
            physical.remove(key)
        assert_logical_matches_physical(logical, physical, "history")
    state_id = semantic_state_id(logical.entries)
    order_id = physical_order_id(entry.key_id for entry in physical.entries())
    return state_id, order_id, physical.capacity


def expected_history_streams(
    keys: dict[str, Key], values: dict[str, Any]
) -> dict[str, dict[str, str]]:
    streams = {
        strategy: {"semantic": [], "physical": [], "capacity": []}
        for strategy in ("separateChaining", "linearProbing")
    }
    count = 0
    for history in iter_histories():
        if history_ordinal(history) != count:
            raise AssertionError("history enumeration is not length-lexicographic")
        if decode_history_ordinal(count) != history:
            raise AssertionError(
                "history ordinal does not decode to the enumerated history"
            )
        for strategy in streams:
            state_id, order_id, capacity = run_history(history, strategy, keys, values)
            streams[strategy]["semantic"].append(HISTORY_ALPHABET[state_id])
            streams[strategy]["physical"].append(PHYSICAL_ALPHABET[order_id])
            streams[strategy]["capacity"].append(CAPACITY_ALPHABET[capacity])
        count += 1
    if count != HISTORY_COUNT:
        raise AssertionError(f"expected {HISTORY_COUNT} histories, generated {count}")
    return {
        strategy: {kind: "".join(values) for kind, values in parts.items()}
        for strategy, parts in streams.items()
    }


def observe_state(physical: PhysicalLayoutModel) -> dict[str, Any]:
    return {
        "count": physical.count,
        "capacity": physical.capacity,
        "load_factor_ratio": [physical.count, physical.capacity],
        "entries": [
            {"key_id": entry.key_id, "value_id": entry.value_id, "value": entry.value}
            for entry in physical.entries()
        ],
    }


HISTORY_KEY_MAP = {
    "k0": Key("k0", "bytes", b"\x00"),
    "k1": Key("k1", "bytes", b"\x04"),
    "k2": Key("k2", "bytes", b"\x08"),
}
