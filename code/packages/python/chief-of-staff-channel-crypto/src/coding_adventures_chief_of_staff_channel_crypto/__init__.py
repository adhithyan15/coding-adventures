"""Portable D18F encrypted messages for Chief of Staff channels."""

from __future__ import annotations

import base64
import binascii
import json
import re
from collections.abc import Callable
from dataclasses import dataclass
from typing import Literal, NoReturn, Protocol

from coding_adventures_chacha20_poly1305 import (  # type: ignore[import-untyped]
    xchacha20_poly1305_aead_decrypt,
    xchacha20_poly1305_aead_encrypt,
)
from coding_adventures_ed25519 import sign, verify  # type: ignore[import-untyped]
from coding_adventures_sha256 import sha256
from coding_adventures_uuid import UUID  # type: ignore[import-untyped]

_MESSAGE_CONTEXT = b"chief-channel-message-v1"
_MESSAGE_MAGIC = b"D18M"
_WIRE_VERSION = 1
_MAX_IDENTITY_BYTES = 4 * 1024
_MAX_CONTENT_TYPE_BYTES = 1024
_MAX_CIPHERTEXT_BYTES = 64 * 1024 * 1024
_MAX_U64 = (1 << 64) - 1
_MAX_UUID_TIMESTAMP = (1 << 48) - 1
_RANDOM_MASK = (1 << 74) - 1
_DECIMAL_RE = re.compile(r"^(?:0|[1-9][0-9]*)$")
_HEX_RE = re.compile(r"^[0-9a-f]+$")
_JSON_FIELDS = (
    "record_type",
    "wire_version",
    "message_id",
    "timestamp_ns",
    "originator_id_b64",
    "channel_id",
    "sequence",
    "key_epoch",
    "content_type",
    "plaintext_hash_hex",
    "ciphertext_b64",
    "authentication_tag_b64",
    "originator_signature_b64",
)

MAX_MESSAGE_JSON_BYTES = 90 * 1024 * 1024
__version__ = "0.1.0"

MessageProfileErrorCode = Literal[
    "invalid_magic",
    "unsupported_version",
    "truncated_record",
    "trailing_bytes",
    "length_limit_exceeded",
    "invalid_utf8",
    "invalid_field",
    "invalid_json",
    "missing_epoch_key",
    "invalid_signature",
    "authentication_failed",
    "plaintext_hash_mismatch",
]


class MessageProfileError(ValueError):
    """One fail-closed D18F operation error with a stable portable code."""

    code: MessageProfileErrorCode

    def __init__(self, code: MessageProfileErrorCode) -> None:
        super().__init__(code)
        self.code = code


@dataclass(frozen=True, slots=True, init=False)
class MessageFields:
    """Immutable fields supplied before hashing, signing, and encryption."""

    message_id: bytes
    timestamp_ns: int
    originator_id: bytes
    channel_id: bytes
    sequence: int
    key_epoch: int
    content_type: str

    def __init__(
        self,
        message_id: bytes,
        timestamp_ns: int,
        originator_id: bytes,
        channel_id: bytes,
        sequence: int,
        key_epoch: int,
        content_type: str,
    ) -> None:
        message_id_copy = _copy_bytes(message_id)
        originator_id_copy = _copy_bytes(originator_id)
        channel_id_copy = _copy_bytes(channel_id)
        _require_length(message_id_copy, 16)
        _require_u64(timestamp_ns)
        if len(originator_id_copy) > _MAX_IDENTITY_BYTES:
            _fail("length_limit_exceeded")
        _require_length(channel_id_copy, 16)
        _require_u64(sequence)
        _require_u64(key_epoch)
        _content_type_bytes(content_type)
        object.__setattr__(self, "message_id", message_id_copy)
        object.__setattr__(self, "timestamp_ns", timestamp_ns)
        object.__setattr__(self, "originator_id", originator_id_copy)
        object.__setattr__(self, "channel_id", channel_id_copy)
        object.__setattr__(self, "sequence", sequence)
        object.__setattr__(self, "key_epoch", key_epoch)
        object.__setattr__(self, "content_type", content_type)


