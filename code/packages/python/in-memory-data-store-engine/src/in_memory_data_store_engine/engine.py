"""Pure-Python execution engine for the in-memory data store stack."""

from __future__ import annotations

import heapq
import math
import time
from collections.abc import Callable
from dataclasses import dataclass
from decimal import Decimal
from enum import StrEnum
from typing import Any

from hyperloglog import HyperLogLog
from in_memory_data_store_protocol import CommandFrame, EngineResponse

_I64_MIN = -(2**63)
_I64_MAX = 2**63 - 1


class EntryType(StrEnum):
    """Wire-visible value types supported by the engine."""

    STRING = "string"
    HASH = "hash"
    LIST = "list"
    SET = "set"
    ZSET = "zset"
    HLL = "hll"


@dataclass(slots=True)
class Entry:
    """A typed value and its optional absolute expiry timestamp."""

    type: EntryType
    value: Any
    expires_at_ms: int | None = None


class SortedSet:
    """Deterministic score/member ordering for Redis-style sorted sets."""

    def __init__(self) -> None:
        self._scores: dict[bytes, float] = {}

    def insert(self, score: float, member: bytes) -> bool:
        is_new = member not in self._scores
        self._scores[bytes(member)] = score
        return is_new

    def remove(self, member: bytes) -> bool:
        if member not in self._scores:
            return False
        del self._scores[member]
        return True

    def rank(self, member: bytes) -> int | None:
        for index, (candidate, _) in enumerate(self.ordered_entries()):
            if candidate == member:
                return index
        return None

    def score(self, member: bytes) -> float | None:
        return self._scores.get(member)

    def __len__(self) -> int:
        return len(self._scores)

    def ordered_entries(self) -> list[tuple[bytes, float]]:
        return sorted(self._scores.items(), key=lambda item: (item[1], item[0]))

    def range_by_index(self, start: int, end: int) -> list[tuple[bytes, float]]:
        entries = self.ordered_entries()
        length = len(entries)
        if length == 0:
            return []
        normalized_start = length + start if start < 0 else start
        normalized_end = length + end if end < 0 else end
        if (
            normalized_start < 0
            or normalized_end < 0
            or normalized_start >= length
            or normalized_start > normalized_end
        ):
            return []
        return entries[normalized_start : min(length, normalized_end + 1)]

    def range_by_score(self, minimum: float, maximum: float) -> list[tuple[bytes, float]]:
        return [item for item in self.ordered_entries() if minimum <= item[1] <= maximum]


class Database:
    """One logical database with lazy and active expiry."""

    def __init__(self) -> None:
        self.entries: dict[bytes, Entry] = {}
        self.ttl_heap: list[tuple[int, bytes]] = []

    def get(self, key: bytes) -> Entry | None:
        entry = self.entries.get(key)
        if entry is None:
            return None
        if entry.expires_at_ms is not None and entry.expires_at_ms <= _now_ms():
            self.delete(key)
            return None
        return entry

    def set(self, key: bytes, entry: Entry) -> None:
        immutable_key = bytes(key)
        self.entries[immutable_key] = entry
        if entry.expires_at_ms is not None:
            heapq.heappush(self.ttl_heap, (entry.expires_at_ms, immutable_key))

    def delete(self, key: bytes) -> bool:
        if key not in self.entries:
            return False
        del self.entries[key]
        return True

    def expire_lazy(self, key: bytes) -> None:
        self.get(key)

    def active_expire(self) -> None:
        now = _now_ms()
        while self.ttl_heap and self.ttl_heap[0][0] <= now:
            expires_at, key = heapq.heappop(self.ttl_heap)
            entry = self.entries.get(key)
            if entry is not None and entry.expires_at_ms == expires_at:
                self.delete(key)

    def keys(self, pattern: bytes) -> list[bytes]:
        self.active_expire()
        return sorted(key for key in self.entries if _glob_match(pattern, key))

    def clear(self) -> None:
        self.entries.clear()
        self.ttl_heap.clear()


class Store:
    """A fixed collection of logical databases."""

    def __init__(self, database_count: int = 16) -> None:
        if database_count <= 0:
            raise ValueError("database_count must be positive")
        self.databases = [Database() for _ in range(database_count)]
        self.active_db = 0

    @property
    def active_database(self) -> Database:
        return self.databases[self.active_db]

    def select(self, index: int) -> None:
        self.active_db = index

    def flushdb(self) -> None:
        self.active_database.clear()

    def flushall(self) -> None:
        for database in self.databases:
            database.clear()


