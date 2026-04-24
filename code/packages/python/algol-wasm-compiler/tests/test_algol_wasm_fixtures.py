"""Golden end-to-end ALGOL programs executed through the local WASM runtime."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import pytest
from wasm_runtime import WasiConfig, WasiHost, WasmRuntime

from algol_wasm_compiler import compile_source


@dataclass(frozen=True)
class GoldenFixture:
    """Expected runtime behavior for a golden ALGOL program."""

    name: str
    result: list[int]
    stdout: str


_FIXTURE_DIR = Path(__file__).with_name("fixtures")
_GOLDEN_FIXTURES = (
    GoldenFixture(name="showcase", result=[12], stdout="ALGOL 2 7.000"),
    GoldenFixture(name="jensens-device", result=[30], stdout=""),
    GoldenFixture(name="control-flow", result=[7], stdout="FLOW 7"),
)


@pytest.mark.parametrize("fixture", _GOLDEN_FIXTURES, ids=lambda fixture: fixture.name)
def test_golden_algol_fixtures_execute_end_to_end(fixture: GoldenFixture) -> None:
    source = (_FIXTURE_DIR / f"{fixture.name}.alg").read_text()
    compiled = compile_source(source)
    captured: list[str] = []
    runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

    assert runtime.load_and_run(compiled.binary, "_start", []) == fixture.result
    assert "".join(captured) == fixture.stdout
