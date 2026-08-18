#!/usr/bin/env python3
"""Validate and execute the six-language D18T durable epoch-activation gate.

D18T is the transaction that makes a rotated channel key *current*. Six
languages implement it against one Rust-authored manifest, and the whole point
of that manifest is that they agree byte for byte. This gate is what makes the
agreement enforced rather than asserted.

It is deliberately fail-closed at every step. A consumer that disappears, a
manifest key that gains a sibling, an error code that moves position, a
generator edited without regenerating its output -- each is a hard reject, not
a warning. The failure mode this exists to prevent is silent drift: five
languages staying honest while the sixth quietly stops checking something, with
CI still green.
"""

from __future__ import annotations

import argparse
import base64
import binascii
import hashlib
import json
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE_PATH = (
    REPO_ROOT / "code/fixtures/chief-of-staff-channel-epoch-activation/v1/manifest.json"
)
GENERATOR_PATH = (
    REPO_ROOT
    / "code/packages/rust/chief-of-staff-channel-epoch-activation"
    / "examples/generate_d18t_fixtures.rs"
)
SPEC_PATH = "code/specs/D18T-chief-of-staff-durable-epoch-activation-profile.md"

MAXIMUM_MANIFEST_BYTES = 2_000_000
MAXIMUM_COMMAND_OUTPUT_BYTES = 2_000_000
COMMAND_TIMEOUT_SECONDS = 1_800

CHANNEL_ID_HEX = "018f47a09b6c7def923456789abcdef0"

EXPECTED_TOP_LEVEL_KEYS = {
    "fixture_format",
    "spec",
    "generator_blob_sha1",
    "warning",
    "constants",
    "test_only_secrets",
    "state_migrations",
    "activation_case",
    "crash_replay_traces",
    "race_traces",
    "stable_error_codes",
    "negative_scenarios",
    "secret_erasure_capability",
}

EXPECTED_CONSTANTS = {
    "state_magic_ascii": "D18S",
    "state_version": "2",
    "plan_magic_ascii": "D18T",
    "plan_version": "1",
    "state_content_type": "application/vnd.coding-adventures.chief-channel-state-v2",
    "plan_content_type": (
        "application/vnd.coding-adventures.chief-channel-epoch-activation-v1"
    ),
    "max_cas_attempts": "16",
}

# Order is part of the contract, not just membership. Six languages expose this
# roster and the ports index it positionally, so a reordering is a breaking
# change that a set comparison would not catch.
EXPECTED_ERROR_CODES = (
    "not_initialized",
    "channel_destroyed",
    "invalid_plan",
    "corrupt_record",
    "pending_append",
    "unactivated_epoch",
    "active_key_missing",
    "conflicting_active_key",
    "preparation_missing",
    "conflicting_preparation",
    "conflicting_plan",
    "conflicting_grant",
    "unexpected_epoch",
    "decreasing_epoch",
    "epoch_exhausted",
    "concurrent_update",
    "storage_error",
    "custody_error",
    "crypto_error",
)

EXPECTED_SECRET_NAMES = {
    "current_cmk_hex",
    "ephemeral_private_key_hex",
    "next_cmk_hex",
    "originator_signing_seed_hex",
    "receiver_a_private_key_hex",
    "receiver_b_private_key_hex",
    "wrapping_nonce_hex",
}

EXPECTED_STATE_MIGRATIONS = ("no-pending", "pending-d18h")

