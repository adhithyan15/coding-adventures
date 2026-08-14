"""Durable D18P membership plus structurally separate endpoint roles."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from typing import Protocol

from coding_adventures_chief_of_staff_channel_crypto import message_verify

from .profile import (
    CHANNEL_DEFINITION_CONTENT_TYPE,
    CHANNEL_STORAGE_NAMESPACE,
    MAX_DEFINITION_CAS_ATTEMPTS,
    ChannelDefinition,
    ChannelProfileError,
    _fail,
    channel_definition_deserialize,
    channel_definition_record_key,
    channel_definition_serialize,
    validate_agent_id,
    validate_uuid_v7,
)
from .storage import (
    AppendRequest,
    ChannelStorageBackend,
    ChannelStore,
    OpaqueKeyGrant,
    StorageConflictError,
    StoragePut,
    StorageRecord,
)


@dataclass(frozen=True, slots=True)
class MessageMetadata:
    """Injected identifier and timestamp for one publication."""

    message_id: bytes
    timestamp_ns: int


class MessageMetadataSource(Protocol):
    """Injected UUID/clock boundary."""

    def next(self) -> MessageMetadata:
        """Return metadata for the next publication."""


@dataclass(frozen=True, slots=True)
class PublishedMessage:
    """Originator-visible publication result."""

    message_id: bytes
    sequence: int
    timestamp_ns: int


@dataclass(frozen=True, slots=True)
class ReceivedMessage(PublishedMessage):
    """Verified receiver-visible plaintext."""

    content_type: str
    payload: bytes


class ReceiverEpochKeyProvider(Protocol):
    """#141-owned key custody boundary used only after grant lookup."""

    @property
    def public_key(self) -> bytes:
        """Return the receiver's X25519 public key."""

    def open_grant(self, key_epoch: int, grant_body: bytes) -> bytes | None:
        """Open an opaque grant or return no usable key."""


class ChannelDefinitionStore:
    """Atomic creation, loading, and irreversible retirement of D18C records."""

    __slots__ = ("_backend",)

    def __init__(self, backend: ChannelStorageBackend) -> None:
        self._backend = backend

    def create(self, definition: ChannelDefinition) -> ChannelDefinition:
        """Create a definition idempotently and initialize its sequence state."""
        if definition.lifecycle != "active":
            _fail("invalid_definition")
        _backend_call(self._backend.initialize)
        key = channel_definition_record_key(definition.channel_id)
        body = channel_definition_serialize(definition)
        try:
            record = self._backend.put(
                StoragePut(
                    CHANNEL_STORAGE_NAMESPACE,
                    key,
                    CHANNEL_DEFINITION_CONTENT_TYPE,
                    body,
                    {},
                    True,
                )
            )
            persisted = _require_definition_record(record, definition.channel_id)
        except StorageConflictError:
            existing = _backend_call(
                lambda: self._backend.get(CHANNEL_STORAGE_NAMESPACE, key)
            )
            if existing is None:
                _fail("definition_not_found")
            if existing.content_type != CHANNEL_DEFINITION_CONTENT_TYPE:
                _fail("corrupt_definition")
            if existing.body != body:
                _fail("conflicting_definition")
            persisted = _require_definition_record(existing, definition.channel_id)
        except Exception as error:
            raise _backend_error(error) from None
        if persisted != definition:
            _fail("conflicting_definition")
        ChannelStore(self._backend, definition.channel_id).initialize()
        return self.require_current(definition)

    def load(self, channel_id: bytes) -> ChannelDefinition | None:
        """Load one definition if it exists."""
        _backend_call(self._backend.initialize)
        loaded = self._load_record(channel_id)
        return None if loaded is None else loaded.definition

    def destroy(self, channel_id: bytes) -> ChannelDefinition:
        """Irreversibly mark one definition destroyed with revision CAS."""
        _backend_call(self._backend.initialize)
        for _ in range(MAX_DEFINITION_CAS_ATTEMPTS):
            loaded = self._load_record(channel_id)
            if loaded is None:
                _fail("definition_not_found")
            if loaded.definition.lifecycle == "destroyed":
                return loaded.definition
            destroyed = loaded.definition.with_lifecycle("destroyed")
            try:
                record = self._backend.put(
                    StoragePut(
                        CHANNEL_STORAGE_NAMESPACE,
                        channel_definition_record_key(channel_id),
                        CHANNEL_DEFINITION_CONTENT_TYPE,
                        channel_definition_serialize(destroyed),
                        {},
                        False,
                        loaded.revision,
                    )
                )
                return _require_definition_record(record, channel_id)
            except StorageConflictError:
                continue
            except Exception as error:
                raise _backend_error(error) from None
        _fail("concurrent_update")

    def require_current(self, expected: ChannelDefinition) -> ChannelDefinition:
        """Require the exact cached definition to remain active."""
        actual = self.load(expected.channel_id)
        if actual is None:
            _fail("definition_not_found")
        if actual.lifecycle == "destroyed":
            _fail("channel_destroyed")
        if actual != expected:
            _fail("definition_changed")
        return actual

    def _load_record(self, channel_id: bytes) -> _LoadedDefinition | None:
        key = channel_definition_record_key(channel_id)
        record = _backend_call(
            lambda: self._backend.get(CHANNEL_STORAGE_NAMESPACE, key)
        )
        if record is None:
            return None
        return _LoadedDefinition(
            _require_definition_record(record, channel_id), record.revision
        )