@dataclass(frozen=True, slots=True, init=False)
class SourcedMessageFields:
    """Immutable creation fields whose identifier and clock are injected."""

    originator_id: bytes
    channel_id: bytes
    sequence: int
    key_epoch: int
    content_type: str

    def __init__(
        self,
        originator_id: bytes,
        channel_id: bytes,
        sequence: int,
        key_epoch: int,
        content_type: str,
    ) -> None:
        originator_id_copy = _copy_bytes(originator_id)
        channel_id_copy = _copy_bytes(channel_id)
        if len(originator_id_copy) > _MAX_IDENTITY_BYTES:
            _fail("length_limit_exceeded")
        _require_length(channel_id_copy, 16)
        _require_u64(sequence)
        _require_u64(key_epoch)
        _content_type_bytes(content_type)
        object.__setattr__(self, "originator_id", originator_id_copy)
        object.__setattr__(self, "channel_id", channel_id_copy)
        object.__setattr__(self, "sequence", sequence)
        object.__setattr__(self, "key_epoch", key_epoch)
        object.__setattr__(self, "content_type", content_type)


@dataclass(frozen=True, slots=True, init=False)
class D18Message:
    """Complete structurally immutable D18F encrypted-message value."""

    message_id: bytes
    timestamp_ns: int
    originator_id: bytes
    channel_id: bytes
    sequence: int
    key_epoch: int
    content_type: str
    plaintext_hash: bytes
    ciphertext: bytes
    authentication_tag: bytes
    originator_signature: bytes

    def __init__(
        self,
        *,
        message_id: bytes,
        timestamp_ns: int,
        originator_id: bytes,
        channel_id: bytes,
        sequence: int,
        key_epoch: int,
        content_type: str,
        plaintext_hash: bytes,
        ciphertext: bytes,
        authentication_tag: bytes,
        originator_signature: bytes,
    ) -> None:
        fields = MessageFields(
            message_id,
            timestamp_ns,
            originator_id,
            channel_id,
            sequence,
            key_epoch,
            content_type,
        )
        content_type_bytes = _content_type_bytes(content_type)
        if len(content_type_bytes) > _MAX_CONTENT_TYPE_BYTES:
            _fail("length_limit_exceeded")
        plaintext_hash_copy = _copy_bytes(plaintext_hash)
        ciphertext_copy = _copy_bytes(ciphertext)
        authentication_tag_copy = _copy_bytes(authentication_tag)
        originator_signature_copy = _copy_bytes(originator_signature)
        _require_length(plaintext_hash_copy, 32)
        if len(ciphertext_copy) > _MAX_CIPHERTEXT_BYTES:
            _fail("length_limit_exceeded")
        _require_length(authentication_tag_copy, 16)
        _require_length(originator_signature_copy, 64)
        for name in (
            "message_id",
            "timestamp_ns",
            "originator_id",
            "channel_id",
            "sequence",
            "key_epoch",
            "content_type",
        ):
            object.__setattr__(self, name, getattr(fields, name))
        object.__setattr__(self, "plaintext_hash", plaintext_hash_copy)
        object.__setattr__(self, "ciphertext", ciphertext_copy)
        object.__setattr__(self, "authentication_tag", authentication_tag_copy)
        object.__setattr__(self, "originator_signature", originator_signature_copy)


class UuidV7Source(Protocol):
    """Injected UUID-v7 source used by convenience creation."""

    def next(self) -> bytes:
        """Return the next UUID-v7 bytes."""


class MonotonicNanosecondSource(Protocol):
    """Injected monotonic nanosecond clock used by convenience creation."""

    def now(self) -> int:
        """Return the current monotonic timestamp."""


