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

    def test_inner_block_writes_outer_result_through_frame(self) -> None:
        result = compile_source(
            "begin integer result; "
            "result := 1; "
            "begin integer inner; inner := 4; result := inner + result end "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [5]

    def test_shadowed_inner_variable_uses_nearest_frame(self) -> None:
        result = compile_source(
            "begin integer x, result; "
            "x := 2; "
            "begin integer x; x := 9; result := x end "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [9]

    def test_integer_value_procedure_returns_result_slot(self) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure inc(x); value x; integer x; "
            "begin inc := x + 1 end; "
            "result := inc(4) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [5]

    def test_void_procedure_statement_writes_outer_frame(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure setresult(x); value x; integer x; "
            "begin result := x end; "
            "setresult(6) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [6]

    def test_value_parameter_assignment_does_not_write_back(self) -> None:
        result = compile_source(
            "begin integer result, y; "
            "procedure bump(x); value x; integer x; "
            "begin x := x + 1; result := x end; "
            "y := 5; bump(y); result := result * 10 + y "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [65]

    def test_recursive_factorial_runs_with_fresh_frames(self) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure fact(n); value n; integer n; "
            "begin if n = 0 then fact := 1 else fact := n * fact(n - 1) end; "
            "result := fact(5) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [120]

    def test_runaway_recursion_hits_bounded_frame_stack(self) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure loop(n); value n; integer n; "
            "begin loop := loop(n + 1) end; "
            "result := loop(0) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [0]

    def test_integer_array_element_store_and_load(self) -> None:
        result = compile_source(
            "begin integer result; integer array a[1:3]; "
            "a[1] := 2; a[2] := 3; result := a[1] + a[2] "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [5]

    def test_multidimensional_integer_array_uses_row_major_offsets(self) -> None:
        result = compile_source(
            "begin integer result; integer array a[1:2, 1:3]; "
            "a[2, 3] := 11; result := a[2, 3] "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [11]

    def test_dynamic_array_bounds_are_evaluated_at_block_entry(self) -> None:
        result = compile_source(
            "begin integer result, lo, hi; "
            "lo := 2; hi := 4; "
            "begin integer array a[lo:hi]; a[3] := 8; result := a[3] end "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [8]

    def test_out_of_bounds_array_access_returns_zero(self) -> None:
        result = compile_source(
            "begin integer result; integer array a[1:2]; "
            "a[3] := 9; result := 1 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [0]

    def test_invalid_array_bounds_return_zero(self) -> None:
        result = compile_source(
            "begin integer result; integer array a[3:1]; result := 1 end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [0]
