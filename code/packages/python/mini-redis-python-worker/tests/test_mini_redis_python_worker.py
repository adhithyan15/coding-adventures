"""Tests for mini-redis-python-worker."""

from __future__ import annotations

import json
from io import StringIO

import pytest

import mini_redis_python_worker.stdio_worker as stdio_worker
from mini_redis_python_worker import (
    JOB_PROTOCOL_VERSION,
    MiniRedisWorker,
    RespReply,
    TcpOutputFrame,
    __version__,
    run_stdio_worker,
)


def job_request(
    job_id: str,
    stream_id: str,
    data: bytes,
    metadata: dict[str, object] | None = None,
) -> dict[str, object]:
    """Build the generic job-protocol request used by Rust language bridges."""

    return {
        "version": JOB_PROTOCOL_VERSION,
        "kind": "request",
        "body": {
            "id": job_id,
            "payload": {
                "stream_id": stream_id,
                "bytes_hex": data.hex(),
            },
            "metadata": metadata or {},
        },
    }


def command(parts: list[bytes]) -> bytes:
    """Encode a RESP array command for Mini Redis tests."""

    output = bytearray()
    output.extend(b"*" + str(len(parts)).encode("ascii") + b"\r\n")
    for part in parts:
        output.extend(b"$" + str(len(part)).encode("ascii") + b"\r\n")
        output.extend(part + b"\r\n")
    return bytes(output)


def receive(worker: MiniRedisWorker, stream_id: str, data: bytes) -> bytes:
    """Return concatenated bytes the worker asks Rust to write."""

    frame = worker.receive_tcp_bytes(stream_id, data)
    return b"".join(frame.writes)


def test_version_exists() -> None:
    """Verify the package is importable."""

    assert __version__ == "0.1.0"


def test_resp_reply_encodes_core_response_types() -> None:
    """RESP replies are byte-exact at the Python/Rust worker boundary."""

    assert RespReply("simple", "OK").encode() == b"+OK\r\n"
    assert RespReply("simple", None).encode() == b"+\r\n"
    assert RespReply("integer", 7).encode() == b":7\r\n"
    assert RespReply("integer", None).encode() == b":0\r\n"
    assert RespReply("bulk", b"abc").encode() == b"$3\r\nabc\r\n"
    assert RespReply("bulk", None).encode() == b"$-1\r\n"
    assert RespReply("error", "ERR nope").encode() == b"-ERR nope\r\n"


def test_resp_reply_rejects_unknown_kinds() -> None:
    """Unknown reply kinds fail loudly before crossing the wire."""

    try:
        RespReply("mystery", "nope").encode()  # type: ignore[arg-type]
    except ValueError as exc:
        assert "unknown RESP reply kind" in str(exc)
    else:  # pragma: no cover - defensive guard for future refactors.
        raise AssertionError("unknown RESP reply kind should raise")


def test_tcp_output_frame_serializes_raw_write_frames() -> None:
    """Worker responses tell Rust what opaque bytes to write."""

    frame = TcpOutputFrame([b"+OK\r\n", b":1\r\n"], close=True)
    assert frame.to_wire_payload() == {
        "writes_hex": ["2b4f4b0d0a", "3a310d0a"],
        "close": True,
    }


def test_worker_queues_tcp_jobs_and_pops_them_for_processing() -> None:
    """The Python side owns the job queue behind the stdio callback."""

    worker = MiniRedisWorker()
    worker.enqueue_tcp_job("stream-1", command([b"PING"]))
    assert len(worker.pending_jobs) == 1
    frame = worker.process_next_job()
    assert frame.writes == [b"+PONG\r\n"]
    assert len(worker.pending_jobs) == 0
    assert worker.process_next_job().writes == []


def test_worker_buffers_fragmented_resp_frames() -> None:
    """RESP framing is application-owned and can span TCP reads."""

    worker = MiniRedisWorker()
    data = command([b"PING"])
    assert receive(worker, "stream-1", data[:3]) == b""
    assert receive(worker, "stream-1", data[3:]) == b"+PONG\r\n"


def test_worker_processes_pipelined_resp_frames() -> None:
    """One TCP job may contain multiple complete application frames."""

    worker = MiniRedisWorker()
    data = command([b"PING"]) + command([b"PING", b"hello"])
    assert receive(worker, "stream-1", data) == b"+PONG\r\n$5\r\nhello\r\n"


