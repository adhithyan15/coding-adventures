"""Injected atomic storage and the authority-free D18P durable channel store."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass, field
from typing import Protocol

from coding_adventures_chief_of_staff_channel_crypto import (
    D18Message,
    MessageFields,
    MessageProfileError,
    message_create,
    message_deserialize,
    message_serialize,
    validate_message_fields,
)
from coding_adventures_sha256 import sha256

from .profile import (
    CHANNEL_ACK_CONTENT_TYPE,
    CHANNEL_GRANT_CONTENT_TYPE,
    CHANNEL_MESSAGE_CONTENT_TYPE,
    CHANNEL_STATE_CONTENT_TYPE,
    CHANNEL_STORAGE_NAMESPACE,
    MAX_CHANNEL_CAS_ATTEMPTS,
    MAX_U64,
    ChannelProfileError,
    ChannelState,
    MessageHeader,
    _fail,
    channel_state_deserialize,
    channel_state_serialize,
    key_grant_record_key,
    message_record_key,
    message_record_prefix,
    receiver_ack_record_key,
    receiver_cursor_deserialize,
    receiver_cursor_serialize,
    sequence_state_record_key,
    validate_agent_id,
    validate_uuid_v7,
)


@dataclass(frozen=True, slots=True)
class StorageRecord:
    """One immutable backend record."""

    namespace: str
    key: str
    content_type: str
    body: bytes
    revision: str
    metadata: dict[str, object] = field(default_factory=dict)

    def __post_init__(self) -> None:
        object.__setattr__(self, "body", bytes(self.body))
        object.__setattr__(self, "metadata", dict(self.metadata))


@dataclass(frozen=True, slots=True)
class StoragePut:
    """One conditional backend write."""

    namespace: str
    key: str
    content_type: str
    body: bytes
    metadata: dict[str, object] = field(default_factory=dict)
    if_absent: bool = False
    if_revision: str | None = None


@dataclass(frozen=True, slots=True)
class StoragePage:
    """One lexicographically ordered backend page."""

    records: tuple[StorageRecord, ...]
    next_cursor: str | None = None


class ChannelStorageBackend(Protocol):
    """Minimal atomic backend required by D18P."""

    def initialize(self) -> None:
        """Prepare the backend for use."""

    def get(self, namespace: str, key: str) -> StorageRecord | None:
        """Load one record."""

    def put(self, value: StoragePut) -> StorageRecord:
        """Perform one atomic create or revision-CAS write."""

    def list(
        self,
        namespace: str,
        *,
        prefix: str,
        recursive: bool,
        page_size: int,
        cursor: str | None = None,
    ) -> StoragePage:
        """Return one lexicographically ordered page."""


class StorageConflictError(RuntimeError):
    """Expected failure of an atomic create or revision-CAS condition."""


class MemoryChannelStorage:
    """Deterministic in-memory backend used by portable conformance tests."""

    __slots__ = ("_records", "_revision")

    def __init__(self) -> None:
        self._records: dict[tuple[str, str], StorageRecord] = {}
        self._revision = 0

    def initialize(self) -> None:
        """The in-memory backend needs no preparation."""

    def get(self, namespace: str, key: str) -> StorageRecord | None:
        """Return a detached immutable record."""
        record = self._records.get((namespace, key))
        return None if record is None else _clone_record(record)

    def put(self, value: StoragePut) -> StorageRecord:
        """Apply exactly one atomic condition."""
        if value.if_absent == (value.if_revision is not None):
            raise ValueError("exactly one storage condition is required")
        map_key = (value.namespace, value.key)
        current = self._records.get(map_key)
        if value.if_absent:
            if current is not None:
                raise StorageConflictError
        elif current is None or current.revision != value.if_revision:
            raise StorageConflictError
        self._revision += 1
        record = StorageRecord(
            value.namespace,
            value.key,
            value.content_type,
            bytes(value.body),
            f"r{self._revision}",
            {},
        )
        self._records[map_key] = record
        return _clone_record(record)

    def list(
        self,
        namespace: str,
        *,
        prefix: str,
        recursive: bool,
        page_size: int,
        cursor: str | None = None,
    ) -> StoragePage:
        """Return keys after the exclusive cursor in raw string order."""
        if page_size <= 0:
            raise ValueError("invalid backend page size")
        if not recursive:
            raise ValueError("D18P lists are recursive")
        records = sorted(
            (
                record
                for record in self._records.values()
                if record.namespace == namespace
                and record.key.startswith(prefix)
                and (cursor is None or record.key > cursor)
            ),
            key=lambda record: record.key,
        )
        selected = tuple(_clone_record(record) for record in records[:page_size])
        next_cursor = selected[-1].key if len(records) > len(selected) else None
        return StoragePage(selected, next_cursor)

    def corrupt(self, record: StorageRecord) -> None:
        """Replace a record for negative tests without weakening put()."""
        self._records[(record.namespace, record.key)] = _clone_record(record)


@dataclass(frozen=True, slots=True)
class AppendRequest:
    """Creation fields supplied before a sequence is reserved."""

    message_id: bytes
    timestamp_ns: int
    originator_id: bytes
    key_epoch: int
    content_type: str


@dataclass(frozen=True, slots=True)
class MessagePage:
    """One ordered page with an optional next start sequence."""

    messages: tuple[D18Message, ...]
    next_start: int | None = None


@dataclass(frozen=True, slots=True)
class OpaqueKeyGrant:
    """A #141-produced grant that D18P persists without opening."""

    channel_id: bytes
    key_epoch: int
    receiver_id: bytes
    body: bytes


