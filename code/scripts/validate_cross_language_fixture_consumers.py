#!/usr/bin/env python3
"""Validate and execute the NN34 Go, Ruby, and Rust fixture consumers."""

from __future__ import annotations

import json
import math
import subprocess
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any

from validate_reference_fixture_catalog import (
    _bounded_subprocess_run,
    _ValidatorOutputLimitExceeded,
)

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "cross-language-consumers-v1"
)
EXPECTED_FILES = {"catalog.json", "schema.json", "README.md", "CHANGELOG.md"}
MAXIMUM_FILE_BYTES = 1_000_000
MAXIMUM_OUTPUT_BYTES = 16_384
TIMEOUT_SECONDS = 60
RECEIPT_KEY_ORDER = (
    "schema_version",
    "lane_id",
    "fixture_id",
    "row",
    "contributions",
    "bias",
    "preactivation",
    "prediction",
    "maximum_absolute_error",
    "passes",
)
RECEIPT_KEYS = set(RECEIPT_KEY_ORDER)
EXPECTED_LANES = {
    "go-native": {
        "language": "Go",
        "family": "compiled-garbage-collected",
        "execution": "native",
        "working_directory": "code/programs/go/neural-fixture-consumer",
        "command": ["go", "run", ".", "--fixture", "{fixture}"],
        "source": "code/programs/go/neural-fixture-consumer/main.go",
    },
    "ruby-native": {
        "language": "Ruby",
        "family": "dynamic-interpreted",
        "execution": "native",
        "working_directory": ".",
        "command": [
            "ruby",
            "code/programs/ruby/neural-fixture-consumer/main.rb",
            "--fixture",
            "{fixture}",
        ],
        "source": "code/programs/ruby/neural-fixture-consumer/main.rb",
    },
    "rust-native": {
        "language": "Rust",
        "family": "systems-native",
        "execution": "native",
        "working_directory": ".",
        "command": [
            "cargo",
            "run",
            "--quiet",
            "--manifest-path",
            "code/programs/rust/neural-fixture-consumer/Cargo.toml",
            "--",
            "--fixture",
            "{fixture}",
        ],
        "source": "code/programs/rust/neural-fixture-consumer/src/main.rs",
    },
}
EXPECTED_LANE_ORDER = tuple(EXPECTED_LANES)


class ConsumerValidationError(ValueError):
    """Raised when the NN34 contract or one native receipt is invalid."""


@dataclass(frozen=True)
class ConsumerRun:
    """One verified native consumer receipt."""

    lane_id: str
    language: str
    maximum_absolute_error: float


Runner = Callable[..., subprocess.CompletedProcess[str]]


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ConsumerValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        size = path.stat().st_size
        if size <= 0 or size > MAXIMUM_FILE_BYTES:
            raise ConsumerValidationError(f"{path}: invalid file size")
        value = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda token: (_ for _ in ()).throw(
                ConsumerValidationError(f"non-finite JSON number: {token}")
            ),
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ConsumerValidationError(f"{path}: {error}") from error
    if not isinstance(value, dict):
        raise ConsumerValidationError(f"{path}: top-level value must be an object")
    return value


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ConsumerValidationError(f"{context}: expected an object")
    if set(value) != keys:
        raise ConsumerValidationError(f"{context}: unexpected keys")
    return value


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ConsumerValidationError(f"{context}: expected a number")
    number = float(value)
    if not math.isfinite(number):
        raise ConsumerValidationError(f"{context}: expected a finite number")
    return number


def _text(value: Any, context: str, maximum: int = 240) -> str:
    if (
        not isinstance(value, str)
        or not value.strip()
        or len(value) > maximum
        or any(ord(character) < 32 for character in value)
    ):
        raise ConsumerValidationError(f"{context}: invalid text")
    return value


def _repo_path(value: Any, allowed_root: str, context: str) -> Path:
    text = _text(value, context)
    if "\\" in text:
        raise ConsumerValidationError(f"{context}: use forward slashes")
    relative = PurePosixPath(text)
    if (
        relative.is_absolute()
        or relative.as_posix() != text
        or "." in relative.parts
        or ".." in relative.parts
    ):
        raise ConsumerValidationError(f"{context}: path is not normalized")
    resolved = (REPO_ROOT / Path(*relative.parts)).resolve()
    allowed = (REPO_ROOT / allowed_root).resolve()
    try:
        resolved.relative_to(allowed)
    except ValueError as error:
        raise ConsumerValidationError(f"{context}: path escapes {allowed_root}") from error
    if not resolved.is_file():
        raise ConsumerValidationError(f"{context}: file does not exist")
    return resolved


