"""Portable D18Q channel-key grants for Chief of Staff channels."""

from __future__ import annotations

import os
from dataclasses import dataclass
from typing import Literal, NoReturn, Protocol

from coding_adventures_chacha20_poly1305 import (  # type: ignore[import-untyped]
    xchacha20_poly1305_aead_decrypt,
    xchacha20_poly1305_aead_encrypt,
)
from coding_adventures_ed25519 import (  # type: ignore[import-untyped]
    generate_keypair as generate_ed25519_keypair,
)
from coding_adventures_ed25519 import sign, verify  # type: ignore[import-untyped]
from coding_adventures_hkdf import hkdf  # type: ignore[import-untyped]
from coding_adventures_x25519 import (  # type: ignore[import-untyped]
    generate_keypair as x25519_public_key,
)
from coding_adventures_x25519 import x25519  # type: ignore[import-untyped]

_GRANT_MAGIC = b"D18G"
_WIRE_VERSION = 1
_MAX_IDENTITY_BYTES = 4096
_MAX_U64 = (1 << 64) - 1
_KEY_GRANT_CONTEXT = b"chief-channel-key-grant-v1"
_KEY_WRAP_CONTEXT = b"chief-channel-key-wrap-v1"
_ROTATION_PLAN_TOKEN = object()

KEY_GRANT_ERROR_CODES = (
    "invalid_magic",
    "unsupported_version",
    "truncated_record",
    "trailing_bytes",
    "length_limit_exceeded",
    "invalid_field",
    "randomness_unavailable",
    "invalid_key_agreement",
    "key_derivation_failed",
    "invalid_signature",
    "unexpected_originator",
    "unexpected_receiver",
    "unexpected_channel",
    "authentication_failed",
    "invalid_wrapped_key",
    "conflicting_grant",
    "decreasing_epoch",
    "epoch_exhausted",
    "missing_epoch_key",
)

KeyGrantErrorCode = Literal[
    "invalid_magic",
    "unsupported_version",
    "truncated_record",
    "trailing_bytes",
    "length_limit_exceeded",
    "invalid_field",
    "randomness_unavailable",
    "invalid_key_agreement",
    "key_derivation_failed",
    "invalid_signature",
    "unexpected_originator",
    "unexpected_receiver",
    "unexpected_channel",
    "authentication_failed",
    "invalid_wrapped_key",
    "conflicting_grant",
    "decreasing_epoch",
    "epoch_exhausted",
    "missing_epoch_key",
]
GrantInstallOutcome = Literal["installed", "idempotent"]
SecretErasureCapability = Literal["guaranteed", "best_effort", "not_enforceable"]


class KeyGrantProfileError(ValueError):
    """One fail-closed D18Q operation error with a stable portable code."""

    code: KeyGrantErrorCode

    def __init__(self, code: KeyGrantErrorCode) -> None:
        super().__init__(code)
        self.code = code


class SecureRandomSource(Protocol):
    """Cryptographically secure byte source for production convenience APIs."""

    def random_bytes(self, length: int) -> bytes:
        """Return exactly ``length`` independent secure random octets."""


class _SystemSecureRandomSource:
    __slots__ = ()

    def random_bytes(self, length: int) -> bytes:
        return os.urandom(length)


SYSTEM_SECURE_RANDOM_SOURCE: SecureRandomSource = _SystemSecureRandomSource()


class ChannelMasterKey:
    """Owned 32-byte CMK with explicit logical destruction."""

    __slots__ = ("_bytes", "_destroyed")

    def __init__(self, value: bytes) -> None:
        copied = _copy_bytes(value)
        _require_length(copied, 32)
        self._bytes = bytearray(copied)
        self._destroyed = False

    @classmethod
    def from_bytes(cls, value: bytes) -> ChannelMasterKey:
        """Copy exactly 32 bytes into a managed secret container."""
        return cls(value)

    @classmethod
    def generate(
        cls,
        source: SecureRandomSource = SYSTEM_SECURE_RANDOM_SOURCE,
    ) -> ChannelMasterKey:
        """Generate a CMK from a complete CSPRNG request."""
        return cls(_secure_random_bytes(source, 32))

    @property
    def bytes(self) -> bytes:
        """Return an immutable copy of the live CMK."""
        self._require_alive()
        return bytes(self._bytes)

    def clone(self) -> ChannelMasterKey:
        """Return an independent managed copy."""
        return ChannelMasterKey(self.bytes)

    def destroy(self) -> None:
        """Overwrite this owned mutable buffer and make the object unusable."""
        _wipe(self._bytes)
        self._destroyed = True

    def _require_alive(self) -> None:
        if self._destroyed:
            _fail("invalid_field")

    def __repr__(self) -> str:
        if self._destroyed:
            return "ChannelMasterKey(<destroyed>)"
        return "ChannelMasterKey(<secret>)"


