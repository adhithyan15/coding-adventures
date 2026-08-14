from __future__ import annotations

import base64
import json
from collections.abc import Callable
from dataclasses import replace
from pathlib import Path
from typing import Any

import pytest
from coding_adventures_chief_of_staff_channel_crypto import message_serialize
from coding_adventures_ed25519 import generate_keypair  # type: ignore[import-untyped]

from coding_adventures_chief_of_staff_channel_store import (
    CHANNEL_ACK_CONTENT_TYPE,
    CHANNEL_DEFINITION_CONTENT_TYPE,
    CHANNEL_ERROR_CODES,
    CHANNEL_GRANT_CONTENT_TYPE,
    CHANNEL_MESSAGE_CONTENT_TYPE,
    CHANNEL_STATE_CONTENT_TYPE,
    CHANNEL_STORAGE_NAMESPACE,
    MAX_CHANNEL_CAS_ATTEMPTS,
    MAX_CHANNEL_RECEIVERS,
    MAX_DEFINITION_CAS_ATTEMPTS,
    MAX_PENDING_HEADER_BYTES,
    AppendRequest,
    ChannelDefinition,
    ChannelDefinitionStore,
    ChannelProfileError,
    ChannelStore,
    DurableOriginator,
    DurableReceiver,
    MemoryChannelStorage,
    MessageHeader,
    MessageMetadata,
    OriginatorIdentity,
    ReceiverIdentity,
    channel_definition_deserialize,
    channel_definition_record_key,
    channel_definition_serialize,
    channel_state_deserialize,
    channel_state_serialize,
    key_grant_record_key,
    message_record_key,
    message_record_prefix,
    receiver_ack_record_key,
    receiver_cursor_deserialize,
    receiver_cursor_serialize,
    sequence_state_record_key,
)

_FIXTURE_PATH = (
    Path(__file__).resolve().parents[4]
    / "fixtures"
    / "chief-of-staff-channel"
    / "v1"
    / "manifest.json"
)
_FIXTURE: dict[str, Any] = json.loads(_FIXTURE_PATH.read_text(encoding="utf-8"))


def _decode(value: str) -> bytes:
    return base64.b64decode(value)


_ACTIVE_BYTES = _decode(_FIXTURE["definition_cases"][0]["d18c_b64"])
_ACTIVE_DEFINITION = channel_definition_deserialize(_ACTIVE_BYTES)
_CHANNEL_ID = _ACTIVE_DEFINITION.channel_id
_ORIGINATOR_ID = _ACTIVE_DEFINITION.originator.agent_id
_BINARY_RECEIVER_ID = _decode(
    _FIXTURE["definition_cases"][0]["canonical_receiver_ids_b64"][0]
)
_TEXT_RECEIVER_ID = _decode(
    _FIXTURE["definition_cases"][0]["canonical_receiver_ids_b64"][1]
)
_PUBLIC_KEY, _SIGNING_SECRET_KEY = generate_keypair(
    bytes.fromhex(_FIXTURE["test_keys"]["originator_signing_seed_hex"])
)
_CHANNEL_MASTER_KEY = bytes.fromhex(_FIXTURE["test_keys"]["channel_master_key_hex"])

_EXPECTED_OPERATION_ERRORS = {
    "conflicting-definition": "conflicting_definition",
    "session-delivery-enforcement": "unknown_message_id",
    "unauthorized-originator": "unauthorized_originator",
    "unauthorized-receiver": "unauthorized_receiver",
    "receiver-public-key-mismatch": "public_key_mismatch",
    "channel-destroyed": "channel_destroyed",
    "missing-key-grant": "missing_key_grant",
    "pending-append": "pending_append",
    "acknowledgement-pending": "acknowledgement_pending",
    "pending-header-mismatch": "pending_header_mismatch",
    "no-pending-append": "no_pending_append",
    "invalid-page-size": "invalid_page_size",
    "invalid-receiver-id": "invalid_receiver_id",
    "acknowledgement-ahead": "acknowledgement_ahead",
    "acknowledgement-regression": "acknowledgement_regression",
    "message-key-body-mismatch": "corrupt_record",
    "message-content-type-mismatch": "corrupt_record",
}


def _expect_error(code: str, operation: Callable[[], object]) -> None:
    with pytest.raises(ChannelProfileError) as raised:
        operation()
    assert raised.value.code == code
    assert str(raised.value) == code


def _operation(name: str) -> dict[str, Any]:
    return next(case for case in _FIXTURE["operation_cases"] if case["name"] == name)


