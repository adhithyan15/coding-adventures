"""FM02 lifecycle state machine for Python-authored stages."""

from __future__ import annotations

import asyncio
import contextlib
import os
import signal
import sys
from collections.abc import AsyncIterator
from typing import Any, BinaryIO

from .context import StageContext
from .peer import Peer, RpcFault
from .stage import CancellationError, CancellationToken, Stage, StageError
from .wire import ProtocolError, wire_value_size

PROTOCOL_VERSION = 1
RUNNER_NAME = "forme-plugin-runner-py"
RUNNER_VERSION = "0.1.0"
MAX_SAFE_INTEGER = 9_007_199_254_740_991


class StreamInput:
    def __init__(self, max_values: int, max_bytes: int) -> None:
        self._max_values = max_values
        self._max_bytes = max_bytes
        self._values: list[tuple[Any, int]] = []
        self._retained_bytes = 0
        self._done = False
        self._error: BaseException | None = None
        self._condition = asyncio.Condition()

    async def push(self, value: Any) -> None:
        try:
            size = wire_value_size(value, self._max_bytes)
        except (TypeError, ValueError) as error:
            raise ProtocolError("stream value is not JSON serializable") from error
        async with self._condition:
            while not self._done and (
                len(self._values) >= self._max_values
                or self._retained_bytes + size > self._max_bytes
            ):
                await self._condition.wait()
            if self._done:
                raise ProtocolError("stream.value targets a completed stream")
            self._values.append((value, size))
            self._retained_bytes += size
            self._condition.notify_all()

    async def end(self) -> None:
        async with self._condition:
            self._done = True
            self._condition.notify_all()

    async def fail(self, error: BaseException) -> None:
        async with self._condition:
            self._error = error
            self._done = True
            self._values.clear()
            self._retained_bytes = 0
            self._condition.notify_all()

    def __aiter__(self) -> AsyncIterator[Any]:
        return self

    async def __anext__(self) -> Any:
        async with self._condition:
            while not self._values and not self._done:
                await self._condition.wait()
            if self._values:
                value, size = self._values.pop(0)
                self._retained_bytes -= size
                self._condition.notify_all()
                return value
            if self._error is not None:
                raise self._error
            raise StopAsyncIteration


