from __future__ import annotations

import json
import subprocess
import sys
from copy import deepcopy
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "code" / "scripts"))

from validate_reference_fixture_catalog import (
    DEFAULT_FIXTURE_ROOT,
    EXPECTED_FAMILY_COUNT,
    EXPECTED_LAB_COUNT,
    MAX_VALIDATOR_OUTPUT_BYTES,
    ReferenceCatalogValidationError,
    _bounded_subprocess_run,
    _ValidatorOutputLimitExceeded,
    execute_catalog,
    load_json,
    validate_catalog_document,
    validate_fixture_root,
)


def catalog_document() -> dict[str, object]:
    return load_json(DEFAULT_FIXTURE_ROOT / "catalog.json")


def test_catalog_covers_every_reference_family_and_lab() -> None:
    catalog = validate_fixture_root()

    assert len(catalog["families"]) == EXPECTED_FAMILY_COUNT
    assert (
        sum(family["lab_count"] for family in catalog["families"]) == EXPECTED_LAB_COUNT
    )
    assert [family["order"] for family in catalog["families"]] == list(range(3, 33))


def test_catalog_matches_the_language_neutral_json_schema() -> None:
    schema = load_json(DEFAULT_FIXTURE_ROOT / "schema.json")
    errors = sorted(
        Draft202012Validator(schema).iter_errors(catalog_document()),
        key=lambda error: list(error.path),
    )

    assert errors == []


def test_hand_check_recomputes_binary64_error() -> None:
    catalog = validate_fixture_root()
    check = catalog["protocol"]["hand_check"]

    assert check["recomputed"] == 0.1 + 0.05
    assert check["absolute_error"] == abs(check["recomputed"] - check["stored"])
    assert check["absolute_error"] <= check["absolute_tolerance"]
    assert check["passes"] is True


def test_execute_catalog_runs_all_reference_validators() -> None:
    results = execute_catalog(validate_fixture_root())

    assert len(results) == EXPECTED_FAMILY_COUNT
    assert sum(result.lab_count for result in results) == EXPECTED_LAB_COUNT
    assert all(result.output.startswith("validated") for result in results)


def test_execute_catalog_uses_no_shell_and_fixed_python_processes() -> None:
    calls: list[tuple[list[str], dict[str, object]]] = []

    def runner(args: list[str], **kwargs: object) -> subprocess.CompletedProcess[str]:
        calls.append((args, kwargs))
        return subprocess.CompletedProcess(args, 0, "validated fake fixture\n", "")

    results = execute_catalog(validate_fixture_root(), runner=runner)

    assert len(calls) == EXPECTED_FAMILY_COUNT
    assert len(results) == EXPECTED_FAMILY_COUNT
    for args, kwargs in calls:
        assert args[0] == sys.executable
        assert len(args) == 2
        assert kwargs["cwd"] == REPO_ROOT
        assert kwargs["check"] is False
        assert "shell" not in kwargs


def test_execute_catalog_can_select_one_registered_family() -> None:
    def runner(args: list[str], **_: object) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(args, 0, "validated one fixture\n", "")

    results = execute_catalog(
        validate_fixture_root(), family_id="precision-residency", runner=runner
    )

    assert [(result.order, result.family_id) for result in results] == [
        (32, "precision-residency")
    ]


def test_execute_catalog_rejects_unknown_family() -> None:
    with pytest.raises(ReferenceCatalogValidationError, match="unknown family id"):
        execute_catalog(validate_fixture_root(), family_id="not-registered")


def test_execute_catalog_propagates_reference_failure() -> None:
    def runner(args: list[str], **_: object) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(args, 7, "", "stored value disagrees")

    with pytest.raises(ReferenceCatalogValidationError, match="exit 7"):
        execute_catalog(
            validate_fixture_root(), family_id="neural-learning", runner=runner
        )


def test_execute_catalog_rejects_silent_success() -> None:
    def runner(args: list[str], **_: object) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(args, 0, "", "")

    with pytest.raises(ReferenceCatalogValidationError, match="no evidence"):
        execute_catalog(
            validate_fixture_root(), family_id="neural-learning", runner=runner
        )


def test_execute_catalog_converts_timeout_to_validation_error() -> None:
    def runner(args: list[str], **_: object) -> subprocess.CompletedProcess[str]:
        raise subprocess.TimeoutExpired(args, 60)

    with pytest.raises(ReferenceCatalogValidationError, match="could not complete"):
        execute_catalog(
            validate_fixture_root(), family_id="neural-learning", runner=runner
        )


def test_bounded_runner_stops_live_output_overflow() -> None:
    command = [
        sys.executable,
        "-c",
        f"import sys; sys.stdout.write('x' * {MAX_VALIDATOR_OUTPUT_BYTES + 4096})",
    ]

    with pytest.raises(_ValidatorOutputLimitExceeded, match="output exceeded"):
        _bounded_subprocess_run(
            command,
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="strict",
            timeout=5,
            check=False,
        )


def test_catalog_rejects_unknown_top_level_key() -> None:
    document = catalog_document()
    document["surprise"] = True

    with pytest.raises(ReferenceCatalogValidationError, match="keys differ"):
        validate_catalog_document(document)


def test_catalog_rejects_path_traversal() -> None:
    document = deepcopy(catalog_document())
    document["families"][0]["spec"] = "../outside.md"

    with pytest.raises(ReferenceCatalogValidationError, match="not normalized"):
        validate_catalog_document(document)


def test_catalog_rejects_duplicate_mapping() -> None:
    document = deepcopy(catalog_document())
    document["families"][1]["id"] = document["families"][0]["id"]
    document["families"][1]["fixture_root"] = document["families"][0]["fixture_root"]

    with pytest.raises(
        ReferenceCatalogValidationError, match="duplicate family mapping"
    ):
        validate_catalog_document(document)


def test_catalog_rejects_wrong_lab_count() -> None:
    document = deepcopy(catalog_document())
    document["families"][0]["lab_count"] = 3

    with pytest.raises(ReferenceCatalogValidationError, match="lab count"):
        validate_catalog_document(document)


def test_catalog_rejects_dishonest_tolerance_result() -> None:
    document = deepcopy(catalog_document())
    document["protocol"]["hand_check"]["passes"] = False

    with pytest.raises(
        ReferenceCatalogValidationError, match="pass result is dishonest"
    ):
        validate_catalog_document(document)


def test_load_json_rejects_duplicate_keys(tmp_path: Path) -> None:
    path = tmp_path / "duplicate.json"
    path.write_text('{"id": 1, "id": 2}', encoding="utf-8")

    with pytest.raises(ReferenceCatalogValidationError, match="duplicate JSON key"):
        load_json(path)


def test_load_json_rejects_non_finite_number(tmp_path: Path) -> None:
    path = tmp_path / "non-finite.json"
    path.write_text('{"value": NaN}', encoding="utf-8")

    with pytest.raises(ReferenceCatalogValidationError, match="non-finite JSON number"):
        load_json(path)


def test_fixture_root_rejects_extra_file(tmp_path: Path) -> None:
    for name in (
        "catalog.json",
        "schema.json",
        "README.md",
        "CHANGELOG.md",
        "extra.txt",
    ):
        (tmp_path / name).write_text(json.dumps({}), encoding="utf-8")

    with pytest.raises(ReferenceCatalogValidationError, match="file roster"):
        validate_fixture_root(tmp_path)