class ChannelStore:
    """CAS-backed view of one encrypted channel."""

    __slots__ = ("_backend", "_channel_id")

    def __init__(self, backend: ChannelStorageBackend, channel_id: bytes) -> None:
        validate_uuid_v7(channel_id, "corrupt_record")
        self._backend = backend
        self._channel_id = bytes(channel_id)

    def initialize(self) -> ChannelState:
        """Initialize state idempotently and recover any pending header."""
        _storage(self._backend.initialize)
        existing = self._state_record()
        if existing is not None:
            return _decode_state_record(existing, self._channel_id)
        body = channel_state_serialize(ChannelState(0))
        try:
            record = self._backend.put(
                _put_input(
                    sequence_state_record_key(self._channel_id),
                    CHANNEL_STATE_CONTENT_TYPE,
                    body,
                    if_absent=True,
                )
            )
            return _decode_state_record(record, self._channel_id)
        except StorageConflictError:
            return self.state()
        except Exception as error:
            raise _storage_error(error) from None

    def state(self) -> ChannelState:
        """Return the durable sequence state."""
        record = self._state_record()
        if record is None:
            _fail("not_initialized")
        return _decode_state_record(record, self._channel_id)

    def reserve_append(self, request: AppendRequest, plaintext: bytes) -> MessageHeader:
        """Durably reserve a fresh sequence before encryption."""
        validate_uuid_v7(request.message_id)
        try:
            validate_message_fields(
                MessageFields(
                    request.message_id,
                    request.timestamp_ns,
                    request.originator_id,
                    self._channel_id,
                    0,
                    request.key_epoch,
                    request.content_type,
                )
            )
        except Exception:
            _fail("crypto_error")
        for _ in range(MAX_CHANNEL_CAS_ATTEMPTS):
            record = self._state_record()
            if record is None:
                _fail("not_initialized")
            current = _decode_state_record(record, self._channel_id)
            if current.pending_header is not None:
                _fail("pending_append")
            if current.next_sequence == MAX_U64:
                _fail("sequence_exhausted")
            header = MessageHeader(
                request.message_id,
                request.timestamp_ns,
                request.originator_id,
                self._channel_id,
                current.next_sequence,
                request.key_epoch,
                request.content_type,
                sha256(bytes(plaintext)),
            )
            updated = channel_state_serialize(
                ChannelState(current.next_sequence + 1, header)
            )
            try:
                self._backend.put(
                    _put_input(
                        sequence_state_record_key(self._channel_id),
                        CHANNEL_STATE_CONTENT_TYPE,
                        updated,
                        if_revision=record.revision,
                    )
                )
                return header
            except StorageConflictError:
                continue
            except Exception as error:
                raise _storage_error(error) from None
        _fail("concurrent_update")

    def commit_reserved(
        self,
        header: MessageHeader,
        plaintext: bytes,
        channel_master_key: bytes,
        signing_secret_key: bytes,
    ) -> D18Message:
        """Encrypt, persist, and idempotently finalize one reservation."""
        if header.channel_id != self._channel_id:
            _fail("pending_header_mismatch")
        state = self.state()
        if state.pending_header is None:
            key = message_record_key(self._channel_id, header.sequence)
            record = _storage(lambda: self._backend.get(CHANNEL_STORAGE_NAMESPACE, key))
            if record is None:
                _fail("no_pending_append")
            _require_content_type(record, CHANNEL_MESSAGE_CONTENT_TYPE)
            stored = _decode_message(record.body)
            if not _message_matches_header(stored, header):
                _fail("conflicting_record")
            expected = _create_message(
                header, plaintext, signing_secret_key, channel_master_key
            )
            if message_serialize(expected) != record.body:
                _fail("conflicting_record")
            return stored
        if state.pending_header != header:
            _fail("pending_header_mismatch")
        message = _create_message(
            header, plaintext, signing_secret_key, channel_master_key
        )
        self._put_idempotent(
            message_record_key(self._channel_id, header.sequence),
            CHANNEL_MESSAGE_CONTENT_TYPE,
            message_serialize(message),
        )
        self._clear_pending(header)
        return message

    def append(
        self,
        request: AppendRequest,
        plaintext: bytes,
        channel_master_key: bytes,
        signing_secret_key: bytes,
    ) -> D18Message:
        """Reserve, encrypt, persist, and finalize one append."""
        header = self.reserve_append(request, plaintext)
        return self.commit_reserved(
            header, plaintext, channel_master_key, signing_secret_key
        )

    def abandon_pending(self) -> MessageHeader | None:
        """Clear a pending reservation while permanently consuming its sequence."""
        for _ in range(MAX_CHANNEL_CAS_ATTEMPTS):
            record = self._state_record()
            if record is None:
                _fail("not_initialized")
            current = _decode_state_record(record, self._channel_id)
            if current.pending_header is None:
                return None
            try:
                self._backend.put(
                    _put_input(
                        sequence_state_record_key(self._channel_id),
                        CHANNEL_STATE_CONTENT_TYPE,
                        channel_state_serialize(ChannelState(current.next_sequence)),
                        if_revision=record.revision,
                    )
                )
                return current.pending_header
            except StorageConflictError:
                continue
            except Exception as error:
                raise _storage_error(error) from None
        _fail("concurrent_update")

    def read_messages(self, start: int, page_size: int) -> MessagePage:
        """Read completed messages in strict sequence order, skipping gaps."""
        if (
            isinstance(page_size, bool)
            or not isinstance(page_size, int)
            or page_size <= 0
        ):
            _fail("invalid_page_size")
        cursor = message_record_key(self._channel_id, start - 1) if start > 0 else None
        page = _storage(
            lambda: self._backend.list(
                CHANNEL_STORAGE_NAMESPACE,
                prefix=message_record_prefix(self._channel_id),
                recursive=True,
                page_size=page_size,
                cursor=cursor,
            )
        )
        messages: list[D18Message] = []
        for record in page.records:
            _require_content_type(record, CHANNEL_MESSAGE_CONTENT_TYPE)
            message = _decode_message(record.body)
            if (
                message.channel_id != self._channel_id
                or message.sequence < start
                or record.key != message_record_key(self._channel_id, message.sequence)
                or (messages and messages[-1].sequence >= message.sequence)
            ):
                _fail("corrupt_record")
            messages.append(message)
        next_start: int | None = None
        if page.next_cursor is not None:
            if not messages or messages[-1].sequence == MAX_U64:
                _fail("corrupt_record")
            next_start = messages[-1].sequence + 1
        return MessagePage(tuple(messages), next_start)

    def read_for_receiver(self, receiver_id: bytes, page_size: int) -> MessagePage:
        """Read from one receiver's durable first-unread cursor."""
        return self.read_messages(self.receiver_cursor(receiver_id), page_size)

    def receiver_cursor(self, receiver_id: bytes) -> int:
        """Return one receiver's durable first-unread sequence."""
        validate_agent_id(receiver_id, "invalid_receiver_id")
        key = receiver_ack_record_key(self._channel_id, receiver_id)
        record = _storage(lambda: self._backend.get(CHANNEL_STORAGE_NAMESPACE, key))
        if record is None:
            return 0
        _require_content_type(record, CHANNEL_ACK_CONTENT_TYPE)
        return receiver_cursor_deserialize(record.body)

    def acknowledge(self, receiver_id: bytes, acknowledged: int) -> int:
        """Advance one receiver cursor monotonically to acknowledged + 1."""
        validate_agent_id(receiver_id, "invalid_receiver_id")
        state = self.state()
        if acknowledged >= state.next_sequence:
            _fail("acknowledgement_ahead")
        if (
            state.pending_header is not None
            and acknowledged >= state.pending_header.sequence
        ):
            _fail("acknowledgement_pending")
        if acknowledged == MAX_U64:
            _fail("sequence_exhausted")
        desired = acknowledged + 1
        key = receiver_ack_record_key(self._channel_id, receiver_id)
        for _ in range(MAX_CHANNEL_CAS_ATTEMPTS):
            record = _storage(lambda: self._backend.get(CHANNEL_STORAGE_NAMESPACE, key))
            if record is None:
                try:
                    self._backend.put(
                        _put_input(
                            key,
                            CHANNEL_ACK_CONTENT_TYPE,
                            receiver_cursor_serialize(desired),
                            if_absent=True,
                        )
                    )
                    return desired
                except StorageConflictError:
                    continue
                except Exception as error:
                    raise _storage_error(error) from None
            _require_content_type(record, CHANNEL_ACK_CONTENT_TYPE)
            current = receiver_cursor_deserialize(record.body)
            if desired < current:
                _fail("acknowledgement_regression")
            if desired == current:
                return current
            try:
                self._backend.put(
                    _put_input(
                        key,
                        CHANNEL_ACK_CONTENT_TYPE,
                        receiver_cursor_serialize(desired),
                        if_revision=record.revision,
                    )
                )
                return desired
            except StorageConflictError:
                continue
            except Exception as error:
                raise _storage_error(error) from None
        _fail("concurrent_update")

    def save_key_grant(self, grant: OpaqueKeyGrant) -> None:
        """Persist one #141-produced opaque grant idempotently."""
        if grant.channel_id != self._channel_id:
            _fail("corrupt_record")
        validate_agent_id(grant.receiver_id, "invalid_receiver_id")
        self._put_idempotent(
            key_grant_record_key(self._channel_id, grant.key_epoch, grant.receiver_id),
            CHANNEL_GRANT_CONTENT_TYPE,
            grant.body,
        )

    def key_grant(self, key_epoch: int, receiver_id: bytes) -> bytes | None:
        """Load one opaque receiver grant."""
        validate_agent_id(receiver_id, "invalid_receiver_id")
        key = key_grant_record_key(self._channel_id, key_epoch, receiver_id)
        record = _storage(lambda: self._backend.get(CHANNEL_STORAGE_NAMESPACE, key))
        if record is None:
            return None
        _require_content_type(record, CHANNEL_GRANT_CONTENT_TYPE)
        return bytes(record.body)

    def _state_record(self) -> StorageRecord | None:
        return _storage(
            lambda: self._backend.get(
                CHANNEL_STORAGE_NAMESPACE,
                sequence_state_record_key(self._channel_id),
            )
        )

    def _put_idempotent(self, key: str, content_type: str, body: bytes) -> None:
        try:
            self._backend.put(_put_input(key, content_type, body, if_absent=True))
        except StorageConflictError:
            current = _storage(
                lambda: self._backend.get(CHANNEL_STORAGE_NAMESPACE, key)
            )
            if (
                current is None
                or current.content_type != content_type
                or current.body != body
            ):
                _fail("conflicting_record")
        except Exception as error:
            raise _storage_error(error) from None

    def _clear_pending(self, expected: MessageHeader) -> None:
        for _ in range(MAX_CHANNEL_CAS_ATTEMPTS):
            record = self._state_record()
            if record is None:
                _fail("not_initialized")
            current = _decode_state_record(record, self._channel_id)
            if current.pending_header is None:
                return
            if current.pending_header != expected:
                _fail("pending_header_mismatch")
            try:
                self._backend.put(
                    _put_input(
                        sequence_state_record_key(self._channel_id),
                        CHANNEL_STATE_CONTENT_TYPE,
                        channel_state_serialize(ChannelState(current.next_sequence)),
                        if_revision=record.revision,
                    )
                )
                return
            except StorageConflictError:
                continue
            except Exception as error:
                raise _storage_error(error) from None
        _fail("concurrent_update")


