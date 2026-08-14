from __future__ import annotations

import base64
import json
from collections.abc import Callable
from dataclasses import FrozenInstanceError
from functools import partial
from pathlib import Path
from typing import Any

import pytest
from coding_adventures_ed25519 import generate_keypair  # type: ignore[import-untyped]

from coding_adventures_chief_of_staff_channel_crypto import (
    MAX_MESSAGE_JSON_BYTES,
    D18Message,
    MessageFields,
    MessageProfileError,
    MonotonicUuidV7Generator,
    SourcedMessageFields,
    message_authenticated_header,
    message_create,
    message_create_with_sources,
    message_deserialize,
    message_from_json,
    message_serialize,
    message_to_json,
    message_verify,
    message_verify_with_key_resolver,
)

_FIXTURE_PATH = (
    Path(__file__).resolve().parents[4]
    / "fixtures"
    / "chief-of-staff-message"
    / "v1"
    / "manifest.json"
)
_FIXTURE: dict[str, Any] = json.loads(_FIXTURE_PATH.read_text(encoding="utf-8"))
_SIGNING_SEED = bytes.fromhex(_FIXTURE["keys"]["originator_signing_seed_hex"])
_PUBLIC_KEY, _SIGNING_SECRET_KEY = generate_keypair(_SIGNING_SEED)
_EXPECTED_PUBLIC_KEY = bytes.fromhex(_FIXTURE["keys"]["originator_public_key_hex"])
_EPOCH_KEYS = {
    int(item["key_epoch"]): bytes.fromhex(item["key_hex"])
    for item in _FIXTURE["keys"]["channel_master_keys"]
}


def _decode(value: str) -> bytes:
    return base64.b64decode(value)


def _fields_of(message: D18Message) -> MessageFields:
    return MessageFields(
        message.message_id,
        message.timestamp_ns,
        message.originator_id,
        message.channel_id,
        message.sequence,
        message.key_epoch,
        message.content_type,
    )


def _expect_profile_error(code: str, operation: Callable[[], object]) -> None:
    with pytest.raises(MessageProfileError) as raised:
        operation()
    assert raised.value.code == code


def test_fixture_provenance_and_public_material_are_locked() -> None:
    assert _FIXTURE["fixture_format"] == "D18F-message-fixtures-v1"
    assert len(_FIXTURE["generator_blob_sha1"]) == 40
    assert "test-only" in _FIXTURE["warning"]
    assert len(_FIXTURE["positive_cases"]) == 8
    assert len(_FIXTURE["binary_negative_cases"]) == 20
    assert len(_FIXTURE["json_negative_cases"]) == 11
    assert _PUBLIC_KEY == _EXPECTED_PUBLIC_KEY


@pytest.mark.parametrize(
    "case", _FIXTURE["positive_cases"], ids=lambda case: case["name"]
)
def test_positive_fixtures_are_reproduced_byte_identically(
    case: dict[str, str],
) -> None:
    binary = _decode(case["d18m_b64"])
    plaintext = _decode(case["plaintext_b64"])
    message = message_deserialize(binary)
    key = _EPOCH_KEYS[message.key_epoch]

    assert message_serialize(message) == binary
    assert message_authenticated_header(message) == _decode(
        case["authenticated_header_b64"]
    )
    assert (
        message_verify_with_key_resolver(
            message, _PUBLIC_KEY, lambda epoch: _EPOCH_KEYS.get(epoch)
        )
        == plaintext
    )
    assert message_verify(message, _PUBLIC_KEY, key) == plaintext

    canonical_json = _decode(case["canonical_json_b64"])
    assert message_to_json(message) == canonical_json
    assert message_serialize(message_from_json(canonical_json)) == binary
    recreated = message_create(
        _fields_of(message), plaintext, _SIGNING_SECRET_KEY, key
    )
    assert message_serialize(recreated) == binary


@pytest.mark.parametrize(
    "case", _FIXTURE["binary_negative_cases"], ids=lambda case: case["name"]
)
def test_binary_mutations_map_to_stable_errors(case: dict[str, str]) -> None:
    def operation() -> None:
        message = message_deserialize(_decode(case["d18m_b64"]))
        if case["phase"] == "verify":
            message_verify_with_key_resolver(
                message, _PUBLIC_KEY, lambda epoch: _EPOCH_KEYS.get(epoch)
            )

    _expect_profile_error(case["expected_error"], operation)


@pytest.mark.parametrize(
    "case", _FIXTURE["json_negative_cases"], ids=lambda case: case["name"]
)
def test_json_mutations_map_to_stable_errors(case: dict[str, str]) -> None:
    _expect_profile_error(
        case["expected_error"], lambda: message_from_json(_decode(case["json_b64"]))
    )


