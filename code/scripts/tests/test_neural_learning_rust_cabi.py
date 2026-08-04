from __future__ import annotations

import importlib
import sys
from copy import deepcopy
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "code" / "scripts"))

validator = importlib.import_module("validate_neural_learning_rust_cabi")
CAbiValidationError = validator.CAbiValidationError
DEFAULT_FIXTURE_ROOT = validator.DEFAULT_FIXTURE_ROOT
execute_abi = validator.execute_abi
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


def test_catalog_recomputes_the_nn03_hand_trace() -> None:
    hand = validate_fixture_root()["hand_check"]

    contributions = [
        hand["inputs"][index] * hand["weights"][index] for index in range(2)
    ]
    assert contributions == [1.0, 0.25]
    assert sum(contributions, hand["bias"]) == 1.35


def test_compiled_dynamic_library_matches_success_and_failure_probes() -> None:
    receipt = execute_abi(validate_fixture_root())

    assert receipt.version == 0x0001_0000
    assert receipt.status == 0
    assert receipt.contributions == (1.0, 0.25)
    assert receipt.prediction == 1.35
    assert receipt.failure_statuses == (1, 2, 3, 5, 7)


def test_catalog_rejects_unknown_fields() -> None:
    document = catalog_document()
    document["surprise"] = True

    with pytest.raises(CAbiValidationError, match="unexpected shape"):
        validate_catalog_document(document)


def test_catalog_rejects_dishonest_arithmetic() -> None:
    document = deepcopy(catalog_document())
    document["hand_check"]["expected_contributions"][1] = -0.25

    with pytest.raises(CAbiValidationError, match="arithmetic is dishonest"):
        validate_catalog_document(document)


def test_catalog_rejects_status_reordering() -> None:
    document = deepcopy(catalog_document())
    document["statuses"][1], document["statuses"][2] = (
        document["statuses"][2],
        document["statuses"][1],
    )

    with pytest.raises(CAbiValidationError, match="status table"):
        validate_catalog_document(document)


def test_fixture_root_rejects_extra_files(tmp_path: Path) -> None:
    for name in ("catalog.json", "schema.json", "README.md", "CHANGELOG.md", "extra.txt"):
        (tmp_path / name).write_text("{}", encoding="utf-8")

    with pytest.raises(CAbiValidationError, match="file roster"):
        validate_fixture_root(tmp_path)
