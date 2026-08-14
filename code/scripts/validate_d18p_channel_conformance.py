#!/usr/bin/env python3
"""Validate and execute the six-language D18P durable-channel conformance gate."""

from __future__ import annotations

import argparse
import base64
import binascii
import hashlib
import json
import re
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE_PATH = REPO_ROOT / "code/fixtures/chief-of-staff-channel/v1/manifest.json"
GENERATOR_PATH = (
    REPO_ROOT
    / "code/packages/rust/chief-of-staff-channel-endpoints/examples/generate_d18p_fixtures.rs"
)
MAXIMUM_MANIFEST_BYTES = 2_000_000
MAXIMUM_COMMAND_OUTPUT_BYTES = 2_000_000
COMMAND_TIMEOUT_SECONDS = 1_200
MAX_U64_TEXT = "18446744073709551615"

EXPECTED_CONSTANTS = {
    "storage_namespace": "chief-channels",
    "content_types": {
        "definition": "application/vnd.coding-adventures.chief-channel-definition-v1",
        "state": "application/vnd.coding-adventures.chief-channel-state-v1",
        "message": "application/vnd.coding-adventures.chief-channel-message-v1",
        "grant": "application/vnd.coding-adventures.chief-channel-key-grant-v1",
        "ack": "application/vnd.coding-adventures.chief-channel-ack-v1",
    },
    "max_receivers": "1024",
    "max_pending_header_bytes": "16384",
    "max_store_cas_attempts": "16",
    "max_definition_cas_attempts": "16",
}
EXPECTED_ERROR_CODES = (
    "invalid_definition",
    "invalid_message_id",
    "definition_not_found",
    "conflicting_definition",
    "corrupt_definition",
    "definition_changed",
    "channel_destroyed",
    "unauthorized_originator",
    "unauthorized_receiver",
    "public_key_mismatch",
    "missing_key_grant",
    "unknown_message_id",
    "unauthorized_message",
    "not_initialized",
    "corrupt_record",
    "pending_append",
    "no_pending_append",
    "pending_header_mismatch",
    "conflicting_record",
    "concurrent_update",
    "invalid_receiver_id",
    "invalid_page_size",
    "acknowledgement_regression",
    "acknowledgement_ahead",
    "acknowledgement_pending",
    "sequence_exhausted",
    "storage_error",
    "wire_error",
    "crypto_error",
    "metadata_error",
)
EXPECTED_CODEC_CASES = {
    "definition-invalid-magic": ("definition", "corrupt_definition"),
    "definition-unsupported-version": ("definition", "corrupt_definition"),
    "definition-truncated": ("definition", "corrupt_definition"),
    "definition-trailing": ("definition", "corrupt_definition"),
    "definition-invalid-channel-uuid": ("definition", "corrupt_definition"),
    "definition-zero-receivers": ("definition", "corrupt_definition"),
    "definition-invalid-lifecycle": ("definition", "corrupt_definition"),
    "state-invalid-magic": ("state", "corrupt_record"),
    "state-unsupported-version": ("state", "corrupt_record"),
    "state-truncated": ("state", "corrupt_record"),
    "state-trailing": ("state", "corrupt_record"),
    "state-invalid-pending-flag": ("state", "corrupt_record"),
    "state-oversized-header": ("state", "corrupt_record"),
    "state-sequence-invariant": ("state", "corrupt_record"),
    "state-channel-invariant": ("state", "corrupt_record"),
    "cursor-invalid-magic": ("cursor", "corrupt_record"),
    "cursor-unsupported-version": ("cursor", "corrupt_record"),
    "cursor-truncated": ("cursor", "corrupt_record"),
    "cursor-trailing": ("cursor", "corrupt_record"),
}
EXPECTED_OPERATION_NEGATIVES = {
    "conflicting-definition": ("create", "conflicting_definition"),
    "session-delivery-enforcement": ("acknowledge", "unknown_message_id"),
    "unauthorized-originator": ("open-originator", "unauthorized_originator"),
    "unauthorized-receiver": ("open-receiver", "unauthorized_receiver"),
    "receiver-public-key-mismatch": ("open-receiver", "public_key_mismatch"),
    "channel-destroyed": ("publish", "channel_destroyed"),
    "missing-key-grant": ("receive", "missing_key_grant"),
    "pending-append": ("reserve", "pending_append"),
    "acknowledgement-pending": ("acknowledge", "acknowledgement_pending"),
    "pending-header-mismatch": ("complete", "pending_header_mismatch"),
    "no-pending-append": ("complete", "no_pending_append"),
    "invalid-page-size": ("read", "invalid_page_size"),
    "invalid-receiver-id": ("receiver-cursor", "invalid_receiver_id"),
    "acknowledgement-ahead": ("acknowledge", "acknowledgement_ahead"),
    "acknowledgement-regression": ("acknowledge", "acknowledgement_regression"),
    "message-key-body-mismatch": ("read", "corrupt_record"),
    "message-content-type-mismatch": ("read", "corrupt_record"),
}
EXPECTED_OPERATION_FIELDS = {
    "definition-create-idempotent": {
        "name",
        "definitions_equal",
        "initial_next_sequence",
    },
    "encrypted-endpoint-round-trip-independent-cursors": {
        "name",
        "published_sequences",
        "binary_receiver_delivered_sequences",
        "text_receiver_delivered_sequences",
        "binary_first_unread_after_zero",
        "binary_first_unread_after_one",
        "binary_first_unread_after_retry",
        "binary_empty_continuation",
        "text_first_unread_after_zero",
    },
    "destroy-idempotent-history-preserved": {
        "name",
        "definitions_equal",
        "history_count",
    },
    "reserve-recover-complete-retry-abandon-gap": {
        "name",
        "recovered_pending_equal",
        "commit_retry_equal",
        "first_d18m_b64",
        "abandoned_sequence",
        "after_gap_sequence",
        "read_sequences",
        "first_page_sequences",
        "first_page_next_start",
        "second_page_sequences",
        "random_access_sequences",
        "empty_continuation",
    },
}
EXPECTED_STORAGE_KEYS = {
    "definition": "018f47a09b6c7def923456789abcdef0/definition",
    "state": "018f47a09b6c7def923456789abcdef0/state/next-sequence",
    "message-zero": "018f47a09b6c7def923456789abcdef0/messages/00000000000000000000",
    "message-max": "018f47a09b6c7def923456789abcdef0/messages/18446744073709551615",
    "message-prefix": "018f47a09b6c7def923456789abcdef0/messages/",
    "grant": "018f47a09b6c7def923456789abcdef0/grants/00000000000000000007/47ffa3ea45a70b8a41c2c0825df323c00a8b7a01c1ea06083cc41dddcc001123",
    "ack-binary-receiver": "018f47a09b6c7def923456789abcdef0/receivers/47ffa3ea45a70b8a41c2c0825df323c00a8b7a01c1ea06083cc41dddcc001123/ack",
}
EXPECTED_OVERSIZE_RECIPES = {
    ("agent-id", "4097", "invalid_definition"),
    ("receiver-count", "1025", "invalid_definition"),
    ("pending-header", "16385", "corrupt_record"),
}
CONSUMER_MARKERS = (
    "chief-of-staff-channel",
    "manifest.json",
    "D18P-durable-channel-fixtures-v1",
    "definition_cases",
    "state_cases",
    "cursor_cases",
    "storage_key_cases",
    "codec_negative_cases",
    "operation_cases",
    "operation_negative_cases",
    "oversize_recipes",
)


