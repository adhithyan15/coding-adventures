from __future__ import annotations

from typing import cast

import pytest
from in_memory_data_store_protocol import CommandFrame, EngineResponse

import in_memory_data_store_engine.engine as engine_module
from in_memory_data_store_engine import (
    Database,
    DataStoreEngine,
    Entry,
    EntryType,
    SortedSet,
    Store,
)


def execute(engine: DataStoreEngine, *parts: str | bytes) -> EngineResponse:
    encoded = [part if isinstance(part, bytes) else part.encode() for part in parts]
    return engine.execute_parts(encoded)


def values(response: EngineResponse) -> list[bytes | int | str | None]:
    assert response.kind == "array"
    assert isinstance(response.value, tuple)
    return cast(list[bytes | int | str | None], [item.value for item in response.value])


def assert_error(response: EngineResponse, contains: str) -> None:
    assert response.kind == "error"
    assert contains in str(response.value)


def test_protocol_strings_numbers_and_keyspace() -> None:
    engine = DataStoreEngine()
    assert execute(engine, "PING") == EngineResponse.simple_string("PONG")
    assert execute(engine, "PING", "hello") == EngineResponse.bulk_string(b"hello")
    assert execute(engine, "ECHO", b"\x00binary") == EngineResponse.bulk_string(b"\x00binary")

    assert execute(engine, "SET", "user:1", "40") == EngineResponse.ok()
    assert execute(engine, "GET", "user:1").value == b"40"
    assert execute(engine, "EXISTS", "user:1", "missing").value == 1
    assert execute(engine, "TYPE", "user:1").value == "string"
    assert execute(engine, "TYPE", "missing").value == "none"
    assert execute(engine, "INCR", "user:1").value == 41
    assert execute(engine, "INCRBY", "user:1", "2").value == 43
    assert execute(engine, "DECR", "user:1").value == 42
    assert execute(engine, "DECRBY", "user:1", "2").value == 40
    assert execute(engine, "APPEND", "user:1", "!").value == 3
    assert execute(engine, "APPEND", "new", "abc").value == 3

    execute(engine, "SET", "user:2", "Lin")
    assert execute(engine, "KEYS", "user:?").value is not None
    assert values(execute(engine, "KEYS", "user:*")) == [b"user:1", b"user:2"]
    assert execute(engine, "RENAME", "user:2", "user:two") == EngineResponse.ok()
    assert execute(engine, "RENAME", "user:two", "user:two") == EngineResponse.ok()
    assert execute(engine, "GET", "user:two").value == b"Lin"
    execute(engine, "SET", "literal[1]", "yes")
    assert values(execute(engine, "KEYS", "literal[1]")) == [b"literal[1]"]
    assert execute(engine, "DEL", "user:1", "missing").value == 1
    assert execute(engine, "GET", "user:1") == EngineResponse.null()


def test_hash_and_list_commands() -> None:
    engine = DataStoreEngine()
    assert execute(engine, "HSET", "user", "name", "Ada", "city", "London").value == 2
    assert execute(engine, "HSET", "user", "name", "Augusta").value == 0
    assert execute(engine, "HGET", "user", "name").value == b"Augusta"
    assert execute(engine, "HGET", "user", "missing") == EngineResponse.null()
    assert execute(engine, "HEXISTS", "user", "city").value == 1
    assert execute(engine, "HLEN", "user").value == 2
    assert values(execute(engine, "HKEYS", "user")) == [b"city", b"name"]
    assert values(execute(engine, "HVALS", "user")) == [b"London", b"Augusta"]
    assert values(execute(engine, "HGETALL", "user")) == [
        b"city",
        b"London",
        b"name",
        b"Augusta",
    ]
    assert execute(engine, "HDEL", "user", "city", "missing").value == 1
    assert execute(engine, "HDEL", "user", "name").value == 1
    assert execute(engine, "HLEN", "user").value == 0
    assert values(execute(engine, "HGETALL", "missing")) == []

    assert execute(engine, "LPUSH", "queue", "b", "a").value == 2
    assert execute(engine, "RPUSH", "queue", "c").value == 3
    assert execute(engine, "LLEN", "queue").value == 3
    assert execute(engine, "LINDEX", "queue", "-1").value == b"c"
    assert execute(engine, "LINDEX", "queue", "99") == EngineResponse.null()
    assert values(execute(engine, "LRANGE", "queue", "0", "-1")) == [b"a", b"b", b"c"]
    assert values(execute(engine, "LRANGE", "queue", "9", "10")) == []
    assert execute(engine, "LPOP", "queue").value == b"a"
    assert execute(engine, "RPOP", "queue").value == b"c"
    assert execute(engine, "RPOP", "queue").value == b"b"
    assert execute(engine, "LPOP", "queue") == EngineResponse.null()