class RunnerState:
    def __init__(
        self,
        stage: Stage,
        stage_id: str,
        config_schema_hash: str | None,
        max_buffered_stream_values: int,
        max_buffered_stream_bytes: int,
    ) -> None:
        self.stage = stage
        self.stage_id = stage_id
        self.config_schema_hash = config_schema_hash
        self.max_buffered_stream_values = max_buffered_stream_values
        self.max_buffered_stream_bytes = max_buffered_stream_bytes
        self.phase = "spawned"
        self.active_id: int | None = None
        self.cancellation: CancellationToken | None = None
        self.active_task: asyncio.Task[Any] | None = None
        self.active_completion = asyncio.Event()
        self.active_completion.set()
        self.lifecycle_task: asyncio.Task[Any] | None = None
        self.lifecycle_completion = asyncio.Event()
        self.lifecycle_completion.set()
        self.last_config: Any = {}
        self._shutdown_task: asyncio.Task[None] | None = None
        self.inputs: dict[int, StreamInput] = {}
        self.capability_input_ids: set[int] = set()
        self.retiring_capability_input_ids: set[int] = set()
        self.peer: Peer | None = None

    async def request(self, request_id: int, method: str, raw_params: Any) -> Any:
        params = _record({} if raw_params is None else raw_params, f"{method} params")
        if method == "handshake":
            return self._handshake(params)
        if method == "announce":
            return self._announce()
        if method == "stage.init":
            return await self._init(params)
        if method == "stage.run":
            return await self._run(request_id, params)
        if method == "stage.dispose":
            await self._dispose()
            assert self.peer is not None
            self.peer.request_stop_after_response()
            return None
        raise RpcFault(-32601, "METHOD_NOT_FOUND", {"method": method})

    async def notification(self, method: str, raw_params: Any) -> None:
        params = _record({} if raw_params is None else raw_params, f"{method} params")
        if method == "$/cancelRequest":
            if self.active_id is not None and params.get("id") == self.active_id:
                reason = params.get("reason")
                assert self.cancellation is not None
                self.cancel(reason if isinstance(reason, str) else "operation cancelled")
            return
        if method not in {"stream.value", "stream.end", "stream.error"}:
            raise RpcFault(-32004, "PROTOCOL_VIOLATION", {"method": method})
        stream_id = _safe_id(params.get("streamId"), "streamId")
        if stream_id in self.retiring_capability_input_ids:
            return
        stream = self.inputs.get(stream_id)
        if stream is None:
            raise RpcFault(-32004, "PROTOCOL_VIOLATION", {"streamId": stream_id})
        if method == "stream.value":
            try:
                await stream.push(params.get("value"))
            except BaseException:
                if stream_id in self.retiring_capability_input_ids:
                    return
                raise
            return
        if method == "stream.end":
            await stream.end()
            return
        if method == "stream.error":
            await stream.fail(StageError("UPSTREAM_STREAM_ERROR", "host input stream failed"))
            return
        raise AssertionError("validated stream notification was not handled")

    def cancel(self, reason: str) -> None:
        if self.cancellation is not None:
            self.cancellation.cancel(reason)
        error = CancellationError(reason)
        for stream_id in self.capability_input_ids:
            stream = self.inputs.get(stream_id)
            if stream is not None:
                asyncio.create_task(stream.fail(error))
        task = self.active_task
        if task is not None and task is not asyncio.current_task() and not task.done():
            task.cancel(reason)

    async def shutdown(self) -> None:
        if self._shutdown_task is None:
            self._shutdown_task = asyncio.create_task(self._shutdown())
        await asyncio.shield(self._shutdown_task)

    async def _shutdown(self) -> None:
        self.cancel("plugin process received termination signal")
        lifecycle = self.lifecycle_task
        if (
            lifecycle is not None
            and lifecycle is not asyncio.current_task()
            and not lifecycle.done()
        ):
            lifecycle.cancel("plugin process received termination signal")
        await self.lifecycle_completion.wait()
        await self.active_completion.wait()
        if self.phase == "initialized":
            await self._dispose()
        assert self.peer is not None
        self.peer.shutdown()

    def _handshake(self, params: dict[str, Any]) -> dict[str, Any]:
        if self.phase != "spawned":
            raise RpcFault(-32004, "PROTOCOL_VIOLATION", {"phase": self.phase})
        expected = {
            "pluginName": self.stage.name,
            "pluginVersion": self.stage.version,
            "apiVersion": self.stage.api_version,
            "protocolVersion": PROTOCOL_VERSION,
        }
        for key, value in expected.items():
            if params.get(key) != value:
                raise RpcFault(-32006, "MANIFEST_MISMATCH", {"field": key})
        self.phase = "handshaken"
        return {**expected, "runner": RUNNER_NAME, "runnerVersion": RUNNER_VERSION}

    def _announce(self) -> dict[str, Any]:
        if self.phase != "handshaken":
            raise RpcFault(-32004, "PROTOCOL_VIOLATION", {"phase": self.phase})
        self.phase = "announced"
        return {
            "stage": {
                "id": self.stage_id,
                "consumes": self.stage.consumes,
                "produces": self.stage.produces,
                "capabilities": list(self.stage.capabilities),
                "configSchemaHash": self.config_schema_hash,
            }
        }

    async def _init(self, params: dict[str, Any]) -> None:
        if self.phase not in {"announced", "initialized"}:
            raise RpcFault(-32004, "PROTOCOL_VIOLATION", {"phase": self.phase})
        if self.phase == "initialized":
            return None
        self.last_config = params.get("config")
        self.phase = "initializing"
        self.lifecycle_task = asyncio.current_task()
        self.lifecycle_completion.clear()
        try:
            if self.stage.init is not None:
                assert self.peer is not None
                await self.stage.init(
                    self.last_config,
                    StageContext(self.peer, 0, CancellationToken(), self.last_config),
                )
            self.phase = "initialized"
            return None
        except BaseException as error:
            self.phase = "failed"
            raise _stage_fault(error) from error
        finally:
            self.lifecycle_task = None
            self.lifecycle_completion.set()

    async def _run(self, request_id: int, params: dict[str, Any]) -> dict[str, Any]:
        if self.phase != "initialized" or self.active_id is not None:
            raise RpcFault(-32004, "PROTOCOL_VIOLATION", {"phase": self.phase})
        stream_id = _safe_id(params.get("streamId"), "streamId")
        cancellation = CancellationToken()
        self.active_id = request_id
        self.cancellation = cancellation
        self.active_task = asyncio.current_task()
        self.active_completion.clear()
        self.last_config = params.get("config")
        input_stream_id: int | None = None
        output_iterator: Any = None
        try:
            input_value = params.get("input")
            if self.stage.consumes.startswith("Stream<"):
                if not _stream_handle(input_value):
                    raise ProtocolError("stream stage requires a canonical stream handle")
                assert isinstance(input_value, dict)
                input_stream_id = _safe_id(input_value["streamId"], "input streamId")
                stream = StreamInput(
                    self.max_buffered_stream_values, self.max_buffered_stream_bytes
                )
                self.inputs[input_stream_id] = stream
                input_value = stream
            assert self.peer is not None
            context = StageContext(
                self.peer,
                stream_id,
                cancellation,
                self.last_config,
                lambda handle: self._open_capability_stream(stream_id, handle),
            )
            output = await self.stage.run(input_value, params.get("config"), context)
            cancellation.throw_if_cancelled()
            if self.stage.produces.startswith("Stream<"):
                if not hasattr(output, "__aiter__"):
                    raise ProtocolError("stream stage returned a non-stream value")
                output_iterator = output.__aiter__()
                produced = 0
                async for value in output_iterator:
                    cancellation.throw_if_cancelled()
                    await self.peer.notify("stream.value", {"streamId": stream_id, "value": value})
                    produced += 1
                return {"kind": "stream", "streamId": stream_id, "produced": produced}
            if hasattr(output, "__aiter__"):
                raise ProtocolError("single stage returned a stream value")
            return {"kind": "single", "value": output}
        except BaseException as error:
            raise _stage_fault(error) from error
        finally:
            await self._close_capability_streams(stream_id)
            if output_iterator is not None:
                await _close_async_iterator(output_iterator)
            if input_stream_id is not None:
                stream = self.inputs.pop(input_stream_id)
                await stream.end()
            self.active_id = None
            self.cancellation = None
            self.active_task = None
            self.active_completion.set()

    async def _open_capability_stream(self, owner_run_id: int, handle: Any) -> AsyncIterator[Any]:
        if (
            not isinstance(handle, dict)
            or set(handle) != {"kind", "streamId"}
            or handle.get("kind") != "stream-handle"
        ):
            raise ProtocolError("storage.watch result is malformed")
        stream_id = _safe_id(handle.get("streamId"), "capability streamId")
        if stream_id == 0 or stream_id in self.inputs:
            raise ProtocolError("storage.watch returned a duplicate stream handle")
        stream = StreamInput(self.max_buffered_stream_values, self.max_buffered_stream_bytes)
        self.inputs[stream_id] = stream
        self.capability_input_ids.add(stream_id)
        started = False
        try:
            assert self.peer is not None
            await self.peer.notify("stream.start", {"streamId": stream_id})
            started = True
            async for value in stream:
                yield value
        finally:
            await self._close_capability_stream(owner_run_id, stream_id, started)

    async def _close_capability_stream(
        self, owner_run_id: int, stream_id: int, notify: bool
    ) -> None:
        if stream_id not in self.capability_input_ids:
            return
        self.capability_input_ids.remove(stream_id)
        self.retiring_capability_input_ids.add(stream_id)
        stream = self.inputs.pop(stream_id, None)
        if stream is not None:
            await stream.end()
        try:
            if notify:
                assert self.peer is not None
                await self.peer.request(
                    "stream.cancel",
                    {"streamId": owner_run_id, "capabilityStreamId": stream_id},
                )
            self.retiring_capability_input_ids.discard(stream_id)
        except BaseException:
            assert self.peer is not None
            self.peer.fail(ProtocolError("host did not acknowledge capability stream cancellation"))
            raise

    async def _close_capability_streams(self, owner_run_id: int) -> None:
        for stream_id in list(self.capability_input_ids):
            await self._close_capability_stream(owner_run_id, stream_id, True)

    async def _dispose(self) -> None:
        if self.phase == "disposed":
            return None
        if self.active_id is not None or self.phase not in {"initialized", "disposed"}:
            raise RpcFault(-32004, "PROTOCOL_VIOLATION", {"phase": self.phase})
        self.phase = "disposing"
        self.lifecycle_task = asyncio.current_task()
        self.lifecycle_completion.clear()
        for stream in self.inputs.values():
            await stream.end()
        self.inputs.clear()
        self.capability_input_ids.clear()
        self.retiring_capability_input_ids.clear()
        try:
            if self.stage.dispose is not None:
                assert self.peer is not None
                await self.stage.dispose(
                    StageContext(self.peer, 0, CancellationToken(), self.last_config)
                )
            self.phase = "disposed"
            return None
        except BaseException as error:
            self.phase = "failed"
            raise _stage_fault(error) from error
        finally:
            self.lifecycle_task = None
            self.lifecycle_completion.set()


