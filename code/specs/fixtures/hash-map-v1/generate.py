"""Generate DT18 hash-map fixture values from the independent Python models."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from hash_map_oracle import (
    HISTORY_ALPHABET,
    HISTORY_COUNT,
    HISTORY_KEYS,
    HISTORY_MAX_LENGTH,
    HISTORY_VALUES,
    INITIAL_CAPACITY,
    MAXIMUM_CAPACITY,
    MAXIMUM_KEY_BYTES,
    PHYSICAL_ALPHABET,
    AssociationListModel,
    Entry,
    Key,
    PhysicalLayoutModel,
    assert_logical_matches_physical,
    expected_history_streams,
    history_operations,
    observe_state,
    parse_key,
    physical_order_table,
    hash_bytes,
)


HERE = Path(__file__).resolve().parent
OUTPUT = HERE / "cases.json"
DEFAULT_SIP_KEY = b"codex-dt18-key!!"
CALLER_SIP_KEY = bytes(range(16))


def key_record(
    key_id: str, kind: str, payload: bytes, text: str | None = None
) -> dict[str, Any]:
    row: dict[str, Any] = {
        "id": key_id,
        "kind": kind,
        "payload_hex": payload.hex(),
        "encoded_hex": Key(key_id, kind, payload).encoded.hex(),
        "shared": True,
    }
    if text is not None:
        row["text"] = text
    return row


def make_keys() -> list[dict[str, Any]]:
    rows = [
        key_record("k0", "bytes", b"\x00"),
        key_record("k1", "bytes", b"\x04"),
        key_record("k2", "bytes", b"\x08"),
        key_record("k0-alias", "bytes", b"\x00"),
        key_record("bytes-empty", "bytes", b""),
        key_record("bytes-a", "bytes", b"a"),
        key_record("string-empty", "string", b"", ""),
        key_record("string-a", "string", b"a", "a"),
        key_record("string-supplementary", "string", "🙂".encode("utf-8"), "🙂"),
        key_record("string-composed", "string", "é".encode("utf-8"), "é"),
        key_record("string-decomposed", "string", "e\u0301".encode("utf-8"), "e\u0301"),
    ]
    rows.extend(
        key_record(f"cycle{value}", "bytes", bytes((value,))) for value in range(8)
    )
    return rows


def make_values() -> list[dict[str, Any]]:
    return [
        {"id": "v0", "value": 7},
        {"id": "v1", "value": 11},
        {"id": "null", "value": None},
        {"id": "c0", "value": 13},
        {"id": "c1", "value": 17},
        {"id": "c2", "value": 19},
        {"id": "c3", "value": 23},
        {"id": "c4", "value": 29},
        {"id": "c5", "value": 31},
        {"id": "c6", "value": 37},
        {"id": "c7", "value": 41},
    ]


def make_hash_configurations() -> list[dict[str, Any]]:
    return [
        {"id": "fnv32", "kind": "fnv1a32"},
        {"id": "murmur-seeded", "kind": "murmur3", "seed_hex": "9747b28c"},
        {
            "id": "siphash-caller-key",
            "kind": "sipHash24",
            "key_hex": CALLER_SIP_KEY.hex(),
        },
        {
            "id": "siphash-default-key",
            "kind": "sipHash24",
            "key_hex": DEFAULT_SIP_KEY.hex(),
            "default_compatibility_key": True,
        },
    ]


def hash_answer_rows(hash_configurations: list[dict[str, Any]]) -> list[dict[str, Any]]:
    return [
        {
            "id": "fnv32-abc-known-answer",
            "configuration_id": "fnv32",
            "data_hex": "616263",
            "expected_hex": "1a47e90b",
            "provenance": "DT17 Warrant known-answer literal fnv32-abc",
        },
        {
            "id": "murmur-seeded-fox-known-answer",
            "configuration_id": "murmur-seeded",
            "data_hex": "54686520717569636b2062726f776e20666f78206a756d7073206f76657220746865206c617a7920646f67",
            "expected_hex": "2fa826cd",
            "provenance": "DT17 Warrant known-answer literal murmur-seed-fox",
        },
        {
            "id": "siphash-caller-key-known-answer",
            "configuration_id": "siphash-caller-key",
            "data_hex": "000102030405060708090a0b0c0d0e0f",
            "expected_hex": "3f2acc7f57c29bdb",
            "provenance": "DT17 Warrant known-answer literal siphash-16",
        },
        {
            "id": "siphash-default-key-empty-model-answer",
            "configuration_id": "siphash-default-key",
            "data_hex": "",
            "expected_hex": f"{hash_bytes(b'', hash_configurations[3]):016x}",
            "provenance": "DT17 exact-integer oracle with the DT18 compatibility key",
        },
    ]


def encoded_key_hash_rows(
    key_records: list[dict[str, Any]], hash_configurations: list[dict[str, Any]]
) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for record in key_records:
        key = parse_key(record)
        for configuration in hash_configurations:
            bits = 32 if configuration["kind"] in ("fnv1a32", "murmur3") else 64
            rows.append(
                {
                    "id": f"{key.key_id}-{configuration['id']}",
                    "key_id": key.key_id,
                    "configuration_id": configuration["id"],
                    "input_hex": key.encoded.hex(),
                    "expected_hex": f"{hash_bytes(key.encoded, configuration):0{bits // 4}x}",
                    "provenance": "independent DT17 exact-integer hash oracle",
                }
            )
    return rows


def value_map(value_records: list[dict[str, Any]]) -> dict[str, Any]:
    return {record["id"]: record["value"] for record in value_records}


def key_map(key_records: list[dict[str, Any]]) -> dict[str, Key]:
    return {record["id"]: parse_key(record) for record in key_records}


def make_operation_case(
    case_id: str,
    configuration: dict[str, Any],
    operations: list[dict[str, Any]],
    keys: dict[str, Key],
    values: dict[str, Any],
) -> dict[str, Any]:
    strategy = configuration["strategy"]
    capacity = configuration["capacity"]
    hash_configuration = configuration["hash_configuration"]
    logical = AssociationListModel()
    physical = PhysicalLayoutModel(strategy, capacity, hash_configuration, keys)
    steps: list[dict[str, Any]] = []
    for operation in operations:
        step = dict(operation)
        key = keys[operation["key_id"]]
        observation: dict[str, Any] | None = None
        if operation["op"] == "set":
            value_id = operation["value_id"]
            value = values[value_id]
            logical.set(key, value_id, value)
            physical.set(Entry(key.identity, key.key_id, value_id, value))
        elif operation["op"] == "remove":
            logical.remove(key)
            physical.remove(key)
        elif operation["op"] == "has":
            observation = {"has": logical.has(key)}
        elif operation["op"] == "get":
            present, value, value_id = logical.value(key)
            observation = {"has": present, "value": value, "value_id": value_id}
        else:
            raise ValueError(f"unknown operation: {operation['op']}")

        if {entry.identity: entry.value_id for entry in logical.entries} != {
            entry.identity: entry.value_id for entry in physical.entries()
        }:
            raise AssertionError(f"logical and physical models diverged in {case_id}")
        expected: dict[str, Any] = {"error_id": None, "state": observe_state(physical)}
        if observation is not None:
            expected["observation"] = observation
        step["expected"] = expected
        steps.append(step)

    case_configuration = {
        "capacity": capacity,
        "strategy": strategy,
        "hash_configuration_id": configuration["hash_configuration_id"],
    }
    if configuration.get("hash_configuration_omitted"):
        case_configuration["hash_configuration_omitted"] = True
    return {
        "id": case_id,
        "configuration": case_configuration,
        "initial_state": observe_state(
            physical.__class__(strategy, capacity, hash_configuration, keys)
        ),
        "steps": steps,
    }


def recipe_operations() -> list[dict[str, str]]:
    return history_operations()


def make_history_recipe(keys: dict[str, Key], values: dict[str, Any]) -> dict[str, Any]:
    streams = expected_history_streams(keys, values)
    table = physical_order_table()
    return {
        "key_ids": list(HISTORY_KEYS),
        "value_ids": list(HISTORY_VALUES),
        "operations": recipe_operations(),
        "lengths": list(range(HISTORY_MAX_LENGTH + 1)),
        "max_length": HISTORY_MAX_LENGTH,
        "order": "length-then-lexicographic-operation-index",
        "ordinal_formula": "sum(9^j for j < length) + base9_rank(operation_indices)",
        "history_count": HISTORY_COUNT,
        "initial_capacity": INITIAL_CAPACITY,
        "hash_configuration_id": "fnv32",
        "state_encoding": {
            "alphabet": HISTORY_ALPHABET,
            "key_order": list(HISTORY_KEYS),
            "digit_meanings": {"absent": 0, "v0": 1, "v1": 2},
            "formula": "d0+3*d1+9*d2",
        },
        "physical_order_table": [
            {"id": PHYSICAL_ALPHABET[index], "key_ids": key_order}
            for index, key_order in enumerate(table)
        ],
        "capacity_alphabet": "0123456789abcdefghijklmnopqrstuvwxyz",
        "expected_semantic_streams": {
            strategy: row["semantic"] for strategy, row in streams.items()
        },
        "expected_physical_order_streams": {
            strategy: row["physical"] for strategy, row in streams.items()
        },
        "expected_capacity_streams": {
            strategy: row["capacity"] for strategy, row in streams.items()
        },
    }


def make_wrap_case(keys: dict[str, Key], values: dict[str, Any]) -> dict[str, Any]:
    return make_operation_case(
        "linear-wrap-tombstone-reuse",
        {
            "capacity": 4,
            "strategy": "linearProbing",
            "hash_configuration_id": "fnv32",
            "hash_configuration": {"kind": "fnv1a32"},
        },
        [
            {"op": "set", "key_id": "k0", "value_id": "v0"},
            {"op": "set", "key_id": "k1", "value_id": "v1"},
            {"op": "set", "key_id": "k2", "value_id": "null"},
            {"op": "remove", "key_id": "k0"},
            {"op": "set", "key_id": "k0", "value_id": "v0"},
            {"op": "get", "key_id": "k2"},
        ],
        keys,
        values,
    )


def make_full_cycle_case(
    keys: dict[str, Key], values: dict[str, Any]
) -> dict[str, Any]:
    operations: list[dict[str, Any]] = []
    for index in range(4):
        operations.append(
            {"op": "set", "key_id": f"cycle{index}", "value_id": f"c{index}"}
        )
        operations.append({"op": "remove", "key_id": f"cycle{index}"})
    operations.extend(
        [
            {"op": "set", "key_id": "cycle4", "value_id": "c4"},
            {"op": "get", "key_id": "cycle5"},
            {"op": "has", "key_id": "cycle5"},
            {"op": "remove", "key_id": "cycle5"},
            {"op": "set", "key_id": "cycle5", "value_id": "c5"},
            {"op": "set", "key_id": "cycle6", "value_id": "c6"},
            {"op": "set", "key_id": "cycle7", "value_id": "c7"},
        ]
    )
    return make_operation_case(
        "linear-full-cycle-tombstone-fallback-and-growth",
        {
            "capacity": 4,
            "strategy": "linearProbing",
            "hash_configuration_id": "fnv32",
            "hash_configuration": {"kind": "fnv1a32"},
        },
        operations,
        keys,
        values,
    )


def make_chain_growth_case(
    keys: dict[str, Key], values: dict[str, Any]
) -> dict[str, Any]:
    return make_operation_case(
        "chaining-non-power-capacity-three-to-six",
        {
            "capacity": 3,
            "strategy": "separateChaining",
            "hash_configuration_id": "fnv32",
            "hash_configuration": {"kind": "fnv1a32"},
        },
        [
            {"op": "set", "key_id": "k0", "value_id": "v0"},
            {"op": "set", "key_id": "k1", "value_id": "v1"},
            {"op": "set", "key_id": "k2", "value_id": "null"},
            {"op": "set", "key_id": "k0-alias", "value_id": "c0"},
            {"op": "set", "key_id": "cycle0", "value_id": "c1"},
        ],
        keys,
        values,
    )


def make_algorithm_selection_cases(
    keys: dict[str, Key], values: dict[str, Any]
) -> list[dict[str, Any]]:
    return [
        make_operation_case(
            "murmur-seeded-map-selection",
            {
                "capacity": 5,
                "strategy": "separateChaining",
                "hash_configuration_id": "murmur-seeded",
                "hash_configuration": {"kind": "murmur3", "seed_hex": "9747b28c"},
            },
            [
                {"op": "set", "key_id": "string-a", "value_id": "v0"},
                {"op": "set", "key_id": "string-supplementary", "value_id": "v1"},
                {"op": "get", "key_id": "string-a"},
            ],
            keys,
            values,
        ),
        make_operation_case(
            "siphash-caller-key-map-selection",
            {
                "capacity": 7,
                "strategy": "linearProbing",
                "hash_configuration_id": "siphash-caller-key",
                "hash_configuration": {
                    "kind": "sipHash24",
                    "key_hex": CALLER_SIP_KEY.hex(),
                },
            },
            [
                {"op": "set", "key_id": "string-composed", "value_id": "v0"},
                {"op": "set", "key_id": "string-decomposed", "value_id": "v1"},
                {"op": "get", "key_id": "string-composed"},
                {"op": "has", "key_id": "string-decomposed"},
            ],
            keys,
            values,
        ),
        make_operation_case(
            "constructor-default-siphash-map-selection",
            {
                "capacity": 16,
                "strategy": "separateChaining",
                "hash_configuration_id": "siphash-default-key",
                "hash_configuration": {
                    "kind": "sipHash24",
                    "key_hex": DEFAULT_SIP_KEY.hex(),
                },
                "hash_configuration_omitted": True,
            },
            [
                {"op": "set", "key_id": "bytes-empty", "value_id": "v0"},
                {"op": "set", "key_id": "string-empty", "value_id": "v1"},
                {"op": "get", "key_id": "bytes-empty"},
            ],
            keys,
            values,
        ),
    ]


def make_from_entries_case(
    keys: dict[str, Key], values: dict[str, Any]
) -> dict[str, Any]:
    configuration = {
        "capacity": 4,
        "strategy": "separateChaining",
        "hash_configuration_id": "fnv32",
        "hash_configuration": {"kind": "fnv1a32"},
    }
    model = PhysicalLayoutModel(
        configuration["strategy"],
        configuration["capacity"],
        configuration["hash_configuration"],
        keys,
    )
    logical = AssociationListModel()
    source_entries = [
        {"key_id": "k0", "value_id": "v0"},
        {"key_id": "k1", "value_id": "v1"},
        {"key_id": "k0-alias", "value_id": "null"},
    ]
    for row in source_entries:
        key = keys[row["key_id"]]
        value_id = row["value_id"]
        logical.set(key, value_id, values[value_id])
        model.set(Entry(key.identity, key.key_id, value_id, values[value_id]))
        assert_logical_matches_physical(logical, model, "fromEntries")
    return {
        "id": "from-entries-duplicate-replaces-in-place",
        "configuration": {
            "capacity": configuration["capacity"],
            "strategy": configuration["strategy"],
            "hash_configuration_id": configuration["hash_configuration_id"],
        },
        "source_entries": source_entries,
        "expected_state": observe_state(model),
    }


def make_merge_case(keys: dict[str, Key], values: dict[str, Any]) -> dict[str, Any]:
    left_configuration = {
        "capacity": 3,
        "strategy": "separateChaining",
        "hash_configuration_id": "fnv32",
        "hash_configuration": {"kind": "fnv1a32"},
    }
    right_configuration = {
        "capacity": 5,
        "strategy": "linearProbing",
        "hash_configuration_id": "siphash-caller-key",
        "hash_configuration": {
            "kind": "sipHash24",
            "key_hex": CALLER_SIP_KEY.hex(),
        },
    }
    left_source = [
        {"key_id": "k0", "value_id": "v0"},
        {"key_id": "k1", "value_id": "v1"},
    ]
    right_source = [
        {"key_id": "k1", "value_id": "null"},
        {"key_id": "k2", "value_id": "v0"},
        {"key_id": "string-a", "value_id": "c0"},
    ]

    def build(
        config: dict[str, Any], rows: list[dict[str, str]]
    ) -> PhysicalLayoutModel:
        model = PhysicalLayoutModel(
            config["strategy"], config["capacity"], config["hash_configuration"], keys
        )
        for row in rows:
            key = keys[row["key_id"]]
            value_id = row["value_id"]
            model.set(Entry(key.identity, key.key_id, value_id, values[value_id]))
        return model

    left_model = build(left_configuration, left_source)
    right_model = build(right_configuration, right_source)
    merged = build(left_configuration, left_source)
    merged_logical = AssociationListModel()
    for row in left_source:
        key = keys[row["key_id"]]
        value_id = row["value_id"]
        merged_logical.set(key, value_id, values[value_id])
    right_physical_order = [entry.key_id for entry in right_model.entries()]
    for entry in right_model.entries():
        merged.set(entry)
        key = keys[entry.key_id]
        merged_logical.set(key, entry.value_id, entry.value)
        assert_logical_matches_physical(merged_logical, merged, "merging")
    return {
        "id": "merge-right-wins-left-configuration-and-right-physical-order",
        "left": {
            "configuration": {
                key: value
                for key, value in left_configuration.items()
                if key != "hash_configuration"
            },
            "source_entries": left_source,
            "expected_state": observe_state(left_model),
        },
        "right": {
            "configuration": {
                key: value
                for key, value in right_configuration.items()
                if key != "hash_configuration"
            },
            "source_entries": right_source,
            "expected_state": observe_state(right_model),
        },
        "expected_right_physical_order": right_physical_order,
        "expected_result": {
            "configuration": {
                key: value
                for key, value in left_configuration.items()
                if key != "hash_configuration"
            },
            "state": observe_state(merged),
        },
        "inputs_unchanged": True,
    }


def make_ownership_cases(keys: dict[str, Key]) -> list[dict[str, Any]]:
    sip_hash_config = {"kind": "sipHash24", "key_hex": CALLER_SIP_KEY.hex()}
    return [
        {
            "id": "byte-key-copies-caller-list",
            "action": "construct-then-mutate-source",
            "key_id": "k0",
            "source_payload_hex": "00",
            "mutation": {"index": 0, "new_value": 255},
            "expected_encoded_hex": keys["k0"].encoded.hex(),
        },
        {
            "id": "encoded-key-getter-returns-detached-list",
            "action": "mutate-returned-encoded-bytes-then-read-again",
            "key_id": "k0",
            "mutation": {"index": 0, "new_value": 255},
            "expected_encoded_hex": keys["k0"].encoded.hex(),
        },
        {
            "id": "siphash-configuration-copies-caller-list",
            "action": "construct-hash-config-then-mutate-source-key",
            "source_key_hex": CALLER_SIP_KEY.hex(),
            "mutation": {"index": 0, "new_value": 255},
            "hash_configuration": sip_hash_config,
            "probe_data_hex": "",
            "expected_hash_hex": f"{hash_bytes(b'', sip_hash_config):016x}",
        },
        {
            "id": "entry-key-and-values-snapshots-are-detached",
            "action": "mutate-returned-entries-keys-values-lists",
            "source_entries": [
                {"key_id": "k0", "value": 7},
                {"key_id": "k1", "value": 11},
            ],
            "expected_entries": [
                {"key_id": "k0", "value": 7},
                {"key_id": "k1", "value": 11},
            ],
        },
        {
            "id": "reference-values-are-shallow",
            "action": "mutate-referenced-value-after-persistent-update",
            "initial_object": {"field": 1},
            "mutation": {"field": 2},
            "expected_old_and_new_map_value_field": 2,
        },
    ]


def make_error_cases() -> list[dict[str, Any]]:
    invalid_sip_key = "000102030405060708090a0b0c0d0e"
    return [
        {
            "id": "nonpositive-capacity-precedes-invalid-sip-key",
            "cohort": "shared",
            "operation": "construct",
            "configuration": {
                "capacity": 0,
                "strategy": "separateChaining",
                "hash_configuration": {"kind": "sipHash24", "key_hex": invalid_sip_key},
            },
            "expected_error": {"id": "invalid_capacity", "fields": {"capacity": 0}},
        },
        {
            "id": "capacity-above-production-limit",
            "cohort": "shared",
            "operation": "construct",
            "configuration": {
                "capacity": MAXIMUM_CAPACITY + 1,
                "strategy": "separateChaining",
                "hash_configuration": {"kind": "fnv1a32"},
            },
            "expected_error": {"id": "capacity_limit_exceeded", "fields": {}},
        },
        {
            "id": "invalid-siphash-key-length",
            "cohort": "shared",
            "operation": "construct",
            "configuration": {
                "capacity": 16,
                "strategy": "separateChaining",
                "hash_configuration": {"kind": "sipHash24", "key_hex": invalid_sip_key},
            },
            "expected_error": {
                "id": "invalid_sip_hash_key_length",
                "fields": {"actual_length": 15},
            },
        },
        {
            "id": "oversized-key-setting-is-atomic",
            "cohort": "shared",
            "operation": "set",
            "configuration": {
                "capacity": 16,
                "strategy": "separateChaining",
                "hash_configuration": {"kind": "fnv1a32"},
            },
            "key_recipe": {
                "kind": "repeated-byte",
                "byte": 0,
                "length": MAXIMUM_KEY_BYTES + 1,
            },
            "value": 7,
            "before_state": {"count": 0, "capacity": 16, "entries": []},
            "expected_error": {
                "id": "key_too_long",
                "fields": {
                    "actual": MAXIMUM_KEY_BYTES + 1,
                    "maximum": MAXIMUM_KEY_BYTES,
                },
            },
            "original_map_unchanged": True,
        },
        {
            "id": "oversized-key-lookup-and-removal-are-absent-no-ops",
            "cohort": "shared",
            "operation": "get-has-remove",
            "configuration": {
                "capacity": 16,
                "strategy": "linearProbing",
                "hash_configuration": {"kind": "fnv1a32"},
            },
            "key_recipe": {
                "kind": "repeated-byte",
                "byte": 0,
                "length": MAXIMUM_KEY_BYTES + 1,
            },
            "expected_observation": {"has": False, "value": None},
            "expected_state_unchanged": True,
        },
    ]


def make_dart_only_cases() -> list[dict[str, Any]]:
    return [
        {
            "id": "negative-byte-before-later-overflow-byte",
            "operation": "HashMapKey.bytes",
            "raw_byte_values": [-1, 256],
            "expected_error": {
                "id": "invalid_byte",
                "fields": {"index": 0, "value": -1},
            },
        },
        {
            "id": "invalid-byte-in-sixteen-byte-siphash-key",
            "operation": "HashAlgorithm.sipHash24",
            "raw_key_values": [0, 0, 0, 0, 0, 256, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            "expected_error": {
                "id": "invalid_byte",
                "fields": {"index": 5, "value": 256},
            },
        },
        {
            "id": "negative-murmur-seed",
            "operation": "HashAlgorithm.murmur3",
            "seed": -1,
            "expected_error": {"id": "invalid_murmur_seed", "fields": {"seed": -1}},
        },
        {
            "id": "murmur-seed-above-uint32",
            "operation": "HashAlgorithm.murmur3",
            "seed": 4_294_967_296,
            "expected_error": {
                "id": "invalid_murmur_seed",
                "fields": {"seed": 4_294_967_296},
            },
        },
        {
            "id": "malformed-utf16-surrogate",
            "operation": "HashMapKey.string-from-code-units",
            "utf16_code_units": [97, 55296, 98],
            "expected_error": {
                "id": "malformed_utf16",
                "fields": {"code_unit_offset": 1},
            },
        },
        {
            "id": "invalid-capacity-precedes-iterable-request",
            "operation": "HashMap.fromEntries",
            "configuration": {
                "capacity": 0,
                "strategy": "separateChaining",
                "hash_configuration_id": "fnv32",
            },
            "iterable_behavior": "throw-if-iterator-requested",
            "expected_error": {"id": "invalid_capacity", "fields": {"capacity": 0}},
            "expected_iterator_requests": 0,
        },
        {
            "id": "iterable-exception-propagates-after-one-entry",
            "operation": "HashMap.fromEntries",
            "configuration": {
                "capacity": 4,
                "strategy": "separateChaining",
                "hash_configuration_id": "fnv32",
            },
            "iterable_behavior": "yield-one-then-throw-fixture-iterator-error",
            "expected_exception_type": "FixtureIteratorError",
            "expected_iterator_requests": 1,
            "expected_entries_yielded": 1,
            "expected_map_returned": False,
        },
        {
            "id": "iterable-consumed-once",
            "operation": "HashMap.fromEntries",
            "configuration": {
                "capacity": 4,
                "strategy": "separateChaining",
                "hash_configuration_id": "fnv32",
            },
            "iterable_behavior": "count-iterator-requests",
            "source_entries": [
                {"key_id": "k0", "value_id": "v0"},
                {"key_id": "k1", "value_id": "v1"},
            ],
            "expected_iterator_requests": 1,
            "expected_count": 2,
        },
    ]


def make_boundary_requirements() -> dict[str, Any]:
    return {
        "claim_scope": "Dart public API gate3; not executed by the phase2 Swift consumer",
        "maximum_capacity": MAXIMUM_CAPACITY,
        "separateChaining": {
            "initial_capacity": MAXIMUM_CAPACITY,
            "fill_count": MAXIMUM_CAPACITY,
            "next_distinct_insert": "capacity_limit_exceeded",
            "replacement": "succeeds_without_growth",
            "remove_then_reinsert": "succeeds_after_count_drops_below_capacity",
        },
        "linearProbing": {
            "initial_capacity": MAXIMUM_CAPACITY,
            "fill_count": 786_432,
            "next_distinct_insert": "capacity_limit_exceeded",
            "replacement": "succeeds_without_growth",
            "remove_then_reinsert": "follows_three_quarters_live_count_threshold",
        },
        "both": ["prior_versions_unchanged", "actual_public_methods", "vm_and_node"],
        "construction": "private_transient_builder_with_profile_sequential_order",
        "smaller_injected_limits": "internal-only and labeled separately",
    }


def make_cases() -> dict[str, Any]:
    key_records = make_keys()
    value_records = make_values()
    keys = key_map(key_records)
    values = value_map(value_records)
    hash_configurations = make_hash_configurations()
    return {
        "schema": "dt18-hash-map-v1",
        "oracle": "dt18-association-list-v1",
        "profile": "code/specs/DT18-hash-map-dart-profile.md",
        "limits": {"maximum_capacity": 1_048_576, "maximum_key_byte_count": 1_048_576},
        "hash_configurations": hash_configurations,
        "key_cases": key_records,
        "values": value_records,
        "hash_answers": hash_answer_rows(hash_configurations),
        "encoded_key_hashes": encoded_key_hash_rows(key_records, hash_configurations),
        "configuration_cases": [
            {
                "id": "constructor-defaults",
                "arguments": {"hash_configuration_omitted": True},
                "expected": {
                    "capacity": 16,
                    "strategy": "separateChaining",
                    "hash_configuration_id": "siphash-default-key",
                },
            },
            {
                "id": "arbitrary-non-power-capacity",
                "arguments": {
                    "capacity": 7,
                    "strategy": "linearProbing",
                    "hash_configuration_id": "fnv32",
                },
                "expected": {
                    "capacity": 7,
                    "strategy": "linearProbing",
                    "hash_configuration_id": "fnv32",
                },
            },
        ],
        "operation_cases": [
            make_wrap_case(keys, values),
            make_full_cycle_case(keys, values),
            make_chain_growth_case(keys, values),
            *make_algorithm_selection_cases(keys, values),
        ],
        "from_entries_cases": [make_from_entries_case(keys, values)],
        "merge_cases": [make_merge_case(keys, values)],
        "ownership_cases": make_ownership_cases(keys),
        "error_cases": make_error_cases(),
        "dart_only_cases": make_dart_only_cases(),
        "dart_gate3_boundary_requirements": make_boundary_requirements(),
        "history_recipe": make_history_recipe(keys, values),
        "negative_controls": [
            {
                "id": "wrong-expected-fnv-answer-reaches-public-comparator",
                "source_hash_answer_id": "fnv32-abc-known-answer",
                "expected_xor_mask": "00000001",
                "mutated_expected_hex": "1a47e90a",
                "well_formed": True,
                "required_stage": "actual-public-hash-comparator",
            }
        ],
    }


def canonical_bytes(document: dict[str, Any]) -> bytes:
    return (json.dumps(document, ensure_ascii=True, indent=2) + "\n").encode("ascii")


def main() -> int:
    OUTPUT.write_bytes(canonical_bytes(make_cases()))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
