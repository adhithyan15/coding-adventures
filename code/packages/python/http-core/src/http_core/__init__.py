"""Shared HTTP message types and helpers.

The HTTP stack has two layers of complexity:

1. wire parsing, which changes across HTTP/1, HTTP/2, and HTTP/3
2. semantic message shape, which stays recognisable across all of them

This module implements the second layer. It gives the version-specific parsers
common request/response head types plus a small set of helpers for interpreting
headers like `Content-Length` and `Content-Type`.
"""

from __future__ import annotations

from dataclasses import dataclass

__all__ = [
    "BodyKind",
    "Header",
    "HttpVersion",
    "RequestHead",
    "ResponseHead",
    "find_header",
    "parse_content_length",
    "parse_content_type",
]

__version__ = "0.1.0"


@dataclass(frozen=True, slots=True)
class Header:
    """One HTTP header line, kept in arrival order."""

    name: str
    value: str


@dataclass(frozen=True, slots=True)
class HttpVersion:
    """A semantic HTTP version represented as major/minor integers."""

    major: int
    minor: int

    @classmethod
    def parse(cls, text: str) -> "HttpVersion":
        if not text.startswith("HTTP/"):
            raise ValueError(f"invalid HTTP version: {text}")
        major_text, dot, minor_text = text[5:].partition(".")
        if dot != "." or not major_text.isdigit() or not minor_text.isdigit():
            raise ValueError(f"invalid HTTP version: {text}")
        return cls(int(major_text), int(minor_text))

    def __str__(self) -> str:
        return f"HTTP/{self.major}.{self.minor}"


@dataclass(frozen=True, slots=True)
class BodyKind:
    """Describes how a caller should consume the message body."""

    mode: str
    length: int | None = None

    @classmethod
    def none(cls) -> "BodyKind":
        return cls("none")

    @classmethod
    def content_length(cls, length: int) -> "BodyKind":
        return cls("content-length", length)

    @classmethod
    def until_eof(cls) -> "BodyKind":
        return cls("until-eof")

    @classmethod
    def chunked(cls) -> "BodyKind":
        return cls("chunked")


@dataclass(frozen=True, slots=True)
class RequestHead:
    """A parsed HTTP request head."""

    method: str
    target: str
    version: HttpVersion
    headers: list[Header]

    def header(self, name: str) -> str | None:
        return find_header(self.headers, name)

    def content_length(self) -> int | None:
        return parse_content_length(self.headers)

    def content_type(self) -> tuple[str, str | None] | None:
        return parse_content_type(self.headers)


@dataclass(frozen=True, slots=True)
class ResponseHead:
    """A parsed HTTP response head."""

    version: HttpVersion
    status: int
    reason: str
    headers: list[Header]

    def header(self, name: str) -> str | None:
        return find_header(self.headers, name)

    def content_length(self) -> int | None:
        return parse_content_length(self.headers)

    def content_type(self) -> tuple[str, str | None] | None:
        return parse_content_type(self.headers)


def find_header(headers: list[Header], name: str) -> str | None:
    """Return the first matching header value using ASCII-insensitive lookup."""

    lowered = name.lower()
    for header in headers:
        if header.name.lower() == lowered:
            return header.value
    return None


def parse_content_length(headers: list[Header]) -> int | None:
    """Return a non-negative Content-Length value when present and valid."""

    value = find_header(headers, "Content-Length")
    if value is None or not value.isdigit():
        return None
    return int(value)


def parse_content_type(headers: list[Header]) -> tuple[str, str | None] | None:
    """Split Content-Type into media type and optional charset."""

    value = find_header(headers, "Content-Type")
    if value is None:
        return None

    pieces = [piece.strip() for piece in value.split(";")]
    media_type = pieces[0]
    if not media_type:
        return None

    charset: str | None = None
    for piece in pieces[1:]:
        key, separator, raw_value = piece.partition("=")
        if separator and key.strip().lower() == "charset":
            charset = raw_value.strip().strip('"')
            break

    return media_type, charset
