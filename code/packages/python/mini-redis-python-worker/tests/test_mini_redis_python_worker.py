"""Tests for mini-redis-python-worker."""

from __future__ import annotations

import json
from io import StringIO

import pytest

import mini_redis_python_worker.stdio_worker as stdio_worker
from mini_redis_python_worker import (
    MiniRedisWorker,
    RespReply,
    __version__,
    run_stdio_worker,
)


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


def test_worker_executes_string_commands() -> None:
    """The worker can execute the Redis string subset used by the prototype."""

    worker = MiniRedisWorker()
    assert worker.execute("1", [b"PING"]) == b"+PONG\r\n"
    assert worker.execute("1", [b"PING", b"hello"]) == b"$5\r\nhello\r\n"
    assert worker.execute("1", [b"SET", b"counter", b"2"]) == b"+OK\r\n"
    assert worker.execute("1", [b"GET", b"counter"]) == b"$1\r\n2\r\n"
    assert worker.execute("1", [b"INCRBY", b"counter", b"5"]) == b":7\r\n"
    assert worker.execute("1", [b"EXISTS", b"counter", b"missing"]) == b":1\r\n"
    assert worker.execute("1", [b"DEL", b"counter", b"missing"]) == b":1\r\n"
    assert worker.execute("1", [b"GET", b"counter"]) == b"$-1\r\n"


def test_worker_executes_hash_commands() -> None:
    """Hash commands maintain Python-owned application state."""

    worker = MiniRedisWorker()
    assert worker.execute("1", [b"HSET", b"user", b"name", b"ada"]) == b":1\r\n"
    assert worker.execute("1", [b"HSET", b"user", b"name", b"ada"]) == b":0\r\n"
    assert worker.execute("1", [b"HGET", b"user", b"name"]) == b"$3\r\nada\r\n"
    assert worker.execute("1", [b"HGET", b"user", b"missing"]) == b"$-1\r\n"
    assert worker.execute("1", [b"HEXISTS", b"user", b"name"]) == b":1\r\n"
    assert worker.execute("1", [b"HEXISTS", b"user", b"age"]) == b":0\r\n"


def test_worker_reports_command_errors_without_crashing() -> None:
    """Bad commands become RESP errors so the TCP side can keep serving."""

    worker = MiniRedisWorker()
    assert worker.execute("1", []) == b"-ERR empty command\r\n"
    assert worker.execute("1", [b"NOPE"]).startswith(b"-ERR unknown command")
    assert worker.execute("1", [b"PING", b"a", b"b"]).startswith(b"-ERR wrong number")
    assert worker.execute("1", [b"SET", b"only-key"]).startswith(b"-ERR wrong number")
    assert worker.execute("1", [b"GET"]).startswith(b"-ERR wrong number")
    assert worker.execute("1", [b"EXISTS"]).startswith(b"-ERR wrong number")
    assert worker.execute("1", [b"DEL"]).startswith(b"-ERR wrong number")
    assert worker.execute("1", [b"INCRBY", b"n"]).startswith(b"-ERR wrong number")
    assert worker.execute("1", [b"INCRBY", b"n", b"not-int"]).startswith(
        b"-ERR value is not an integer"
    )
    assert worker.execute("1", [b"SET", b"n", b"also-not-int"]) == b"+OK\r\n"
    assert worker.execute("1", [b"INCRBY", b"n", b"1"]).startswith(
        b"-ERR value is not an integer"
    )
    assert worker.execute("1", [b"HSET", b"h", b"field"]).startswith(
        b"-ERR wrong number"
    )
    assert worker.execute("1", [b"HGET", b"h"]).startswith(b"-ERR wrong number")
    assert worker.execute("1", [b"HEXISTS", b"h"]).startswith(b"-ERR wrong number")
    assert worker.execute("1", [b"SELECT"]).startswith(b"-ERR wrong number")
    assert worker.execute("1", [b"SELECT", b"banana"]) == b"-ERR invalid DB index\r\n"
    assert worker.execute("1", [b"SELECT", b"999"]) == b"-ERR invalid DB index\r\n"


