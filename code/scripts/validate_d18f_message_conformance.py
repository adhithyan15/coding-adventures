#!/usr/bin/env python3
"""Validate and execute the six-language D18F message conformance gate."""

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
FIXTURE_PATH = REPO_ROOT / "code/fixtures/chief-of-staff-message/v1/manifest.json"
GENERATOR_PATH = (
    REPO_ROOT
    / "code/packages/rust/chief-of-staff-channel-crypto/examples/generate_d18f_fixtures.rs"
)
MAXIMUM_MANIFEST_BYTES = 2_000_000
MAXIMUM_COMMAND_OUTPUT_BYTES = 2_000_000
COMMAND_TIMEOUT_SECONDS = 1_200

EXPECTED_POSITIVE_CASES = {
    "empty",
    "utf8-text",
    "structured-json",
    "arbitrary-binary",
    "multipart-related",
    "rotated-key-epoch",
    "stream-chunk-zero",
    "stream-chunk-one-final",
}
EXPECTED_BINARY_CASES = {
    "invalid-magic": ("deserialize", "invalid_magic"),
    "unsupported-version": ("deserialize", "unsupported_version"),
    "truncated-record": ("deserialize", "truncated_record"),
    "trailing-byte": ("deserialize", "trailing_bytes"),
    "oversized-originator-length": ("deserialize", "length_limit_exceeded"),
    "invalid-content-type-utf8": ("deserialize", "invalid_utf8"),
    "invalid-message-uuid": ("verify", "invalid_field"),
    "authenticated-message-id": ("verify", "invalid_signature"),
    "authenticated-originator-id": ("verify", "invalid_signature"),
    "authenticated-channel-id": ("verify", "invalid_signature"),
    "authenticated-sequence": ("verify", "invalid_signature"),
    "invalid-mime": ("verify", "invalid_field"),
    "authenticated-content-type": ("verify", "invalid_signature"),
    "authenticated-timestamp": ("verify", "invalid_signature"),
    "missing-key-epoch": ("verify", "missing_epoch_key"),
    "authenticated-plaintext-hash": ("verify", "invalid_signature"),
    "ciphertext": ("verify", "authentication_failed"),
    "authentication-tag": ("verify", "authentication_failed"),
    "originator-signature": ("verify", "invalid_signature"),
    "plaintext-hash-mismatch": ("verify", "plaintext_hash_mismatch"),
}
EXPECTED_JSON_CASES = {
    "syntax": "invalid_json",
    "duplicate-key": "invalid_json",
    "unknown-key": "invalid_json",
    "missing-key": "invalid_json",
    "record-type": "invalid_magic",
    "wire-version": "unsupported_version",
    "wire-version-type": "invalid_json",
    "decimal-leading-zero": "invalid_field",
    "uppercase-uuid": "invalid_field",
    "base64-without-padding": "invalid_field",
    "uppercase-hash": "invalid_field",
}
EXPECTED_OVERSIZE_RECIPES = {
    ("originator-id", "4097", "length_limit_exceeded"),
    ("content-type", "1025", "length_limit_exceeded"),
    ("ciphertext", "67108865", "length_limit_exceeded"),
    ("json-input", "94371841", "length_limit_exceeded"),
}
CONSUMER_MARKERS = (
    "chief-of-staff-message",
    "manifest.json",
    "D18F-message-fixtures-v1",
    "positive_cases",
    "binary_negative_cases",
    "json_negative_cases",
    "oversize_recipes",
)


class D18FConformanceError(ValueError):
    """Raised when the shared contract or one language lane is incomplete."""


@dataclass(frozen=True)
class Lane:
    """One package-native consumer of the shared D18F manifest."""

    lane_id: str
    package_root: str
    consumer_test: str