class ReceiverKeyPair:
    """Receiver X25519 key pair with no private-key accessor."""

    __slots__ = ("_destroyed", "_private_key", "_public_key")

    def __init__(self, private_key: bytes, public_key: bytes) -> None:
        self._private_key = bytearray(private_key)
        self._public_key = bytes(public_key)
        self._destroyed = False

    @classmethod
    def from_private_key(cls, private_key: bytes) -> ReceiverKeyPair:
        """Derive one public key from an explicitly supplied 32-byte scalar."""
        private_copy = _copy_bytes(private_key)
        _require_length(private_copy, 32)
        try:
            public_key = x25519_public_key(private_copy)
        except (ArithmeticError, TypeError, ValueError):
            _fail("invalid_key_agreement")
        return cls(private_copy, public_key)

    @classmethod
    def generate(
        cls,
        source: SecureRandomSource = SYSTEM_SECURE_RANDOM_SOURCE,
    ) -> ReceiverKeyPair:
        """Generate a receiver key pair from one complete CSPRNG request."""
        return cls.from_private_key(_secure_random_bytes(source, 32))

    @property
    def public_key(self) -> bytes:
        """Return the receiver's immutable public key."""
        self._require_alive()
        return bytes(self._public_key)

    def agree(self, peer_public_key: bytes) -> bytes:
        """Derive one X25519 shared secret without exposing the private key."""
        self._require_alive()
        peer_copy = _copy_bytes(peer_public_key)
        _require_length(peer_copy, 32)
        try:
            return x25519(bytes(self._private_key), peer_copy)
        except (ArithmeticError, TypeError, ValueError):
            _fail("invalid_key_agreement")

    def clone(self) -> ReceiverKeyPair:
        """Return an independent managed key-pair copy."""
        self._require_alive()
        return ReceiverKeyPair(bytes(self._private_key), self._public_key)

    def destroy(self) -> None:
        """Overwrite the owned private-key buffer."""
        _wipe(self._private_key)
        self._destroyed = True

    def _require_alive(self) -> None:
        if self._destroyed:
            _fail("invalid_field")

    def __repr__(self) -> str:
        state = "destroyed" if self._destroyed else "secret"
        return f"ReceiverKeyPair(<{state}>, public_key={self._public_key.hex()})"


class OriginatorSigningKey:
    """Originator Ed25519 identity with no signing-secret accessor."""

    __slots__ = ("_destroyed", "_public_key", "_secret_key")

    def __init__(self, secret_key: bytes, public_key: bytes) -> None:
        self._secret_key = bytearray(secret_key)
        self._public_key = bytes(public_key)
        self._destroyed = False

    @classmethod
    def from_seed(cls, seed: bytes) -> OriginatorSigningKey:
        """Derive an Ed25519 identity from an explicit 32-byte seed."""
        seed_copy = _copy_bytes(seed)
        _require_length(seed_copy, 32)
        try:
            public_key, secret_key = generate_ed25519_keypair(seed_copy)
        except (ArithmeticError, TypeError, ValueError):
            _fail("invalid_field")
        return cls(secret_key, public_key)

    @classmethod
    def generate(
        cls,
        source: SecureRandomSource = SYSTEM_SECURE_RANDOM_SOURCE,
    ) -> OriginatorSigningKey:
        """Generate an Ed25519 identity from one complete CSPRNG request."""
        return cls.from_seed(_secure_random_bytes(source, 32))

    @property
    def public_key(self) -> bytes:
        """Return the immutable Ed25519 public key."""
        self._require_alive()
        return bytes(self._public_key)

    def sign(self, message: bytes) -> bytes:
        """Sign one canonical grant input."""
        self._require_alive()
        return sign(_copy_bytes(message), bytes(self._secret_key))

    def destroy(self) -> None:
        """Overwrite the owned signing-secret buffer."""
        _wipe(self._secret_key)
        self._destroyed = True

    def _require_alive(self) -> None:
        if self._destroyed:
            _fail("invalid_field")

    def __repr__(self) -> str:
        state = "destroyed" if self._destroyed else "secret"
        return f"OriginatorSigningKey(<{state}>, public_key={self._public_key.hex()})"


