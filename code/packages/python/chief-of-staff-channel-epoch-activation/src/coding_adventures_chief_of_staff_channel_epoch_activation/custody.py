"""Injected atomic originator-key custody for D18T."""

from __future__ import annotations

import hmac
from collections.abc import Callable
from dataclasses import dataclass
from typing import Literal, Protocol, TypeVar

from coding_adventures_chief_of_staff_channel_crypto import ChannelMasterKey

CustodySelection = Literal["selected", "idempotent", "conflict"]
T = TypeVar("T")


class CustodyError(RuntimeError):
    """Secret-free custody failure."""

    code = "custody_error"

    def __init__(self) -> None:
        super().__init__(self.code)


@dataclass(frozen=True, slots=True, repr=False)
class EpochKeyHandle:
    """Opaque, redacted reference to one retained epoch key."""

    channel_id: bytes
    epoch: int

    def __post_init__(self) -> None:
        object.__setattr__(self, "channel_id", bytes(self.channel_id))

    def __repr__(self) -> str:
        return "EpochKeyHandle([REDACTED])"


@dataclass(frozen=True, slots=True, init=False)
class PublicPreparation:
    """Exact secret-free recovery bundle retained beside a prepared CMK."""

    channel_id: bytes
    base_epoch: int
    new_epoch: int
    plan_bytes: bytes
    grants: tuple[bytes, ...]

    def __init__(
        self,
        channel_id: bytes,
        base_epoch: int,
        new_epoch: int,
        plan_bytes: bytes,
        grants: tuple[bytes, ...],
    ) -> None:
        object.__setattr__(self, "channel_id", bytes(channel_id))
        object.__setattr__(self, "base_epoch", base_epoch)
        object.__setattr__(self, "new_epoch", new_epoch)
        object.__setattr__(self, "plan_bytes", bytes(plan_bytes))
        object.__setattr__(self, "grants", tuple(bytes(grant) for grant in grants))

    def clone(self) -> PublicPreparation:
        return PublicPreparation(
            self.channel_id,
            self.base_epoch,
            self.new_epoch,
            self.plan_bytes,
            self.grants,
        )


class PreparedEpoch:
    """One indivisible candidate offered to custody."""

    __slots__ = ("_cmk", "public_preparation")

    def __init__(
        self, public_preparation: PublicPreparation, cmk: ChannelMasterKey
    ) -> None:
        self.public_preparation = public_preparation.clone()
        self._cmk = cmk.clone()

    def clone_cmk(self) -> ChannelMasterKey:
        return self._cmk.clone()

    def destroy(self) -> None:
        self._cmk.destroy()

    def __repr__(self) -> str:
        return "PreparedEpoch([REDACTED])"


class OriginatorKeyCustody(Protocol):
    """Atomic, restart-safe custody boundary; production must be durable."""

    durable: bool

    def import_active_if_absent(
        self, channel_id: bytes, epoch: int, cmk: ChannelMasterKey
    ) -> CustodySelection: ...

    def resolve_handle(
        self, channel_id: bytes, epoch: int
    ) -> EpochKeyHandle | None: ...

    def prepare_if_absent(self, prepared: PreparedEpoch) -> CustodySelection: ...

    def load_preparation(
        self, channel_id: bytes, new_epoch: int
    ) -> PublicPreparation | None: ...

    def with_key(
        self,
        handle: EpochKeyHandle,
        operation: Callable[[ChannelMasterKey], T],
    ) -> T: ...

    def destroy_channel(self, channel_id: bytes) -> None: ...


class InMemoryKeyCustody:
    """Deterministic, explicitly non-durable custody for conformance tests."""

    durable = False

    def __init__(self) -> None:
        self._keys: dict[tuple[bytes, int], ChannelMasterKey] = {}
        self._preparations: dict[tuple[bytes, int], PublicPreparation] = {}

    def import_active_if_absent(
        self, channel_id: bytes, epoch: int, cmk: ChannelMasterKey
    ) -> CustodySelection:
        slot = (bytes(channel_id), epoch)
        current = self._keys.get(slot)
        if current is None:
            self._keys[slot] = cmk.clone()
            return "selected"
        return "idempotent" if _same_cmk(current, cmk) else "conflict"

    def resolve_handle(self, channel_id: bytes, epoch: int) -> EpochKeyHandle | None:
        channel = bytes(channel_id)
        return (
            EpochKeyHandle(channel, epoch) if (channel, epoch) in self._keys else None
        )

    def prepare_if_absent(self, prepared: PreparedEpoch) -> CustodySelection:
        public = prepared.public_preparation
        slot = (public.channel_id, public.new_epoch)
        current_public = self._preparations.get(slot)
        current_cmk = self._keys.get(slot)
        if current_public is None and current_cmk is None:
            self._preparations[slot] = public.clone()
            self._keys[slot] = prepared.clone_cmk()
            return "selected"
        if current_public is None or current_cmk is None or current_public != public:
            return "conflict"
        candidate = prepared.clone_cmk()
        try:
            return "idempotent" if _same_cmk(current_cmk, candidate) else "conflict"
        finally:
            candidate.destroy()

    def load_preparation(
        self, channel_id: bytes, new_epoch: int
    ) -> PublicPreparation | None:
        preparation = self._preparations.get((bytes(channel_id), new_epoch))
        return None if preparation is None else preparation.clone()

    def with_key(
        self, handle: EpochKeyHandle, operation: Callable[[ChannelMasterKey], T]
    ) -> T:
        cmk = self._keys.get((handle.channel_id, handle.epoch))
        if cmk is None:
            raise CustodyError
        transient = cmk.clone()
        try:
            return operation(transient)
        finally:
            transient.destroy()

    def destroy_channel(self, channel_id: bytes) -> None:
        channel = bytes(channel_id)
        for slot, cmk in tuple(self._keys.items()):
            if slot[0] == channel:
                cmk.destroy()
                del self._keys[slot]
        for slot in tuple(self._preparations):
            if slot[0] == channel:
                del self._preparations[slot]

    @property
    def retained_key_count(self) -> int:
        return len(self._keys)


def _same_cmk(left: ChannelMasterKey, right: ChannelMasterKey) -> bool:
    return hmac.compare_digest(left.bytes, right.bytes)
