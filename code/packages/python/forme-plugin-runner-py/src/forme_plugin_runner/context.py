"""Wire-backed asynchronous StageContext APIs."""

from __future__ import annotations

import base64
import binascii
import time
from collections.abc import AsyncIterator, Sequence
from dataclasses import dataclass
from typing import Any

from .peer import Peer, RemoteError
from .stage import CancellationError, CancellationToken, CapabilityError, StageError

MAX_MEDIATED_BYTES = 1024 * 1024


def _decode_bytes(value: Any, label: str) -> bytes:
    if not isinstance(value, str):
        raise StageError("PLUGIN_PROTOCOL_ERROR", f"{label} is malformed")
    try:
        decoded = base64.b64decode(value, validate=True)
    except (binascii.Error, ValueError) as error:
        raise StageError("PLUGIN_PROTOCOL_ERROR", f"{label} is malformed") from error
    if base64.b64encode(decoded).decode("ascii") != value:
        raise StageError("PLUGIN_PROTOCOL_ERROR", f"{label} is malformed")
    if len(decoded) > MAX_MEDIATED_BYTES:
        raise StageError("RESOURCE_LIMIT_EXCEEDED", "mediated bytes exceed the configured bound")
    return decoded


def _result_bytes(value: Any, label: str) -> bytes:
    if not isinstance(value, dict):
        raise StageError("PLUGIN_PROTOCOL_ERROR", f"{label} result is malformed")
    return _decode_bytes(value.get("bytes"), f"{label} bytes")


class _Api:
    def __init__(self, context: StageContext) -> None:
        self._context = context

    async def _request(self, method: str, params: dict[str, Any] | None = None) -> Any:
        return await self._context._request(method, params or {})


class StorageApi(_Api):
    async def read(self, path: str) -> bytes:
        return _result_bytes(
            await self._request("ctx.storage.read", {"path": path}), "storage.read"
        )

    async def read_bounded(self, path: str, max_bytes: int) -> bytes:
        _assert_bound(max_bytes)
        value = await self.read(path)
        if len(value) > max_bytes:
            raise StageError("RESOURCE_LIMIT_EXCEEDED", "storage value exceeds requested bound")
        return value

    async def write(self, path: str, value: bytes) -> None:
        _assert_bytes(value)
        await self._request(
            "ctx.storage.write",
            {"path": path, "bytes": base64.b64encode(value).decode("ascii")},
        )

    async def exists(self, path: str) -> bool:
        value = await self._request("ctx.storage.exists", {"path": path})
        if not isinstance(value, bool):
            raise StageError("PLUGIN_PROTOCOL_ERROR", "storage.exists result is malformed")
        return value

    async def stat(self, path: str) -> dict[str, Any]:
        value = await self._request("ctx.storage.stat", {"path": path})
        if not isinstance(value, dict):
            raise StageError("PLUGIN_PROTOCOL_ERROR", "storage.stat result is malformed")
        return value

    async def list(self, path: str) -> AsyncIterator[Any]:
        values = await self._request("ctx.storage.list", {"path": path})
        if not isinstance(values, list):
            raise StageError("PLUGIN_PROTOCOL_ERROR", "storage.list result is malformed")
        for value in values:
            yield value

    async def watch(self, path: str) -> AsyncIterator[Any]:
        values = await self._request("ctx.storage.watch", {"path": path})
        if not isinstance(values, list):
            raise StageError("PLUGIN_PROTOCOL_ERROR", "storage.watch result is malformed")
        for value in values:
            yield value

    async def remove(self, path: str) -> None:
        await self._request("ctx.storage.remove", {"path": path})


class EnvApi(_Api):
    async def get(self, name: str) -> str | None:
        value = await self._request("ctx.env.get", {"name": name})
        if value is None:
            return None
        if not isinstance(value, str):
            raise StageError("PLUGIN_PROTOCOL_ERROR", "env.get result is malformed")
        return value

    async def get_or_throw(self, name: str) -> str:
        value = await self.get(name)
        if value is None:
            raise StageError("ENV_MISSING", f"Environment variable {name} is not set")
        return value


