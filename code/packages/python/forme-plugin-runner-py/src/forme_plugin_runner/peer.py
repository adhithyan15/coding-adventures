"""Concurrent JSON-RPC peer over bounded stdin/stdout frames."""

from __future__ import annotations

import asyncio
import contextlib
import os
import threading
from collections.abc import Awaitable, Callable
from typing import Any, BinaryIO

from .wire import FrameDecoder, ProtocolError, encode_frame

RequestHandler = Callable[[int, str, Any], Awaitable[Any]]
NotificationHandler = Callable[[str, Any], Awaitable[None]]


class RemoteError(Exception):
    def __init__(self, code: int, message: str, data: Any) -> None:
        super().__init__(message)
        self.code = code
        self.data = data


class RpcFault(Exception):
    def __init__(self, code: int, message: str, data: Any = None) -> None:
        super().__init__(message)
        self.code = code
        self.data = data


class Peer:
    def __init__(
        self,
        input_stream: BinaryIO,
        output_stream: BinaryIO,
        *,
        max_frame_bytes: int,
        max_header_bytes: int,
        max_ingress_chunks: int = 4,
        max_inflight_requests: int = 32,
        max_pending_requests: int = 64,
        max_outgoing_notifications: int = 64,
        on_request: RequestHandler,
        on_notification: NotificationHandler,
    ) -> None:
        self._input = input_stream
        self._output = output_stream
        self._decoder = FrameDecoder(max_frame_bytes, max_header_bytes)
        self._max_frame_bytes = max_frame_bytes
        self._max_ingress_chunks = max_ingress_chunks
        self._max_inflight_requests = max_inflight_requests
        self._max_pending_requests = max_pending_requests
        self._max_outgoing_notifications = max_outgoing_notifications
        self._on_request = on_request
        self._on_notification = on_notification
        self._next_id = -1
        self._pending: dict[int, asyncio.Future[Any]] = {}
        self._answers: set[asyncio.Task[None]] = set()
        self._outgoing: set[asyncio.Task[None]] = set()
        self._write_lock = asyncio.Lock()
        self._stop = asyncio.Event()
        self._stop_after_response = False
        self._failure: BaseException | None = None

    async def run(self) -> None:
        loop = asyncio.get_running_loop()
        chunks: asyncio.Queue[bytes | BaseException | None] = asyncio.Queue(
            maxsize=self._max_ingress_chunks
        )

        def read_forever() -> None:
            def put(value: bytes | BaseException | None) -> None:
                asyncio.run_coroutine_threadsafe(chunks.put(value), loop).result()

            try:
                descriptor = self._input.fileno()
                while True:
                    chunk = os.read(descriptor, 65536)
                    put(chunk or None)
                    if not chunk:
                        return
            except BaseException as error:
                with contextlib.suppress(BaseException):
                    put(error)

        threading.Thread(target=read_forever, daemon=True, name="forme-runner-stdin").start()
        try:
            while not self._stop.is_set():
                chunk_task = asyncio.create_task(chunks.get())
                stop_task = asyncio.create_task(self._stop.wait())
                done, pending = await asyncio.wait(
                    {chunk_task, stop_task}, return_when=asyncio.FIRST_COMPLETED
                )
                for task in pending:
                    task.cancel()
                if stop_task in done and stop_task.result():
                    chunk_task.cancel()
                    break
                chunk = chunk_task.result()
                if isinstance(chunk, BaseException):
                    raise chunk
                if chunk is None:
                    self._decoder.finish()
                    if not self._stop_after_response:
                        raise ProtocolError("host wire closed unexpectedly")
                    break
                for message in self._decoder.push(chunk):
                    await self._receive(message)
        except BaseException as error:
            self.fail(error)
            raise
        finally:
            if self._answers:
                for task in tuple(self._answers):
                    task.cancel()
                await asyncio.gather(*tuple(self._answers), return_exceptions=True)
            if self._outgoing:
                for task in tuple(self._outgoing):
                    task.cancel()
                await asyncio.gather(*tuple(self._outgoing), return_exceptions=True)

    async def request(self, method: str, params: Any) -> Any:
        self._assert_open()
        if len(self._pending) >= self._max_pending_requests:
            raise ProtocolError("too many pending plugin requests")
        request_id = self._next_id
        self._next_id -= 1
        future: asyncio.Future[Any] = asyncio.get_running_loop().create_future()
        self._pending[request_id] = future
        try:
            await self._write(
                {"jsonrpc": "2.0", "id": request_id, "method": method, "params": params}
            )
        except BaseException:
            self._pending.pop(request_id, None)
            raise
        try:
            return await future
        finally:
            pending = self._pending.pop(request_id, None)
            if pending is not None and not pending.done():
                pending.cancel()

    async def notify(self, method: str, params: Any) -> None:
        self._assert_open()
        await self._write({"jsonrpc": "2.0", "method": method, "params": params})

    def notify_nowait(self, method: str, params: Any) -> None:
        self._assert_open()
        if len(self._outgoing) >= self._max_outgoing_notifications:
            raise ProtocolError("too many pending plugin notifications")
        task = asyncio.create_task(self.notify(method, params))
        self._outgoing.add(task)

    async def drain_notifications(self) -> None:
        if self._outgoing:
            tasks = tuple(self._outgoing)
            try:
                results = await asyncio.gather(*tasks, return_exceptions=True)
            finally:
                self._outgoing.difference_update(tasks)
            for result in results:
                if isinstance(result, BaseException):
                    raise result

    def request_stop_after_response(self) -> None:
        self._stop_after_response = True

    def shutdown(self) -> None:
        self._stop_after_response = True
        self.fail(ProtocolError("runner shutdown"))

    def fail(self, error: BaseException) -> None:
        if self._failure is not None:
            return
        self._failure = error
        for future in self._pending.values():
            if not future.done():
                future.set_exception(error)
        self._pending.clear()
        self._stop.set()

    async def _receive(self, message: dict[str, Any]) -> None:
        if message.get("jsonrpc") != "2.0":
            raise ProtocolError("jsonrpc must equal 2.0")
        method = message.get("method")
        if isinstance(method, str):
            if "id" not in message:
                await self._on_notification(method, message.get("params"))
                return
            request_id = message["id"]
            if not _safe_integer(request_id) or request_id <= 0:
                raise ProtocolError("host request ids must be positive safe integers")
            if len(self._answers) >= self._max_inflight_requests:
                raise ProtocolError("too many concurrent host requests")
            task = asyncio.create_task(self._answer(request_id, method, message.get("params")))
            self._answers.add(task)
            task.add_done_callback(self._answers.discard)
            await asyncio.sleep(0)
            return
        request_id = message.get("id")
        if not _safe_integer(request_id):
            raise ProtocolError("plugin response ids must be negative safe integers")
        assert isinstance(request_id, int)
        if request_id >= 0:
            raise ProtocolError("plugin response ids must be negative safe integers")
        future = self._pending.pop(request_id, None)
        if future is None:
            raise ProtocolError("response has an unknown request id")
        has_result = "result" in message
        has_error = "error" in message
        if has_result == has_error:
            future.set_exception(ProtocolError("response must contain exactly one result or error"))
            return
        if has_result:
            future.set_result(message["result"])
            return
        error = message["error"]
        if (
            not isinstance(error, dict)
            or not _safe_integer(error.get("code"))
            or not isinstance(error.get("message"), str)
        ):
            future.set_exception(ProtocolError("malformed JSON-RPC error"))
            return
        future.set_exception(RemoteError(error["code"], error["message"], error.get("data")))

    async def _answer(self, request_id: int, method: str, params: Any) -> None:
        try:
            result = await self._on_request(request_id, method, params)
            await self.drain_notifications()
            await self._write({"jsonrpc": "2.0", "id": request_id, "result": result})
        except BaseException as error:
            fault = _rpc_error(error)
            with contextlib.suppress(BaseException):
                await self._write({"jsonrpc": "2.0", "id": request_id, "error": fault})
        finally:
            if self._stop_after_response:
                self._stop.set()

    async def _write(self, message: dict[str, Any]) -> None:
        frame = encode_frame(message, self._max_frame_bytes)
        async with self._write_lock:
            await asyncio.to_thread(self._write_sync, frame)

    def _write_sync(self, frame: bytes) -> None:
        self._output.write(frame)
        self._output.flush()

    def _assert_open(self) -> None:
        if self._failure is not None:
            raise self._failure


def _safe_integer(value: Any) -> bool:
    return (
        isinstance(value, int)
        and not isinstance(value, bool)
        and abs(value) <= 9_007_199_254_740_991
    )


def _rpc_error(error: BaseException) -> dict[str, Any]:
    if isinstance(error, RpcFault):
        result: dict[str, Any] = {"code": error.code, "message": str(error)}
        if error.data is not None:
            result["data"] = error.data
        return result
    return {"code": -32603, "message": "INTERNAL_ERROR"}
