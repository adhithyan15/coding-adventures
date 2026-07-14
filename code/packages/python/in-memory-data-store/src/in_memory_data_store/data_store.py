"""RESP composition and optional append-only persistence for the data store."""

from __future__ import annotations

import os
from collections.abc import Iterable
from enum import StrEnum
from pathlib import Path
from typing import cast

from in_memory_data_store_engine import DataStoreEngine, Store
from in_memory_data_store_protocol import CommandFrame, EngineResponse
from resp_protocol import (
    RespError,
    RespValue,
    decode,
    decode_all,
    encode_array,
    encode_bulk_string,
    encode_error,
    encode_integer,
    encode_simple_string,
)


class AofSyncPolicy(StrEnum):
    """Durability policy for append-only file writes."""

    ALWAYS = "always"
    NONE = "none"


class InMemoryDataStore:
    """Compose RESP decoding, command execution, encoding, and AOF replay."""

    def __init__(
        self,
        *,
        engine: DataStoreEngine | None = None,
        store: Store | None = None,
        aof_path: str | os.PathLike[str] | None = None,
        aof_sync_policy: AofSyncPolicy = AofSyncPolicy.ALWAYS,
    ) -> None:
        if engine is not None and store is not None:
            raise ValueError("engine and store are mutually exclusive")
        self.engine = engine or DataStoreEngine(store)
        self.aof_path = None if aof_path is None else Path(aof_path)
        self.aof_sync_policy = aof_sync_policy
        self._aof = None
        if self.aof_path is not None:
            self._replay_aof()
            self.aof_path.parent.mkdir(parents=True, exist_ok=True)
            self._aof = self.aof_path.open("ab")

    @property
    def store(self) -> Store:
        return self.engine.store

    def execute_frame(self, frame: CommandFrame | None) -> EngineResponse:
        response = self.engine.execute_frame(frame)
        self._append_to_aof(frame, response)
        return response

    def execute_parts(self, parts: list[bytes] | tuple[bytes, ...]) -> EngineResponse:
        return self.execute_frame(CommandFrame.from_parts(parts))

    def execute_resp_value(self, value: RespValue) -> EngineResponse:
        return self.execute_frame(_command_from_resp(value))

    def execute_resp_bytes(self, request: bytes) -> bytes:
        value, consumed = decode(request)
        if consumed == 0:
            return encode_error("ERR incomplete RESP frame")
        return encode_engine_response(self.execute_resp_value(value))

    def process(self, request: bytes) -> list[EngineResponse]:
        values, _ = decode_all(request)
        return [self.execute_resp_value(value) for value in values]

    def handle(self, request: bytes) -> bytes:
        return b"".join(encode_engine_response(value) for value in self.process(request))

    def close(self) -> None:
        if self._aof is not None:
            self._aof.close()
            self._aof = None

    def __enter__(self) -> InMemoryDataStore:
        return self

    def __exit__(self, *_: object) -> None:
        self.close()

    def _replay_aof(self) -> None:
        if self.aof_path is None or not self.aof_path.exists():
            return
        values, _ = decode_all(self.aof_path.read_bytes())
        for value in values:
            self.engine.execute_frame(_command_from_resp(value))

    def _append_to_aof(self, frame: CommandFrame | None, response: EngineResponse) -> None:
        if (
            frame is None
            or self._aof is None
            or response.kind == "error"
            or frame.command not in _AOF_COMMANDS
        ):
            return
        command = _canonical_command(frame)
        self._aof.write(encode_array(command.to_parts()))
        self._aof.flush()
        if self.aof_sync_policy is AofSyncPolicy.ALWAYS:
            os.fsync(self._aof.fileno())


DataStoreManager = InMemoryDataStore


def create_in_memory_data_store(
    *,
    engine: DataStoreEngine | None = None,
    store: Store | None = None,
    aof_path: str | os.PathLike[str] | None = None,
    aof_sync_policy: AofSyncPolicy = AofSyncPolicy.ALWAYS,
) -> InMemoryDataStore:
    """Create a composed data store with the supplied constructor options."""

    return InMemoryDataStore(
        engine=engine,
        store=store,
        aof_path=aof_path,
        aof_sync_policy=aof_sync_policy,
    )


def encode_engine_response(response: EngineResponse) -> bytes:
    """Encode the protocol response IR without losing RESP type distinctions."""

    if response.kind == "simple_string":
        return encode_simple_string(cast(str, response.value))
    if response.kind == "error":
        return encode_error(cast(str, response.value))
    if response.kind == "integer":
        return encode_integer(cast(int, response.value))
    if response.kind == "bulk_string":
        return encode_bulk_string(cast(bytes | None, response.value))
    values = cast(tuple[EngineResponse, ...] | None, response.value)
    if values is None:
        return b"*-1\r\n"
    encoded = b"".join(encode_engine_response(value) for value in values)
    return b"*" + str(len(values)).encode("ascii") + b"\r\n" + encoded


def response_to_resp_value(response: EngineResponse) -> RespValue:
    """Convert the response IR to the native values used by ``resp_protocol``."""

    if response.kind == "error":
        return RespError(cast(str, response.value))
    if response.kind in {"simple_string", "integer", "bulk_string"}:
        return response.value
    values = cast(tuple[EngineResponse, ...] | None, response.value)
    if values is None:
        return None
    return [response_to_resp_value(value) for value in values]


def encode_resp_stream(values: Iterable[EngineResponse]) -> bytes:
    """Encode several responses as one RESP byte stream."""

    return b"".join(encode_engine_response(value) for value in values)


def _command_from_resp(value: RespValue) -> CommandFrame | None:
    if not isinstance(value, list) or not value:
        return None
    parts: list[bytes] = []
    for part in value:
        if isinstance(part, bytes):
            parts.append(part)
        elif isinstance(part, str):
            parts.append(part.encode())
        else:
            return None
    return CommandFrame.from_parts(parts)


def _canonical_command(frame: CommandFrame) -> CommandFrame:
    if frame.command != "EXPIRE" or len(frame.args) != 2:
        return frame
    seconds = int(frame.args[1].decode("ascii"))
    absolute_seconds = DataStoreEngine.current_time_ms() // 1000 + seconds
    return CommandFrame.new("EXPIREAT", (frame.args[0], str(absolute_seconds).encode("ascii")))


_AOF_COMMANDS = frozenset(
    {
        "SET",
        "DEL",
        "RENAME",
        "INCR",
        "DECR",
        "INCRBY",
        "DECRBY",
        "APPEND",
        "HSET",
        "HDEL",
        "LPUSH",
        "RPUSH",
        "LPOP",
        "RPOP",
        "SADD",
        "SREM",
        "ZADD",
        "ZREM",
        "PFADD",
        "PFMERGE",
        "EXPIRE",
        "EXPIREAT",
        "PERSIST",
        "SELECT",
        "FLUSHDB",
        "FLUSHALL",
    }
)
