"""Comprehensive tests for the json_rpc package.

Test Strategy
-------------

Tests are organized into five groups:

1. **Message parsing** — ``parse_message`` and ``message_to_dict`` round-trips.
2. **MessageReader** — framing, back-to-back messages, EOF, errors.
3. **MessageWriter** — correct Content-Length, UTF-8 payload, CRLF separator.
4. **Server** — dispatch, unknown method, handler errors, notifications.
5. **Round-trip** — write then read back through a shared BytesIO buffer.

Each test is named ``test_<component>_<scenario>`` for clarity.
"""

from __future__ import annotations

import io
import json
from typing import Any

import pytest

from json_rpc import (
    INTERNAL_ERROR,
    INVALID_REQUEST,
    METHOD_NOT_FOUND,
    PARSE_ERROR,
    JsonRpcError,
    MessageReader,
    MessageWriter,
    Notification,
    Request,
    Response,
    ResponseError,
    Server,
    message_to_dict,
    parse_message,
)


# ===========================================================================
# Helpers
# ===========================================================================


def make_framed(payload: str) -> bytes:
    """Build a Content-Length-framed message from a JSON string."""
    encoded = payload.encode("utf-8")
    header = f"Content-Length: {len(encoded)}\r\n\r\n"
    return header.encode("ascii") + encoded


def make_reader(payload: str) -> MessageReader:
    """Build a MessageReader backed by a single framed message string."""
    return MessageReader(io.BytesIO(make_framed(payload)))


def make_reader_bytes(data: bytes) -> MessageReader:
    """Build a MessageReader backed by raw bytes."""
    return MessageReader(io.BytesIO(data))


# ===========================================================================
# 1. parse_message / message_to_dict
# ===========================================================================