def test_sets_sorted_sets_and_hyperloglog() -> None:
    engine = DataStoreEngine()
    assert execute(engine, "SADD", "left", "a", "b", "c", "a").value == 3
    assert execute(engine, "SADD", "right", "b", "c", "d").value == 3
    assert execute(engine, "SISMEMBER", "left", "b").value == 1
    assert execute(engine, "SCARD", "left").value == 3
    assert values(execute(engine, "SMEMBERS", "left")) == [b"a", b"b", b"c"]
    assert values(execute(engine, "SUNION", "left", "right")) == [b"a", b"b", b"c", b"d"]
    assert values(execute(engine, "SINTER", "left", "right")) == [b"b", b"c"]
    assert values(execute(engine, "SDIFF", "left", "right")) == [b"a"]
    assert values(execute(engine, "SINTER", "left", "missing")) == []
    assert execute(engine, "SREM", "left", "a", "missing").value == 1
    assert execute(engine, "SREM", "left", "b", "c").value == 2
    assert execute(engine, "SCARD", "left").value == 0

    assert (
        execute(
            engine,
            "ZADD",
            "scores",
            "1",
            "alice",
            "2",
            "bob",
            "1.5",
            "cara",
        ).value
        == 3
    )
    assert execute(engine, "ZADD", "scores", "3", "alice").value == 0
    assert values(execute(engine, "ZRANGE", "scores", "0", "-1")) == [b"cara", b"bob", b"alice"]
    assert values(execute(engine, "ZRANGE", "scores", "0", "1", "WITHSCORES")) == [
        b"cara",
        b"1.5",
        b"bob",
        b"2",
    ]
    assert values(execute(engine, "ZRANGEBYSCORE", "scores", "1", "2")) == [b"cara", b"bob"]
    assert execute(engine, "ZRANK", "scores", "bob").value == 1
    assert execute(engine, "ZRANK", "scores", "missing") == EngineResponse.null()
    assert execute(engine, "ZSCORE", "scores", "cara").value == b"1.5"
    assert execute(engine, "ZSCORE", "scores", "missing") == EngineResponse.null()
    assert execute(engine, "ZCARD", "scores").value == 3
    assert execute(engine, "ZREM", "scores", "bob", "missing").value == 1
    assert execute(engine, "ZREM", "scores", "cara", "alice").value == 2
    assert execute(engine, "ZCARD", "scores").value == 0

    assert execute(engine, "PFADD", "visitors", "alice", "bob").value == 1
    assert execute(engine, "PFADD", "visitors", "alice").value == 0
    assert execute(engine, "PFADD", "other", "cara").value == 1
    assert cast(int, execute(engine, "PFCOUNT", "visitors").value) >= 2
    assert cast(int, execute(engine, "PFCOUNT", "visitors", "other").value) >= 3
    assert execute(engine, "PFMERGE", "all", "visitors", "other") == EngineResponse.ok()
    assert cast(int, execute(engine, "PFCOUNT", "all").value) >= 3
    assert execute(engine, "PFCOUNT", "missing").value == 0