@dataclass(frozen=True, slots=True, init=False)
class KeyGrantFields:
    """Immutable validated logical fields for one receiver-bound grant."""

    originator_id: bytes
    receiver_id: bytes
    channel_id: bytes
    key_epoch: int

    def __init__(
        self,
        originator_id: bytes,
        receiver_id: bytes,
        channel_id: bytes,
        key_epoch: int,
    ) -> None:
        originator_copy = _copy_bytes(originator_id)
        receiver_copy = _copy_bytes(receiver_id)
        channel_copy = _copy_bytes(channel_id)
        _validate_identity(originator_copy)
        _validate_identity(receiver_copy)
        _validate_channel_id(channel_copy)
        _require_u64(key_epoch)
        object.__setattr__(self, "originator_id", originator_copy)
        object.__setattr__(self, "receiver_id", receiver_copy)
        object.__setattr__(self, "channel_id", channel_copy)
        object.__setattr__(self, "key_epoch", key_epoch)


@dataclass(frozen=True, slots=True, init=False)
class PortableKeyGrant:
    """Immutable public D18G fields; structural decoding does not imply trust."""

    originator_id: bytes
    receiver_id: bytes
    channel_id: bytes
    key_epoch: int
    ephemeral_public_key: bytes
    wrapping_nonce: bytes
    wrapped_cmk: bytes
    originator_signature: bytes

    def __init__(
        self,
        *,
        originator_id: bytes,
        receiver_id: bytes,
        channel_id: bytes,
        key_epoch: int,
        ephemeral_public_key: bytes,
        wrapping_nonce: bytes,
        wrapped_cmk: bytes,
        originator_signature: bytes,
    ) -> None:
        originator_copy = _copy_bytes(originator_id)
        receiver_copy = _copy_bytes(receiver_id)
        if (
            len(originator_copy) > _MAX_IDENTITY_BYTES
            or len(receiver_copy) > _MAX_IDENTITY_BYTES
        ):
            _fail("length_limit_exceeded")
        values: tuple[tuple[str, bytes, int], ...] = (
            ("channel_id", _copy_bytes(channel_id), 16),
            ("ephemeral_public_key", _copy_bytes(ephemeral_public_key), 32),
            ("wrapping_nonce", _copy_bytes(wrapping_nonce), 24),
            ("wrapped_cmk", _copy_bytes(wrapped_cmk), 48),
            ("originator_signature", _copy_bytes(originator_signature), 64),
        )
        _require_u64(key_epoch)
        object.__setattr__(self, "originator_id", originator_copy)
        object.__setattr__(self, "receiver_id", receiver_copy)
        object.__setattr__(self, "key_epoch", key_epoch)
        for name, value, length in values:
            _require_length(value, length)
            object.__setattr__(self, name, value)


def grant_deserialize(data: bytes) -> PortableKeyGrant:
    """Structurally decode one complete bounded D18G version 1 record."""
    decoder = _GrantDecoder(data)
    if decoder.take(4) != _GRANT_MAGIC:
        _fail("invalid_magic")
    if decoder.take(1)[0] != _WIRE_VERSION:
        _fail("unsupported_version")
    grant = PortableKeyGrant(
        originator_id=decoder.read_identity(),
        receiver_id=decoder.read_identity(),
        channel_id=decoder.take(16),
        key_epoch=decoder.read_u64(),
        ephemeral_public_key=decoder.take(32),
        wrapping_nonce=decoder.take(24),
        wrapped_cmk=decoder.take(48),
        originator_signature=decoder.take(64),
    )
    decoder.finish()
    return grant


