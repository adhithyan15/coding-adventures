from __future__ import annotations

import importlib
import json
import subprocess
import sys
import time
from copy import deepcopy
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "code" / "scripts"))

validator = importlib.import_module("validate_cross_language_fixture_consumers")
DEFAULT_FIXTURE_ROOT = validator.DEFAULT_FIXTURE_ROOT
EXPECTED_LANES = validator.EXPECTED_LANES
ConsumerValidationError = validator.ConsumerValidationError
_bounded_subprocess_run = validator._bounded_subprocess_run
execute_consumers = validator.execute_consumers
load_json = validator.load_json
validate_catalog_document = validator.validate_catalog_document
validate_fixture_root = validator.validate_fixture_root


def catalog_document() -> dict[str, object]:
    return load_json(DEFAULT_FIXTURE_ROOT / "catalog.json")


def receipt(lane_id: str) -> str:
    return json.dumps(
        {
            "schema_version": 1,
            "lane_id": lane_id,
            "fixture_id": "weighted-neuron-forward",
            "row": "worked example",
            "contributions": [1.0, 0.25],
            "bias": 0.1,
            "preactivation": 1.35,
            "prediction": [1.35],
            "maximum_absolute_error": 0.0,
            "passes": True,
        }
    )


def lane_from_command(arguments: list[str]) -> str:
    return {"go": "go-native", "ruby": "ruby-native", "cargo": "rust-native"}[
        arguments[0]
    ]


def test_catalog_matches_language_neutral_schema() -> None:
    schema = load_json(DEFAULT_FIXTURE_ROOT / "schema.json")
    errors = sorted(
        Draft202012Validator(schema).iter_errors(catalog_document()),
        key=lambda error: list(error.path),
    )

    assert errors == []


def test_schema_rejects_cross_wired_or_injected_lane() -> None:
    schema = load_json(DEFAULT_FIXTURE_ROOT / "schema.json")
    document = deepcopy(catalog_document())
    document["lanes"][0] = {
        "id": "go-native",
        "language": "Rust",
        "family": "systems-native",
        "execution": "native",
        "working_directory": "../../outside",
        "command": ["sh", "-c", "echo injected", "x"],
        "source": "code/programs/go/neural-fixture-consumer/main.go/extra",
    }

    assert list(Draft202012Validator(schema).iter_errors(document))


def test_hand_check_recomputes_every_contribution() -> None:
    hand = validate_fixture_root()["hand_check"]

    assert hand["contributions"] == [
        hand["input"][index] * hand["weights"][index] for index in range(2)
    ]
    assert hand["preactivation"] == sum(hand["contributions"], hand["bias"])
    assert hand["prediction"] == 1.35


def test_execute_consumers_runs_three_real_native_lanes() -> None:
    results = execute_consumers(validate_fixture_root())

    assert [result.lane_id for result in results] == list(EXPECTED_LANES)
    assert all(result.maximum_absolute_error == 0 for result in results)


def test_execute_consumers_uses_fixed_argument_vectors_without_a_shell() -> None:
    calls: list[tuple[list[str], dict[str, object]]] = []

    def runner(
        arguments: list[str], **kwargs: object
    ) -> subprocess.CompletedProcess[str]:
        calls.append((arguments, kwargs))
        return subprocess.CompletedProcess(
            arguments, 0, receipt(lane_from_command(arguments)), ""
        )

    results = execute_consumers(validate_fixture_root(), runner=runner)

    assert len(results) == 3
    assert [arguments[0] for arguments, _ in calls] == ["go", "ruby", "cargo"]
    for arguments, kwargs in calls:
        assert arguments.count(str(validate_fixture_root()["source_fixture"])) == 1
        assert kwargs["check"] is False
        assert "shell" not in kwargs