class TestParseMessage:
    """Tests for parse_message() — the JSON-to-typed-message conversion."""

    def test_parse_request_with_params(self) -> None:
        raw = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"x":1}}'
        msg = parse_message(raw)
        assert isinstance(msg, Request)
        assert msg.id == 1
        assert msg.method == "initialize"
        assert msg.params == {"x": 1}

    def test_parse_request_without_params(self) -> None:
        raw = '{"jsonrpc":"2.0","id":"abc","method":"shutdown"}'
        msg = parse_message(raw)
        assert isinstance(msg, Request)
        assert msg.id == "abc"
        assert msg.method == "shutdown"
        assert msg.params is None

    def test_parse_notification_with_params(self) -> None:
        raw = '{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":"file:///a.bf"}}'
        msg = parse_message(raw)
        assert isinstance(msg, Notification)
        assert msg.method == "textDocument/didOpen"
        assert msg.params == {"uri": "file:///a.bf"}

    def test_parse_notification_without_params(self) -> None:
        raw = '{"jsonrpc":"2.0","method":"exit"}'
        msg = parse_message(raw)
        assert isinstance(msg, Notification)
        assert msg.method == "exit"
        assert msg.params is None

    def test_parse_response_success(self) -> None:
        raw = '{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}'
        msg = parse_message(raw)
        assert isinstance(msg, Response)
        assert msg.id == 1
        assert msg.result == {"capabilities": {}}
        assert msg.error is None

    def test_parse_response_null_result(self) -> None:
        raw = '{"jsonrpc":"2.0","id":2,"result":null}'
        msg = parse_message(raw)
        assert isinstance(msg, Response)
        assert msg.id == 2
        assert msg.result is None
        assert msg.error is None

    def test_parse_response_error(self) -> None:
        raw = '{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}'
        msg = parse_message(raw)
        assert isinstance(msg, Response)
        assert msg.id == 1
        assert msg.error is not None
        assert msg.error.code == -32601
        assert msg.error.message == "Method not found"
        assert msg.error.data is None

    def test_parse_response_error_with_data(self) -> None:
        raw = '{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid","data":"details"}}'
        msg = parse_message(raw)
        assert isinstance(msg, Response)
        assert msg.error is not None
        assert msg.error.data == "details"

    def test_parse_invalid_json_raises_parse_error(self) -> None:
        with pytest.raises(JsonRpcError) as exc_info:
            parse_message("{not valid json}")
        assert exc_info.value.code == PARSE_ERROR

    def test_parse_json_array_raises_invalid_request(self) -> None:
        # A JSON array is not a valid JSON-RPC message object.
        with pytest.raises(JsonRpcError) as exc_info:
            parse_message('[{"jsonrpc":"2.0"}]')
        assert exc_info.value.code == INVALID_REQUEST

    def test_parse_missing_jsonrpc_field_raises_invalid_request(self) -> None:
        with pytest.raises(JsonRpcError) as exc_info:
            parse_message('{"id":1,"method":"foo"}')
        assert exc_info.value.code == INVALID_REQUEST

    def test_parse_wrong_jsonrpc_version_raises_invalid_request(self) -> None:
        with pytest.raises(JsonRpcError) as exc_info:
            parse_message('{"jsonrpc":"1.0","id":1,"method":"foo"}')
        assert exc_info.value.code == INVALID_REQUEST

    def test_parse_missing_method_raises_invalid_request(self) -> None:
        with pytest.raises(JsonRpcError) as exc_info:
            parse_message('{"jsonrpc":"2.0","id":1}')
        assert exc_info.value.code == INVALID_REQUEST

    def test_parse_non_string_method_raises_invalid_request(self) -> None:
        with pytest.raises(JsonRpcError) as exc_info:
            parse_message('{"jsonrpc":"2.0","id":1,"method":42}')
        assert exc_info.value.code == INVALID_REQUEST

    def test_parse_null_id_raises_invalid_request(self) -> None:
        # "id" present but null is invalid for a Request (id must be str or int)
        with pytest.raises(JsonRpcError) as exc_info:
            parse_message('{"jsonrpc":"2.0","id":null,"method":"foo"}')
        assert exc_info.value.code == INVALID_REQUEST

    def test_parse_response_bad_error_object_raises(self) -> None:
        raw = '{"jsonrpc":"2.0","id":1,"error":"not an object"}'
        with pytest.raises(JsonRpcError) as exc_info:
            parse_message(raw)
        assert exc_info.value.code == INVALID_REQUEST

    def test_parse_response_error_non_int_code_raises(self) -> None:
        raw = '{"jsonrpc":"2.0","id":1,"error":{"code":"oops","message":"bad"}}'
        with pytest.raises(JsonRpcError) as exc_info:
            parse_message(raw)
        assert exc_info.value.code == INVALID_REQUEST


class TestMessageToDict:
    """Tests for message_to_dict() — the typed-message-to-JSON-dict conversion."""

    def test_request_with_params(self) -> None:
        msg = Request(id=1, method="foo", params={"a": 1})
        d = message_to_dict(msg)
        assert d == {"jsonrpc": "2.0", "id": 1, "method": "foo", "params": {"a": 1}}

    def test_request_without_params(self) -> None:
        msg = Request(id=2, method="bar")
        d = message_to_dict(msg)
        assert "params" not in d
        assert d["id"] == 2

    def test_response_success(self) -> None:
        msg = Response(id=1, result={"ok": True})
        d = message_to_dict(msg)
        assert d["result"] == {"ok": True}
        assert "error" not in d

    def test_response_error(self) -> None:
        err = ResponseError(code=-32601, message="Not found")
        msg = Response(id=1, error=err)
        d = message_to_dict(msg)
        assert "result" not in d
        assert d["error"]["code"] == -32601
        assert d["error"]["message"] == "Not found"
        assert "data" not in d["error"]

    def test_response_error_with_data(self) -> None:
        err = ResponseError(code=-32600, message="Bad", data={"detail": "x"})
        msg = Response(id=1, error=err)
        d = message_to_dict(msg)
        assert d["error"]["data"] == {"detail": "x"}

    def test_notification_with_params(self) -> None:
        msg = Notification(method="evt", params={"x": 1})
        d = message_to_dict(msg)
        assert d == {"jsonrpc": "2.0", "method": "evt", "params": {"x": 1}}

    def test_notification_without_params(self) -> None:
        msg = Notification(method="exit")
        d = message_to_dict(msg)
        assert "params" not in d

    def test_unknown_type_raises_typeerror(self) -> None:
        with pytest.raises(TypeError):
            message_to_dict("not a message")  # type: ignore[arg-type]


