"""A small, strict implementation of CBR01 canonical CBOR.

Ordinary CBOR permits several byte strings for one logical value.  CBR01 is the
deliberately narrower profile used by the repository's Vault layers: definite
lengths, shortest arguments, and RFC 8949 section 4.2.3 length-first map keys.
The implementation below uses only Python's standard library and keeps all I/O,
cryptography, clocks, randomness, and storage outside this package.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Final, cast

__version__ = "0.1.0"

MAX_NESTING_DEPTH: Final = 128
MAX_ENCODED_BYTES: Final = 1_048_576
_U64_MAX: Final = (1 << 64) - 1

ERROR_IDS: Final = (
    "unexpected-eof",
    "trailing-bytes",
    "reserved",
    "indefinite",
    "non-minimal-integer",
    "invalid-utf8",
    "non-canonical-map-order",
    "unsupported-simple",
    "float-not-supported",
    "too-deep",
    "length-too-large",
    "duplicate-map-key",
    "encode-too-deep",
    "encode-too-large",
)

_ERROR_MESSAGES: Final = {
    "unexpected-eof": "canonical-cbor: unexpected end of input",
    "trailing-bytes": "canonical-cbor: trailing bytes after decoded item",
    "reserved": "canonical-cbor: reserved additional-info value",
    "indefinite": "canonical-cbor: indefinite item rejected",
    "non-minimal-integer": "canonical-cbor: argument is not in smallest form",
    "invalid-utf8": "canonical-cbor: text is not valid UTF-8",
    "non-canonical-map-order": "canonical-cbor: map key order is not canonical",
    "unsupported-simple": "canonical-cbor: unsupported simple value",
    "float-not-supported": "canonical-cbor: floats are not supported",
    "too-deep": "canonical-cbor: decoded nesting is too deep",
    "length-too-large": "canonical-cbor: declared length is too large",
    "duplicate-map-key": "canonical-cbor: duplicate canonical map key",
    "encode-too-deep": "canonical-cbor: encoded nesting is too deep",
    "encode-too-large": "canonical-cbor: encoded item is too large",
}


class CborError(ValueError):
    """One of CBR01's fourteen stable, payload-blind conformance errors."""

    def __init__(self, error_id: str) -> None:
        try:
            message = _ERROR_MESSAGES[error_id]
        except KeyError as error:
            raise ValueError("canonical-cbor: unknown error identifier") from error
        self.error_id = error_id
        super().__init__(message)


class CborValue:
    """Marker base class for the closed value algebra."""

    __slots__ = ()


def _require_u64(value: object, name: str) -> int:
    if type(value) is not int:
        raise TypeError(f"canonical-cbor: {name} must be an integer")
    if value < 0 or value > _U64_MAX:
        raise ValueError(f"canonical-cbor: {name} must be an unsigned 64-bit integer")
    return value


def _validate_scalar_text(value: object) -> str:
    if type(value) is not str:
        raise TypeError("canonical-cbor: text value must be a string")
    # Python can contain lone surrogate code points even though UTF-8 cannot.
    # Checking code points before calling encode keeps the diagnostic static.
    if any(0xD800 <= ord(character) <= 0xDFFF for character in value):
        raise ValueError("canonical-cbor: text is not Unicode scalar data")
    return value


@dataclass(frozen=True, slots=True)
class CborUnsigned(CborValue):
    """CBOR major type 0 over the complete unsigned 64-bit domain."""

    value: int

    def __post_init__(self) -> None:
        _require_u64(self.value, "unsigned value")


@dataclass(frozen=True, slots=True)
class CborNegative(CborValue):
    """CBOR major type 1, storing ``n`` for the value ``-1 - n``."""

    value: int

    def __post_init__(self) -> None:
        _require_u64(self.value, "negative argument")


@dataclass(frozen=True, slots=True, init=False)
class CborByteString(CborValue):
    """An immutable, defensively copied byte string."""

    value: bytes

    def __init__(self, value: bytes | bytearray | memoryview) -> None:
        if not isinstance(value, bytes | bytearray | memoryview):
            raise TypeError("canonical-cbor: byte string must be bytes-like")
        object.__setattr__(self, "value", bytes(value))


@dataclass(frozen=True, slots=True)
class CborText(CborValue):
    """A Python string restricted to Unicode scalar values."""

    value: str

    def __post_init__(self) -> None:
        _validate_scalar_text(self.value)


