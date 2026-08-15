from __future__ import annotations

import base64
import json
from collections.abc import Callable
from dataclasses import FrozenInstanceError
from functools import partial
from pathlib import Path
from typing import Any

import pytest
from coding_adventures_x25519 import (  # type: ignore[import-untyped]
    generate_keypair as x25519_public_key,
)
from coding_adventures_x25519 import x25519  # type: ignore[import-untyped]

from coding_adventures_chief_of_staff_channel_crypto import (
    KEY_GRANT_ERROR_CODES,
    ChannelMasterKey,
    KeyGrantFields,
    KeyGrantProfileError,
    OriginatorSigningKey,
    PortableKeyGrant,
    ReceiverEpochKeys,
    ReceiverKeyPair,
    RotationReceiver,
    grant_deserialize,
    grant_serialize,
    key_grant_aad,
    key_grant_hkdf_info,
    key_grant_hkdf_salt,
    key_grant_signature_input,
    key_grant_wrapping_key,
    open_channel_key_grant,
    plan_rotation,
    seal_channel_key,
    seal_channel_key_with_material,
    secret_erasure_capability,
)

_FIXTURE_PATH = (
    Path(__file__).resolve().parents[4]
    / "fixtures"
    / "chief-of-staff-channel-key-grant"
    / "v1"
    / "manifest.json"
)
_FIXTURE: dict[str, Any] = json.loads(_FIXTURE_PATH.read_text(encoding="utf-8"))
_SIGNER = OriginatorSigningKey.from_seed(
    bytes.fromhex(_FIXTURE["test_signing_key"]["seed_hex"])
)
_ORIGINATOR_PUBLIC_KEY = bytes.fromhex(_FIXTURE["test_signing_key"]["public_key_hex"])
_CHANNEL_ID = bytes.fromhex(_FIXTURE["positive_cases"][0]["channel_id_hex"])


def _decode(value: str) -> bytes:
    return base64.b64decode(value)


def _expect_error(code: str, operation: Callable[[], object]) -> None:
    with pytest.raises(KeyGrantProfileError) as raised:
        operation()
    assert raised.value.code == code
    assert str(raised.value) == code


def _assert_case_roster(
    cases: list[dict[str, Any]],
    names: list[str],
    fields: list[str],
) -> None:
    assert [case["name"] for case in cases] == names
    for case in cases:
        assert list(case) == fields


