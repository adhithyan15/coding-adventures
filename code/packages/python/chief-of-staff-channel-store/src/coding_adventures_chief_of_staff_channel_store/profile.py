"""Portable D18P durable-channel values, codecs, keys, and stable failures."""

from __future__ import annotations

import builtins
from dataclasses import dataclass
from typing import Literal, NoReturn

from coding_adventures_sha256 import sha256

CHANNEL_STORAGE_NAMESPACE = "chief-channels"
CHANNEL_DEFINITION_CONTENT_TYPE = (
    "application/vnd.coding-adventures.chief-channel-definition-v1"
)
CHANNEL_STATE_CONTENT_TYPE = "application/vnd.coding-adventures.chief-channel-state-v1"
CHANNEL_MESSAGE_CONTENT_TYPE = (
    "application/vnd.coding-adventures.chief-channel-message-v1"
)
CHANNEL_GRANT_CONTENT_TYPE = (
    "application/vnd.coding-adventures.chief-channel-key-grant-v1"
)
CHANNEL_ACK_CONTENT_TYPE = "application/vnd.coding-adventures.chief-channel-ack-v1"
MAX_IDENTITY_BYTES = 4 * 1024
MAX_CONTENT_TYPE_BYTES = 1024
MAX_CHANNEL_RECEIVERS = 1024
MAX_PENDING_HEADER_BYTES = 16 * 1024
MAX_CHANNEL_CAS_ATTEMPTS = 16
MAX_DEFINITION_CAS_ATTEMPTS = 16
MAX_U64 = (1 << 64) - 1

CHANNEL_ERROR_CODES = (
    "invalid_definition",
    "invalid_message_id",
    "definition_not_found",
    "conflicting_definition",
    "corrupt_definition",
    "definition_changed",
    "channel_destroyed",
    "unauthorized_originator",
    "unauthorized_receiver",
    "public_key_mismatch",
    "missing_key_grant",
    "unknown_message_id",
    "unauthorized_message",
    "not_initialized",
    "corrupt_record",
    "pending_append",
    "no_pending_append",
    "pending_header_mismatch",
    "conflicting_record",
    "concurrent_update",
    "invalid_receiver_id",
    "invalid_page_size",
    "acknowledgement_regression",
    "acknowledgement_ahead",
    "acknowledgement_pending",
    "sequence_exhausted",
    "storage_error",
    "wire_error",
    "crypto_error",
    "metadata_error",
)

ChannelErrorCode = Literal[
    "invalid_definition",
    "invalid_message_id",
    "definition_not_found",
    "conflicting_definition",
    "corrupt_definition",
    "definition_changed",
    "channel_destroyed",
    "unauthorized_originator",
    "unauthorized_receiver",
    "public_key_mismatch",
    "missing_key_grant",
    "unknown_message_id",
    "unauthorized_message",
    "not_initialized",
    "corrupt_record",
    "pending_append",
    "no_pending_append",
    "pending_header_mismatch",
    "conflicting_record",
    "concurrent_update",
    "invalid_receiver_id",
    "invalid_page_size",
    "acknowledgement_regression",
    "acknowledgement_ahead",
    "acknowledgement_pending",
    "sequence_exhausted",
    "storage_error",
    "wire_error",
    "crypto_error",
    "metadata_error",
]
ChannelLifecycle = Literal["active", "destroyed"]


class ChannelProfileError(ValueError):
    """One fail-closed D18P operation error with a stable portable code."""

    code: ChannelErrorCode

    def __init__(self, code: ChannelErrorCode) -> None:
        super().__init__(code)
        self.code = code


@dataclass(frozen=True, slots=True)
class OriginatorIdentity:
    """The single durable writer identity."""

    agent_id: bytes
    public_key: bytes

    def __post_init__(self) -> None:
        object.__setattr__(self, "agent_id", bytes(self.agent_id))
        object.__setattr__(self, "public_key", bytes(self.public_key))


@dataclass(frozen=True, slots=True)
class ReceiverIdentity:
    """One durable read-only member identity."""

    agent_id: bytes
    public_key: bytes

    def __post_init__(self) -> None:
        object.__setattr__(self, "agent_id", bytes(self.agent_id))
        object.__setattr__(self, "public_key", bytes(self.public_key))