def test_fixture_provenance_constants_and_closed_error_roster() -> None:
    assert _FIXTURE["fixture_format"] == "D18P-durable-channel-fixtures-v1"
    assert len(_FIXTURE["generator_blob_sha1"]) == 40
    assert _FIXTURE["constants"] == {
        "storage_namespace": CHANNEL_STORAGE_NAMESPACE,
        "content_types": {
            "definition": CHANNEL_DEFINITION_CONTENT_TYPE,
            "state": CHANNEL_STATE_CONTENT_TYPE,
            "message": CHANNEL_MESSAGE_CONTENT_TYPE,
            "grant": CHANNEL_GRANT_CONTENT_TYPE,
            "ack": CHANNEL_ACK_CONTENT_TYPE,
        },
        "max_receivers": str(MAX_CHANNEL_RECEIVERS),
        "max_pending_header_bytes": str(MAX_PENDING_HEADER_BYTES),
        "max_store_cas_attempts": str(MAX_CHANNEL_CAS_ATTEMPTS),
        "max_definition_cas_attempts": str(MAX_DEFINITION_CAS_ATTEMPTS),
    }
    assert list(CHANNEL_ERROR_CODES) == _FIXTURE["stable_error_codes"]
    assert {
        case["name"]: case["expected_error"]
        for case in _FIXTURE["operation_negative_cases"]
    } == _EXPECTED_OPERATION_ERRORS


def test_all_definition_state_cursor_and_storage_key_cases_are_exact() -> None:
    for case in _FIXTURE["definition_cases"]:
        encoded = _decode(case["d18c_b64"])
        definition = channel_definition_deserialize(encoded)
        assert definition.lifecycle == case["lifecycle"]
        assert channel_definition_serialize(definition) == encoded
    assert [
        base64.b64encode(receiver.agent_id).decode()
        for receiver in _ACTIVE_DEFINITION.receivers
    ] == _FIXTURE["definition_cases"][0]["canonical_receiver_ids_b64"]

    for case in _FIXTURE["state_cases"]:
        encoded = _decode(case["d18s_b64"])
        state = channel_state_deserialize(encoded, _CHANNEL_ID)
        assert state.next_sequence == int(case["next_sequence"])
        assert (state.pending_header is not None) is case["pending"]
        assert channel_state_serialize(state) == encoded
        if "d18h_b64" in case:
            assert channel_state_serialize(state)[18:] == _decode(case["d18h_b64"])

    for case in _FIXTURE["cursor_cases"]:
        encoded = _decode(case["d18a_b64"])
        cursor = receiver_cursor_deserialize(encoded)
        assert cursor == int(case["first_unread_sequence"])
        assert receiver_cursor_serialize(cursor) == encoded

    actual = {
        "definition": channel_definition_record_key(_CHANNEL_ID),
        "state": sequence_state_record_key(_CHANNEL_ID),
        "message-zero": message_record_key(_CHANNEL_ID, 0),
        "message-max": message_record_key(_CHANNEL_ID, (1 << 64) - 1),
        "message-prefix": message_record_prefix(_CHANNEL_ID),
        "grant": key_grant_record_key(_CHANNEL_ID, 7, _BINARY_RECEIVER_ID),
        "ack-binary-receiver": receiver_ack_record_key(
            _CHANNEL_ID, _BINARY_RECEIVER_ID
        ),
    }
    for case in _FIXTURE["storage_key_cases"]:
        assert actual[case["name"]] == case["expected_key"]


@pytest.mark.parametrize(
    "case", _FIXTURE["codec_negative_cases"], ids=lambda case: case["name"]
)
def test_all_malformed_codec_records_map_to_declared_errors(
    case: dict[str, str],
) -> None:
    def operation() -> None:
        encoded = _decode(case["record_b64"])
        if case["kind"] == "definition":
            channel_definition_deserialize(encoded)
        elif case["kind"] == "state":
            channel_state_deserialize(encoded, _CHANNEL_ID)
        else:
            receiver_cursor_deserialize(encoded)

    _expect_error(case["expected_error"], operation)


def test_all_compact_oversize_recipes_are_enforced() -> None:
    recipes = _FIXTURE["oversize_recipes"]
    _expect_error(
        "invalid_definition",
        lambda: ChannelDefinition(
            _CHANNEL_ID,
            OriginatorIdentity(
                bytes(int(recipes[0]["declared_length"])),
                _ACTIVE_DEFINITION.originator.public_key,
            ),
            _ACTIVE_DEFINITION.receivers,
            0,
            0,
        ),
    )
    oversized_receivers = tuple(
        ReceiverIdentity(bytes((index >> 8, index & 0xFF)), bytes(32))
        for index in range(int(recipes[1]["declared_length"]))
    )
    _expect_error(
        "invalid_definition",
        lambda: ChannelDefinition(
            _CHANNEL_ID,
            _ACTIVE_DEFINITION.originator,
            oversized_receivers,
            0,
            0,
        ),
    )
    oversized_state = bytes((68, 49, 56, 83, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 64, 1))
    _expect_error(
        "corrupt_record",
        lambda: channel_state_deserialize(oversized_state, _CHANNEL_ID),
    )


