"""Comprehensive tests for the rpc package.

Test Strategy
-------------

Because the ``rpc`` package is codec-agnostic, tests use a pair of lightweight
test doubles:

- **MockCodec** — a simple codec that serialises :class:`~rpc.message.RpcMessage`
  objects to/from Python's ``pickle`` format (stdlib, zero deps, supports all
  message types out of the box).
- **MockFramer** — a framer backed by a pair of ``io.BytesIO`` buffers (one for
  reading, one for writing).  Each frame is length-prefixed with a 4-byte
  big-endian integer.

This setup lets every test exercise real server and client code paths without
any network I/O, JSON parsing, or Content-Length headers.

Test groups
-----------

1. **Messages** — dataclass construction, equality, repr.
2. **Errors** — constants, RpcDecodeError.
3. **MockCodec / MockFramer** — verify the test doubles behave correctly.
4. **RpcServer** — dispatch, method-not-found, handler errors, notifications,
   panic safety, decode errors, round-trips.
5. **RpcClient** — request happy path, error response, connection closed,
   notify, server-push notifications, monotonic ids.
6. **Integration** — client → server round-trip through shared BytesIO.
7. **check_codec / check_framer** — helper guards.
"""

from __future__ import annotations

import io
import pickle
import struct
from typing import Any, Optional

import pytest

import rpc
from rpc import (
    INTERNAL_ERROR,
    INVALID_REQUEST,
    METHOD_NOT_FOUND,
    PARSE_ERROR,
    RpcClient,
    RpcDecodeError,
    RpcErrorResponse,
    RpcNotification,
    RpcRemoteError,
    RpcRequest,
    RpcResponse,
    RpcServer,
)
from rpc.codec import check_codec
from rpc.framer import check_framer


# ===========================================================================
# Test doubles
# ===========================================================================


class MockCodec:
    """A codec that serialises RpcMessage objects using pickle.

    Pickle handles all Python dataclasses natively, making it perfect for
    testing without any external dependencies.

    Injecting a ``fail_decode`` flag causes the next ``decode()`` call to raise
    an :class:`~rpc.errors.RpcDecodeError`, simulating a bad frame.
    """

    def __init__(self, fail_decode: bool = False, fail_code: int = PARSE_ERROR) -> None:
        self.fail_decode = fail_decode
        self.fail_code = fail_code
        self.encoded: list[bytes] = []   # accumulates all encoded payloads

    def encode(self, msg: Any) -> bytes:
        data = pickle.dumps(msg)
        self.encoded.append(data)
        return data

    def decode(self, data: bytes) -> Any:
        if self.fail_decode:
            self.fail_decode = False   # fail once, then recover
            raise RpcDecodeError(self.fail_code, "simulated decode failure")
        return pickle.loads(data)  # noqa: S301 — safe in tests


class MockFramer:
    """A framer backed by two ``io.BytesIO`` buffers.

    Frames are length-prefixed: ``<4-byte big-endian length><payload>``.

    ``read_buf``: bytes the *server/client under test* will read from.
    ``write_buf``: bytes the *server/client under test* will write to.

    Typical test setup::

        in_buf  = io.BytesIO()   # will hold frames to be read
        out_buf = io.BytesIO()   # will hold frames written by the server
        framer  = MockFramer(in_buf, out_buf)

        # Pre-populate in_buf with a request frame:
        encode_frame(in_buf, codec.encode(request))
        in_buf.seek(0)
    """

    def __init__(self, read_buf: io.BytesIO, write_buf: io.BytesIO) -> None:
        self._read_buf = read_buf
        self._write_buf = write_buf

    def read_frame(self) -> Optional[bytes]:
        header = self._read_buf.read(4)
        if len(header) < 4:
            return None   # EOF
        (length,) = struct.unpack(">I", header)
        payload = self._read_buf.read(length)
        if len(payload) < length:
            return None   # truncated — treat as EOF
        return payload

    def write_frame(self, data: bytes) -> None:
        header = struct.pack(">I", len(data))
        self._write_buf.write(header + data)


# ---------------------------------------------------------------------------
# Frame helper: encode a single frame into a BytesIO
# ---------------------------------------------------------------------------


def make_frame(payload: bytes) -> bytes:
    """Wrap *payload* in a 4-byte length-prefix frame."""
    return struct.pack(">I", len(payload)) + payload