def _create_message(
    header: MessageHeader,
    plaintext: bytes,
    signing_secret_key: bytes,
    channel_master_key: bytes,
) -> D18Message:
    if sha256(bytes(plaintext)) != header.plaintext_hash:
        _fail("crypto_error")
    try:
        return message_create(
            MessageFields(
                header.message_id,
                header.timestamp_ns,
                header.originator_id,
                header.channel_id,
                header.sequence,
                header.key_epoch,
                header.content_type,
            ),
            bytes(plaintext),
            bytes(signing_secret_key),
            bytes(channel_master_key),
        )
    except Exception:
        _fail("crypto_error")


def _decode_message(data: bytes) -> D18Message:
    try:
        return message_deserialize(data)
    except MessageProfileError:
        _fail("wire_error")


def _message_matches_header(message: D18Message, header: MessageHeader) -> bool:
    return (
        message.message_id == header.message_id
        and message.timestamp_ns == header.timestamp_ns
        and message.originator_id == header.originator_id
        and message.channel_id == header.channel_id
        and message.sequence == header.sequence
        and message.key_epoch == header.key_epoch
        and message.content_type == header.content_type
        and message.plaintext_hash == header.plaintext_hash
    )


def _decode_state_record(record: StorageRecord, channel_id: bytes) -> ChannelState:
    _require_content_type(record, CHANNEL_STATE_CONTENT_TYPE)
    return channel_state_deserialize(record.body, channel_id)


def _require_content_type(record: StorageRecord, expected: str) -> None:
    if record.content_type != expected:
        _fail("corrupt_record")


def _put_input(
    key: str,
    content_type: str,
    body: bytes,
    *,
    if_absent: bool = False,
    if_revision: str | None = None,
) -> StoragePut:
    return StoragePut(
        CHANNEL_STORAGE_NAMESPACE,
        key,
        content_type,
        bytes(body),
        {},
        if_absent,
        if_revision,
    )


def _storage[T](operation: Callable[[], T]) -> T:
    try:
        return operation()
    except (ChannelProfileError, StorageConflictError):
        raise
    except Exception as error:
        raise _storage_error(error) from None


def _storage_error(error: Exception) -> ChannelProfileError:
    if isinstance(error, ChannelProfileError):
        return error
    return ChannelProfileError("storage_error")


def _clone_record(record: StorageRecord) -> StorageRecord:
    return StorageRecord(
        record.namespace,
        record.key,
        record.content_type,
        bytes(record.body),
        record.revision,
        {},
    )
