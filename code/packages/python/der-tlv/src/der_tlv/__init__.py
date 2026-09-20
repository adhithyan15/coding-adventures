"""Bounded, payload-blind DER tag-length-value framing.

The decoder intentionally knows nothing about ASN.1 value semantics. It only
identifies one canonical DER frame and returns memoryview slices into the
caller's input. A declared wire length is validated before any value slice is
formed and never drives allocation.
"""

from __future__ import annotations

import sys
from dataclasses import dataclass
from enum import StrEnum

__version__ = "0.1.0"

DEFAULT_MAX_INPUT_LEN = 1024 * 1024
DEFAULT_MAX_VALUE_LEN = 1024 * 1024
DEFAULT_MAX_ELEMENTS = 4096


class TagClass(StrEnum):
    """The two class bits of an ASN.1 identifier octet."""

    UNIVERSAL = "universal"
    APPLICATION = "application"
    CONTEXT_SPECIFIC = "context-specific"
    PRIVATE = "private"


class DerErrorKind(StrEnum):
    """Stable error identifiers shared by every portable implementation."""

    EMPTY_INPUT = "empty-input"
    TRUNCATED_HIGH_TAG = "truncated-high-tag"
    TRUNCATED_LENGTH = "truncated-length"
    TRUNCATED_VALUE = "truncated-value"
    END_OF_CONTENTS = "end-of-contents"
    NON_MINIMAL_TAG = "non-minimal-tag"
    TAG_OVERFLOW = "tag-overflow"
    INDEFINITE_LENGTH = "indefinite-length"
    RESERVED_LENGTH = "reserved-length"
    NON_MINIMAL_LENGTH = "non-minimal-length"
    LENGTH_TOO_WIDE = "length-too-wide"
    LENGTH_HOST_OVERFLOW = "length-host-overflow"
    INPUT_LIMIT_EXCEEDED = "input-limit-exceeded"
    VALUE_LIMIT_EXCEEDED = "value-limit-exceeded"
    ELEMENT_LIMIT_EXCEEDED = "element-limit-exceeded"
    TAG_LIMIT_EXCEEDED = "tag-limit-exceeded"
    TRAILING_DATA = "trailing-data"


class DerError(ValueError):
    """A payload-blind framing failure at an input byte offset."""

    def __init__(self, kind: DerErrorKind, offset: int) -> None:
        self.kind = kind
        self.offset = offset
        super().__init__(f"DER framing error {kind.value} at byte {offset}")


@dataclass(frozen=True, slots=True)
class DerLimits:
    """Explicit resource and representation limits for hostile input."""

    max_input_len: int = DEFAULT_MAX_INPUT_LEN
    max_value_len: int = DEFAULT_MAX_VALUE_LEN
    max_elements: int = DEFAULT_MAX_ELEMENTS
    max_tag_number: int = 0xFFFF_FFFF

    def __post_init__(self) -> None:
        for name, value in (
            ("max_input_len", self.max_input_len),
            ("max_value_len", self.max_value_len),
            ("max_elements", self.max_elements),
            ("max_tag_number", self.max_tag_number),
        ):
            if value < 0:
                raise ValueError(f"{name} must be non-negative")
        if self.max_tag_number > 0xFFFF_FFFF:
            raise ValueError("max_tag_number must fit u32")


@dataclass(frozen=True, slots=True)
class DerTag:
    """One decoded DER identifier."""

    class_: TagClass
    constructed: bool
    number: int


@dataclass(frozen=True, slots=True)
class DerElement:
    """One frame represented as zero-copy views over the supplied input."""

    tag: DerTag
    _input: memoryview
    _start: int
    _header_len: int
    _encoded_len: int

    @property
    def header(self) -> memoryview:
        return self._input[self._start : self._start + self._header_len]

    @property
    def value(self) -> memoryview:
        start = self._start + self._header_len
        return self._input[start : self._start + self._encoded_len]

    @property
    def encoded(self) -> memoryview:
        return self._input[self._start : self._start + self._encoded_len]


def _fail(kind: DerErrorKind, offset: int) -> None:
    raise DerError(kind, offset)