def test_json_field_order_is_irrelevant_and_output_is_canonical() -> None:
    canonical = _decode(_FIXTURE["positive_cases"][2]["canonical_json_b64"])
    reversed_value = dict(reversed(tuple(json.loads(canonical).items())))
    reordered = json.dumps(reversed_value, separators=(",", ":")).encode()
    assert message_to_json(message_from_json(reordered)) == canonical


def test_json_rejects_unpaired_surrogates() -> None:
    canonical = _decode(_FIXTURE["positive_cases"][0]["canonical_json_b64"])
    malformed = canonical.replace(
        b'"content_type":"application/octet-stream"',
        b'"content_type":"\\ud800"',
    )
    _expect_profile_error("invalid_field", lambda: message_from_json(malformed))


class _LogicalOversizeJson:
    def __len__(self) -> int:
        return MAX_MESSAGE_JSON_BYTES + 1


def test_compact_oversize_recipes_are_enforced() -> None:
    baseline = _decode(_FIXTURE["positive_cases"][0]["d18m_b64"])
    for recipe in _FIXTURE["oversize_recipes"]:
        field = recipe["field"]
        if field == "json-input":
            _expect_profile_error(
                recipe["expected_error"],
                lambda: message_from_json(_LogicalOversizeJson()),  # type: ignore[arg-type]
            )
            continue
        changed = bytearray(baseline)
        length = int(recipe["declared_length"])
        if field == "originator-id":
            changed[29:33] = length.to_bytes(4, "big")
        elif field == "content-type":
            changed[83:87] = length.to_bytes(4, "big")
        else:
            changed[143:151] = length.to_bytes(8, "big")
        _expect_profile_error(
            recipe["expected_error"],
            partial(message_deserialize, bytes(changed)),
        )


def test_messages_copy_mutable_inputs_and_are_frozen() -> None:
    source = message_deserialize(_decode(_FIXTURE["positive_cases"][1]["d18m_b64"]))
    buffers = {
        "message_id": bytearray(source.message_id),
        "originator_id": bytearray(source.originator_id),
        "channel_id": bytearray(source.channel_id),
        "plaintext_hash": bytearray(source.plaintext_hash),
        "ciphertext": bytearray(source.ciphertext),
        "authentication_tag": bytearray(source.authentication_tag),
        "originator_signature": bytearray(source.originator_signature),
    }
    message = D18Message(
        message_id=buffers["message_id"],  # type: ignore[arg-type]
        timestamp_ns=source.timestamp_ns,
        originator_id=buffers["originator_id"],  # type: ignore[arg-type]
        channel_id=buffers["channel_id"],  # type: ignore[arg-type]
        sequence=source.sequence,
        key_epoch=source.key_epoch,
        content_type=source.content_type,
        plaintext_hash=buffers["plaintext_hash"],  # type: ignore[arg-type]
        ciphertext=buffers["ciphertext"],  # type: ignore[arg-type]
        authentication_tag=buffers["authentication_tag"],  # type: ignore[arg-type]
        originator_signature=buffers["originator_signature"],  # type: ignore[arg-type]
    )
    original = message_serialize(message)
    for value in buffers.values():
        value[:] = bytes(len(value))
    with pytest.raises(FrozenInstanceError):
        message.sequence = 999  # type: ignore[misc]
    with pytest.raises(TypeError):
        message.ciphertext[0] = 0  # type: ignore[index]
    assert message_serialize(message) == original


class _UuidSource:
    def __init__(self, value: bytes) -> None:
        self.value = value

    def next(self) -> bytes:
        return self.value


class _Clock:
    def now(self) -> int:
        return 456


def test_creation_uses_injected_uuid_and_monotonic_clock_sources() -> None:
    source = message_deserialize(_decode(_FIXTURE["positive_cases"][0]["d18m_b64"]))
    key = _EPOCH_KEYS[source.key_epoch]
    message = message_create_with_sources(
        SourcedMessageFields(
            source.originator_id,
            source.channel_id,
            123,
            source.key_epoch,
            source.content_type,
        ),
        b"\x01\x02\x03",
        _SIGNING_SECRET_KEY,
        key,
        _UuidSource(source.message_id),
        _Clock(),
    )
    assert message.message_id == source.message_id
    assert message.timestamp_ns == 456
    assert message_verify(message, _PUBLIC_KEY, key) == b"\x01\x02\x03"


def test_uuid_v7_generator_orders_1000_values_in_one_millisecond() -> None:
    generator = MonotonicUuidV7Generator()
    previous: bytes | None = None
    for _ in range(1000):
        current = generator.next(1_725_000_000_000, bytes([0x55]) * 10)
        assert current[6] >> 4 == 7
        assert current[8] & 0xC0 == 0x80
        if previous is not None:
            assert previous < current
        previous = current