def run_plugin(
    stage: Stage,
    *,
    argv: list[str] | None = None,
    input_stream: BinaryIO | None = None,
    output_stream: BinaryIO | None = None,
    max_frame_bytes: int = 8 * 1024 * 1024,
    max_header_bytes: int = 8 * 1024,
    max_buffered_stream_values: int = 64,
    max_buffered_stream_bytes: int = 8 * 1024 * 1024,
    signal_shutdown_timeout_ms: int = 1_000,
) -> None:
    """Run one authored stage as a bounded FM02 subprocess peer."""

    _validate_stage(stage)
    arguments = argv if argv is not None else sys.argv
    stage_id = arguments[1] if len(arguments) > 1 else stage.name.rsplit("/", 1)[-1]
    config_schema_hash = arguments[2] if len(arguments) > 2 and arguments[2] != "-" else None
    if not _token(stage_id, 128):
        raise ProtocolError("runner stage id is invalid")
    if config_schema_hash is not None and not _token(config_schema_hash, 256):
        raise ProtocolError("runner schema identity is invalid")
    for value in (
        max_frame_bytes,
        max_header_bytes,
        max_buffered_stream_values,
        max_buffered_stream_bytes,
        signal_shutdown_timeout_ms,
    ):
        if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
            raise ValueError("runner limits must be positive integers")
    asyncio.run(
        _run(
            stage,
            stage_id,
            config_schema_hash,
            input_stream or sys.stdin.buffer,
            output_stream or sys.stdout.buffer,
            max_frame_bytes,
            max_header_bytes,
            max_buffered_stream_values,
            max_buffered_stream_bytes,
            signal_shutdown_timeout_ms,
        )
    )