# ===========================================================================
# 2. MessageReader
# ===========================================================================


class TestMessageReader:
    """Tests for MessageReader — reading Content-Length-framed messages."""

    def test_read_single_request(self) -> None:
        raw = '{"jsonrpc":"2.0","id":1,"method":"foo"}'
        reader = make_reader(raw)
        msg = reader.read_message()
        assert isinstance(msg, Request)
        assert msg.id == 1
        assert msg.method == "foo"

    def test_read_returns_none_on_eof(self) -> None:
        reader = MessageReader(io.BytesIO(b""))
        msg = reader.read_message()
        assert msg is None

    def test_read_raw_returns_json_string(self) -> None:
        raw = '{"jsonrpc":"2.0","method":"ping"}'
        reader = make_reader(raw)
        result = reader.read_raw()
        assert result == raw

    def test_read_raw_returns_none_on_eof(self) -> None:
        reader = MessageReader(io.BytesIO(b""))
        result = reader.read_raw()
        assert result is None

    def test_read_back_to_back_messages(self) -> None:
        """Two consecutive framed messages on the same stream."""
        msg1 = '{"jsonrpc":"2.0","id":1,"method":"foo"}'
        msg2 = '{"jsonrpc":"2.0","method":"bar"}'
        data = make_framed(msg1) + make_framed(msg2)
        reader = MessageReader(io.BytesIO(data))

        first = reader.read_message()
        assert isinstance(first, Request)
        assert first.method == "foo"

        second = reader.read_message()
        assert isinstance(second, Notification)
        assert second.method == "bar"

        # After both messages, EOF should return None
        third = reader.read_message()
        assert third is None

    def test_read_malformed_json_raises_parse_error(self) -> None:
        """Content-Length is valid but payload is not JSON."""
        payload = b"not-json!!!"
        header = f"Content-Length: {len(payload)}\r\n\r\n".encode()
        reader = MessageReader(io.BytesIO(header + payload))
        with pytest.raises(JsonRpcError) as exc_info:
            reader.read_message()
        assert exc_info.value.code == PARSE_ERROR

    def test_read_valid_json_not_a_message_raises_invalid_request(self) -> None:
        """Valid JSON but not a JSON-RPC object."""
        raw = '"just a string"'
        reader = make_reader(raw)
        with pytest.raises(JsonRpcError) as exc_info:
            reader.read_message()
        assert exc_info.value.code == INVALID_REQUEST

    def test_read_ignores_content_type_header(self) -> None:
        """Extra headers (like Content-Type) should be accepted and ignored."""
        payload = '{"jsonrpc":"2.0","method":"ping"}'.encode("utf-8")
        header = (
            f"Content-Length: {len(payload)}\r\n"
            "Content-Type: application/vscode-jsonrpc; charset=utf-8\r\n"
            "\r\n"
        ).encode("ascii")
        reader = MessageReader(io.BytesIO(header + payload))
        msg = reader.read_message()
        assert isinstance(msg, Notification)
        assert msg.method == "ping"

    def test_read_missing_content_length_header_raises(self) -> None:
        """A framed block with no Content-Length header should raise."""
        data = b"Content-Type: text/plain\r\n\r\nhello"
        reader = make_reader_bytes(data)
        with pytest.raises(JsonRpcError) as exc_info:
            reader.read_message()
        assert exc_info.value.code == PARSE_ERROR

    def test_read_truncated_payload_raises(self) -> None:
        """Content-Length says 100 but only 5 bytes follow."""
        header = b"Content-Length: 100\r\n\r\nhello"
        reader = MessageReader(io.BytesIO(header))
        with pytest.raises(JsonRpcError) as exc_info:
            reader.read_message()
        assert exc_info.value.code == PARSE_ERROR

    def test_read_unicode_payload(self) -> None:
        """Payload containing multi-byte UTF-8 characters."""
        # The Japanese characters are multi-byte in UTF-8.
        payload = '{"jsonrpc":"2.0","method":"test","params":{"msg":"日本語"}}'
        reader = make_reader(payload)
        msg = reader.read_message()
        assert isinstance(msg, Notification)
        assert msg.params["msg"] == "日本語"

    def test_read_notification_message(self) -> None:
        raw = '{"jsonrpc":"2.0","method":"textDocument/didClose","params":{"uri":"f"}}'
        reader = make_reader(raw)
        msg = reader.read_message()
        assert isinstance(msg, Notification)
        assert msg.method == "textDocument/didClose"

    def test_read_response_message(self) -> None:
        raw = '{"jsonrpc":"2.0","id":5,"result":42}'
        reader = make_reader(raw)
        msg = reader.read_message()
        assert isinstance(msg, Response)
        assert msg.id == 5
        assert msg.result == 42


