"""Validate DT18 hash-map fixture shape, model values, and canonical bytes."""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator

from generate import OUTPUT, canonical_bytes, make_cases
from hash_map_oracle import (
    HISTORY_ALPHABET,
    HISTORY_COUNT,
    HISTORY_KEYS,
    HISTORY_MAX_LENGTH,
    HISTORY_VALUES,
    PHYSICAL_ALPHABET,
    decode_history_ordinal,
    history_ordinal,
    history_operations,
    hash_bytes,
    parse_key,
    bucket_index,
)


HERE = Path(__file__).resolve().parent
SCHEMA_PATH = HERE / "schema.json"


class FixtureValidationError(ValueError):
    """The fixture shape or one of its independent expected values is wrong."""


def _first_difference(expected: Any, actual: Any, path: str = "$") -> str:
    if type(expected) is not type(actual):
        return (
            f"{path}: expected {type(expected).__name__}, got {type(actual).__name__}"
        )
    if isinstance(expected, dict):
        if expected.keys() != actual.keys():
            missing = sorted(expected.keys() - actual.keys())
            extra = sorted(actual.keys() - expected.keys())
            return f"{path}: missing keys {missing}; extra keys {extra}"
        for key in expected:
            difference = _first_difference(expected[key], actual[key], f"{path}.{key}")
            if difference:
                return difference
        return ""
    if isinstance(expected, list):
        if len(expected) != len(actual):
            return f"{path}: expected {len(expected)} items, got {len(actual)}"
        for index, (left, right) in enumerate(zip(expected, actual, strict=True)):
            difference = _first_difference(left, right, f"{path}[{index}]")
            if difference:
                return difference
        return ""
    if expected != actual:
        return f"{path}: expected {expected!r}, got {actual!r}"
    return ""


def _validate_unique_ids(document: dict[str, Any]) -> None:
    groups = (
        "hash_configurations",
        "key_cases",
        "values",
        "hash_answers",
        "encoded_key_hashes",
        "configuration_cases",
        "operation_cases",
        "from_entries_cases",
        "merge_cases",
        "ownership_cases",
        "error_cases",
        "dart_only_cases",
        "negative_controls",
    )
    for group_name in groups:
        ids = [row["id"] for row in document[group_name]]
        if len(ids) != len(set(ids)):
            raise FixtureValidationError(f"{group_name} contains duplicate IDs")


def _validate_keys_and_hash_answers(document: dict[str, Any]) -> None:
    keys = {row["id"]: row for row in document["key_cases"]}
    configurations = {row["id"]: row for row in document["hash_configurations"]}
    for row in document["key_cases"]:
        key = parse_key(row)
        if key.encoded.hex() != row["encoded_hex"]:
            raise FixtureValidationError(f"key encoding mismatch for {row['id']}")
        if key.payload.hex() != row["payload_hex"]:
            raise FixtureValidationError(f"key payload mismatch for {row['id']}")

    identities = {parse_key(keys[key_id]).identity for key_id in HISTORY_KEYS}
    if len(identities) != len(HISTORY_KEYS):
        raise FixtureValidationError(
            "the three exhaustive history keys must be distinct"
        )
    fnv = configurations["fnv32"]
    if any(
        bucket_index(parse_key(keys[key_id]), 4, fnv) != 2 for key_id in HISTORY_KEYS
    ):
        raise FixtureValidationError(
            "history keys must collide at bucket two under FNV32/capacity four"
        )

    for row in document["hash_answers"]:
        actual = hash_bytes(
            bytes.fromhex(row["data_hex"]), configurations[row["configuration_id"]]
        )
        if f"{actual:0{len(row['expected_hex'])}x}" != row["expected_hex"]:
            raise FixtureValidationError(f"hash answer mismatch for {row['id']}")

    for row in document["encoded_key_hashes"]:
        key = parse_key(keys[row["key_id"]])
        configuration = configurations[row["configuration_id"]]
        if row["input_hex"] != key.encoded.hex():
            raise FixtureValidationError(f"encoded hash input mismatch for {row['id']}")
        actual = hash_bytes(key.encoded, configuration)
        if f"{actual:0{len(row['expected_hex'])}x}" != row["expected_hex"]:
            raise FixtureValidationError(f"encoded key hash mismatch for {row['id']}")


