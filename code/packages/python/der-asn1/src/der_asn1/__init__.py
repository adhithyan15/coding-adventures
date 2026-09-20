"""Typed, bounded ASN.1 DER values built on payload-blind DER framing."""

from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum

from der_tlv import (
    DerCursor,
    DerElement,
    DerError,
    DerErrorKind,
    DerLimits,
    TagClass,
)
from der_tlv import decode_exact as _decode_der_exact

__version__ = "0.1.0"

DEFAULT_MAX_DEPTH = 32
DEFAULT_MAX_TOTAL_ELEMENTS = 16_384
DEFAULT_MAX_OID_ARCS = 128


@dataclass(frozen=True, slots=True)
class Asn1Limits:
    """Explicit framing, nesting, whole-document work, and OID limits."""

    der: DerLimits = DerLimits()
    max_depth: int = DEFAULT_MAX_DEPTH
    max_total_elements: int = DEFAULT_MAX_TOTAL_ELEMENTS
    max_oid_arcs: int = DEFAULT_MAX_OID_ARCS

    def __post_init__(self) -> None:
        for name, value in (
            ("max_depth", self.max_depth),
            ("max_total_elements", self.max_total_elements),
            ("max_oid_arcs", self.max_oid_arcs),
        ):
            if value < 0:
                raise ValueError(f"{name} must be non-negative")


class Asn1ErrorKind(StrEnum):
    """Stable, payload-free failure identifiers."""

    FRAMING = "framing"
    UNEXPECTED_TAG = "unexpected-tag"
    DECODER_LIMIT_MISMATCH = "decoder-limit-mismatch"
    DEPTH_LIMIT_EXCEEDED = "depth-limit-exceeded"
    ELEMENT_LIMIT_EXCEEDED = "element-limit-exceeded"
    INVALID_BOOLEAN_LENGTH = "invalid-boolean-length"
    INVALID_BOOLEAN_VALUE = "invalid-boolean-value"
    EMPTY_INTEGER = "empty-integer"
    NON_MINIMAL_INTEGER = "non-minimal-integer"
    NEGATIVE_INTEGER = "negative-integer"
    INTEGER_OVERFLOW = "integer-overflow"
    MISSING_UNUSED_BIT_COUNT = "missing-unused-bit-count"
    INVALID_UNUSED_BIT_COUNT = "invalid-unused-bit-count"
    NON_ZERO_BIT_PADDING = "non-zero-bit-padding"
    BIT_LENGTH_OVERFLOW = "bit-length-overflow"
    NON_EMPTY_NULL = "non-empty-null"
    NON_ASCII_IA5_STRING = "non-ascii-ia5-string"
    EMPTY_OBJECT_IDENTIFIER = "empty-object-identifier"
    UNTERMINATED_OBJECT_IDENTIFIER = "unterminated-object-identifier"
    NON_MINIMAL_OBJECT_IDENTIFIER = "non-minimal-object-identifier"
    OBJECT_IDENTIFIER_OVERFLOW = "object-identifier-overflow"
    OID_ARC_LIMIT_EXCEEDED = "oid-arc-limit-exceeded"


class Asn1Error(ValueError):
    """A redacted typed-DER failure at a local byte offset."""

    def __init__(
        self,
        kind: Asn1ErrorKind,
        offset: int,
        framing_kind: DerErrorKind | None = None,
    ) -> None:
        self.kind = kind
        self.offset = offset
        self.framing_kind = framing_kind
        super().__init__(f"ASN.1 DER value error {kind.value} at byte {offset}")

    @classmethod
    def framing(cls, error: DerError) -> Asn1Error:
        return cls(Asn1ErrorKind.FRAMING, error.offset, error.kind)


