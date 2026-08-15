#!/usr/bin/env python3
"""Validate and execute the six-language D18Q key-grant conformance gate."""

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
FIXTURE_PATH = (
    REPO_ROOT / "code/fixtures/chief-of-staff-channel-key-grant/v1/manifest.json"
)
GENERATOR_PATH = (
    REPO_ROOT
    / "code/packages/rust/chief-of-staff-channel-crypto/examples/generate_d18q_fixtures.rs"
)
MAXIMUM_MANIFEST_BYTES = 2_000_000
MAXIMUM_COMMAND_OUTPUT_BYTES = 2_000_000
COMMAND_TIMEOUT_SECONDS = 1_200

EXPECTED_CONSTANTS = {
    "key_grant_context_ascii": "chief-channel-key-grant-v1",
    "key_wrap_context_ascii": "chief-channel-key-wrap-v1",
    "max_identity_bytes": "4096",
    "wire_magic_ascii": "D18G",
    "wire_version": "1",
}
EXPECTED_ERROR_CODES = (
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
EXPECTED_POSITIVES = (
    "epoch-zero-receiver-a",
    "epoch-zero-receiver-b",
    "maximum-epoch-receiver-a",
)
EXPECTED_STRUCTURAL_NEGATIVES = {
    "wrong-magic": "invalid_magic",
    "unsupported-version": "unsupported_version",
    "trailing-byte": "trailing_bytes",
}
EXPECTED_FIELD_NEGATIVES = {
    "empty-originator": "invalid_field",
    "empty-receiver": "invalid_field",
    "invalid-uuid-version": "invalid_field",
    "invalid-uuid-variant": "invalid_field",
    "oversized-originator": "length_limit_exceeded",
    "oversized-receiver": "length_limit_exceeded",
}
EXPECTED_SEAL_NEGATIVES = {
    "low-order-receiver-public-key": "invalid_key_agreement",
}
EXPECTED_OPENING_NEGATIVES = {
    "unexpected-originator": "unexpected_originator",
    "unexpected-receiver": "unexpected_receiver",
    "unexpected-channel": "unexpected_channel",
    "invalid-signature": "invalid_signature",
    "invalid-signature-before-key-agreement": "invalid_signature",
    "low-order-ephemeral-public-key": "invalid_key_agreement",
    "wrong-receiver-private-key": "authentication_failed",
    "wrong-wrapping-nonce": "authentication_failed",
    "mutated-wrapped-cmk": "authentication_failed",
    "mutated-tag": "authentication_failed",
    "epoch-derivation-binding": "authentication_failed",
    "receiver-derivation-binding": "authentication_failed",
    "channel-aad-binding": "authentication_failed",
    "originator-aad-binding": "authentication_failed",
}
EXPECTED_TRACE_STEPS = {
    "install-epoch-zero": ("epoch_zero_b64", "installed", "0", ("0",)),
    "retry-epoch-zero": ("epoch_zero_b64", "idempotent", "0", ("0",)),
    "same-epoch-conflict": (
        "same_epoch_conflict_b64",
        "conflicting_grant",
        "0",
        ("0",),
    ),
    "failed-higher-open": (
        "failed_higher_epoch_b64",
        "authentication_failed",
        "0",
        ("0",),
    ),
    "install-skipped-epoch-three": (
        "skipped_epoch_three_b64",
        "installed",
        "3",
        ("0", "3"),
    ),
    "decreasing-epoch": (
        "epoch_zero_b64",
        "decreasing_epoch",
        "3",
        ("0", "3"),
    ),
}
CONSUMER_MARKERS = (
    "chief-of-staff-channel-key-grant",
    "manifest.json",
    "D18Q-channel-key-grant-fixtures-v1",
    "positive_cases",
    "structural_negative_cases",
    "opening_negative_cases",
    "receiver_state_trace",
    "rotation_case",
    "secret_erasure_capabilities",
    "stable_error_codes",
)


class D18QConformanceError(ValueError):
    """Raised when the shared contract or one language lane is incomplete."""


@dataclass(frozen=True)
class Lane:
    """One package-native consumer of the shared D18Q manifest."""

    lane_id: str
    package_root: str
    consumer_test: str


LANES = (
    Lane(
        "rust",
        "code/packages/rust/chief-of-staff-channel-crypto",
        "code/packages/rust/chief-of-staff-channel-crypto/tests/d18q_fixtures.rs",
    ),
    Lane(
        "typescript",
        "code/packages/typescript/chief-of-staff-channel-crypto",
        "code/packages/typescript/chief-of-staff-channel-crypto/tests/d18q-fixtures.test.ts",
    ),
    Lane(
        "python",
        "code/packages/python/chief-of-staff-channel-crypto",
        "code/packages/python/chief-of-staff-channel-crypto/tests/test_d18q_fixtures.py",
    ),
    Lane(
        "go",
        "code/packages/go/chief-of-staff-channel-crypto",
        "code/packages/go/chief-of-staff-channel-crypto/grant_test.go",
    ),
    Lane(
        "ruby",
        "code/packages/ruby/chief-of-staff-channel-crypto",
        "code/packages/ruby/chief-of-staff-channel-crypto/test/test_d18q_fixtures.rb",
    ),
    Lane(
        "elixir",
        "code/packages/elixir/chief-of-staff-channel-crypto",
        "code/packages/elixir/chief-of-staff-channel-crypto/test/d18q_fixtures_test.exs",
    ),
)
EXPECTED_LANE_IDS = {"rust", "typescript", "python", "go", "ruby", "elixir"}


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise D18QConformanceError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _reject_constant(token: str) -> None:
    raise D18QConformanceError(f"non-finite JSON number: {token}")


def load_manifest(path: Path = FIXTURE_PATH) -> tuple[bytes, dict[str, Any]]:
    try:
        raw = path.read_bytes()
    except OSError as error:
        raise D18QConformanceError(f"cannot read fixture manifest: {error}") from error
    if not raw or len(raw) > MAXIMUM_MANIFEST_BYTES:
        raise D18QConformanceError("fixture manifest has an invalid byte length")
    try:
        document = json.loads(
            raw,
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=_reject_constant,
        )
    except (UnicodeError, json.JSONDecodeError) as error:
        raise D18QConformanceError(
            f"fixture manifest is not strict JSON: {error}"
        ) from error
    if not isinstance(document, dict):
        raise D18QConformanceError("fixture manifest must be an object")
    return raw, document


def _exact_keys(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        raise D18QConformanceError(f"{context}: unexpected object fields")
    return value


def _strict_base64(value: Any, context: str) -> bytes:
    if not isinstance(value, str):
        raise D18QConformanceError(f"{context}: expected base64 text")
    try:
        decoded = base64.b64decode(value, validate=True)
    except (ValueError, binascii.Error) as error:
        raise D18QConformanceError(f"{context}: invalid base64") from error
    if base64.b64encode(decoded).decode("ascii") != value:
        raise D18QConformanceError(f"{context}: non-canonical base64")
    return decoded


def _hex(value: Any, byte_length: int, context: str) -> None:
    if not re.fullmatch(rf"[0-9a-f]{{{byte_length * 2}}}", str(value)):
        raise D18QConformanceError(f"{context}: invalid {byte_length}-byte hex")


def _named_items(value: Any, context: str) -> dict[str, dict[str, Any]]:
    if not isinstance(value, list):
        raise D18QConformanceError(f"{context}: expected an array")
    result: dict[str, dict[str, Any]] = {}
    for index, item in enumerate(value):
        if not isinstance(item, dict) or not isinstance(item.get("name"), str):
            raise D18QConformanceError(f"{context} {index}: invalid named object")
        if item["name"] in result:
            raise D18QConformanceError(f"{context}: duplicated case {item['name']}")
        result[item["name"]] = item
    return result


def _validate_named_errors(
    value: Any,
    expected: dict[str, str],
    fields: set[str],
    context: str,
) -> dict[str, dict[str, Any]]:
    cases = _named_items(value, context)
    for name, item in cases.items():
        _exact_keys(item, fields, f"{context} {name}")
    if {name: str(item["expected_error"]) for name, item in cases.items()} != expected:
        raise D18QConformanceError(f"{context} roster or semantics drifted")
    return cases


def validate_manifest(document: dict[str, Any]) -> None:
    _exact_keys(
        document,
        {
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
        },
        "manifest",
    )
    if document["fixture_format"] != "D18Q-channel-key-grant-fixtures-v1":
        raise D18QConformanceError("fixture identity is not D18Q v1")
    if (
        document["spec"]
        != "code/specs/D18Q-chief-of-staff-channel-key-grant-profile.md"
    ):
        raise D18QConformanceError("fixture spec path is not canonical")
    if not re.fullmatch(r"[0-9a-f]{40}", str(document["generator_blob_sha1"])):
        raise D18QConformanceError("generator blob identifier is invalid")
    warning = document["warning"]
    if (
        not isinstance(warning, str)
        or "test-only" not in warning
        or "Never log" not in warning
    ):
        raise D18QConformanceError("fixture secret-material warning is missing")
    if document["constants"] != EXPECTED_CONSTANTS:
        raise D18QConformanceError("D18Q constants drifted")
    if tuple(document["stable_error_codes"]) != EXPECTED_ERROR_CODES:
        raise D18QConformanceError("stable D18Q error roster drifted")
    if (
        document["secret_erasure_capabilities"]
        != [
            "guaranteed",
            "best_effort",
            "not_enforceable",
        ]
        or document["rust_secret_erasure_capability"] != "guaranteed"
    ):
        raise D18QConformanceError("secret-erasure capability vocabulary drifted")

    signing_key = _exact_keys(
        document["test_signing_key"], {"seed_hex", "public_key_hex"}, "test signing key"
    )
    _hex(signing_key["seed_hex"], 32, "signing seed")
    _hex(signing_key["public_key_hex"], 32, "signing public key")

    positives = _named_items(document["positive_cases"], "positive cases")
    if tuple(positives) != EXPECTED_POSITIVES:
        raise D18QConformanceError("positive fixture roster drifted")
    positive_fields = {
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
    }
    hex_lengths = {
        "channel_id_hex": 16,
        "cmk_hex": 32,
        "receiver_private_key_hex": 32,
        "receiver_public_key_hex": 32,
        "ephemeral_private_key_hex": 32,
        "ephemeral_public_key_hex": 32,
        "shared_secret_hex": 32,
        "wrapping_key_hex": 32,
        "wrapping_nonce_hex": 24,
        "wrapped_cmk_hex": 48,
        "signature_hex": 64,
        "expected_opened_cmk_hex": 32,
    }
    b64_fields = {
        "originator_id_b64",
        "receiver_id_b64",
        "hkdf_salt_b64",
        "hkdf_info_b64",
        "grant_aad_b64",
        "signature_input_b64",
        "d18g_b64",
    }
    for name, item in positives.items():
        _exact_keys(item, positive_fields, f"positive case {name}")
        for field, length in hex_lengths.items():
            _hex(item[field], length, f"{name}.{field}")
        for field in b64_fields:
            decoded = _strict_base64(item[field], f"{name}.{field}")
            if field == "d18g_b64" and not decoded.startswith(b"D18G\x01"):
                raise D18QConformanceError(f"{name}: invalid D18G prefix")
        epoch = str(item["key_epoch"])
        if not re.fullmatch(r"0|[1-9][0-9]*", epoch) or int(epoch) > 2**64 - 1:
            raise D18QConformanceError(f"{name}: invalid key epoch")

    structural = _validate_named_errors(
        document["structural_negative_cases"],
        EXPECTED_STRUCTURAL_NEGATIVES,
        {"name", "d18g_b64", "expected_error"},
        "structural negatives",
    )
    for name, item in structural.items():
        _strict_base64(item["d18g_b64"], name)
    recipe = _exact_keys(
        document["truncated_prefix_recipe"],
        {"source_case", "first_length", "last_length_exclusive", "expected_error"},
        "truncated-prefix recipe",
    )
    if recipe != {
        "source_case": "epoch-zero-receiver-a",
        "first_length": "0",
        "last_length_exclusive": "215",
        "expected_error": "truncated_record",
    }:
        raise D18QConformanceError("truncated-prefix recipe drifted")

    recipes = document["oversize_recipes"]
    expected_recipes = {
        ("originator_id", "5", "4097", "length_limit_exceeded"),
        ("receiver_id", "16", "4097", "length_limit_exceeded"),
    }
    if not isinstance(recipes, list):
        raise D18QConformanceError("oversize recipes must be an array")
    observed_recipes = set()
    for index, value in enumerate(recipes):
        item = _exact_keys(
            value,
            {"field", "length_offset", "declared_length", "expected_error"},
            f"oversize recipe {index}",
        )
        observed_recipes.add(
            (
                str(item["field"]),
                str(item["length_offset"]),
                str(item["declared_length"]),
                str(item["expected_error"]),
            )
        )
    if observed_recipes != expected_recipes or len(recipes) != len(expected_recipes):
        raise D18QConformanceError("oversize recipe roster drifted")

    _validate_named_errors(
        document["field_negative_cases"],
        EXPECTED_FIELD_NEGATIVES,
        {"name", "expected_error"},
        "field negatives",
    )
    _validate_named_errors(
        document["seal_negative_cases"],
        EXPECTED_SEAL_NEGATIVES,
        {"name", "expected_error"},
        "seal negatives",
    )
    opening = _validate_named_errors(
        document["opening_negative_cases"],
        EXPECTED_OPENING_NEGATIVES,
        {
            "name",
            "d18g_b64",
            "expected_originator_id_b64",
            "expected_receiver_id_b64",
            "expected_channel_id_hex",
            "receiver_private_key_hex",
            "expected_error",
        },
        "opening negatives",
    )
    for name, item in opening.items():
        if not _strict_base64(item["d18g_b64"], name).startswith(b"D18G\x01"):
            raise D18QConformanceError(f"{name}: invalid D18G prefix")
        _strict_base64(item["expected_originator_id_b64"], f"{name}.originator")
        _strict_base64(item["expected_receiver_id_b64"], f"{name}.receiver")
        _hex(item["expected_channel_id_hex"], 16, f"{name}.channel")
        _hex(item["receiver_private_key_hex"], 32, f"{name}.private key")

    trace = _exact_keys(
        document["receiver_state_trace"],
        {"grants", "steps", "missing_epoch", "missing_epoch_error"},
        "receiver-state trace",
    )
    grants = _exact_keys(
        trace["grants"],
        {
            "epoch_zero_b64",
            "same_epoch_conflict_b64",
            "failed_higher_epoch_b64",
            "skipped_epoch_three_b64",
        },
        "receiver-state grants",
    )
    for name, value in grants.items():
        if not _strict_base64(value, name).startswith(b"D18G\x01"):
            raise D18QConformanceError(f"{name}: invalid D18G prefix")
    steps = _named_items(trace["steps"], "receiver-state steps")
    observed_steps = {}
    for name, item in steps.items():
        _exact_keys(
            item,
            {"name", "grant", "expected", "latest_epoch", "retained_epochs"},
            f"receiver-state step {name}",
        )
        observed_steps[name] = (
            str(item["grant"]),
            str(item["expected"]),
            str(item["latest_epoch"]),
            tuple(str(epoch) for epoch in item["retained_epochs"]),
        )
    if observed_steps != EXPECTED_TRACE_STEPS:
        raise D18QConformanceError("receiver-state transition trace drifted")
    if (
        trace["missing_epoch"] != "1"
        or trace["missing_epoch_error"] != "missing_epoch_key"
    ):
        raise D18QConformanceError("receiver-state missing-epoch semantics drifted")

    rotation = _exact_keys(
        document["rotation_case"],
        {
            "name",
            "current_epoch",
            "new_epoch",
            "new_cmk_hex",
            "authorized_receiver_ids_b64",
            "new_grants_b64",
            "receiver_a_retains_epochs",
            "receiver_b_retains_epochs",
            "receiver_a_new_grant",
        },
        "rotation case",
    )
    if (
        rotation["name"] != "receivers-a-plus-b-to-b-only"
        or rotation["current_epoch"] != "0"
        or rotation["new_epoch"] != "1"
        or rotation["receiver_a_retains_epochs"] != ["0"]
        or rotation["receiver_b_retains_epochs"] != ["0", "1"]
        or rotation["receiver_a_new_grant"] is not None
        or len(rotation["authorized_receiver_ids_b64"]) != 1
        or len(rotation["new_grants_b64"]) != 1
    ):
        raise D18QConformanceError("rotation fixture semantics drifted")
    _hex(rotation["new_cmk_hex"], 32, "rotation CMK")
    _strict_base64(rotation["authorized_receiver_ids_b64"][0], "rotation receiver")
    if not _strict_base64(rotation["new_grants_b64"][0], "rotation grant").startswith(
        b"D18G\x01"
    ):
        raise D18QConformanceError("rotation grant has an invalid D18G prefix")


def git_blob_sha1(data: bytes) -> str:
    header = f"blob {len(data)}\0".encode("ascii")
    return hashlib.sha1(header + data, usedforsecurity=False).hexdigest()


def validate_lane_roster(lanes: tuple[Lane, ...] = LANES) -> None:
    lane_ids = [lane.lane_id for lane in lanes]
    if set(lane_ids) != EXPECTED_LANE_IDS or len(lane_ids) != len(EXPECTED_LANE_IDS):
        raise D18QConformanceError("language lanes must be exactly the supported six")
    package_roots = [lane.package_root for lane in lanes]
    consumer_tests = [lane.consumer_test for lane in lanes]
    if len(package_roots) != len(set(package_roots)) or len(consumer_tests) != len(
        set(consumer_tests)
    ):
        raise D18QConformanceError("language lane paths must be unique")


def validate_repository(root: Path = REPO_ROOT) -> dict[str, Any]:
    validate_lane_roster()
    _, document = load_manifest(root / FIXTURE_PATH.relative_to(REPO_ROOT))
    validate_manifest(document)
    generator_path = root / GENERATOR_PATH.relative_to(REPO_ROOT)
    try:
        generator_hash = git_blob_sha1(generator_path.read_bytes())
    except OSError as error:
        raise D18QConformanceError(f"cannot read fixture generator: {error}") from error
    if generator_hash != document["generator_blob_sha1"]:
        raise D18QConformanceError(
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
            raise D18QConformanceError(
                f"{lane.lane_id}: package, BUILD, or consumer is missing"
            )
        try:
            consumer = consumer_path.read_text(encoding="utf-8")
        except (OSError, UnicodeError) as error:
            raise D18QConformanceError(
                f"{lane.lane_id}: cannot read fixture consumer: {error}"
            ) from error
        missing = [marker for marker in CONSUMER_MARKERS if marker not in consumer]
        if missing:
            raise D18QConformanceError(
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
        raise D18QConformanceError(
            f"{context}: command could not complete: {error}"
        ) from error
    output = result.stdout + result.stderr
    if len(output) > MAXIMUM_COMMAND_OUTPUT_BYTES:
        raise D18QConformanceError(
            f"{context}: command output exceeded the safety limit"
        )
    if output:
        print(output.decode("utf-8", errors="replace"), end="")
    if result.returncode != 0:
        raise D18QConformanceError(
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
        raise D18QConformanceError(
            f"{lane.lane_id}: cannot read package BUILD: {error}"
        ) from error
    if not commands:
        raise D18QConformanceError(f"{lane.lane_id}: package BUILD has no commands")
    for index, command in enumerate(commands, start=1):
        _run(
            ["bash", "-c", command],
            package_root,
            f"D18Q {lane.lane_id} lane command {index}/{len(commands)}",
        )


def verify_regeneration(document: dict[str, Any], root: Path = REPO_ROOT) -> None:
    with tempfile.TemporaryDirectory(
        prefix="d18q-channel-key-grant-conformance-"
    ) as directory:
        generated = Path(directory) / "manifest.json"
        command = [
            "cargo",
            "run",
            "--quiet",
            "--manifest-path",
            "code/packages/rust/Cargo.toml",
            "-p",
            "chief-of-staff-channel-crypto",
            "--example",
            "generate_d18q_fixtures",
            "--",
            str(generated),
            document["generator_blob_sha1"],
        ]
        _run(command, root, "D18Q fixture regeneration")
        if (
            generated.read_bytes()
            != (root / FIXTURE_PATH.relative_to(REPO_ROOT)).read_bytes()
        ):
            raise D18QConformanceError("generated D18Q manifest is stale")


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
            "validated the D18Q manifest, generator provenance, and six consumer registrations"
        )
        if arguments.check_only:
            return 0
        selected = set(arguments.lane or EXPECTED_LANE_IDS)
        for lane in LANES:
            if lane.lane_id in selected:
                run_lane(lane)
        if "rust" in selected:
            verify_regeneration(document)
        print("D18Q six-language channel-key grant conformance passed")
        return 0
    except D18QConformanceError as error:
        print(f"D18Q channel-key grant conformance failed: {error}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