def test_manifest_topology_error_vocabulary_and_erasure_are_closed() -> None:
    assert list(_FIXTURE) == [
        "fixture_format",
        "spec",
        "generator_blob_sha1",
        "warning",
        "constants",
        "test_signing_key",
        "positive_cases",
        "structural_negative_cases",
        "truncated_prefix_recipe",
        "oversize_recipes",
        "field_negative_cases",
        "seal_negative_cases",
        "opening_negative_cases",
        "receiver_state_trace",
        "rotation_case",
        "secret_erasure_capabilities",
        "rust_secret_erasure_capability",
        "stable_error_codes",
    ]
    assert _FIXTURE["fixture_format"] == "D18Q-channel-key-grant-fixtures-v1"
    assert _FIXTURE["spec"] == (
        "code/specs/D18Q-chief-of-staff-channel-key-grant-profile.md"
    )
    assert len(_FIXTURE["generator_blob_sha1"]) == 40
    assert "test-only" in _FIXTURE["warning"]
    assert "Never log" in _FIXTURE["warning"]
    assert _FIXTURE["constants"] == {
        "key_grant_context_ascii": "chief-channel-key-grant-v1",
        "key_wrap_context_ascii": "chief-channel-key-wrap-v1",
        "max_identity_bytes": "4096",
        "wire_magic_ascii": "D18G",
        "wire_version": "1",
    }
    assert list(KEY_GRANT_ERROR_CODES) == _FIXTURE["stable_error_codes"]
    assert _FIXTURE["secret_erasure_capabilities"] == [
        "guaranteed",
        "best_effort",
        "not_enforceable",
    ]
    assert secret_erasure_capability() == "not_enforceable"
    assert _FIXTURE["rust_secret_erasure_capability"] == "guaranteed"
    assert _SIGNER.public_key == _ORIGINATOR_PUBLIC_KEY

    positives = _FIXTURE["positive_cases"]
    assert [case["name"] for case in positives] == [
        "epoch-zero-receiver-a",
        "epoch-zero-receiver-b",
        "maximum-epoch-receiver-a",
    ]
    for case in positives:
        assert list(case) == [
            "name",
            "originator_id_b64",
            "receiver_id_b64",
            "channel_id_hex",
            "key_epoch",
            "cmk_hex",
            "receiver_private_key_hex",
            "receiver_public_key_hex",
            "ephemeral_private_key_hex",
            "ephemeral_public_key_hex",
            "shared_secret_hex",
            "hkdf_salt_b64",
            "hkdf_info_b64",
            "wrapping_key_hex",
            "wrapping_nonce_hex",
            "grant_aad_b64",
            "wrapped_cmk_hex",
            "signature_input_b64",
            "signature_hex",
            "d18g_b64",
            "expected_opened_cmk_hex",
        ]
    _assert_case_roster(
        _FIXTURE["structural_negative_cases"],
        ["wrong-magic", "unsupported-version", "trailing-byte"],
        ["name", "d18g_b64", "expected_error"],
    )
    _assert_case_roster(
        _FIXTURE["field_negative_cases"],
        [
            "empty-originator",
            "empty-receiver",
            "invalid-uuid-version",
            "invalid-uuid-variant",
            "oversized-originator",
            "oversized-receiver",
        ],
        ["name", "expected_error"],
    )
    _assert_case_roster(
        _FIXTURE["seal_negative_cases"],
        ["low-order-receiver-public-key"],
        ["name", "expected_error"],
    )
    _assert_case_roster(
        _FIXTURE["opening_negative_cases"],
        [
            "unexpected-originator",
            "unexpected-receiver",
            "unexpected-channel",
            "invalid-signature",
            "invalid-signature-before-key-agreement",
            "low-order-ephemeral-public-key",
            "wrong-receiver-private-key",
            "wrong-wrapping-nonce",
            "mutated-wrapped-cmk",
            "mutated-tag",
            "epoch-derivation-binding",
            "receiver-derivation-binding",
            "channel-aad-binding",
            "originator-aad-binding",
        ],
        [
            "name",
            "d18g_b64",
            "expected_originator_id_b64",
            "expected_receiver_id_b64",
            "expected_channel_id_hex",
            "receiver_private_key_hex",
            "expected_error",
        ],
    )
    assert list(_FIXTURE["truncated_prefix_recipe"]) == [
        "source_case",
        "first_length",
        "last_length_exclusive",
        "expected_error",
    ]
    for recipe in _FIXTURE["oversize_recipes"]:
        assert list(recipe) == [
            "field",
            "length_offset",
            "declared_length",
            "expected_error",
        ]
    trace = _FIXTURE["receiver_state_trace"]
    assert list(trace) == [
        "grants",
        "steps",
        "missing_epoch",
        "missing_epoch_error",
    ]
    assert [step["name"] for step in trace["steps"]] == [
        "install-epoch-zero",
        "retry-epoch-zero",
        "same-epoch-conflict",
        "failed-higher-open",
        "install-skipped-epoch-three",
        "decreasing-epoch",
    ]
    for step in trace["steps"]:
        assert list(step) == [
            "name",
            "grant",
            "expected",
            "latest_epoch",
            "retained_epochs",
        ]
    assert list(_FIXTURE["rotation_case"]) == [
        "name",
        "current_epoch",
        "new_epoch",
        "new_cmk_hex",
        "authorized_receiver_ids_b64",
        "new_grants_b64",
        "receiver_a_retains_epochs",
        "receiver_b_retains_epochs",
        "receiver_a_new_grant",
    ]