def frames_to_buf(*payloads: bytes) -> io.BytesIO:
    """Return a BytesIO containing one or more frames ready to be read."""
    data = b"".join(make_frame(p) for p in payloads)
    return io.BytesIO(data)


# ---------------------------------------------------------------------------
# Helpers for reading written frames back out
# ---------------------------------------------------------------------------


def read_all_frames(buf: io.BytesIO) -> list[bytes]:
    """Read all frames from *buf* (which has been written to by the SUT)."""
    buf.seek(0)
    framer = MockFramer(buf, io.BytesIO())
    frames: list[bytes] = []
    while True:
        frame = framer.read_frame()
        if frame is None:
            break
        frames.append(frame)
    return frames


# ===========================================================================
# 1. Message types
# ===========================================================================


class TestMessages:
    """Tests for the four RpcMessage dataclasses."""

    def test_request_defaults(self) -> None:
        req = RpcRequest(id=1, method="ping")
        assert req.id == 1
        assert req.method == "ping"
        assert req.params is None

    def test_request_with_params(self) -> None:
        req = RpcRequest(id="abc", method="add", params={"a": 1, "b": 2})
        assert req.id == "abc"
        assert req.params == {"a": 1, "b": 2}

    def test_response_defaults(self) -> None:
        resp = RpcResponse(id=1)
        assert resp.result is None

    def test_response_with_result(self) -> None:
        resp = RpcResponse(id=42, result={"ok": True})
        assert resp.id == 42
        assert resp.result == {"ok": True}

    def test_error_response_required_fields(self) -> None:
        err = RpcErrorResponse(code=-32601, message="Method not found")
        assert err.code == -32601
        assert err.message == "Method not found"
        assert err.id is None
        assert err.data is None

    def test_error_response_with_all_fields(self) -> None:
        err = RpcErrorResponse(code=-32603, message="boom", id=7, data="traceback")
        assert err.id == 7
        assert err.data == "traceback"

    def test_notification_defaults(self) -> None:
        notif = RpcNotification(method="log")
        assert notif.method == "log"
        assert notif.params is None

    def test_notification_with_params(self) -> None:
        notif = RpcNotification(method="ping", params={"ts": 1234})
        assert notif.params == {"ts": 1234}

    def test_message_equality(self) -> None:
        r1 = RpcRequest(id=1, method="foo", params={"x": 1})
        r2 = RpcRequest(id=1, method="foo", params={"x": 1})
        assert r1 == r2

    def test_message_inequality(self) -> None:
        r1 = RpcRequest(id=1, method="foo")
        r2 = RpcRequest(id=2, method="foo")
        assert r1 != r2

    def test_repr_contains_method(self) -> None:
        req = RpcRequest(id=5, method="greet")
        assert "greet" in repr(req)


# ===========================================================================
# 2. Error codes and RpcDecodeError
# ===========================================================================


class TestErrors:
    """Tests for error code constants and RpcDecodeError."""

    def test_error_codes_are_correct(self) -> None:
        assert rpc.PARSE_ERROR == -32700
        assert rpc.INVALID_REQUEST == -32600
        assert rpc.METHOD_NOT_FOUND == -32601
        assert rpc.INVALID_PARAMS == -32602
        assert rpc.INTERNAL_ERROR == -32603

    def test_rpc_decode_error_inherits_exception(self) -> None:
        err = RpcDecodeError(PARSE_ERROR, "bad bytes")
        assert isinstance(err, Exception)

    def test_rpc_decode_error_attributes(self) -> None:
        err = RpcDecodeError(-32600, "not a request")
        assert err.code == -32600
        assert err.message == "not a request"

    def test_rpc_decode_error_str(self) -> None:
        err = RpcDecodeError(PARSE_ERROR, "oops")
        assert "oops" in str(err)

    def test_rpc_decode_error_repr(self) -> None:
        err = RpcDecodeError(PARSE_ERROR, "oops")
        r = repr(err)
        assert "RpcDecodeError" in r
        assert "-32700" in r

    def test_rpc_decode_error_is_raiseable(self) -> None:
        with pytest.raises(RpcDecodeError) as exc_info:
            raise RpcDecodeError(PARSE_ERROR, "fail")
        assert exc_info.value.code == PARSE_ERROR


# ===========================================================================
# 3. MockCodec and MockFramer (verify test doubles)
# ===========================================================================


