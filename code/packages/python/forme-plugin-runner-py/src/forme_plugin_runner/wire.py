"""Bounded Content-Length framing and Forme wire-value envelopes."""

from __future__ import annotations

import base64
import binascii
import json
import math
import re
from typing import Any

TAG = "$forme"
_CANONICAL_LENGTH = re.compile(rb"(?:0|[1-9][0-9]*)\Z")


class ProtocolError(Exception):
    """A malformed or out-of-bounds FM02 peer."""


def _canonical_base64(value: str) -> bool:
    try:
        decoded = base64.b64decode(value, validate=True)
    except (binascii.Error, ValueError):
        return False
    return base64.b64encode(decoded).decode("ascii") == value


def encode_wire_value(value: Any) -> Any:
    """Translate bytes and reserved-looking dictionaries into wire envelopes."""
    snapshot, _size = _snapshot_wire_value(value, None)
    return snapshot


def decode_wire_value(value: Any) -> Any:
    """Decode strict binary and escaped-object envelopes recursively."""

    def visit(entry: Any) -> Any:
        if isinstance(entry, list):
            return [visit(child) for child in entry]
        if not isinstance(entry, dict):
            return entry
        if entry.get(TAG) == "bytes" and set(entry) == {TAG, "base64"}:
            encoded = entry["base64"]
            if not isinstance(encoded, str) or not _canonical_base64(encoded):
                raise ProtocolError("wire bytes envelope is not canonical")
            return base64.b64decode(encoded)
        if entry.get(TAG) == "escaped-object" and set(entry) == {TAG, "entries"}:
            pairs = entry["entries"]
            if not isinstance(pairs, list):
                raise ProtocolError("invalid escaped-object wire envelope")
            result: dict[str, Any] = {}
            for pair in pairs:
                if (
                    not isinstance(pair, list)
                    or len(pair) != 2
                    or not isinstance(pair[0], str)
                    or pair[0] in result
                ):
                    raise ProtocolError("invalid escaped-object wire envelope")
                result[pair[0]] = visit(pair[1])
            return result
        return {key: visit(child) for key, child in entry.items()}

    return visit(value)


def _json_string_bytes(value: str) -> int:
    total = 2
    for character in value:
        codepoint = ord(character)
        if character in {'"', "\\", "\b", "\f", "\n", "\r", "\t"}:
            total += 2
        elif codepoint < 0x20:
            total += 6
        elif 0xD800 <= codepoint <= 0xDFFF:
            raise TypeError("surrogate code points are not valid UTF-8")
        elif codepoint < 0x80:
            total += 1
        elif codepoint < 0x800:
            total += 2
        elif codepoint < 0x10000:
            total += 3
        else:
            total += 4
    return total


def wire_value_size(value: Any, limit: int) -> int:
    """Build one bounded private snapshot and return its safe size bound."""

    _snapshot, size = _snapshot_wire_value(value, limit)
    return size