@pytest.mark.parametrize(
    "case", _FIXTURE["positive_cases"], ids=lambda case: case["name"]
)
def test_positive_cases_lock_every_intermediate_and_d18g_byte(
    case: dict[str, str],
) -> None:
    originator_id = _decode(case["originator_id_b64"])
    receiver_id = _decode(case["receiver_id_b64"])
    channel_id = bytes.fromhex(case["channel_id_hex"])
    epoch = int(case["key_epoch"])
    receiver = ReceiverKeyPair.from_private_key(
        bytes.fromhex(case["receiver_private_key_hex"])
    )
    assert receiver.public_key == bytes.fromhex(case["receiver_public_key_hex"])
    ephemeral_private = bytes.fromhex(case["ephemeral_private_key_hex"])
    ephemeral_public = x25519_public_key(ephemeral_private)
    assert ephemeral_public == bytes.fromhex(case["ephemeral_public_key_hex"])
    shared_secret = x25519(ephemeral_private, receiver.public_key)
    assert shared_secret == bytes.fromhex(case["shared_secret_hex"])
    assert key_grant_hkdf_salt(channel_id, epoch) == _decode(case["hkdf_salt_b64"])
    assert key_grant_hkdf_info(receiver_id) == _decode(case["hkdf_info_b64"])
    assert key_grant_wrapping_key(
        shared_secret, channel_id, epoch, receiver_id
    ) == bytes.fromhex(case["wrapping_key_hex"])
    fields = KeyGrantFields(originator_id, receiver_id, channel_id, epoch)
    cmk = ChannelMasterKey.from_bytes(bytes.fromhex(case["cmk_hex"]))
    grant = seal_channel_key_with_material(
        fields,
        cmk,
        receiver.public_key,
        _SIGNER,
        ephemeral_private,
        bytes.fromhex(case["wrapping_nonce_hex"]),
    )
    record = _decode(case["d18g_b64"])
    assert grant_serialize(grant) == record
    assert grant.wrapped_cmk == bytes.fromhex(case["wrapped_cmk_hex"])
    assert grant.originator_signature == bytes.fromhex(case["signature_hex"])
    assert key_grant_aad(grant) == _decode(case["grant_aad_b64"])
    assert key_grant_signature_input(grant) == _decode(case["signature_input_b64"])
    decoded = grant_deserialize(record)
    assert grant_serialize(decoded) == record
    opened = open_channel_key_grant(
        decoded,
        originator_id,
        receiver_id,
        channel_id,
        receiver,
        _ORIGINATOR_PUBLIC_KEY,
    )
    assert opened.bytes == bytes.fromhex(case["expected_opened_cmk_hex"])
    opened.destroy()
    cmk.destroy()
    receiver.destroy()


def test_structural_field_and_seal_failures_use_declared_codes() -> None:
    base = _decode(_FIXTURE["positive_cases"][0]["d18g_b64"])
    for case in _FIXTURE["structural_negative_cases"]:
        _expect_error(
            case["expected_error"],
            partial(grant_deserialize, _decode(case["d18g_b64"])),
        )
    recipe = _FIXTURE["truncated_prefix_recipe"]
    last = int(recipe["last_length_exclusive"])
    assert last == len(base)
    for end in range(int(recipe["first_length"]), last):
        _expect_error(
            recipe["expected_error"],
            partial(grant_deserialize, base[:end]),
        )
    for oversize in _FIXTURE["oversize_recipes"]:
        changed = bytearray(base)
        offset = int(oversize["length_offset"])
        changed[offset : offset + 4] = int(oversize["declared_length"]).to_bytes(
            4, "big"
        )
        _expect_error(
            oversize["expected_error"],
            partial(grant_deserialize, bytes(changed)),
        )
    for case in _FIXTURE["field_negative_cases"]:
        originator = b"originator"
        receiver = b"receiver"
        channel = bytearray(_CHANNEL_ID)
        if case["name"] == "empty-originator":
            originator = b""
        elif case["name"] == "empty-receiver":
            receiver = b""
        elif case["name"] == "invalid-uuid-version":
            channel[6] = 0x60
        elif case["name"] == "invalid-uuid-variant":
            channel[8] = 0x10
        elif case["name"] == "oversized-originator":
            originator = bytes(4097)
        elif case["name"] == "oversized-receiver":
            receiver = bytes(4097)
        _expect_error(
            case["expected_error"],
            partial(KeyGrantFields, originator, receiver, bytes(channel), 0),
        )
    fields = KeyGrantFields(b"originator", b"receiver", _CHANNEL_ID, 0)
    _expect_error(
        _FIXTURE["seal_negative_cases"][0]["expected_error"],
        lambda: seal_channel_key_with_material(
            fields,
            ChannelMasterKey.from_bytes(bytes([0x22]) * 32),
            bytes(32),
            _SIGNER,
            bytes([0x51]) * 32,
            bytes([0x61]) * 24,
        ),
    )


