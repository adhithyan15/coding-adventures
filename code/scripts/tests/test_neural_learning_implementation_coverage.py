from __future__ import annotations

import importlib
import json
import shutil
import sys
from copy import deepcopy
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "code" / "scripts"))

validator = importlib.import_module("validate_neural_learning_implementation_coverage")
DEFAULT_FIXTURE_ROOT = validator.DEFAULT_FIXTURE_ROOT
ImplementationCoverageError = validator.ImplementationCoverageError
execute_coverage = validator.execute_coverage
load_json = validator.load_json
validate_catalog_document = validator.validate_catalog_document
validate_fixture_root = validator.validate_fixture_root


def catalog_document() -> dict[str, object]:
    return load_json(DEFAULT_FIXTURE_ROOT / "catalog.json")


def test_catalog_matches_language_neutral_schema() -> None:
    schema = load_json(DEFAULT_FIXTURE_ROOT / "schema.json")
    errors = sorted(
        Draft202012Validator(schema).iter_errors(catalog_document()),
        key=lambda error: list(error.path),
    )

    assert errors == []


def test_hand_check_recomputes_arithmetic_and_coverage_counts() -> None:
    hand = validate_fixture_root()["hand_check"]

    contributions = [
        hand["inputs"][index] * hand["weights"][index] for index in range(2)
    ]
    assert contributions == [1.0, 0.25]
    assert sum(contributions, hand["bias"]) == 1.35
    assert hand["native_implementations"] + hand["rust_core_bindings"] == 4


def test_executes_three_native_lanes_and_one_real_rust_core_binding() -> None:
    receipt = execute_coverage(validate_fixture_root())

    assert receipt.native_lane_ids == ("go-native", "ruby-native", "rust-native")
    assert receipt.binding_lane_ids == ("python-ctypes-rust-core",)
    assert receipt.prediction == 1.35
    assert receipt.total_verified_lanes == 4


def test_catalog_rejects_unknown_fields() -> None:
    document = catalog_document()
    document["surprise"] = True

    with pytest.raises(ImplementationCoverageError, match="unexpected shape"):
        validate_catalog_document(document)


def test_catalog_rejects_dishonest_coverage_count() -> None:
    document = deepcopy(catalog_document())
    document["hand_check"]["rust_core_bindings"] = 2

    with pytest.raises(ImplementationCoverageError, match="coverage count"):
        validate_catalog_document(document)


def test_catalog_rejects_booleans_in_integer_fields() -> None:
    boolean_version = deepcopy(catalog_document())
    boolean_version["schema_version"] = True

    with pytest.raises(ImplementationCoverageError, match="identity"):
        validate_catalog_document(boolean_version)

    boolean_count = deepcopy(catalog_document())
    boolean_count["hand_check"]["rust_core_bindings"] = True

    with pytest.raises(ImplementationCoverageError, match="coverage count"):
        validate_catalog_document(boolean_count)


def test_catalog_rejects_reclassified_lane() -> None:
    document = deepcopy(catalog_document())
    document["lanes"][0]["implementation"] = "rust-core-binding"

    with pytest.raises(ImplementationCoverageError, match="not canonical"):
        validate_catalog_document(document)


def test_fixture_root_rejects_extra_files(tmp_path: Path) -> None:
    for name in (
        "catalog.json",
        "schema.json",
        "README.md",
        "CHANGELOG.md",
        "extra.txt",
    ):
        (tmp_path / name).write_text("{}", encoding="utf-8")

    with pytest.raises(ImplementationCoverageError, match="file roster"):
        validate_fixture_root(tmp_path)


def test_fixture_root_rejects_schema_body_drift(tmp_path: Path) -> None:
    root = tmp_path / "fixture"
    shutil.copytree(DEFAULT_FIXTURE_ROOT, root)
    schema_path = root / "schema.json"
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    schema["title"] = "drifted"
    schema_path.write_text(json.dumps(schema), encoding="utf-8")

    with pytest.raises(ImplementationCoverageError, match="schema body"):
        validate_fixture_root(root)
