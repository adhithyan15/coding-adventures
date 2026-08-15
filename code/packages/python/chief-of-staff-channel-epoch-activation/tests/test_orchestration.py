"""Crash, retry, activation, publish, and destruction orchestration tests."""

from collections.abc import Callable
from dataclasses import replace

import pytest
from coding_adventures_chief_of_staff_channel_crypto import (
    ChannelMasterKey,
    OriginatorSigningKey,
    ReceiverKeyPair,
    RotationPlan,
    RotationReceiver,
    plan_rotation,
)
from coding_adventures_chief_of_staff_channel_store import (
    CHANNEL_STORAGE_NAMESPACE,
    ChannelDefinition,
    ChannelDefinitionStore,
    MemoryChannelStorage,
    OriginatorIdentity,
    ReceiverIdentity,
    StorageRecord,
    sequence_state_record_key,
)

from coding_adventures_chief_of_staff_channel_epoch_activation import (
    ActiveEpochAppendRequest,
    CustodyError,
    EpochActivationError,
    EpochActivationStore,
    InMemoryKeyCustody,
    prepare_rotation_candidate,
)

CHANNEL_ID = bytes.fromhex("018f47a09b6c7def923456789abcdef0")
MESSAGE_ID = bytes.fromhex("018f47a09b6c7def923456789abcdef1")
CURRENT_CMK = bytes([0x22]) * 32
NEXT_CMK = bytes([0x33]) * 32


class DurableMemoryCustody(InMemoryKeyCustody):
    durable = True


class Context:
    def __init__(self, *, production: bool = False) -> None:
        self.signer = OriginatorSigningKey.from_seed(bytes([0x11]) * 32)
        self.receiver_a_key = ReceiverKeyPair.from_private_key(bytes([0x41]) * 32)
        self.receiver_b_key = ReceiverKeyPair.from_private_key(bytes([0x42]) * 32)
        self.receiver_a = ReceiverIdentity(
            b"receiver-a", self.receiver_a_key.public_key
        )
        self.receiver_b = ReceiverIdentity(
            b"receiver-b", self.receiver_b_key.public_key
        )
        self.definition = ChannelDefinition(
            CHANNEL_ID,
            OriginatorIdentity(b"originator", self.signer.public_key),
            (self.receiver_a, self.receiver_b),
            1_725_000_000_000_000_000,
            0,
        )
        self.backend = MemoryChannelStorage()
        self.custody = DurableMemoryCustody() if production else InMemoryKeyCustody()
        opener = (
            EpochActivationStore.open
            if production
            else EpochActivationStore.open_for_testing
        )
        self.store = opener(self.backend, self.custody, CHANNEL_ID)

    def create(self) -> None:
        state = self.store.create_epoch_channel(
            self.definition, ChannelMasterKey.from_bytes(CURRENT_CMK)
        )
        assert (state.active_epoch, state.next_sequence, state.pending_header) == (
            0,
            0,
            None,
        )

    def rotation(self) -> RotationPlan:
        return plan_rotation(
            b"originator",
            CHANNEL_ID,
            0,
            ChannelMasterKey.from_bytes(NEXT_CMK),
            [
                RotationReceiver.with_material(
                    self.receiver_b.agent_id,
                    self.receiver_b.public_key,
                    bytes([0x51]) * 32,
                    bytes([0x61]) * 24,
                )
            ],
            self.signer,
        )

    def close(self) -> None:
        self.signer.destroy()
        self.receiver_a_key.destroy()
        self.receiver_b_key.destroy()


def test_production_rejects_non_durable_custody_and_accepts_durable() -> None:
    backend = MemoryChannelStorage()
    with pytest.raises(EpochActivationError, match="custody_error"):
        EpochActivationStore.open(backend, InMemoryKeyCustody(), CHANNEL_ID)
    context = Context(production=True)
    context.create()
    context.close()


def test_custody_first_creation_is_idempotent_and_conflicts_fail_closed() -> None:
    context = Context()
    context.create()
    same = context.store.create_epoch_channel(
        context.definition, ChannelMasterKey.from_bytes(CURRENT_CMK)
    )
    assert same.active_epoch == 0
    with pytest.raises(EpochActivationError, match="conflicting_active_key"):
        context.store.create_epoch_channel(
            context.definition, ChannelMasterKey.from_bytes(bytes([0x99]) * 32)
        )
    context.close()


def test_prepare_retry_recovery_activation_and_prospective_revocation() -> None:
    context = Context()
    context.create()
    assert (
        context.store.prepare_rotation(
            context.definition, (context.receiver_b,), context.rotation()
        )
        == "prepared"
    )
    assert context.store.activation_plan(1) is not None
    assert context.store.recover_preparation(context.definition, 1) == "idempotent"
    assert context.store.activate_prepared_epoch(context.definition, 1) == "activated"
    assert context.store.activate_prepared_epoch(context.definition, 1) == "idempotent"
    assert context.store.state().active_epoch == 1
    assert context.custody.resolve_handle(CHANNEL_ID, 0) is not None
    assert context.custody.resolve_handle(CHANNEL_ID, 1) is not None
    context.close()


def test_crash_after_custody_selection_replays_public_records() -> None:
    context = Context()
    context.create()
    prepared = prepare_rotation_candidate(
        context.definition, 0, (context.receiver_b,), context.rotation()
    )
    assert repr(prepared) == "PreparedEpoch([REDACTED])"
    assert context.custody.prepare_if_absent(prepared) == "selected"
    assert context.custody.prepare_if_absent(prepared) == "idempotent"
    prepared.destroy()
    assert context.store.activation_plan(1) is None
    assert context.store.recover_preparation(context.definition, 1) == "idempotent"
    assert context.store.activation_plan(1) is not None
    context.close()