@pytest.mark.parametrize(
    "case", _FIXTURE["opening_negative_cases"], ids=lambda case: case["name"]
)
def test_opening_failures_follow_normative_validation_order(
    case: dict[str, str],
) -> None:
    receiver = ReceiverKeyPair.from_private_key(
        bytes.fromhex(case["receiver_private_key_hex"])
    )
    _expect_error(
        case["expected_error"],
        lambda: open_channel_key_grant(
            grant_deserialize(_decode(case["d18g_b64"])),
            _decode(case["expected_originator_id_b64"]),
            _decode(case["expected_receiver_id_b64"]),
            bytes.fromhex(case["expected_channel_id_hex"]),
            receiver,
            _ORIGINATOR_PUBLIC_KEY,
        ),
    )
    receiver.destroy()


def _retained_epochs(state: ReceiverEpochKeys, maximum: int) -> list[str]:
    retained: list[str] = []
    for epoch in range(maximum + 1):
        try:
            key = state.key(epoch)
        except KeyGrantProfileError:
            continue
        key.destroy()
        retained.append(str(epoch))
    return retained


def test_receiver_trace_is_atomic_monotonic_and_allows_skipped_epochs() -> None:
    first = _FIXTURE["positive_cases"][0]
    original_receiver = ReceiverKeyPair.from_private_key(
        bytes.fromhex(first["receiver_private_key_hex"])
    )
    state = ReceiverEpochKeys(
        _decode(first["originator_id_b64"]),
        _decode(first["receiver_id_b64"]),
        _CHANNEL_ID,
        original_receiver,
        _ORIGINATOR_PUBLIC_KEY,
    )
    trace = _FIXTURE["receiver_state_trace"]
    for step in trace["steps"]:
        grant = grant_deserialize(_decode(trace["grants"][step["grant"]]))
        actual: str
        try:
            actual = state.install_grant(grant)
        except KeyGrantProfileError as error:
            actual = error.code
        assert actual == step["expected"], step["name"]
        assert str(state.latest_epoch) == step["latest_epoch"]
        assert _retained_epochs(state, 3) == step["retained_epochs"]
    _expect_error(
        trace["missing_epoch_error"], lambda: state.key(int(trace["missing_epoch"]))
    )
    latest_epoch = state.latest_epoch
    assert latest_epoch is not None
    malformed_same_epoch = PortableKeyGrant(
        originator_id=b"",
        receiver_id=b"",
        channel_id=bytes(16),
        key_epoch=latest_epoch,
        ephemeral_public_key=bytes(32),
        wrapping_nonce=bytes(24),
        wrapped_cmk=bytes(48),
        originator_signature=bytes(64),
    )
    _expect_error(
        "conflicting_grant", lambda: state.install_grant(malformed_same_epoch)
    )
    assert state.receiver_public_key == original_receiver.public_key
    state.destroy()
    original_receiver.destroy()