class D18PConformanceError(ValueError):
    """Raised when the shared contract or one language lane is incomplete."""


@dataclass(frozen=True)
class Lane:
    """One package-native consumer of the shared D18P manifest."""

    lane_id: str
    package_root: str
    consumer_test: str


LANES = (
    Lane(
        "rust",
        "code/packages/rust/chief-of-staff-channel-endpoints",
        "code/packages/rust/chief-of-staff-channel-endpoints/tests/d18p_fixtures.rs",
    ),
    Lane(
        "typescript",
        "code/packages/typescript/chief-of-staff-channel-store",
        "code/packages/typescript/chief-of-staff-channel-store/tests/d18p-fixtures.test.ts",
    ),
    Lane(
        "python",
        "code/packages/python/chief-of-staff-channel-store",
        "code/packages/python/chief-of-staff-channel-store/tests/test_d18p_fixtures.py",
    ),
    Lane(
        "go",
        "code/packages/go/chief-of-staff-channel-store",
        "code/packages/go/chief-of-staff-channel-store/store_test.go",
    ),
    Lane(
        "ruby",
        "code/packages/ruby/chief-of-staff-channel-store",
        "code/packages/ruby/chief-of-staff-channel-store/test/test_d18p_fixtures.rb",
    ),
    Lane(
        "elixir",
        "code/packages/elixir/chief-of-staff-channel-store",
        "code/packages/elixir/chief-of-staff-channel-store/test/d18p_fixtures_test.exs",
    ),
)
EXPECTED_LANE_IDS = {"rust", "typescript", "python", "go", "ruby", "elixir"}


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise D18PConformanceError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _reject_constant(token: str) -> None:
    raise D18PConformanceError(f"non-finite JSON number: {token}")


