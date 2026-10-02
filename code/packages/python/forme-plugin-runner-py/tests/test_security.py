from __future__ import annotations

import asyncio
import io
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

import pytest

from forme_plugin_runner import define_stage
from forme_plugin_runner.peer import Peer, ProtocolError, RpcFault, _rpc_error
from forme_plugin_runner.runner import RunnerState, StreamInput
from forme_plugin_runner.wire import encode_frame


async def _request(_request_id: int, _method: str, _params: Any) -> Any:
    return None


async def _notification(_method: str, _params: Any) -> None:
    return None


def _peer(**limits: int) -> Peer:
    return Peer(
        io.BytesIO(),
        io.BytesIO(),
        max_frame_bytes=4096,
        max_header_bytes=512,
        on_request=_request,
        on_notification=_notification,
        **limits,
    )


@define_stage(
    name="@example/security-passthrough",
    version="1.0.0",
    api_version=1,
    consumes="ContentNode",
    produces="ContentNode",
    capabilities=[],
)
async def _passthrough_stage(value: Any, _config: Any, _context: Any) -> Any:
    return value


async def test_stream_failure_discards_buffered_values_and_wakes_reader() -> None:
    stream = StreamInput(2, 128)
    await stream.push({"stale": True})
    await stream.fail(RuntimeError("stop"))
    with pytest.raises(RuntimeError, match="stop"):
        await stream.__anext__()

    empty = StreamInput(2, 128)
    blocked = asyncio.create_task(empty.__anext__())
    await asyncio.sleep(0)
    await empty.fail(RuntimeError("stop"))
    with pytest.raises(RuntimeError, match="stop"):
        await blocked


async def test_retiring_capability_stream_wakes_and_discards_blocked_push() -> None:
    @define_stage(
        name="@example/retiring-stream",
        version="1.0.0",
        api_version=1,
        consumes="ContentNode",
        produces="ContentNode",
        capabilities=[],
    )
    async def stage(value: Any, _config: Any, _context: Any) -> Any:
        return value

    state = RunnerState(stage, "retiring-stream", None, 1, 128)
    stream = StreamInput(1, 128)
    state.inputs[701] = stream
    await state.notification("stream.value", {"streamId": 701, "value": {"first": True}})
    blocked = asyncio.create_task(
        state.notification("stream.value", {"streamId": 701, "value": {"late": True}})
    )
    await asyncio.sleep(0)
    assert not blocked.done()
    state.retiring_capability_input_ids.add(701)
    await stream.end()
    await blocked


async def test_capability_retirement_requires_ack_and_retains_failed_tombstone() -> None:
    class AckPeer:
        def __init__(self, failure: BaseException | None = None) -> None:
            self.failure = failure
            self.failed_with: BaseException | None = None
            self.requests: list[tuple[str, Any]] = []

        async def request(self, method: str, params: Any) -> Any:
            self.requests.append((method, params))
            if self.failure is not None:
                raise self.failure
            return None

        def fail(self, error: BaseException) -> None:
            self.failed_with = error

    acknowledged = RunnerState(_passthrough_stage, "security-passthrough", None, 1, 128)
    acknowledged_peer = AckPeer()
    acknowledged.peer = acknowledged_peer  # type: ignore[assignment]
    acknowledged.capability_input_ids.add(701)
    acknowledged.inputs[701] = StreamInput(1, 128)
    await acknowledged._close_capability_stream(11, 701, True)
    assert acknowledged_peer.requests == [
        ("stream.cancel", {"streamId": 11, "capabilityStreamId": 701})
    ]
    assert 701 not in acknowledged.retiring_capability_input_ids

    failed = RunnerState(_passthrough_stage, "security-passthrough", None, 1, 128)
    failed_peer = AckPeer(ProtocolError("no acknowledgement"))
    failed.peer = failed_peer  # type: ignore[assignment]
    failed.capability_input_ids.add(702)
    failed.inputs[702] = StreamInput(1, 128)
    with pytest.raises(ProtocolError, match="no acknowledgement"):
        await failed._close_capability_stream(12, 702, True)
    assert 702 in failed.retiring_capability_input_ids
    assert isinstance(failed_peer.failed_with, ProtocolError)

    for method in ("stream.value", "stream.end", "stream.error"):
        await failed.notification(method, {"streamId": 702, "value": "late"})
    with pytest.raises(RpcFault, match="PROTOCOL_VIOLATION"):
        await failed.notification("stream.unknown", {"streamId": 702})


async def test_peer_budgets_outgoing_work_and_rejects_boolean_error_codes() -> None:
    peer = _peer(max_outgoing_notifications=1)
    peer.notify_nowait("log", {"message": "one"})
    with pytest.raises(ProtocolError, match="too many"):
        peer.notify_nowait("log", {"message": "two"})
    await peer.drain_notifications()

    request = asyncio.create_task(peer.request("ctx.env.get", {}))
    await asyncio.sleep(0)
    await peer._receive({"jsonrpc": "2.0", "id": -1, "error": {"code": True, "message": "bad"}})
    with pytest.raises(ProtocolError, match="malformed"):
        await request