class TestMockCodec:
    """Verify the MockCodec test double itself."""

    def test_round_trip_request(self) -> None:
        codec = MockCodec()
        msg = RpcRequest(id=1, method="add", params={"a": 1})
        assert codec.decode(codec.encode(msg)) == msg

    def test_round_trip_response(self) -> None:
        codec = MockCodec()
        msg = RpcResponse(id=1, result=42)
        assert codec.decode(codec.encode(msg)) == msg

    def test_round_trip_error_response(self) -> None:
        codec = MockCodec()
        msg = RpcErrorResponse(code=-32601, message="not found", id=1)
        assert codec.decode(codec.encode(msg)) == msg

    def test_round_trip_notification(self) -> None:
        codec = MockCodec()
        msg = RpcNotification(method="ping", params=None)
        assert codec.decode(codec.encode(msg)) == msg

    def test_fail_decode_raises(self) -> None:
        codec = MockCodec(fail_decode=True)
        with pytest.raises(RpcDecodeError):
            codec.decode(b"garbage")

    def test_fail_decode_recovers(self) -> None:
        """After a single failure, the codec should work normally again."""
        codec = MockCodec(fail_decode=True)
        msg = RpcNotification(method="x")
        with pytest.raises(RpcDecodeError):
            codec.decode(codec.encode(msg))
        # Second call should succeed.
        assert codec.decode(codec.encode(msg)) == msg


class TestMockFramer:
    """Verify the MockFramer test double itself."""

    def test_write_then_read_round_trips(self) -> None:
        buf = io.BytesIO()
        framer = MockFramer(buf, buf)
        payload = b"hello world"
        framer.write_frame(payload)
        buf.seek(0)
        assert framer.read_frame() == payload

    def test_multiple_frames(self) -> None:
        write_buf = io.BytesIO()
        payloads = [b"frame1", b"frame number two", b"3"]
        for p in payloads:
            framer = MockFramer(io.BytesIO(), write_buf)
            framer.write_frame(p)

        write_buf.seek(0)
        read_framer = MockFramer(write_buf, io.BytesIO())
        result = []
        while True:
            frame = read_framer.read_frame()
            if frame is None:
                break
            result.append(frame)

        assert result == payloads

    def test_eof_returns_none(self) -> None:
        framer = MockFramer(io.BytesIO(b""), io.BytesIO())
        assert framer.read_frame() is None


# ===========================================================================
# 4. RpcServer
# ===========================================================================


