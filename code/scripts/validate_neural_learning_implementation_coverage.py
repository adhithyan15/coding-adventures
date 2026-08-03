#!/usr/bin/env python3
"""Validate NN36 native-versus-Rust-core coverage with real execution evidence."""

from __future__ import annotations

import hashlib
import json
import math
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any

from validate_cross_language_fixture_consumers import (
    ConsumerValidationError,
    execute_consumers,
)
from validate_cross_language_fixture_consumers import (
    validate_fixture_root as validate_native_fixture_root,
)
from validate_neural_learning_rust_cabi import (
    CAbiValidationError,
    execute_abi,
)
from validate_neural_learning_rust_cabi import (
    validate_fixture_root as validate_cabi_fixture_root,
)

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT
    / "code"
    / "specs"
    / "fixtures"
    / "neural-learning-implementation-coverage-v1"
)
EXPECTED_FILES = {"catalog.json", "schema.json", "README.md", "CHANGELOG.md"}
MAXIMUM_FILE_BYTES = 1_000_000
EXPECTED_SCHEMA_SHA256 = (
    "5b366838673ddcd0fad66dcf0560e97b79ce9f777b3fd1b4744dda44a984e86b"
)
EXPECTED_CONTRACTS = {
    "native_catalog": "code/specs/fixtures/cross-language-consumers-v1/catalog.json",
    "rust_c_abi_catalog": "code/specs/fixtures/neural-learning-rust-cabi-v1/catalog.json",
    "native_validator": "code/scripts/validate_cross_language_fixture_consumers.py",
    "binding_validator": "code/scripts/validate_neural_learning_rust_cabi.py",
    "coverage_validator": "code/scripts/validate_neural_learning_implementation_coverage.py",
}
EXPECTED_LANES = (
    {
        "id": "go-native",
        "language": "Go",
        "implementation": "native",
        "arithmetic_owner": "Go",
        "interface": "fixture JSON to Go arithmetic",
        "evidence": "code/programs/go/neural-fixture-consumer/main.go",
        "validator": EXPECTED_CONTRACTS["native_validator"],
    },
    {
        "id": "ruby-native",
        "language": "Ruby",
        "implementation": "native",
        "arithmetic_owner": "Ruby",
        "interface": "fixture JSON to Ruby arithmetic",
        "evidence": "code/programs/ruby/neural-fixture-consumer/main.rb",
        "validator": EXPECTED_CONTRACTS["native_validator"],
    },
    {
        "id": "rust-native",
        "language": "Rust",
        "implementation": "native",
        "arithmetic_owner": "Rust",
        "interface": "fixture JSON to Rust arithmetic",
        "evidence": "code/programs/rust/neural-fixture-consumer/src/main.rs",
        "validator": EXPECTED_CONTRACTS["native_validator"],
    },
    {
        "id": "python-ctypes-rust-core",
        "language": "Python",
        "implementation": "rust-core-binding",
        "arithmetic_owner": "Rust",
        "interface": "Python ctypes to versioned C ABI",
        "evidence": "code/scripts/validate_neural_learning_rust_cabi.py",
        "validator": EXPECTED_CONTRACTS["binding_validator"],
    },
)
EXPECTED_RULES = (
    "native means the language lane owns the weighted-neuron arithmetic",
    "rust-core-binding means the caller crosses the stable C ABI and Rust owns the arithmetic",
    "a registered lane counts as verified only after its executable validator passes",
    "coverage counts implementation paths, not code quality, speed, or curriculum mastery",
)


class ImplementationCoverageError(ValueError):
    """Raised when NN36 metadata or execution evidence is inconsistent."""


@dataclass(frozen=True)
class CoverageReceipt:
    """Counts derived only after all registered implementation paths execute."""

    native_lane_ids: tuple[str, ...]
    binding_lane_ids: tuple[str, ...]
    prediction: float
    total_verified_lanes: int


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ImplementationCoverageError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        size = path.stat().st_size
        if size <= 0 or size > MAXIMUM_FILE_BYTES:
            raise ImplementationCoverageError(f"{path}: invalid file size")
        value = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda token: (_ for _ in ()).throw(
                ImplementationCoverageError(f"non-finite JSON number: {token}")
            ),
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ImplementationCoverageError(f"{path}: {error}") from error
    if not isinstance(value, dict):
        raise ImplementationCoverageError(f"{path}: top-level value must be an object")
    return value


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        raise ImplementationCoverageError(f"{context}: unexpected shape")
    return value


