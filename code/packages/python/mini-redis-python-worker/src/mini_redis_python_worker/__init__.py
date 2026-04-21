"""Python Mini Redis worker for the Rust TCP runtime prototype.

The Rust side owns the TCP listener, native event loop, socket buffers, RESP
framing, and response writes. This package owns the application command logic.

The bridge between the two is intentionally plain: one JSON object per line.
Requests contain command arguments as hex-encoded bytes, and responses contain a
hex-encoded RESP frame. That format is not the final high-performance worker
protocol; it is the smallest debuggable seam that proves Python can be a Mini
Redis consumer behind the Rust TCP runtime.
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
type RespKind = Literal["simple", "error", "integer", "bulk"]

DEFAULT_DATABASES = 16


@dataclass(frozen=True)
class RespReply:
    """A tiny RESP2 reply builder used by the worker command layer."""

    kind: RespKind
    value: bytes | str | int | None = None

    def encode(self) -> bytes:
        """Serialize this reply to RESP2 bytes."""

        if self.kind == "simple":
            return b"+" + _as_bytes(self.value) + b"\r\n"
        if self.kind == "error":
            return b"-" + _as_bytes(self.value) + b"\r\n"
        if self.kind == "integer":
            return b":" + str(int(self.value or 0)).encode("ascii") + b"\r\n"
        if self.kind == "bulk":
            if self.value is None:
                return b"$-1\r\n"
            data = _as_bytes(self.value)
            return b"$" + str(len(data)).encode("ascii") + b"\r\n" + data + b"\r\n"
        raise ValueError(f"unknown RESP reply kind: {self.kind}")


@dataclass
class MiniRedisWorker:
    """Stateful Mini Redis command engine.

    The worker stores connection-local selected database state because `SELECT`
    is part of the Redis session, not part of the TCP connection itself. Rust
    still owns the socket; Python owns the command semantics.
    """

    database_count: int = DEFAULT_DATABASES
    databases: list[dict[bytes, RedisValue]] = field(init=False)
    selected_db_by_connection: dict[str, int] = field(default_factory=dict)

    def __post_init__(self) -> None:
        self.databases = [dict() for _ in range(self.database_count)]

    def execute(self, connection_id: str, argv: list[bytes]) -> bytes:
        """Execute one Redis-style command and return a RESP2 response."""

        if not argv:
            return _error("ERR empty command").encode()

        command = argv[0].decode("ascii", errors="replace").upper()
        args = argv[1:]
        db = self._database(connection_id)

        try:
            reply = self._execute_command(connection_id, db, command, args)
        except Exception as exc:  # noqa: BLE001 - convert worker bugs to RESP.
            reply = _error(f"ERR worker error: {exc}")
        return reply.encode()

    def handle_wire_request(self, line: str) -> str:
        """Handle one JSON-line worker request and return one JSON line."""

        request = json.loads(line)
        job_id = str(request["id"])
        connection_id = str(request["connection_id"])
        sequence = int(request.get("sequence", 0))
        argv = [bytes.fromhex(part) for part in request["argv_hex"]]
        response = self.execute(connection_id, argv)
        return json.dumps(
            {
                "id": job_id,
                "connection_id": connection_id,
                "sequence": sequence,
                "ok": True,
                "resp_hex": response.hex(),
            },
            separators=(",", ":"),
        )

    def _database(self, connection_id: str) -> dict[bytes, RedisValue]:
        selected = self.selected_db_by_connection.get(connection_id, 0)
        return self.databases[selected]

    def _execute_command(
        self,
        connection_id: str,
        db: dict[bytes, RedisValue],
        command: str,
        args: list[bytes],
    ) -> RespReply:
        if command == "PING":
            return self._ping(args)
        if command == "SET":
            return self._set(db, args)
        if command == "GET":
            return self._get(db, args)
        if command == "EXISTS":
            return self._exists(db, args)
        if command == "DEL":
            return self._delete(db, args)
        if command == "INCRBY":
            return self._incrby(db, args)
        if command == "HSET":
            return self._hset(db, args)
        if command == "HGET":
            return self._hget(db, args)
        if command == "HEXISTS":
            return self._hexists(db, args)
        if command == "SELECT":
            return self._select(connection_id, args)
        return _error(f"ERR unknown command '{command}'")

    def _ping(self, args: list[bytes]) -> RespReply:
        if not args:
            return RespReply("simple", b"PONG")
        if len(args) == 1:
            return RespReply("bulk", args[0])
        return _wrong_arity("PING")

    def _set(self, db: dict[bytes, RedisValue], args: list[bytes]) -> RespReply:
        if len(args) != 2:
            return _wrong_arity("SET")
        key, value = args
        db[key] = value
        return RespReply("simple", b"OK")

    def _get(self, db: dict[bytes, RedisValue], args: list[bytes]) -> RespReply:
        if len(args) != 1:
            return _wrong_arity("GET")
        value = db.get(args[0])
        if value is None:
            return RespReply("bulk", None)
        if not isinstance(value, bytes):
            return _wrong_type()
        return RespReply("bulk", value)

    def _exists(self, db: dict[bytes, RedisValue], args: list[bytes]) -> RespReply:
        if not args:
            return _wrong_arity("EXISTS")
        return RespReply("integer", sum(1 for key in args if key in db))

    def _delete(self, db: dict[bytes, RedisValue], args: list[bytes]) -> RespReply:
        if not args:
            return _wrong_arity("DEL")
        removed = 0
        for key in args:
            if key in db:
                removed += 1
                del db[key]
        return RespReply("integer", removed)

    def _incrby(self, db: dict[bytes, RedisValue], args: list[bytes]) -> RespReply:
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
        return RespReply("integer", next_value)

    def _hset(self, db: dict[bytes, RedisValue], args: list[bytes]) -> RespReply:
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
        return RespReply("integer", added)

    def _hget(self, db: dict[bytes, RedisValue], args: list[bytes]) -> RespReply:
        if len(args) != 2:
            return _wrong_arity("HGET")
        value = db.get(args[0])
        if value is None:
            return RespReply("bulk", None)
        if not isinstance(value, dict):
            return _wrong_type()
        return RespReply("bulk", value.get(args[1]))

    def _hexists(self, db: dict[bytes, RedisValue], args: list[bytes]) -> RespReply:
        if len(args) != 2:
            return _wrong_arity("HEXISTS")
        value = db.get(args[0])
        if value is None:
            return RespReply("integer", 0)
        if not isinstance(value, dict):
            return _wrong_type()
        return RespReply("integer", int(args[1] in value))

    def _select(self, connection_id: str, args: list[bytes]) -> RespReply:
        if len(args) != 1:
            return _wrong_arity("SELECT")
        try:
            index = int(args[0].decode("ascii"))
        except ValueError:
            return _error("ERR invalid DB index")
        if index < 0 or index >= self.database_count:
            return _error("ERR invalid DB index")
        self.selected_db_by_connection[connection_id] = index
        return RespReply("simple", b"OK")


def run_stdio_worker(
    stdin: TextIOBase | None = None,
    stdout: TextIOBase | None = None,
) -> None:
    """Run the JSON-line worker protocol on standard input and output."""

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
            response = json.dumps(
                {
                    "id": "unknown",
                    "connection_id": "unknown",
                    "sequence": 0,
                    "ok": False,
                    "error": f"worker protocol error: {exc}",
                },
                separators=(",", ":"),
            )
        output_stream.write(response + "\n")
        output_stream.flush()


def _as_bytes(value: bytes | str | int | None) -> bytes:
    if value is None:
        return b""
    if isinstance(value, bytes):
        return value
    if isinstance(value, int):
        return str(value).encode("ascii")
    return value.encode("utf-8")


def _error(message: str) -> RespReply:
    return RespReply("error", message)


def _wrong_arity(command: str) -> RespReply:
    return _error(f"ERR wrong number of arguments for '{command}'")


def _wrong_type() -> RespReply:
    return _error("WRONGTYPE Operation against a key holding the wrong kind of value")