def grant_serialize(grant: PortableKeyGrant) -> bytes:
    """Validate and encode one immutable grant as exact D18G v1 bytes."""
    _validate_grant(grant)
    return b"".join(
        (
            _GRANT_MAGIC,
            bytes((_WIRE_VERSION,)),
            len(grant.originator_id).to_bytes(4, "big"),
            grant.originator_id,
            len(grant.receiver_id).to_bytes(4, "big"),
            grant.receiver_id,
            grant.channel_id,
            _u64be(grant.key_epoch),
            grant.ephemeral_public_key,
            grant.wrapping_nonce,
            grant.wrapped_cmk,
            grant.originator_signature,
        )
    )


def seal_channel_key(
    fields: KeyGrantFields,
    cmk: ChannelMasterKey,
    receiver_public_key: bytes,
    signing_key: OriginatorSigningKey,
    source: SecureRandomSource = SYSTEM_SECURE_RANDOM_SOURCE,
) -> PortableKeyGrant:
    """Seal using independent production ephemeral-key and nonce material."""
    ephemeral_private_key = _secure_random_bytes(source, 32)
    wrapping_nonce = _secure_random_bytes(source, 24)
    return seal_channel_key_with_material(
        fields,
        cmk,
        receiver_public_key,
        signing_key,
        ephemeral_private_key,
        wrapping_nonce,
    )


def seal_channel_key_with_material(
    fields: KeyGrantFields,
    cmk: ChannelMasterKey,
    receiver_public_key: bytes,
    signing_key: OriginatorSigningKey,
    ephemeral_private_key: bytes,
    wrapping_nonce: bytes,
) -> PortableKeyGrant:
    """Seal deterministic material through the same production primitives."""
    receiver_public_copy = _copy_bytes(receiver_public_key)
    ephemeral_private_copy = _copy_bytes(ephemeral_private_key)
    wrapping_nonce_copy = _copy_bytes(wrapping_nonce)
    _require_length(receiver_public_copy, 32)
    _require_length(ephemeral_private_copy, 32)
    _require_length(wrapping_nonce_copy, 24)
    try:
        ephemeral_public_key = x25519_public_key(ephemeral_private_copy)
        shared_secret = x25519(ephemeral_private_copy, receiver_public_copy)
    except (ArithmeticError, TypeError, ValueError):
        _fail("invalid_key_agreement")
    wrapping_key = _derive_wrapping_key(
        shared_secret, fields.channel_id, fields.key_epoch, fields.receiver_id
    )
    aad = _grant_aad(
        fields.originator_id,
        fields.receiver_id,
        fields.channel_id,
        fields.key_epoch,
        ephemeral_public_key,
    )
    try:
        ciphertext, tag = xchacha20_poly1305_aead_encrypt(
            cmk.bytes, wrapping_key, wrapping_nonce_copy, aad
        )
    except (ArithmeticError, TypeError, ValueError):
        _fail("authentication_failed")
    wrapped_cmk = ciphertext + tag
    signature_input = _grant_signature_input(
        fields.originator_id,
        fields.receiver_id,
        fields.channel_id,
        fields.key_epoch,
        ephemeral_public_key,
        wrapping_nonce_copy,
        wrapped_cmk,
    )
    return PortableKeyGrant(
        originator_id=fields.originator_id,
        receiver_id=fields.receiver_id,
        channel_id=fields.channel_id,
        key_epoch=fields.key_epoch,
        ephemeral_public_key=ephemeral_public_key,
        wrapping_nonce=wrapping_nonce_copy,
        wrapped_cmk=wrapped_cmk,
        originator_signature=signing_key.sign(signature_input),
    )


def open_channel_key_grant(
    grant: PortableKeyGrant,
    expected_originator_id: bytes,
    expected_receiver_id: bytes,
    expected_channel_id: bytes,
    receiver_key_pair: ReceiverKeyPair,
    originator_public_key: bytes,
) -> ChannelMasterKey:
    """Verify expected bindings in normative order, then unwrap one CMK."""
    verify_grant_signature(
        grant,
        expected_originator_id,
        expected_receiver_id,
        expected_channel_id,
        originator_public_key,
    )
    shared_secret = receiver_key_pair.agree(grant.ephemeral_public_key)
    wrapping_key = _derive_wrapping_key(
        shared_secret, grant.channel_id, grant.key_epoch, grant.receiver_id
    )
    aad = _grant_aad(
        grant.originator_id,
        grant.receiver_id,
        grant.channel_id,
        grant.key_epoch,
        grant.ephemeral_public_key,
    )
    try:
        plaintext = xchacha20_poly1305_aead_decrypt(
            grant.wrapped_cmk[:32],
            wrapping_key,
            grant.wrapping_nonce,
            aad,
            grant.wrapped_cmk[32:],
        )
    except (ArithmeticError, TypeError, ValueError):
        _fail("authentication_failed")
    if len(plaintext) != 32:
        _fail("invalid_wrapped_key")
    return ChannelMasterKey.from_bytes(plaintext)