def _repo_file(value: Any, expected: str, context: str) -> Path:
    if value != expected or not isinstance(value, str) or "\\" in value:
        raise ImplementationCoverageError(f"{context}: path is not canonical")
    relative = PurePosixPath(value)
    if relative.is_absolute() or "." in relative.parts or ".." in relative.parts:
        raise ImplementationCoverageError(f"{context}: path is not normalized")
    resolved = (REPO_ROOT / Path(*relative.parts)).resolve()
    try:
        resolved.relative_to(REPO_ROOT.resolve())
    except ValueError as error:
        raise ImplementationCoverageError(
            f"{context}: path escapes the repository"
        ) from error
    if not resolved.is_file():
        raise ImplementationCoverageError(f"{context}: file does not exist")
    return resolved


def _finite_number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ImplementationCoverageError(f"{context}: expected a number")
    result = float(value)
    if not math.isfinite(result):
        raise ImplementationCoverageError(f"{context}: expected a finite number")
    return result


def validate_catalog_document(document: Any) -> dict[str, Any]:
    catalog = _object(
        document,
        {
            "schema_version",
            "id",
            "title",
            "question",
            "source_fixture",
            "contracts",
            "hand_check",
            "lanes",
            "rules",
        },
        "catalog",
    )
    if (
        type(catalog["schema_version"]) is not int
        or catalog["schema_version"] != 1
        or catalog["id"] != "weighted-neuron-implementation-coverage"
    ):
        raise ImplementationCoverageError("catalog identity is not canonical")
    for key in ("title", "question"):
        value = catalog[key]
        if not isinstance(value, str) or not value.strip() or len(value) > 300:
            raise ImplementationCoverageError(f"catalog.{key}: invalid text")
    source_fixture = _repo_file(
        catalog["source_fixture"],
        "code/specs/fixtures/neural-learning-v1/labs/00-weighted-neuron.json",
        "source_fixture",
    )

    contracts = _object(catalog["contracts"], set(EXPECTED_CONTRACTS), "contracts")
    if contracts != EXPECTED_CONTRACTS:
        raise ImplementationCoverageError("contract paths are not canonical")
    contract_paths = {
        key: _repo_file(value, EXPECTED_CONTRACTS[key], f"contracts.{key}")
        for key, value in contracts.items()
    }

    hand = _object(
        catalog["hand_check"],
        {
            "inputs",
            "weights",
            "contributions",
            "bias",
            "prediction",
            "native_implementations",
            "rust_core_bindings",
            "total_verified_lanes",
        },
        "hand_check",
    )
    if hand["inputs"] != [2.0, -1.0] or hand["weights"] != [0.5, -0.25]:
        raise ImplementationCoverageError("hand-check operands are not canonical")
    contributions = [
        _finite_number(hand["inputs"][index], f"hand_check.inputs[{index}]")
        * _finite_number(hand["weights"][index], f"hand_check.weights[{index}]")
        for index in range(2)
    ]
    prediction = sum(contributions, _finite_number(hand["bias"], "hand_check.bias"))
    if hand["contributions"] != contributions or hand["prediction"] != prediction:
        raise ImplementationCoverageError("hand-check arithmetic is dishonest")
    expected_counts = {
        "native_implementations": 3,
        "rust_core_bindings": 1,
        "total_verified_lanes": 4,
    }
    if any(
        type(hand[key]) is not int or hand[key] != expected
        for key, expected in expected_counts.items()
    ) or (
        hand["native_implementations"] + hand["rust_core_bindings"]
        != hand["total_verified_lanes"]
    ):
        raise ImplementationCoverageError("hand-check coverage count is dishonest")

    if catalog["lanes"] != list(EXPECTED_LANES):
        raise ImplementationCoverageError("coverage lanes are not canonical")
    for index, lane in enumerate(catalog["lanes"]):
        _object(
            lane,
            {
                "id",
                "language",
                "implementation",
                "arithmetic_owner",
                "interface",
                "evidence",
                "validator",
            },
            f"lanes[{index}]",
        )
        _repo_file(
            lane["evidence"],
            EXPECTED_LANES[index]["evidence"],
            f"lanes[{index}].evidence",
        )
        _repo_file(
            lane["validator"],
            EXPECTED_LANES[index]["validator"],
            f"lanes[{index}].validator",
        )
    if catalog["rules"] != list(EXPECTED_RULES):
        raise ImplementationCoverageError("coverage rules are not canonical")

    source = load_json(source_fixture)
    if source.get("id") != "weighted-neuron-forward":
        raise ImplementationCoverageError("source fixture identity is not canonical")

    native_catalog = validate_native_fixture_root()
    if [lane["id"] for lane in native_catalog["lanes"]] != [
        lane["id"] for lane in EXPECTED_LANES[:3]
    ]:
        raise ImplementationCoverageError("NN34 native lanes drifted")
    cabi_catalog = validate_cabi_fixture_root()
    if cabi_catalog["abi"]["version_number"] != 0x0001_0000:
        raise ImplementationCoverageError("NN35 ABI version drifted")

    return {
        **catalog,
        "source_fixture": source_fixture,
        "contract_paths": contract_paths,
        "native_catalog": native_catalog,
        "cabi_catalog": cabi_catalog,
    }