def load_manifest(path: Path = FIXTURE_PATH) -> tuple[bytes, dict[str, Any]]:
    try:
        raw = path.read_bytes()
    except OSError as error:
        raise D18PConformanceError(f"cannot read fixture manifest: {error}") from error
    if not raw or len(raw) > MAXIMUM_MANIFEST_BYTES:
        raise D18PConformanceError("fixture manifest has an invalid byte length")
    try:
        document = json.loads(
            raw,
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=_reject_constant,
        )
    except (UnicodeError, json.JSONDecodeError) as error:
        raise D18PConformanceError(
            f"fixture manifest is not strict JSON: {error}"
        ) from error
    if not isinstance(document, dict):
        raise D18PConformanceError("fixture manifest must be an object")
    return raw, document


def _exact_keys(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        raise D18PConformanceError(f"{context}: unexpected object fields")
    return value


def _strict_base64(value: Any, context: str) -> bytes:
    if not isinstance(value, str):
        raise D18PConformanceError(f"{context}: expected base64 text")
    try:
        decoded = base64.b64decode(value, validate=True)
    except (ValueError, binascii.Error) as error:
        raise D18PConformanceError(f"{context}: invalid base64") from error
    if base64.b64encode(decoded).decode("ascii") != value:
        raise D18PConformanceError(f"{context}: non-canonical base64")
    return decoded


def _named_items(value: Any, context: str) -> dict[str, dict[str, Any]]:
    if not isinstance(value, list):
        raise D18PConformanceError(f"{context}: expected an array")
    result: dict[str, dict[str, Any]] = {}
    for index, item in enumerate(value):
        if not isinstance(item, dict) or not isinstance(item.get("name"), str):
            raise D18PConformanceError(f"{context} {index}: invalid named object")
        name = item["name"]
        if name in result:
            raise D18PConformanceError(f"{context}: duplicated case {name}")
        result[name] = item
    return result


def validate_manifest(document: dict[str, Any]) -> None:
    _exact_keys(
        document,
        {
            "fixture_format",
            "spec",
            "generator_blob_sha1",
            "warning",
            "constants",
            "test_keys",
            "definition_cases",
            "state_cases",
            "cursor_cases",
            "storage_key_cases",
            "codec_negative_cases",
            "operation_cases",
            "operation_negative_cases",
            "oversize_recipes",
            "stable_error_codes",
        },
        "manifest",
    )
    if document["fixture_format"] != "D18P-durable-channel-fixtures-v1":
        raise D18PConformanceError("fixture identity is not D18P v1")
    if document["spec"] != "code/specs/D18P-chief-of-staff-durable-channel-profile.md":
        raise D18PConformanceError("fixture spec path is not canonical")
    if not re.fullmatch(r"[0-9a-f]{40}", str(document["generator_blob_sha1"])):
        raise D18PConformanceError("generator blob identifier is invalid")
    if (
        not isinstance(document["warning"], str)
        or "test-only" not in document["warning"]
    ):
        raise D18PConformanceError("fixture secret-material warning is missing")
    if document["constants"] != EXPECTED_CONSTANTS:
        raise D18PConformanceError("D18P constants drifted")
    if tuple(document["stable_error_codes"]) != EXPECTED_ERROR_CODES:
        raise D18PConformanceError("stable D18P error roster drifted")

    keys = _exact_keys(
        document["test_keys"],
        {
            "originator_signing_seed_hex",
            "originator_public_key_hex",
            "channel_master_key_hex",
            "binary_receiver_private_key_hex",
            "binary_receiver_public_key_hex",
            "text_receiver_private_key_hex",
            "text_receiver_public_key_hex",
        },
        "test_keys",
    )
    for field, value in keys.items():
        if not re.fullmatch(r"[0-9a-f]{64}", str(value)):
            raise D18PConformanceError(f"test_keys.{field}: invalid 32-byte hex")

    definitions = _named_items(document["definition_cases"], "definition cases")
    if set(definitions) != {"active-binary-sorted-receivers", "destroyed"}:
        raise D18PConformanceError("definition fixture roster drifted")
    active = _exact_keys(
        definitions["active-binary-sorted-receivers"],
        {"name", "lifecycle", "canonical_receiver_ids_b64", "d18c_b64"},
        "active definition",
    )
    destroyed = _exact_keys(
        definitions["destroyed"],
        {"name", "lifecycle", "d18c_b64"},
        "destroyed definition",
    )
    if active["lifecycle"] != "active" or destroyed["lifecycle"] != "destroyed":
        raise D18PConformanceError("definition lifecycle semantics drifted")
    if active["canonical_receiver_ids_b64"] != ["AP8B", "emVk"]:
        raise D18PConformanceError("canonical receiver ordering drifted")
    for name, item in definitions.items():
        if not _strict_base64(item["d18c_b64"], name).startswith(b"D18C\x01"):
            raise D18PConformanceError(f"{name}: invalid D18C prefix")

    states = _named_items(document["state_cases"], "state cases")
    if set(states) != {"initial", "pending"}:
        raise D18PConformanceError("state fixture roster drifted")
    initial = _exact_keys(
        states["initial"],
        {"name", "next_sequence", "pending", "d18s_b64"},
        "initial state",
    )
    pending = _exact_keys(
        states["pending"],
        {"name", "next_sequence", "pending", "d18h_b64", "d18s_b64"},
        "pending state",
    )
    if (initial["next_sequence"], initial["pending"]) != ("0", False) or (
        pending["next_sequence"],
        pending["pending"],
    ) != ("8", True):
        raise D18PConformanceError("state transition fixture semantics drifted")
    for name, item in states.items():
        if not _strict_base64(item["d18s_b64"], name).startswith(b"D18S\x01"):
            raise D18PConformanceError(f"{name}: invalid D18S prefix")
    if not _strict_base64(pending["d18h_b64"], "pending header").startswith(
        b"D18H\x01"
    ):
        raise D18PConformanceError("pending header: invalid D18H prefix")

    cursors = document["cursor_cases"]
    if not isinstance(cursors, list) or len(cursors) != 4:
        raise D18PConformanceError("cursor fixture roster drifted")
    cursor_values: list[str] = []
    for index, value in enumerate(cursors):
        item = _exact_keys(
            value, {"first_unread_sequence", "d18a_b64"}, f"cursor {index}"
        )
        cursor_values.append(str(item["first_unread_sequence"]))
        if not _strict_base64(item["d18a_b64"], f"cursor {index}").startswith(
            b"D18A\x01"
        ):
            raise D18PConformanceError(f"cursor {index}: invalid D18A prefix")
    if cursor_values != ["0", "1", "42", MAX_U64_TEXT]:
        raise D18PConformanceError("cursor boundary semantics drifted")

    storage = _named_items(document["storage_key_cases"], "storage key cases")
    for name, item in storage.items():
        _exact_keys(item, {"name", "expected_key"}, f"storage key {name}")
    if {
        name: item["expected_key"] for name, item in storage.items()
    } != EXPECTED_STORAGE_KEYS:
        raise D18PConformanceError("canonical storage keys drifted")

    codec = _named_items(document["codec_negative_cases"], "codec negative cases")
    observed_codec: dict[str, tuple[str, str]] = {}
    for name, item in codec.items():
        _exact_keys(item, {"name", "kind", "record_b64", "expected_error"}, name)
        _strict_base64(item["record_b64"], name)
        observed_codec[name] = (str(item["kind"]), str(item["expected_error"]))
    if observed_codec != EXPECTED_CODEC_CASES:
        raise D18PConformanceError("codec negative roster or semantics drifted")

    operations = _named_items(document["operation_cases"], "operation cases")
    if set(operations) != set(EXPECTED_OPERATION_FIELDS):
        raise D18PConformanceError("operation fixture roster drifted")
    for name, fields in EXPECTED_OPERATION_FIELDS.items():
        _exact_keys(operations[name], fields, f"operation {name}")
    d18m = _strict_base64(
        operations["reserve-recover-complete-retry-abandon-gap"]["first_d18m_b64"],
        "operation D18M",
    )
    if not d18m.startswith(b"D18M\x01"):
        raise D18PConformanceError("operation fixture has an invalid D18M prefix")

    negatives = _named_items(
        document["operation_negative_cases"], "operation negatives"
    )
    observed_negatives: dict[str, tuple[str, str]] = {}
    for name, item in negatives.items():
        _exact_keys(item, {"name", "operation", "expected_error"}, name)
        observed_negatives[name] = (str(item["operation"]), str(item["expected_error"]))
    if observed_negatives != EXPECTED_OPERATION_NEGATIVES:
        raise D18PConformanceError("operation negative roster or semantics drifted")

    recipes = document["oversize_recipes"]
    if not isinstance(recipes, list):
        raise D18PConformanceError("oversize_recipes must be an array")
    observed_recipes = set()
    for index, value in enumerate(recipes):
        item = _exact_keys(
            value,
            {"field", "declared_length", "expected_error"},
            f"oversize recipe {index}",
        )
        observed_recipes.add(
            (
                str(item["field"]),
                str(item["declared_length"]),
                str(item["expected_error"]),
            )
        )
    if observed_recipes != EXPECTED_OVERSIZE_RECIPES or len(recipes) != len(
        EXPECTED_OVERSIZE_RECIPES
    ):
        raise D18PConformanceError("oversize recipe roster drifted")


def git_blob_sha1(data: bytes) -> str:
    header = f"blob {len(data)}\0".encode("ascii")
    return hashlib.sha1(header + data, usedforsecurity=False).hexdigest()


def validate_lane_roster(lanes: tuple[Lane, ...] = LANES) -> None:
    lane_ids = [lane.lane_id for lane in lanes]
    if set(lane_ids) != EXPECTED_LANE_IDS or len(lane_ids) != len(EXPECTED_LANE_IDS):
        raise D18PConformanceError("language lanes must be exactly the supported six")
    package_roots = [lane.package_root for lane in lanes]
    consumer_tests = [lane.consumer_test for lane in lanes]
    if len(package_roots) != len(set(package_roots)) or len(consumer_tests) != len(
        set(consumer_tests)
    ):
        raise D18PConformanceError("language lane paths must be unique")


def validate_repository(root: Path = REPO_ROOT) -> dict[str, Any]:
    validate_lane_roster()
    _, document = load_manifest(root / FIXTURE_PATH.relative_to(REPO_ROOT))
    validate_manifest(document)

    generator_path = root / GENERATOR_PATH.relative_to(REPO_ROOT)
    try:
        generator_hash = git_blob_sha1(generator_path.read_bytes())
    except OSError as error:
        raise D18PConformanceError(f"cannot read fixture generator: {error}") from error
    if generator_hash != document["generator_blob_sha1"]:
        raise D18PConformanceError(
            "fixture generator changed without regenerating the shared manifest"
        )

    for lane in LANES:
        package_root = root / lane.package_root
        build_path = package_root / "BUILD"
        consumer_path = root / lane.consumer_test
        if (
            not package_root.is_dir()
            or not build_path.is_file()
            or not consumer_path.is_file()
        ):
            raise D18PConformanceError(
                f"{lane.lane_id}: package, BUILD, or consumer is missing"
            )
        try:
            consumer = consumer_path.read_text(encoding="utf-8")
        except (OSError, UnicodeError) as error:
            raise D18PConformanceError(
                f"{lane.lane_id}: cannot read fixture consumer: {error}"
            ) from error
        missing = [marker for marker in CONSUMER_MARKERS if marker not in consumer]
        if missing:
            raise D18PConformanceError(
                f"{lane.lane_id}: fixture consumer is missing markers: {', '.join(missing)}"
            )
    return document


def _run(command: list[str], cwd: Path, context: str) -> None:
    print(f"--- {context}: {' '.join(command)}", flush=True)
    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            check=False,
            capture_output=True,
            timeout=COMMAND_TIMEOUT_SECONDS,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise D18PConformanceError(
            f"{context}: command could not complete: {error}"
        ) from error
    output = result.stdout + result.stderr
    if len(output) > MAXIMUM_COMMAND_OUTPUT_BYTES:
        raise D18PConformanceError(
            f"{context}: command output exceeded the safety limit"
        )
    if output:
        print(output.decode("utf-8", errors="replace"), end="")
    if result.returncode != 0:
        raise D18PConformanceError(
            f"{context}: command failed with exit code {result.returncode}"
        )


def run_lane(lane: Lane, root: Path = REPO_ROOT) -> None:
    package_root = root / lane.package_root
    try:
        commands = [
            line.strip()
            for line in (package_root / "BUILD")
            .read_text(encoding="utf-8")
            .splitlines()
            if line.strip() and not line.lstrip().startswith("#")
        ]
    except (OSError, UnicodeError) as error:
        raise D18PConformanceError(
            f"{lane.lane_id}: cannot read package BUILD: {error}"
        ) from error
    if not commands:
        raise D18PConformanceError(f"{lane.lane_id}: package BUILD has no commands")
    for index, command in enumerate(commands, start=1):
        _run(
            ["bash", "-c", command],
            package_root,
            f"D18P {lane.lane_id} lane command {index}/{len(commands)}",
        )


def verify_regeneration(document: dict[str, Any], root: Path = REPO_ROOT) -> None:
    with tempfile.TemporaryDirectory(prefix="d18p-channel-conformance-") as directory:
        generated = Path(directory) / "manifest.json"
        command = [
            "cargo",
            "run",
            "--quiet",
            "--manifest-path",
            "code/packages/rust/Cargo.toml",
            "-p",
            "chief-of-staff-channel-endpoints",
            "--example",
            "generate_d18p_fixtures",
            "--",
            str(generated),
            document["generator_blob_sha1"],
        ]
        _run(command, root, "D18P fixture regeneration")
        if (
            generated.read_bytes()
            != (root / FIXTURE_PATH.relative_to(REPO_ROOT)).read_bytes()
        ):
            raise D18PConformanceError("generated D18P manifest is stale")


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check-only",
        action="store_true",
        help="Validate the manifest, generator provenance, and six consumer registrations only.",
    )
    parser.add_argument(
        "--lane",
        action="append",
        choices=sorted(EXPECTED_LANE_IDS),
        help="Run only the selected language lane; may be repeated.",
    )
    return parser.parse_args()


def main() -> int:
    arguments = parse_arguments()
    try:
        document = validate_repository()
        print(
            "validated the D18P manifest, generator provenance, and six consumer registrations"
        )
        if arguments.check_only:
            return 0
        selected = set(arguments.lane or EXPECTED_LANE_IDS)
        for lane in LANES:
            if lane.lane_id in selected:
                run_lane(lane)
        if "rust" in selected:
            verify_regeneration(document)
        print("D18P six-language durable-channel conformance passed")
        return 0
    except D18PConformanceError as error:
        print(f"D18P channel conformance failed: {error}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