async def _run(
    stage: Stage,
    stage_id: str,
    config_schema_hash: str | None,
    input_stream: BinaryIO,
    output_stream: BinaryIO,
    max_frame_bytes: int,
    max_header_bytes: int,
    max_buffered_stream_values: int,
    max_buffered_stream_bytes: int,
    signal_shutdown_timeout_ms: int,
) -> None:
    state = RunnerState(
        stage,
        stage_id,
        config_schema_hash,
        max_buffered_stream_values,
        max_buffered_stream_bytes,
    )
    peer = Peer(
        input_stream,
        output_stream,
        max_frame_bytes=max_frame_bytes,
        max_header_bytes=max_header_bytes,
        on_request=state.request,
        on_notification=state.notification,
    )
    state.peer = peer
    loop = asyncio.get_running_loop()
    installed_signals: list[signal.Signals] = []
    shutdown_task: asyncio.Task[bool] | None = None
    force_exit: asyncio.TimerHandle | None = None

    def on_signal() -> None:
        nonlocal force_exit, shutdown_task
        if shutdown_task is None:
            force_exit = loop.call_later(signal_shutdown_timeout_ms / 1000 + 0.1, os._exit, 1)
            shutdown_task = asyncio.create_task(
                _bounded_signal_shutdown(state, peer, signal_shutdown_timeout_ms)
            )

    for signum in (signal.SIGINT, signal.SIGTERM):
        with contextlib.suppress(NotImplementedError, RuntimeError, ValueError):
            loop.add_signal_handler(signum, on_signal)
            installed_signals.append(signum)
    try:
        await peer.run()
    finally:
        for signum in installed_signals:
            with contextlib.suppress(NotImplementedError, RuntimeError, ValueError):
                loop.remove_signal_handler(signum)
        if shutdown_task is not None:
            with contextlib.suppress(BaseException):
                if await shutdown_task and force_exit is not None:
                    force_exit.cancel()


