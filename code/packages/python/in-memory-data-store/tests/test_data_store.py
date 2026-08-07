from __future__ import annotations

from pathlib import Path

import pytest
from in_memory_data_store_engine import DataStoreEngine, Store
from in_memory_data_store_protocol import CommandFrame, EngineResponse
from resp_protocol import RespError, decode, decode_all, encode

from in_memory_data_store import (
    AofSyncPolicy,
    DataStoreManager,
    InMemoryDataStore,
    create_in_memory_data_store,
    encode_engine_response,
    encode_resp_stream,
    response_to_resp_value,
)


def command(*parts: str) -> bytes:
    return encode([part.encode() for part in parts])


def decoded(value: bytes) -> object:
    result, consumed = decode(value)
    assert consumed == len(value)
    return result


def test_resp_execution_composition_and_streaming() -> None:
    store = InMemoryDataStore()
    assert store.execute_resp_bytes(command("PING")) == b"+PONG\r\n"
    assert store.execute_resp_bytes(command("SET", "alpha", "1")) == b"+OK\r\n"
    assert decoded(store.execute_resp_bytes(command("GET", "alpha"))) == b"1"
    assert store.execute_resp_bytes(b"*2\r\n") == b"-ERR incomplete RESP frame\r\n"
    assert store.execute_resp_value(b"not-an-array").kind == "error"
    assert store.execute_resp_value([b"SET", 1]).kind == "error"
    assert store.execute_resp_value(["PING"]).value == "PONG"

    stream = command("INCR", "count") + command("INCR", "count")
    responses = store.process(stream)
    assert [response.value for response in responses] == [1, 2]
    assert store.handle(command("PING") + command("GET", "count")) == (b"+PONG\r\n$1\r\n2\r\n")
    assert store.store is store.engine.store
    store.close()


def test_constructor_alias_factory_and_context_manager() -> None:
    engine = DataStoreEngine()
    with pytest.raises(ValueError, match="mutually exclusive"):
        InMemoryDataStore(engine=engine, store=Store())
    created = create_in_memory_data_store(engine=engine)
    assert isinstance(created, InMemoryDataStore)
    assert DataStoreManager is InMemoryDataStore
    with created as managed:
        assert managed.execute_frame(CommandFrame.new("PING")).value == "PONG"


def test_response_conversions_cover_all_resp_types() -> None:
    responses = [
        EngineResponse.simple_string("OK"),
        EngineResponse.error("ERR bad"),
        EngineResponse.integer(42),
        EngineResponse.bulk_string(b"data"),
        EngineResponse.bulk_string(None),
        EngineResponse.array([EngineResponse.integer(1), EngineResponse.bulk_string(b"x")]),
        EngineResponse.array(None),
    ]
    assert encode_engine_response(responses[0]) == b"+OK\r\n"
    assert encode_engine_response(responses[1]) == b"-ERR bad\r\n"
    assert encode_engine_response(responses[2]) == b":42\r\n"
    assert encode_engine_response(responses[3]) == b"$4\r\ndata\r\n"
    assert encode_engine_response(responses[4]) == b"$-1\r\n"
    assert encode_engine_response(responses[5]) == b"*2\r\n:1\r\n$1\r\nx\r\n"
    assert encode_engine_response(responses[6]) == b"*-1\r\n"
    assert isinstance(response_to_resp_value(responses[1]), RespError)
    assert response_to_resp_value(responses[0]) == "OK"
    assert response_to_resp_value(responses[2]) == 42
    assert response_to_resp_value(responses[4]) is None
    assert response_to_resp_value(responses[5]) == [1, b"x"]
    assert response_to_resp_value(responses[6]) is None
    assert encode_resp_stream(responses[:2]) == b"+OK\r\n-ERR bad\r\n"


def test_aof_replays_successful_mutations_and_canonicalizes_expire(
    tmp_path: Path,
) -> None:
    aof = tmp_path / "nested" / "appendonly.aof"
    with InMemoryDataStore(aof_path=aof) as store:
        store.execute_parts([b"SET", b"persistent", b"yes"])
        store.execute_parts([b"INCR", b"count"])
        store.execute_parts([b"SELECT", b"1"])
        store.execute_parts([b"SET", b"db1", b"value"])
        store.execute_parts([b"SELECT", b"0"])
        store.execute_parts([b"SET", b"ttl", b"value"])
        store.execute_parts([b"EXPIRE", b"ttl", b"60"])
        store.execute_parts([b"GET", b"persistent"])
        store.execute_parts([b"UNKNOWN"])

    frames, consumed = decode_all(aof.read_bytes())
    assert consumed == len(aof.read_bytes())
    assert frames[0] == [b"SET", b"persistent", b"yes"]
    assert [b"SELECT", b"1"] in frames
    assert any(frame[0] == b"EXPIREAT" and frame[1] == b"ttl" for frame in frames)
    assert not any(frame[0] in {b"GET", b"UNKNOWN"} for frame in frames)

    with InMemoryDataStore(aof_path=aof) as replayed:
        assert replayed.execute_parts([b"GET", b"persistent"]).value == b"yes"
        assert replayed.execute_parts([b"GET", b"count"]).value == b"1"
        replayed.execute_parts([b"SELECT", b"1"])
        assert replayed.execute_parts([b"GET", b"db1"]).value == b"value"


def test_aof_none_sync_policy_and_empty_replay(tmp_path: Path) -> None:
    aof = tmp_path / "appendonly.aof"
    store = InMemoryDataStore(aof_path=aof, aof_sync_policy=AofSyncPolicy.NONE)
    store.execute_parts([b"SET", b"key", b"value"])
    store.close()
    store.close()
    assert aof.exists()
    aof.write_bytes(aof.read_bytes() + b"*2\r\n")
    with InMemoryDataStore(aof_path=aof) as replayed:
        assert replayed.execute_parts([b"GET", b"key"]).value == b"value"