class TimeApi(_Api):
    async def now_ms(self) -> float:
        value = await self._request("ctx.time.nowMs")
        if not isinstance(value, int | float) or isinstance(value, bool):
            raise StageError("PLUGIN_PROTOCOL_ERROR", "time.nowMs result is malformed")
        return float(value) if isinstance(value, float) else value

    async def now_iso(self) -> str:
        value = await self._request("ctx.time.nowIso")
        if not isinstance(value, str):
            raise StageError("PLUGIN_PROTOCOL_ERROR", "time.nowIso result is malformed")
        return value

    def monotonic_ms(self) -> float:
        return time.monotonic() * 1000


class FilesystemApi(_Api):
    async def home_dir(self) -> str:
        return await self._string("ctx.filesystem.homeDir", "filesystem.homeDir")

    async def temp_dir(self) -> str:
        return await self._string("ctx.filesystem.tempDir", "filesystem.tempDir")

    async def read_absolute(self, path: str) -> bytes:
        return _result_bytes(
            await self._request("ctx.filesystem.readAbsolute", {"path": path}),
            "filesystem.readAbsolute",
        )

    async def read_absolute_bounded(self, path: str, max_bytes: int) -> bytes:
        _assert_bound(max_bytes)
        value = await self.read_absolute(path)
        if len(value) > max_bytes:
            raise StageError("RESOURCE_LIMIT_EXCEEDED", "filesystem value exceeds requested bound")
        return value

    async def write_absolute(self, path: str, value: bytes) -> None:
        _assert_bytes(value)
        await self._request(
            "ctx.filesystem.writeAbsolute",
            {"path": path, "bytes": base64.b64encode(value).decode("ascii")},
        )

    async def _string(self, method: str, label: str) -> str:
        value = await self._request(method)
        if not isinstance(value, str):
            raise StageError("PLUGIN_PROTOCOL_ERROR", f"{label} result is malformed")
        return value


@dataclass(frozen=True)
class NetworkResponse:
    status: int
    status_text: str
    headers: dict[str, str]
    url: str
    _bytes: bytes

    async def array_buffer(self) -> bytes:
        return self._bytes


class NetworkApi(_Api):
    async def fetch(self, url: str, init: dict[str, Any] | None = None) -> NetworkResponse:
        source = init or {}
        body = source.get("body")
        encoded_body: str | None = None
        if body is not None:
            if isinstance(body, str):
                body = body.encode()
            if not isinstance(body, bytes | bytearray | memoryview):
                raise StageError("INVALID_NETWORK_BODY", "plugin fetch body must be text or bytes")
            _assert_bytes(bytes(body))
            encoded_body = base64.b64encode(bytes(body)).decode("ascii")
        request_init = {
            "method": source.get("method"),
            "headers": source.get("headers"),
            "body": encoded_body,
        }
        result = await self._request("ctx.network.fetch", {"url": url, "init": request_init})
        if not isinstance(result, dict) or not isinstance(result.get("status"), int):
            raise StageError("PLUGIN_PROTOCOL_ERROR", "network.fetch result is malformed")
        headers = result.get("headers", {})
        if not isinstance(headers, dict) or not all(
            isinstance(key, str) and isinstance(value, str) for key, value in headers.items()
        ):
            raise StageError("PLUGIN_PROTOCOL_ERROR", "network.fetch headers are malformed")
        status_text = result.get("statusText")
        response_url = result.get("url")
        return NetworkResponse(
            status=result["status"],
            status_text=status_text if isinstance(status_text, str) else "",
            headers=headers,
            url=response_url if isinstance(response_url, str) else url,
            _bytes=_decode_bytes(result.get("bytes"), "network.fetch bytes"),
        )