def verify_grant_signature(
    grant: PortableKeyGrant,
    expected_originator_id: bytes,
    expected_receiver_id: bytes,
    expected_channel_id: bytes,
    originator_public_key: bytes,
) -> None:
    """Verify public D18G bindings and signature without a receiver secret."""
    _validate_grant(grant)
    originator_copy = _copy_bytes(expected_originator_id)
    receiver_copy = _copy_bytes(expected_receiver_id)
    channel_copy = _copy_bytes(expected_channel_id)
    public_key_copy = _copy_bytes(originator_public_key)
    _require_length(channel_copy, 16)
    _require_length(public_key_copy, 32)
    if not _equal_bytes(grant.originator_id, originator_copy):
        _fail("unexpected_originator")
    if not _equal_bytes(grant.receiver_id, receiver_copy):
        _fail("unexpected_receiver")
    if not _equal_bytes(grant.channel_id, channel_copy):
        _fail("unexpected_channel")
    signature_input = _grant_signature_input(
        grant.originator_id,
        grant.receiver_id,
        grant.channel_id,
        grant.key_epoch,
        grant.ephemeral_public_key,
        grant.wrapping_nonce,
        grant.wrapped_cmk,
    )
    try:
        signature_valid = verify(
            signature_input, grant.originator_signature, public_key_copy
        )
    except (ArithmeticError, TypeError, ValueError):
        signature_valid = False
    if not signature_valid:
        _fail("invalid_signature")


class ReceiverEpochKeys:
    """Receiver-local monotonic state for one originator/receiver/channel tuple."""

    __slots__ = (
        "_channel_id",
        "_epoch_keys",
        "_latest_grant",
        "_originator_id",
        "_originator_public_key",
        "_receiver_id",
        "_receiver_key_pair",
    )

    def __init__(
        self,
        originator_id: bytes,
        receiver_id: bytes,
        channel_id: bytes,
        receiver_key_pair: ReceiverKeyPair,
        originator_public_key: bytes,
    ) -> None:
        originator_copy = _copy_bytes(originator_id)
        receiver_copy = _copy_bytes(receiver_id)
        channel_copy = _copy_bytes(channel_id)
        public_key_copy = _copy_bytes(originator_public_key)
        _validate_identity(originator_copy)
        _validate_identity(receiver_copy)
        _validate_channel_id(channel_copy)
        _require_length(public_key_copy, 32)
        self._originator_id = originator_copy
        self._receiver_id = receiver_copy
        self._channel_id = channel_copy
        self._receiver_key_pair = receiver_key_pair.clone()
        self._originator_public_key = public_key_copy
        self._epoch_keys: dict[int, ChannelMasterKey] = {}
        self._latest_grant: PortableKeyGrant | None = None

    @property
    def receiver_public_key(self) -> bytes:
        """Return the receiver public key used to prepare grants."""
        return self._receiver_key_pair.public_key

    @property
    def latest_epoch(self) -> int | None:
        """Return the newest installed epoch."""
        return None if self._latest_grant is None else self._latest_grant.key_epoch

    def install_grant(self, grant: PortableKeyGrant) -> GrantInstallOutcome:
        """Install atomically after enforcing retry/conflict/ordering rules."""
        latest = self._latest_grant
        if latest is not None:
            if grant.key_epoch < latest.key_epoch:
                _fail("decreasing_epoch")
            if grant.key_epoch == latest.key_epoch:
                if grant == latest:
                    return "idempotent"
                _fail("conflicting_grant")
        _validate_grant(grant)
        key = open_channel_key_grant(
            grant,
            self._originator_id,
            self._receiver_id,
            self._channel_id,
            self._receiver_key_pair,
            self._originator_public_key,
        )
        self._epoch_keys[grant.key_epoch] = key
        self._latest_grant = grant
        return "installed"

    def key(self, epoch: int) -> ChannelMasterKey:
        """Return an independent managed key for one retained epoch."""
        _require_u64(epoch)
        key = self._epoch_keys.get(epoch)
        if key is None:
            _fail("missing_epoch_key")
        return key.clone()

    def destroy(self) -> None:
        """Destroy every retained local secret."""
        for key in self._epoch_keys.values():
            key.destroy()
        self._epoch_keys.clear()
        self._receiver_key_pair.destroy()
        self._latest_grant = None