class DataStoreEngine:
    """Execute protocol command frames against an in-memory :class:`Store`."""

    def __init__(self, store: Store | None = None) -> None:
        self.store = store or Store()
        self._commands: dict[str, Callable[[tuple[bytes, ...]], EngineResponse]] = {
            "PING": self._ping,
            "ECHO": self._echo,
            "SET": self._set,
            "GET": self._get,
            "DEL": self._delete,
            "EXISTS": self._exists,
            "KEYS": self._keys,
            "TYPE": self._type,
            "RENAME": self._rename,
            "APPEND": self._append,
            "INCR": lambda args: self._incr_by(args, 1, "incr"),
            "DECR": lambda args: self._incr_by(args, -1, "decr"),
            "INCRBY": lambda args: self._incr_by(args, None, "incrby"),
            "DECRBY": self._decr_by,
            "HSET": self._hset,
            "HGET": self._hget,
            "HDEL": self._hdel,
            "HGETALL": self._hgetall,
            "HLEN": self._hlen,
            "HEXISTS": self._hexists,
            "HKEYS": self._hkeys,
            "HVALS": self._hvals,
            "LPUSH": lambda args: self._push_list(args, True),
            "RPUSH": lambda args: self._push_list(args, False),
            "LPOP": lambda args: self._pop_list(args, True),
            "RPOP": lambda args: self._pop_list(args, False),
            "LLEN": self._llen,
            "LINDEX": self._lindex,
            "LRANGE": self._lrange,
            "SADD": self._sadd,
            "SREM": self._srem,
            "SISMEMBER": self._sismember,
            "SMEMBERS": self._smembers,
            "SCARD": self._scard,
            "SUNION": lambda args: self._set_operation(args, "sunion", "union"),
            "SINTER": lambda args: self._set_operation(args, "sinter", "intersection"),
            "SDIFF": lambda args: self._set_operation(args, "sdiff", "difference"),
            "ZADD": self._zadd,
            "ZRANGE": self._zrange,
            "ZRANGEBYSCORE": self._zrange_by_score,
            "ZRANK": self._zrank,
            "ZSCORE": self._zscore,
            "ZCARD": self._zcard,
            "ZREM": self._zrem,
            "PFADD": self._pfadd,
            "PFCOUNT": self._pfcount,
            "PFMERGE": self._pfmerge,
            "EXPIRE": lambda args: self._expire(args, False),
            "EXPIREAT": lambda args: self._expire(args, True),
            "TTL": self._ttl,
            "PTTL": self._pttl,
            "PERSIST": self._persist,
            "SELECT": self._select,
            "FLUSHDB": self._flushdb,
            "FLUSHALL": self._flushall,
            "DBSIZE": self._dbsize,
            "INFO": self._info,
        }

    @staticmethod
    def current_time_ms() -> int:
        return _now_ms()

    def execute_frame(self, frame: CommandFrame | None) -> EngineResponse:
        if frame is None:
            return _error("ERR protocol error: expected array of bulk strings")
        self.store.active_database.active_expire()
        handler = self._commands.get(frame.command.upper())
        if handler is None:
            return _error(f"ERR unknown command '{frame.command.lower()}'")
        return handler(frame.args)

    def execute_parts(self, parts: list[bytes] | tuple[bytes, ...]) -> EngineResponse:
        return self.execute_frame(CommandFrame.from_parts(parts))

    def _ping(self, args: tuple[bytes, ...]) -> EngineResponse:
        if not args:
            return EngineResponse.simple_string("PONG")
        if len(args) == 1:
            return _bulk(args[0])
        return _wrong_arity("ping")

    def _echo(self, args: tuple[bytes, ...]) -> EngineResponse:
        return _bulk(args[0]) if len(args) == 1 else _wrong_arity("echo")

    def _set(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) != 2:
            return _wrong_arity("set")
        self.store.active_database.set(args[0], Entry(EntryType.STRING, bytes(args[1])))
        return EngineResponse.ok()

    def _get(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) != 1:
            return _wrong_arity("get")
        entry = self._key_entry(args[0])
        if entry is None:
            return EngineResponse.null()
        if entry.type is not EntryType.STRING:
            return _wrong_type()
        return _bulk(entry.value)

    def _delete(self, args: tuple[bytes, ...]) -> EngineResponse:
        if not args:
            return _wrong_arity("del")
        return _integer(sum(self.store.active_database.delete(key) for key in args))

    def _exists(self, args: tuple[bytes, ...]) -> EngineResponse:
        if not args:
            return _wrong_arity("exists")
        return _integer(sum(self._key_entry(key) is not None for key in args))

    def _keys(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) != 1:
            return _wrong_arity("keys")
        return _array(_bulk(key) for key in self.store.active_database.keys(args[0]))

    def _type(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) != 1:
            return _wrong_arity("type")
        entry = self._key_entry(args[0])
        return EngineResponse.simple_string(entry.type.value if entry else "none")

    def _rename(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) != 2:
            return _wrong_arity("rename")
        source, destination = args
        entry = self._key_entry(source)
        if entry is None:
            return _error("ERR no such key")
        if source != destination:
            self.store.active_database.delete(source)
            self.store.active_database.set(destination, entry)
        return EngineResponse.ok()

    def _append(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) != 2:
            return _wrong_arity("append")
        key, suffix = args
        entry = self._key_entry(key)
        if entry is None:
            self.store.active_database.set(key, Entry(EntryType.STRING, bytes(suffix)))
            return _integer(len(suffix))
        if entry.type is not EntryType.STRING:
            return _wrong_type()
        entry.value += suffix
        return _integer(len(entry.value))

    def _incr_by(
        self,
        args: tuple[bytes, ...],
        fixed_delta: int | None,
        command: str,
    ) -> EngineResponse:
        expected = 2 if fixed_delta is None else 1
        if len(args) != expected:
            return _wrong_arity(command)
        delta = fixed_delta if fixed_delta is not None else _parse_i64(args[1])
        if delta is None:
            return _integer_error()
        entry = self._key_entry(args[0])
        if entry is not None and entry.type is not EntryType.STRING:
            return _wrong_type()
        current = 0 if entry is None else _parse_i64(entry.value)
        if current is None:
            return _integer_error()
        result = current + delta
        if not _I64_MIN <= result <= _I64_MAX:
            return _integer_error()
        expiry = None if entry is None else entry.expires_at_ms
        self.store.active_database.set(
            args[0], Entry(EntryType.STRING, str(result).encode(), expiry)
        )
        return _integer(result)

    def _decr_by(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) != 2:
            return _wrong_arity("decrby")
        delta = _parse_i64(args[1])
        if delta is None or delta == _I64_MIN:
            return _integer_error()
        return self._incr_by((args[0], str(-delta).encode()), None, "decrby")

    def _hset(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) < 3 or len(args) % 2 == 0:
            return _wrong_arity("hset")
        entry = self._key_entry(args[0])
        if entry is None:
            entry = Entry(EntryType.HASH, {})
            self.store.active_database.set(args[0], entry)
        if entry.type is not EntryType.HASH:
            return _wrong_type()
        added = 0
        for index in range(1, len(args), 2):
            field = bytes(args[index])
            added += field not in entry.value
            entry.value[field] = bytes(args[index + 1])
        return _integer(added)

    def _hget(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) != 2:
            return _wrong_arity("hget")
        entry = self._key_entry(args[0])
        if entry is None:
            return EngineResponse.null()
        if entry.type is not EntryType.HASH:
            return _wrong_type()
        return _bulk(entry.value.get(args[1]))

    def _hdel(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) < 2:
            return _wrong_arity("hdel")
        entry = self._key_entry(args[0])
        if entry is None:
            return EngineResponse.zero()
        if entry.type is not EntryType.HASH:
            return _wrong_type()
        removed = 0
        for field in args[1:]:
            if field in entry.value:
                del entry.value[field]
                removed += 1
        if not entry.value:
            self.store.active_database.delete(args[0])
        return _integer(removed)

    def _hgetall(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) != 1:
            return _wrong_arity("hgetall")
        entry = self._key_entry(args[0])
        if entry is None:
            return _array([])
        if entry.type is not EntryType.HASH:
            return _wrong_type()
        values: list[EngineResponse] = []
        for field in sorted(entry.value):
            values.extend((_bulk(field), _bulk(entry.value[field])))
        return _array(values)

    def _hlen(self, args: tuple[bytes, ...]) -> EngineResponse:
        return self._hash_count(args, "hlen", lambda value: len(value))

    def _hexists(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) != 2:
            return _wrong_arity("hexists")
        entry = self._key_entry(args[0])
        if entry is None:
            return EngineResponse.zero()
        if entry.type is not EntryType.HASH:
            return _wrong_type()
        return _integer(args[1] in entry.value)

    def _hkeys(self, args: tuple[bytes, ...]) -> EngineResponse:
        return self._hash_array(args, "hkeys", lambda value: sorted(value))

    def _hvals(self, args: tuple[bytes, ...]) -> EngineResponse:
        return self._hash_array(args, "hvals", lambda value: [value[key] for key in sorted(value)])

    def _hash_count(
        self,
        args: tuple[bytes, ...],
        command: str,
        function: Callable[[dict[bytes, bytes]], int],
    ) -> EngineResponse:
        if len(args) != 1:
            return _wrong_arity(command)
        entry = self._key_entry(args[0])
        if entry is None:
            return EngineResponse.zero()
        if entry.type is not EntryType.HASH:
            return _wrong_type()
        return _integer(function(entry.value))

    def _hash_array(
        self,
        args: tuple[bytes, ...],
        command: str,
        function: Callable[[dict[bytes, bytes]], list[bytes]],
    ) -> EngineResponse:
        if len(args) != 1:
            return _wrong_arity(command)
        entry = self._key_entry(args[0])
        if entry is None:
            return _array([])
        if entry.type is not EntryType.HASH:
            return _wrong_type()
        return _array(_bulk(value) for value in function(entry.value))

    def _push_list(self, args: tuple[bytes, ...], left: bool) -> EngineResponse:
        command = "lpush" if left else "rpush"
        if len(args) < 2:
            return _wrong_arity(command)
        entry = self._ensure_collection(args[0], EntryType.LIST, list)
        if entry is None:
            return _wrong_type()
        for value in args[1:]:
            entry.value.insert(0, bytes(value)) if left else entry.value.append(bytes(value))
        return _integer(len(entry.value))

    def _pop_list(self, args: tuple[bytes, ...], left: bool) -> EngineResponse:
        command = "lpop" if left else "rpop"
        if len(args) != 1:
            return _wrong_arity(command)
        entry = self._key_entry(args[0])
        if entry is None:
            return EngineResponse.null()
        if entry.type is not EntryType.LIST:
            return _wrong_type()
        value = entry.value.pop(0 if left else -1)
        if not entry.value:
            self.store.active_database.delete(args[0])
        return _bulk(value)

    def _llen(self, args: tuple[bytes, ...]) -> EngineResponse:
        return self._typed_length(args, "llen", EntryType.LIST)

    def _lindex(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) != 2:
            return _wrong_arity("lindex")
        entry = self._key_entry(args[0])
        if entry is None:
            return EngineResponse.null()
        if entry.type is not EntryType.LIST:
            return _wrong_type()
        index = _parse_i64(args[1])
        if index is None:
            return _integer_error()
        resolved = len(entry.value) + index if index < 0 else index
        if 0 <= resolved < len(entry.value):
            return _bulk(entry.value[resolved])
        return EngineResponse.null()

    def _lrange(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) != 3:
            return _wrong_arity("lrange")
        entry = self._key_entry(args[0])
        if entry is None:
            return _array([])
        if entry.type is not EntryType.LIST:
            return _wrong_type()
        start, stop = _parse_i64(args[1]), _parse_i64(args[2])
        if start is None or stop is None:
            return _integer_error()
        length = len(entry.value)
        start = max(0, length + start if start < 0 else start)
        stop = min(length - 1, length + stop if stop < 0 else stop)
        if length == 0 or start > stop or start >= length:
            return _array([])
        return _array(_bulk(value) for value in entry.value[start : stop + 1])

    def _sadd(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) < 2:
            return _wrong_arity("sadd")
        entry = self._ensure_collection(args[0], EntryType.SET, set)
        if entry is None:
            return _wrong_type()
        before = len(entry.value)
        entry.value.update(bytes(value) for value in args[1:])
        return _integer(len(entry.value) - before)

    def _srem(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) < 2:
            return _wrong_arity("srem")
        entry = self._key_entry(args[0])
        if entry is None:
            return EngineResponse.zero()
        if entry.type is not EntryType.SET:
            return _wrong_type()
        removed = 0
        for value in args[1:]:
            if value in entry.value:
                entry.value.remove(value)
                removed += 1
        if not entry.value:
            self.store.active_database.delete(args[0])
        return _integer(removed)

    def _sismember(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) != 2:
            return _wrong_arity("sismember")
        entry = self._key_entry(args[0])
        if entry is None:
            return EngineResponse.zero()
        if entry.type is not EntryType.SET:
            return _wrong_type()
        return _integer(args[1] in entry.value)

    def _smembers(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) != 1:
            return _wrong_arity("smembers")
        entry = self._key_entry(args[0])
        if entry is None:
            return _array([])
        if entry.type is not EntryType.SET:
            return _wrong_type()
        return _array(_bulk(value) for value in sorted(entry.value))

    def _scard(self, args: tuple[bytes, ...]) -> EngineResponse:
        return self._typed_length(args, "scard", EntryType.SET)

    def _set_operation(
        self, args: tuple[bytes, ...], command: str, operation: str
    ) -> EngineResponse:
        if not args:
            return _wrong_arity(command)
        sets: list[set[bytes]] = []
        for key in args:
            entry = self._key_entry(key)
            if entry is None:
                sets.append(set())
            elif entry.type is EntryType.SET:
                sets.append(set(entry.value))
            else:
                return _wrong_type()
        result = sets[0]
        for value in sets[1:]:
            if operation == "union":
                result |= value
            elif operation == "intersection":
                result &= value
            else:
                result -= value
        return _array(_bulk(value) for value in sorted(result))

    def _zadd(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) < 3 or len(args) % 2 == 0:
            return _wrong_arity("zadd")
        parsed: list[tuple[float, bytes]] = []
        for index in range(1, len(args), 2):
            score = _parse_float(args[index])
            if score is None:
                return _float_error()
            parsed.append((score, args[index + 1]))
        entry = self._key_entry(args[0])
        if entry is None:
            entry = Entry(EntryType.ZSET, SortedSet())
            self.store.active_database.set(args[0], entry)
        if entry.type is not EntryType.ZSET:
            return _wrong_type()
        return _integer(sum(entry.value.insert(score, member) for score, member in parsed))

    def _zrange(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) not in (3, 4):
            return _wrong_arity("zrange")
        start, end = _parse_i64(args[1]), _parse_i64(args[2])
        if start is None or end is None:
            return _integer_error()
        entry = self._key_entry(args[0])
        if entry is None:
            return _array([])
        if entry.type is not EntryType.ZSET:
            return _wrong_type()
        with_scores = len(args) == 4 and args[3].upper() == b"WITHSCORES"
        return _array(self._flatten_zset(entry.value.range_by_index(start, end), with_scores))

    def _zrange_by_score(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) not in (3, 4):
            return _wrong_arity("zrangebyscore")
        minimum, maximum = _parse_float(args[1]), _parse_float(args[2])
        if minimum is None or maximum is None:
            return _float_error()
        entry = self._key_entry(args[0])
        if entry is None:
            return _array([])
        if entry.type is not EntryType.ZSET:
            return _wrong_type()
        with_scores = len(args) == 4 and args[3].upper() == b"WITHSCORES"
        return _array(self._flatten_zset(entry.value.range_by_score(minimum, maximum), with_scores))

    def _zrank(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) != 2:
            return _wrong_arity("zrank")
        entry = self._key_entry(args[0])
        if entry is None:
            return EngineResponse.null()
        if entry.type is not EntryType.ZSET:
            return _wrong_type()
        rank = entry.value.rank(args[1])
        return EngineResponse.null() if rank is None else _integer(rank)

    def _zscore(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) != 2:
            return _wrong_arity("zscore")
        entry = self._key_entry(args[0])
        if entry is None:
            return EngineResponse.null()
        if entry.type is not EntryType.ZSET:
            return _wrong_type()
        score = entry.value.score(args[1])
        return EngineResponse.null() if score is None else _bulk(_format_score(score).encode())

    def _zcard(self, args: tuple[bytes, ...]) -> EngineResponse:
        return self._typed_length(args, "zcard", EntryType.ZSET)

    def _zrem(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) < 2:
            return _wrong_arity("zrem")
        entry = self._key_entry(args[0])
        if entry is None:
            return EngineResponse.zero()
        if entry.type is not EntryType.ZSET:
            return _wrong_type()
        removed = sum(entry.value.remove(member) for member in args[1:])
        if len(entry.value) == 0:
            self.store.active_database.delete(args[0])
        return _integer(removed)

    def _pfadd(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) < 2:
            return _wrong_arity("pfadd")
        entry = self._key_entry(args[0])
        if entry is None:
            entry = Entry(EntryType.HLL, HyperLogLog())
            self.store.active_database.set(args[0], entry)
        if entry.type is not EntryType.HLL:
            return _wrong_type()
        before = tuple(entry.value._registers)
        for value in args[1:]:
            entry.value.add(value)
        return _integer(before != tuple(entry.value._registers))

    def _pfcount(self, args: tuple[bytes, ...]) -> EngineResponse:
        if not args:
            return _wrong_arity("pfcount")
        aggregate: HyperLogLog | None = None
        for key in args:
            entry = self._key_entry(key)
            if entry is None:
                continue
            if entry.type is not EntryType.HLL:
                return _wrong_type()
            aggregate = entry.value if aggregate is None else aggregate.merge(entry.value)
        return _integer(0 if aggregate is None else aggregate.count())

    def _pfmerge(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) < 2:
            return _wrong_arity("pfmerge")
        aggregate: HyperLogLog | None = None
        for key in args[1:]:
            entry = self._key_entry(key)
            if entry is None:
                continue
            if entry.type is not EntryType.HLL:
                return _wrong_type()
            aggregate = entry.value if aggregate is None else aggregate.merge(entry.value)
        destination = self._key_entry(args[0])
        expiry = None if destination is None else destination.expires_at_ms
        self.store.active_database.set(
            args[0], Entry(EntryType.HLL, aggregate or HyperLogLog(), expiry)
        )
        return EngineResponse.ok()

    def _expire(self, args: tuple[bytes, ...], absolute: bool) -> EngineResponse:
        command = "expireat" if absolute else "expire"
        if len(args) != 2:
            return _wrong_arity(command)
        entry = self._key_entry(args[0])
        if entry is None:
            return EngineResponse.zero()
        seconds = _parse_i64(args[1])
        if seconds is None:
            return _integer_error()
        expires_at = seconds * 1000 if absolute else _now_ms() + seconds * 1000
        entry.expires_at_ms = expires_at
        heapq.heappush(self.store.active_database.ttl_heap, (expires_at, bytes(args[0])))
        return EngineResponse.one()

    def _ttl(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) != 1:
            return _wrong_arity("ttl")
        entry = self._key_entry(args[0])
        if entry is None:
            return _integer(-2)
        if entry.expires_at_ms is None:
            return _integer(-1)
        return _integer(max(-2, (entry.expires_at_ms - _now_ms()) // 1000))

    def _pttl(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) != 1:
            return _wrong_arity("pttl")
        entry = self._key_entry(args[0])
        if entry is None:
            return _integer(-2)
        if entry.expires_at_ms is None:
            return _integer(-1)
        return _integer(max(-1, entry.expires_at_ms - _now_ms()))

    def _persist(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) != 1:
            return _wrong_arity("persist")
        entry = self._key_entry(args[0])
        if entry is None or entry.expires_at_ms is None:
            return EngineResponse.zero()
        entry.expires_at_ms = None
        return EngineResponse.one()

    def _select(self, args: tuple[bytes, ...]) -> EngineResponse:
        if len(args) != 1:
            return _wrong_arity("select")
        index = _parse_i64(args[0])
        if index is None or not 0 <= index < len(self.store.databases):
            return _error("ERR DB index is out of range")
        self.store.select(index)
        return EngineResponse.ok()

    def _flushdb(self, args: tuple[bytes, ...]) -> EngineResponse:
        if args:
            return _wrong_arity("flushdb")
        self.store.flushdb()
        return EngineResponse.ok()

    def _flushall(self, args: tuple[bytes, ...]) -> EngineResponse:
        if args:
            return _wrong_arity("flushall")
        self.store.flushall()
        return EngineResponse.ok()

    def _dbsize(self, args: tuple[bytes, ...]) -> EngineResponse:
        if args:
            return _wrong_arity("dbsize")
        self.store.active_database.active_expire()
        return _integer(len(self.store.active_database.entries))

    def _info(self, args: tuple[bytes, ...]) -> EngineResponse:
        if args:
            return _wrong_arity("info")
        size = len(self.store.active_database.entries)
        text = (
            "# Server\r\nin_memory_data_store_version:0.1.0\r\n"
            f"active_db:{self.store.active_db}\r\ndbsize:{size}\r\n"
        )
        return _bulk(text.encode())

    def _key_entry(self, key: bytes) -> Entry | None:
        return self.store.active_database.get(key)

    def _ensure_collection(
        self, key: bytes, entry_type: EntryType, factory: Callable[[], Any]
    ) -> Entry | None:
        entry = self._key_entry(key)
        if entry is None:
            entry = Entry(entry_type, factory())
            self.store.active_database.set(key, entry)
        return entry if entry.type is entry_type else None

    def _typed_length(
        self, args: tuple[bytes, ...], command: str, entry_type: EntryType
    ) -> EngineResponse:
        if len(args) != 1:
            return _wrong_arity(command)
        entry = self._key_entry(args[0])
        if entry is None:
            return EngineResponse.zero()
        if entry.type is not entry_type:
            return _wrong_type()
        return _integer(len(entry.value))

    @staticmethod
    def _flatten_zset(values: list[tuple[bytes, float]], with_scores: bool) -> list[EngineResponse]:
        result: list[EngineResponse] = []
        for member, score in values:
            result.append(_bulk(member))
            if with_scores:
                result.append(_bulk(_format_score(score).encode()))
        return result


def _now_ms() -> int:
    return time.time_ns() // 1_000_000


def _glob_match(pattern: bytes, value: bytes) -> bool:
    pattern_index = 0
    value_index = 0
    star_index = -1
    retry_value_index = 0
    while value_index < len(value):
        if pattern_index < len(pattern) and pattern[pattern_index] in {
            ord("?"),
            value[value_index],
        }:
            pattern_index += 1
            value_index += 1
        elif pattern_index < len(pattern) and pattern[pattern_index] == ord("*"):
            star_index = pattern_index
            retry_value_index = value_index
            pattern_index += 1
        elif star_index != -1:
            retry_value_index += 1
            value_index = retry_value_index
            pattern_index = star_index + 1
        else:
            return False
    while pattern_index < len(pattern) and pattern[pattern_index] == ord("*"):
        pattern_index += 1
    return pattern_index == len(pattern)


def _parse_i64(value: bytes) -> int | None:
    try:
        parsed = int(value.decode("ascii"))
    except (UnicodeDecodeError, ValueError):
        return None
    return parsed if _I64_MIN <= parsed <= _I64_MAX else None


def _parse_float(value: bytes) -> float | None:
    try:
        parsed = float(value.decode("ascii"))
    except (UnicodeDecodeError, ValueError):
        return None
    return parsed if math.isfinite(parsed) else None


def _format_score(score: float) -> str:
    decimal = Decimal(str(score)).normalize()
    return format(decimal, "f")


def _bulk(value: bytes | None) -> EngineResponse:
    return EngineResponse.bulk_string(value)


def _integer(value: int | bool) -> EngineResponse:
    return EngineResponse.integer(int(value))


def _array(values: Any) -> EngineResponse:
    return EngineResponse.array(list(values))


def _error(message: str) -> EngineResponse:
    return EngineResponse.error(message)


def _wrong_arity(command: str) -> EngineResponse:
    return _error(f"ERR wrong number of arguments for '{command}' command")


def _wrong_type() -> EngineResponse:
    return _error("WRONGTYPE Operation against a key holding the wrong kind of value")


def _integer_error() -> EngineResponse:
    return _error("ERR value is not an integer or out of range")


def _float_error() -> EngineResponse:
    return _error("ERR value is not a valid float")
