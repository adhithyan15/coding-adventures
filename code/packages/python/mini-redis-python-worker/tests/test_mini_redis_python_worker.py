"""Tests for mini-redis-python-worker."""

from __future__ import annotations

import json
from io import StringIO

import pytest

import mini_redis_python_worker.stdio_worker as stdio_worker
from mini_redis_python_worker import (
    JOB_PROTOCOL_VERSION,
    EngineResponse,
    MiniRedisWorker,
    __version__,
    run_stdio_worker,
)


def job_request(
    job_id: str,
    selected_db: int,
    command: str,
    args: list[bytes],
    metadata: dict[str, object] | None = None,
) -> dict[str, object]:
    """Build the generic job-protocol request used by Rust language bridges."""

    return {
        "version": JOB_PROTOCOL_VERSION,
        "kind": "request",
        "body": {
            "id": job_id,
            "payload": {
                "selected_db": selected_db,
                "command": command,
                "args_hex": [part.hex() for part in args],
            },
            "metadata": metadata or {},
        },
    }


def payload(
    worker: MiniRedisWorker,
    selected_db: int,
    command: str,
    *args: bytes,
) -> dict:
    """Execute one command and return the wire payload Rust receives."""

    return worker.execute(selected_db, command, list(args)).to_wire_payload()


def test_version_exists() -> None:
    """Verify the package is importable."""

    assert __version__ == "0.1.0"


def test_engine_response_serializes_core_response_types() -> None:
    """Engine responses mirror the Rust-side response enum."""

    assert EngineResponse("simple_string", "OK").to_wire() == {
        "kind": "simple_string",
        "value": "OK",
    }
    assert EngineResponse("integer", 7).to_wire() == {"kind": "integer", "value": 7}
    assert EngineResponse("integer", None).to_wire() == {"kind": "integer", "value": 0}
    assert EngineResponse("bulk_string", b"abc").to_wire() == {
        "kind": "bulk_string",
        "value_hex": "616263",
    }
    assert EngineResponse("bulk_string", None).to_wire() == {
        "kind": "bulk_string",
        "value_hex": None,
    }
    assert EngineResponse("error", "ERR nope").to_wire() == {
        "kind": "error",
        "message": "ERR nope",
    }
    assert EngineResponse(
        "array",
        [EngineResponse("simple_string", "OK")],
    ).to_wire() == {
        "kind": "array",
        "values": [{"kind": "simple_string", "value": "OK"}],
    }


def test_engine_response_rejects_unknown_kinds() -> None:
    """Unknown reply kinds fail loudly before crossing the wire."""

    try:
        EngineResponse("mystery", "nope").to_wire()  # type: ignore[arg-type]
    except ValueError as exc:
        assert "unknown engine response kind" in str(exc)
    else:  # pragma: no cover - defensive guard for future refactors.
        raise AssertionError("unknown engine response kind should raise")


def test_worker_executes_string_commands() -> None:
    """The worker can execute the Redis string subset used by the prototype."""

    worker = MiniRedisWorker()
    assert payload(worker, 0, "PING") == {
        "selected_db": 0,
        "response": {"kind": "simple_string", "value": "PONG"},
    }
    assert payload(worker, 0, "PING", b"hello") == {
        "selected_db": 0,
        "response": {"kind": "bulk_string", "value_hex": "68656c6c6f"},
    }
    assert payload(worker, 0, "SET", b"counter", b"2")["response"] == {
        "kind": "simple_string",
        "value": "OK",
    }
    assert payload(worker, 0, "GET", b"counter")["response"] == {
        "kind": "bulk_string",
        "value_hex": "32",
    }
    assert payload(worker, 0, "INCRBY", b"counter", b"5")["response"] == {
        "kind": "integer",
        "value": 7,
    }
    assert payload(worker, 0, "EXISTS", b"counter", b"missing")["response"] == {
        "kind": "integer",
        "value": 1,
    }
    assert payload(worker, 0, "DEL", b"counter", b"missing")["response"] == {
        "kind": "integer",
        "value": 1,
    }
    assert payload(worker, 0, "GET", b"counter")["response"] == {
        "kind": "bulk_string",
        "value_hex": None,
    }


def test_worker_executes_hash_commands() -> None:
    """Hash commands maintain Python-owned application state."""

    worker = MiniRedisWorker()
    assert payload(worker, 0, "HSET", b"user", b"name", b"ada")["response"] == {
        "kind": "integer",
        "value": 1,
    }
    assert payload(worker, 0, "HSET", b"user", b"name", b"ada")["response"] == {
        "kind": "integer",
        "value": 0,
    }
    assert payload(worker, 0, "HGET", b"user", b"name")["response"] == {
        "kind": "bulk_string",
        "value_hex": "616461",
    }
    assert payload(worker, 0, "HGET", b"user", b"missing")["response"] == {
        "kind": "bulk_string",
        "value_hex": None,
    }
    assert payload(worker, 0, "HEXISTS", b"user", b"name")["response"] == {
        "kind": "integer",
        "value": 1,
    }
    assert payload(worker, 0, "HEXISTS", b"user", b"age")["response"] == {
        "kind": "integer",
        "value": 0,
    }