class RotationReceiver:
    """One authorized receiver and its one-shot independent seal material."""

    __slots__ = (
        "_destroyed",
        "_ephemeral_private_key",
        "_public_key",
        "_receiver_id",
        "_wrapping_nonce",
    )

    def __init__(
        self,
        receiver_id: bytes,
        public_key: bytes,
        ephemeral_private_key: bytes,
        wrapping_nonce: bytes,
    ) -> None:
        receiver_copy = _copy_bytes(receiver_id)
        public_copy = _copy_bytes(public_key)
        ephemeral_copy = _copy_bytes(ephemeral_private_key)
        nonce_copy = _copy_bytes(wrapping_nonce)
        _validate_identity(receiver_copy)
        _require_length(public_copy, 32)
        _require_length(ephemeral_copy, 32)
        _require_length(nonce_copy, 24)
        self._receiver_id = receiver_copy
        self._public_key = public_copy
        self._ephemeral_private_key = bytearray(ephemeral_copy)
        self._wrapping_nonce = nonce_copy
        self._destroyed = False

    @classmethod
    def with_material(
        cls,
        receiver_id: bytes,
        public_key: bytes,
        ephemeral_private_key: bytes,
        wrapping_nonce: bytes,
    ) -> RotationReceiver:
        """Create a deterministic receiver binding for fixtures/preparation."""
        return cls(receiver_id, public_key, ephemeral_private_key, wrapping_nonce)

    @classmethod
    def generate(
        cls,
        receiver_id: bytes,
        public_key: bytes,
        source: SecureRandomSource = SYSTEM_SECURE_RANDOM_SOURCE,
    ) -> RotationReceiver:
        """Create one production binding with independent CSPRNG material."""
        return cls(
            receiver_id,
            public_key,
            _secure_random_bytes(source, 32),
            _secure_random_bytes(source, 24),
        )

    @property
    def receiver_id(self) -> bytes:
        return bytes(self._receiver_id)

    def seal(
        self,
        fields: KeyGrantFields,
        cmk: ChannelMasterKey,
        signing_key: OriginatorSigningKey,
    ) -> PortableKeyGrant:
        if self._destroyed:
            _fail("invalid_field")
        return seal_channel_key_with_material(
            fields,
            cmk,
            self._public_key,
            signing_key,
            bytes(self._ephemeral_private_key),
            self._wrapping_nonce,
        )

    def destroy(self) -> None:
        _wipe(self._ephemeral_private_key)
        self._destroyed = True


class RotationPlan:
    """Pure receiver-sorted rotation result; durable activation is separate."""

    __slots__ = ("_new_cmk", "grants", "new_epoch")

    def __init__(
        self,
        new_epoch: int,
        new_cmk: ChannelMasterKey,
        grants: tuple[PortableKeyGrant, ...],
        *,
        _token: object,
    ) -> None:
        if _token is not _ROTATION_PLAN_TOKEN:
            _fail("invalid_field")
        self.new_epoch = new_epoch
        self._new_cmk = new_cmk.clone()
        self.grants = tuple(grants)

    @property
    def new_cmk(self) -> ChannelMasterKey:
        return self._new_cmk.clone()

    def destroy(self) -> None:
        self._new_cmk.destroy()