def validate_catalog_document(document: Any) -> dict[str, Any]:
    catalog = _object(
        document,
        {
            "schema_version",
            "id",
            "title",
            "question",
            "source_fixture",
            "protocol",
            "hand_check",
            "lanes",
        },
        "catalog",
    )
    if catalog["schema_version"] != 1 or catalog["id"] != "weighted-neuron-language-consumers":
        raise ConsumerValidationError("catalog identity is not canonical")
    _text(catalog["title"], "catalog.title")
    _text(catalog["question"], "catalog.question")
    if catalog["source_fixture"] != "code/specs/fixtures/neural-learning-v1/labs/00-weighted-neuron.json":
        raise ConsumerValidationError("source fixture is not canonical")
    source_fixture = _repo_path(catalog["source_fixture"], "code/specs/fixtures", "source_fixture")

    protocol = _object(
        catalog["protocol"],
        {"command", "success_exit_code", "steps", "receipt_keys"},
        "protocol",
    )
    if protocol["command"] != "python code/scripts/validate_cross_language_fixture_consumers.py" or protocol["success_exit_code"] != 0:
        raise ConsumerValidationError("protocol command is not canonical")
    if not isinstance(protocol["steps"], list) or len(protocol["steps"]) != 4:
        raise ConsumerValidationError("protocol must contain four steps")
    for index, step in enumerate(protocol["steps"]):
        _text(step, f"protocol.steps[{index}]")
    if not isinstance(protocol["receipt_keys"], list) or tuple(protocol["receipt_keys"]) != RECEIPT_KEY_ORDER:
        raise ConsumerValidationError("receipt key order is not canonical")

    hand = _object(
        catalog["hand_check"],
        {"input", "weights", "contributions", "bias", "preactivation", "activation", "prediction", "absolute_tolerance"},
        "hand_check",
    )
    if hand["input"] != [2.0, -1.0] or hand["weights"] != [0.5, -0.25] or hand["activation"] != "identity":
        raise ConsumerValidationError("hand-check operands are not canonical")
    contributions = [hand["input"][index] * hand["weights"][index] for index in range(2)]
    preactivation = sum(contributions, _number(hand["bias"], "hand_check.bias"))
    tolerance = _number(hand["absolute_tolerance"], "hand_check.absolute_tolerance")
    if hand["contributions"] != contributions or hand["preactivation"] != preactivation or hand["prediction"] != preactivation or tolerance <= 0:
        raise ConsumerValidationError("hand-check arithmetic is dishonest")

    lanes = catalog["lanes"]
    if not isinstance(lanes, list) or len(lanes) != len(EXPECTED_LANES):
        raise ConsumerValidationError("catalog must contain exactly three lanes")
    normalized_lanes: list[dict[str, Any]] = []
    seen: set[str] = set()
    lane_keys = {"id", "language", "family", "execution", "working_directory", "command", "source"}
    for index, raw_lane in enumerate(lanes):
        lane = _object(raw_lane, lane_keys, f"lanes[{index}]")
        lane_id = _text(lane["id"], f"lanes[{index}].id", 80)
        if lane_id in seen or lane_id not in EXPECTED_LANES:
            raise ConsumerValidationError(f"lanes[{index}]: unknown or duplicate lane")
        expected_lane = EXPECTED_LANES[lane_id]
        if {key: lane[key] for key in expected_lane} != expected_lane:
            raise ConsumerValidationError(f"lanes[{index}]: lane contract is not canonical")
        _repo_path(lane["source"], f"code/programs/{lane['language'].lower()}", f"lanes[{index}].source")
        cwd_text = lane["working_directory"]
        cwd = REPO_ROOT if cwd_text == "." else (REPO_ROOT / Path(*PurePosixPath(cwd_text).parts)).resolve()
        if not cwd.is_dir():
            raise ConsumerValidationError(f"lanes[{index}].working_directory: directory does not exist")
        command = lane["command"]
        if not isinstance(command, list) or command.count("{fixture}") != 1 or any(not isinstance(token, str) or not token for token in command):
            raise ConsumerValidationError(f"lanes[{index}].command: invalid argument vector")
        seen.add(lane_id)
        normalized_lanes.append(json.loads(json.dumps(lane)))
    if tuple(lane["id"] for lane in normalized_lanes) != EXPECTED_LANE_ORDER:
        raise ConsumerValidationError("lane order is not canonical")
    return {
        "schema_version": 1,
        "id": catalog["id"],
        "title": catalog["title"],
        "question": catalog["question"],
        "source_fixture": source_fixture,
        "protocol": json.loads(json.dumps(protocol)),
        "hand_check": json.loads(json.dumps(hand)),
        "lanes": normalized_lanes,
    }


def validate_fixture_root(root: Path = DEFAULT_FIXTURE_ROOT) -> dict[str, Any]:
    try:
        files = {path.relative_to(root).as_posix() for path in root.rglob("*") if path.is_file()}
    except OSError as error:
        raise ConsumerValidationError(f"cannot enumerate fixture root: {error}") from error
    if files != EXPECTED_FILES:
        raise ConsumerValidationError("fixture file roster is not canonical")
    schema = load_json(root / "schema.json")
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema" or schema.get("$id") != "https://coding-adventures.dev/schemas/cross-language-consumers-v1.json":
        raise ConsumerValidationError("schema identity is not canonical")
    return validate_catalog_document(load_json(root / "catalog.json"))


