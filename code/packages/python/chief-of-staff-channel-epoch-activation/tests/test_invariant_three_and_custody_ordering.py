"""Regression tests for two defects found in the Go D18T security review.

Both defects existed here too, in already-shipped code. Both tests below are
written to FAIL if their fix is reverted -- that property is the whole point,
because the first attempt at the invariant-3 test in the Go port passed even
with the entire check deleted.
"""

from typing import Literal

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
    ChannelDefinition,
    ChannelDefinitionStore,
    MemoryChannelStorage,
    OriginatorIdentity,
    ReceiverIdentity,
    StoragePage,
    StoragePut,
    StorageRecord,
    key_grant_record_key,
)

from coding_adventures_chief_of_staff_channel_epoch_activation import (
    EpochActivationError,
    EpochActivationStore,
    InMemoryKeyCustody,
)

CHANNEL_ID = bytes.fromhex("018f47a09b6c7def923456789abcdef0")
CURRENT_CMK = bytes([0x22]) * 32
NEXT_CMK = bytes([0x33]) * 32

GrantReadFault = Literal["healthy", "vanishes", "mutated_body", "mutated_content_type"]


class GrantFaultBackend:
    """Accepts writes normally; diverges only on reads of one grant key.

    This models a write-behind or eventually-consistent backend, whose echoed
    put result does not prove the record is retrievable. It is precisely the
    case ``_put_immutable`` structurally cannot observe, because that helper
    only ever reads a key it has just failed to create.
    """

    def __init__(self, inner: MemoryChannelStorage, grant_key: str) -> None:
        self._inner = inner
        self._grant_key = grant_key
        self.fault: GrantReadFault = "healthy"
        # Leave the first read healthy: _put_immutable's conflict re-get would
        # otherwise absorb the fault and return its own conflicting_grant, so
        # the phase-6 loop would never be reached and the test would silently
        # measure the wrong check.
        self.skip_reads = 1

    def initialize(self) -> None:
        self._inner.initialize()

    def get(self, namespace: str, key: str) -> StorageRecord | None:
        record = self._inner.get(namespace, key)
        if record is None or key != self._grant_key:
            return record
        if self.skip_reads > 0:
            self.skip_reads -= 1
            return record
        if self.fault == "vanishes":
            return None
        if self.fault == "mutated_body":
            return StorageRecord(
                record.namespace,
                record.key,
                record.content_type,
                record.body + b"\x00",
                record.revision,
                record.metadata,
            )
        if self.fault == "mutated_content_type":
            return StorageRecord(
                record.namespace,
                record.key,
                "application/vnd.wrong",
                record.body,
                record.revision,
                record.metadata,
            )
        return record

    def put(self, value: StoragePut) -> StorageRecord:
        return self._inner.put(value)

    def list(
        self,
        namespace: str,
        *,
        prefix: str,
        recursive: bool,
        page_size: int,
        cursor: str | None = None,
    ) -> StoragePage:
        return self._inner.list(
            namespace,
            prefix=prefix,
            recursive=recursive,
            page_size=page_size,
            cursor=cursor,
        )