def test_worker_executes_string_commands() -> None:
    """The worker can execute the Redis string subset used by the prototype."""

    worker = MiniRedisWorker()
    assert receive(worker, "1", command([b"PING"])) == b"+PONG\r\n"
    assert receive(worker, "1", command([b"PING", b"hello"])) == b"$5\r\nhello\r\n"
    assert receive(worker, "1", command([b"SET", b"counter", b"2"])) == b"+OK\r\n"
    assert receive(worker, "1", command([b"GET", b"counter"])) == b"$1\r\n2\r\n"
    assert receive(worker, "1", command([b"INCRBY", b"counter", b"5"])) == b":7\r\n"
    assert receive(worker, "1", command([b"EXISTS", b"counter", b"missing"])) == (
        b":1\r\n"
    )
    assert receive(worker, "1", command([b"DEL", b"counter", b"missing"])) == b":1\r\n"
    assert receive(worker, "1", command([b"GET", b"counter"])) == b"$-1\r\n"


def test_worker_executes_hash_commands() -> None:
    """Hash commands maintain Python-owned application state."""

    worker = MiniRedisWorker()
    assert receive(worker, "1", command([b"HSET", b"user", b"name", b"ada"])) == (
        b":1\r\n"
    )
    assert receive(worker, "1", command([b"HSET", b"user", b"name", b"ada"])) == (
        b":0\r\n"
    )
    assert receive(worker, "1", command([b"HGET", b"user", b"name"])) == (
        b"$3\r\nada\r\n"
    )
    assert receive(worker, "1", command([b"HGET", b"user", b"missing"])) == b"$-1\r\n"
    assert receive(worker, "1", command([b"HEXISTS", b"user", b"name"])) == b":1\r\n"
    assert receive(worker, "1", command([b"HEXISTS", b"user", b"age"])) == b":0\r\n"


def test_worker_reports_command_errors_without_crashing() -> None:
    """Bad commands become RESP errors so the TCP side can keep serving."""

    worker = MiniRedisWorker()
    assert receive(worker, "1", command([])) == b"-ERR empty command\r\n"
    assert receive(worker, "1", command([b"NOPE"])).startswith(b"-ERR unknown command")
    assert receive(worker, "1", command([b"PING", b"a", b"b"])).startswith(
        b"-ERR wrong number"
    )
    assert receive(worker, "1", command([b"SET", b"only-key"])).startswith(
        b"-ERR wrong number"
    )
    assert receive(worker, "1", command([b"GET"])).startswith(b"-ERR wrong number")
    assert receive(worker, "1", command([b"EXISTS"])).startswith(b"-ERR wrong number")
    assert receive(worker, "1", command([b"DEL"])).startswith(b"-ERR wrong number")
    assert receive(worker, "1", command([b"INCRBY", b"n"])).startswith(
        b"-ERR wrong number"
    )
    assert receive(worker, "1", command([b"INCRBY", b"n", b"not-int"])).startswith(
        b"-ERR value is not an integer"
    )
    assert receive(worker, "1", command([b"SET", b"n", b"also-not-int"])) == b"+OK\r\n"
    assert receive(worker, "1", command([b"INCRBY", b"n", b"1"])).startswith(
        b"-ERR value is not an integer"
    )
    assert receive(worker, "1", command([b"HSET", b"h", b"field"])).startswith(
        b"-ERR wrong number"
    )
    assert receive(worker, "1", command([b"HGET", b"h"])).startswith(
        b"-ERR wrong number"
    )
    assert receive(worker, "1", command([b"HEXISTS", b"h"])).startswith(
        b"-ERR wrong number"
    )
    assert receive(worker, "1", command([b"SELECT"])).startswith(b"-ERR wrong number")
    assert receive(worker, "1", command([b"SELECT", b"banana"])) == (
        b"-ERR invalid DB index\r\n"
    )
    assert receive(worker, "1", command([b"SELECT", b"999"])) == (
        b"-ERR invalid DB index\r\n"
    )


def test_worker_preserves_wrong_type_errors() -> None:
    """String and hash commands reject keys holding the other Redis type."""

    worker = MiniRedisWorker()
    assert receive(worker, "1", command([b"SET", b"string", b"value"])) == b"+OK\r\n"
    assert receive(worker, "1", command([b"HSET", b"string", b"f", b"v"])).startswith(
        b"-WRONGTYPE"
    )

    assert receive(worker, "1", command([b"HSET", b"hash", b"f", b"1"])) == b":1\r\n"
    assert receive(worker, "1", command([b"GET", b"hash"])).startswith(b"-WRONGTYPE")
    assert receive(worker, "1", command([b"INCRBY", b"hash", b"1"])).startswith(
        b"-WRONGTYPE"
    )
    assert receive(worker, "1", command([b"HGET", b"string", b"f"])).startswith(
        b"-WRONGTYPE"
    )
    assert receive(worker, "1", command([b"HEXISTS", b"string", b"f"])).startswith(
        b"-WRONGTYPE"
    )