def _snapshot_wire_value(value: Any, limit: int | None) -> tuple[Any, int]:
    """Snapshot one traversal, enforcing the bound before every amplified copy."""

    total = 0
    seen: set[int] = set()

    def add(amount: int) -> None:
        nonlocal total
        total += amount
        if limit is not None and total > limit:
            raise ProtocolError("wire payload exceeds configured bound")

    def visit(entry: Any) -> Any:
        if isinstance(entry, bytes | bytearray | memoryview):
            view = memoryview(entry)
            try:
                encoded_bytes = 4 * ((view.nbytes + 2) // 3)
                add(len('{"$forme":"bytes","base64":""}') + encoded_bytes)
                bytes_snapshot = view.tobytes()
            finally:
                view.release()
            return {TAG: "bytes", "base64": base64.b64encode(bytes_snapshot).decode("ascii")}
        if entry is None:
            add(4)
            return None
        if isinstance(entry, bool):
            add(4 if entry else 5)
            return entry
        if isinstance(entry, str):
            string_snapshot = str(entry)
            add(_json_string_bytes(string_snapshot))
            return string_snapshot
        if isinstance(entry, int):
            integer_snapshot = int(entry)
            magnitude = abs(integer_snapshot)
            digits = 1 if magnitude == 0 else (magnitude.bit_length() * 30_103 + 99_999) // 100_000
            add(digits + (1 if integer_snapshot < 0 else 0))
            return integer_snapshot
        if isinstance(entry, float):
            float_snapshot = float(entry)
            if not math.isfinite(float_snapshot):
                raise TypeError("non-finite floats are not JSON serializable")
            add(len(repr(float_snapshot)))
            return float_snapshot
        identity = id(entry)
        if identity in seen:
            raise TypeError("circular JSON value")
        seen.add(identity)
        try:
            if isinstance(entry, list | tuple):
                add(2)
                snapshot_list: list[Any] = []
                for index, child in enumerate(entry):
                    if index:
                        add(1)
                    snapshot_list.append(visit(child))
                return snapshot_list
            if isinstance(entry, dict):
                normal_structure = 2
                escaped_structure = len('{"$forme":"escaped-object","entries":[') + 2
                add(normal_structure)
                pairs: list[list[Any]] = []
                keys: set[str] = set()
                for index, (raw_key, child) in enumerate(entry.items()):
                    if not isinstance(raw_key, str):
                        raise TypeError("JSON object keys must be strings")
                    key = str(raw_key)
                    if key in keys:
                        raise TypeError("JSON object keys must be unique")
                    keys.add(key)
                    key_bytes = _json_string_bytes(key)
                    normal_increment = (1 if index else 0) + key_bytes + 1
                    escaped_increment = (1 if index else 0) + key_bytes + 3
                    normal_structure += normal_increment
                    escaped_structure += escaped_increment
                    add(normal_increment)
                    pairs.append([key, visit(child)])
                if TAG in keys:
                    add(escaped_structure - normal_structure)
                    return {TAG: "escaped-object", "entries": pairs}
                return {key: child for key, child in pairs}
            raise TypeError(f"unsupported wire value: {type(entry).__name__}")
        finally:
            seen.remove(identity)

    return visit(value), total


def encode_frame(message: dict[str, Any], max_frame_bytes: int) -> bytes:
    """Encode one bounded canonical JSON-RPC frame."""

    try:
        snapshot, _size = _snapshot_wire_value(message, max_frame_bytes)
        payload = json.dumps(
            snapshot,
            ensure_ascii=False,
            allow_nan=False,
            separators=(",", ":"),
        ).encode("utf-8")
    except (TypeError, ValueError) as error:
        raise ProtocolError("wire payload is not JSON serializable") from error
    if len(payload) > max_frame_bytes:
        raise ProtocolError("wire payload exceeds configured bound")
    return f"Content-Length: {len(payload)}\r\n\r\n".encode("ascii") + payload


class FrameDecoder:
    """Incremental bounded decoder for the FM02 subprocess transport."""

    def __init__(self, max_frame_bytes: int, max_header_bytes: int) -> None:
        self._max_frame_bytes = max_frame_bytes
        self._max_header_bytes = max_header_bytes
        self._buffer = bytearray()
        self._expected: int | None = None

    def push(self, chunk: bytes) -> list[dict[str, Any]]:
        self._buffer.extend(chunk)
        messages: list[dict[str, Any]] = []
        while True:
            if self._expected is None:
                end = self._buffer.find(b"\r\n\r\n")
                if end < 0:
                    if len(self._buffer) > self._max_header_bytes:
                        raise ProtocolError("wire header exceeds configured bound")
                    break
                if end > self._max_header_bytes:
                    raise ProtocolError("wire header exceeds configured bound")
                header = bytes(self._buffer[:end])
                del self._buffer[: end + 4]
                lengths: list[bytes] = []
                for line in header.split(b"\r\n"):
                    key, separator, value = line.partition(b":")
                    if separator and key.lower() == b"content-length":
                        lengths.append(value.strip(b" \t"))
                if len(lengths) != 1 or _CANONICAL_LENGTH.fullmatch(lengths[0]) is None:
                    raise ProtocolError("frame requires exactly one canonical Content-Length")
                self._expected = int(lengths[0])
                if self._expected > self._max_frame_bytes:
                    raise ProtocolError("wire payload exceeds configured bound")
            if len(self._buffer) < self._expected:
                break
            payload = bytes(self._buffer[: self._expected])
            del self._buffer[: self._expected]
            self._expected = None
            try:
                decoded = json.loads(
                    payload.decode("utf-8"),
                    parse_constant=lambda value: (_ for _ in ()).throw(
                        ValueError(f"invalid JSON constant {value}")
                    ),
                )
                decoded = decode_wire_value(decoded)
            except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
                raise ProtocolError("wire payload is not valid JSON") from error
            if not isinstance(decoded, dict):
                raise ProtocolError("JSON-RPC message must be an object")
            messages.append(decoded)
        return messages

    def finish(self) -> None:
        if self._buffer or self._expected is not None:
            raise ProtocolError("wire ended in the middle of a frame")