LANES = (
    Lane(
        "rust",
        "code/packages/rust/chief-of-staff-channel-crypto",
        "code/packages/rust/chief-of-staff-channel-crypto/tests/d18f_fixtures.rs",
    ),
    Lane(
        "python",
        "code/packages/python/chief-of-staff-channel-crypto",
        "code/packages/python/chief-of-staff-channel-crypto/tests/test_d18f_fixtures.py",
    ),
    Lane(
        "typescript",
        "code/packages/typescript/chief-of-staff-channel-crypto",
        "code/packages/typescript/chief-of-staff-channel-crypto/tests/d18f-fixtures.test.ts",
    ),
    Lane(
        "go",
        "code/packages/go/chief-of-staff-channel-crypto",
        "code/packages/go/chief-of-staff-channel-crypto/message_test.go",
    ),
    Lane(
        "ruby",
        "code/packages/ruby/chief-of-staff-channel-crypto",
        "code/packages/ruby/chief-of-staff-channel-crypto/test/test_d18f_fixtures.rb",
    ),
    Lane(
        "elixir",
        "code/packages/elixir/chief-of-staff-channel-crypto",
        "code/packages/elixir/chief-of-staff-channel-crypto/test/d18f_fixtures_test.exs",
    ),
)
EXPECTED_LANE_IDS = {"rust", "python", "typescript", "go", "ruby", "elixir"}


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise D18FConformanceError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _reject_constant(token: str) -> None:
    raise D18FConformanceError(f"non-finite JSON number: {token}")


def load_manifest(path: Path = FIXTURE_PATH) -> tuple[bytes, dict[str, Any]]:
    try:
        raw = path.read_bytes()
    except OSError as error:
        raise D18FConformanceError(f"cannot read fixture manifest: {error}") from error
    if not raw or len(raw) > MAXIMUM_MANIFEST_BYTES:
        raise D18FConformanceError("fixture manifest has an invalid byte length")
    try:
        document = json.loads(
            raw,
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=_reject_constant,
        )
    except (UnicodeError, json.JSONDecodeError) as error:
        raise D18FConformanceError(
            f"fixture manifest is not strict JSON: {error}"
        ) from error
    if not isinstance(document, dict):
        raise D18FConformanceError("fixture manifest must be an object")
    return raw, document