@dataclass(frozen=True, slots=True)
class _LoadedDefinition:
    definition: ChannelDefinition
    revision: str


class DurableOriginator:
    """The only endpoint role with a publish operation."""

    __slots__ = (
        "_backend",
        "_channel_master_key",
        "_definition",
        "_metadata_source",
        "_signing_secret_key",
    )

    def __init__(
        self,
        backend: ChannelStorageBackend,
        definition: ChannelDefinition,
        signing_secret_key: bytes,
        channel_master_key: bytes,
        metadata_source: MessageMetadataSource,
    ) -> None:
        self._backend = backend
        self._definition = definition
        self._signing_secret_key = bytes(signing_secret_key)
        self._channel_master_key = bytes(channel_master_key)
        self._metadata_source = metadata_source

    @classmethod
    def open(
        cls,
        backend: ChannelStorageBackend,
        channel_id: bytes,
        agent_id: bytes,
        signing_secret_key: bytes,
        channel_master_key: bytes,
        metadata_source: MessageMetadataSource,
    ) -> DurableOriginator:
        """Authorize and open one originator endpoint."""
        definition = _active_definition(backend, channel_id)
        if definition.originator.agent_id != agent_id:
            _fail("unauthorized_originator")
        if (
            len(signing_secret_key) != 64
            or definition.originator.public_key != signing_secret_key[32:]
        ):
            _fail("public_key_mismatch")
        if len(channel_master_key) != 32:
            _fail("crypto_error")
        ChannelStore(backend, channel_id).initialize()
        return cls(
            backend,
            definition,
            signing_secret_key,
            channel_master_key,
            metadata_source,
        )

    @property
    def id(self) -> bytes:
        """Return the originator agent ID."""
        return self._definition.originator.agent_id

    @property
    def channel_id(self) -> bytes:
        """Return the bound channel ID."""
        return self._definition.channel_id

    @property
    def public_key(self) -> bytes:
        """Return the originator Ed25519 public key."""
        return self._definition.originator.public_key

    def publish(self, payload: bytes, content_type: str) -> PublishedMessage:
        """Publish using the injected metadata source."""
        try:
            metadata = self._metadata_source.next()
        except Exception:
            _fail("metadata_error")
        return self.publish_with_metadata(metadata, payload, content_type)

    def publish_with_metadata(
        self, metadata: MessageMetadata, payload: bytes, content_type: str
    ) -> PublishedMessage:
        """Publish with explicit deterministic metadata."""
        validate_uuid_v7(metadata.message_id)
        ChannelDefinitionStore(self._backend).require_current(self._definition)
        message = ChannelStore(self._backend, self._definition.channel_id).append(
            _append_request(
                metadata,
                self._definition.originator.agent_id,
                self._definition.key_epoch,
                content_type,
            ),
            payload,
            self._channel_master_key,
            self._signing_secret_key,
        )
        return PublishedMessage(
            bytes(metadata.message_id), message.sequence, metadata.timestamp_ns
        )

    def save_receiver_grant(self, receiver_id: bytes, grant_body: bytes) -> None:
        """Persist a #141-produced opaque grant after membership checks."""
        definition = ChannelDefinitionStore(self._backend).require_current(
            self._definition
        )
        if definition.receiver(receiver_id) is None:
            _fail("unauthorized_receiver")
        ChannelStore(self._backend, definition.channel_id).save_key_grant(
            OpaqueKeyGrant(
                definition.channel_id,
                definition.key_epoch,
                bytes(receiver_id),
                bytes(grant_body),
            )
        )


