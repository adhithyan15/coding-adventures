"""Bounded generic RFC 5280 Extension decoding."""

from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum

from der_asn1 import (
    Asn1Cursor,
    Asn1Decoder,
    Asn1Element,
    Asn1Error,
    Asn1ErrorKind,
    ObjectIdentifier,
    decode_boolean,
    decode_object_identifier,
    decode_octet_string,
)
from der_tlv import DerErrorKind

__version__ = "0.1.0"

BOOLEAN_TAG = 1


class X509ExtensionErrorKind(StrEnum):
    """Stable payload-free failure identifiers."""

    STRUCTURE = "structure"
    MISSING_EXTENSION_ID = "missing-extension-id"
    INVALID_EXTENSION_ID = "invalid-extension-id"
    INVALID_CRITICAL = "invalid-critical"
    ENCODED_DEFAULT_CRITICAL = "encoded-default-critical"
    MISSING_EXTENSION_VALUE = "missing-extension-value"
    INVALID_EXTENSION_VALUE = "invalid-extension-value"
    TRAILING_ELEMENT = "trailing-element"


class X509ExtensionError(ValueError):
    """A redacted Extension failure at an Extension-local byte offset."""

    def __init__(
        self,
        kind: X509ExtensionErrorKind,
        offset: int,
        asn1_kind: Asn1ErrorKind | None = None,
        framing_kind: DerErrorKind | None = None,
    ) -> None:
        self.kind = kind
        self.offset = offset
        self.asn1_kind = asn1_kind
        self.framing_kind = framing_kind
        super().__init__(f"X.509 extension error {kind.value} at byte {offset}")

    @classmethod
    def structure(
        cls, error: Asn1Error, offset: int | None = None
    ) -> X509ExtensionError:
        return cls(
            X509ExtensionErrorKind.STRUCTURE,
            error.offset if offset is None else offset,
            error.kind,
            error.framing_kind,
        )

    @classmethod
    def child_structure(
        cls, error: Asn1Error, value_offset: int, child_offset: int
    ) -> X509ExtensionError:
        offset = (
            value_offset + error.offset
            if error.kind is Asn1ErrorKind.FRAMING
            else child_offset + error.offset
        )
        return cls.structure(error, offset)

    @classmethod
    def semantic(
        cls, kind: X509ExtensionErrorKind, error: Asn1Error, child_offset: int
    ) -> X509ExtensionError:
        return cls(kind, child_offset + error.offset, error.kind, error.framing_kind)


@dataclass(frozen=True, slots=True, init=False)
class X509Extension:
    """One validated generic Extension with detached immutable bytes."""

    extension_id: ObjectIdentifier
    critical: bool
    extension_value: bytes

    def __new__(cls, *_args: object, **_kwargs: object) -> X509Extension:
        raise TypeError("X509Extension values are created by decode_x509_extension")

    @classmethod
    def _from_validated(
        cls,
        extension_id: ObjectIdentifier,
        critical: bool,
        extension_value: bytes | bytearray | memoryview,
    ) -> X509Extension:
        instance = object.__new__(cls)
        object.__setattr__(instance, "extension_id", extension_id)
        object.__setattr__(instance, "critical", critical)
        object.__setattr__(instance, "extension_value", bytes(extension_value))
        return instance


def decode_x509_extension(decoder: Asn1Decoder, element: Asn1Element) -> X509Extension:
    """Decode one RFC 5280 Extension under the decoder's shared budgets."""

    value_offset = len(element.header)
    value_length = len(element.value)
    try:
        fields = decoder.sequence(element)
    except Asn1Error as error:
        raise X509ExtensionError.structure(error) from error

    extension_id_offset = _child_offset(
        value_offset, value_length, len(fields.remaining)
    )
    extension_id_element = _read_child(
        decoder, fields, value_offset, extension_id_offset
    )
    if extension_id_element is None:
        raise X509ExtensionError(
            X509ExtensionErrorKind.MISSING_EXTENSION_ID, extension_id_offset
        )
    try:
        extension_id = decode_object_identifier(extension_id_element, decoder.limits)
    except Asn1Error as error:
        raise X509ExtensionError.semantic(
            X509ExtensionErrorKind.INVALID_EXTENSION_ID, error, extension_id_offset
        ) from error

    second_offset = _child_offset(value_offset, value_length, len(fields.remaining))
    second = _read_child(decoder, fields, value_offset, second_offset)
    if second is None:
        raise X509ExtensionError(
            X509ExtensionErrorKind.MISSING_EXTENSION_VALUE, second_offset
        )

    if second.tag.number == BOOLEAN_TAG:
        try:
            critical = decode_boolean(second)
        except Asn1Error as error:
            raise X509ExtensionError.semantic(
                X509ExtensionErrorKind.INVALID_CRITICAL, error, second_offset
            ) from error
        if not critical:
            raise X509ExtensionError(
                X509ExtensionErrorKind.ENCODED_DEFAULT_CRITICAL, second_offset
            )
        extension_value_offset = _child_offset(
            value_offset, value_length, len(fields.remaining)
        )
        extension_value_element = _read_child(
            decoder, fields, value_offset, extension_value_offset
        )
        if extension_value_element is None:
            raise X509ExtensionError(
                X509ExtensionErrorKind.MISSING_EXTENSION_VALUE,
                extension_value_offset,
            )
    else:
        critical = False
        extension_value_element = second
        extension_value_offset = second_offset

    try:
        extension_value = decode_octet_string(extension_value_element)
    except Asn1Error as error:
        raise X509ExtensionError.semantic(
            X509ExtensionErrorKind.INVALID_EXTENSION_VALUE,
            error,
            extension_value_offset,
        ) from error

    trailing_offset = _child_offset(value_offset, value_length, len(fields.remaining))
    if _read_child(decoder, fields, value_offset, trailing_offset) is not None:
        raise X509ExtensionError(
            X509ExtensionErrorKind.TRAILING_ELEMENT, trailing_offset
        )
    return X509Extension._from_validated(extension_id, critical, extension_value)


def _read_child(
    decoder: Asn1Decoder,
    fields: Asn1Cursor,
    value_offset: int,
    child_offset: int,
) -> Asn1Element | None:
    try:
        return fields.read(decoder)
    except Asn1Error as error:
        raise X509ExtensionError.child_structure(
            error, value_offset, child_offset
        ) from error


def _child_offset(value_offset: int, value_length: int, remaining_length: int) -> int:
    return value_offset + value_length - remaining_length


__all__ = [
    "X509Extension",
    "X509ExtensionError",
    "X509ExtensionErrorKind",
    "decode_x509_extension",
]
