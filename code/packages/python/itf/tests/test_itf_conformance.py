from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

from itf import InvalidItfInputError, encode_itf, normalize_itf

REPO_ROOT = Path(__file__).resolve().parents[5]
CASES_PATH = REPO_ROOT / "code/specs/fixtures/barcode-symbologies-v1/cases.json"
CASES = [
    case
    for case in json.loads(CASES_PATH.read_text(encoding="utf-8"))["cases"]
    if case["symbology"] == "itf"
]


def _input(case: dict[str, object]) -> str:
    input_value = case["input"]
    assert isinstance(input_value, dict)
    if "text" in input_value:
        return str(input_value["text"])
    repeat = input_value["repeat"]
    assert isinstance(repeat, dict)
    return str(repeat["text"]) * int(repeat["count"])


def _modules(data: str) -> str:
    return "1010" + "".join(pair.binary_pattern for pair in encode_itf(data)) + "11101"


def _run_lengths(modules: str) -> list[int]:
    runs: list[int] = []
    previous: str | None = None
    for module in modules:
        if module != previous:
            runs.append(1)
            previous = module
        else:
            runs[-1] += 1
    return runs


def _sha256(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


@pytest.mark.parametrize("case", CASES, ids=lambda case: str(case["id"]))
def test_itf_v1_cases(case: dict[str, object]) -> None:
    data = _input(case)
    expected = case["expected"]
    assert isinstance(expected, dict)

    if "error" in expected:
        with pytest.raises(InvalidItfInputError) as captured:
            normalize_itf(data)
        assert captured.value.error_id == expected["error"]
        return

    normalized = normalize_itf(data)
    modules = _modules(data)
    runs = _run_lengths(modules)
    if "normalized" in expected:
        assert normalized == expected["normalized"]
        assert modules == expected["modules"]
        assert runs == expected["run_lengths"]
    else:
        assert _sha256(normalized) == expected["normalized_sha256"]
        assert len(modules) == expected["module_count"]
        assert _sha256(modules) == expected["module_sha256"]
        assert len(runs) == expected["run_count"]
        compact_runs = json.dumps(runs, separators=(",", ":"))
        assert _sha256(compact_runs) == expected["run_lengths_sha256"]