@dataclass(frozen=True, slots=True, init=False)
class Asn1Element:
    """One immutable framed element plus its document depth."""

    element: DerElement
    depth: int

    def __new__(cls) -> Asn1Element:
        raise TypeError("Asn1Element values are created by Asn1Decoder")

    @classmethod
    def _from_der(cls, element: DerElement, depth: int) -> Asn1Element:
        instance = object.__new__(cls)
        object.__setattr__(instance, "element", element)
        object.__setattr__(instance, "depth", depth)
        return instance

    @property
    def tag(self):  # noqa: ANN201 - public type comes from der-tlv
        return self.element.tag

    @property
    def header(self) -> memoryview:
        return self.element.header

    @property
    def value(self) -> memoryview:
        return self.element.value

    @property
    def encoded(self) -> memoryview:
        return self.element.encoded

    @property
    def value_offset(self) -> int:
        return len(self.header)


class Asn1Decoder:
    """Shared depth and total-element authority for one schema walk."""

    def __init__(self, limits: Asn1Limits | None = None) -> None:
        self._limits = limits or Asn1Limits()
        self._elements_read = 0

    @property
    def limits(self) -> Asn1Limits:
        return self._limits

    @property
    def elements_read(self) -> int:
        return self._elements_read

    def decode_exact(self, data: bytes | bytearray | memoryview) -> Asn1Element:
        if self._limits.max_depth == 0:
            raise Asn1Error(Asn1ErrorKind.DEPTH_LIMIT_EXCEEDED, 0)
        self._require_element_capacity(0)
        try:
            element = _decode_der_exact(data, self._limits.der)
        except DerError as error:
            raise Asn1Error.framing(error) from error
        self._elements_read += 1
        return Asn1Element._from_der(element, 0)

    def sequence(self, element: Asn1Element) -> Asn1Cursor:
        return self._constructed(element, TagClass.UNIVERSAL, 16)

    def set(self, element: Asn1Element) -> Asn1Cursor:
        return self._constructed(element, TagClass.UNIVERSAL, 17)

    def explicit(self, element: Asn1Element, tag_number: int) -> Asn1Element:
        _expect_tag(element, TagClass.CONTEXT_SPECIFIC, True, tag_number)
        child_depth = self._child_depth(element)
        self._require_element_capacity(element.value_offset)
        try:
            child = _decode_der_exact(element.value, self._limits.der)
        except DerError as error:
            raise Asn1Error.framing(error) from error
        self._elements_read += 1
        return Asn1Element._from_der(child, child_depth)

    def _constructed(
        self, element: Asn1Element, class_: TagClass, number: int
    ) -> Asn1Cursor:
        _expect_tag(element, class_, True, number)
        child_depth = self._child_depth(element)
        try:
            cursor = DerCursor(element.value, self._limits.der)
        except DerError as error:
            raise Asn1Error.framing(error) from error
        return Asn1Cursor(cursor, child_depth, self._limits)

    def _child_depth(self, element: Asn1Element) -> int:
        child_depth = element.depth + 1
        if child_depth >= self._limits.max_depth:
            raise Asn1Error(Asn1ErrorKind.DEPTH_LIMIT_EXCEEDED, 0)
        return child_depth

    def _require_element_capacity(self, offset: int) -> None:
        if self._elements_read >= self._limits.max_total_elements:
            raise Asn1Error(Asn1ErrorKind.ELEMENT_LIMIT_EXCEEDED, offset)


class Asn1Cursor:
    """An iterative sibling cursor sharing an :class:`Asn1Decoder` budget."""

    def __init__(self, cursor: DerCursor, child_depth: int, limits: Asn1Limits) -> None:
        self._cursor = cursor
        self._child_depth = child_depth
        self._limits = limits

    @property
    def remaining(self) -> memoryview:
        return self._cursor.remaining

    def read(self, decoder: Asn1Decoder) -> Asn1Element | None:
        if not self._cursor.remaining:
            return None
        if decoder.limits != self._limits:
            raise Asn1Error(Asn1ErrorKind.DECODER_LIMIT_MISMATCH, 0)
        decoder._require_element_capacity(0)
        try:
            element = self._cursor.read()
        except DerError as error:
            raise Asn1Error.framing(error) from error
        if element is None:
            return None
        decoder._elements_read += 1
        return Asn1Element._from_der(element, self._child_depth)

    def finish(self) -> None:
        try:
            self._cursor.finish()
        except DerError as error:
            raise Asn1Error.framing(error) from error


