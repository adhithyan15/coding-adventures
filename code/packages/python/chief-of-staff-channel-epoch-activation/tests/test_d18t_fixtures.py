"""Direct consumers of the canonical Rust-authored D18T manifest."""

import base64
import json
from pathlib import Path

import pytest
from coding_adventures_chief_of_staff_channel_crypto import (
    ChannelMasterKey,
    OriginatorSigningKey,
    ReceiverKeyPair,
    RotationReceiver,
    plan_rotation,
)
from coding_adventures_chief_of_staff_channel_store import (
    ChannelDefinition,
    OriginatorIdentity,
    ReceiverIdentity,
    channel_state_deserialize,
)

from coding_adventures_chief_of_staff_channel_epoch_activation import (
    ACTIVATION_PLAN_CONTENT_TYPE,
    EPOCH_ACTIVATION_ERROR_CODES,
    EPOCH_STATE_CONTENT_TYPE,
    ActivationPlan,
    EpochWireError,
    activation_plan_deserialize,
    activation_plan_record_key,
    activation_plan_serialize,
    epoch_activation_secret_erasure_capability,
    epoch_state_deserialize,
    epoch_state_serialize,
    prepare_rotation_candidate,
)

MANIFEST_PATH = (
    Path(__file__).parents[4]
    / "fixtures/chief-of-staff-channel-epoch-activation/v1/manifest.json"
)
MANIFEST = json.loads(MANIFEST_PATH.read_text())
CHANNEL_ID = bytes.fromhex("018f47a09b6c7def923456789abcdef0")


def test_manifest_contract_and_secret_boundary() -> None:
    assert MANIFEST["fixture_format"] == "D18T-durable-epoch-activation-fixtures-v1"
    assert (
        MANIFEST["spec"]
        == "code/specs/D18T-chief-of-staff-durable-epoch-activation-profile.md"
    )
    assert "Never log" in MANIFEST["warning"]
    assert MANIFEST["constants"] == {
        "state_magic_ascii": "D18S",
        "state_version": "2",
        "plan_magic_ascii": "D18T",
        "plan_version": "1",
        "state_content_type": EPOCH_STATE_CONTENT_TYPE,
        "plan_content_type": ACTIVATION_PLAN_CONTENT_TYPE,
        "max_cas_attempts": "16",
    }
    assert list(EPOCH_ACTIVATION_ERROR_CODES) == MANIFEST["stable_error_codes"]
    assert [item["name"] for item in MANIFEST["crash_replay_traces"]] == [
        "after-custody-selection",
        "after-plan-write",
        "after-first-grant",
        "after-all-grants",
        "after-activation-cas",
    ]
    assert len(MANIFEST["race_traces"]) == 4
    assert len(MANIFEST["negative_scenarios"]) == 6
    assert MANIFEST["secret_erasure_capability"] == "guaranteed"
    assert epoch_activation_secret_erasure_capability() == "not_enforceable"
    text = json.dumps(MANIFEST)
    for secret in MANIFEST["test_only_secrets"].values():
        assert text.count(secret) == 1


def test_exact_v1_to_v2_state_migrations() -> None:
    assert [item["name"] for item in MANIFEST["state_migrations"]] == [
        "no-pending",
        "pending-d18h",
    ]
    for vector in MANIFEST["state_migrations"]:
        v1 = channel_state_deserialize(_b64(vector["d18s_v1_b64"]), CHANNEL_ID)
        expected = _b64(vector["d18s_v2_b64"])
        v2 = epoch_state_deserialize(expected, CHANNEL_ID)
        assert v2.active_epoch == int(vector["active_epoch"])
        assert v2.next_sequence == v1.next_sequence == int(vector["next_sequence"])
        assert v2.pending_header == v1.pending_header
        assert epoch_state_serialize(v2) == expected