class TestRpcServer:
    """Tests for RpcServer — dispatch, error handling, panic safety."""

    def _make_server(
        self,
        *request_frames: bytes,
        fail_decode: bool = False,
        fail_code: int = PARSE_ERROR,
    ) -> tuple[RpcServer, MockCodec, io.BytesIO]:
        """Helper: build a server with pre-loaded read frames."""
        codec = MockCodec(fail_decode=fail_decode, fail_code=fail_code)
        read_buf = frames_to_buf(*request_frames)
        write_buf = io.BytesIO()
        framer = MockFramer(read_buf, write_buf)
        server: RpcServer = RpcServer(codec, framer)
        return server, codec, write_buf

    def _read_response(self, codec: MockCodec, write_buf: io.BytesIO) -> Any:
        """Read back the single response written by the server."""
        frames = read_all_frames(write_buf)
        assert len(frames) == 1
        return codec.decode(frames[0])

    def _read_responses(self, codec: MockCodec, write_buf: io.BytesIO) -> list[Any]:
        """Read back all responses written by the server."""
        frames = read_all_frames(write_buf)
        return [codec.decode(f) for f in frames]

    # --- Happy path ---

    def test_dispatches_request_to_handler(self) -> None:
        codec = MockCodec()
        req = RpcRequest(id=1, method="add", params={"a": 3, "b": 4})
        server, codec, write_buf = self._make_server(codec.encode(req))
        server.on_request("add", lambda id, p: p["a"] + p["b"])
        server.serve()

        resp = self._read_response(codec, write_buf)
        assert isinstance(resp, RpcResponse)
        assert resp.id == 1
        assert resp.result == 7

    def test_handler_returns_none_result(self) -> None:
        """Handlers that return None produce a response with result=None."""
        codec = MockCodec()
        req = RpcRequest(id=5, method="noop")
        server, codec, write_buf = self._make_server(codec.encode(req))
        server.on_request("noop", lambda id, p: None)
        server.serve()

        resp = self._read_response(codec, write_buf)
        assert isinstance(resp, RpcResponse)
        assert resp.result is None

    def test_dispatches_notification_to_handler(self) -> None:
        received: list[Any] = []
        codec = MockCodec()
        notif = RpcNotification(method="log", params={"msg": "hello"})
        server, codec, write_buf = self._make_server(codec.encode(notif))
        server.on_notification("log", lambda p: received.append(p))
        server.serve()

        # No response should be written for a notification.
        assert read_all_frames(write_buf) == []
        assert received == [{"msg": "hello"}]

    def test_method_chaining(self) -> None:
        """on_request and on_notification return self for chaining."""
        codec = MockCodec()
        server: RpcServer = RpcServer(codec, MockFramer(io.BytesIO(), io.BytesIO()))
        result = server.on_request("x", lambda id, p: None).on_notification("y", lambda p: None)
        assert result is server

    # --- Method not found ---

    def test_unknown_request_returns_method_not_found(self) -> None:
        codec = MockCodec()
        req = RpcRequest(id=2, method="unknown")
        server, codec, write_buf = self._make_server(codec.encode(req))
        server.serve()

        resp = self._read_response(codec, write_buf)
        assert isinstance(resp, RpcErrorResponse)
        assert resp.code == METHOD_NOT_FOUND
        assert resp.id == 2

    def test_unknown_notification_is_silently_dropped(self) -> None:
        codec = MockCodec()
        notif = RpcNotification(method="unknown_event")
        server, codec, write_buf = self._make_server(codec.encode(notif))
        server.serve()

        # Absolutely no response.
        assert read_all_frames(write_buf) == []

    # --- Handler raises ---

    def test_handler_exception_returns_internal_error(self) -> None:
        codec = MockCodec()
        req = RpcRequest(id=3, method="boom")
        server, codec, write_buf = self._make_server(codec.encode(req))
        server.on_request("boom", lambda id, p: 1 / 0)  # ZeroDivisionError
        server.serve()

        resp = self._read_response(codec, write_buf)
        assert isinstance(resp, RpcErrorResponse)
        assert resp.code == INTERNAL_ERROR
        assert resp.id == 3

    def test_handler_system_exit_returns_internal_error(self) -> None:
        """Even SystemExit must be caught — the server must stay alive."""
        codec = MockCodec()
        req = RpcRequest(id=4, method="exit")
        server, codec, write_buf = self._make_server(codec.encode(req))

        def raise_system_exit(id: Any, p: Any) -> None:  # noqa: ANN401
            raise SystemExit(1)

        server.on_request("exit", raise_system_exit)
        server.serve()

        resp = self._read_response(codec, write_buf)
        assert isinstance(resp, RpcErrorResponse)
        assert resp.code == INTERNAL_ERROR

    def test_notification_handler_exception_is_silently_dropped(self) -> None:
        """A crashing notification handler must not produce any response."""
        codec = MockCodec()
        notif = RpcNotification(method="bad_notif")
        server, codec, write_buf = self._make_server(codec.encode(notif))
        server.on_notification("bad_notif", lambda p: 1 / 0)
        server.serve()

        assert read_all_frames(write_buf) == []

    # --- Decode errors ---

    def test_decode_error_sends_error_with_null_id(self) -> None:
        """When the codec cannot decode a frame, error response has id=None."""
        # We need a codec that fails, but we still need to be able to read
        # back the response.  Use a second codec instance for reading.
        write_codec = MockCodec()  # for reading back responses
        # This codec will fail on the first decode call.
        fail_codec = MockCodec(fail_decode=True, fail_code=PARSE_ERROR)

        read_buf = frames_to_buf(b"not a valid pickle frame at all")
        write_buf = io.BytesIO()
        framer = MockFramer(read_buf, write_buf)
        server: RpcServer = RpcServer(fail_codec, framer)
        server.serve()

        frames = read_all_frames(write_buf)
        assert len(frames) == 1
        resp = write_codec.decode(frames[0])
        assert isinstance(resp, RpcErrorResponse)
        assert resp.id is None
        assert resp.code == PARSE_ERROR

    def test_decode_error_invalid_request_code(self) -> None:
        """Decode errors can carry INVALID_REQUEST code too."""
        write_codec = MockCodec()
        fail_codec = MockCodec(fail_decode=True, fail_code=INVALID_REQUEST)

        read_buf = frames_to_buf(b"garbage")
        write_buf = io.BytesIO()
        framer = MockFramer(read_buf, write_buf)
        server: RpcServer = RpcServer(fail_codec, framer)
        server.serve()

        frames = read_all_frames(write_buf)
        resp = write_codec.decode(frames[0])
        assert isinstance(resp, RpcErrorResponse)
        assert resp.code == INVALID_REQUEST

    # --- Multiple messages ---

    def test_multiple_requests_all_dispatched(self) -> None:
        codec = MockCodec()
        reqs = [
            RpcRequest(id=i, method="echo", params={"n": i})
            for i in range(1, 4)
        ]
        read_buf = frames_to_buf(*[codec.encode(r) for r in reqs])
        write_buf = io.BytesIO()
        framer = MockFramer(read_buf, write_buf)
        server: RpcServer = RpcServer(codec, framer)
        server.on_request("echo", lambda id, p: p["n"])
        server.serve()

        responses = self._read_responses(codec, write_buf)
        assert len(responses) == 3
        for i, resp in enumerate(responses, start=1):
            assert isinstance(resp, RpcResponse)
            assert resp.id == i
            assert resp.result == i

    def test_responses_and_error_responses_are_ignored(self) -> None:
        """Servers silently ignore incoming response messages."""
        codec = MockCodec()
        resp_msg = RpcResponse(id=99, result="ignored")
        err_msg = RpcErrorResponse(code=-32601, message="x", id=98)
        read_buf = frames_to_buf(codec.encode(resp_msg), codec.encode(err_msg))
        write_buf = io.BytesIO()
        framer = MockFramer(read_buf, write_buf)
        server: RpcServer = RpcServer(codec, framer)
        server.serve()

        # No response should be written.
        assert read_all_frames(write_buf) == []

    # --- on_request overwrite ---

    def test_on_request_second_registration_replaces_first(self) -> None:
        codec = MockCodec()
        req = RpcRequest(id=1, method="greet")
        server, codec, write_buf = self._make_server(codec.encode(req))
        server.on_request("greet", lambda id, p: "first")
        server.on_request("greet", lambda id, p: "second")
        server.serve()

        resp = self._read_response(codec, write_buf)
        assert resp.result == "second"