def _exact_keys(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        raise D18FConformanceError(f"{context}: unexpected object fields")
    return value


def _strict_base64(value: Any, context: str) -> bytes:
    if not isinstance(value, str):
        raise D18FConformanceError(f"{context}: expected base64 text")
    try:
        decoded = base64.b64decode(value, validate=True)
    except (ValueError, binascii.Error) as error:
        raise D18FConformanceError(f"{context}: invalid base64") from error
    if base64.b64encode(decoded).decode("ascii") != value:
        raise D18FConformanceError(f"{context}: non-canonical base64")
    return decoded


def validate_manifest(document: dict[str, Any]) -> None:
    _exact_keys(
        document,
        {
            "fixture_format",
            "spec",
            "generator_blob_sha1",
            "warning",
            "keys",
            "positive_cases",
            "binary_negative_cases",
            "json_negative_cases",
            "oversize_recipes",
        },
        "manifest",
    )
    if document["fixture_format"] != "D18F-message-fixtures-v1":
        raise D18FConformanceError("fixture identity is not D18F v1")
    if document["spec"] != "code/specs/D18F-chief-of-staff-message-profile.md":
        raise D18FConformanceError("fixture spec path is not canonical")
    if not re.fullmatch(r"[0-9a-f]{40}", str(document["generator_blob_sha1"])):
        raise D18FConformanceError("generator blob identifier is invalid")
    if (
        not isinstance(document["warning"], str)
        or "test-only" not in document["warning"]
    ):
        raise D18FConformanceError("fixture secret-material warning is missing")

    keys = _exact_keys(
        document["keys"],
        {
            "originator_signing_seed_hex",
            "originator_public_key_hex",
            "channel_master_keys",
        },
        "keys",
    )
    for field in ("originator_signing_seed_hex", "originator_public_key_hex"):
        if not re.fullmatch(r"[0-9a-f]{64}", str(keys[field])):
            raise D18FConformanceError(f"keys.{field}: invalid 32-byte hex")
    epoch_keys = keys["channel_master_keys"]
    if not isinstance(epoch_keys, list) or len(epoch_keys) != 2:
        raise D18FConformanceError("keys.channel_master_keys: expected two epochs")
    epochs: set[str] = set()
    for index, value in enumerate(epoch_keys):
        item = _exact_keys(value, {"key_epoch", "key_hex"}, f"epoch key {index}")
        if item["key_epoch"] not in {"0", "7"}:
            raise D18FConformanceError("unexpected fixture key epoch")
        if not re.fullmatch(r"[0-9a-f]{64}", str(item["key_hex"])):
            raise D18FConformanceError("invalid fixture epoch key")
        epochs.add(item["key_epoch"])
    if epochs != {"0", "7"}:
        raise D18FConformanceError("fixture epochs must be exactly 0 and 7")

    positives = document["positive_cases"]
    if not isinstance(positives, list):
        raise D18FConformanceError("positive_cases must be an array")
    positive_names: set[str] = set()
    for index, value in enumerate(positives):
        item = _exact_keys(
            value,
            {
                "name",
                "plaintext_b64",
                "authenticated_header_b64",
                "d18m_b64",
                "canonical_json_b64",
            },
            f"positive case {index}",
        )
        positive_names.add(str(item["name"]))
        _strict_base64(item["plaintext_b64"], f"positive case {index} plaintext")
        _strict_base64(
            item["authenticated_header_b64"], f"positive case {index} header"
        )
        record = _strict_base64(item["d18m_b64"], f"positive case {index} record")
        if not record.startswith(b"D18M\x01"):
            raise D18FConformanceError(f"positive case {index}: invalid D18M prefix")
        canonical_json = _strict_base64(
            item["canonical_json_b64"], f"positive case {index} canonical JSON"
        )
        try:
            json_value = json.loads(
                canonical_json,
                object_pairs_hook=_reject_duplicate_pairs,
                parse_constant=_reject_constant,
            )
        except (UnicodeError, json.JSONDecodeError) as error:
            raise D18FConformanceError(
                f"positive case {index}: invalid canonical JSON"
            ) from error
        if not isinstance(json_value, dict) or json_value.get("record_type") != "D18M":
            raise D18FConformanceError(
                f"positive case {index}: invalid JSON record type"
            )
    if positive_names != EXPECTED_POSITIVE_CASES or len(positives) != len(
        EXPECTED_POSITIVE_CASES
    ):
        raise D18FConformanceError(
            "positive fixture roster is incomplete or duplicated"
        )

    binary_cases = document["binary_negative_cases"]
    if not isinstance(binary_cases, list):
        raise D18FConformanceError("binary_negative_cases must be an array")
    observed_binary: dict[str, tuple[str, str]] = {}
    for index, value in enumerate(binary_cases):
        item = _exact_keys(
            value,
            {"name", "phase", "d18m_b64", "expected_error"},
            f"binary negative case {index}",
        )
        name = str(item["name"])
        if name in observed_binary:
            raise D18FConformanceError(f"duplicated binary negative case: {name}")
        observed_binary[name] = (str(item["phase"]), str(item["expected_error"]))
        _strict_base64(item["d18m_b64"], f"binary negative case {index}")
    if observed_binary != EXPECTED_BINARY_CASES:
        raise D18FConformanceError(
            "binary negative fixture roster or semantics drifted"
        )

    json_cases = document["json_negative_cases"]
    if not isinstance(json_cases, list):
        raise D18FConformanceError("json_negative_cases must be an array")
    observed_json: dict[str, str] = {}
    for index, value in enumerate(json_cases):
        item = _exact_keys(
            value,
            {"name", "json_b64", "expected_error"},
            f"JSON negative case {index}",
        )
        name = str(item["name"])
        if name in observed_json:
            raise D18FConformanceError(f"duplicated JSON negative case: {name}")
        observed_json[name] = str(item["expected_error"])
        _strict_base64(item["json_b64"], f"JSON negative case {index}")
    if observed_json != EXPECTED_JSON_CASES:
        raise D18FConformanceError("JSON negative fixture roster or semantics drifted")

    recipes = document["oversize_recipes"]
    if not isinstance(recipes, list):
        raise D18FConformanceError("oversize_recipes must be an array")
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
        raise D18FConformanceError("oversize recipe roster drifted")


def git_blob_sha1(data: bytes) -> str:
    header = f"blob {len(data)}\0".encode("ascii")
    return hashlib.sha1(header + data, usedforsecurity=False).hexdigest()


def validate_lane_roster(lanes: tuple[Lane, ...] = LANES) -> None:
    lane_ids = [lane.lane_id for lane in lanes]
    if set(lane_ids) != EXPECTED_LANE_IDS or len(lane_ids) != len(EXPECTED_LANE_IDS):
        raise D18FConformanceError("language lanes must be exactly the supported six")
    package_roots = [lane.package_root for lane in lanes]
    consumer_tests = [lane.consumer_test for lane in lanes]
    if len(package_roots) != len(set(package_roots)) or len(consumer_tests) != len(
        set(consumer_tests)
    ):
        raise D18FConformanceError("language lane paths must be unique")


def validate_repository(root: Path = REPO_ROOT) -> dict[str, Any]:
    validate_lane_roster()
    _, document = load_manifest(root / FIXTURE_PATH.relative_to(REPO_ROOT))
    validate_manifest(document)

    generator_path = root / GENERATOR_PATH.relative_to(REPO_ROOT)
    try:
        generator_hash = git_blob_sha1(generator_path.read_bytes())
    except OSError as error:
        raise D18FConformanceError(f"cannot read fixture generator: {error}") from error
    if generator_hash != document["generator_blob_sha1"]:
        raise D18FConformanceError(
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
            raise D18FConformanceError(
                f"{lane.lane_id}: package, BUILD, or consumer is missing"
            )
        try:
            consumer = consumer_path.read_text(encoding="utf-8")
        except (OSError, UnicodeError) as error:
            raise D18FConformanceError(
                f"{lane.lane_id}: cannot read fixture consumer: {error}"
            ) from error
        missing = [marker for marker in CONSUMER_MARKERS if marker not in consumer]
        if missing:
            raise D18FConformanceError(
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
        raise D18FConformanceError(
            f"{context}: command could not complete: {error}"
        ) from error
    output = result.stdout + result.stderr
    if len(output) > MAXIMUM_COMMAND_OUTPUT_BYTES:
        raise D18FConformanceError(
            f"{context}: command output exceeded the safety limit"
        )
    if output:
        print(output.decode("utf-8", errors="replace"), end="")
    if result.returncode != 0:
        raise D18FConformanceError(
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
        raise D18FConformanceError(
            f"{lane.lane_id}: cannot read package BUILD: {error}"
        ) from error
    if not commands:
        raise D18FConformanceError(f"{lane.lane_id}: package BUILD has no commands")
    for index, command in enumerate(commands, start=1):
        _run(
            ["bash", "-c", command],
            package_root,
            f"D18F {lane.lane_id} lane command {index}/{len(commands)}",
        )


def verify_regeneration(document: dict[str, Any], root: Path = REPO_ROOT) -> None:
    with tempfile.TemporaryDirectory(prefix="d18f-message-conformance-") as directory:
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
            "generate_d18f_fixtures",
            "--",
            str(generated),
            document["generator_blob_sha1"],
        ]
        _run(command, root, "D18F fixture regeneration")
        if (
            generated.read_bytes()
            != (root / FIXTURE_PATH.relative_to(REPO_ROOT)).read_bytes()
        ):
            raise D18FConformanceError("generated D18F manifest is stale")


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
            "validated the D18F manifest, generator provenance, and six consumer registrations"
        )
        if arguments.check_only:
            return 0
        selected = set(arguments.lane or EXPECTED_LANE_IDS)
        for lane in LANES:
            if lane.lane_id in selected:
                run_lane(lane)
        if "rust" in selected:
            verify_regeneration(document)
        print("D18F six-language message conformance passed")
        return 0
    except D18FConformanceError as error:
        print(f"D18F message conformance failed: {error}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