# Traces are pinned by exact (operation, expected) pair rather than against a
# permitted vocabulary. A gate that merely accepted "some known outcome" would
# not notice a trace quietly changing what it asserts -- and these traces ARE
# the crash and concurrency contract, so a changed expectation is a changed
# protocol.
#
# Note that `activation-wins` expects a descriptive outcome rather than a
# stable error code, because the interesting assertion there is that the
# sequence survived the epoch advance, not that anything failed.
EXPECTED_CRASH_TRACES = {
    "after-custody-selection": ("replay-plan-and-all-grants", "prepared"),
    "after-plan-write": ("replay-all-grants", "prepared"),
    "after-first-grant": ("replay-remaining-grants", "prepared"),
    "after-all-grants": ("verify-and-activate", "activated"),
    "after-activation-cas": ("verify-exact-plan", "idempotent"),
}
EXPECTED_RACE_TRACES = {
    "publish-reservation-wins": ("activation", "pending_append"),
    "activation-wins": ("next-publish", "epoch-1-sequence-preserved"),
    "same-candidate-retry": ("custody-selection", "idempotent"),
    "different-candidate-loses": ("custody-selection", "conflicting_preparation"),
}
EXPECTED_NEGATIVE_SCENARIOS = {
    "pending-append": ("activation", "pending_append"),
    "corrupt-public-record": ("recovery", "corrupt_record"),
    "missing-custody": ("activation", "preparation_missing"),
    "destroyed-channel": ("activation", "channel_destroyed"),
    "epoch-exhaustion": ("preparation", "epoch_exhausted"),
    "sixteen-cas-conflicts": ("activation", "concurrent_update"),
}

# Markers every consumer must contain. These are the load-bearing sections of
# the profile: if a port stops mentioning one, it has almost certainly stopped
# testing it. Cheap to check, and it catches the "quietly deleted a test" case
# that a green suite would otherwise hide.
#
# This is a substring scan, so it cannot prove a consumer still *asserts*
# anything -- a file reduced to a docstring naming these would pass. It is a
# floor, not a ceiling: the lane's own BUILD still runs the real suite. Pinning
# the structural sections as well as the identity strings raises that floor to
# match the D18Q sibling gate, so gutting a consumer takes deliberate effort
# rather than a deletion.
CONSUMER_MARKERS = (
    "D18T-durable-epoch-activation-fixtures-v1",
    SPEC_PATH,
    "state_migrations",
    "activation_case",
    "crash_replay_traces",
    "race_traces",
    "stable_error_codes",
    "test_only_secrets",
)


class D18TConformanceError(ValueError):
    """Raised when the shared contract or one language lane is incomplete."""


@dataclass(frozen=True)
class Lane:
    """One package-native consumer of the shared D18T manifest."""

    lane_id: str
    package_root: str
    consumer_test: str


LANES = (
    Lane(
        "rust",
        "code/packages/rust/chief-of-staff-channel-epoch-activation",
        "code/packages/rust/chief-of-staff-channel-epoch-activation/tests/d18t_fixtures.rs",
    ),
    Lane(
        "typescript",
        "code/packages/typescript/chief-of-staff-channel-epoch-activation",
        "code/packages/typescript/chief-of-staff-channel-epoch-activation/tests/fixtures.test.ts",
    ),
    Lane(
        "python",
        "code/packages/python/chief-of-staff-channel-epoch-activation",
        "code/packages/python/chief-of-staff-channel-epoch-activation/tests/test_d18t_fixtures.py",
    ),
    Lane(
        "go",
        "code/packages/go/chief-of-staff-channel-epoch-activation",
        "code/packages/go/chief-of-staff-channel-epoch-activation/fixtures_test.go",
    ),
    Lane(
        "ruby",
        "code/packages/ruby/chief-of-staff-channel-epoch-activation",
        "code/packages/ruby/chief-of-staff-channel-epoch-activation/test/test_d18t_fixtures.rb",
    ),
    Lane(
        "elixir",
        "code/packages/elixir/chief-of-staff-channel-epoch-activation",
        "code/packages/elixir/chief-of-staff-channel-epoch-activation/test/d18t_fixtures_test.exs",
    ),
)
EXPECTED_LANE_IDS = {"rust", "typescript", "python", "go", "ruby", "elixir"}


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    """Refuse a JSON object with a repeated key.

    Python's json would silently keep the last value, so a manifest carrying
    two `stable_error_codes` entries would validate against whichever one came
    second. Rejecting outright means the six ports cannot disagree about which
    they read.
    """
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise D18TConformanceError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _reject_constant(token: str) -> None:
    raise D18TConformanceError(f"non-finite JSON number: {token}")


