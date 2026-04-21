"""Python Mini Redis worker for the Rust TCP runtime prototype.

The Rust side owns the TCP listener, native event loop, socket lifecycle, and
socket writes. This package owns the Mini Redis application protocol:
per-stream buffering, RESP frame parsing, command execution, and RESP frame
assembly.

The stdio bridge still uses ``generic-job-protocol`` JSON lines for the
prototype process boundary. The payload is intentionally transport-shaped:
``stream_id`` plus raw TCP bytes in, raw TCP write frames out. That keeps the
Rust TCP layer pure enough for Redis, IRC, WebSocket, or any other protocol to
sit on top without changing the transport crate.
"""

from __future__ import annotations

import json
import sys
from collections import deque
from dataclasses import dataclass, field
from io import TextIOBase
from typing import Literal

__version__ = "0.1.0"

type RedisString = bytes
type RedisHash = dict[bytes, bytes]
type RedisValue = RedisString | RedisHash
type RespKind = Literal["simple", "error", "integer", "bulk"]

DEFAULT_DATABASES = 16
JOB_PROTOCOL_VERSION = 1
MAX_BUFFERED_STREAM_BYTES = 1024 * 1024


class RespProtocolError(ValueError):
    """Raised when a stream contains malformed RESP."""


@dataclass(frozen=True)
class RespReply:
    """A tiny RESP2 reply builder owned by the Python protocol layer."""

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


@dataclass(frozen=True)
class TcpInputJob:
    """Opaque TCP bytes delivered from Rust for one logical stream."""

    stream_id: str
    data: bytes


@dataclass(frozen=True)
class TcpOutputFrame:
    """Opaque TCP bytes that Rust should write back to the same stream."""

    writes: list[bytes] = field(default_factory=list)
    close: bool = False

    def to_wire_payload(self) -> dict[str, object]:
        """Serialize the frame returned to Rust."""

        return {
            "writes_hex": [chunk.hex() for chunk in self.writes],
            "close": self.close,
        }


@dataclass
class RedisStreamSession:
    """Application-protocol state for one TCP stream."""

    selected_db: int = 0
    buffer: bytearray = field(default_factory=bytearray)


@dataclass
class MiniRedisWorker:
    """Stateful Mini Redis command engine and RESP protocol adapter.

    The worker queues raw TCP byte jobs from Rust, pops them for processing, and
    returns raw write frames. It never opens sockets, but it does own the
    application protocol state keyed by an opaque stream id.
    """

    database_count: int = DEFAULT_DATABASES
    databases: list[dict[bytes, RedisValue]] = field(init=False)
    sessions: dict[str, RedisStreamSession] = field(default_factory=dict)
    pending_jobs: deque[TcpInputJob] = field(default_factory=deque)

    def __post_init__(self) -> None:
        self.databases = [dict() for _ in range(self.database_count)]

    def enqueue_tcp_job(self, stream_id: str, data: bytes) -> None:
        """Queue an inbound TCP byte job for later application processing."""

        self.pending_jobs.append(TcpInputJob(stream_id, data))

    def process_next_job(self) -> TcpOutputFrame:
        """Pop one queued TCP job and return bytes Rust should write."""

        if not self.pending_jobs:
            return TcpOutputFrame()
        job = self.pending_jobs.popleft()
        return self.receive_tcp_bytes(job.stream_id, job.data)

    def receive_tcp_bytes(self, stream_id: str, data: bytes) -> TcpOutputFrame:
        """Buffer bytes for one stream, parse complete RESP commands, and reply."""

        session = self.sessions.setdefault(stream_id, RedisStreamSession())
        if len(session.buffer) + len(data) > MAX_BUFFERED_STREAM_BYTES:
            session.buffer.clear()
            error = "ERR protocol error: request exceeds maximum buffered size"
            return TcpOutputFrame(
                [_error(error).encode()],
                close=True,
            )

        session.buffer.extend(data)
        writes: list[bytes] = []

        while session.buffer:
            try:
                parsed = _parse_resp_command(session.buffer)
            except RespProtocolError as exc:
                session.buffer.clear()
                writes.append(_error(f"ERR {exc}").encode())
                break

            if parsed is None:
                break

            argv, consumed = parsed
            del session.buffer[:consumed]
            writes.append(self._execute_argv(session, argv).encode())

        return TcpOutputFrame(writes)

    def handle_wire_request(self, line: str) -> str:
        """Handle one generic job-protocol request and return one response."""

        job_id, metadata, payload = _decode_job_request(line)
        stream_id, data = _decode_tcp_payload(payload)
        self.enqueue_tcp_job(stream_id, data)
        frame = self.process_next_job()
        return _encode_job_response(job_id, metadata, frame.to_wire_payload())

    def _execute_argv(
        self,
        session: RedisStreamSession,
        argv: list[bytes],
    ) -> RespReply:
        if not argv:
            return _error("ERR empty command")

        command = argv[0].decode("ascii", errors="replace").upper()
        args = argv[1:]
        db = self.databases[session.selected_db]

        try:
            next_selected_db, reply = self._execute_command(
                session.selected_db,
                db,
                command,
                args,
            )
            session.selected_db = next_selected_db
        except Exception as exc:  # noqa: BLE001 - convert worker bugs to RESP.
            reply = _error(f"ERR worker error: {exc}")
        return reply

    def _execute_command(
        self,
        selected_db: int,
        db: dict[bytes, RedisValue],
        command: str,
        args: list[bytes],
    ) -> tuple[int, RespReply]:
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

    def _select(self, selected_db: int, args: list[bytes]) -> tuple[int, RespReply]:
        if len(args) != 1:
            return selected_db, _wrong_arity("SELECT")
        try:
            index = int(args[0].decode("ascii"))
        except ValueError:
            return selected_db, _error("ERR invalid DB index")
        if index < 0 or index >= self.database_count:
            return selected_db, _error("ERR invalid DB index")
        return index, RespReply("simple", b"OK")


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