class DurableReceiver:
    """Receiver role: verified delivery and session-bound acknowledgement only."""

    __slots__ = (
        "_backend",
        "_definition",
        "_delivered",
        "_key_provider",
        "_receiver_id",
    )

    def __init__(
        self,
        backend: ChannelStorageBackend,
        definition: ChannelDefinition,
        receiver_id: bytes,
        key_provider: ReceiverEpochKeyProvider,
    ) -> None:
        self._backend = backend
        self._definition = definition
        self._receiver_id = bytes(receiver_id)
        self._key_provider = key_provider
        self._delivered: dict[bytes, int] = {}

    @classmethod
    def open(
        cls,
        backend: ChannelStorageBackend,
        channel_id: bytes,
        receiver_id: bytes,
        key_provider: ReceiverEpochKeyProvider,
    ) -> DurableReceiver:
        """Authorize and open one receiver endpoint."""
        validate_agent_id(receiver_id, "invalid_receiver_id")
        definition = _active_definition(backend, channel_id)
        receiver = definition.receiver(receiver_id)
        if receiver is None:
            _fail("unauthorized_receiver")
        if receiver.public_key != key_provider.public_key:
            _fail("public_key_mismatch")
        ChannelStore(backend, channel_id).initialize()
        return cls(backend, definition, receiver_id, key_provider)

    @property
    def id(self) -> bytes:
        """Return this receiver ID."""
        return self._receiver_id

    @property
    def channel_id(self) -> bytes:
        """Return the bound channel ID."""
        return self._definition.channel_id

    @property
    def public_key(self) -> bytes:
        """Return this receiver's X25519 public key."""
        return bytes(self._key_provider.public_key)

    def receive(self, limit: int) -> tuple[ReceivedMessage, ...]:
        """Return one ordered page of verified plaintext messages."""
        ChannelDefinitionStore(self._backend).require_current(self._definition)
        store = ChannelStore(self._backend, self._definition.channel_id)
        page = store.read_for_receiver(self._receiver_id, limit)
        delivered: list[ReceivedMessage] = []
        for message in page.messages:
            if (
                message.channel_id != self._definition.channel_id
                or message.originator_id != self._definition.originator.agent_id
                or message.key_epoch > self._definition.key_epoch
            ):
                _fail("unauthorized_message")
            grant = store.key_grant(message.key_epoch, self._receiver_id)
            if grant is None:
                _fail("missing_key_grant")
            try:
                channel_key = self._key_provider.open_grant(message.key_epoch, grant)
            except Exception:
                _fail("crypto_error")
            if channel_key is None:
                _fail("missing_key_grant")
            try:
                payload = message_verify(
                    message,
                    self._definition.originator.public_key,
                    channel_key,
                )
            except Exception:
                _fail("crypto_error")
            validate_uuid_v7(message.message_id)
            previous = self._delivered.get(message.message_id)
            if previous is not None and previous != message.sequence:
                _fail("unauthorized_message")
            self._delivered[message.message_id] = message.sequence
            delivered.append(
                ReceivedMessage(
                    message.message_id,
                    message.sequence,
                    message.timestamp_ns,
                    message.content_type,
                    bytes(payload),
                )
            )
        return tuple(delivered)

    def acknowledge(self, message_id: bytes) -> int:
        """Acknowledge only a message delivered by this endpoint session."""
        validate_uuid_v7(message_id)
        ChannelDefinitionStore(self._backend).require_current(self._definition)
        sequence = self._delivered.get(bytes(message_id))
        if sequence is None:
            _fail("unknown_message_id")
        return ChannelStore(self._backend, self._definition.channel_id).acknowledge(
            self._receiver_id, sequence
        )


def _active_definition(
    backend: ChannelStorageBackend, channel_id: bytes
) -> ChannelDefinition:
    definition = ChannelDefinitionStore(backend).load(channel_id)
    if definition is None:
        _fail("definition_not_found")
    if definition.lifecycle == "destroyed":
        _fail("channel_destroyed")
    return definition


def _require_definition_record(
    record: StorageRecord, channel_id: bytes
) -> ChannelDefinition:
    if record.content_type != CHANNEL_DEFINITION_CONTENT_TYPE:
        _fail("corrupt_definition")
    definition = channel_definition_deserialize(record.body)
    if (
        definition.channel_id != channel_id
        or record.key != channel_definition_record_key(channel_id)
    ):
        _fail("corrupt_definition")
    return definition


def _backend_call[T](operation: Callable[[], T]) -> T:
    try:
        return operation()
    except (ChannelProfileError, StorageConflictError):
        raise
    except Exception as error:
        raise _backend_error(error) from None


def _backend_error(error: Exception) -> ChannelProfileError:
    if isinstance(error, ChannelProfileError):
        return error
    return ChannelProfileError("storage_error")


def _append_request(
    metadata: MessageMetadata,
    originator_id: bytes,
    key_epoch: int,
    content_type: str,
) -> AppendRequest:
    return AppendRequest(
        metadata.message_id,
        metadata.timestamp_ns,
        originator_id,
        key_epoch,
        content_type,
    )