def test_definition_create_is_idempotent_and_conflicts_are_closed() -> None:
    expected = _operation("definition-create-idempotent")
    backend = MemoryChannelStorage()
    definitions = ChannelDefinitionStore(backend)
    first = definitions.create(_ACTIVE_DEFINITION)
    second = definitions.create(_ACTIVE_DEFINITION)
    assert (first == second) is expected["definitions_equal"]
    assert (
        str(ChannelStore(backend, _CHANNEL_ID).state().next_sequence)
        == expected["initial_next_sequence"]
    )
    conflict = ChannelDefinition(
        _CHANNEL_ID,
        _ACTIVE_DEFINITION.originator,
        _ACTIVE_DEFINITION.receivers,
        _ACTIVE_DEFINITION.created_at_ns + 1,
        _ACTIVE_DEFINITION.key_epoch,
    )
    _expect_error("conflicting_definition", lambda: definitions.create(conflict))


def test_recovery_retry_abandon_gap_paging_and_acknowledgement_trace() -> None:
    expected = _operation("reserve-recover-complete-retry-abandon-gap")
    backend = MemoryChannelStorage()
    store = ChannelStore(backend, _CHANNEL_ID)
    store.initialize()
    header = store.reserve_append(_request(20, 20_000_000_020), b"recoverable")
    recovered = ChannelStore(backend, _CHANNEL_ID)
    assert (recovered.initialize().pending_header == header) is expected[
        "recovered_pending_equal"
    ]
    _expect_error(
        "pending_append",
        lambda: store.reserve_append(_request(21, 20_000_000_021), b"pending"),
    )
    _expect_error(
        "acknowledgement_pending",
        lambda: store.acknowledge(_BINARY_RECEIVER_ID, 0),
    )
    mismatch = MessageHeader(
        _uuid_v7(22),
        20_000_000_022,
        _ORIGINATOR_ID,
        _CHANNEL_ID,
        0,
        0,
        "text/plain",
        header.plaintext_hash,
    )
    _expect_error(
        "pending_header_mismatch",
        lambda: recovered.commit_reserved(
            mismatch, b"recoverable", _CHANNEL_MASTER_KEY, _SIGNING_SECRET_KEY
        ),
    )
    first = recovered.commit_reserved(
        header, b"recoverable", _CHANNEL_MASTER_KEY, _SIGNING_SECRET_KEY
    )
    retry = recovered.commit_reserved(
        header, b"recoverable", _CHANNEL_MASTER_KEY, _SIGNING_SECRET_KEY
    )
    assert message_serialize(first) == _decode(expected["first_d18m_b64"])
    assert (message_serialize(first) == message_serialize(retry)) is expected[
        "commit_retry_equal"
    ]
    abandoned = recovered.reserve_append(_request(23, 20_000_000_023), b"abandoned")
    result = recovered.abandon_pending()
    assert result is not None
    assert str(result.sequence) == expected["abandoned_sequence"]
    _expect_error(
        "no_pending_append",
        lambda: recovered.commit_reserved(
            abandoned, b"abandoned", _CHANNEL_MASTER_KEY, _SIGNING_SECRET_KEY
        ),
    )
    after_gap = recovered.append(
        _request(24, 20_000_000_024),
        b"after gap",
        _CHANNEL_MASTER_KEY,
        _SIGNING_SECRET_KEY,
    )
    assert str(after_gap.sequence) == expected["after_gap_sequence"]
    assert [
        str(message.sequence) for message in recovered.read_messages(0, 10).messages
    ] == expected["read_sequences"]
    first_page = recovered.read_messages(0, 1)
    assert [str(message.sequence) for message in first_page.messages] == expected[
        "first_page_sequences"
    ]
    assert str(first_page.next_start) == expected["first_page_next_start"]
    assert [
        str(message.sequence)
        for message in recovered.read_messages(first_page.next_start or 0, 1).messages
    ] == expected["second_page_sequences"]
    assert [
        str(message.sequence) for message in recovered.read_messages(2, 10).messages
    ] == expected["random_access_sequences"]
    assert (not recovered.read_messages(3, 10).messages) is expected[
        "empty_continuation"
    ]
    _expect_error("invalid_page_size", lambda: recovered.read_messages(0, 0))
    _expect_error(
        "acknowledgement_ahead",
        lambda: recovered.acknowledge(_BINARY_RECEIVER_ID, 3),
    )
    assert recovered.acknowledge(_BINARY_RECEIVER_ID, 0) == 1
    assert recovered.acknowledge(_BINARY_RECEIVER_ID, 2) == 3
    _expect_error(
        "acknowledgement_regression",
        lambda: recovered.acknowledge(_BINARY_RECEIVER_ID, 0),
    )
    _expect_error("invalid_receiver_id", lambda: recovered.receiver_cursor(b""))