@dataclass(frozen=True, slots=True, init=False)
class CborArray(CborValue):
    """An immutable ordered sequence of CBOR values."""

    items: tuple[CborValue, ...]

    def __init__(self, items: object) -> None:
        if type(items) not in {list, tuple}:
            raise TypeError(
                "canonical-cbor: array items must be a list or tuple of "
                "CborValue objects"
            )
        owned: tuple[object, ...] = tuple(
            cast(list[object] | tuple[object, ...], items)
        )
        if any(not _is_exact_cbor_value(item) for item in owned):
            raise TypeError("canonical-cbor: array items must be CborValue objects")
        object.__setattr__(self, "items", cast(tuple[CborValue, ...], owned))


@dataclass(frozen=True, slots=True)
class CborMapEntry:
    """One map entry before the encoder canonicalizes key order."""

    key: CborValue
    value: CborValue

    def __post_init__(self) -> None:
        if not _is_exact_cbor_value(self.key) or not _is_exact_cbor_value(self.value):
            raise TypeError(
                "canonical-cbor: map entry values must be CborValue objects"
            )


@dataclass(frozen=True, slots=True, init=False)
class CborMap(CborValue):
    """An immutable sequence of entries, canonicalized only while encoding."""

    entries: tuple[CborMapEntry, ...]

    def __init__(self, entries: object) -> None:
        if type(entries) not in {list, tuple}:
            raise TypeError(
                "canonical-cbor: map requires a list or tuple of CborMapEntry objects"
            )
        owned: tuple[object, ...] = tuple(
            cast(list[object] | tuple[object, ...], entries)
        )
        if any(type(entry) is not CborMapEntry for entry in owned):
            raise TypeError("canonical-cbor: map requires CborMapEntry objects")
        object.__setattr__(self, "entries", cast(tuple[CborMapEntry, ...], owned))


@dataclass(frozen=True, slots=True)
class CborTag(CborValue):
    """An uninterpreted unsigned tag and its nested value."""

    number: int
    value: CborValue

    def __post_init__(self) -> None:
        _require_u64(self.number, "tag number")
        if not _is_exact_cbor_value(self.value):
            raise TypeError("canonical-cbor: tag value must be a CborValue object")


@dataclass(frozen=True, slots=True)
class CborBoolean(CborValue):
    """The two supported CBOR boolean simple values."""

    value: bool

    def __post_init__(self) -> None:
        if type(self.value) is not bool:
            raise TypeError("canonical-cbor: boolean value must be a boolean")


@dataclass(frozen=True, slots=True)
class CborNull(CborValue):
    """The sole payload-free null kind."""


NULL: Final = CborNull()

type Cbor = (
    CborUnsigned
    | CborNegative
    | CborByteString
    | CborText
    | CborArray
    | CborMap
    | CborTag
    | CborBoolean
    | CborNull
)

_EXACT_VALUE_TYPES: Final = {
    CborUnsigned,
    CborNegative,
    CborByteString,
    CborText,
    CborArray,
    CborMap,
    CborTag,
    CborBoolean,
    CborNull,
}


def _is_exact_cbor_value(value: object) -> bool:
    return type(value) in _EXACT_VALUE_TYPES


def _argument_size(argument: int) -> int:
    if argument <= 23:
        return 1
    if argument <= 0xFF:
        return 2
    if argument <= 0xFFFF:
        return 3
    if argument <= 0xFFFF_FFFF:
        return 5
    # Every collection payload is capped at 1 MiB before publication, so a
    # nine-byte collection header cannot be reached without first exhausting
    # host memory while constructing the caller-owned value.
    return 9  # pragma: no cover


def _utf8_length(value: str) -> int:
    length = 0
    for character in value:
        point = ord(character)
        if point <= 0x7F:
            length += 1
        elif point <= 0x7FF:
            length += 2
        elif point <= 0xFFFF:
            length += 3
        else:
            length += 4
    return length


