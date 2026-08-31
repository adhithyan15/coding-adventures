import json
import sys
from pathlib import Path

import pytest


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from taskapp_native_control_contract import CONTRACTS, validate  # noqa: E402


@pytest.mark.parametrize("backend", sorted(CONTRACTS))
def test_accepts_every_complete_backend_contract(tmp_path: Path, backend: str) -> None:
    (tmp_path / "mosaic-degradations.json").write_text(
        json.dumps({"nativeComplete": True, "degradations": []}),
        encoding="utf-8",
    )
    for relative_path, markers in CONTRACTS[backend].items():
        path = tmp_path / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("\n".join(markers), encoding="utf-8")

    assert validate(backend, tmp_path) == []


def test_rejects_disconnected_control_and_degraded_output(tmp_path: Path) -> None:
    backend = "swiftui"
    (tmp_path / "mosaic-degradations.json").write_text(
        json.dumps({"nativeComplete": False, "degradations": [{"code": "runtime.sample-fallback"}]}),
        encoding="utf-8",
    )
    for relative_path, markers in CONTRACTS[backend].items():
        path = tmp_path / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("\n".join(markers[1:]), encoding="utf-8")

    errors = validate(backend, tmp_path)

    assert any("nativeComplete is not true" in error for error in errors)
    assert any("degradations are not empty" in error for error in errors)
    assert any("name-input" in error for error in errors)