class ShellApi(_Api):
    async def run(
        self, command: str, args: Sequence[str], options: dict[str, Any] | None = None
    ) -> dict[str, Any]:
        source = dict(options or {})
        stdin = source.get("stdin")
        if stdin is not None:
            if not isinstance(stdin, bytes | bytearray | memoryview):
                raise StageError("PLUGIN_PROTOCOL_ERROR", "shell stdin is malformed")
            _assert_bytes(bytes(stdin))
            source["stdin"] = base64.b64encode(bytes(stdin)).decode("ascii")
        result = await self._request(
            "ctx.shell.run", {"command": command, "args": list(args), "options": source}
        )
        if not isinstance(result, dict) or not isinstance(result.get("exitCode"), int):
            raise StageError("PLUGIN_PROTOCOL_ERROR", "shell.run result is malformed")
        return {
            "exitCode": result["exitCode"],
            "stdout": _decode_bytes(result.get("stdout"), "shell.run stdout"),
            "stderr": _decode_bytes(result.get("stderr"), "shell.run stderr"),
        }


class Logger:
    def __init__(self, peer: Peer, fields: dict[str, Any] | None = None) -> None:
        self._peer = peer
        self._fields = fields or {}

    def _emit(self, level: str, message: str, fields: dict[str, Any] | None) -> None:
        self._peer.notify_nowait(
            "log",
            {"level": level, "message": message, "fields": {**self._fields, **(fields or {})}},
        )

    def trace(self, message: str, fields: dict[str, Any] | None = None) -> None:
        self._emit("trace", message, fields)

    def debug(self, message: str, fields: dict[str, Any] | None = None) -> None:
        self._emit("debug", message, fields)

    def info(self, message: str, fields: dict[str, Any] | None = None) -> None:
        self._emit("info", message, fields)

    def warn(self, message: str, fields: dict[str, Any] | None = None) -> None:
        self._emit("warn", message, fields)

    def error(self, message: str, fields: dict[str, Any] | None = None) -> None:
        self._emit("error", message, fields)

    def child(self, fields: dict[str, Any]) -> Logger:
        return Logger(self._peer, {**self._fields, **fields})


class StageContext:
    def __init__(
        self,
        peer: Peer,
        stream_id: int,
        cancellation: CancellationToken,
        config: Any = None,
    ) -> None:
        self._peer = peer
        self._stream_id = stream_id
        self.cancellation = cancellation
        self.config = config
        self.storage = StorageApi(self)
        self.env = EnvApi(self)
        self.time = TimeApi(self)
        self.filesystem = FilesystemApi(self)
        self.network = NetworkApi(self)
        self.shell = ShellApi(self)
        self.logger = Logger(peer)

    async def _request(self, method: str, params: dict[str, Any]) -> Any:
        self.cancellation.throw_if_cancelled()
        try:
            result = await self._peer.request(method, {**params, "streamId": self._stream_id})
        except RemoteError as error:
            if error.code == -32800:
                raise CancellationError(str(error)) from error
            if error.code == -32001:
                data = error.data if isinstance(error.data, dict) else {}
                capability = data.get("capability", "unknown")
                raise CapabilityError(
                    str(error), capability if isinstance(capability, str) else "unknown"
                ) from error
            raise StageError(
                "PLUGIN_PROTOCOL_ERROR", str(error), fields={"rpcCode": error.code}
            ) from error
        self.cancellation.throw_if_cancelled()
        return result


def _assert_bound(value: int) -> None:
    if (
        isinstance(value, bool)
        or not isinstance(value, int)
        or value < 0
        or value > MAX_MEDIATED_BYTES
    ):
        raise ValueError("max_bytes must be within the mediated byte limit")


def _assert_bytes(value: bytes) -> None:
    if len(value) > MAX_MEDIATED_BYTES:
        raise StageError("RESOURCE_LIMIT_EXCEEDED", "mediated bytes exceed the configured bound")