async def _bounded_signal_shutdown(state: RunnerState, peer: Peer, timeout_ms: int) -> bool:
    try:
        await asyncio.wait_for(state.shutdown(), timeout=timeout_ms / 1000)
        return True
    except BaseException:
        peer.shutdown()
        return False


def _record(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ProtocolError(f"{label} must be an object")
    return value


def _safe_id(value: Any, label: str) -> int:
    if (
        isinstance(value, bool)
        or not isinstance(value, int)
        or value < 0
        or value > MAX_SAFE_INTEGER
    ):
        raise RpcFault(-32602, "INVALID_PARAMS", {"field": label})
    return value


def _stream_handle(value: Any) -> bool:
    return (
        isinstance(value, dict)
        and set(value) == {"kind", "streamId"}
        and value.get("kind") == "stream-handle"
    )


def _stage_fault(error: BaseException) -> RpcFault:
    if isinstance(error, RpcFault):
        return error
    if isinstance(error, asyncio.CancelledError):
        return RpcFault(-32800, str(error) or "operation cancelled", {})
    if isinstance(error, CancellationError):
        return RpcFault(-32800, str(error), {})
    if isinstance(error, StageError):
        return RpcFault(
            -32900,
            str(error),
            {
                "stageErrorCode": error.code,
                "inputPath": error.input_path,
                "inputId": error.input_id,
                "stageName": error.stage_name,
                "recoverable": error.recoverable,
                "fields": error.fields,
            },
        )
    if isinstance(error, ProtocolError):
        return RpcFault(-32004, "PROTOCOL_VIOLATION", {})
    return RpcFault(-32603, "Plugin stage failed", {})


async def _close_async_iterator(iterator: Any) -> None:
    close = getattr(iterator, "aclose", None)
    if close is None:
        return
    with contextlib.suppress(BaseException):
        await asyncio.wait_for(close(), timeout=0.25)


def _validate_stage(stage: Stage) -> None:
    if (
        not _token(stage.name, 256)
        or not _token(stage.version, 128)
        or not _token(stage.description, 16_384)
        or stage.api_version != 1
        or not _kind(stage.consumes)
        or not _kind(stage.produces)
        or len(stage.capabilities) > 256
        or any(not _token(value, 512) for value in stage.capabilities)
        or (stage.init is not None and not callable(stage.init))
        or (stage.dispose is not None and not callable(stage.dispose))
    ):
        raise ProtocolError("stage metadata is invalid")


def _kind(value: str) -> bool:
    if value.startswith("Stream<") and value.endswith(">"):
        inner = value[7:-1]
        return _token(inner, 128) and not inner.startswith("Stream<")
    return _token(value, 128)


def _token(value: Any, max_length: int) -> bool:
    return isinstance(value, str) and 0 < len(value) <= max_length and "\0" not in value
