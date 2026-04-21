"""Python Mini Redis worker for the Rust TCP runtime prototype.

The Rust side owns the TCP listener, native event loop, socket buffers, RESP
framing, per-connection selected database state, and response writes. This
package owns only application command semantics.

The boundary intentionally mirrors ``code/packages/wasm/mini-redis``: Rust
turns protocol bytes into a command frame, the embedded command engine returns
an engine response, and Rust turns that response back into protocol bytes. JSON
lines are still the prototype process encoding, but the payload is no longer a
RESP blob and it does not contain socket identifiers.
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass, field
from io import TextIOBase
from typing import Literal

__version__ = "0.1.0"

type RedisString = bytes
type RedisHash = dict[bytes, bytes]
type RedisValue = RedisString | RedisHash
type EngineResponseKind = Literal[
    "simple_string",
    "error",
    "integer",
    "bulk_string",
    "array",
]

DEFAULT_DATABASES = 16
JOB_PROTOCOL_VERSION = 1


@dataclass(frozen=True)
class EngineResponse:
    """Language-neutral response shape returned by the command layer.

    This mirrors the Rust ``EngineResponse`` enum closely enough for generic
    job workers in Python, Ruby, Lua, or any other language to implement the
    same protocol without knowing anything about TCP sockets or RESP encoding.
    """

    kind: EngineResponseKind
    value: bytes | str | int | list[EngineResponse] | None = None

    def to_wire(self) -> dict[str, object]:
        """Serialize this engine response into the job-protocol payload."""

        if self.kind == "simple_string":
            return {"kind": "simple_string", "value": _as_text(self.value)}
        if self.kind == "error":
            return {"kind": "error", "message": _as_text(self.value)}
        if self.kind == "integer":
            return {"kind": "integer", "value": int(self.value or 0)}
        if self.kind == "bulk_string":
            value = None if self.value is None else _as_bytes(self.value).hex()
            return {"kind": "bulk_string", "value_hex": value}
        if self.kind == "array":
            value = self.value
            if value is None:
                return {"kind": "array", "values": None}
            if not isinstance(value, list):
                raise ValueError("array engine responses require a list value")
            return {
                "kind": "array",
                "values": [item.to_wire() for item in value],
            }
        raise ValueError(f"unknown engine response kind: {self.kind}")


@dataclass(frozen=True)
class JobExecution:
    """Result of one socket-blind command job."""

    selected_db: int
    response: EngineResponse

    def to_wire_payload(self) -> dict[str, object]:
        """Serialize the command result returned to Rust."""

        return {
            "selected_db": self.selected_db,
            "response": self.response.to_wire(),
        }


@dataclass
class MiniRedisWorker:
    """Stateful Mini Redis command engine.

    The worker stores Redis key/value data. It does not store session state by
    socket; Rust passes the currently selected database into each job and stores
    the returned database index on the connection state after ``SELECT``.
    """

    database_count: int = DEFAULT_DATABASES
    databases: list[dict[bytes, RedisValue]] = field(init=False)

    def __post_init__(self) -> None:
        self.databases = [dict() for _ in range(self.database_count)]

    def execute(
        self,
        selected_db: int,
        command: str,
        args: list[bytes],
    ) -> JobExecution:
        """Execute one Redis-style command and return an engine response."""

        if not 0 <= selected_db < self.database_count:
            return JobExecution(selected_db, _error("ERR invalid DB index"))
        if not command:
            return JobExecution(selected_db, _error("ERR empty command"))

        normalized_command = command.upper()
        db = self.databases[selected_db]

        try:
            next_selected_db, response = self._execute_command(
                selected_db,
                db,
                normalized_command,
                args,
            )
        except Exception as exc:  # noqa: BLE001 - convert worker bugs to payload errors.
            next_selected_db = selected_db
            response = _error(f"ERR worker error: {exc}")
        return JobExecution(next_selected_db, response)

    def handle_wire_request(self, line: str) -> str:
        """Handle one generic job-protocol request and return one response."""

        job_id, metadata, payload = _decode_job_request(line)
        selected_db, command, args = _decode_command_payload(payload)
        execution = self.execute(selected_db, command, args)
        return _encode_job_response(job_id, metadata, execution.to_wire_payload())

    def _execute_command(
        self,
        selected_db: int,
        db: dict[bytes, RedisValue],
        command: str,
        args: list[bytes],
    ) -> tuple[int, EngineResponse]:
        if command == "PING":
            return selected_db, self._ping(args)
        if command == "SET":
            return selected_db, self._set(db, args)
        if command == "GET":
            return selected_db, self._get(db, args)
        if command == "EXISTS":
            return selected_db, self._exists(db, args)
        if command == "DEL":
            return selected_db, self._delete(db, args)
        if command == "INCRBY":
            return selected_db, self._incrby(db, args)
        if command == "HSET":
            return selected_db, self._hset(db, args)
        if command == "HGET":
            return selected_db, self._hget(db, args)
        if command == "HEXISTS":
            return selected_db, self._hexists(db, args)
        if command == "SELECT":
            return self._select(selected_db, args)
        return selected_db, _error(f"ERR unknown command '{command}'")

    def _ping(self, args: list[bytes]) -> EngineResponse:
        if not args:
            return EngineResponse("simple_string", "PONG")
        if len(args) == 1:
            return EngineResponse("bulk_string", args[0])
        return _wrong_arity("PING")

    def _set(self, db: dict[bytes, RedisValue], args: list[bytes]) -> EngineResponse:
        if len(args) != 2:
            return _wrong_arity("SET")
        key, value = args
        db[key] = value
        return EngineResponse("simple_string", "OK")

    def _get(self, db: dict[bytes, RedisValue], args: list[bytes]) -> EngineResponse:
        if len(args) != 1:
            return _wrong_arity("GET")
        value = db.get(args[0])
        if value is None:
            return EngineResponse("bulk_string", None)
        if not isinstance(value, bytes):
            return _wrong_type()
        return EngineResponse("bulk_string", value)

    def _exists(self, db: dict[bytes, RedisValue], args: list[bytes]) -> EngineResponse:
        if not args:
            return _wrong_arity("EXISTS")
        return EngineResponse("integer", sum(1 for key in args if key in db))

    def _delete(self, db: dict[bytes, RedisValue], args: list[bytes]) -> EngineResponse:
        if not args:
            return _wrong_arity("DEL")
        removed = 0
        for key in args:
            if key in db:
                removed += 1
                del db[key]
        return EngineResponse("integer", removed)

    def _incrby(self, db: dict[bytes, RedisValue], args: list[bytes]) -> EngineResponse:
        if len(args) != 2:
            return _wrong_arity("INCRBY")
        key, delta_raw = args
        try:
            delta = int(delta_raw.decode("ascii"))
        except ValueError:
            return _error("ERR value is not an integer or out of range")

        current = db.get(key, b"0")
        if not isinstance(current, bytes):
            return _wrong_type()
        try:
            next_value = int(current.decode("ascii")) + delta
        except ValueError:
            return _error("ERR value is not an integer or out of range")
        db[key] = str(next_value).encode("ascii")
        return EngineResponse("integer", next_value)

    def _hset(self, db: dict[bytes, RedisValue], args: list[bytes]) -> EngineResponse:
        if len(args) < 3 or len(args) % 2 == 0:
            return _wrong_arity("HSET")
        key = args[0]
        value = db.get(key)
        if value is None:
            mapping: RedisHash = {}
        elif isinstance(value, dict):
            mapping = value
        else:
            return _wrong_type()

        added = 0
        for field_name, field_value in zip(args[1::2], args[2::2], strict=True):
            if field_name not in mapping:
                added += 1
            mapping[field_name] = field_value
        db[key] = mapping
        return EngineResponse("integer", added)

    def _hget(self, db: dict[bytes, RedisValue], args: list[bytes]) -> EngineResponse:
        if len(args) != 2:
            return _wrong_arity("HGET")
        value = db.get(args[0])
        if value is None:
            return EngineResponse("bulk_string", None)
        if not isinstance(value, dict):
            return _wrong_type()
        return EngineResponse("bulk_string", value.get(args[1]))

    def _hexists(
        self,
        db: dict[bytes, RedisValue],
        args: list[bytes],
    ) -> EngineResponse:
        if len(args) != 2:
            return _wrong_arity("HEXISTS")
        value = db.get(args[0])
        if value is None:
            return EngineResponse("integer", 0)
        if not isinstance(value, dict):
            return _wrong_type()
        return EngineResponse("integer", int(args[1] in value))

    def _select(
        self,
        selected_db: int,
        args: list[bytes],
    ) -> tuple[int, EngineResponse]:
        if len(args) != 1:
            return selected_db, _wrong_arity("SELECT")
        try:
            index = int(args[0].decode("ascii"))
        except ValueError:
            return selected_db, _error("ERR invalid DB index")
        if index < 0 or index >= self.database_count:
            return selected_db, _error("ERR invalid DB index")
        return index, EngineResponse("simple_string", "OK")


def run_stdio_worker(
    stdin: TextIOBase | None = None,
    stdout: TextIOBase | None = None,
) -> None:
    """Run the generic job-protocol JSON-line worker loop."""

    input_stream = stdin or sys.stdin
    output_stream = stdout or sys.stdout
    worker = MiniRedisWorker()

    for line in input_stream:
        line = line.strip()
        if not line:
            continue
        try:
            response = worker.handle_wire_request(line)
        except Exception as exc:  # noqa: BLE001 - worker protocol must survive.
            response = _encode_job_error_response(
                "unknown",
                {},
                "worker_protocol_error",
                f"worker protocol error: {exc}",
            )
        output_stream.write(response + "\n")
        output_stream.flush()


def _decode_job_request(line: str) -> tuple[str, dict[str, object], dict[str, object]]:
    frame = json.loads(line)
    if frame.get("version") != JOB_PROTOCOL_VERSION:
        raise ValueError(f"unsupported job protocol version: {frame.get('version')}")
    if frame.get("kind") != "request":
        raise ValueError(f"expected request frame, got {frame.get('kind')!r}")

    body = frame["body"]
    metadata = body.get("metadata", {})
    payload = body["payload"]
    if not isinstance(metadata, dict):
        raise ValueError("job metadata must be an object")
    if not isinstance(payload, dict):
        raise ValueError("job payload must be an object")
    return str(body["id"]), metadata, payload


def _decode_command_payload(payload: dict[str, object]) -> tuple[int, str, list[bytes]]:
    selected_db = payload.get("selected_db", 0)
    command = payload["command"]
    args_hex = payload.get("args_hex", [])
    if not isinstance(selected_db, int):
        raise ValueError("selected_db must be an integer")
    if not isinstance(command, str):
        raise ValueError("command must be a string")
    if not isinstance(args_hex, list) or not all(
        isinstance(item, str) for item in args_hex
    ):
        raise ValueError("args_hex must be a list of strings")
    return selected_db, command, [bytes.fromhex(part) for part in args_hex]


def _encode_job_response(
    job_id: str,
    metadata: dict[str, object],
    payload: dict[str, object],
) -> str:
    return json.dumps(
        {
            "version": JOB_PROTOCOL_VERSION,
            "kind": "response",
            "body": {
                "id": job_id,
                "result": {
                    "status": "ok",
                    "payload": payload,
                },
                "metadata": metadata,
            },
        },
        separators=(",", ":"),
    )


def _encode_job_error_response(
    job_id: str,
    metadata: dict[str, object],
    code: str,
    message: str,
) -> str:
    return json.dumps(
        {
            "version": JOB_PROTOCOL_VERSION,
            "kind": "response",
            "body": {
                "id": job_id,
                "result": {
                    "status": "error",
                    "error": {
                        "code": code,
                        "message": message,
                        "retryable": False,
                        "origin": "worker",
                        "detail": None,
                    },
                },
                "metadata": metadata,
            },
        },
        separators=(",", ":"),
    )


def _as_bytes(value: bytes | str | int | list[EngineResponse] | None) -> bytes:
    if value is None:
        return b""
    if isinstance(value, bytes):
        return value
    if isinstance(value, int):
        return str(value).encode("ascii")
    if isinstance(value, list):
        raise TypeError("array values cannot be coerced to bytes")
    return value.encode("utf-8")


def _as_text(value: bytes | str | int | list[EngineResponse] | None) -> str:
    return _as_bytes(value).decode("utf-8", errors="replace")


def _error(message: str) -> EngineResponse:
    return EngineResponse("error", message)


def _wrong_arity(command: str) -> EngineResponse:
    return _error(f"ERR wrong number of arguments for '{command}'")


def _wrong_type() -> EngineResponse:
    return _error("WRONGTYPE Operation against a key holding the wrong kind of value")