def test_rotation_reproduces_prospective_revocation_fixture() -> None:
    first, second = _FIXTURE["positive_cases"][:2]
    receiver_a = ReceiverKeyPair.from_private_key(
        bytes.fromhex(first["receiver_private_key_hex"])
    )
    receiver_b = ReceiverKeyPair.from_private_key(
        bytes.fromhex(second["receiver_private_key_hex"])
    )
    state_a = ReceiverEpochKeys(
        _decode(first["originator_id_b64"]),
        _decode(first["receiver_id_b64"]),
        _CHANNEL_ID,
        receiver_a,
        _ORIGINATOR_PUBLIC_KEY,
    )
    state_b = ReceiverEpochKeys(
        _decode(second["originator_id_b64"]),
        _decode(second["receiver_id_b64"]),
        _CHANNEL_ID,
        receiver_b,
        _ORIGINATOR_PUBLIC_KEY,
    )
    state_a.install_grant(grant_deserialize(_decode(first["d18g_b64"])))
    state_b.install_grant(grant_deserialize(_decode(second["d18g_b64"])))
    rotation = _FIXTURE["rotation_case"]
    new_cmk = ChannelMasterKey.from_bytes(bytes.fromhex(rotation["new_cmk_hex"]))
    plan = plan_rotation(
        _decode(first["originator_id_b64"]),
        _CHANNEL_ID,
        int(rotation["current_epoch"]),
        new_cmk,
        [
            RotationReceiver.with_material(
                _decode(second["receiver_id_b64"]),
                receiver_b.public_key,
                bytes([0x71]) * 32,
                bytes([0x81]) * 24,
            )
        ],
        _SIGNER,
    )
    assert plan.new_epoch == int(rotation["new_epoch"])
    actual_grants = [
        base64.b64encode(grant_serialize(grant)).decode() for grant in plan.grants
    ]
    assert actual_grants == rotation["new_grants_b64"]
    assert [base64.b64encode(grant.receiver_id).decode() for grant in plan.grants] == (
        rotation["authorized_receiver_ids_b64"]
    )
    state_b.install_grant(plan.grants[0])
    assert _retained_epochs(state_a, 1) == rotation["receiver_a_retains_epochs"]
    assert _retained_epochs(state_b, 1) == rotation["receiver_b_retains_epochs"]
    assert rotation["receiver_a_new_grant"] is None
    planned_cmk = plan.new_cmk
    installed_cmk = state_b.key(1)
    assert installed_cmk.bytes == planned_cmk.bytes
    installed_cmk.destroy()
    planned_cmk.destroy()
    plan.destroy()
    new_cmk.destroy()
    state_a.destroy()
    state_b.destroy()
    receiver_a.destroy()
    receiver_b.destroy()


class _QueuedRandom:
    def __init__(self, chunks: list[bytes]) -> None:
        self._chunks = iter(chunks)

    def random_bytes(self, length: int) -> bytes:
        value = next(self._chunks)
        assert len(value) == length
        return value


class _ShortRandom:
    def random_bytes(self, length: int) -> bytes:
        return bytes(length - 1)


class _FailingRandom:
    def random_bytes(self, length: int) -> bytes:
        raise OSError(f"secret request length {length}")


