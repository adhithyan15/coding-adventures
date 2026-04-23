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

    def test_procedure_call_expression_by_name_runs_through_eval_thunk(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure inc(n); value n; integer n; "
            "begin inc := n + 1 end; "
            "integer procedure id(x); integer x; begin id := x end; "
            "result := id(inc(4)) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [5]

    def test_runtime_smoke_returns_result(self) -> None:
        result = compile_source(
            "begin integer result, i; "
            "result := 0; "
            "for i := 1 step 1 until 4 do result := result + i "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [10]

    def test_boolean_variable_assignment_drives_condition(self) -> None:
        result = compile_source(
            "begin integer result; "
            "boolean flag; "
            "flag := true; "
            "if flag then result := 7 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_boolean_variable_shadowing_uses_nearest_frame(self) -> None:
        result = compile_source(
            "begin integer result; "
            "boolean flag; "
            "flag := false; "
            "begin boolean flag; "
            "flag := true; "
            "if flag then result := 9 else result := 0 "
            "end; "
            "if flag then result := 1 else result := result "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [9]

    def test_forward_goto_skips_statements_until_label(self) -> None:
        result = compile_source(
            "begin integer result; "
            "result := 1; "
            "goto done; "
            "result := 99; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [1]

    def test_backward_goto_runs_local_loop(self) -> None:
        result = compile_source(
            "begin integer result, i; "
            "i := 0; "
            "loop: i := i + 1; "
            "if i < 4 then goto loop; "
            "result := i "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [4]

    def test_local_goto_strategy_preserves_procedure_calls(self) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure inc(x); value x; integer x; "
            "begin inc := x + 1 end; "
            "result := inc(4); "
            "goto done; "
            "result := 0; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [5]

    def test_direct_nonlocal_block_goto_exits_inner_frame(self) -> None:
        result = compile_source(
            "begin integer result; "
            "begin integer inner; goto done; inner := 99 end; "
            "result := 0; "
            "done: result := 7 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_nonlocal_block_goto_restores_frame_for_later_block(self) -> None:
        result = compile_source(
            "begin integer result; "
            "begin integer inner; goto done; inner := 99 end; "
            "done: begin integer later; later := 8; result := later end "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [8]

    def test_direct_nonlocal_block_goto_inside_procedure(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure escape; "
            "begin begin integer inner; goto done; inner := 99 end; "
            "done: result := 6 end; "
            "escape "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [6]

    def test_procedure_crossing_goto_unwinds_back_to_caller_label(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure escape; "
            "begin goto outerdone; result := 9 end; "
            "escape; "
            "result := 0; "
            "outerdone: result := 7 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_procedure_crossing_goto_propagates_through_intermediate_procedure(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure outer; "
            "begin "
            "procedure inner; begin goto done end; "
            "inner; "
            "result := 3; "
            "done: result := 8 "
            "end; "
            "outer "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [8]

    def test_conditional_designational_goto_selects_branch(self) -> None:
        result = compile_source(
            "begin integer result; "
            "goto if false then left else right; "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_switch_designational_goto_selects_entry(self) -> None:
        result = compile_source(
            "begin integer result, i; "
            "switch s := first, second; "
            "i := 2; goto s[i]; "
            "first: result := 1; goto done; "
            "second: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_switch_entry_can_be_conditional_designational(self) -> None:
        result = compile_source(
            "begin integer result, i; "
            "switch s := if i = 1 then first else second; "
            "i := 2; goto s[1]; "
            "first: result := 1; goto done; "
            "second: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_repeated_switch_designational_gotos_use_distinct_dispatch(self) -> None:
        result = compile_source(
            "begin integer result, i; "
            "switch s := first, second; "
            "i := 1; goto s[i]; "
            "first: i := 2; goto s[i]; "
            "second: result := 7 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

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

    def test_boolean_value_procedure_returns_boolean_result(self) -> None:
        result = compile_source(
            "begin integer result; "
            "boolean procedure negate(x); value x; boolean x; "
            "begin negate := not x end; "
            "if negate(false) then result := 7 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

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

    def test_scalar_by_name_parameter_assignment_writes_back(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure bump(x); integer x; begin x := x + 1 end; "
            "result := 4; bump(result) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [5]

    def test_boolean_by_name_parameter_assignment_writes_back(self) -> None:
        result = compile_source(
            "begin integer result; boolean flag; "
            "procedure settrue(x); boolean x; begin x := true end; "
            "flag := false; settrue(flag); "
            "if flag then result := 1 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [1]

    def test_scalar_by_name_parameter_reads_forwarded_pointer(self) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure id(x); integer x; begin id := x end; "
            "result := 12; result := id(result) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [12]

    def test_array_element_by_name_parameter_assignment_writes_back(self) -> None:
        result = compile_source(
            "begin integer result; integer array a[1:2]; "
            "procedure put(x); integer x; begin x := 7 end; "
            "put(a[1]); result := a[1] "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_array_element_by_name_relocates_on_each_access(self) -> None:
        result = compile_source(
            "begin integer result, i; integer array a[1:2]; "
            "procedure bump(x); integer x; "
            "begin x := x + 1; i := 2; x := x + 1 end; "
            "a[1] := 10; a[2] := 20; i := 1; "
            "bump(a[i]); result := a[1] * 100 + a[2] "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [1121]

    def test_read_only_array_element_by_name_relocates_on_each_read(self) -> None:
        result = compile_source(
            "begin integer result, i; integer array a[1:2]; "
            "integer procedure probe(x); integer x; "
            "begin probe := x; i := 2; probe := probe * 100 + x end; "
            "a[1] := 3; a[2] := 8; i := 1; "
            "result := probe(a[i]) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [308]

    def test_array_element_by_name_bounds_failure_propagates_from_eval(self) -> None:
        result = compile_source(
            "begin integer result, i; integer array a[1:2]; "
            "integer procedure id(x); integer x; begin id := x end; "
            "i := 3; result := id(a[i]) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [0]

    def test_array_read_expression_by_name_re_evaluates_on_each_read(self) -> None:
        result = compile_source(
            "begin integer result, i; integer array a[1:2]; "
            "integer procedure probe(x); integer x; "
            "begin probe := x; a[1] := 9; probe := probe * 100 + x end; "
            "a[1] := 3; i := 1; "
            "result := probe(a[i] + 1) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [410]

    def test_array_read_expression_by_name_bounds_failure_propagates(self) -> None:
        result = compile_source(
            "begin integer result, i; integer array a[1:2]; "
            "integer procedure id(x); integer x; begin id := x end; "
            "i := 3; result := id(a[i] + 1) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [0]

    def test_procedure_expression_by_name_re_evaluates_on_each_read(self) -> None:
        result = compile_source(
            "begin integer result, count; "
            "integer procedure next(n); value n; integer n; "
            "begin count := count + 1; next := n + count end; "
            "integer procedure probe(x); integer x; "
            "begin probe := x; probe := probe * 100 + x end; "
            "count := 0; result := probe(next(3)) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [405]

    def test_procedure_expression_by_name_can_allocate_nested_thunks(self) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure use(x); integer x; begin use := x end; "
            "integer procedure probe(x); integer x; "
            "begin probe := x; result := 10; probe := probe * 100 + x end; "
            "result := 3; result := probe(use(result + 1)) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [411]

    def test_procedure_expression_by_name_bounds_failure_propagates(self) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure bad(n); value n; integer n; "
            "begin integer array a[1:1]; bad := a[2] end; "
            "integer procedure id(x); integer x; begin id := x end; "
            "result := id(bad(1) + 1) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [0]

    def test_procedure_expression_by_name_reads_array_value_argument(self) -> None:
        result = compile_source(
            "begin integer result, i; integer array a[1:2]; "
            "integer procedure inc(n); value n; integer n; begin inc := n + 1 end; "
            "integer procedure id(x); integer x; begin id := x end; "
            "a[1] := 7; i := 1; result := id(inc(a[i])) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [8]

    def test_integer_by_name_acceptance_surface(self) -> None:
        result = compile_source(
            "begin integer result, s, i; integer array a[1:3]; "
            "procedure bump(x); integer x; begin x := x + 1 end; "
            "integer procedure probe(x); integer x; "
            "begin probe := x; s := s + 10; probe := probe * 100 + x end; "
            "integer procedure inc(n); value n; integer n; begin inc := n + 1 end; "
            "integer procedure sum(k, lo, hi, term); "
            "value lo, hi; integer k, lo, hi, term; "
            "begin sum := 0; for k := lo step 1 until hi do sum := sum + term end; "
            "a[1] := 2; a[2] := 3; a[3] := 5; "
            "s := 1; bump(s); result := s; "
            "i := 1; bump(a[i]); result := result * 10 + a[1]; "
            "s := 4; result := result * 1000 + probe(s + 1); "
            "result := result * 100 + inc(a[1]); "
            "result := result * 100 + sum(i, 1, 3, a[i] * i) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [235150424]

    def test_jensens_device_sums_array_element_expression(self) -> None:
        result = compile_source(
            "begin integer result, i; integer array a[1:3]; "
            "integer procedure sum(k, lo, hi, term); "
            "value lo, hi; integer k, lo, hi, term; "
            "begin sum := 0; for k := lo step 1 until hi do sum := sum + term end; "
            "a[1] := 2; a[2] := 3; a[3] := 5; "
            "result := sum(i, 1, 3, a[i]) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [10]

    def test_jensens_device_sums_array_expression(self) -> None:
        result = compile_source(
            "begin integer result, i; integer array a[1:3]; "
            "integer procedure sum(k, lo, hi, term); "
            "value lo, hi; integer k, lo, hi, term; "
            "begin sum := 0; for k := lo step 1 until hi do sum := sum + term end; "
            "a[1] := 2; a[2] := 3; a[3] := 5; "
            "result := sum(i, 1, 3, a[i] * i) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [23]

    def test_read_only_by_name_expression_re_evaluates_on_each_read(self) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure probe(x); integer x; "
            "begin probe := x; result := 10; probe := probe * 100 + x end; "
            "result := 3; result := probe(result + 1) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [411]

    def test_read_only_by_name_literal_expression_runs_through_eval_thunk(self) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure id(x); integer x; begin id := x end; "
            "result := id(7) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_nested_by_name_parameter_forwards_original_storage_pointer(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure outer(x); integer x; "
            "begin "
            "procedure inner(y); integer y; begin y := y + 1 end; "
            "inner(x) "
            "end; "
            "result := 4; outer(result) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [5]

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

    def test_array_failure_in_procedure_unwinds_before_caller_continues(self) -> None:
        result = compile_source(
            "begin integer result, i; "
            "integer procedure bad(n); value n; integer n; "
            "begin integer array a[1:2]; a[3] := n; bad := n end; "
            "result := 0; "
            "for i := 1 step 1 until 2500 do result := result + bad(i); "
            "begin integer x; x := 5; result := result + x end "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [5]

    def test_invalid_array_bounds_return_zero(self) -> None:
        result = compile_source(
            "begin integer result; integer array a[3:1]; result := 1 end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [0]