def validate_fixture_root(root: Path = DEFAULT_FIXTURE_ROOT) -> dict[str, Any]:
    try:
        files = {
            path.relative_to(root).as_posix()
            for path in root.rglob("*")
            if path.is_file()
        }
    except OSError as error:
        raise ImplementationCoverageError(
            f"cannot enumerate fixture root: {error}"
        ) from error
    if files != EXPECTED_FILES:
        raise ImplementationCoverageError("fixture file roster is not canonical")
    schema_path = root / "schema.json"
    try:
        schema_digest = hashlib.sha256(schema_path.read_bytes()).hexdigest()
    except OSError as error:
        raise ImplementationCoverageError(f"cannot read schema: {error}") from error
    if schema_digest != EXPECTED_SCHEMA_SHA256:
        raise ImplementationCoverageError("schema body is not canonical")
    schema = load_json(schema_path)
    if (
        schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema"
        or schema.get("$id")
        != "https://coding-adventures.dev/schemas/neural-learning-implementation-coverage-v1.json"
    ):
        raise ImplementationCoverageError("schema identity is not canonical")
    return validate_catalog_document(load_json(root / "catalog.json"))


def execute_coverage(catalog: dict[str, Any]) -> CoverageReceipt:
    native_results = execute_consumers(catalog["native_catalog"])
    native_lane_ids = tuple(result.lane_id for result in native_results)
    expected_native_ids = tuple(lane["id"] for lane in EXPECTED_LANES[:3])
    if native_lane_ids != expected_native_ids:
        raise ImplementationCoverageError("native execution evidence is incomplete")

    binding_receipt = execute_abi(catalog["cabi_catalog"])
    if (
        binding_receipt.version != 0x0001_0000
        or binding_receipt.status != 0
        or binding_receipt.contributions != (1.0, 0.25)
        or binding_receipt.prediction != catalog["hand_check"]["prediction"]
    ):
        raise ImplementationCoverageError("Rust-core binding evidence is dishonest")

    binding_lane_ids = (EXPECTED_LANES[3]["id"],)
    total = len(native_lane_ids) + len(binding_lane_ids)
    if total != catalog["hand_check"]["total_verified_lanes"]:
        raise ImplementationCoverageError("executed coverage count is dishonest")
    return CoverageReceipt(
        native_lane_ids=native_lane_ids,
        binding_lane_ids=binding_lane_ids,
        prediction=binding_receipt.prediction,
        total_verified_lanes=total,
    )


def main() -> int:
    try:
        catalog = validate_fixture_root()
        receipt = execute_coverage(catalog)
    except (
        ImplementationCoverageError,
        ConsumerValidationError,
        CAbiValidationError,
    ) as error:
        raise SystemExit(
            f"neural-learning implementation coverage invalid: {error}"
        ) from error
    print(
        "NN36 implementation coverage passed: "
        f"{len(receipt.native_lane_ids)} native + "
        f"{len(receipt.binding_lane_ids)} Rust-core binding = "
        f"{receipt.total_verified_lanes} verified lanes; "
        f"prediction {receipt.prediction:g}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