def test_worker_reports_command_errors_without_crashing() -> None:
    """Bad commands become engine errors so the TCP side can keep serving."""

    worker = MiniRedisWorker()
    assert payload(worker, 0, "", )["response"]["message"] == "ERR empty command"
    assert payload(worker, 20, "PING")["response"]["message"] == "ERR invalid DB index"
    assert payload(worker, 0, "NOPE")["response"]["message"].startswith(
        "ERR unknown command"
    )
    assert payload(worker, 0, "PING", b"a", b"b")["response"]["message"].startswith(
        "ERR wrong number"
    )
    assert payload(worker, 0, "SET", b"only-key")["response"]["message"].startswith(
        "ERR wrong number"
    )
    assert payload(worker, 0, "GET")["response"]["message"].startswith(
        "ERR wrong number"
    )
    assert payload(worker, 0, "EXISTS")["response"]["message"].startswith(
        "ERR wrong number"
    )
    assert payload(worker, 0, "DEL")["response"]["message"].startswith(
        "ERR wrong number"
    )
    assert payload(worker, 0, "INCRBY", b"n")["response"]["message"].startswith(
        "ERR wrong number"
    )
    assert payload(worker, 0, "INCRBY", b"n", b"not-int")["response"][
        "message"
    ].startswith("ERR value is not an integer")
    assert payload(worker, 0, "SET", b"n", b"also-not-int")["response"] == {
        "kind": "simple_string",
        "value": "OK",
    }
    assert payload(worker, 0, "INCRBY", b"n", b"1")["response"]["message"].startswith(
        "ERR value is not an integer"
    )
    assert payload(worker, 0, "HSET", b"h", b"field")["response"][
        "message"
    ].startswith("ERR wrong number")
    assert payload(worker, 0, "HGET", b"h")["response"]["message"].startswith(
        "ERR wrong number"
    )
    assert payload(worker, 0, "HEXISTS", b"h")["response"]["message"].startswith(
        "ERR wrong number"
    )
    assert payload(worker, 0, "SELECT")["response"]["message"].startswith(
        "ERR wrong number"
    )
    assert payload(worker, 0, "SELECT", b"banana")["response"] == {
        "kind": "error",
        "message": "ERR invalid DB index",
    }
    assert payload(worker, 0, "SELECT", b"999")["response"] == {
        "kind": "error",
        "message": "ERR invalid DB index",
    }


def test_worker_preserves_wrong_type_errors() -> None:
    """String and hash commands reject keys holding the other Redis type."""

    worker = MiniRedisWorker()
    assert payload(worker, 0, "SET", b"string", b"value")["response"] == {
        "kind": "simple_string",
        "value": "OK",
    }
    assert payload(worker, 0, "HSET", b"string", b"f", b"v")["response"][
        "message"
    ].startswith("WRONGTYPE")

    assert payload(worker, 0, "HSET", b"hash", b"f", b"1")["response"] == {
        "kind": "integer",
        "value": 1,
    }
    assert payload(worker, 0, "GET", b"hash")["response"]["message"].startswith(
        "WRONGTYPE"
    )
    assert payload(worker, 0, "INCRBY", b"hash", b"1")["response"][
        "message"
    ].startswith("WRONGTYPE")
    assert payload(worker, 0, "HGET", b"string", b"f")["response"][
        "message"
    ].startswith("WRONGTYPE")
    assert payload(worker, 0, "HEXISTS", b"string", b"f")["response"][
        "message"
    ].startswith("WRONGTYPE")


def test_select_state_is_passed_in_and_out_by_rust() -> None:
    """Rust can keep session state while Python remains socket-blind."""

    worker = MiniRedisWorker()
    assert payload(worker, 0, "SET", b"k", b"db0")["response"] == {
        "kind": "simple_string",
        "value": "OK",
    }

    selected = worker.execute(0, "SELECT", [b"1"]).to_wire_payload()["selected_db"]
    assert selected == 1
    assert payload(worker, selected, "GET", b"k")["response"] == {
        "kind": "bulk_string",
        "value_hex": None,
    }
    assert payload(worker, selected, "SET", b"k", b"db1")["response"] == {
        "kind": "simple_string",
        "value": "OK",
    }
    assert payload(worker, 0, "GET", b"k")["response"] == {
        "kind": "bulk_string",
        "value_hex": "646230",
    }
    assert payload(worker, selected, "GET", b"k")["response"] == {
        "kind": "bulk_string",
        "value_hex": "646231",
    }


def test_wire_request_round_trips_command_frame_payload() -> None:
    """Generic job-protocol requests avoid raw binary at the process boundary."""

    worker = MiniRedisWorker()
    metadata = {"trace_id": "trace-1", "tags": {"consumer": "test"}}
    request = job_request("job-1", 0, "PING", [], metadata)
    response = json.loads(worker.handle_wire_request(json.dumps(request)))
    assert response == {
        "version": JOB_PROTOCOL_VERSION,
        "kind": "response",
        "body": {
            "id": "job-1",
            "result": {
                "status": "ok",
                "payload": {
                    "selected_db": 0,
                    "response": {"kind": "simple_string", "value": "PONG"},
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
                json.dumps(job_request("a", 0, "SET", [b"k", b"v"])),
                json.dumps(job_request("b", 0, "GET", [b"k"])),
            ]
        )
        + "\n"
    )
    stdout = StringIO()
    run_stdio_worker(stdin, stdout)
    lines = [json.loads(line) for line in stdout.getvalue().splitlines()]
    assert lines[0]["body"]["result"]["payload"]["response"] == {
        "kind": "simple_string",
        "value": "OK",
    }
    assert lines[1]["body"]["result"]["payload"]["response"] == {
        "kind": "bulk_string",
        "value_hex": "76",
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