@dataclass(frozen=True, slots=True)
class ChannelDefinition:
    """Immutable canonical D18C channel membership."""

    channel_id: bytes
    originator: OriginatorIdentity
    receivers: tuple[ReceiverIdentity, ...]
    created_at_ns: int
    key_epoch: int
    lifecycle: ChannelLifecycle = "active"

    def __post_init__(self) -> None:
        channel_id = bytes(self.channel_id)
        originator = OriginatorIdentity(
            self.originator.agent_id, self.originator.public_key
        )
        receivers = tuple(
            sorted(
                (
                    ReceiverIdentity(receiver.agent_id, receiver.public_key)
                    for receiver in self.receivers
                ),
                key=lambda receiver: receiver.agent_id,
            )
        )
        validate_uuid_v7(channel_id, "invalid_definition")
        validate_agent_id(originator.agent_id, "invalid_definition")
        _require_length(originator.public_key, 32, "invalid_definition")
        if not 1 <= len(receivers) <= MAX_CHANNEL_RECEIVERS:
            _fail("invalid_definition")
        _require_u64(self.created_at_ns, "invalid_definition")
        _require_u64(self.key_epoch, "invalid_definition")
        if self.lifecycle not in ("active", "destroyed"):
            _fail("invalid_definition")
        previous: bytes | None = None
        for receiver in receivers:
            validate_agent_id(receiver.agent_id, "invalid_definition")
            _require_length(receiver.public_key, 32, "invalid_definition")
            if (
                receiver.agent_id == originator.agent_id
                or receiver.agent_id == previous
            ):
                _fail("invalid_definition")
            previous = receiver.agent_id
        object.__setattr__(self, "channel_id", channel_id)
        object.__setattr__(self, "originator", originator)
        object.__setattr__(self, "receivers", receivers)

    def receiver(self, agent_id: bytes) -> ReceiverIdentity | None:
        """Return one defensively immutable member value."""
        return next(
            (receiver for receiver in self.receivers if receiver.agent_id == agent_id),
            None,
        )

    def with_lifecycle(self, lifecycle: ChannelLifecycle) -> ChannelDefinition:
        """Return the same definition with a new lifecycle."""
        return ChannelDefinition(
            self.channel_id,
            self.originator,
            self.receivers,
            self.created_at_ns,
            self.key_epoch,
            lifecycle,
        )


@dataclass(frozen=True, slots=True)
class MessageHeader:
    """Exact D18H value consumed by reserve-before-encrypt recovery."""

    message_id: bytes
    timestamp_ns: int
    originator_id: bytes
    channel_id: bytes
    sequence: int
    key_epoch: int
    content_type: str
    plaintext_hash: bytes

    def __post_init__(self) -> None:
        message_id = bytes(self.message_id)
        originator_id = bytes(self.originator_id)
        channel_id = bytes(self.channel_id)
        plaintext_hash = bytes(self.plaintext_hash)
        _require_length(message_id, 16, "wire_error")
        _require_u64(self.timestamp_ns, "wire_error")
        if len(originator_id) > MAX_IDENTITY_BYTES:
            _fail("wire_error")
        _require_length(channel_id, 16, "wire_error")
        _require_u64(self.sequence, "wire_error")
        _require_u64(self.key_epoch, "wire_error")
        if len(_encode_utf8(self.content_type, "wire_error")) > MAX_CONTENT_TYPE_BYTES:
            _fail("wire_error")
        _require_length(plaintext_hash, 32, "wire_error")
        object.__setattr__(self, "message_id", message_id)
        object.__setattr__(self, "originator_id", originator_id)
        object.__setattr__(self, "channel_id", channel_id)
        object.__setattr__(self, "plaintext_hash", plaintext_hash)


@dataclass(frozen=True, slots=True)
class ChannelState:
    """Durable next sequence and optional reserved header."""

    next_sequence: int
    pending_header: MessageHeader | None = None


def channel_definition_serialize(definition: ChannelDefinition) -> bytes:
    """Encode one definition as exact D18C version 1 bytes."""
    writer = _Writer()
    writer.bytes(b"D18C").u8(1).bytes(definition.channel_id)
    writer.sized32(definition.originator.agent_id).bytes(
        definition.originator.public_key
    )
    writer.u32(len(definition.receivers))
    for receiver in definition.receivers:
        writer.sized32(receiver.agent_id).bytes(receiver.public_key)
    writer.u64(definition.created_at_ns).u64(definition.key_epoch)
    writer.u8(0 if definition.lifecycle == "active" else 1)
    return writer.finish()