class _Encoder:
    """Private staging buffer: callers see bytes only after full validation."""

    def __init__(self) -> None:
        self.output = bytearray()

    def ensure_fits(self, additional: int) -> None:
        if additional > MAX_ENCODED_BYTES - len(self.output):
            raise CborError("encode-too-large")

    def write_byte(self, value: int) -> None:
        self.ensure_fits(1)
        self.output.append(value & 0xFF)

    def write_bytes(self, value: bytes) -> None:
        self.ensure_fits(len(value))
        self.output.extend(value)

    def write_unsigned(self, value: int, width: int) -> None:
        self.write_bytes(value.to_bytes(width, "big"))

    def write_argument(self, major: int, argument: int) -> None:
        prefix = major << 5
        if argument <= 23:
            self.write_byte(prefix | argument)
        elif argument <= 0xFF:
            self.write_byte(prefix | 24)
            self.write_byte(argument)
        elif argument <= 0xFFFF:
            self.write_byte(prefix | 25)
            self.write_unsigned(argument, 2)
        elif argument <= 0xFFFF_FFFF:
            self.write_byte(prefix | 26)
            self.write_unsigned(argument, 4)
        else:
            self.write_byte(prefix | 27)
            self.write_unsigned(argument, 8)

    def write_value(self, value: object, depth: int) -> None:
        if depth > MAX_NESTING_DEPTH:
            raise CborError("encode-too-deep")
        value_type = type(value)
        if value_type is CborUnsigned:
            unsigned = cast(CborUnsigned, value)
            self.write_argument(0, _require_u64(unsigned.value, "unsigned value"))
        elif value_type is CborNegative:
            negative = cast(CborNegative, value)
            self.write_argument(1, _require_u64(negative.value, "negative argument"))
        elif value_type is CborByteString:
            byte_string = cast(CborByteString, value)
            payload = byte_string.value
            if type(payload) is not bytes:
                raise TypeError("canonical-cbor: invalid byte string value")
            self.ensure_fits(_argument_size(len(payload)) + len(payload))
            self.write_argument(2, len(payload))
            self.write_bytes(payload)
        elif value_type is CborText:
            text_value = cast(CborText, value)
            text = _validate_scalar_text(text_value.value)
            length = _utf8_length(text)
            self.ensure_fits(_argument_size(length) + length)
            payload = text.encode("utf-8")
            self.write_argument(3, len(payload))
            self.write_bytes(payload)
        elif value_type is CborArray:
            array = cast(CborArray, value)
            items = array.items
            if type(items) is not tuple or any(
                not _is_exact_cbor_value(item) for item in items
            ):
                raise TypeError("canonical-cbor: array items must be CborValue objects")
            self.ensure_fits(_argument_size(len(items)) + len(items))
            self.write_argument(4, len(items))
            for item in items:
                self.write_value(item, depth + 1)
        elif value_type is CborMap:
            self.write_map(cast(CborMap, value), depth)
        elif value_type is CborTag:
            tag = cast(CborTag, value)
            number = _require_u64(tag.number, "tag number")
            if not _is_exact_cbor_value(tag.value):
                raise TypeError("canonical-cbor: tag value must be a CborValue object")
            self.write_argument(6, number)
            self.write_value(tag.value, depth + 1)
        elif value_type is CborBoolean:
            boolean = cast(CborBoolean, value)
            if type(boolean.value) is not bool:
                raise TypeError("canonical-cbor: boolean value must be a boolean")
            self.write_byte(0xF5 if boolean.value else 0xF4)
        elif value_type is CborNull:
            self.write_byte(0xF6)
        else:
            raise TypeError("canonical-cbor: value must be an exact CborValue")

    def write_map(self, value: CborMap, depth: int) -> None:
        entries = value.entries
        if type(entries) is not tuple or any(
            type(entry) is not CborMapEntry for entry in entries
        ):
            raise TypeError("canonical-cbor: map requires CborMapEntry objects")
        self.ensure_fits(_argument_size(len(entries)) + len(entries) * 2)

        # Keys are encoded independently because the order is over complete wire
        # encodings, not over Python values.  Retained key bytes count against the
        # same 1 MiB budget before we allocate a large sorted staging set.
        staged: list[tuple[bytes, CborValue]] = []
        retained = 0
        for entry in entries:
            if not _is_exact_cbor_value(entry.key) or not _is_exact_cbor_value(
                entry.value
            ):
                raise TypeError(
                    "canonical-cbor: map entry values must be CborValue objects"
                )
            key_encoder = _Encoder()
            key_encoder.write_value(entry.key, depth + 1)
            key = bytes(key_encoder.output)
            retained += len(key)
            self.ensure_fits(_argument_size(len(entries)) + len(entries) + retained)
            staged.append((key, entry.value))

        staged.sort(key=lambda item: (len(item[0]), item[0]))
        if any(
            staged[index - 1][0] == staged[index][0]
            for index in range(1, len(staged))
        ):
            raise CborError("duplicate-map-key")

        self.write_argument(5, len(staged))
        for key, item in staged:
            self.write_bytes(key)
            self.write_value(item, depth + 1)


def encode_checked(value: CborValue) -> bytes:
    """Validate and encode one value without publishing partial bytes."""

    encoder = _Encoder()
    encoder.write_value(value, 0)
    return bytes(encoder.output)


def encode_into_checked(value: CborValue, destination: bytearray) -> None:
    """Append one complete encoding and restore the destination on host failure."""

    if type(destination) is not bytearray:
        raise TypeError("canonical-cbor: destination must be an exact bytearray")
    encoded = encode_checked(value)
    original_length = len(destination)
    try:
        destination.extend(encoded)
    except BaseException:
        del destination[original_length:]
        raise