def test_expiry_database_and_admin_commands(monkeypatch: pytest.MonkeyPatch) -> None:
    now = 2_000_000
    monkeypatch.setattr(engine_module, "_now_ms", lambda: now)
    engine = DataStoreEngine()
    execute(engine, "SET", "temporary", "value")
    assert execute(engine, "TTL", "temporary").value == -1
    assert execute(engine, "PERSIST", "temporary").value == 0
    assert execute(engine, "EXPIRE", "temporary", "10").value == 1
    assert execute(engine, "TTL", "temporary").value == 10
    assert execute(engine, "PTTL", "temporary").value == 10_000
    assert execute(engine, "PERSIST", "temporary").value == 1
    assert execute(engine, "TTL", "temporary").value == -1
    assert execute(engine, "EXPIREAT", "temporary", "1999").value == 1
    assert execute(engine, "GET", "temporary") == EngineResponse.null()
    assert execute(engine, "TTL", "temporary").value == -2
    assert execute(engine, "EXPIRE", "missing", "1").value == 0

    execute(engine, "SET", "db0", "zero")
    assert execute(engine, "SELECT", "1") == EngineResponse.ok()
    execute(engine, "SET", "db1", "one")
    assert execute(engine, "DBSIZE").value == 1
    assert b"active_db:1" in cast(bytes, execute(engine, "INFO").value)
    assert execute(engine, "FLUSHDB") == EngineResponse.ok()
    assert execute(engine, "DBSIZE").value == 0
    execute(engine, "SET", "again", "one")
    assert execute(engine, "FLUSHALL") == EngineResponse.ok()
    execute(engine, "SELECT", "0")
    assert execute(engine, "DBSIZE").value == 0


@pytest.mark.parametrize(
    ("parts", "message"),
    [
        (("PING", "a", "b"), "wrong number"),
        (("ECHO",), "wrong number"),
        (("SET", "a"), "wrong number"),
        (("GET",), "wrong number"),
        (("DEL",), "wrong number"),
        (("EXISTS",), "wrong number"),
        (("KEYS",), "wrong number"),
        (("TYPE",), "wrong number"),
        (("RENAME", "a"), "wrong number"),
        (("APPEND", "a"), "wrong number"),
        (("INCRBY", "a"), "wrong number"),
        (("DECRBY", "a"), "wrong number"),
        (("HSET", "a", "b"), "wrong number"),
        (("HGET", "a"), "wrong number"),
        (("HDEL", "a"), "wrong number"),
        (("HGETALL",), "wrong number"),
        (("HLEN",), "wrong number"),
        (("HEXISTS", "a"), "wrong number"),
        (("HKEYS",), "wrong number"),
        (("HVALS",), "wrong number"),
        (("LPUSH", "a"), "wrong number"),
        (("LPOP", "a", "b"), "wrong number"),
        (("LLEN",), "wrong number"),
        (("LINDEX", "a"), "wrong number"),
        (("LRANGE", "a", "0"), "wrong number"),
        (("SADD", "a"), "wrong number"),
        (("SREM", "a"), "wrong number"),
        (("SISMEMBER", "a"), "wrong number"),
        (("SMEMBERS",), "wrong number"),
        (("SCARD",), "wrong number"),
        (("SUNION",), "wrong number"),
        (("ZADD", "a", "1"), "wrong number"),
        (("ZRANGE", "a", "0"), "wrong number"),
        (("ZRANGEBYSCORE", "a", "0"), "wrong number"),
        (("ZRANK", "a"), "wrong number"),
        (("ZSCORE", "a"), "wrong number"),
        (("ZCARD",), "wrong number"),
        (("ZREM", "a"), "wrong number"),
        (("PFADD", "a"), "wrong number"),
        (("PFCOUNT",), "wrong number"),
        (("PFMERGE", "a"), "wrong number"),
        (("EXPIRE", "a"), "wrong number"),
        (("TTL",), "wrong number"),
        (("PTTL",), "wrong number"),
        (("PERSIST",), "wrong number"),
        (("SELECT",), "wrong number"),
        (("FLUSHDB", "x"), "wrong number"),
        (("FLUSHALL", "x"), "wrong number"),
        (("DBSIZE", "x"), "wrong number"),
        (("INFO", "x"), "wrong number"),
    ],
)
def test_arity_errors(parts: tuple[str, ...], message: str) -> None:
    assert_error(execute(DataStoreEngine(), *parts), message)