def _decode_at(
    input_view: memoryview, start: int, available: int, limits: DerLimits
) -> tuple[DerElement, int]:
    if available > limits.max_input_len:
        _fail(DerErrorKind.INPUT_LIMIT_EXCEEDED, start)
    if available == 0:
        _fail(DerErrorKind.EMPTY_INPUT, start)

    first = input_view[start]
    class_ = (
        TagClass.UNIVERSAL,
        TagClass.APPLICATION,
        TagClass.CONTEXT_SPECIFIC,
        TagClass.PRIVATE,
    )[first >> 6]
    constructed = bool(first & 0x20)
    low = first & 0x1F
    if low != 0x1F:
        number = low
        identifier_len = 1
        if number > limits.max_tag_number:
            _fail(DerErrorKind.TAG_LIMIT_EXCEEDED, start)
    else:
        number = 0
        index = 1
        while True:
            if index >= available:
                _fail(DerErrorKind.TRUNCATED_HIGH_TAG, start + index)
            octet = input_view[start + index]
            payload = octet & 0x7F
            if index == 1 and payload == 0:
                _fail(DerErrorKind.NON_MINIMAL_TAG, start + index)
            candidate = number * 128 + payload
            if candidate > 0xFFFF_FFFF:
                _fail(DerErrorKind.TAG_OVERFLOW, start + index)
            number = candidate
            if number > limits.max_tag_number:
                _fail(DerErrorKind.TAG_LIMIT_EXCEEDED, start + index)
            index += 1
            if not octet & 0x80:
                break
        if number < 31:
            _fail(DerErrorKind.NON_MINIMAL_TAG, start)
        identifier_len = index

    if class_ is TagClass.UNIVERSAL and number == 0:
        _fail(DerErrorKind.END_OF_CONTENTS, start)

    length_offset = start + identifier_len
    if identifier_len >= available:
        _fail(DerErrorKind.TRUNCATED_LENGTH, length_offset)
    first_length = input_view[length_offset]
    if first_length < 0x80:
        value_len = first_length
        length_len = 1
    else:
        if first_length == 0x80:
            _fail(DerErrorKind.INDEFINITE_LENGTH, length_offset)
        if first_length == 0xFF:
            _fail(DerErrorKind.RESERVED_LENGTH, length_offset)
        count = first_length & 0x7F
        if count > 8:
            _fail(DerErrorKind.LENGTH_TOO_WIDE, length_offset)
        value_start = identifier_len + 1
        value_end = value_start + count
        if value_end > available:
            _fail(DerErrorKind.TRUNCATED_LENGTH, start + available)
        if input_view[start + value_start] == 0:
            _fail(DerErrorKind.NON_MINIMAL_LENGTH, start + value_start)
        value_len = 0
        for index in range(value_start, value_end):
            value_len = value_len * 256 + input_view[start + index]
        if value_len < 128:
            _fail(DerErrorKind.NON_MINIMAL_LENGTH, length_offset)
        if value_len > sys.maxsize:
            _fail(DerErrorKind.LENGTH_HOST_OVERFLOW, length_offset)
        length_len = 1 + count

    if value_len > limits.max_value_len:
        _fail(DerErrorKind.VALUE_LIMIT_EXCEEDED, length_offset)
    header_len = identifier_len + length_len
    if value_len > sys.maxsize - header_len:
        _fail(DerErrorKind.LENGTH_HOST_OVERFLOW, length_offset)
    encoded_len = header_len + value_len
    if encoded_len > available:
        _fail(DerErrorKind.TRUNCATED_VALUE, start + available)

    element = DerElement(
        tag=DerTag(class_, constructed, number),
        _input=input_view,
        _start=start,
        _header_len=header_len,
        _encoded_len=encoded_len,
    )
    return element, start + encoded_len


def decode_one(
    data: bytes | bytearray | memoryview, limits: DerLimits | None = None
) -> tuple[DerElement, memoryview]:
    """Decode one canonical frame and return its untouched remainder."""

    input_view = memoryview(data).cast("B")
    element, next_offset = _decode_at(
        input_view, 0, len(input_view), limits or DerLimits()
    )
    return element, input_view[next_offset:]


def decode_exact(
    data: bytes | bytearray | memoryview, limits: DerLimits | None = None
) -> DerElement:
    """Decode exactly one frame, rejecting even valid trailing bytes."""

    input_view = memoryview(data).cast("B")
    element, next_offset = _decode_at(
        input_view, 0, len(input_view), limits or DerLimits()
    )
    if next_offset != len(input_view):
        _fail(DerErrorKind.TRAILING_DATA, next_offset)
    return element


class DerCursor:
    """Iterative sibling decoder whose failure paths never advance state."""

    def __init__(
        self, data: bytes | bytearray | memoryview, limits: DerLimits | None = None
    ) -> None:
        self._input = memoryview(data).cast("B")
        self._limits = limits or DerLimits()
        if len(self._input) > self._limits.max_input_len:
            _fail(DerErrorKind.INPUT_LIMIT_EXCEEDED, 0)
        self._offset = 0
        self._elements_read = 0

    @property
    def elements_read(self) -> int:
        return self._elements_read

    @property
    def remaining(self) -> memoryview:
        return self._input[self._offset :]

    def read(self) -> DerElement | None:
        if self._offset == len(self._input):
            return None
        if self._elements_read >= self._limits.max_elements:
            _fail(DerErrorKind.ELEMENT_LIMIT_EXCEEDED, self._offset)
        element, next_offset = _decode_at(
            self._input,
            self._offset,
            len(self._input) - self._offset,
            self._limits,
        )
        self._offset = next_offset
        self._elements_read += 1
        return element

    def finish(self) -> None:
        if self._offset != len(self._input):
            _fail(DerErrorKind.TRAILING_DATA, self._offset)


__all__ = [
    "DEFAULT_MAX_ELEMENTS",
    "DEFAULT_MAX_INPUT_LEN",
    "DEFAULT_MAX_VALUE_LEN",
    "DerCursor",
    "DerElement",
    "DerError",
    "DerErrorKind",
    "DerLimits",
    "DerTag",
    "TagClass",
    "decode_exact",
    "decode_one",
]