def test_select_is_stream_local() -> None:
    """Two TCP streams can select different logical databases."""

    worker = MiniRedisWorker()
    assert receive(worker, "1", command([b"SET", b"k", b"db0"])) == b"+OK\r\n"
    assert receive(worker, "2", command([b"SELECT", b"1"])) == b"+OK\r\n"
    assert receive(worker, "2", command([b"GET", b"k"])) == b"$-1\r\n"
    assert receive(worker, "2", command([b"SET", b"k", b"db1"])) == b"+OK\r\n"
    assert receive(worker, "1", command([b"GET", b"k"])) == b"$3\r\ndb0\r\n"
    assert receive(worker, "2", command([b"GET", b"k"])) == b"$3\r\ndb1\r\n"


def test_malformed_resp_returns_resp_error_from_python_layer() -> None:
    """RESP protocol errors are assembled by Python, not by Rust."""

    worker = MiniRedisWorker()
    assert receive(worker, "1", b"not-resp").startswith(
        b"-ERR protocol error: expected array command frame"
    )


def test_oversized_stream_buffer_returns_error_and_close_frame() -> None:
    """The application layer can ask Rust to close a stream."""

    worker = MiniRedisWorker()
    frame = worker.receive_tcp_bytes("1", b"x" * (1024 * 1024 + 1))
    assert frame.close is True
    assert frame.writes[0].startswith(b"-ERR protocol error")


def test_wire_request_round_trips_opaque_tcp_payload() -> None:
    """Generic job-protocol requests carry raw TCP bytes, not RESP objects."""

    worker = MiniRedisWorker()
    metadata = {"trace_id": "trace-1", "tags": {"consumer": "test"}}
    request = job_request("job-1", "stream-7", command([b"PING"]), metadata)
    response = json.loads(worker.handle_wire_request(json.dumps(request)))
    assert response == {
        "version": JOB_PROTOCOL_VERSION,
        "kind": "response",
        "body": {
            "id": "job-1",
            "result": {
                "status": "ok",
                "payload": {
                    "writes_hex": [b"+PONG\r\n".hex()],
                    "close": False,
                },
            },
            "metadata": metadata,
        },
    }


def test_stdio_worker_processes_multiple_lines() -> None:
    """The worker CLI protocol preserves one response per request line."""

    stdin = StringIO(
        "\n".join(
            [
                json.dumps(job_request("a", "stream-1", command([b"SET", b"k", b"v"]))),
                json.dumps(job_request("b", "stream-1", command([b"GET", b"k"]))),
            ]
        )
        + "\n"
    )
    stdout = StringIO()
    run_stdio_worker(stdin, stdout)
    lines = [json.loads(line) for line in stdout.getvalue().splitlines()]
    assert lines[0]["body"]["result"]["payload"] == {
        "writes_hex": [b"+OK\r\n".hex()],
        "close": False,
    }
    assert lines[1]["body"]["result"]["payload"] == {
        "writes_hex": [b"$1\r\nv\r\n".hex()],
        "close": False,
    }


def test_stdio_worker_reports_protocol_errors_and_skips_blank_lines() -> None:
    """Malformed JSON produces an error response while blank lines are ignored."""

    stdout = StringIO()
    run_stdio_worker(StringIO("\nnot-json\n"), stdout)
    lines = [json.loads(line) for line in stdout.getvalue().splitlines()]
    assert len(lines) == 1
    assert lines[0]["version"] == JOB_PROTOCOL_VERSION
    assert lines[0]["kind"] == "response"
    assert lines[0]["body"]["id"] == "unknown"
    assert lines[0]["body"]["result"]["status"] == "error"
    assert lines[0]["body"]["result"]["error"]["message"].startswith(
        "worker protocol error:"
    )


def test_stdio_worker_main_delegates_to_worker_loop(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    """The installed CLI entry point starts the stdio worker loop."""

    called = False

    def fake_run_stdio_worker() -> None:
        nonlocal called
        called = True

    monkeypatch.setattr(stdio_worker, "run_stdio_worker", fake_run_stdio_worker)
    stdio_worker.main()
    assert called