def plan_rotation(
    originator_id: bytes,
    channel_id: bytes,
    current_epoch: int,
    new_cmk: ChannelMasterKey,
    receivers: list[RotationReceiver],
    signing_key: OriginatorSigningKey,
) -> RotationPlan:
    """Return a complete ordered next-epoch plan or no plan at all."""
    originator_copy = _copy_bytes(originator_id)
    channel_copy = _copy_bytes(channel_id)
    _validate_identity(originator_copy)
    _validate_channel_id(channel_copy)
    _require_u64(current_epoch)
    if current_epoch == _MAX_U64:
        _fail("epoch_exhausted")
    if not receivers:
        _fail("invalid_field")
    ordered = sorted(receivers, key=lambda receiver: receiver.receiver_id)
    for previous, current in zip(ordered, ordered[1:], strict=False):
        if previous.receiver_id == current.receiver_id:
            for receiver in ordered:
                receiver.destroy()
            _fail("invalid_field")
    grants: list[PortableKeyGrant] = []
    try:
        for receiver in ordered:
            fields = KeyGrantFields(
                originator_copy,
                receiver.receiver_id,
                channel_copy,
                current_epoch + 1,
            )
            grants.append(receiver.seal(fields, new_cmk, signing_key))
        return RotationPlan(
            current_epoch + 1,
            new_cmk,
            tuple(grants),
            _token=_ROTATION_PLAN_TOKEN,
        )
    finally:
        for receiver in ordered:
            receiver.destroy()


def secret_erasure_capability() -> SecretErasureCapability:
    """Report Python's honest physical-erasure capability."""
    return "not_enforceable"


def key_grant_hkdf_salt(channel_id: bytes, key_epoch: int) -> bytes:
    """Return the canonical D18Q HKDF salt for conformance diagnostics."""
    channel_copy = _copy_bytes(channel_id)
    _require_length(channel_copy, 16)
    _require_u64(key_epoch)
    return _frame((channel_copy, _u64be(key_epoch)))


def key_grant_hkdf_info(receiver_id: bytes) -> bytes:
    """Return the canonical D18Q HKDF info for conformance diagnostics."""
    receiver_copy = _copy_bytes(receiver_id)
    if len(receiver_copy) > _MAX_IDENTITY_BYTES:
        _fail("length_limit_exceeded")
    return _frame((_KEY_WRAP_CONTEXT, receiver_copy))


def key_grant_aad(grant: PortableKeyGrant) -> bytes:
    """Return the canonical grant AAD for conformance diagnostics."""
    return _grant_aad(
        grant.originator_id,
        grant.receiver_id,
        grant.channel_id,
        grant.key_epoch,
        grant.ephemeral_public_key,
    )


def key_grant_signature_input(grant: PortableKeyGrant) -> bytes:
    """Return the canonical signature input for conformance diagnostics."""
    return _grant_signature_input(
        grant.originator_id,
        grant.receiver_id,
        grant.channel_id,
        grant.key_epoch,
        grant.ephemeral_public_key,
        grant.wrapping_nonce,
        grant.wrapped_cmk,
    )


def key_grant_wrapping_key(
    shared_secret: bytes,
    channel_id: bytes,
    key_epoch: int,
    receiver_id: bytes,
) -> bytes:
    """Return the canonical receiver-specific wrapping key for fixtures."""
    return _derive_wrapping_key(
        _copy_bytes(shared_secret),
        _copy_bytes(channel_id),
        key_epoch,
        _copy_bytes(receiver_id),
    )


def _validate_grant(grant: PortableKeyGrant) -> None:
    _validate_identity(grant.originator_id)
    _validate_identity(grant.receiver_id)
    _validate_channel_id(grant.channel_id)
    _require_u64(grant.key_epoch)


def _validate_identity(identity: bytes) -> None:
    if not identity:
        _fail("invalid_field")
    if len(identity) > _MAX_IDENTITY_BYTES:
        _fail("length_limit_exceeded")


def _validate_channel_id(channel_id: bytes) -> None:
    _require_length(channel_id, 16)
    if channel_id[6] >> 4 != 7 or channel_id[8] >> 6 != 2:
        _fail("invalid_field")


def _derive_wrapping_key(
    shared_secret: bytes,
    channel_id: bytes,
    key_epoch: int,
    receiver_id: bytes,
) -> bytes:
    _require_length(shared_secret, 32)
    try:
        key = hkdf(
            key_grant_hkdf_salt(channel_id, key_epoch),
            shared_secret,
            key_grant_hkdf_info(receiver_id),
            32,
            "sha256",
        )
    except KeyGrantProfileError:
        raise
    except (ArithmeticError, TypeError, ValueError):
        _fail("key_derivation_failed")
    if len(key) != 32:
        _fail("key_derivation_failed")
    return key