def channel_definition_deserialize(data: bytes) -> ChannelDefinition:
    """Decode one fail-closed D18C record."""
    try:
        reader = _Reader(data, "corrupt_definition")
        reader.magic(b"D18C").version()
        channel_id = reader.bytes(16)
        originator = OriginatorIdentity(
            reader.sized32(MAX_IDENTITY_BYTES), reader.bytes(32)
        )
        receiver_count = reader.u32()
        if not 1 <= receiver_count <= MAX_CHANNEL_RECEIVERS:
            _fail("corrupt_definition")
        receivers = tuple(
            ReceiverIdentity(reader.sized32(MAX_IDENTITY_BYTES), reader.bytes(32))
            for _ in range(receiver_count)
        )
        created_at_ns = reader.u64()
        key_epoch = reader.u64()
        lifecycle_byte = reader.u8()
        if lifecycle_byte not in (0, 1):
            _fail("corrupt_definition")
        reader.finish()
        return ChannelDefinition(
            channel_id,
            originator,
            receivers,
            created_at_ns,
            key_epoch,
            "active" if lifecycle_byte == 0 else "destroyed",
        )
    except Exception as error:
        _remap(error, "corrupt_definition")


def message_header_serialize(header: MessageHeader) -> bytes:
    """Encode one reservation as exact D18H version 1 bytes."""
    writer = _Writer()
    writer.bytes(b"D18H").u8(1).bytes(header.message_id).u64(header.timestamp_ns)
    writer.sized32(header.originator_id).bytes(header.channel_id)
    writer.u64(header.sequence).u64(header.key_epoch)
    writer.sized32(_encode_utf8(header.content_type, "wire_error"))
    writer.bytes(header.plaintext_hash)
    return writer.finish()


def message_header_deserialize(data: bytes) -> MessageHeader:
    """Decode one D18H record."""
    reader = _Reader(data, "wire_error")
    reader.magic(b"D18H").version()
    message_id = reader.bytes(16)
    timestamp_ns = reader.u64()
    originator_id = reader.sized32(MAX_IDENTITY_BYTES)
    channel_id = reader.bytes(16)
    sequence = reader.u64()
    key_epoch = reader.u64()
    try:
        content_type = reader.sized32(MAX_CONTENT_TYPE_BYTES).decode(
            "utf-8", errors="strict"
        )
    except UnicodeDecodeError:
        _fail("wire_error")
    plaintext_hash = reader.bytes(32)
    reader.finish()
    return MessageHeader(
        message_id,
        timestamp_ns,
        originator_id,
        channel_id,
        sequence,
        key_epoch,
        content_type,
        plaintext_hash,
    )


def channel_state_serialize(state: ChannelState) -> bytes:
    """Encode one D18S record."""
    _require_u64(state.next_sequence, "corrupt_record")
    writer = _Writer().bytes(b"D18S").u8(1).u64(state.next_sequence)
    if state.pending_header is None:
        writer.u8(0)
    else:
        header = message_header_serialize(state.pending_header)
        if len(header) > MAX_PENDING_HEADER_BYTES:
            _fail("corrupt_record")
        writer.u8(1).u32(len(header)).bytes(header)
    return writer.finish()


def channel_state_deserialize(data: bytes, channel_id: bytes) -> ChannelState:
    """Decode one fail-closed D18S record."""
    try:
        reader = _Reader(data, "corrupt_record")
        reader.magic(b"D18S").version()
        next_sequence = reader.u64()
        flag = reader.u8()
        pending: MessageHeader | None = None
        if flag == 0:
            reader.finish()
        elif flag == 1:
            length = reader.u32()
            if length > MAX_PENDING_HEADER_BYTES:
                _fail("corrupt_record")
            try:
                pending = message_header_deserialize(reader.bytes(length))
            except Exception:
                _fail("corrupt_record")
            reader.finish()
            if (
                pending.channel_id != channel_id
                or pending.sequence == MAX_U64
                or pending.sequence + 1 != next_sequence
            ):
                _fail("corrupt_record")
        else:
            _fail("corrupt_record")
        return ChannelState(next_sequence, pending)
    except Exception as error:
        _remap(error, "corrupt_record")


def receiver_cursor_serialize(first_unread_sequence: int) -> bytes:
    """Encode one D18A receiver cursor."""
    _require_u64(first_unread_sequence, "corrupt_record")
    return _Writer().bytes(b"D18A").u8(1).u64(first_unread_sequence).finish()


def receiver_cursor_deserialize(data: bytes) -> int:
    """Decode one fail-closed D18A cursor."""
    try:
        reader = _Reader(data, "corrupt_record")
        reader.magic(b"D18A").version()
        cursor = reader.u64()
        reader.finish()
        return cursor
    except Exception as error:
        _remap(error, "corrupt_record")


def channel_definition_record_key(channel_id: bytes) -> str:
    """Return the canonical definition key."""
    _require_length(channel_id, 16, "invalid_definition")
    return f"{channel_id.hex()}/definition"


def sequence_state_record_key(channel_id: bytes) -> str:
    """Return the canonical sequence-state key."""
    _require_length(channel_id, 16, "invalid_definition")
    return f"{channel_id.hex()}/state/next-sequence"