def _parse_resp_command(buffer: bytearray) -> tuple[list[bytes], int] | None:
    if not buffer:
        return None
    if buffer[0] != ord("*"):
        raise RespProtocolError("protocol error: expected array command frame")

    line = _read_line(buffer, 0)
    if line is None:
        return None
    header, position = line
    try:
        count = int(header[1:].decode("ascii"))
    except ValueError as exc:
        raise RespProtocolError("protocol error: invalid array length") from exc
    if count < 0:
        raise RespProtocolError("protocol error: null command arrays are not supported")

    parts: list[bytes] = []
    for _ in range(count):
        if position >= len(buffer):
            return None
        prefix = buffer[position]
        if prefix == ord("$"):
            parsed_bulk = _parse_bulk_string(buffer, position)
            if parsed_bulk is None:
                return None
            part, position = parsed_bulk
            parts.append(part)
        elif prefix == ord("+") or prefix == ord(":"):
            parsed_line = _read_line(buffer, position)
            if parsed_line is None:
                return None
            part, position = parsed_line
            parts.append(bytes(part[1:]))
        else:
            raise RespProtocolError("protocol error: expected bulk string command part")

    return parts, position


def _parse_bulk_string(buffer: bytearray, position: int) -> tuple[bytes, int] | None:
    parsed_line = _read_line(buffer, position)
    if parsed_line is None:
        return None
    line, position = parsed_line
    try:
        length = int(line[1:].decode("ascii"))
    except ValueError as exc:
        raise RespProtocolError("protocol error: invalid bulk string length") from exc
    if length < 0:
        message = "protocol error: null bulk command parts are not supported"
        raise RespProtocolError(message)

    end = position + length
    if len(buffer) < end + 2:
        return None
    if buffer[end : end + 2] != b"\r\n":
        raise RespProtocolError("protocol error: malformed bulk string terminator")
    return bytes(buffer[position:end]), end + 2


def _read_line(buffer: bytearray, position: int) -> tuple[bytes, int] | None:
    end = buffer.find(b"\r\n", position)
    if end == -1:
        return None
    return bytes(buffer[position:end]), end + 2


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


def _decode_tcp_payload(payload: dict[str, object]) -> tuple[str, bytes]:
    stream_id = payload["stream_id"]
    bytes_hex = payload["bytes_hex"]
    if not isinstance(stream_id, str):
        raise ValueError("stream_id must be a string")
    if not isinstance(bytes_hex, str):
        raise ValueError("bytes_hex must be a string")
    return stream_id, bytes.fromhex(bytes_hex)


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
