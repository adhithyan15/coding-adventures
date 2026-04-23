"""HTTP/1 request and response head parsing.

This module is deliberately narrower than a full browser networking stack. It
parses the textual HTTP/1 head and tells callers how the body bytes should be
consumed next.
"""

from __future__ import annotations

from dataclasses import dataclass

from http_core import BodyKind, Header, HttpVersion, RequestHead, ResponseHead

__all__ = [
    "Http1ParseError",
    "ParsedRequestHead",
    "ParsedResponseHead",
    "parse_request_head",
    "parse_response_head",
]

__version__ = "0.1.0"


class Http1ParseError(ValueError):
    """Raised when the input does not contain a valid HTTP/1 head."""


@dataclass(frozen=True, slots=True)
class ParsedRequestHead:
    head: RequestHead
    body_offset: int
    body_kind: BodyKind


@dataclass(frozen=True, slots=True)
class ParsedResponseHead:
    head: ResponseHead
    body_offset: int
    body_kind: BodyKind


def parse_request_head(input_bytes: bytes) -> ParsedRequestHead:
    lines, body_offset = _split_head_lines(input_bytes)
    if not lines:
        raise Http1ParseError("invalid HTTP/1 start line")

    parts = lines[0].split()
    if len(parts) != 3:
        raise Http1ParseError(f"invalid HTTP/1 start line: {lines[0]}")

    method, target, version_text = parts
    try:
        version = HttpVersion.parse(version_text)
    except ValueError as error:
        raise Http1ParseError(str(error)) from error

    headers = _parse_headers(lines[1:])
    body_kind = _request_body_kind(headers)
    return ParsedRequestHead(
        head=RequestHead(method=method, target=target, version=version, headers=headers),
        body_offset=body_offset,
        body_kind=body_kind,
    )


def parse_response_head(input_bytes: bytes) -> ParsedResponseHead:
    lines, body_offset = _split_head_lines(input_bytes)
    if not lines:
        raise Http1ParseError("invalid HTTP/1 status line")

    parts = lines[0].split()
    if len(parts) < 2:
        raise Http1ParseError(f"invalid HTTP/1 status line: {lines[0]}")

    version_text, status_text, *reason_parts = parts
    try:
        version = HttpVersion.parse(version_text)
    except ValueError as error:
        raise Http1ParseError(str(error)) from error

    if not status_text.isdigit():
        raise Http1ParseError(f"invalid HTTP status: {status_text}")

    headers = _parse_headers(lines[1:])
    body_kind = _response_body_kind(int(status_text), headers)
    return ParsedResponseHead(
        head=ResponseHead(
            version=version,
            status=int(status_text),
            reason=" ".join(reason_parts),
            headers=headers,
        ),
        body_offset=body_offset,
        body_kind=body_kind,
    )


def _split_head_lines(input_bytes: bytes) -> tuple[list[str], int]:
    index = 0
    while input_bytes[index : index + 2] == b"\r\n" or input_bytes[index : index + 1] == b"\n":
        index += 2 if input_bytes[index : index + 2] == b"\r\n" else 1

    lines: list[str] = []
    while True:
        if index >= len(input_bytes):
            raise Http1ParseError("incomplete HTTP/1 head")

        line_end = input_bytes.find(b"\n", index)
        if line_end == -1:
            raise Http1ParseError("incomplete HTTP/1 head")

        raw_line = input_bytes[index:line_end]
        if raw_line.endswith(b"\r"):
            raw_line = raw_line[:-1]
        index = line_end + 1

        if not raw_line:
            return lines, index

        lines.append(raw_line.decode("latin-1"))


def _parse_headers(lines: list[str]) -> list[Header]:
    headers: list[Header] = []
    for line in lines:
        name, separator, raw_value = line.partition(":")
        if separator != ":" or not name.strip():
            raise Http1ParseError(f"invalid HTTP/1 header: {line}")
        headers.append(Header(name=name.strip(), value=raw_value.strip(" \t")))
    return headers


def _request_body_kind(headers: list[Header]) -> BodyKind:
    if _has_chunked_transfer_encoding(headers):
        return BodyKind.chunked()

    declared_length = _declared_content_length(headers)
    if declared_length in (None, 0):
        return BodyKind.none()
    return BodyKind.content_length(declared_length)


def _response_body_kind(status: int, headers: list[Header]) -> BodyKind:
    if 100 <= status < 200 or status in (204, 304):
        return BodyKind.none()
    if _has_chunked_transfer_encoding(headers):
        return BodyKind.chunked()

    declared_length = _declared_content_length(headers)
    if declared_length is None:
        return BodyKind.until_eof()
    if declared_length == 0:
        return BodyKind.none()
    return BodyKind.content_length(declared_length)


def _declared_content_length(headers: list[Header]) -> int | None:
    for header in headers:
        if header.name.lower() != "content-length":
            continue
        if not header.value.isdigit():
            raise Http1ParseError(f"invalid Content-Length: {header.value}")
        return int(header.value)
    return None


def _has_chunked_transfer_encoding(headers: list[Header]) -> bool:
    for header in headers:
        if header.name.lower() != "transfer-encoding":
            continue
        if any(piece.strip().lower() == "chunked" for piece in header.value.split(",")):
            return True
    return False