def test_execute_consumers_propagates_nonzero_exit() -> None:
    def runner(
        arguments: list[str], **_: object
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(arguments, 9, "", "consumer broke")

    with pytest.raises(ConsumerValidationError, match="exit 9") as caught:
        execute_consumers(validate_fixture_root(), runner=runner)

    assert "consumer broke" not in str(caught.value)


def test_execute_consumers_rejects_successful_stderr() -> None:
    def runner(
        arguments: list[str], **_: object
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(
            arguments, 0, receipt(lane_from_command(arguments)), "warning"
        )

    with pytest.raises(ConsumerValidationError, match="wrote to stderr"):
        execute_consumers(validate_fixture_root(), runner=runner)


def test_execute_consumers_rejects_explanatory_stdout() -> None:
    def runner(
        arguments: list[str], **_: object
    ) -> subprocess.CompletedProcess[str]:
        output = f"result follows\n{receipt(lane_from_command(arguments))}"
        return subprocess.CompletedProcess(arguments, 0, output, "")

    with pytest.raises(ConsumerValidationError, match="not one JSON receipt"):
        execute_consumers(validate_fixture_root(), runner=runner)


def test_execute_consumers_rejects_dishonest_receipt() -> None:
    def runner(
        arguments: list[str], **_: object
    ) -> subprocess.CompletedProcess[str]:
        document = json.loads(receipt(lane_from_command(arguments)))
        document["preactivation"] = 99
        return subprocess.CompletedProcess(arguments, 0, json.dumps(document), "")

    with pytest.raises(ConsumerValidationError, match="trace is dishonest"):
        execute_consumers(validate_fixture_root(), runner=runner)


def test_execute_consumers_rejects_non_array_receipt_values() -> None:
    def runner(
        arguments: list[str], **_: object
    ) -> subprocess.CompletedProcess[str]:
        document = json.loads(receipt(lane_from_command(arguments)))
        document["contributions"] = 1
        return subprocess.CompletedProcess(arguments, 0, json.dumps(document), "")

    with pytest.raises(ConsumerValidationError, match="must contain two numbers"):
        execute_consumers(validate_fixture_root(), runner=runner)


def test_bounded_runner_terminates_pipe_holding_descendants(tmp_path: Path) -> None:
    started = tmp_path / "child-started"
    survived = tmp_path / "child-survived"
    child = (
        "import pathlib,sys,time; "
        "pathlib.Path(sys.argv[1]).write_text('started'); "
        "time.sleep(2); "
        "pathlib.Path(sys.argv[2]).write_text('survived'); "
        "time.sleep(30)"
    )
    parent = (
        "import subprocess,sys,time; "
        "subprocess.Popen([sys.executable, '-c', sys.argv[1], sys.argv[2], sys.argv[3]]); "
        "time.sleep(30)"
    )

    with pytest.raises(subprocess.TimeoutExpired):
        _bounded_subprocess_run(
            [sys.executable, "-c", parent, child, str(started), str(survived)],
            cwd=tmp_path,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="strict",
            timeout=1,
            check=False,
        )

    assert started.is_file()
    time.sleep(1.5)
    assert not survived.exists()


def test_catalog_rejects_unknown_key() -> None:
    document = catalog_document()
    document["surprise"] = True

    with pytest.raises(ConsumerValidationError, match="unexpected keys"):
        validate_catalog_document(document)


def test_catalog_rejects_path_traversal() -> None:
    document = deepcopy(catalog_document())
    document["source_fixture"] = "../outside.json"

    with pytest.raises(ConsumerValidationError, match="not canonical"):
        validate_catalog_document(document)


def test_catalog_rejects_dishonest_hand_arithmetic() -> None:
    document = deepcopy(catalog_document())
    document["hand_check"]["contributions"][1] = -0.25

    with pytest.raises(ConsumerValidationError, match="arithmetic is dishonest"):
        validate_catalog_document(document)


def test_catalog_rejects_changed_command() -> None:
    document = deepcopy(catalog_document())
    document["lanes"][0]["command"] = ["go", "run", "unsafe.go"]

    with pytest.raises(ConsumerValidationError, match="not canonical"):
        validate_catalog_document(document)


def test_catalog_rejects_reordered_lanes() -> None:
    document = deepcopy(catalog_document())
    document["lanes"][0], document["lanes"][1] = (
        document["lanes"][1],
        document["lanes"][0],
    )

    with pytest.raises(ConsumerValidationError, match="lane order"):
        validate_catalog_document(document)


def test_load_json_rejects_duplicate_keys(tmp_path: Path) -> None:
    path = tmp_path / "duplicate.json"
    path.write_text('{"id": 1, "id": 2}', encoding="utf-8")

    with pytest.raises(ConsumerValidationError, match="duplicate JSON key"):
        load_json(path)


def test_fixture_root_rejects_extra_file(tmp_path: Path) -> None:
    for name in ("catalog.json", "schema.json", "README.md", "CHANGELOG.md", "extra.txt"):
        (tmp_path / name).write_text("{}", encoding="utf-8")

    with pytest.raises(ConsumerValidationError, match="file roster"):
        validate_fixture_root(tmp_path)
