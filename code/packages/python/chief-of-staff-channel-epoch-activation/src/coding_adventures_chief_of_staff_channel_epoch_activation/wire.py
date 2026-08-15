"""Exact D18S v2 state and D18T v1 activation-plan codecs."""

from __future__ import annotations

from dataclasses import dataclass
from typing import NoReturn

from coding_adventures_chief_of_staff_channel_store import (
    MAX_PENDING_HEADER_BYTES,
    MessageHeader,
    message_header_deserialize,
    message_header_serialize,
    validate_uuid_v7,
)

EPOCH_STATE_CONTENT_TYPE = "application/vnd.coding-adventures.chief-channel-state-v2"
ACTIVATION_PLAN_CONTENT_TYPE = (
    "application/vnd.coding-adventures.chief-channel-epoch-activation-v1"
)
MAX_PLAN_RECEIVERS = 1024
MAX_U64 = (1 << 64) - 1


class EpochWireError(ValueError):
    """Stable failure for malformed or non-canonical public records."""

    code = "corrupt_record"

    def __init__(self) -> None:
        super().__init__(self.code)


@dataclass(frozen=True, slots=True, init=False)
class EpochState:
    """D18S v2 active epoch, next sequence, and optional reservation."""

    active_epoch: int
    next_sequence: int
    pending_header: MessageHeader | None

    def __init__(
        self,
        channel_id: bytes,
        active_epoch: int,
        next_sequence: int,
        pending_header: MessageHeader | None = None,
    ) -> None:
        _require_u64(active_epoch)
        _require_u64(next_sequence)
        if pending_header is not None and (
            pending_header.channel_id != channel_id
            or pending_header.sequence == MAX_U64
            or pending_header.sequence + 1 != next_sequence
            or pending_header.key_epoch != active_epoch
        ):
            _fail()
        object.__setattr__(self, "active_epoch", active_epoch)
        object.__setattr__(self, "next_sequence", next_sequence)
        object.__setattr__(self, "pending_header", pending_header)

    def with_active_epoch(self, channel_id: bytes, active_epoch: int) -> EpochState:
        return EpochState(
            channel_id, active_epoch, self.next_sequence, self.pending_header
        )

    def with_pending(
        self,
        channel_id: bytes,
        next_sequence: int,
        pending_header: MessageHeader | None = None,
    ) -> EpochState:
        return EpochState(channel_id, self.active_epoch, next_sequence, pending_header)


@dataclass(frozen=True, slots=True)
class ActivationPlanEntry:
    receiver_id_hash: bytes
    grant_hash: bytes

    def __post_init__(self) -> None:
        receiver = bytes(self.receiver_id_hash)
        grant = bytes(self.grant_hash)
        if len(receiver) != 32 or len(grant) != 32:
            _fail()
        object.__setattr__(self, "receiver_id_hash", receiver)
        object.__setattr__(self, "grant_hash", grant)


@dataclass(frozen=True, slots=True, init=False)
class ActivationPlan:
    channel_id: bytes
    base_epoch: int
    new_epoch: int
    receivers: tuple[ActivationPlanEntry, ...]

    def __init__(
        self,
        channel_id: bytes,
        base_epoch: int,
        new_epoch: int,
        receivers: tuple[ActivationPlanEntry, ...],
    ) -> None:
        channel = bytes(channel_id)
        try:
            validate_uuid_v7(channel)
        except Exception:
            _fail()
        _require_u64(base_epoch)
        _require_u64(new_epoch)
        ordered = tuple(sorted(receivers, key=lambda entry: entry.receiver_id_hash))
        if (
            base_epoch == MAX_U64
            or new_epoch != base_epoch + 1
            or not 1 <= len(ordered) <= MAX_PLAN_RECEIVERS
        ):
            _fail()
        if len({entry.receiver_id_hash for entry in ordered}) != len(ordered):
            _fail()
        if len({entry.grant_hash for entry in ordered}) != len(ordered):
            _fail()
        object.__setattr__(self, "channel_id", channel)
        object.__setattr__(self, "base_epoch", base_epoch)
        object.__setattr__(self, "new_epoch", new_epoch)
        object.__setattr__(self, "receivers", ordered)