@dataclass(frozen=True, slots=True)
class DerInteger:
    signed_bytes: memoryview
    value_offset: int

    @property
    def is_negative(self) -> bool:
        return bool(self.signed_bytes[0] & 0x80)

    def to_u64(self) -> int:
        if self.is_negative:
            raise Asn1Error(Asn1ErrorKind.NEGATIVE_INTEGER, self.value_offset)
        magnitude = (
            self.signed_bytes[1:] if self.signed_bytes[0] == 0 else self.signed_bytes
        )
        if len(magnitude) > 8:
            raise Asn1Error(Asn1ErrorKind.INTEGER_OVERFLOW, self.value_offset)
        return int.from_bytes(magnitude, "big")


@dataclass(frozen=True, slots=True)
class DerBitString:
    bytes: memoryview
    unused_bits: int
    bit_length: int


@dataclass(frozen=True, slots=True)
class ObjectIdentifier:
    encoded: memoryview
    arcs: tuple[int, ...]

    @property
    def arc_count(self) -> int:
        return len(self.arcs)

    def equals(self, expected: tuple[int, ...] | list[int]) -> bool:
        return tuple(expected) == self.arcs


def decode_boolean(element: Asn1Element) -> bool:
    _expect_universal_primitive(element, 1)
    if len(element.value) != 1:
        raise Asn1Error(Asn1ErrorKind.INVALID_BOOLEAN_LENGTH, element.value_offset)
    if element.value[0] == 0:
        return False
    if element.value[0] == 0xFF:
        return True
    raise Asn1Error(Asn1ErrorKind.INVALID_BOOLEAN_VALUE, element.value_offset)


def decode_integer(element: Asn1Element) -> DerInteger:
    _expect_universal_primitive(element, 2)
    value = element.value
    if not value:
        raise Asn1Error(Asn1ErrorKind.EMPTY_INTEGER, element.value_offset)
    if len(value) > 1 and (
        (value[0] == 0 and value[1] & 0x80 == 0)
        or (value[0] == 0xFF and value[1] & 0x80 != 0)
    ):
        raise Asn1Error(Asn1ErrorKind.NON_MINIMAL_INTEGER, element.value_offset)
    return DerInteger(value, element.value_offset)


def decode_bit_string(element: Asn1Element) -> DerBitString:
    _expect_universal_primitive(element, 3)
    value = element.value
    if not value:
        raise Asn1Error(Asn1ErrorKind.MISSING_UNUSED_BIT_COUNT, element.value_offset)
    unused_bits = value[0]
    payload = value[1:]
    if unused_bits > 7 or (not payload and unused_bits != 0):
        raise Asn1Error(Asn1ErrorKind.INVALID_UNUSED_BIT_COUNT, element.value_offset)
    if unused_bits and payload[-1] & ((1 << unused_bits) - 1):
        raise Asn1Error(
            Asn1ErrorKind.NON_ZERO_BIT_PADDING,
            element.value_offset + len(value) - 1,
        )
    return DerBitString(payload, unused_bits, len(payload) * 8 - unused_bits)


def decode_octet_string(element: Asn1Element) -> memoryview:
    _expect_universal_primitive(element, 4)
    return element.value


def decode_implicit_octet_string(element: Asn1Element, tag_number: int) -> memoryview:
    _expect_context_primitive(element, tag_number)
    return element.value


def decode_ia5_string(element: Asn1Element) -> str:
    _expect_universal_primitive(element, 22)
    return _decode_ia5_contents(element)


def decode_implicit_ia5_string(element: Asn1Element, tag_number: int) -> str:
    _expect_context_primitive(element, tag_number)
    return _decode_ia5_contents(element)


def decode_null(element: Asn1Element) -> None:
    _expect_universal_primitive(element, 5)
    if element.value:
        raise Asn1Error(Asn1ErrorKind.NON_EMPTY_NULL, element.value_offset)