class MonotonicUuidV7Generator:
    """Stateful RFC 9562 UUID-v7 generator with same-millisecond ordering."""

    __slots__ = ("_last_random", "_last_timestamp_ms")

    def __init__(self) -> None:
        self._last_timestamp_ms: int | None = None
        self._last_random = 0

    def next(self, timestamp_ms: int, entropy: bytes) -> bytes:
        """Generate a UUID-v7 from explicit time and ten entropy bytes."""
        entropy_copy = _copy_bytes(entropy)
        if (
            isinstance(timestamp_ms, bool)
            or not isinstance(timestamp_ms, int)
            or timestamp_ms < 0
            or timestamp_ms > _MAX_UUID_TIMESTAMP
            or len(entropy_copy) != 10
        ):
            _fail("invalid_field")
        supplied_random = int.from_bytes(entropy_copy, "big") & _RANDOM_MASK
        effective_timestamp = timestamp_ms
        random = supplied_random
        if (
            self._last_timestamp_ms is not None
            and timestamp_ms <= self._last_timestamp_ms
        ):
            effective_timestamp = self._last_timestamp_ms
            if self._last_random < _RANDOM_MASK:
                random = self._last_random + 1
            elif effective_timestamp < _MAX_UUID_TIMESTAMP:
                effective_timestamp += 1
                random = 0
            else:
                _fail("invalid_field")
        self._last_timestamp_ms = effective_timestamp
        self._last_random = random
        random_a = (random >> 62) & 0xFFF
        random_b = random & ((1 << 62) - 1)
        value = (
            (effective_timestamp << 80)
            | (7 << 76)
            | (random_a << 64)
            | (2 << 62)
            | random_b
        )
        return value.to_bytes(16, "big")


def validate_message_fields(fields: MessageFields) -> None:
    """Validate the high-level D18F creation and delivery rules."""
    _validate_uuid_v7(fields.message_id)
    _validate_uuid_v7(fields.channel_id)
    _require_u64(fields.timestamp_ns)
    _require_u64(fields.sequence)
    _require_u64(fields.key_epoch)
    if not fields.originator_id:
        _fail("invalid_field")
    if len(fields.originator_id) > _MAX_IDENTITY_BYTES:
        _fail("length_limit_exceeded")
    content_type_bytes = _content_type_bytes(fields.content_type)
    if len(content_type_bytes) > _MAX_CONTENT_TYPE_BYTES:
        _fail("length_limit_exceeded")
    _validate_mime(fields.content_type)


def message_create(
    fields: MessageFields,
    plaintext: bytes,
    signing_secret_key: bytes,
    channel_master_key: bytes,
) -> D18Message:
    """Validate, hash, sign, and encrypt one D18F message."""
    validate_message_fields(fields)
    plaintext_copy = _copy_bytes(plaintext)
    signing_key_copy = _copy_bytes(signing_secret_key)
    channel_key_copy = _copy_bytes(channel_master_key)
    if len(plaintext_copy) > _MAX_CIPHERTEXT_BYTES:
        _fail("length_limit_exceeded")
    _require_length(signing_key_copy, 64)
    _require_length(channel_key_copy, 32)
    plaintext_hash = sha256(plaintext_copy)
    header = _authenticated_header_from_fields(fields, plaintext_hash)
    ciphertext, authentication_tag = xchacha20_poly1305_aead_encrypt(
        plaintext_copy,
        channel_key_copy,
        _message_nonce(fields.channel_id, fields.sequence),
        header,
    )
    return D18Message(
        message_id=fields.message_id,
        timestamp_ns=fields.timestamp_ns,
        originator_id=fields.originator_id,
        channel_id=fields.channel_id,
        sequence=fields.sequence,
        key_epoch=fields.key_epoch,
        content_type=fields.content_type,
        plaintext_hash=plaintext_hash,
        ciphertext=ciphertext,
        authentication_tag=authentication_tag,
        originator_signature=sign(header, signing_key_copy),
    )


def message_create_with_sources(
    fields: SourcedMessageFields,
    plaintext: bytes,
    signing_secret_key: bytes,
    channel_master_key: bytes,
    uuid_source: UuidV7Source,
    clock: MonotonicNanosecondSource,
) -> D18Message:
    """Create a message with injected UUID-v7 and monotonic-clock sources."""
    return message_create(
        MessageFields(
            uuid_source.next(),
            clock.now(),
            fields.originator_id,
            fields.channel_id,
            fields.sequence,
            fields.key_epoch,
            fields.content_type,
        ),
        plaintext,
        signing_secret_key,
        channel_master_key,
    )