# ===========================================================================
# 3. MessageWriter
# ===========================================================================


class TestMessageWriter:
    """Tests for MessageWriter — writing Content-Length-framed messages."""

    def test_write_produces_correct_content_length(self) -> None:
        buf = io.BytesIO()
        writer = MessageWriter(buf)
        msg = Request(id=1, method="foo")
        writer.write_message(msg)

        buf.seek(0)
        data = buf.read()
        # Find the Content-Length value
        header_end = data.index(b"\r\n\r\n")
        header_block = data[:header_end].decode("ascii")
        payload = data[header_end + 4:]

        cl_line = next(
            line for line in header_block.split("\r\n")
            if line.lower().startswith("content-length:")
        )
        declared_length = int(cl_line.split(":")[1].strip())
        assert declared_length == len(payload)

    def test_write_produces_crlf_separator(self) -> None:
        buf = io.BytesIO()
        writer = MessageWriter(buf)
        writer.write_raw('{"jsonrpc":"2.0","method":"ping"}')
        buf.seek(0)
        data = buf.read()
        # The separator between headers and payload must be \r\n\r\n
        assert b"\r\n\r\n" in data

    def test_write_payload_is_valid_json(self) -> None:
        buf = io.BytesIO()
        writer = MessageWriter(buf)
        msg = Response(id=1, result={"status": "ok"})
        writer.write_message(msg)

        buf.seek(0)
        data = buf.read()
        header_end = data.index(b"\r\n\r\n")
        payload = data[header_end + 4:].decode("utf-8")
        parsed = json.loads(payload)
        assert parsed["jsonrpc"] == "2.0"
        assert parsed["id"] == 1
        assert parsed["result"] == {"status": "ok"}

    def test_write_utf8_payload_byte_count(self) -> None:
        """Content-Length must be byte count, not character count."""
        buf = io.BytesIO()
        writer = MessageWriter(buf)
        # The emoji 🎸 is 4 bytes in UTF-8 but 1 character.
        writer.write_raw('{"jsonrpc":"2.0","method":"🎸"}')

        buf.seek(0)
        data = buf.read()
        header_end = data.index(b"\r\n\r\n")
        header_block = data[:header_end].decode("ascii")
        payload_bytes = data[header_end + 4:]

        cl_line = next(
            line for line in header_block.split("\r\n")
            if line.lower().startswith("content-length:")
        )
        declared_length = int(cl_line.split(":")[1].strip())
        assert declared_length == len(payload_bytes)

    def test_write_notification(self) -> None:
        buf = io.BytesIO()
        writer = MessageWriter(buf)
        msg = Notification(method="exit")
        writer.write_message(msg)

        buf.seek(0)
        data = buf.read()
        header_end = data.index(b"\r\n\r\n")
        payload = json.loads(data[header_end + 4:].decode("utf-8"))
        assert payload["method"] == "exit"
        assert "id" not in payload

    def test_write_error_response(self) -> None:
        buf = io.BytesIO()
        writer = MessageWriter(buf)
        err = ResponseError(code=-32601, message="Method not found")
        msg = Response(id=1, error=err)
        writer.write_message(msg)

        buf.seek(0)
        data = buf.read()
        header_end = data.index(b"\r\n\r\n")
        payload = json.loads(data[header_end + 4:].decode("utf-8"))
        assert payload["error"]["code"] == -32601
        assert "result" not in payload