def test_worker_preserves_wrong_type_errors() -> None:
    """String and hash commands reject keys holding the other Redis type."""

    worker = MiniRedisWorker()
    assert worker.execute("1", [b"SET", b"string", b"value"]) == b"+OK\r\n"
    assert worker.execute("1", [b"HSET", b"string", b"f", b"v"]).startswith(
        b"-WRONGTYPE"
    )

    assert worker.execute("1", [b"HSET", b"hash", b"f", b"1"]) == b":1\r\n"
    assert worker.execute("1", [b"GET", b"hash"]).startswith(b"-WRONGTYPE")
    assert worker.execute("1", [b"INCRBY", b"hash", b"1"]).startswith(b"-WRONGTYPE")
    assert worker.execute("1", [b"HGET", b"string", b"f"]).startswith(b"-WRONGTYPE")
    assert worker.execute("1", [b"HEXISTS", b"string", b"f"]).startswith(b"-WRONGTYPE")


def test_select_is_connection_local() -> None:
    """Two TCP connections can select different logical databases."""

    worker = MiniRedisWorker()
    assert worker.execute("1", [b"SET", b"k", b"db0"]) == b"+OK\r\n"
    assert worker.execute("2", [b"SELECT", b"1"]) == b"+OK\r\n"
    assert worker.execute("2", [b"GET", b"k"]) == b"$-1\r\n"
    assert worker.execute("2", [b"SET", b"k", b"db1"]) == b"+OK\r\n"
    assert worker.execute("1", [b"GET", b"k"]) == b"$3\r\ndb0\r\n"
    assert worker.execute("2", [b"GET", b"k"]) == b"$3\r\ndb1\r\n"


def test_wire_request_round_trips_hex_encoded_arguments() -> None:
    """JSON-line requests avoid raw binary at the process boundary."""

    worker = MiniRedisWorker()
    request = {
        "id": "job-1",
        "connection_id": "7",
        "sequence": 4,
        "argv_hex": [b"PING".hex()],
    }
    response = json.loads(worker.handle_wire_request(json.dumps(request)))
    assert response == {
        "id": "job-1",
        "connection_id": "7",
        "sequence": 4,
        "ok": True,
        "resp_hex": b"+PONG\r\n".hex(),
    }


def test_stdio_worker_processes_multiple_lines() -> None:
    """The worker CLI protocol preserves one response per request line."""

    stdin = StringIO(
        "\n".join(
            [
                json.dumps(
                    {
                        "id": "a",
                        "connection_id": "1",
                        "sequence": 1,
                        "argv_hex": [b"SET".hex(), b"k".hex(), b"v".hex()],
                    }
                ),
                json.dumps(
                    {
                        "id": "b",
                        "connection_id": "1",
                        "sequence": 2,
                        "argv_hex": [b"GET".hex(), b"k".hex()],
                    }
                ),
            ]
        )
        + "\n"
    )
    stdout = StringIO()
    run_stdio_worker(stdin, stdout)
    lines = [json.loads(line) for line in stdout.getvalue().splitlines()]
    assert lines[0]["resp_hex"] == b"+OK\r\n".hex()
    assert lines[1]["resp_hex"] == b"$1\r\nv\r\n".hex()


def test_stdio_worker_reports_protocol_errors_and_skips_blank_lines() -> None:
    """Malformed JSON produces an error response while blank lines are ignored."""

    stdout = StringIO()
    run_stdio_worker(StringIO("\nnot-json\n"), stdout)
    lines = [json.loads(line) for line in stdout.getvalue().splitlines()]
    assert len(lines) == 1
    assert lines[0]["ok"] is False
    assert lines[0]["id"] == "unknown"
    assert lines[0]["error"].startswith("worker protocol error:")


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