def test_encrypted_endpoints_independent_cursors_sessions_and_destruction() -> None:
    expected = _operation("encrypted-endpoint-round-trip-independent-cursors")
    backend = MemoryChannelStorage()
    definitions = ChannelDefinitionStore(backend)
    definitions.create(_ACTIVE_DEFINITION)
    metadata = _MetadataSource(
        [
            MessageMetadata(_uuid_v7(1), 10_000_000_001),
            MessageMetadata(_uuid_v7(2), 10_000_000_002),
            MessageMetadata(_uuid_v7(3), 10_000_000_003),
        ]
    )
    originator = DurableOriginator.open(
        backend,
        _CHANNEL_ID,
        _ORIGINATOR_ID,
        _SIGNING_SECRET_KEY,
        _CHANNEL_MASTER_KEY,
        metadata,
    )
    originator.save_receiver_grant(_BINARY_RECEIVER_ID, b"\x01")
    originator.save_receiver_grant(_TEXT_RECEIVER_ID, b"\x02")
    first = originator.publish(b"message zero", "text/plain")
    second = originator.publish(b"message one", "application/octet-stream")
    assert [str(first.sequence), str(second.sequence)] == expected[
        "published_sequences"
    ]

    binary = DurableReceiver.open(
        backend, _CHANNEL_ID, _BINARY_RECEIVER_ID, _provider(_BINARY_RECEIVER_ID)
    )
    binary_zero = binary.receive(1)
    assert [str(message.sequence) for message in binary_zero] == expected[
        "binary_receiver_delivered_sequences"
    ][:1]
    assert (
        str(binary.acknowledge(binary_zero[0].message_id))
        == expected["binary_first_unread_after_zero"]
    )
    binary_one = binary.receive(10)
    assert [
        str(message.sequence) for message in (*binary_zero, *binary_one)
    ] == expected["binary_receiver_delivered_sequences"]
    assert (
        str(binary.acknowledge(binary_one[0].message_id))
        == expected["binary_first_unread_after_one"]
    )
    assert (
        str(binary.acknowledge(binary_one[0].message_id))
        == expected["binary_first_unread_after_retry"]
    )
    assert (not binary.receive(10)) is expected["binary_empty_continuation"]

    text = DurableReceiver.open(
        backend, _CHANNEL_ID, _TEXT_RECEIVER_ID, _provider(_TEXT_RECEIVER_ID)
    )
    text_messages = text.receive(10)
    assert [str(message.sequence) for message in text_messages] == expected[
        "text_receiver_delivered_sequences"
    ]
    assert (
        str(text.acknowledge(text_messages[0].message_id))
        == expected["text_first_unread_after_zero"]
    )
    store = ChannelStore(backend, _CHANNEL_ID)
    assert (
        str(store.receiver_cursor(_BINARY_RECEIVER_ID))
        == expected["binary_first_unread_after_retry"]
    )
    assert (
        str(store.receiver_cursor(_TEXT_RECEIVER_ID))
        == expected["text_first_unread_after_zero"]
    )

    failing_provider = _provider(_TEXT_RECEIVER_ID, fail=True)
    failing = DurableReceiver.open(
        backend, _CHANNEL_ID, _TEXT_RECEIVER_ID, failing_provider
    )
    _expect_error("crypto_error", lambda: failing.receive(1))
    fresh = DurableReceiver.open(
        backend, _CHANNEL_ID, _BINARY_RECEIVER_ID, _provider(_BINARY_RECEIVER_ID)
    )
    _expect_error("unknown_message_id", lambda: fresh.acknowledge(first.message_id))
    _expect_error(
        "unauthorized_originator",
        lambda: DurableOriginator.open(
            backend,
            _CHANNEL_ID,
            b"intruder",
            _SIGNING_SECRET_KEY,
            _CHANNEL_MASTER_KEY,
            metadata,
        ),
    )
    _expect_error(
        "unauthorized_receiver",
        lambda: DurableReceiver.open(
            backend, _CHANNEL_ID, b"intruder", _provider(_BINARY_RECEIVER_ID)
        ),
    )
    _expect_error(
        "public_key_mismatch",
        lambda: DurableReceiver.open(
            backend,
            _CHANNEL_ID,
            _BINARY_RECEIVER_ID,
            _Provider(bytes(32)),
        ),
    )

    first_destroyed = definitions.destroy(_CHANNEL_ID)
    retry_destroyed = definitions.destroy(_CHANNEL_ID)
    destroyed = _operation("destroy-idempotent-history-preserved")
    assert (first_destroyed == retry_destroyed) is destroyed["definitions_equal"]
    assert str(len(store.read_messages(0, 10).messages)) == destroyed["history_count"]
    _expect_error(
        "channel_destroyed",
        lambda: originator.publish(b"denied", "text/plain"),
    )