# ===========================================================================
# 5. RpcClient
# ===========================================================================


class TestRpcClient:
    """Tests for RpcClient — request, notify, server-push, id management."""

    def _make_client_with_server_frames(
        self,
        *server_payloads: Any,
    ) -> tuple[RpcClient, MockCodec, io.BytesIO]:
        """Return a client whose read buffer contains pre-encoded server frames."""
        codec = MockCodec()
        # Encode server payloads and put them in the read buffer.
        read_buf = frames_to_buf(*[codec.encode(p) for p in server_payloads])
        write_buf = io.BytesIO()
        framer = MockFramer(read_buf, write_buf)
        client: RpcClient = RpcClient(codec, framer)
        return client, codec, write_buf

    def _read_sent_messages(self, codec: MockCodec, write_buf: io.BytesIO) -> list[Any]:
        """Decode all frames the client wrote to write_buf."""
        return [codec.decode(f) for f in read_all_frames(write_buf)]

    # --- request() happy path ---

    def test_request_sends_correct_message(self) -> None:
        """The client should encode an RpcRequest and send it."""
        codec = MockCodec()
        # Pre-load the read buffer with a matching response.
        response = RpcResponse(id=1, result=42)
        read_buf = frames_to_buf(codec.encode(response))
        write_buf = io.BytesIO()
        framer = MockFramer(read_buf, write_buf)
        client: RpcClient = RpcClient(codec, framer)

        result = client.request("add", {"a": 40, "b": 2})
        assert result == 42

        # Verify what was written.
        msgs = self._read_sent_messages(codec, write_buf)
        assert len(msgs) == 1
        sent = msgs[0]
        assert isinstance(sent, RpcRequest)
        assert sent.method == "add"
        assert sent.params == {"a": 40, "b": 2}
        assert sent.id == 1

    def test_request_returns_result_value(self) -> None:
        codec = MockCodec()
        response = RpcResponse(id=1, result={"answer": 99})
        read_buf = frames_to_buf(codec.encode(response))
        client: RpcClient = RpcClient(codec, MockFramer(read_buf, io.BytesIO()))
        assert client.request("foo") == {"answer": 99}

    def test_request_with_none_params(self) -> None:
        codec = MockCodec()
        response = RpcResponse(id=1, result="ok")
        read_buf = frames_to_buf(codec.encode(response))
        client: RpcClient = RpcClient(codec, MockFramer(read_buf, io.BytesIO()))
        assert client.request("ping") == "ok"

    # --- request() error path ---

    def test_request_raises_on_error_response(self) -> None:
        codec = MockCodec()
        err = RpcErrorResponse(code=METHOD_NOT_FOUND, message="no such method", id=1)
        read_buf = frames_to_buf(codec.encode(err))
        client: RpcClient = RpcClient(codec, MockFramer(read_buf, io.BytesIO()))

        with pytest.raises(RpcRemoteError) as exc_info:
            client.request("unknown")
        assert exc_info.value.error.code == METHOD_NOT_FOUND

    def test_request_raises_connection_error_on_eof(self) -> None:
        codec = MockCodec()
        client: RpcClient = RpcClient(codec, MockFramer(io.BytesIO(), io.BytesIO()))
        with pytest.raises(ConnectionError):
            client.request("anything")

    # --- notify() ---

    def test_notify_sends_notification_and_does_not_read(self) -> None:
        codec = MockCodec()
        write_buf = io.BytesIO()
        # The read buffer is empty — if notify() tried to read, it would fail.
        client: RpcClient = RpcClient(codec, MockFramer(io.BytesIO(), write_buf))
        client.notify("log", {"msg": "hello"})

        msgs = self._read_sent_messages(codec, write_buf)
        assert len(msgs) == 1
        sent = msgs[0]
        assert isinstance(sent, RpcNotification)
        assert sent.method == "log"
        assert sent.params == {"msg": "hello"}

    def test_notify_with_none_params(self) -> None:
        codec = MockCodec()
        write_buf = io.BytesIO()
        client: RpcClient = RpcClient(codec, MockFramer(io.BytesIO(), write_buf))
        client.notify("ping")

        msgs = self._read_sent_messages(codec, write_buf)
        assert isinstance(msgs[0], RpcNotification)
        assert msgs[0].method == "ping"
        assert msgs[0].params is None

    # --- Server-push notifications while waiting for response ---

    def test_server_push_notification_dispatched_while_waiting(self) -> None:
        """Notifications received during request() are dispatched to handlers."""
        received: list[Any] = []
        codec = MockCodec()

        push_notif = RpcNotification(method="ping", params={"ts": 42})
        response = RpcResponse(id=1, result="done")
        read_buf = frames_to_buf(codec.encode(push_notif), codec.encode(response))

        client: RpcClient = RpcClient(codec, MockFramer(read_buf, io.BytesIO()))
        client.on_notification("ping", lambda p: received.append(p))
        result = client.request("do_thing")

        assert result == "done"
        assert received == [{"ts": 42}]

    def test_server_push_without_handler_is_ignored(self) -> None:
        """Notifications with no registered handler are silently discarded."""
        codec = MockCodec()
        push = RpcNotification(method="unhandled", params=None)
        response = RpcResponse(id=1, result="ok")
        read_buf = frames_to_buf(codec.encode(push), codec.encode(response))

        client: RpcClient = RpcClient(codec, MockFramer(read_buf, io.BytesIO()))
        result = client.request("x")
        assert result == "ok"

    def test_notification_handler_exception_does_not_abort_wait(self) -> None:
        """A crashing notification handler must not break the request loop."""
        codec = MockCodec()
        push = RpcNotification(method="crash", params=None)
        response = RpcResponse(id=1, result="safe")
        read_buf = frames_to_buf(codec.encode(push), codec.encode(response))

        client: RpcClient = RpcClient(codec, MockFramer(read_buf, io.BytesIO()))
        client.on_notification("crash", lambda p: 1 / 0)  # will raise
        result = client.request("safe_method")
        assert result == "safe"

    # --- Response for different id is skipped ---

    def test_response_for_wrong_id_is_ignored(self) -> None:
        """Response with id != our request id must be skipped, not returned."""
        codec = MockCodec()
        wrong_resp = RpcResponse(id=99, result="wrong")
        right_resp = RpcResponse(id=1, result="right")
        read_buf = frames_to_buf(codec.encode(wrong_resp), codec.encode(right_resp))

        client: RpcClient = RpcClient(codec, MockFramer(read_buf, io.BytesIO()))
        result = client.request("foo")
        assert result == "right"

    # --- Monotonically increasing ids ---

    def test_request_ids_are_monotonically_increasing(self) -> None:
        codec = MockCodec()
        responses = [RpcResponse(id=i, result=i * 10) for i in range(1, 4)]
        read_buf = frames_to_buf(*[codec.encode(r) for r in responses])
        write_buf = io.BytesIO()
        client: RpcClient = RpcClient(codec, MockFramer(read_buf, write_buf))

        results = [client.request("get") for _ in range(3)]

        assert results == [10, 20, 30]

        sent = self._read_sent_messages(codec, write_buf)
        ids = [msg.id for msg in sent]
        assert ids == [1, 2, 3]

    def test_ids_start_at_one(self) -> None:
        codec = MockCodec()
        response = RpcResponse(id=1, result=None)
        read_buf = frames_to_buf(codec.encode(response))
        write_buf = io.BytesIO()
        client: RpcClient = RpcClient(codec, MockFramer(read_buf, write_buf))
        client.request("first")

        sent = self._read_sent_messages(codec, write_buf)
        assert sent[0].id == 1

    # --- on_notification chaining ---

    def test_on_notification_chaining(self) -> None:
        codec = MockCodec()
        client: RpcClient = RpcClient(codec, MockFramer(io.BytesIO(), io.BytesIO()))
        result = (
            client
            .on_notification("a", lambda p: None)
            .on_notification("b", lambda p: None)
        )
        assert result is client

    # --- RpcRemoteError attributes ---

    def test_remote_error_attributes(self) -> None:
        codec = MockCodec()
        # The client auto-assigns id=1 for its first request, so the error response
        # must carry id=1 to match — otherwise the client keeps reading and hits EOF.
        err = RpcErrorResponse(code=INTERNAL_ERROR, message="crash", id=1, data="tb")
        read_buf = frames_to_buf(codec.encode(err))
        client: RpcClient = RpcClient(codec, MockFramer(read_buf, io.BytesIO()))

        with pytest.raises(RpcRemoteError) as exc_info:
            client.request("boom")
        exc = exc_info.value
        assert exc.error.code == INTERNAL_ERROR
        assert exc.error.message == "crash"
        assert exc.error.data == "tb"
        assert "RpcRemoteError" in repr(exc)