def _grant_aad(
    originator_id: bytes,
    receiver_id: bytes,
    channel_id: bytes,
    key_epoch: int,
    ephemeral_public_key: bytes,
) -> bytes:
    return _frame(
        (
            _KEY_GRANT_CONTEXT,
            originator_id,
            channel_id,
            _u64be(key_epoch),
            receiver_id,
            ephemeral_public_key,
        )
    )


def _grant_signature_input(
    originator_id: bytes,
    receiver_id: bytes,
    channel_id: bytes,
    key_epoch: int,
    ephemeral_public_key: bytes,
    wrapping_nonce: bytes,
    wrapped_cmk: bytes,
) -> bytes:
    return _frame(
        (
            _KEY_GRANT_CONTEXT,
            originator_id,
            channel_id,
            _u64be(key_epoch),
            receiver_id,
            ephemeral_public_key,
            wrapping_nonce,
            wrapped_cmk,
        )
    )


def _frame(fields: tuple[bytes, ...]) -> bytes:
    return b"".join(len(field).to_bytes(8, "big") + field for field in fields)


def _secure_random_bytes(source: SecureRandomSource, length: int) -> bytes:
    try:
        value = source.random_bytes(length)
        copied = _copy_bytes(value)
    except KeyGrantProfileError:
        raise
    except (ArithmeticError, OSError, TypeError, ValueError):
        _fail("randomness_unavailable")
    if len(copied) != length:
        _fail("randomness_unavailable")
    return copied


def _copy_bytes(value: bytes) -> bytes:
    if not isinstance(value, (bytes, bytearray, memoryview)):
        _fail("invalid_field")
    return bytes(bytearray(value))


def _require_length(value: bytes, length: int) -> None:
    if len(value) != length:
        _fail("invalid_field")


def _require_u64(value: int) -> None:
    if (
        isinstance(value, bool)
        or not isinstance(value, int)
        or not 0 <= value <= _MAX_U64
    ):
        _fail("invalid_field")


def _u64be(value: int) -> bytes:
    _require_u64(value)
    return value.to_bytes(8, "big")


def _equal_bytes(left: bytes, right: bytes) -> bool:
    if len(left) != len(right):
        return False
    difference = 0
    for left_byte, right_byte in zip(left, right, strict=True):
        difference |= left_byte ^ right_byte
    return difference == 0


def _wipe(value: bytearray) -> None:
    value[:] = bytes(len(value))


def _fail(code: KeyGrantErrorCode) -> NoReturn:
    raise KeyGrantProfileError(code)


class _GrantDecoder:
    __slots__ = ("_data", "_offset")

    def __init__(self, data: bytes) -> None:
        self._data = _copy_bytes(data)
        self._offset = 0

    def take(self, length: int) -> bytes:
        end = self._offset + length
        if length < 0 or end > len(self._data):
            _fail("truncated_record")
        value = self._data[self._offset : end]
        self._offset = end
        return value

    def read_identity(self) -> bytes:
        length = int.from_bytes(self.take(4), "big")
        if length > _MAX_IDENTITY_BYTES:
            _fail("length_limit_exceeded")
        return self.take(length)

    def read_u64(self) -> int:
        return int.from_bytes(self.take(8), "big")

    def finish(self) -> None:
        if self._offset != len(self._data):
            _fail("trailing_bytes")


__all__ = [
    "KEY_GRANT_ERROR_CODES",
    "SYSTEM_SECURE_RANDOM_SOURCE",
    "ChannelMasterKey",
    "GrantInstallOutcome",
    "KeyGrantErrorCode",
    "KeyGrantFields",
    "KeyGrantProfileError",
    "OriginatorSigningKey",
    "PortableKeyGrant",
    "ReceiverEpochKeys",
    "ReceiverKeyPair",
    "RotationPlan",
    "RotationReceiver",
    "SecretErasureCapability",
    "SecureRandomSource",
    "grant_deserialize",
    "grant_serialize",
    "key_grant_aad",
    "key_grant_hkdf_info",
    "key_grant_hkdf_salt",
    "key_grant_signature_input",
    "key_grant_wrapping_key",
    "open_channel_key_grant",
    "plan_rotation",
    "seal_channel_key",
    "seal_channel_key_with_material",
    "secret_erasure_capability",
    "verify_grant_signature",
]