def epoch_state_serialize(state: EpochState) -> bytes:
    result = b"D18S" + b"\x02" + _u64(state.active_epoch) + _u64(state.next_sequence)
    if state.pending_header is None:
        return result + b"\x00"
    header = message_header_serialize(state.pending_header)
    if len(header) > MAX_PENDING_HEADER_BYTES:
        _fail()
    return result + b"\x01" + _u32(len(header)) + header


def epoch_state_deserialize(data: bytes, channel_id: bytes) -> EpochState:
    try:
        reader = _Reader(data)
        if reader.take(4) != b"D18S" or reader.u8() != 2:
            _fail()
        active_epoch = reader.u64()
        next_sequence = reader.u64()
        flag = reader.u8()
        pending: MessageHeader | None = None
        if flag == 1:
            length = reader.u32()
            if length > MAX_PENDING_HEADER_BYTES:
                _fail()
            pending = message_header_deserialize(reader.take(length))
        elif flag != 0:
            _fail()
        reader.finish()
        return EpochState(channel_id, active_epoch, next_sequence, pending)
    except EpochWireError:
        raise
    except Exception:
        _fail()


def activation_plan_serialize(plan: ActivationPlan) -> bytes:
    parts = [
        b"D18T",
        b"\x01",
        plan.channel_id,
        _u64(plan.base_epoch),
        _u64(plan.new_epoch),
        _u32(len(plan.receivers)),
    ]
    for receiver in plan.receivers:
        parts.extend((receiver.receiver_id_hash, receiver.grant_hash))
    return b"".join(parts)


def activation_plan_deserialize(data: bytes) -> ActivationPlan:
    try:
        reader = _Reader(data)
        if reader.take(4) != b"D18T" or reader.u8() != 1:
            _fail()
        channel_id = reader.take(16)
        base_epoch = reader.u64()
        new_epoch = reader.u64()
        count = reader.u32()
        if not 1 <= count <= MAX_PLAN_RECEIVERS:
            _fail()
        entries = tuple(
            ActivationPlanEntry(reader.take(32), reader.take(32)) for _ in range(count)
        )
        reader.finish()
        if any(
            left.receiver_id_hash >= right.receiver_id_hash
            for left, right in zip(entries, entries[1:], strict=False)
        ):
            _fail()
        plan = ActivationPlan(channel_id, base_epoch, new_epoch, entries)
        if plan.receivers != entries:
            _fail()
        return plan
    except EpochWireError:
        raise
    except Exception:
        _fail()


def activation_plan_record_key(channel_id: bytes, new_epoch: int) -> str:
    if len(channel_id) != 16:
        _fail()
    _require_u64(new_epoch)
    return f"{channel_id.hex()}/epochs/{new_epoch:020d}/activation"


class _Reader:
    def __init__(self, source: bytes) -> None:
        self.source = bytes(source)
        self.offset = 0

    def take(self, length: int) -> bytes:
        if length < 0 or self.offset + length > len(self.source):
            _fail()
        result = self.source[self.offset : self.offset + length]
        self.offset += length
        return result

    def u8(self) -> int:
        return self.take(1)[0]

    def u32(self) -> int:
        return int.from_bytes(self.take(4), "big")

    def u64(self) -> int:
        return int.from_bytes(self.take(8), "big")

    def finish(self) -> None:
        if self.offset != len(self.source):
            _fail()


def _u32(value: int) -> bytes:
    if (
        not isinstance(value, int)
        or isinstance(value, bool)
        or not 0 <= value < 1 << 32
    ):
        _fail()
    return value.to_bytes(4, "big")


def _u64(value: int) -> bytes:
    _require_u64(value)
    return value.to_bytes(8, "big")


def _require_u64(value: int) -> None:
    if (
        not isinstance(value, int)
        or isinstance(value, bool)
        or not 0 <= value <= MAX_U64
    ):
        _fail()


def _fail() -> NoReturn:
    raise EpochWireError