async def test_peer_rejects_excess_concurrent_host_requests() -> None:
    gate = asyncio.Event()

    async def blocked(_request_id: int, _method: str, _params: Any) -> Any:
        await gate.wait()
        return None

    peer = Peer(
        io.BytesIO(),
        io.BytesIO(),
        max_frame_bytes=4096,
        max_header_bytes=512,
        max_inflight_requests=1,
        on_request=blocked,
        on_notification=_notification,
    )
    await peer._receive({"jsonrpc": "2.0", "id": 1, "method": "one", "params": {}})
    with pytest.raises(ProtocolError, match="too many"):
        await peer._receive({"jsonrpc": "2.0", "id": 2, "method": "two", "params": {}})
    gate.set()
    await asyncio.gather(*tuple(peer._answers))


async def test_notification_drain_tracks_every_task_before_raising() -> None:
    peer = _peer()
    gate = asyncio.Event()

    async def fail() -> None:
        raise ProtocolError("first")

    async def finish() -> None:
        await gate.wait()

    peer._outgoing.update({asyncio.create_task(fail()), asyncio.create_task(finish())})
    draining = asyncio.create_task(peer.drain_notifications())
    await asyncio.sleep(0)
    assert not draining.done()
    gate.set()
    with pytest.raises(ProtocolError, match="first"):
        await draining
    assert not peer._outgoing


async def test_strict_params_and_lifecycle_hooks() -> None:
    events: list[tuple[str, Any]] = []

    async def initialize(config: Any, context: Any) -> None:
        events.append(("init", config))
        assert context.config == config

    async def dispose(context: Any) -> None:
        events.append(("dispose", context.config))

    @define_stage(
        name="@example/lifecycle",
        version="1.0.0",
        api_version=1,
        consumes="ContentNode",
        produces="ContentNode",
        capabilities=[],
        init=initialize,
        dispose=dispose,
    )
    async def stage(value: Any, _config: Any, _context: Any) -> Any:
        return value

    state = RunnerState(stage, "lifecycle", None, 2, 128)
    state.peer = _peer()
    with pytest.raises(ProtocolError, match="must be an object"):
        await state.request(1, "handshake", [])
    await state.request(
        2,
        "handshake",
        {
            "pluginName": "@example/lifecycle",
            "pluginVersion": "1.0.0",
            "apiVersion": 1,
            "protocolVersion": 1,
        },
    )
    await state.request(3, "announce", {})
    await state.request(4, "stage.init", {"config": {"safe": True}})
    await state._dispose()
    assert events == [("init", {"safe": True}), ("dispose", {"safe": True})]


async def test_cancellation_closes_blocked_output_iterator() -> None:
    closed = asyncio.Event()
    gate = asyncio.Event()

    @define_stage(
        name="@example/stream",
        version="1.0.0",
        api_version=1,
        consumes="ContentNode",
        produces="Stream<ContentNode>",
        capabilities=[],
    )
    async def stage(_value: Any, _config: Any, _context: Any) -> Any:
        async def output() -> Any:
            try:
                await gate.wait()
                yield {"never": True}
            finally:
                closed.set()

        return output()

    state = RunnerState(stage, "stream", None, 2, 128)
    state.peer = _peer()
    state.phase = "initialized"
    running = asyncio.create_task(
        state.request(7, "stage.run", {"input": {}, "config": {}, "streamId": 9})
    )
    await asyncio.sleep(0)
    state.cancel("stop")
    with pytest.raises(RpcFault) as failure:
        await running
    assert failure.value.code == -32800
    assert closed.is_set()


def test_unexpected_errors_are_redacted() -> None:
    assert _rpc_error(RuntimeError("secret path /private/key")) == {
        "code": -32603,
        "message": "INTERNAL_ERROR",
    }


@pytest.mark.parametrize("active", [False, True])
def test_runner_exits_after_termination_signal(active: bool) -> None:
    fixture = Path(__file__).with_name("fixture.py")
    process = subprocess.Popen(
        [sys.executable, str(fixture), "conformance", "-", "single"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    try:
        if active:
            messages = [
                {
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "handshake",
                    "params": {
                        "pluginName": "@forme/conformance",
                        "pluginVersion": "1.0.0",
                        "apiVersion": 1,
                        "protocolVersion": 1,
                    },
                },
                {"jsonrpc": "2.0", "id": 2, "method": "announce", "params": {}},
                {
                    "jsonrpc": "2.0",
                    "id": 3,
                    "method": "stage.init",
                    "params": {"config": {}},
                },
                {
                    "jsonrpc": "2.0",
                    "id": 4,
                    "method": "stage.run",
                    "params": {
                        "input": {"operation": "waitForCancel"},
                        "config": {},
                        "streamId": 21,
                    },
                },
            ]
            assert process.stdin is not None
            process.stdin.write(b"".join(encode_frame(message, 4096) for message in messages))
            process.stdin.flush()
        time.sleep(0.1)
        process.terminate()
        process.wait(timeout=2)
        if os.name != "nt":
            assert process.returncode == 0
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=2)