def message_verify(
    message: D18Message,
    originator_public_key: bytes,
    channel_master_key: bytes,
) -> bytes:
    """Verify and decrypt with an explicitly selected epoch key."""
    validate_message_fields(_message_fields(message))
    channel_key_copy = _copy_bytes(channel_master_key)
    _require_length(channel_key_copy, 32)
    return _verify_cryptography(message, originator_public_key, channel_key_copy)


def message_verify_with_key_resolver(
    message: D18Message,
    originator_public_key: bytes,
    key_for_epoch: Callable[[int], bytes | None],
) -> bytes:
    """Resolve the named epoch before signature and AEAD verification."""
    validate_message_fields(_message_fields(message))
    key = key_for_epoch(message.key_epoch)
    if key is None:
        _fail("missing_epoch_key")
    key_copy = _copy_bytes(key)
    _require_length(key_copy, 32)
    return _verify_cryptography(message, originator_public_key, key_copy)


def _verify_cryptography(
    message: D18Message,
    originator_public_key: bytes,
    channel_master_key: bytes,
) -> bytes:
    public_key_copy = _copy_bytes(originator_public_key)
    _require_length(public_key_copy, 32)
    header = message_authenticated_header(message)
    try:
        signature_valid = verify(
            header, message.originator_signature, public_key_copy
        )
    except (ArithmeticError, ValueError):
        signature_valid = False
    if not signature_valid:
        _fail("invalid_signature")
    try:
        plaintext = xchacha20_poly1305_aead_decrypt(
            message.ciphertext,
            channel_master_key,
            _message_nonce(message.channel_id, message.sequence),
            header,
            message.authentication_tag,
        )
    except (ArithmeticError, ValueError):
        _fail("authentication_failed")
    if not _equal_bytes(sha256(plaintext), message.plaintext_hash):
        _fail("plaintext_hash_mismatch")
    return plaintext


def message_authenticated_header(message: D18Message) -> bytes:
    """Return the exact D18F authenticated header."""
    return _authenticated_header_from_fields(
        _message_fields(message), message.plaintext_hash
    )


def message_serialize(message: D18Message) -> bytes:
    """Serialize one message as the unchanged D18M version 1 record."""
    content_type = _content_type_bytes(message.content_type)
    if (
        len(message.originator_id) > _MAX_IDENTITY_BYTES
        or len(content_type) > _MAX_CONTENT_TYPE_BYTES
        or len(message.ciphertext) > _MAX_CIPHERTEXT_BYTES
    ):
        _fail("length_limit_exceeded")
    return b"".join(
        (
            _MESSAGE_MAGIC,
            bytes((_WIRE_VERSION,)),
            message.message_id,
            _u64be(message.timestamp_ns),
            len(message.originator_id).to_bytes(4, "big"),
            message.originator_id,
            message.channel_id,
            _u64be(message.sequence),
            _u64be(message.key_epoch),
            len(content_type).to_bytes(4, "big"),
            content_type,
            message.plaintext_hash,
            _u64be(len(message.ciphertext)),
            message.ciphertext,
            message.authentication_tag,
            message.originator_signature,
        )
    )


def message_deserialize(data: bytes) -> D18Message:
    """Structurally decode one D18M version 1 binary record."""
    decoder = _Decoder(data)
    if decoder.take(4) != _MESSAGE_MAGIC:
        _fail("invalid_magic")
    if decoder.take(1)[0] != _WIRE_VERSION:
        _fail("unsupported_version")
    message_id = decoder.take(16)
    timestamp_ns = decoder.read_u64()
    originator_id = decoder.read_bounded_u32(_MAX_IDENTITY_BYTES)
    channel_id = decoder.take(16)
    sequence = decoder.read_u64()
    key_epoch = decoder.read_u64()
    content_type_bytes = decoder.read_bounded_u32(_MAX_CONTENT_TYPE_BYTES)
    try:
        content_type = content_type_bytes.decode("utf-8", errors="strict")
    except UnicodeDecodeError:
        _fail("invalid_utf8")
    plaintext_hash = decoder.take(32)
    ciphertext = decoder.read_bounded_u64(_MAX_CIPHERTEXT_BYTES)
    authentication_tag = decoder.take(16)
    originator_signature = decoder.take(64)
    decoder.finish()
    return D18Message(
        message_id=message_id,
        timestamp_ns=timestamp_ns,
        originator_id=originator_id,
        channel_id=channel_id,
        sequence=sequence,
        key_epoch=key_epoch,
        content_type=content_type,
        plaintext_hash=plaintext_hash,
        ciphertext=ciphertext,
        authentication_tag=authentication_tag,
        originator_signature=originator_signature,
    )