def test_missing_grants_and_corrupt_message_envelopes_fail_closed() -> None:
    missing_backend = MemoryChannelStorage()
    ChannelDefinitionStore(missing_backend).create(_ACTIVE_DEFINITION)
    originator = DurableOriginator.open(
        missing_backend,
        _CHANNEL_ID,
        _ORIGINATOR_ID,
        _SIGNING_SECRET_KEY,
        _CHANNEL_MASTER_KEY,
        _MetadataSource([MessageMetadata(_uuid_v7(9), 10_000_000_009)]),
    )
    originator.publish(b"no grant", "text/plain")
    receiver = DurableReceiver.open(
        missing_backend,
        _CHANNEL_ID,
        _BINARY_RECEIVER_ID,
        _provider(_BINARY_RECEIVER_ID),
    )
    _expect_error("missing_key_grant", lambda: receiver.receive(1))

    key_mismatch = _backend_with_one_message()
    original = key_mismatch.get(
        CHANNEL_STORAGE_NAMESPACE, message_record_key(_CHANNEL_ID, 0)
    )
    assert original is not None
    key_mismatch.corrupt(replace(original, key=message_record_key(_CHANNEL_ID, 1)))
    _expect_error(
        "corrupt_record",
        lambda: ChannelStore(key_mismatch, _CHANNEL_ID).read_messages(0, 10),
    )

    type_backend = _backend_with_one_message()
    typed = type_backend.get(
        CHANNEL_STORAGE_NAMESPACE, message_record_key(_CHANNEL_ID, 0)
    )
    assert typed is not None
    type_backend.corrupt(replace(typed, content_type="application/octet-stream"))
    _expect_error(
        "corrupt_record",
        lambda: ChannelStore(type_backend, _CHANNEL_ID).read_messages(0, 10),
    )


def _backend_with_one_message() -> MemoryChannelStorage:
    backend = MemoryChannelStorage()
    store = ChannelStore(backend, _CHANNEL_ID)
    store.initialize()
    store.append(_request(30, 30), b"record", _CHANNEL_MASTER_KEY, _SIGNING_SECRET_KEY)
    return backend


def _request(byte: int, timestamp_ns: int) -> AppendRequest:
    return AppendRequest(_uuid_v7(byte), timestamp_ns, _ORIGINATOR_ID, 0, "text/plain")


def _uuid_v7(byte: int) -> bytes:
    value = bytearray([byte] * 16)
    value[6] = 0x70 | (byte & 0x0F)
    value[8] = 0x80 | (byte & 0x3F)
    return bytes(value)


class _MetadataSource:
    def __init__(self, values: list[MessageMetadata]) -> None:
        self._values = list(values)

    def next(self) -> MessageMetadata:
        if not self._values:
            raise RuntimeError("metadata exhausted")
        return self._values.pop(0)


class _Provider:
    def __init__(self, public_key: bytes, *, fail: bool = False) -> None:
        self.public_key = public_key
        self._fail = fail

    def open_grant(self, key_epoch: int, grant_body: bytes) -> bytes | None:
        if self._fail:
            raise RuntimeError("provider details must not escape")
        return _CHANNEL_MASTER_KEY


def _provider(receiver_id: bytes, *, fail: bool = False) -> _Provider:
    receiver = _ACTIVE_DEFINITION.receiver(receiver_id)
    assert receiver is not None
    return _Provider(receiver.public_key, fail=fail)


def test_fixture_originator_public_key_matches_python_crypto() -> None:
    assert (
        bytes.fromhex(_FIXTURE["test_keys"]["originator_public_key_hex"]) == _PUBLIC_KEY
    )