def decode_object_identifier(
    element: Asn1Element, limits: Asn1Limits | None = None
) -> ObjectIdentifier:
    _expect_universal_primitive(element, 6)
    return _decode_oid_contents(element, limits or Asn1Limits())


def decode_implicit_object_identifier(
    element: Asn1Element, tag_number: int, limits: Asn1Limits | None = None
) -> ObjectIdentifier:
    _expect_context_primitive(element, tag_number)
    return _decode_oid_contents(element, limits or Asn1Limits())


def _decode_ia5_contents(element: Asn1Element) -> str:
    for offset, octet in enumerate(element.value):
        if octet > 0x7F:
            raise Asn1Error(
                Asn1ErrorKind.NON_ASCII_IA5_STRING, element.value_offset + offset
            )
    return bytes(element.value).decode("ascii")


def _decode_oid_contents(element: Asn1Element, limits: Asn1Limits) -> ObjectIdentifier:
    encoded = element.value
    if not encoded:
        raise Asn1Error(Asn1ErrorKind.EMPTY_OBJECT_IDENTIFIER, element.value_offset)
    combined, offset = _parse_base128(encoded, 0, element.value_offset)
    if combined < 40:
        arcs = [0, combined]
    elif combined < 80:
        arcs = [1, combined - 40]
    else:
        arcs = [2, combined - 80]
    if len(arcs) > limits.max_oid_arcs:
        raise Asn1Error(Asn1ErrorKind.OID_ARC_LIMIT_EXCEEDED, element.value_offset)
    while offset < len(encoded):
        arc_start = offset
        arc, offset = _parse_base128(encoded, offset, element.value_offset)
        arcs.append(arc)
        if len(arcs) > limits.max_oid_arcs:
            raise Asn1Error(
                Asn1ErrorKind.OID_ARC_LIMIT_EXCEEDED,
                element.value_offset + arc_start,
            )
    return ObjectIdentifier(encoded, tuple(arcs))


def _parse_base128(
    encoded: memoryview, start: int, value_offset: int
) -> tuple[int, int]:
    if encoded[start] == 0x80:
        raise Asn1Error(
            Asn1ErrorKind.NON_MINIMAL_OBJECT_IDENTIFIER, value_offset + start
        )
    value = 0
    offset = start
    while True:
        if offset >= len(encoded):
            raise Asn1Error(
                Asn1ErrorKind.UNTERMINATED_OBJECT_IDENTIFIER, value_offset + offset
            )
        octet = encoded[offset]
        candidate = value * 128 + (octet & 0x7F)
        if candidate > 0xFFFF_FFFF_FFFF_FFFF:
            raise Asn1Error(
                Asn1ErrorKind.OBJECT_IDENTIFIER_OVERFLOW, value_offset + offset
            )
        value = candidate
        offset += 1
        if octet & 0x80 == 0:
            return value, offset


def _expect_universal_primitive(element: Asn1Element, number: int) -> None:
    _expect_tag(element, TagClass.UNIVERSAL, False, number)


def _expect_context_primitive(element: Asn1Element, number: int) -> None:
    _expect_tag(element, TagClass.CONTEXT_SPECIFIC, False, number)


def _expect_tag(
    element: Asn1Element, class_: TagClass, constructed: bool, number: int
) -> None:
    tag = element.tag
    if (
        tag.class_ is not class_
        or tag.constructed != constructed
        or tag.number != number
    ):
        raise Asn1Error(Asn1ErrorKind.UNEXPECTED_TAG, 0)


__all__ = [
    "Asn1Cursor",
    "Asn1Decoder",
    "Asn1Element",
    "Asn1Error",
    "Asn1ErrorKind",
    "Asn1Limits",
    "DerBitString",
    "DerInteger",
    "ObjectIdentifier",
    "decode_bit_string",
    "decode_boolean",
    "decode_ia5_string",
    "decode_implicit_ia5_string",
    "decode_implicit_object_identifier",
    "decode_implicit_octet_string",
    "decode_integer",
    "decode_null",
    "decode_object_identifier",
    "decode_octet_string",
]
