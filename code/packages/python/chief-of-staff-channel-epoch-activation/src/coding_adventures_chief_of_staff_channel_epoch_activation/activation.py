"""Portable D18T durable epoch-activation orchestration."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from typing import Literal, NoReturn

from coding_adventures_chief_of_staff_channel_crypto import (
    ChannelMasterKey,
    D18Message,
    MessageFields,
    OriginatorSigningKey,
    PortableKeyGrant,
    RotationPlan,
    grant_deserialize,
    grant_serialize,
    message_create_with_signer,
    message_deserialize,
    message_serialize,
    secret_erasure_capability,
    verify_grant_signature,
)
from coding_adventures_chief_of_staff_channel_store import (
    CHANNEL_GRANT_CONTENT_TYPE,
    CHANNEL_MESSAGE_CONTENT_TYPE,
    CHANNEL_STATE_CONTENT_TYPE,
    CHANNEL_STORAGE_NAMESPACE,
    ChannelDefinition,
    ChannelDefinitionStore,
    ChannelProfileError,
    ChannelStorageBackend,
    MessageHeader,
    ReceiverIdentity,
    StorageConflictError,
    StoragePut,
    StorageRecord,
    channel_state_deserialize,
    key_grant_record_key,
    message_record_key,
    sequence_state_record_key,
)
from coding_adventures_sha256 import sha256

from .custody import (
    EpochKeyHandle,
    OriginatorKeyCustody,
    PreparedEpoch,
    PublicPreparation,
)
from .wire import (
    ACTIVATION_PLAN_CONTENT_TYPE,
    EPOCH_STATE_CONTENT_TYPE,
    MAX_PLAN_RECEIVERS,
    MAX_U64,
    ActivationPlan,
    ActivationPlanEntry,
    EpochState,
    activation_plan_deserialize,
    activation_plan_record_key,
    activation_plan_serialize,
    epoch_state_deserialize,
    epoch_state_serialize,
)

MAX_EPOCH_CAS_ATTEMPTS = 16
EPOCH_ACTIVATION_ERROR_CODES = (
    "not_initialized",
    "channel_destroyed",
    "invalid_plan",
    "corrupt_record",
    "pending_append",
    "unactivated_epoch",
    "active_key_missing",
    "conflicting_active_key",
    "preparation_missing",
    "conflicting_preparation",
    "conflicting_plan",
    "conflicting_grant",
    "unexpected_epoch",
    "decreasing_epoch",
    "epoch_exhausted",
    "concurrent_update",
    "storage_error",
    "custody_error",
    "crypto_error",
)
EpochActivationErrorCode = Literal[
    "not_initialized",
    "channel_destroyed",
    "invalid_plan",
    "corrupt_record",
    "pending_append",
    "unactivated_epoch",
    "active_key_missing",
    "conflicting_active_key",
    "preparation_missing",
    "conflicting_preparation",
    "conflicting_plan",
    "conflicting_grant",
    "unexpected_epoch",
    "decreasing_epoch",
    "epoch_exhausted",
    "concurrent_update",
    "storage_error",
    "custody_error",
    "crypto_error",
]
PreparationOutcome = Literal["prepared", "idempotent"]
ActivationOutcome = Literal["activated", "idempotent"]


class EpochActivationError(ValueError):
    """Stable D18T failure whose message never contains secret data."""

    def __init__(self, code: EpochActivationErrorCode) -> None:
        super().__init__(code)
        self.code = code


def epoch_activation_secret_erasure_capability() -> str:
    """Report the honest D18Q/Python secret-erasure capability."""
    return secret_erasure_capability()


@dataclass(frozen=True, slots=True)
class ActiveEpochAppendRequest:
    message_id: bytes
    timestamp_ns: int
    originator_id: bytes
    content_type: str
    key_epoch: int | None = None


@dataclass(frozen=True, slots=True)
class EpochReservation:
    header: MessageHeader
    key_handle: EpochKeyHandle


class EpochActivationStore:
    """D18T coordinator over injected public storage and secret custody."""

    def __init__(
        self,
        backend: ChannelStorageBackend,
        custody: OriginatorKeyCustody,
        channel_id: bytes,
    ) -> None:
        self._backend = backend
        self._custody = custody
        self._channel_id = bytes(channel_id)

    @classmethod
    def open(
        cls,
        backend: ChannelStorageBackend,
        custody: OriginatorKeyCustody,
        channel_id: bytes,
    ) -> EpochActivationStore:
        if not custody.durable:
            _fail("custody_error")
        _backend_call(backend.initialize)
        return cls(backend, custody, channel_id)

    @classmethod
    def open_for_testing(
        cls,
        backend: ChannelStorageBackend,
        custody: OriginatorKeyCustody,
        channel_id: bytes,
    ) -> EpochActivationStore:
        _backend_call(backend.initialize)
        return cls(backend, custody, channel_id)

    def create_epoch_channel(
        self, definition: ChannelDefinition, initial_cmk: ChannelMasterKey
    ) -> EpochState:
        """Create a D18T-aware channel, custody before any D18S state.

        The definition is settled *before* the custody import, and that ordering
        matters. Custody slots are keyed by ``(channel_id, epoch)`` and the first
        writer wins permanently. Importing first would let a caller presenting a
        mismatched definition claim an unclaimed slot and then fail, leaving the
        legitimate import to hit ``conflicting_active_key`` forever -- fail
        closed, but permanently wedged. D18T only requires custody before
        *state*, so validating the definition first costs nothing.
        """
        consumed = False
        try:
            if (
                definition.channel_id != self._channel_id
                or definition.lifecycle != "active"
            ):
                _fail("invalid_plan")
            definitions = ChannelDefinitionStore(self._backend)
            try:
                existing = definitions.load(self._channel_id)
                if existing is None:
                    definitions.create(definition)
                elif existing.lifecycle == "destroyed":
                    _fail("channel_destroyed")
                elif existing != definition:
                    _fail("invalid_plan")
            except EpochActivationError:
                raise
            except ChannelProfileError as error:
                if error.code == "channel_destroyed":
                    _fail("channel_destroyed")
                if error.code == "conflicting_definition":
                    _fail("invalid_plan")
                _fail("corrupt_record")
            except Exception:
                _fail("storage_error")
            consumed = True
            self._import_initial_key(definition.key_epoch, initial_cmk)
        finally:
            # _import_initial_key owns the erase once it runs; every earlier
            # exit must still erase, so an unused secret never outlives the call.
            if not consumed:
                initial_cmk.destroy()
        return self.migrate_epoch_state(definition)

    def migrate_epoch_state(
        self,
        definition: ChannelDefinition,
        current_cmk: ChannelMasterKey | None = None,
    ) -> EpochState:
        self._require_definition(definition, False)
        for _ in range(MAX_EPOCH_CAS_ATTEMPTS):
            record = self._state_record()
            if record is not None and record.content_type == EPOCH_STATE_CONTENT_TYPE:
                state = self._decode_v2_state_record(record)
                active_epoch = state.active_epoch
                if self._resolve_handle(active_epoch) is None:
                    _fail("active_key_missing")
                return state
            self._ensure_initial_key(definition.key_epoch, current_cmk)
            if record is None:
                state = EpochState(self._channel_id, definition.key_epoch, 0)
            else:
                self._require_envelope(
                    record,
                    sequence_state_record_key(self._channel_id),
                    CHANNEL_STATE_CONTENT_TYPE,
                )
                try:
                    prior = channel_state_deserialize(record.body, self._channel_id)
                except Exception:
                    _fail("corrupt_record")
                if (
                    prior.pending_header is not None
                    and prior.pending_header.key_epoch != definition.key_epoch
                ):
                    _fail("corrupt_record")
                state = EpochState(
                    self._channel_id,
                    definition.key_epoch,
                    prior.next_sequence,
                    prior.pending_header,
                )
            try:
                stored = self._backend.put(
                    _public_put(
                        sequence_state_record_key(self._channel_id),
                        EPOCH_STATE_CONTENT_TYPE,
                        epoch_state_serialize(state),
                        if_absent=record is None,
                        if_revision=None if record is None else record.revision,
                    )
                )
                return self._decode_v2_state_record(stored)
            except StorageConflictError:
                continue
            except EpochActivationError:
                raise
            except Exception:
                _fail("storage_error")
        _fail("concurrent_update")

    def state(self) -> EpochState:
        record = self._state_record()
        if record is None:
            _fail("not_initialized")
        return self._decode_v2_state_record(record)

    def prepare_rotation(
        self,
        definition: ChannelDefinition,
        target_roster: tuple[ReceiverIdentity, ...],
        rotation: RotationPlan,
    ) -> PreparationOutcome:
        self._require_definition(definition, False)
        state = self.state()
        if state.pending_header is not None:
            _fail("pending_append")
        if state.active_epoch == MAX_U64:
            _fail("epoch_exhausted")
        expected = state.active_epoch + 1
        if rotation.new_epoch != expected:
            _fail("unexpected_epoch")
        prepared = prepare_rotation_candidate(
            definition, state.active_epoch, target_roster, rotation
        )
        try:
            selection = self._custody_call(
                lambda: self._custody.prepare_if_absent(prepared)
            )
        finally:
            prepared.destroy()
        if selection == "conflict":
            _fail("conflicting_preparation")
        self._replay_preparation(definition, expected)
        return "prepared" if selection == "selected" else "idempotent"

    def recover_preparation(
        self, definition: ChannelDefinition, new_epoch: int
    ) -> PreparationOutcome:
        self._require_definition(definition, False)
        active = self.state().active_epoch
        if new_epoch < active:
            _fail("decreasing_epoch")
        if new_epoch != active:
            if active == MAX_U64:
                _fail("epoch_exhausted")
            if new_epoch != active + 1:
                _fail("unexpected_epoch")
        self._replay_preparation(definition, new_epoch)
        return "idempotent"

    def activate_prepared_epoch(
        self, definition: ChannelDefinition, new_epoch: int
    ) -> ActivationOutcome:
        self._require_definition(definition, False)
        prepared = self._custody_call(
            lambda: self._custody.load_preparation(self._channel_id, new_epoch)
        )
        if prepared is None:
            _fail("preparation_missing")
        for _ in range(MAX_EPOCH_CAS_ATTEMPTS):
            self._require_definition(definition, False)
            record = self._state_record()
            if record is None:
                _fail("not_initialized")
            state = self._decode_v2_state_record(record)
            if state.active_epoch == new_epoch:
                self._validate_and_replay(definition, prepared)
                self._require_handle(new_epoch)
                return "idempotent"
            if state.active_epoch > new_epoch:
                _fail("decreasing_epoch")
            if state.active_epoch == MAX_U64:
                _fail("epoch_exhausted")
            if (
                state.active_epoch + 1 != new_epoch
                or prepared.base_epoch != state.active_epoch
                or prepared.new_epoch != new_epoch
            ):
                _fail("unexpected_epoch")
            self._validate_and_replay(definition, prepared)
            self._require_handle(new_epoch)
            if state.pending_header is not None:
                _fail("pending_append")
            updated = state.with_active_epoch(self._channel_id, new_epoch)
            try:
                stored = self._backend.put(
                    _public_put(
                        sequence_state_record_key(self._channel_id),
                        EPOCH_STATE_CONTENT_TYPE,
                        epoch_state_serialize(updated),
                        if_revision=record.revision,
                    )
                )
                if self._decode_v2_state_record(stored) != updated:
                    _fail("corrupt_record")
                return "activated"
            except StorageConflictError:
                continue
            except EpochActivationError:
                raise
            except Exception:
                _fail("storage_error")
        _fail("concurrent_update")

    def reserve_publish_using_active_epoch(
        self,
        definition: ChannelDefinition,
        request: ActiveEpochAppendRequest,
        plaintext: bytes,
    ) -> EpochReservation:
        self._require_definition(definition, False)
        if request.originator_id != definition.originator.agent_id:
            _fail("invalid_plan")
        for _ in range(MAX_EPOCH_CAS_ATTEMPTS):
            record = self._state_record()
            if record is None:
                _fail("not_initialized")
            state = self._decode_v2_state_record(record)
            if (
                request.key_epoch is not None
                and request.key_epoch != state.active_epoch
            ):
                _fail("unactivated_epoch")
            handle = self._require_handle(state.active_epoch)
            if state.pending_header is not None:
                _fail("pending_append")
            if state.next_sequence == MAX_U64:
                _fail("crypto_error")
            try:
                header = MessageHeader(
                    request.message_id,
                    request.timestamp_ns,
                    request.originator_id,
                    self._channel_id,
                    state.next_sequence,
                    state.active_epoch,
                    request.content_type,
                    sha256(bytes(plaintext)),
                )
            except Exception:
                _fail("crypto_error")
            updated = state.with_pending(
                self._channel_id, state.next_sequence + 1, header
            )
            try:
                self._backend.put(
                    _public_put(
                        sequence_state_record_key(self._channel_id),
                        EPOCH_STATE_CONTENT_TYPE,
                        epoch_state_serialize(updated),
                        if_revision=record.revision,
                    )
                )
                return EpochReservation(header, handle)
            except StorageConflictError:
                continue
            except Exception:
                _fail("storage_error")
        _fail("concurrent_update")

    def commit_reserved(
        self,
        definition: ChannelDefinition,
        reservation: EpochReservation,
        plaintext: bytes,
        signing_key: OriginatorSigningKey,
    ) -> D18Message:
        self._require_definition(definition, False)
        header = reservation.header
        if (
            header.channel_id != self._channel_id
            or header.key_epoch != reservation.key_handle.epoch
            or reservation.key_handle.channel_id != self._channel_id
            or signing_key.public_key != definition.originator.public_key
            or sha256(bytes(plaintext)) != header.plaintext_hash
        ):
            _fail("invalid_plan")
        state = self.state()
        if state.pending_header is None:
            key = message_record_key(self._channel_id, header.sequence)
            stored = _backend_call(
                lambda: self._backend.get(CHANNEL_STORAGE_NAMESPACE, key)
            )
            if stored is None:
                _fail("corrupt_record")
            self._require_envelope(stored, key, CHANNEL_MESSAGE_CONTENT_TYPE)
            try:
                message = message_deserialize(stored.body)
            except Exception:
                _fail("corrupt_record")
            if not _message_matches_header(message, header):
                _fail("corrupt_record")
            expected = self._encrypt_with_handle(
                reservation.key_handle, header, plaintext, signing_key
            )
            if message_serialize(expected) != stored.body:
                _fail("corrupt_record")
            return message
        if state.pending_header != header:
            _fail("invalid_plan")
        message = self._encrypt_with_handle(
            reservation.key_handle, header, plaintext, signing_key
        )
        self._put_immutable(
            message_record_key(self._channel_id, header.sequence),
            CHANNEL_MESSAGE_CONTENT_TYPE,
            message_serialize(message),
            "corrupt_record",
        )
        self._clear_pending(header)
        return message

    def abandon_pending(self) -> MessageHeader | None:
        for _ in range(MAX_EPOCH_CAS_ATTEMPTS):
            record = self._state_record()
            if record is None:
                _fail("not_initialized")
            state = self._decode_v2_state_record(record)
            if state.pending_header is None:
                return None
            updated = state.with_pending(self._channel_id, state.next_sequence)
            try:
                self._backend.put(
                    _public_put(
                        sequence_state_record_key(self._channel_id),
                        EPOCH_STATE_CONTENT_TYPE,
                        epoch_state_serialize(updated),
                        if_revision=record.revision,
                    )
                )
                return state.pending_header
            except StorageConflictError:
                continue
            except Exception:
                _fail("storage_error")
        _fail("concurrent_update")

    def activation_plan(self, new_epoch: int) -> ActivationPlan | None:
        key = activation_plan_record_key(self._channel_id, new_epoch)
        record = _backend_call(
            lambda: self._backend.get(CHANNEL_STORAGE_NAMESPACE, key)
        )
        if record is None:
            return None
        self._require_envelope(record, key, ACTIVATION_PLAN_CONTENT_TYPE)
        try:
            plan = activation_plan_deserialize(record.body)
        except Exception:
            _fail("corrupt_record")
        if plan.channel_id != self._channel_id or plan.new_epoch != new_epoch:
            _fail("corrupt_record")
        return plan

    def apply_destruction(self, definition: ChannelDefinition) -> None:
        self._require_definition(definition, True)
        self._custody_call(lambda: self._custody.destroy_channel(self._channel_id))

    def _ensure_initial_key(
        self, epoch: int, current_cmk: ChannelMasterKey | None
    ) -> None:
        if (
            self._custody_call(
                lambda: self._custody.resolve_handle(self._channel_id, epoch)
            )
            is not None
        ):
            return
        if current_cmk is None:
            _fail("active_key_missing")
        self._import_initial_key(epoch, current_cmk)

    def _import_initial_key(self, epoch: int, current_cmk: ChannelMasterKey) -> None:
        try:
            selection = self._custody_call(
                lambda: self._custody.import_active_if_absent(
                    self._channel_id, epoch, current_cmk
                )
            )
        finally:
            current_cmk.destroy()
        if selection == "conflict":
            _fail("conflicting_active_key")

    def _replay_preparation(
        self, definition: ChannelDefinition, new_epoch: int
    ) -> None:
        prepared = self._custody_call(
            lambda: self._custody.load_preparation(self._channel_id, new_epoch)
        )
        if prepared is None:
            _fail("preparation_missing")
        self._validate_and_replay(definition, prepared)

    def _validate_and_replay(
        self, definition: ChannelDefinition, prepared: PublicPreparation
    ) -> None:
        plan = _validate_public_preparation(definition, prepared)
        self._put_immutable(
            activation_plan_record_key(self._channel_id, plan.new_epoch),
            ACTIVATION_PLAN_CONTENT_TYPE,
            prepared.plan_bytes,
            "conflicting_plan",
        )
        for data in prepared.grants:
            try:
                grant = grant_deserialize(data)
            except Exception:
                _fail("crypto_error")
            self._put_immutable(
                key_grant_record_key(
                    self._channel_id, grant.key_epoch, grant.receiver_id
                ),
                CHANNEL_GRANT_CONTENT_TYPE,
                data,
                "conflicting_grant",
            )
        stored = self.activation_plan(plan.new_epoch)
        if stored != plan:
            _fail("corrupt_record")
        # Phase 6 reloads every grant from public storage. This is invariant 3,
        # "all grants before visibility" -- not paranoia about our own writes.
        # The record a put echoes back sits on the same trust boundary as the
        # write itself, so against a write-behind or eventually-consistent
        # backend an echoed success does not prove the grant is retrievable.
        # Activation may only advance the epoch once every receiver's grant can
        # actually be read.
        for data in prepared.grants:
            grant = _deserialize_grant(data)
            key = key_grant_record_key(
                self._channel_id, grant.key_epoch, grant.receiver_id
            )
            record = self._stored_record(key)
            if record is None:
                _fail("corrupt_record")
            self._require_envelope(record, key, CHANNEL_GRANT_CONTENT_TYPE)
            if record.body != data:
                # corrupt_record, not conflicting_grant: _put_immutable above
                # already reports a genuine slot conflict, so reaching here
                # means the backend returned something other than what it had
                # acknowledged writing. Matches the Rust reference.
                _fail("corrupt_record")

    def _encrypt_with_handle(
        self,
        handle: EpochKeyHandle,
        header: MessageHeader,
        plaintext: bytes,
        signing_key: OriginatorSigningKey,
    ) -> D18Message:
        def encrypt(cmk: ChannelMasterKey) -> D18Message:
            try:
                return message_create_with_signer(
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
                    signing_key.sign,
                    cmk.bytes,
                )
            except Exception:
                _fail("crypto_error")

        return self._custody_call(lambda: self._custody.with_key(handle, encrypt))

    def _stored_record(self, key: str) -> StorageRecord | None:
        """Read one public record, mapping backend failure to storage_error."""
        return _backend_call(
            lambda: self._backend.get(CHANNEL_STORAGE_NAMESPACE, key)
        )

    def _require_handle(self, epoch: int) -> EpochKeyHandle:
        handle = self._resolve_handle(epoch)
        if handle is None:
            _fail("active_key_missing")
        return handle

    def _resolve_handle(self, epoch: int) -> EpochKeyHandle | None:
        return self._custody_call(
            lambda: self._custody.resolve_handle(self._channel_id, epoch)
        )

    def _clear_pending(self, expected: MessageHeader) -> None:
        for _ in range(MAX_EPOCH_CAS_ATTEMPTS):
            record = self._state_record()
            if record is None:
                _fail("not_initialized")
            state = self._decode_v2_state_record(record)
            if state.pending_header is None:
                return
            if state.pending_header != expected:
                _fail("invalid_plan")
            updated = state.with_pending(self._channel_id, state.next_sequence)
            try:
                self._backend.put(
                    _public_put(
                        sequence_state_record_key(self._channel_id),
                        EPOCH_STATE_CONTENT_TYPE,
                        epoch_state_serialize(updated),
                        if_revision=record.revision,
                    )
                )
                return
            except StorageConflictError:
                continue
            except Exception:
                _fail("storage_error")
        _fail("concurrent_update")

    def _require_definition(
        self, expected: ChannelDefinition, require_destroyed: bool
    ) -> None:
        if expected.channel_id != self._channel_id:
            _fail("invalid_plan")
        try:
            actual = ChannelDefinitionStore(self._backend).load(self._channel_id)
        except Exception:
            _fail("storage_error")
        if actual is None:
            _fail("not_initialized")
        if actual != expected:
            _fail("invalid_plan")
        if not require_destroyed and actual.lifecycle == "destroyed":
            _fail("channel_destroyed")
        if require_destroyed and actual.lifecycle != "destroyed":
            _fail("invalid_plan")

    def _state_record(self) -> StorageRecord | None:
        return _backend_call(
            lambda: self._backend.get(
                CHANNEL_STORAGE_NAMESPACE,
                sequence_state_record_key(self._channel_id),
            )
        )

    def _decode_v2_state_record(self, record: StorageRecord) -> EpochState:
        self._require_envelope(
            record,
            sequence_state_record_key(self._channel_id),
            EPOCH_STATE_CONTENT_TYPE,
        )
        try:
            return epoch_state_deserialize(record.body, self._channel_id)
        except Exception:
            _fail("corrupt_record")

    @staticmethod
    def _require_envelope(record: StorageRecord, key: str, content_type: str) -> None:
        if (
            record.namespace != CHANNEL_STORAGE_NAMESPACE
            or record.key != key
            or record.content_type != content_type
            or record.metadata
        ):
            _fail("corrupt_record")

    def _put_immutable(
        self,
        key: str,
        content_type: str,
        body: bytes,
        conflict_code: EpochActivationErrorCode,
    ) -> None:
        try:
            record = self._backend.put(
                _public_put(key, content_type, body, if_absent=True)
            )
            self._require_envelope(record, key, content_type)
            if record.body != body:
                _fail("corrupt_record")
        except StorageConflictError:
            existing = _backend_call(
                lambda: self._backend.get(CHANNEL_STORAGE_NAMESPACE, key)
            )
            if existing is None:
                _fail("corrupt_record")
            self._require_envelope(existing, key, content_type)
            if existing.body != body:
                _fail(conflict_code)
        except EpochActivationError:
            raise
        except Exception:
            _fail("storage_error")

    @staticmethod
    def _custody_call[T](operation: Callable[[], T]) -> T:
        try:
            return operation()
        except EpochActivationError:
            raise
        except Exception:
            _fail("custody_error")


def prepare_rotation_candidate(
    definition: ChannelDefinition,
    base_epoch: int,
    target_roster: tuple[ReceiverIdentity, ...],
    rotation: RotationPlan,
) -> PreparedEpoch:
    """Build one pure custody candidate and consume the trusted rotation."""
    try:
        if not 1 <= len(target_roster) <= MAX_PLAN_RECEIVERS or len(
            target_roster
        ) != len(rotation.grants):
            _fail("invalid_plan")
        roster = tuple(sorted(target_roster, key=lambda receiver: receiver.agent_id))
        if len({receiver.agent_id for receiver in roster}) != len(roster):
            _fail("invalid_plan")
        for receiver, grant in zip(roster, rotation.grants, strict=True):
            if (
                receiver.agent_id != grant.receiver_id
                or grant.key_epoch != rotation.new_epoch
            ):
                _fail("invalid_plan")
            _verify_grant_public(
                definition,
                grant,
                receiver.agent_id,
            )
        if base_epoch == MAX_U64:
            _fail("epoch_exhausted")
        if rotation.new_epoch != base_epoch + 1:
            _fail("unexpected_epoch")
        grant_bytes = tuple(_serialize_grant(grant) for grant in rotation.grants)
        entries = tuple(
            ActivationPlanEntry(sha256(grant.receiver_id), sha256(data))
            for grant, data in zip(rotation.grants, grant_bytes, strict=True)
        )
        plan = ActivationPlan(
            definition.channel_id, base_epoch, rotation.new_epoch, entries
        )
        public = PublicPreparation(
            definition.channel_id,
            base_epoch,
            rotation.new_epoch,
            activation_plan_serialize(plan),
            grant_bytes,
        )
        cmk = rotation.new_cmk
        try:
            return PreparedEpoch(public, cmk)
        finally:
            cmk.destroy()
    finally:
        rotation.destroy()


def _validate_public_preparation(
    definition: ChannelDefinition, prepared: PublicPreparation
) -> ActivationPlan:
    if (
        prepared.channel_id != definition.channel_id
        or prepared.base_epoch == MAX_U64
        or prepared.new_epoch != prepared.base_epoch + 1
        or not 1 <= len(prepared.grants) <= MAX_PLAN_RECEIVERS
    ):
        _fail("invalid_plan")
    try:
        plan = activation_plan_deserialize(prepared.plan_bytes)
    except Exception:
        _fail("corrupt_record")
    if (
        plan.channel_id != prepared.channel_id
        or plan.base_epoch != prepared.base_epoch
        or plan.new_epoch != prepared.new_epoch
        or len(plan.receivers) != len(prepared.grants)
    ):
        _fail("invalid_plan")
    prior: bytes | None = None
    entries: list[ActivationPlanEntry] = []
    for data in prepared.grants:
        grant = _deserialize_grant(data)
        if (
            grant.channel_id != prepared.channel_id
            or grant.key_epoch != prepared.new_epoch
            or (prior is not None and prior >= grant.receiver_id)
        ):
            _fail("invalid_plan")
        _verify_grant_public(definition, grant, grant.receiver_id)
        prior = grant.receiver_id
        entries.append(ActivationPlanEntry(sha256(grant.receiver_id), sha256(data)))
    expected = ActivationPlan(
        prepared.channel_id, prepared.base_epoch, prepared.new_epoch, tuple(entries)
    )
    if plan != expected:
        _fail("invalid_plan")
    return plan


def _public_put(
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


def _backend_call[T](operation: Callable[[], T]) -> T:
    try:
        return operation()
    except EpochActivationError:
        raise
    except Exception:
        _fail("storage_error")


def _crypto_call[T](operation: Callable[[], T]) -> T:
    try:
        return operation()
    except EpochActivationError:
        raise
    except Exception:
        _fail("crypto_error")


def _serialize_grant(grant: PortableKeyGrant) -> bytes:
    return _crypto_call(lambda: grant_serialize(grant))


def _deserialize_grant(data: bytes) -> PortableKeyGrant:
    return _crypto_call(lambda: grant_deserialize(data))


def _verify_grant_public(
    definition: ChannelDefinition,
    grant: PortableKeyGrant,
    receiver_id: bytes,
) -> None:
    _crypto_call(
        lambda: verify_grant_signature(
            grant,
            definition.originator.agent_id,
            receiver_id,
            definition.channel_id,
            definition.originator.public_key,
        )
    )


def _fail(code: EpochActivationErrorCode) -> NoReturn:
    raise EpochActivationError(code)