# ===========================================================================
# 4. Server
# ===========================================================================


def _make_server_with_messages(*payloads: str) -> tuple[Server, io.BytesIO]:
    """Build a Server reading from the given framed payloads and writing to a BytesIO."""
    data = b"".join(make_framed(p) for p in payloads)
    in_stream = io.BytesIO(data)
    out_stream = io.BytesIO()
    server = Server(in_stream, out_stream)
    return server, out_stream


def _read_response(buf: io.BytesIO) -> dict[str, Any]:
    """Read and parse the first JSON-RPC response from a BytesIO buffer."""
    buf.seek(0)
    reader = MessageReader(buf)
    msg = reader.read_message()
    assert msg is not None
    return message_to_dict(msg)  # type: ignore[arg-type]


class TestServer:
    """Tests for the Server dispatch loop."""

    def test_server_dispatches_request_to_handler(self) -> None:
        req = '{"jsonrpc":"2.0","id":1,"method":"add","params":{"a":1,"b":2}}'
        server, out = _make_server_with_messages(req)
        server.on_request("add", lambda id, params: params["a"] + params["b"])
        server.serve()

        response = _read_response(out)
        assert response["id"] == 1
        assert response["result"] == 3
        assert "error" not in response

    def test_server_sends_method_not_found_for_unknown_request(self) -> None:
        req = '{"jsonrpc":"2.0","id":1,"method":"unknown"}'
        server, out = _make_server_with_messages(req)
        server.serve()

        response = _read_response(out)
        assert response["id"] == 1
        assert response["error"]["code"] == METHOD_NOT_FOUND

    def test_server_dispatches_notification_no_response(self) -> None:
        notif = '{"jsonrpc":"2.0","method":"ping"}'
        called: list[bool] = []
        server, out = _make_server_with_messages(notif)
        server.on_notification("ping", lambda params: called.append(True))
        server.serve()

        assert called == [True]
        # No response should have been written
        out.seek(0)
        assert out.read() == b""

    def test_server_ignores_unknown_notification_silently(self) -> None:
        notif = '{"jsonrpc":"2.0","method":"unknown_event"}'
        server, out = _make_server_with_messages(notif)
        server.serve()

        out.seek(0)
        assert out.read() == b""

    def test_server_sends_error_when_handler_returns_response_error(self) -> None:
        req = '{"jsonrpc":"2.0","id":1,"method":"fail"}'
        server, out = _make_server_with_messages(req)
        server.on_request(
            "fail",
            lambda id, params: ResponseError(code=-32602, message="Bad params"),
        )
        server.serve()

        response = _read_response(out)
        assert response["id"] == 1
        assert response["error"]["code"] == -32602
        assert response["error"]["message"] == "Bad params"

    def test_server_handler_exception_returns_internal_error(self) -> None:
        req = '{"jsonrpc":"2.0","id":1,"method":"boom"}'
        server, out = _make_server_with_messages(req)

        def handler(id: int, params: object) -> None:
            raise RuntimeError("something went wrong")

        server.on_request("boom", handler)
        server.serve()

        response = _read_response(out)
        assert response["id"] == 1
        assert response["error"]["code"] == INTERNAL_ERROR

    def test_server_chained_registration(self) -> None:
        """on_request / on_notification return self for chaining."""
        req = '{"jsonrpc":"2.0","id":1,"method":"ping"}'
        server, out = _make_server_with_messages(req)
        (
            server
            .on_request("ping", lambda id, params: "pong")
            .on_notification("exit", lambda params: None)
        )
        server.serve()

        response = _read_response(out)
        assert response["result"] == "pong"

    def test_server_handles_multiple_requests_in_sequence(self) -> None:
        req1 = '{"jsonrpc":"2.0","id":1,"method":"echo","params":"hello"}'
        req2 = '{"jsonrpc":"2.0","id":2,"method":"echo","params":"world"}'
        server, out = _make_server_with_messages(req1, req2)
        server.on_request("echo", lambda id, params: params)
        server.serve()

        out.seek(0)
        reader = MessageReader(out)
        r1 = reader.read_message()
        r2 = reader.read_message()
        assert isinstance(r1, Response)
        assert isinstance(r2, Response)
        assert r1.result == "hello"
        assert r2.result == "world"

    def test_server_null_result_is_valid(self) -> None:
        req = '{"jsonrpc":"2.0","id":1,"method":"shutdown"}'
        server, out = _make_server_with_messages(req)
        server.on_request("shutdown", lambda id, params: None)
        server.serve()

        response = _read_response(out)
        assert response["id"] == 1
        assert response["result"] is None

    def test_server_sends_error_response_on_parse_error(self) -> None:
        """Malformed framing should produce a -32700 error response."""
        # Craft a message where the payload is not valid JSON.
        bad_payload = b"NOT JSON"
        header = f"Content-Length: {len(bad_payload)}\r\n\r\n".encode()
        in_stream = io.BytesIO(header + bad_payload)
        out_stream = io.BytesIO()
        server = Server(in_stream, out_stream)
        server.serve()

        response = _read_response(out_stream)
        assert response["error"]["code"] == PARSE_ERROR