# ===========================================================================
# 6. Integration — client sends request to server via shared BytesIO
# ===========================================================================


class TestIntegration:
    """End-to-end tests: client writes to a BytesIO that the server reads."""

    def test_client_server_round_trip(self) -> None:
        """Client request → server dispatch → response back to client."""
        # Pipe: client writes to client_to_server_buf, server reads from it.
        #       server writes to server_to_client_buf, client reads from it.
        codec = MockCodec()

        # We'll manually orchestrate a single request-response cycle.
        client_to_server_buf = io.BytesIO()

        # 1. Client writes a request.
        client_write_framer = MockFramer(io.BytesIO(), client_to_server_buf)
        client_write_framer.write_frame(
            codec.encode(RpcRequest(id=1, method="double", params={"n": 21}))
        )

        # 2. Server reads the request, dispatches, writes a response.
        client_to_server_buf.seek(0)
        server_to_client_buf = io.BytesIO()
        server_codec = MockCodec()
        server_framer = MockFramer(client_to_server_buf, server_to_client_buf)
        server: RpcServer = RpcServer(server_codec, server_framer)
        server.on_request("double", lambda id, p: p["n"] * 2)
        server.serve()

        # 3. Client reads the response.
        server_to_client_buf.seek(0)
        frames = read_all_frames(server_to_client_buf)
        assert len(frames) == 1
        resp = codec.decode(frames[0])
        assert isinstance(resp, RpcResponse)
        assert resp.id == 1
        assert resp.result == 42

    def test_full_client_object_round_trip(self) -> None:
        """Use RpcClient to send and RpcServer to respond, then verify."""
        # Step 1: have the client send a request.
        codec = MockCodec()
        pipe_c_to_s = io.BytesIO()
        client: RpcClient = RpcClient(
            codec, MockFramer(io.BytesIO(), pipe_c_to_s)
        )
        # We manually invoke just the write part of request().
        req = RpcRequest(id=1, method="greet", params={"name": "World"})
        codec2 = MockCodec()
        pipe_c_to_s.write(make_frame(codec2.encode(req)))

        # Step 2: server processes it.
        pipe_c_to_s.seek(0)
        pipe_s_to_c = io.BytesIO()
        server_codec = MockCodec()
        server: RpcServer = RpcServer(
            server_codec, MockFramer(pipe_c_to_s, pipe_s_to_c)
        )
        server.on_request("greet", lambda id, p: f"Hello, {p['name']}!")
        server.serve()

        # Step 3: read the response back.
        pipe_s_to_c.seek(0)
        frames = read_all_frames(pipe_s_to_c)
        resp = server_codec.decode(frames[0])
        assert resp.result == "Hello, World!"

    def test_multiple_mixed_messages(self) -> None:
        """Server handles a mix of requests and notifications in order."""
        codec = MockCodec()
        received_notifs: list[Any] = []

        req1 = RpcRequest(id=1, method="inc", params={"n": 10})
        notif = RpcNotification(method="event", params={"x": 5})
        req2 = RpcRequest(id=2, method="inc", params={"n": 20})

        read_buf = frames_to_buf(
            codec.encode(req1),
            codec.encode(notif),
            codec.encode(req2),
        )
        write_buf = io.BytesIO()
        server: RpcServer = RpcServer(codec, MockFramer(read_buf, write_buf))
        server.on_request("inc", lambda id, p: p["n"] + 1)
        server.on_notification("event", lambda p: received_notifs.append(p))
        server.serve()

        frames = read_all_frames(write_buf)
        # Only 2 responses (for req1 and req2); the notification has no response.
        assert len(frames) == 2
        resps = [codec.decode(f) for f in frames]
        assert resps[0].result == 11
        assert resps[1].result == 21
        assert received_notifs == [{"x": 5}]