def test_pending_publish_serializes_rotation_and_uses_active_epoch() -> None:
    context = Context()
    context.create()
    request = ActiveEpochAppendRequest(
        MESSAGE_ID,
        1_725_000_000_000_000_001,
        b"originator",
        "application/octet-stream",
        0,
    )
    reservation = context.store.reserve_publish_using_active_epoch(
        context.definition, request, b"hello"
    )
    assert reservation.header.sequence == 0
    with pytest.raises(EpochActivationError, match="pending_append"):
        context.store.prepare_rotation(
            context.definition, (context.receiver_b,), context.rotation()
        )
    message = context.store.commit_reserved(
        context.definition, reservation, b"hello", context.signer
    )
    assert message.key_epoch == 0
    assert context.store.state().pending_header is None
    assert (
        context.store.commit_reserved(
            context.definition, reservation, b"hello", context.signer
        )
        == message
    )
    context.close()


def test_unactivated_epoch_is_rejected_and_pending_can_be_abandoned() -> None:
    context = Context()
    context.create()
    request = ActiveEpochAppendRequest(
        MESSAGE_ID,
        1,
        b"originator",
        "application/octet-stream",
        1,
    )
    with pytest.raises(EpochActivationError, match="unactivated_epoch"):
        context.store.reserve_publish_using_active_epoch(
            context.definition, request, b"hello"
        )
    reservation = context.store.reserve_publish_using_active_epoch(
        context.definition, replace(request, key_epoch=0), b"hello"
    )
    assert context.store.abandon_pending() == reservation.header
    assert context.store.abandon_pending() is None
    context.close()


def test_additional_fail_closed_preconditions_and_redacted_handles() -> None:
    context = Context()
    context.create()
    handle = context.custody.resolve_handle(CHANNEL_ID, 0)
    assert handle is not None
    assert repr(handle) == "EpochKeyHandle([REDACTED])"
    assert context.custody.with_key(handle, lambda cmk: cmk.bytes) == CURRENT_CMK
    missing = replace(handle, epoch=99)
    with pytest.raises(CustodyError, match="custody_error"):
        context.custody.with_key(missing, lambda cmk: cmk.bytes)
    with pytest.raises(EpochActivationError, match="preparation_missing"):
        context.store.activate_prepared_epoch(context.definition, 1)
    with pytest.raises(EpochActivationError, match="unexpected_epoch"):
        context.store.recover_preparation(context.definition, 2)
    with pytest.raises(EpochActivationError, match="invalid_plan"):
        context.store.reserve_publish_using_active_epoch(
            context.definition,
            ActiveEpochAppendRequest(
                MESSAGE_ID, 1, b"not-originator", "application/octet-stream"
            ),
            b"hello",
        )
    isolated = EpochActivationStore.open_for_testing(
        context.backend, InMemoryKeyCustody(), CHANNEL_ID
    )
    with pytest.raises(EpochActivationError, match="active_key_missing"):
        isolated.migrate_epoch_state(context.definition)
    context.close()


def test_invalid_rotation_roster_is_rejected_and_consumed() -> None:
    context = Context()
    context.create()
    with pytest.raises(EpochActivationError, match="invalid_plan"):
        context.store.prepare_rotation(context.definition, (), context.rotation())
    context.close()


def test_corrupt_state_and_conflicting_public_records_fail_closed() -> None:
    context = Context()
    context.create()
    key = sequence_state_record_key(CHANNEL_ID)
    record = context.backend.get(CHANNEL_STORAGE_NAMESPACE, key)
    assert record is not None
    context.backend.corrupt(
        StorageRecord(
            record.namespace,
            record.key,
            record.content_type,
            record.body + b"\x00",
            record.revision,
            {},
        )
    )
    with pytest.raises(EpochActivationError, match="corrupt_record"):
        context.store.state()
    context.close()


def test_destroy_wipes_custody_but_retains_public_history() -> None:
    context = Context()
    context.create()
    context.store.prepare_rotation(
        context.definition, (context.receiver_b,), context.rotation()
    )
    context.store.activate_prepared_epoch(context.definition, 1)
    destroyed = ChannelDefinitionStore(context.backend).destroy(CHANNEL_ID)
    plan = context.store.activation_plan(1)
    context.store.apply_destruction(destroyed)
    assert context.custody.retained_key_count == 0
    assert context.store.activation_plan(1) == plan
    with pytest.raises(EpochActivationError, match="channel_destroyed"):
        context.store.reserve_publish_using_active_epoch(
            destroyed,
            ActiveEpochAppendRequest(
                MESSAGE_ID, 1, b"originator", "application/octet-stream"
            ),
            b"hello",
        )
    context.close()


@pytest.mark.parametrize(
    ("operation", "code"),
    [
        (lambda context: context.store.state(), "not_initialized"),
        (
            lambda context: context.store.recover_preparation(context.definition, 1),
            "not_initialized",
        ),
    ],
)
def test_stable_precondition_errors(
    operation: Callable[[Context], object], code: str
) -> None:
    context = Context()
    with pytest.raises(EpochActivationError) as raised:
        operation(context)
    assert raised.value.code == code
    assert str(raised.value) == code
    context.close()