class _Cursor:
    """A single strict parse over an immutable input copy."""

    def __init__(self, value: bytes) -> None:
        self.data = value
        self.position = 0

    @property
    def remaining(self) -> int:
        return len(self.data) - self.position

    def read_byte(self) -> int:
        if self.position >= len(self.data):
            raise CborError("unexpected-eof")
        value = self.data[self.position]
        self.position += 1
        return value

    def read_unsigned(self, width: int) -> int:
        value = 0
        for _ in range(width):
            value = (value << 8) | self.read_byte()
        return value

    def read_header(self) -> tuple[int, int, int]:
        initial = self.read_byte()
        major = initial >> 5
        info = initial & 0x1F
        if info <= 23:
            argument = info
        elif info == 24:
            argument = self.read_byte()
            if major != 7 and argument <= 23:
                raise CborError("non-minimal-integer")
        elif info == 25:
            argument = self.read_unsigned(2)
            if major != 7 and argument <= 0xFF:
                raise CborError("non-minimal-integer")
        elif info == 26:
            argument = self.read_unsigned(4)
            if major != 7 and argument <= 0xFFFF:
                raise CborError("non-minimal-integer")
        elif info == 27:
            argument = self.read_unsigned(8)
            if major != 7 and argument <= 0xFFFF_FFFF:
                raise CborError("non-minimal-integer")
        elif info <= 30:
            raise CborError("reserved")
        else:
            raise CborError("indefinite")
        return major, info, argument

    def checked_length(self, declared: int, minimum_bytes: int) -> int:
        if declared > self.remaining // minimum_bytes:
            raise CborError("length-too-large")
        return declared

    def read_bytes(self, length: int) -> bytes:
        # All current callers preflight with ``checked_length``.  Retain this
        # final guard so future internal callers cannot turn a short read into
        # a silently truncated value.
        if length > self.remaining:  # pragma: no cover - defensive backstop
            raise CborError("unexpected-eof")
        start = self.position
        self.position += length
        return self.data[start : self.position]

    def read_value(self, depth: int) -> Cbor:
        if depth > MAX_NESTING_DEPTH:
            raise CborError("too-deep")
        major, info, argument = self.read_header()
        if major == 0:
            return CborUnsigned(argument)
        if major == 1:
            return CborNegative(argument)
        if major == 2:
            return CborByteString(self.read_bytes(self.checked_length(argument, 1)))
        if major == 3:
            payload = self.read_bytes(self.checked_length(argument, 1))
            try:
                text = payload.decode("utf-8", "strict")
            except UnicodeDecodeError:
                pass
            else:
                return CborText(text)
            # Raise after leaving the exception handler so the rejected payload
            # is absent from both the cause and context chains.
            raise CborError("invalid-utf8")
        if major == 4:
            count = self.checked_length(argument, 1)
            return CborArray(tuple(self.read_value(depth + 1) for _ in range(count)))
        if major == 5:
            return self.read_map(self.checked_length(argument, 2), depth)
        if major == 6:
            return CborTag(argument, self.read_value(depth + 1))
        return self.read_simple(info)

    def read_map(self, count: int, depth: int) -> CborMap:
        entries: list[CborMapEntry] = []
        previous_key: bytes | None = None
        for _ in range(count):
            key_start = self.position
            key = self.read_value(depth + 1)
            encoded_key = self.data[key_start : self.position]
            if previous_key is not None and (len(previous_key), previous_key) >= (
                len(encoded_key),
                encoded_key,
            ):
                raise CborError("non-canonical-map-order")
            previous_key = encoded_key
            entries.append(CborMapEntry(key, self.read_value(depth + 1)))
        return CborMap(entries)

    @staticmethod
    def read_simple(info: int) -> Cbor:
        if info == 20:
            return CborBoolean(False)
        if info == 21:
            return CborBoolean(True)
        if info == 22:
            return NULL
        if info in {25, 26, 27}:
            raise CborError("float-not-supported")
        raise CborError("unsupported-simple")


def decode(value: bytes | bytearray | memoryview) -> Cbor:
    """Decode exactly one canonical item from a defensive input copy."""

    if not isinstance(value, bytes | bytearray | memoryview):
        raise TypeError("canonical-cbor: decode input must be bytes-like")
    cursor = _Cursor(bytes(value))
    result = cursor.read_value(0)
    if cursor.remaining != 0:
        raise CborError("trailing-bytes")
    return result


__all__ = [
    "ERROR_IDS",
    "MAX_ENCODED_BYTES",
    "MAX_NESTING_DEPTH",
    "NULL",
    "CborArray",
    "CborBoolean",
    "CborByteString",
    "CborError",
    "CborMap",
    "CborMapEntry",
    "CborNegative",
    "CborNull",
    "CborTag",
    "CborText",
    "CborUnsigned",
    "CborValue",
    "decode",
    "encode_checked",
    "encode_into_checked",
]
