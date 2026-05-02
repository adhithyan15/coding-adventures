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
    GoldenFixture(name="convergence", result=[39], stdout="CONVERGE 39"),
    GoldenFixture(name="standard-real-math", result=[478], stdout="MATH 478"),
    GoldenFixture(name="full-surface", result=[81], stdout="COMPLETE 81"),
    GoldenFixture(name="switch-actuals", result=[42], stdout="SWITCH 42"),
    GoldenFixture(name="typed-comparisons", result=[31], stdout="READY 31"),
    GoldenFixture(name="recursive-lexical", result=[125], stdout="RECURSE 125"),
    GoldenFixture(name="procedure-formal-closure", result=[25], stdout="FORMAL 25"),
    GoldenFixture(name="nonlocal-unwind", result=[42], stdout="UNWIND 42"),
    GoldenFixture(name="dynamic-array-bounds", result=[63], stdout="DYNAMIC 63"),
    GoldenFixture(name="by-name-mixed-scalars", result=[27], stdout="BYNAME 27"),
    GoldenFixture(name="runtime-guards", result=[0], stdout=""),
)


@pytest.mark.parametrize("fixture", _GOLDEN_FIXTURES, ids=lambda fixture: fixture.name)
def test_golden_algol_fixtures_execute_end_to_end(fixture: GoldenFixture) -> None:
    source = (_FIXTURE_DIR / f"{fixture.name}.alg").read_text()
    compiled = compile_source(source)
    captured: list[str] = []
    runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

    assert runtime.load_and_run(compiled.binary, "_start", []) == fixture.result
    assert "".join(captured) == fixture.stdout