# ===========================================================================
# 5. Round-trip tests
# ===========================================================================


class TestRoundTrip:
    """Write a message, read it back, verify it survived the wire intact."""

    def test_roundtrip_request(self) -> None:
        buf = io.BytesIO()
        writer = MessageWriter(buf)
        original = Request(id=42, method="textDocument/hover", params={"line": 5})
        writer.write_message(original)

        buf.seek(0)
        reader = MessageReader(buf)
        recovered = reader.read_message()

        assert isinstance(recovered, Request)
        assert recovered.id == 42
        assert recovered.method == "textDocument/hover"
        assert recovered.params == {"line": 5}

    def test_roundtrip_notification(self) -> None:
        buf = io.BytesIO()
        writer = MessageWriter(buf)
        original = Notification(method="initialized", params={})
        writer.write_message(original)

        buf.seek(0)
        reader = MessageReader(buf)
        recovered = reader.read_message()

        assert isinstance(recovered, Notification)
        assert recovered.method == "initialized"
        assert recovered.params == {}

    def test_roundtrip_response_success(self) -> None:
        buf = io.BytesIO()
        writer = MessageWriter(buf)
        original = Response(id=1, result={"capabilities": {"hoverProvider": True}})
        writer.write_message(original)

        buf.seek(0)
        reader = MessageReader(buf)
        recovered = reader.read_message()

        assert isinstance(recovered, Response)
        assert recovered.id == 1
        assert recovered.result == {"capabilities": {"hoverProvider": True}}
        assert recovered.error is None

    def test_roundtrip_response_error(self) -> None:
        buf = io.BytesIO()
        writer = MessageWriter(buf)
        err = ResponseError(code=-32601, message="Method not found", data="details")
        original = Response(id=3, error=err)
        writer.write_message(original)

        buf.seek(0)
        reader = MessageReader(buf)
        recovered = reader.read_message()

        assert isinstance(recovered, Response)
        assert recovered.id == 3
        assert recovered.error is not None
        assert recovered.error.code == -32601
        assert recovered.error.message == "Method not found"
        assert recovered.error.data == "details"
        assert recovered.result is None

    def test_roundtrip_unicode_strings(self) -> None:
        buf = io.BytesIO()
        writer = MessageWriter(buf)
        original = Notification(method="test", params={"text": "日本語 🌸"})
        writer.write_message(original)

        buf.seek(0)
        reader = MessageReader(buf)
        recovered = reader.read_message()

        assert isinstance(recovered, Notification)
        assert recovered.params["text"] == "日本語 🌸"