def message_to_json(message: D18Message) -> bytes:
    """Encode one message as canonical, lossless D18F JSON bytes."""
    value = {
        "record_type": "D18M",
        "wire_version": 1,
        "message_id": _uuid_string(message.message_id),
        "timestamp_ns": str(message.timestamp_ns),
        "originator_id_b64": _encode_base64(message.originator_id),
        "channel_id": _uuid_string(message.channel_id),
        "sequence": str(message.sequence),
        "key_epoch": str(message.key_epoch),
        "content_type": message.content_type,
        "plaintext_hash_hex": message.plaintext_hash.hex(),
        "ciphertext_b64": _encode_base64(message.ciphertext),
        "authentication_tag_b64": _encode_base64(message.authentication_tag),
        "originator_signature_b64": _encode_base64(message.originator_signature),
    }
    try:
        encoded = json.dumps(
            value, ensure_ascii=False, separators=(",", ":")
        ).encode("utf-8", errors="strict")
    except (TypeError, UnicodeEncodeError, ValueError):
        _fail("invalid_field")
    if len(encoded) > MAX_MESSAGE_JSON_BYTES:
        _fail("length_limit_exceeded")
    return encoded


def message_from_json(data: bytes) -> D18Message:
    """Structurally decode lossless D18F JSON into an immutable message."""
    if len(data) > MAX_MESSAGE_JSON_BYTES:
        _fail("length_limit_exceeded")
    try:
        text = bytes(data).decode("utf-8", errors="strict")
        value = json.loads(
            text,
            object_pairs_hook=_strict_json_object,
            parse_constant=_reject_json_constant,
        )
    except (TypeError, ValueError, UnicodeDecodeError, json.JSONDecodeError):
        _fail("invalid_json")
    if not isinstance(value, dict):
        _fail("invalid_json")
    if len(value) != len(_JSON_FIELDS) or set(value) != set(_JSON_FIELDS):
        _fail("invalid_json")
    record_type = _string_field(value, "record_type")
    if record_type != "D18M":
        _fail("invalid_magic")
    wire_version = value["wire_version"]
    if isinstance(wire_version, bool) or not isinstance(wire_version, (int, float)):
        _fail("invalid_json")
    if not isinstance(wire_version, int) or wire_version != 1:
        _fail("unsupported_version")
    message_id = _decode_uuid_v7(_string_field(value, "message_id"))
    timestamp_ns = _decode_decimal(_string_field(value, "timestamp_ns"))
    originator_id = _decode_base64(
        _string_field(value, "originator_id_b64"), _MAX_IDENTITY_BYTES
    )
    channel_id = _decode_uuid_v7(_string_field(value, "channel_id"))
    sequence = _decode_decimal(_string_field(value, "sequence"))
    key_epoch = _decode_decimal(_string_field(value, "key_epoch"))
    content_type = _string_field(value, "content_type")
    if len(_content_type_bytes(content_type)) > _MAX_CONTENT_TYPE_BYTES:
        _fail("length_limit_exceeded")
    plaintext_hash = _decode_hex(
        _string_field(value, "plaintext_hash_hex"), 32
    )
    ciphertext = _decode_base64(
        _string_field(value, "ciphertext_b64"), _MAX_CIPHERTEXT_BYTES
    )
    authentication_tag = _decode_base64_exact(
        _string_field(value, "authentication_tag_b64"), 16
    )
    originator_signature = _decode_base64_exact(
        _string_field(value, "originator_signature_b64"), 64
    )
    return D18Message(
        message_id=message_id,
        timestamp_ns=timestamp_ns,
        originator_id=originator_id,
        channel_id=channel_id,
        sequence=sequence,
        key_epoch=key_epoch,
        content_type=content_type,
        plaintext_hash=plaintext_hash,
        ciphertext=ciphertext,
        authentication_tag=authentication_tag,
        originator_signature=originator_signature,
    )


