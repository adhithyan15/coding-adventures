"""Tests for ALGOL 60 to WebAssembly packaging."""

from pathlib import Path

import pytest
from wasm_runtime import WasmRuntime

from algol_wasm_compiler import (
    AlgolWasmError,
    __version__,
    compile_source,
    pack_source,
    write_wasm_file,
)


class TestVersion:
    """Verify the package is importable and has a version."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


class TestAlgolWasmCompiler:
    """The package exposes the same compile/pack/write shape as sibling lanes."""

    def test_compile_source_produces_wasm_bytes(self) -> None:
        result = compile_source("begin integer result; result := 7 end")
        assert result.binary.startswith(b"\x00asm")
        assert result.typed.ok

    def test_pack_source_aliases_compile_source(self) -> None:
        result = pack_source("begin integer result; result := 1 + 2 * 3 end")
        assert len(result.binary) > 8

    def test_write_wasm_file(self, tmp_path: Path) -> None:
        out = tmp_path / "answer.wasm"
        result = write_wasm_file("begin integer result; result := 9 end", out)
        assert result.wasm_path == out
        assert out.read_bytes() == result.binary

    def test_type_error_reports_stage(self) -> None:
        with pytest.raises(AlgolWasmError) as raised:
            compile_source("begin integer result; result := false end")
        assert raised.value.stage == "type-check"

    def test_runtime_smoke_returns_result(self) -> None:
        result = compile_source(
            "begin integer result, i; "
            "result := 0; "
            "for i := 1 step 1 until 4 do result := result + i "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [10]