def load_manifest(path: Path = FIXTURE_PATH) -> tuple[bytes, dict[str, Any]]:
    try:
        raw = path.read_bytes()
    except OSError as error:
        raise D18TConformanceError(f"cannot read the D18T manifest: {error}") from error
    if len(raw) > MAXIMUM_MANIFEST_BYTES:
        raise D18TConformanceError("D18T manifest exceeds the safety limit")
    try:
        document = json.loads(
            raw.decode("utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=_reject_constant,
        )
    except (UnicodeError, ValueError) as error:
        raise D18TConformanceError(f"D18T manifest is not valid JSON: {error}") from error
    if not isinstance(document, dict):
        raise D18TConformanceError("D18T manifest must be a JSON object")
    return raw, document


def _exact_keys(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise D18TConformanceError(f"{context} must be an object")
    if set(value) != keys:
        missing = sorted(keys - set(value))
        extra = sorted(set(value) - keys)
        raise D18TConformanceError(
            f"{context} key roster drifted (missing: {missing}, unexpected: {extra})"
        )
    return value


def _strict_base64(value: Any, context: str) -> bytes:
    if not isinstance(value, str):
        raise D18TConformanceError(f"{context} must be a base64 string")
    try:
        return base64.b64decode(value, validate=True)
    except (binascii.Error, ValueError) as error:
        raise D18TConformanceError(f"{context} is not strict base64: {error}") from error


def _hex_bytes(value: Any, byte_length: int, context: str) -> bytes:
    if not isinstance(value, str) or len(value) != byte_length * 2:
        raise D18TConformanceError(f"{context} must be {byte_length} hex-encoded bytes")
    try:
        return bytes.fromhex(value)
    except ValueError as error:
        raise D18TConformanceError(f"{context} is not hexadecimal: {error}") from error


def _decimal_string(value: Any, context: str) -> int:
    if not isinstance(value, str) or not value.isdigit():
        raise D18TConformanceError(f"{context} must be a decimal string")
    parsed = int(value)
    if parsed < 0 or parsed > (1 << 64) - 1:
        raise D18TConformanceError(f"{context} must fit in an unsigned 64-bit integer")
    return parsed


def _named_sequence(
    value: Any, expected: tuple[str, ...], keys: set[str], context: str
) -> list[dict[str, Any]]:
    """Require an exact, ordered list of named entries with a closed key set."""
    if not isinstance(value, list):
        raise D18TConformanceError(f"{context} must be a list")
    names = []
    for index, item in enumerate(value):
        entry = _exact_keys(item, keys, f"{context}[{index}]")
        name = entry.get("name")
        if not isinstance(name, str) or not name:
            raise D18TConformanceError(f"{context}[{index}] must carry a name")
        names.append(name)
    if tuple(names) != expected:
        raise D18TConformanceError(
            f"{context} roster drifted: {names} (expected {list(expected)})"
        )
    if len(set(names)) != len(names):
        raise D18TConformanceError(f"{context} contains duplicate names")
    return value


def _validate_traces(
    document: dict[str, Any], key: str, expected: dict[str, tuple[str, str]]
) -> None:
    """Pin each trace's name, operation, and expected outcome exactly."""
    entries = _named_sequence(
        document[key], tuple(expected), {"name", "operation", "expected"}, key
    )
    for entry in entries:
        name = entry["name"]
        wanted_operation, wanted_outcome = expected[name]
        if entry["operation"] != wanted_operation:
            raise D18TConformanceError(
                f"{key}: {name} operation is {entry['operation']!r}, "
                f"expected {wanted_operation!r}"
            )
        if entry["expected"] != wanted_outcome:
            raise D18TConformanceError(
                f"{key}: {name} expects {entry['expected']!r}, "
                f"expected {wanted_outcome!r}"
            )


def validate_manifest(document: dict[str, Any]) -> None:
    _exact_keys(document, EXPECTED_TOP_LEVEL_KEYS, "D18T manifest")

    if document["fixture_format"] != "D18T-durable-epoch-activation-fixtures-v1":
        raise D18TConformanceError("unexpected D18T fixture format")
    if document["spec"] != SPEC_PATH:
        raise D18TConformanceError("D18T manifest does not point at its own spec")

    warning = document["warning"]
    if not isinstance(warning, str) or "Never log" not in warning:
        raise D18TConformanceError("D18T manifest is missing its secret-handling warning")

    generator_hash = document["generator_blob_sha1"]
    if not isinstance(generator_hash, str) or len(generator_hash) != 40:
        raise D18TConformanceError("generator_blob_sha1 must be a git blob SHA-1")
    try:
        bytes.fromhex(generator_hash)
    except ValueError as error:
        raise D18TConformanceError("generator_blob_sha1 is not hexadecimal") from error

    constants = _exact_keys(
        document["constants"], set(EXPECTED_CONSTANTS), "D18T constants"
    )
    for name, expected_value in EXPECTED_CONSTANTS.items():
        if constants[name] != expected_value:
            raise D18TConformanceError(
                f"constant {name} is {constants[name]!r}, expected {expected_value!r}"
            )

    codes = document["stable_error_codes"]
    if not isinstance(codes, list) or tuple(codes) != EXPECTED_ERROR_CODES:
        raise D18TConformanceError(
            "stable_error_codes must be exactly the D18T roster, in order"
        )

    if document["secret_erasure_capability"] != "guaranteed":
        raise D18TConformanceError(
            "the Rust reference must report guaranteed secret erasure"
        )

    _validate_secrets(document)
    _validate_state_migrations(document)
    _validate_activation_case(document)
    _validate_traces(document, "crash_replay_traces", EXPECTED_CRASH_TRACES)
    _validate_traces(document, "race_traces", EXPECTED_RACE_TRACES)
    _validate_traces(document, "negative_scenarios", EXPECTED_NEGATIVE_SCENARIOS)


def _validate_secrets(document: dict[str, Any]) -> None:
    """Every labelled secret must be well-formed and appear exactly once.

    A second occurrence would mean a test-only key leaked into a summary, a
    public record, or an expected-error string -- the exact thing the manifest's
    own warning tells readers not to do.
    """
    secrets = _exact_keys(
        document["test_only_secrets"], EXPECTED_SECRET_NAMES, "test_only_secrets"
    )
    lengths = {
        "current_cmk_hex": 32,
        "next_cmk_hex": 32,
        "originator_signing_seed_hex": 32,
        "receiver_a_private_key_hex": 32,
        "receiver_b_private_key_hex": 32,
        "ephemeral_private_key_hex": 32,
        "wrapping_nonce_hex": 24,
    }
    for name, byte_length in lengths.items():
        _hex_bytes(secrets[name], byte_length, f"test_only_secrets.{name}")

    serialized = json.dumps(document, sort_keys=True)
    for name, secret in secrets.items():
        if serialized.count(secret) != 1:
            raise D18TConformanceError(
                f"test-only secret {name} appears more than once in the manifest"
            )

    if len(set(secrets.values())) != len(secrets):
        raise D18TConformanceError("test-only secrets must be distinct")


def _validate_state_migrations(document: dict[str, Any]) -> None:
    entries = _named_sequence(
        document["state_migrations"],
        EXPECTED_STATE_MIGRATIONS,
        {"name", "d18s_v1_b64", "d18s_v2_b64", "active_epoch", "next_sequence"},
        "state_migrations",
    )
    for entry in entries:
        name = entry["name"]
        version_one = _strict_base64(entry["d18s_v1_b64"], f"{name}.d18s_v1_b64")
        version_two = _strict_base64(entry["d18s_v2_b64"], f"{name}.d18s_v2_b64")
        _decimal_string(entry["active_epoch"], f"{name}.active_epoch")
        _decimal_string(entry["next_sequence"], f"{name}.next_sequence")

        # The migration is only meaningful if the two records really are v1 and
        # v2 of the same format. Checking the magic and version bytes here means
        # a swapped or truncated vector fails the gate rather than six suites.
        if version_one[:5] != b"D18S\x01":
            raise D18TConformanceError(f"{name}: d18s_v1 is not a D18S version 1 record")
        if version_two[:5] != b"D18S\x02":
            raise D18TConformanceError(f"{name}: d18s_v2 is not a D18S version 2 record")
        if len(version_two) <= len(version_one):
            raise D18TConformanceError(
                f"{name}: the v2 record must carry the added active epoch"
            )


def _validate_activation_case(document: dict[str, Any]) -> None:
    activation = _exact_keys(
        document["activation_case"],
        {
            "name",
            "base_epoch",
            "new_epoch",
            "plan_record_key",
            "plan_content_type",
            "plan_b64",
            "grant_b64",
            "receiver_a_retains_epochs",
            "receiver_b_retains_epochs",
            "receiver_a_new_grant",
        },
        "activation_case",
    )
    base_epoch = _decimal_string(activation["base_epoch"], "activation_case.base_epoch")
    new_epoch = _decimal_string(activation["new_epoch"], "activation_case.new_epoch")
    if new_epoch != base_epoch + 1:
        raise D18TConformanceError("activation_case must advance exactly one epoch")

    if activation["plan_content_type"] != EXPECTED_CONSTANTS["plan_content_type"]:
        raise D18TConformanceError("activation_case plan content type drifted")

    expected_key = f"{CHANNEL_ID_HEX}/epochs/{new_epoch:020d}/activation"
    if activation["plan_record_key"] != expected_key:
        raise D18TConformanceError(
            f"activation_case plan record key is {activation['plan_record_key']!r}, "
            f"expected {expected_key!r}"
        )

    plan = _strict_base64(activation["plan_b64"], "activation_case.plan_b64")
    if plan[:5] != b"D18T\x01":
        raise D18TConformanceError("activation_case plan is not a D18T version 1 record")
    if plan[5:21].hex() != CHANNEL_ID_HEX:
        raise D18TConformanceError("activation_case plan names a different channel")

    grants = activation["grant_b64"]
    if not isinstance(grants, list) or not grants:
        raise D18TConformanceError("activation_case must carry at least one grant")
    decoded = [
        _strict_base64(grant, f"activation_case.grant_b64[{index}]")
        for index, grant in enumerate(grants)
    ]
    for index, grant in enumerate(decoded):
        if grant[:5] != b"D18G\x01":
            raise D18TConformanceError(
                f"activation_case.grant_b64[{index}] is not a D18G version 1 record"
            )
    if len(set(decoded)) != len(decoded):
        raise D18TConformanceError("activation_case grants must be distinct")

    # Prospective revocation, asserted as data rather than prose: the rotated-out
    # receiver gets no new grant and keeps only the old epoch, while the retained
    # receiver keeps both. If this ever inverts, revocation has silently become
    # retrospective.
    if activation["receiver_a_new_grant"] is not None:
        raise D18TConformanceError("the revoked receiver must receive no new grant")
    if activation["receiver_a_retains_epochs"] != [str(base_epoch)]:
        raise D18TConformanceError("the revoked receiver must retain only the old epoch")
    if activation["receiver_b_retains_epochs"] != [str(base_epoch), str(new_epoch)]:
        raise D18TConformanceError("the retained receiver must keep both epochs")


def git_blob_sha1(data: bytes) -> str:
    header = f"blob {len(data)}\0".encode("ascii")
    return hashlib.sha1(header + data, usedforsecurity=False).hexdigest()


def validate_lane_roster(lanes: tuple[Lane, ...] = LANES) -> None:
    """Exactly six lanes, no duplicates, no substitutions.

    This is what makes the gate reject a *missing* consumer rather than quietly
    passing with five. A seventh language cannot be added without amending the
    spec, which is deliberate.
    """
    lane_ids = [lane.lane_id for lane in lanes]
    if set(lane_ids) != EXPECTED_LANE_IDS or len(lane_ids) != len(EXPECTED_LANE_IDS):
        raise D18TConformanceError("language lanes must be exactly the supported six")
    package_roots = [lane.package_root for lane in lanes]
    consumer_tests = [lane.consumer_test for lane in lanes]
    if len(package_roots) != len(set(package_roots)) or len(consumer_tests) != len(
        set(consumer_tests)
    ):
        raise D18TConformanceError("language lane paths must be unique")


def validate_repository(root: Path = REPO_ROOT) -> dict[str, Any]:
    validate_lane_roster()
    _, document = load_manifest(root / FIXTURE_PATH.relative_to(REPO_ROOT))
    validate_manifest(document)

    generator_path = root / GENERATOR_PATH.relative_to(REPO_ROOT)
    try:
        generator_hash = git_blob_sha1(generator_path.read_bytes())
    except OSError as error:
        raise D18TConformanceError(f"cannot read fixture generator: {error}") from error
    if generator_hash != document["generator_blob_sha1"]:
        raise D18TConformanceError(
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
            raise D18TConformanceError(
                f"{lane.lane_id}: package, BUILD, or consumer is missing"
            )
        try:
            consumer = consumer_path.read_text(encoding="utf-8")
        except (OSError, UnicodeError) as error:
            raise D18TConformanceError(
                f"{lane.lane_id}: cannot read fixture consumer: {error}"
            ) from error
        missing = [marker for marker in CONSUMER_MARKERS if marker not in consumer]
        if missing:
            raise D18TConformanceError(
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
        raise D18TConformanceError(
            f"{context}: command could not complete: {error}"
        ) from error
    output = result.stdout + result.stderr
    if len(output) > MAXIMUM_COMMAND_OUTPUT_BYTES:
        raise D18TConformanceError(f"{context}: command output exceeded the safety limit")
    if output:
        print(output.decode("utf-8", errors="replace"), end="")
    if result.returncode != 0:
        raise D18TConformanceError(
            f"{context}: command failed with exit code {result.returncode}"
        )


def run_lane(lane: Lane, root: Path = REPO_ROOT) -> None:
    """Execute a lane through its own BUILD file, line by line.

    Running the package's real front door -- rather than a command this script
    invents -- is what keeps the gate honest. If CI and the gate disagree about
    how a package is built, the gate is testing something nobody ships.
    """
    package_root = root / lane.package_root
    try:
        commands = [
            line.strip()
            for line in (package_root / "BUILD").read_text(encoding="utf-8").splitlines()
            if line.strip() and not line.lstrip().startswith("#")
        ]
    except (OSError, UnicodeError) as error:
        raise D18TConformanceError(
            f"{lane.lane_id}: cannot read package BUILD: {error}"
        ) from error
    if not commands:
        raise D18TConformanceError(f"{lane.lane_id}: package BUILD has no commands")
    for index, command in enumerate(commands, start=1):
        _run(
            ["bash", "-c", command],
            package_root,
            f"D18T {lane.lane_id} lane command {index}/{len(commands)}",
        )


def verify_regeneration(document: dict[str, Any], root: Path = REPO_ROOT) -> None:
    """Regenerate the manifest and require byte-for-byte equality.

    Provenance by hash proves the generator has not changed. This proves the
    generator still *produces* what is checked in -- the other half, and the one
    that catches a hand-edited manifest.
    """
    with tempfile.TemporaryDirectory(
        prefix="d18t-durable-epoch-activation-conformance-"
    ) as directory:
        generated = Path(directory) / "manifest.json"
        command = [
            "cargo",
            "run",
            "--quiet",
            "--manifest-path",
            "code/packages/rust/Cargo.toml",
            "-p",
            "chief-of-staff-channel-epoch-activation",
            "--example",
            "generate_d18t_fixtures",
            "--",
            str(generated),
            document["generator_blob_sha1"],
        ]
        _run(command, root, "D18T fixture regeneration")
        if (
            generated.read_bytes()
            != (root / FIXTURE_PATH.relative_to(REPO_ROOT)).read_bytes()
        ):
            raise D18TConformanceError("generated D18T manifest is stale")


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check-only",
        action="store_true",
        help=(
            "Validate the manifest, generator provenance, and six consumer "
            "registrations only."
        ),
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
            "validated the D18T manifest, generator provenance, and six consumer "
            "registrations"
        )
        if arguments.check_only:
            return 0
        selected = set(arguments.lane or EXPECTED_LANE_IDS)
        for lane in LANES:
            if lane.lane_id in selected:
                run_lane(lane)
        if "rust" in selected:
            verify_regeneration(document)
        print("D18T six-language durable epoch-activation conformance passed")
        return 0
    except D18TConformanceError as error:
        print(f"D18T durable epoch-activation conformance failed: {error}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