def _authenticated_header_from_fields(
    fields: MessageFields, plaintext_hash: bytes
) -> bytes:
    return _frame(
        (
            _MESSAGE_CONTEXT,
            fields.message_id,
            _u64be(fields.timestamp_ns),
            fields.originator_id,
            fields.channel_id,
            _u64be(fields.sequence),
            _u64be(fields.key_epoch),
            _content_type_bytes(fields.content_type),
            plaintext_hash,
        )
    )


def _message_nonce(channel_id: bytes, sequence: int) -> bytes:
    _require_length(channel_id, 16)
    return channel_id + _u64be(sequence)


def _frame(fields: tuple[bytes, ...]) -> bytes:
    return b"".join(len(field).to_bytes(8, "big") + field for field in fields)


def _message_fields(message: D18Message) -> MessageFields:
    return MessageFields(
        message.message_id,
        message.timestamp_ns,
        message.originator_id,
        message.channel_id,
        message.sequence,
        message.key_epoch,
        message.content_type,
    )


def _validate_uuid_v7(value: bytes) -> None:
    _require_length(value, 16)
    if value[6] >> 4 != 7 or value[8] & 0xC0 != 0x80:
        _fail("invalid_field")


def _decode_uuid_v7(value: str) -> bytes:
    try:
        uuid = UUID(value)
    except (TypeError, ValueError):
        _fail("invalid_field")
    if str(uuid) != value:
        _fail("invalid_field")
    result = uuid.bytes
    _validate_uuid_v7(result)
    return result


def _uuid_string(value: bytes) -> str:
    try:
        return str(UUID(value))
    except (TypeError, ValueError):
        _fail("invalid_field")


def _decode_decimal(value: str) -> int:
    if _DECIMAL_RE.fullmatch(value) is None:
        _fail("invalid_field")
    decoded = int(value)
    if decoded > _MAX_U64:
        _fail("invalid_field")
    return decoded


def _validate_mime(value: str) -> None:
    encoded = _content_type_bytes(value)
    if not encoded or any(byte < 0x20 or byte > 0x7E for byte in encoded):
        _fail("invalid_field")
    index = _consume_token(encoded, 0)
    if index >= len(encoded) or encoded[index] != ord("/"):
        _fail("invalid_field")
    index = _consume_token(encoded, index + 1)
    while index < len(encoded):
        index = _consume_spaces(encoded, index)
        if index >= len(encoded) or encoded[index] != ord(";"):
            _fail("invalid_field")
        index = _consume_spaces(encoded, index + 1)
        index = _consume_token(encoded, index)
        index = _consume_spaces(encoded, index)
        if index >= len(encoded) or encoded[index] != ord("="):
            _fail("invalid_field")
        index = _consume_spaces(encoded, index + 1)
        if index < len(encoded) and encoded[index] == ord('"'):
            index += 1
            while True:
                if index >= len(encoded):
                    _fail("invalid_field")
                if encoded[index] == ord('"'):
                    index += 1
                    break
                if encoded[index] == ord("\\"):
                    index += 1
                    if index >= len(encoded):
                        _fail("invalid_field")
                index += 1
        else:
            index = _consume_token(encoded, index)


def _consume_token(value: bytes, index: int) -> int:
    start = index
    while index < len(value) and _is_mime_token(value[index]):
        index += 1
    if index == start:
        _fail("invalid_field")
    return index


def _consume_spaces(value: bytes, index: int) -> int:
    while index < len(value) and value[index] == 0x20:
        index += 1
    return index


def _is_mime_token(byte: int) -> bool:
    return (
        ord("0") <= byte <= ord("9")
        or ord("A") <= byte <= ord("Z")
        or ord("a") <= byte <= ord("z")
        or byte in b"!#$%&'*+-.^_`|~"
    )


def _encode_base64(value: bytes) -> str:
    return base64.b64encode(value).decode("ascii")