def _parse_receipt(output: str, lane: dict[str, Any], hand: dict[str, Any]) -> ConsumerRun:
    if not output or len(output.encode("utf-8")) > MAXIMUM_OUTPUT_BYTES:
        raise ConsumerValidationError(f"{lane['id']}: missing or oversized stdout")
    try:
        receipt = json.loads(
            output,
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda token: (_ for _ in ()).throw(
                ConsumerValidationError(f"non-finite receipt number: {token}")
            ),
        )
    except json.JSONDecodeError as error:
        raise ConsumerValidationError(f"{lane['id']}: stdout is not one JSON receipt: {error}") from error
    receipt = _object(receipt, RECEIPT_KEYS, f"{lane['id']}.receipt")
    if receipt["schema_version"] != 1 or receipt["lane_id"] != lane["id"] or receipt["fixture_id"] != "weighted-neuron-forward" or receipt["row"] != "worked example":
        raise ConsumerValidationError(f"{lane['id']}: receipt identity is dishonest")
    if not isinstance(receipt["contributions"], list) or len(receipt["contributions"]) != 2:
        raise ConsumerValidationError(f"{lane['id']}: receipt contributions must contain two numbers")
    if not isinstance(receipt["prediction"], list) or len(receipt["prediction"]) != 1:
        raise ConsumerValidationError(f"{lane['id']}: receipt prediction must contain one number")
    contributions = [
        _number(value, f"{lane['id']}.contribution")
        for value in receipt["contributions"]
    ]
    prediction = [
        _number(value, f"{lane['id']}.prediction")
        for value in receipt["prediction"]
    ]
    if contributions != hand["contributions"] or prediction != [hand["prediction"]]:
        raise ConsumerValidationError(f"{lane['id']}: receipt arithmetic disagrees with the hand check")
    if _number(receipt["bias"], f"{lane['id']}.bias") != hand["bias"] or _number(receipt["preactivation"], f"{lane['id']}.preactivation") != hand["preactivation"]:
        raise ConsumerValidationError(f"{lane['id']}: receipt trace is dishonest")
    error = abs(prediction[0] - hand["prediction"])
    reported_error = _number(receipt["maximum_absolute_error"], f"{lane['id']}.maximum_absolute_error")
    if reported_error != error or not isinstance(receipt["passes"], bool) or receipt["passes"] != (error <= hand["absolute_tolerance"]):
        raise ConsumerValidationError(f"{lane['id']}: receipt tolerance result is dishonest")
    if not receipt["passes"]:
        raise ConsumerValidationError(f"{lane['id']}: prediction failed tolerance")
    return ConsumerRun(lane["id"], lane["language"], error)


def execute_consumers(
    catalog: dict[str, Any], *, runner: Runner = _bounded_subprocess_run
) -> list[ConsumerRun]:
    results: list[ConsumerRun] = []
    fixture = str(catalog["source_fixture"])
    for lane in catalog["lanes"]:
        command = [fixture if token == "{fixture}" else token for token in lane["command"]]
        cwd = REPO_ROOT if lane["working_directory"] == "." else (REPO_ROOT / Path(*PurePosixPath(lane["working_directory"]).parts)).resolve()
        try:
            completed = runner(
                command,
                cwd=cwd,
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="strict",
                timeout=TIMEOUT_SECONDS,
                check=False,
            )
        except (OSError, UnicodeError, subprocess.TimeoutExpired, _ValidatorOutputLimitExceeded) as error:
            raise ConsumerValidationError(f"{lane['id']}: consumer could not complete: {error}") from error
        if completed.returncode != 0:
            stdout_bytes = len(completed.stdout.encode("utf-8"))
            stderr_bytes = len(completed.stderr.encode("utf-8"))
            raise ConsumerValidationError(
                f"{lane['id']}: consumer failed with exit {completed.returncode} "
                f"(stdout {stdout_bytes} bytes, stderr {stderr_bytes} bytes)"
            )
        if completed.stderr.strip():
            raise ConsumerValidationError(f"{lane['id']}: successful consumer wrote to stderr")
        results.append(_parse_receipt(completed.stdout.strip(), lane, catalog["hand_check"]))
    return results


def main() -> int:
    try:
        catalog = validate_fixture_root()
        results = execute_consumers(catalog)
    except ConsumerValidationError as error:
        raise SystemExit(f"cross-language fixture consumers invalid: {error}") from error
    for result in results:
        print(f"{result.lane_id}: {result.language} native receipt passed (max error {result.maximum_absolute_error:g})")
    print(f"validated {len(results)} native language-family consumers")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