def test_type_parse_and_command_errors() -> None:
    engine = DataStoreEngine()
    assert_error(engine.execute_frame(None), "protocol error")
    assert_error(execute(engine, "NOPE"), "unknown command")
    assert_error(execute(engine, "RENAME", "missing", "other"), "no such key")
    assert_error(execute(engine, "SELECT", "99"), "DB index")
    assert_error(execute(engine, "SELECT", "bad"), "DB index")

    execute(engine, "SET", "string", "value")
    for parts in [
        ("HGET", "string", "field"),
        ("HDEL", "string", "field"),
        ("HGETALL", "string"),
        ("HLEN", "string"),
        ("HEXISTS", "string", "field"),
        ("HKEYS", "string"),
        ("HVALS", "string"),
        ("LPUSH", "string", "value"),
        ("LPOP", "string"),
        ("LLEN", "string"),
        ("LINDEX", "string", "0"),
        ("LRANGE", "string", "0", "1"),
        ("SADD", "string", "value"),
        ("SREM", "string", "value"),
        ("SISMEMBER", "string", "value"),
        ("SMEMBERS", "string"),
        ("SCARD", "string"),
        ("ZADD", "string", "1", "value"),
        ("ZRANGE", "string", "0", "1"),
        ("ZRANGEBYSCORE", "string", "0", "1"),
        ("ZRANK", "string", "value"),
        ("ZSCORE", "string", "value"),
        ("ZCARD", "string"),
        ("ZREM", "string", "value"),
        ("PFADD", "string", "value"),
        ("PFCOUNT", "string"),
        ("PFMERGE", "hll", "string"),
        ("SUNION", "string"),
    ]:
        assert_error(execute(engine, *parts), "WRONGTYPE")

    assert_error(execute(engine, "INCR", "string"), "integer")
    assert_error(execute(engine, "INCRBY", "n", "bad"), "integer")
    assert_error(execute(engine, "DECRBY", "n", str(-(2**63))), "integer")
    execute(engine, "SET", "max", str(2**63 - 1))
    assert_error(execute(engine, "INCR", "max"), "integer")
    execute(engine, "RPUSH", "list", "a")
    assert_error(execute(engine, "LINDEX", "list", "bad"), "integer")
    assert_error(execute(engine, "LRANGE", "list", "bad", "1"), "integer")
    assert_error(execute(engine, "ZADD", "z", "nan", "a"), "float")
    assert_error(execute(engine, "ZRANGEBYSCORE", "z", "bad", "1"), "float")
    assert_error(execute(engine, "EXPIRE", "string", "bad"), "integer")


def test_storage_helpers_and_sorted_set_edges(monkeypatch: pytest.MonkeyPatch) -> None:
    with pytest.raises(ValueError, match="positive"):
        Store(0)
    store = Store(2)
    store.select(1)
    assert store.active_db == 1

    now = 50_000
    monkeypatch.setattr(engine_module, "_now_ms", lambda: now)
    database = Database()
    database.set(b"live", Entry(EntryType.STRING, b"yes"))
    database.set(b"old", Entry(EntryType.STRING, b"no", now - 1))
    assert database.get(b"missing") is None
    assert database.get(b"old") is None
    assert database.delete(b"missing") is False
    assert database.keys(b"l?ve") == [b"live"]
    database.expire_lazy(b"live")
    database.clear()
    assert database.entries == {}

    sorted_set = SortedSet()
    assert sorted_set.range_by_index(0, 1) == []
    assert sorted_set.insert(1.0, b"b") is True
    assert sorted_set.insert(1.0, b"a") is True
    assert sorted_set.insert(2.0, b"b") is False
    assert sorted_set.rank(b"a") == 0
    assert sorted_set.rank(b"missing") is None
    assert sorted_set.range_by_index(-99, 1) == []
    assert sorted_set.range_by_index(9, 10) == []
    assert sorted_set.range_by_score(0.0, 1.5) == [(b"a", 1.0)]
    assert sorted_set.remove(b"missing") is False
    assert sorted_set.remove(b"a") is True


def test_public_frame_and_time_helpers(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(engine_module, "_now_ms", lambda: 1234)
    engine = DataStoreEngine()
    assert engine.current_time_ms() == 1234
    frame = CommandFrame.new("ping")
    assert engine.execute_frame(frame).value == "PONG"