def _decode_base64(value: str, maximum: int) -> bytes:
    if len(value) % 4 != 0:
        _fail("invalid_field")
    if len(value) // 4 * 3 > maximum + 2:
        _fail("length_limit_exceeded")
    try:
        decoded = base64.b64decode(value, validate=True)
    except (binascii.Error, ValueError):
        _fail("invalid_field")
    if len(decoded) > maximum:
        _fail("length_limit_exceeded")
    if _encode_base64(decoded) != value:
        _fail("invalid_field")
    return decoded


def _decode_base64_exact(value: str, length: int) -> bytes:
    decoded = _decode_base64(value, length)
    if len(decoded) != length:
        _fail("invalid_field")
    return decoded


def _decode_hex(value: str, length: int) -> bytes:
    if len(value) != length * 2 or _HEX_RE.fullmatch(value) is None:
        _fail("invalid_field")
    return bytes.fromhex(value)


class _DuplicateJsonKey(ValueError):
    pass


def _strict_json_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            raise _DuplicateJsonKey(key)
        result[key] = value
    return result


def _reject_json_constant(value: str) -> NoReturn:
    raise ValueError(value)


def _string_field(value: dict[str, object], name: str) -> str:
    field = value[name]
    if not isinstance(field, str):
        _fail("invalid_json")
    return field


def _content_type_bytes(value: str) -> bytes:
    if not isinstance(value, str):
        _fail("invalid_field")
    try:
        return value.encode("utf-8", errors="strict")
    except UnicodeEncodeError:
        _fail("invalid_field")


def _copy_bytes(value: bytes) -> bytes:
    if not isinstance(value, (bytes, bytearray, memoryview)):
        _fail("invalid_field")
    try:
        return bytes(value)
    except (TypeError, ValueError):
        _fail("invalid_field")


def _u64be(value: int) -> bytes:
    _require_u64(value)
    return value.to_bytes(8, "big")


def _require_u64(value: int) -> None:
    if (
        isinstance(value, bool)
        or not isinstance(value, int)
        or value < 0
        or value > _MAX_U64
    ):
        _fail("invalid_field")


def _require_length(value: bytes, length: int) -> None:
    if len(value) != length:
        _fail("invalid_field")


def _equal_bytes(left: bytes, right: bytes) -> bool:
    if len(left) != len(right):
        return False
    difference = 0
    for left_byte, right_byte in zip(left, right, strict=True):
        difference |= left_byte ^ right_byte
    return difference == 0


def _fail(code: MessageProfileErrorCode) -> NoReturn:
    raise MessageProfileError(code)


class _Decoder:
    __slots__ = ("_data", "_position")

    def __init__(self, data: bytes) -> None:
        self._data = _copy_bytes(data)
        self._position = 0

    def take(self, length: int) -> bytes:
        end = self._position + length
        if length < 0 or end > len(self._data):
            _fail("truncated_record")
        result = self._data[self._position : end]
        self._position = end
        return result

    def read_u64(self) -> int:
        return int.from_bytes(self.take(8), "big")

    def read_bounded_u32(self, maximum: int) -> bytes:
        return self._read_bounded(int.from_bytes(self.take(4), "big"), maximum)

    def read_bounded_u64(self, maximum: int) -> bytes:
        return self._read_bounded(self.read_u64(), maximum)

    def _read_bounded(self, length: int, maximum: int) -> bytes:
        if length > maximum:
            _fail("length_limit_exceeded")
        return self.take(length)

    def finish(self) -> None:
        if self._position != len(self._data):
            _fail("trailing_bytes")


__all__ = [
    "D18Message",
    "MAX_MESSAGE_JSON_BYTES",
    "MessageFields",
    "MessageProfileError",
    "MessageProfileErrorCode",
    "MonotonicNanosecondSource",
    "MonotonicUuidV7Generator",
    "SourcedMessageFields",
    "UuidV7Source",
    "message_authenticated_header",
    "message_create",
    "message_create_with_sources",
    "message_deserialize",
    "message_from_json",
    "message_serialize",
    "message_to_json",
    "message_verify",
    "message_verify_with_key_resolver",
    "validate_message_fields",
]