def message_record_prefix(channel_id: bytes) -> str:
    """Return the canonical message prefix."""
    _require_length(channel_id, 16, "invalid_definition")
    return f"{channel_id.hex()}/messages/"


def message_record_key(channel_id: bytes, sequence: int) -> str:
    """Return one fixed-width message key."""
    return f"{message_record_prefix(channel_id)}{_decimal20(sequence)}"


def key_grant_record_key(channel_id: bytes, key_epoch: int, receiver_id: bytes) -> str:
    """Return one receiver-bound opaque grant key."""
    validate_agent_id(receiver_id, "invalid_receiver_id")
    return (
        f"{channel_id.hex()}/grants/{_decimal20(key_epoch)}/{sha256(receiver_id).hex()}"
    )


def receiver_ack_record_key(channel_id: bytes, receiver_id: bytes) -> str:
    """Return one receiver cursor key."""
    validate_agent_id(receiver_id, "invalid_receiver_id")
    return f"{channel_id.hex()}/receivers/{sha256(receiver_id).hex()}/ack"


def validate_uuid_v7(
    value: bytes, code: ChannelErrorCode = "invalid_message_id"
) -> None:
    """Require a 16-byte RFC-variant UUID-v7 value."""
    _require_length(value, 16, code)
    if value[6] >> 4 != 7 or value[8] & 0xC0 != 0x80:
        _fail(code)


def validate_agent_id(value: bytes, code: ChannelErrorCode) -> None:
    """Require one non-empty bounded agent identifier."""
    if not value or len(value) > MAX_IDENTITY_BYTES:
        _fail(code)


def _fail(code: ChannelErrorCode) -> NoReturn:
    raise ChannelProfileError(code)


def _require_length(value: bytes, length: int, code: ChannelErrorCode) -> None:
    if len(value) != length:
        _fail(code)


def _require_u64(value: int, code: ChannelErrorCode) -> None:
    if (
        isinstance(value, bool)
        or not isinstance(value, int)
        or not 0 <= value <= MAX_U64
    ):
        _fail(code)


def _decimal20(value: int) -> str:
    _require_u64(value, "corrupt_record")
    return str(value).zfill(20)


def _encode_utf8(value: str, code: ChannelErrorCode) -> bytes:
    try:
        return value.encode("utf-8", errors="strict")
    except (AttributeError, UnicodeEncodeError):
        _fail(code)


def _remap(error: Exception, code: ChannelErrorCode) -> NoReturn:
    if isinstance(error, ChannelProfileError) and error.code == code:
        raise error
    _fail(code)


class _Writer:
    __slots__ = ("_value",)

    def __init__(self) -> None:
        self._value = bytearray()

    def bytes(self, value: builtins.bytes) -> _Writer:
        self._value.extend(value)
        return self

    def u8(self, value: int) -> _Writer:
        self._value.append(value & 0xFF)
        return self

    def u32(self, value: int) -> _Writer:
        if (
            isinstance(value, bool)
            or not isinstance(value, int)
            or not 0 <= value <= 0xFFFFFFFF
        ):
            _fail("wire_error")
        self._value.extend(value.to_bytes(4, "big"))
        return self

    def u64(self, value: int) -> _Writer:
        _require_u64(value, "wire_error")
        self._value.extend(value.to_bytes(8, "big"))
        return self

    def sized32(self, value: builtins.bytes) -> _Writer:
        return self.u32(len(value)).bytes(value)

    def finish(self) -> builtins.bytes:
        return bytes(self._value)


class _Reader:
    __slots__ = ("_code", "_data", "_position")

    def __init__(self, data: builtins.bytes, code: ChannelErrorCode) -> None:
        self._data = bytes(data)
        self._code = code
        self._position = 0

    def bytes(self, length: int) -> builtins.bytes:
        if length < 0 or self._position + length > len(self._data):
            _fail(self._code)
        value = self._data[self._position : self._position + length]
        self._position += length
        return value

    def u8(self) -> int:
        return self.bytes(1)[0]

    def u32(self) -> int:
        return int.from_bytes(self.bytes(4), "big")

    def u64(self) -> int:
        return int.from_bytes(self.bytes(8), "big")

    def sized32(self, maximum: int) -> builtins.bytes:
        length = self.u32()
        if length > maximum:
            _fail(self._code)
        return self.bytes(length)

    def magic(self, expected: builtins.bytes) -> _Reader:
        if self.bytes(4) != expected:
            _fail(self._code)
        return self

    def version(self) -> _Reader:
        if self.u8() != 1:
            _fail(self._code)
        return self

    def finish(self) -> None:
        if self._position != len(self._data):
            _fail(self._code)