# ===========================================================================
# 7. check_codec / check_framer guards
# ===========================================================================


class TestGuards:
    """Tests for check_codec() and check_framer() helper functions."""

    def test_check_codec_passes_for_valid_codec(self) -> None:
        assert check_codec(MockCodec()) is None

    def test_check_codec_fails_for_missing_encode(self) -> None:
        class NoEncode:
            def decode(self, data: bytes) -> Any:
                return None

        result = check_codec(NoEncode())
        assert result is not None
        assert "encode" in result

    def test_check_codec_fails_for_missing_decode(self) -> None:
        class NoDecode:
            def encode(self, msg: Any) -> bytes:
                return b""

        result = check_codec(NoDecode())
        assert result is not None
        assert "decode" in result

    def test_check_codec_fails_for_plain_object(self) -> None:
        assert check_codec(object()) is not None

    def test_check_framer_passes_for_valid_framer(self) -> None:
        framer = MockFramer(io.BytesIO(), io.BytesIO())
        assert check_framer(framer) is None

    def test_check_framer_fails_for_missing_read_frame(self) -> None:
        class NoRead:
            def write_frame(self, data: bytes) -> None:
                pass

        result = check_framer(NoRead())
        assert result is not None
        assert "read_frame" in result

    def test_check_framer_fails_for_missing_write_frame(self) -> None:
        class NoWrite:
            def read_frame(self) -> Optional[bytes]:
                return None

        result = check_framer(NoWrite())
        assert result is not None
        assert "write_frame" in result

    def test_check_framer_fails_for_plain_object(self) -> None:
        assert check_framer(object()) is not None


# ===========================================================================
# 8. __init__ public API surface
# ===========================================================================


class TestPublicApi:
    """Smoke tests to verify __init__.py re-exports everything expected."""

    def test_all_exports_present(self) -> None:
        expected = [
            "RpcRequest", "RpcResponse", "RpcErrorResponse", "RpcNotification",
            "RpcId", "RpcMessage",
            "RpcServer", "RpcClient", "RpcRemoteError",
            "RpcCodec", "RpcFramer",
            "PARSE_ERROR", "INVALID_REQUEST", "METHOD_NOT_FOUND",
            "INVALID_PARAMS", "INTERNAL_ERROR",
            "RpcDecodeError",
        ]
        for name in expected:
            assert hasattr(rpc, name), f"rpc.{name} is missing from public API"