def test_entropy_lifecycle_immutability_and_rotation_edges() -> None:
    first = _FIXTURE["positive_cases"][0]
    fields = KeyGrantFields(
        _decode(first["originator_id_b64"]),
        _decode(first["receiver_id_b64"]),
        _CHANNEL_ID,
        int(first["key_epoch"]),
    )
    cmk = ChannelMasterKey.from_bytes(bytes.fromhex(first["cmk_hex"]))
    receiver_public = bytes.fromhex(first["receiver_public_key_hex"])
    grant = seal_channel_key(
        fields,
        cmk,
        receiver_public,
        _SIGNER,
        _QueuedRandom(
            [
                bytes.fromhex(first["ephemeral_private_key_hex"]),
                bytes.fromhex(first["wrapping_nonce_hex"]),
            ]
        ),
    )
    assert grant_serialize(grant) == _decode(first["d18g_b64"])
    with pytest.raises(FrozenInstanceError):
        grant.key_epoch = 9  # type: ignore[misc]
    mutable_originator = bytearray(first["originator_id_b64"].encode())
    copied_fields = KeyGrantFields(mutable_originator, b"receiver", _CHANNEL_ID, 0)  # type: ignore[arg-type]
    mutable_originator[:] = bytes(len(mutable_originator))
    assert copied_fields.originator_id != bytes(mutable_originator)

    generated_cmk = ChannelMasterKey.generate(_QueuedRandom([bytes([9]) * 32]))
    generated_receiver = ReceiverKeyPair.generate(_QueuedRandom([bytes([10]) * 32]))
    generated_signer = OriginatorSigningKey.generate(_QueuedRandom([bytes([11]) * 32]))
    assert generated_cmk.bytes == bytes([9]) * 32
    assert len(generated_receiver.public_key) == 32
    assert len(generated_signer.public_key) == 32
    assert "0909" not in repr(generated_cmk)
    assert "0a0a" not in repr(generated_receiver)
    assert "0b0b" not in repr(generated_signer)
    generated_cmk.destroy()
    generated_receiver.destroy()
    generated_signer.destroy()
    _expect_error("invalid_field", lambda: generated_cmk.bytes)
    _expect_error("invalid_field", lambda: generated_receiver.public_key)
    _expect_error("invalid_field", lambda: generated_signer.public_key)

    _expect_error(
        "randomness_unavailable", lambda: ChannelMasterKey.generate(_ShortRandom())
    )
    _expect_error(
        "randomness_unavailable", lambda: ReceiverKeyPair.generate(_FailingRandom())
    )
    _expect_error(
        "randomness_unavailable",
        lambda: OriginatorSigningKey.generate(_ShortRandom()),
    )
    _expect_error(
        "randomness_unavailable",
        lambda: seal_channel_key(fields, cmk, receiver_public, _SIGNER, _ShortRandom()),
    )
    _expect_error(
        "randomness_unavailable",
        lambda: RotationReceiver.generate(b"receiver", receiver_public, _ShortRandom()),
    )
    _expect_error(
        "epoch_exhausted",
        lambda: plan_rotation(
            b"originator",
            _CHANNEL_ID,
            (1 << 64) - 1,
            cmk,
            [
                RotationReceiver.with_material(
                    b"receiver",
                    receiver_public,
                    bytes([3]) * 32,
                    bytes([4]) * 24,
                )
            ],
            _SIGNER,
        ),
    )
    _expect_error(
        "invalid_field",
        lambda: plan_rotation(b"originator", _CHANNEL_ID, 0, cmk, [], _SIGNER),
    )
    duplicate_a = RotationReceiver.with_material(
        b"duplicate", receiver_public, bytes([5]) * 32, bytes([6]) * 24
    )
    duplicate_b = RotationReceiver.with_material(
        b"duplicate", receiver_public, bytes([7]) * 32, bytes([8]) * 24
    )
    _expect_error(
        "invalid_field",
        lambda: plan_rotation(
            b"originator",
            _CHANNEL_ID,
            0,
            cmk,
            [duplicate_b, duplicate_a],
            _SIGNER,
        ),
    )
    _expect_error(
        "invalid_field",
        lambda: duplicate_a.seal(fields, cmk, _SIGNER),
    )
    sorted_plan = plan_rotation(
        b"originator",
        _CHANNEL_ID,
        0,
        cmk,
        [
            RotationReceiver.with_material(
                b"receiver-b",
                receiver_public,
                bytes([12]) * 32,
                bytes([13]) * 24,
            ),
            RotationReceiver.with_material(
                b"receiver-a",
                receiver_public,
                bytes([14]) * 32,
                bytes([15]) * 24,
            ),
        ],
        _SIGNER,
    )
    assert [grant.receiver_id for grant in sorted_plan.grants] == [
        b"receiver-a",
        b"receiver-b",
    ]
    sorted_plan.destroy()
    cmk.destroy()


def test_public_constructor_shapes_and_high_level_encoder_are_fail_closed() -> None:
    _expect_error("invalid_field", lambda: ChannelMasterKey.from_bytes(bytes(31)))
    _expect_error("invalid_field", lambda: ReceiverKeyPair.from_private_key(bytes(31)))
    _expect_error("invalid_field", lambda: OriginatorSigningKey.from_seed(bytes(31)))
    _expect_error(
        "invalid_field",
        lambda: PortableKeyGrant(
            originator_id=b"originator",
            receiver_id=b"receiver",
            channel_id=bytes(15),
            key_epoch=0,
            ephemeral_public_key=bytes(32),
            wrapping_nonce=bytes(24),
            wrapped_cmk=bytes(48),
            originator_signature=bytes(64),
        ),
    )
    structurally_decodable = PortableKeyGrant(
        originator_id=b"",
        receiver_id=b"",
        channel_id=bytes(16),
        key_epoch=0,
        ephemeral_public_key=bytes(32),
        wrapping_nonce=bytes(24),
        wrapped_cmk=bytes(48),
        originator_signature=bytes(64),
    )
    _expect_error("invalid_field", lambda: grant_serialize(structurally_decodable))