def test_consumes_and_reencodes_canonical_activation_plan() -> None:
    activation = MANIFEST["activation_case"]
    expected = _b64(activation["plan_b64"])
    plan = activation_plan_deserialize(expected)
    assert plan.channel_id == CHANNEL_ID
    assert (plan.base_epoch, plan.new_epoch, len(plan.receivers)) == (0, 1, 1)
    assert activation_plan_serialize(plan) == expected
    assert activation_plan_record_key(CHANNEL_ID, 1) == activation["plan_record_key"]
    assert activation["plan_content_type"] == ACTIVATION_PLAN_CONTENT_TYPE
    assert len(activation["grant_b64"]) == 1
    assert activation["receiver_a_new_grant"] is None
    assert activation["receiver_a_retains_epochs"] == ["0"]
    assert activation["receiver_b_retains_epochs"] == ["0", "1"]


def test_reproduces_rust_authored_plan_and_grant_bytes() -> None:
    secrets = MANIFEST["test_only_secrets"]
    signer = OriginatorSigningKey.from_seed(
        bytes.fromhex(secrets["originator_signing_seed_hex"])
    )
    receiver_a_key = ReceiverKeyPair.from_private_key(
        bytes.fromhex(secrets["receiver_a_private_key_hex"])
    )
    receiver_b_key = ReceiverKeyPair.from_private_key(
        bytes.fromhex(secrets["receiver_b_private_key_hex"])
    )
    receiver_a = ReceiverIdentity(b"receiver-a", receiver_a_key.public_key)
    receiver_b = ReceiverIdentity(b"receiver-b", receiver_b_key.public_key)
    definition = ChannelDefinition(
        CHANNEL_ID,
        OriginatorIdentity(b"originator", signer.public_key),
        (receiver_a, receiver_b),
        1_725_000_000_000_000_000,
        0,
    )
    rotation = plan_rotation(
        b"originator",
        CHANNEL_ID,
        0,
        ChannelMasterKey.from_bytes(bytes.fromhex(secrets["next_cmk_hex"])),
        [
            RotationReceiver.with_material(
                receiver_b.agent_id,
                receiver_b.public_key,
                bytes.fromhex(secrets["ephemeral_private_key_hex"]),
                bytes.fromhex(secrets["wrapping_nonce_hex"]),
            )
        ],
        signer,
    )
    prepared = prepare_rotation_candidate(definition, 0, (receiver_b,), rotation)
    assert prepared.public_preparation.plan_bytes == _b64(
        MANIFEST["activation_case"]["plan_b64"]
    )
    assert prepared.public_preparation.grants == tuple(
        _b64(value) for value in MANIFEST["activation_case"]["grant_b64"]
    )
    prepared.destroy()
    signer.destroy()
    receiver_a_key.destroy()
    receiver_b_key.destroy()


@pytest.mark.parametrize("mutation", ["truncated", "version", "flag"])
def test_rejects_malformed_state(mutation: str) -> None:
    data = bytearray(_b64(MANIFEST["state_migrations"][0]["d18s_v2_b64"]))
    if mutation == "truncated":
        data.pop()
    elif mutation == "version":
        data[4] = 3
    else:
        data[-1] = 2
    with pytest.raises(EpochWireError, match="corrupt_record"):
        epoch_state_deserialize(bytes(data), CHANNEL_ID)


def test_rejects_noncanonical_plans_and_untrusted_constructor() -> None:
    canonical = _b64(MANIFEST["activation_case"]["plan_b64"])
    with pytest.raises(EpochWireError):
        activation_plan_deserialize(canonical + b"\x00")
    wrong_order = bytearray(canonical[:41] + bytes(128))
    wrong_order[37:41] = (2).to_bytes(4, "big")
    wrong_order[41:73] = bytes([2]) * 32
    wrong_order[73:105] = bytes([4]) * 32
    wrong_order[105:137] = bytes([1]) * 32
    wrong_order[137:169] = bytes([3]) * 32
    with pytest.raises(EpochWireError):
        activation_plan_deserialize(bytes(wrong_order))
    with pytest.raises(EpochWireError):
        ActivationPlan(CHANNEL_ID, 0, 2, ())


def _b64(value: str) -> bytes:
    return base64.b64decode(value, validate=True)