def _validate_history_recipe(recipe: dict[str, Any]) -> None:
    if recipe["history_count"] != HISTORY_COUNT:
        raise FixtureValidationError("history recipe has the wrong total count")
    if recipe["operations"] != history_operations():
        raise FixtureValidationError("history operation alphabet or order changed")
    if recipe["ordinal_formula"] != (
        "sum(9^j for j < length) + base9_rank(operation_indices)"
    ):
        raise FixtureValidationError("history ordinal formula changed")
    if recipe["key_ids"] != list(HISTORY_KEYS):
        raise FixtureValidationError("history key order changed")
    if recipe["value_ids"] != list(HISTORY_VALUES):
        raise FixtureValidationError("history value order changed")
    if recipe["lengths"] != list(range(HISTORY_MAX_LENGTH + 1)):
        raise FixtureValidationError("history length domain changed")

    for ordinal in range(HISTORY_COUNT):
        operations = decode_history_ordinal(ordinal)
        if history_ordinal(operations) != ordinal:
            raise FixtureValidationError(
                f"history ordinal {ordinal} does not round-trip"
            )

    physical_rows = recipe["physical_order_table"]
    if len(physical_rows) != len(PHYSICAL_ALPHABET):
        raise FixtureValidationError("physical order table must contain 16 rows")
    if [row["id"] for row in physical_rows] != list(PHYSICAL_ALPHABET):
        raise FixtureValidationError("physical order IDs are missing or reordered")

    for kind, alphabet in (
        ("expected_semantic_streams", HISTORY_ALPHABET),
        ("expected_physical_order_streams", PHYSICAL_ALPHABET),
        ("expected_capacity_streams", recipe["capacity_alphabet"]),
    ):
        streams = recipe[kind]
        for strategy in ("separateChaining", "linearProbing"):
            stream = streams[strategy]
            if len(stream) != HISTORY_COUNT:
                raise FixtureValidationError(
                    f"{kind}.{strategy} has length {len(stream)}, expected {HISTORY_COUNT}"
                )
            if any(character not in alphabet for character in stream):
                raise FixtureValidationError(
                    f"{kind}.{strategy} contains an invalid symbol"
                )

    for strategy in ("separateChaining", "linearProbing"):
        if recipe["expected_semantic_streams"][strategy][0] != HISTORY_ALPHABET[0]:
            raise FixtureValidationError(
                f"{strategy} empty-history state anchor changed"
            )
        if (
            recipe["expected_physical_order_streams"][strategy][0]
            != PHYSICAL_ALPHABET[0]
        ):
            raise FixtureValidationError(
                f"{strategy} empty-history order anchor changed"
            )
        if recipe["expected_capacity_streams"][strategy][0] != "4":
            raise FixtureValidationError(f"{strategy} initial-capacity anchor changed")


def _validate_negative_control(document: dict[str, Any]) -> None:
    controls = {row["id"]: row for row in document["negative_controls"]}
    answers = {row["id"]: row for row in document["hash_answers"]}
    control = controls["wrong-expected-fnv-answer-reaches-public-comparator"]
    source = answers[control["source_hash_answer_id"]]
    expected = int(source["expected_hex"], 16)
    mask = int(control["expected_xor_mask"], 16)
    mutated = f"{expected ^ mask:0{len(source['expected_hex'])}x}"
    if mutated != control["mutated_expected_hex"]:
        raise FixtureValidationError(
            "wrong-answer control does not apply its declared XOR mask"
        )
    if len(mutated) != len(source["expected_hex"]) or any(
        character not in "0123456789abcdef" for character in mutated
    ):
        raise FixtureValidationError(
            "wrong-answer control is not a well-formed hash value"
        )
    if control["required_stage"] != "actual-public-hash-comparator":
        raise FixtureValidationError(
            "wrong-answer control must reach the public comparator"
        )


def validate_document(document: dict[str, Any]) -> None:
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(document),
        key=lambda error: (
            tuple(str(part) for part in error.absolute_path),
            error.message,
        ),
    )
    if errors:
        error = errors[0]
        location = ".".join(str(part) for part in error.absolute_path) or "$"
        raise FixtureValidationError(f"schema {location}: {error.message}")

    _validate_unique_ids(document)
    _validate_keys_and_hash_answers(document)
    _validate_history_recipe(document["history_recipe"])
    _validate_negative_control(document)

    expected = make_cases()
    difference = _first_difference(expected, document)
    if difference:
        raise FixtureValidationError(f"independent model mismatch: {difference}")


def validate_bytes(raw: bytes) -> dict[str, Any]:
    try:
        document = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise FixtureValidationError(
            f"fixture is not valid UTF-8 JSON: {error}"
        ) from error
    if not isinstance(document, dict):
        raise FixtureValidationError("fixture top level must be an object")
    validate_document(document)
    return document


def validate_file(path: Path = OUTPUT) -> tuple[dict[str, Any], str]:
    raw = path.read_bytes()
    document = validate_bytes(raw)
    expected_bytes = canonical_bytes(make_cases())
    if raw != expected_bytes:
        raise FixtureValidationError(
            "cases.json differs from deterministic generator bytes"
        )
    return document, hashlib.sha256(raw).hexdigest()


def main() -> int:
    try:
        document, digest = validate_file()
    except (OSError, FixtureValidationError) as error:
        print(f"DT18 hash-map fixture validation failed: {error}", file=sys.stderr)
        return 1
    recipe = document["history_recipe"]
    print(
        "DT18 hash-map fixture valid: "
        f"sha256={digest} bytes={OUTPUT.stat().st_size} "
        f"operations={len(document['operation_cases'])} "
        f"histories_per_strategy={recipe['history_count']} "
        "semantic_physical_capacity_streams=3x2"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