class Context:
    def __init__(self) -> None:
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
        self.custody = InMemoryKeyCustody()
        self.store = EpochActivationStore.open_for_testing(
            self.backend, self.custody, CHANNEL_ID
        )

    def create(self) -> None:
        self.store.create_epoch_channel(
            self.definition, ChannelMasterKey.from_bytes(CURRENT_CMK)
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


@pytest.mark.parametrize(
    "fault", ["vanishes", "mutated_body", "mutated_content_type"]
)
def test_activation_refuses_when_a_grant_is_not_retrievable(
    fault: GrantReadFault,
) -> None:
    """Invariant 3: all grants durable and READABLE before visibility.

    Writing a grant successfully is not the same as being able to read it back.
    If activation trusts the echoed put, it can advance to E+1 while a
    receiver's grant is unretrievable -- locking out a receiver that was
    authorized for that epoch.
    """
    context = Context()
    context.create()
    context.store.prepare_rotation(
        context.definition, (context.receiver_b,), context.rotation()
    )

    grant_key = key_grant_record_key(CHANNEL_ID, 1, context.receiver_b.agent_id)
    faulty = GrantFaultBackend(context.backend, grant_key)
    store = EpochActivationStore.open_for_testing(faulty, context.custody, CHANNEL_ID)
    faulty.fault = fault

    with pytest.raises(EpochActivationError) as raised:
        store.activate_prepared_epoch(context.definition, 1)
    assert raised.value.code == "corrupt_record"

    # And the epoch must not have advanced.
    faulty.fault = "healthy"
    assert store.state().active_epoch == 0
    context.close()


def test_grant_read_back_is_reached_at_all() -> None:
    """Guard against the phase-6 loop being skipped entirely.

    Without this, a refactor that drops the read-back would leave the tests
    above still passing via some earlier check, which is exactly how the Go
    port's first attempt at this test went wrong.
    """
    context = Context()
    context.create()
    context.store.prepare_rotation(
        context.definition, (context.receiver_b,), context.rotation()
    )
    grant_key = key_grant_record_key(CHANNEL_ID, 1, context.receiver_b.agent_id)

    reads: list[str] = []

    class CountingBackend(GrantFaultBackend):
        def get(self, namespace: str, key: str) -> StorageRecord | None:
            if key == grant_key:
                reads.append(key)
            return MemoryChannelStorage.get(self._inner, namespace, key)

    counting = CountingBackend(context.backend, grant_key)
    store = EpochActivationStore.open_for_testing(counting, context.custody, CHANNEL_ID)
    store.activate_prepared_epoch(context.definition, 1)

    # One read from _put_immutable's conflict path, one from phase 6.
    assert len(reads) >= 2, (
        f"expected the grant to be re-read during replay, saw {len(reads)} read(s)"
    )
    context.close()


def test_mismatched_definition_cannot_wedge_the_custody_slot() -> None:
    """A failed create must not leave the real channel permanently unopenable.

    The vulnerable state is a channel whose definition is already durable but
    whose D18T custody import never completed -- reachable by a crash between
    the two, or by a D18P channel awaiting migration.

    Custody slots are keyed by (channel_id, epoch) and the first writer wins
    permanently. Importing before validating the definition lets any caller
    claim the slot with its own CMK and then fail, after which the legitimate
    create hits conflicting_active_key forever. Fail-closed, but unrecoverable.
    """
    context = Context()
    # Definition durable, custody empty.
    ChannelDefinitionStore(context.backend).create(context.definition)
    assert context.custody.retained_key_count == 0

    # Same channel and epoch, different membership, and crucially a DIFFERENT
    # CMK -- reusing the real one would make the pre-fix import merely
    # idempotent and the test would pass either way.
    impostor = ChannelDefinition(
        CHANNEL_ID,
        OriginatorIdentity(b"originator", context.signer.public_key),
        (context.receiver_a,),
        1_725_000_000_000_000_000,
        0,
    )
    with pytest.raises(EpochActivationError) as raised:
        context.store.create_epoch_channel(
            impostor, ChannelMasterKey.from_bytes(bytes([0x99]) * 32)
        )
    assert raised.value.code == "invalid_plan"

    # The slot was never claimed, so the legitimate owner can still open it.
    assert context.custody.retained_key_count == 0
    state = context.store.create_epoch_channel(
        context.definition, ChannelMasterKey.from_bytes(CURRENT_CMK)
    )
    assert state.active_epoch == 0
    context.close()


def test_create_erases_the_callers_key_on_every_rejection_path() -> None:
    """An unused secret must not outlive a refused create.

    Not a regression test for the reordering -- the pre-fix code also erased on
    this path. It pins the property the reordering had to preserve: moving the
    import later must not introduce an exit that forgets the key.
    """
    context = Context()
    foreign = ChannelDefinition(
        bytes.fromhex("018f47a09b6c7def923456789abcdef9"),
        OriginatorIdentity(b"originator", context.signer.public_key),
        (context.receiver_a,),
        1_725_000_000_000_000_000,
        0,
    )
    cmk = ChannelMasterKey.from_bytes(CURRENT_CMK)
    with pytest.raises(EpochActivationError) as raised:
        context.store.create_epoch_channel(foreign, cmk)
    assert raised.value.code == "invalid_plan"
    # A destroyed key refuses to hand its bytes back.
    assert repr(cmk) == "ChannelMasterKey(<destroyed>)"
    context.close()
