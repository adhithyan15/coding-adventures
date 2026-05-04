"""Tests for ALGOL 60 to WebAssembly packaging."""

from pathlib import Path

import pytest
from algol_ir_compiler.compiler import _MAX_STRING_OUTPUT_BYTES, _MAX_TOTAL_OUTPUT_BYTES
from wasm_execution import TrapError
from wasm_runtime import WasiConfig, WasiHost, WasmExecutionLimits, WasmRuntime

from algol_wasm_compiler import (
    MAX_SOURCE_LENGTH,
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

    def test_uppercase_keywords_execute(self) -> None:
        result = compile_source("BEGIN INTEGER result; result := 7 END")
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_uppercase_comment_is_ignored(self) -> None:
        result = compile_source("begin COMMENT setup; integer result; result := 7 end")
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_comment_prefixed_identifier_is_not_skipped(self) -> None:
        result = compile_source(
            "begin integer result, commentary; commentary := 7; "
            "result := commentary end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_angle_not_equal_operator_executes(self) -> None:
        result = compile_source(
            "begin integer result; if 1 <> 2 then result := 7 else result := 0 end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_publication_symbol_operators_execute(self) -> None:
        result = compile_source(
            "begin integer result; "
            "if (2 ↑ 3 = 8) ∧ (3 ≤ 4) ∧ (5 ≥ 5) ∧ (1 ≠ 2) "
            "∧ (¬ false) ∧ (true ⊃ true) ∧ (true ≡ true) "
            "then result := 7 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_double_quoted_string_literal_writes_stdout(self) -> None:
        result = compile_source('begin output("Hi") end')
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [0]
        assert "".join(captured) == "Hi"

    def test_write_wasm_file(self, tmp_path: Path) -> None:
        out = tmp_path / "answer.wasm"
        result = write_wasm_file("begin integer result; result := 9 end", out)
        assert result.wasm_path == out
        assert out.read_bytes() == result.binary

    def test_type_error_reports_stage(self) -> None:
        with pytest.raises(AlgolWasmError) as raised:
            compile_source("begin integer result; result := false end")
        assert raised.value.stage == "type-check"

    def test_source_length_limit_reports_before_parse(self) -> None:
        with pytest.raises(AlgolWasmError) as raised:
            compile_source("x" * (MAX_SOURCE_LENGTH + 1))
        assert raised.value.stage == "source"
        assert "source length" in raised.value.message

    def test_program_without_result_variable_returns_zero(self) -> None:
        compiled = compile_source("begin print('Hi') end")
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(compiled.binary, "_start", []) == [0]
        assert "".join(captured) == "Hi"

    def test_non_integer_result_name_returns_zero(self) -> None:
        compiled = compile_source("begin real result; result := 2.5; print(result) end")
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(compiled.binary, "_start", []) == [0]
        assert "".join(captured) == "2.500"

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

    def test_chained_assignment_stores_right_to_left(self) -> None:
        result = compile_source(
            "begin integer result, other; result := other := 7; "
            "result := result + other end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [14]

    def test_integer_exponentiation_is_left_associative(self) -> None:
        result = compile_source("begin integer result; result := 2 ** 3 ** 2 end")
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [64]

    def test_real_base_integer_exponentiation_runs_in_wasm(self) -> None:
        result = compile_source(
            "begin integer result; real x; x := 2.0 ^ (0 - 3); "
            "if x < 0.126 then result := 8 else result := 0 end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [8]

    def test_real_exponentiation_runs_through_pow_import(self) -> None:
        result = compile_source(
            "begin integer result; real x; "
            "x := 9.0 ^ 0.5; "
            "result := entier(x * 10) "
            "end"
        )
        assert WasmRuntime(host=WasiHost()).load_and_run(
            result.binary, "_start", []
        ) == [30]

    def test_integer_base_real_exponentiation_promotes_to_real(self) -> None:
        result = compile_source(
            "begin integer result; real x; "
            "x := 4 ^ 0.5; "
            "result := entier(x * 10) "
            "end"
        )
        assert WasmRuntime(host=WasiHost()).load_and_run(
            result.binary, "_start", []
        ) == [20]

    def test_conditional_expression_assigns_selected_branch(self) -> None:
        result = compile_source(
            "begin integer result; result := if false then 1 else 7 end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_conditional_expression_only_evaluates_selected_branch(self) -> None:
        result = compile_source(
            "begin integer result; integer array a[1:1]; "
            "result := if true then 7 else a[2] "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_conditional_expression_promotes_integer_branch_to_real(self) -> None:
        result = compile_source(
            "begin integer result; real x; "
            "x := if false then 1 else 2.5; "
            "if x > 2.0 then result := 9 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [9]

    def test_string_and_boolean_conditional_expressions_execute(self) -> None:
        result = compile_source(
            "begin integer result; boolean ok; string word; "
            "ok := if false then false else true; "
            "word := if ok then 'YES' else 'NO'; "
            "if ok and (word = 'YES') then result := 7 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_statement_lists_allow_trailing_and_repeated_semicolons(self) -> None:
        result = compile_source(
            "begin integer result; ; result := 1;; result := 2; end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_simple_for_element_executes_once(self) -> None:
        result = compile_source(
            "begin integer result, i; "
            "result := 0; "
            "for i := 5 do result := result + i "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [5]

    def test_while_for_element_reevaluates_value_each_iteration(self) -> None:
        result = compile_source(
            "begin integer result, i, x; "
            "result := 0; x := 3; "
            "for i := x while x > 0 do "
            "begin result := result + i; x := x - 1 end "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [6]

    def test_multiple_for_elements_share_one_body(self) -> None:
        result = compile_source(
            "begin integer result, i; "
            "result := 0; "
            "for i := 1, 2 do "
            "begin result := result * 10 + i; result := result * 10 + i end "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [1122]

    def test_real_step_until_for_element_supports_negative_step(self) -> None:
        result = compile_source(
            "begin integer result; real x; "
            "result := 0; "
            "for x := 1.5 step -0.5 until 0.5 do result := result + 1 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [3]

    def test_array_element_for_control_variable_runs(self) -> None:
        result = compile_source(
            "begin integer result; integer array a[1:1]; "
            "result := 0; "
            "for a[1] := 1 step 1 until 4 do result := result + a[1] "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [10]

    def test_standard_numeric_builtin_functions_execute(self) -> None:
        result = compile_source(
            "begin integer result; real x, y; "
            "x := 0.0 - 2.5; y := abs(x); "
            "if y > 2.4 then result := 10 else result := 0; "
            "result := result + abs(0 - 7) + sign(x) + sign(0) + sign(5) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [17]

    def test_mixed_case_standard_builtin_functions_execute(self) -> None:
        result = compile_source(
            "begin integer result; real x; "
            "x := SQRT(9); "
            "result := ABS(0 - 7) + SIGN(x) + ENTIER(x) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [11]

    def test_publication_multiply_and_divide_execute(self) -> None:
        result = compile_source(
            "begin integer result; "
            "result := 6 × 7 + entier(8 ÷ 2) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [46]

    def test_entier_floors_positive_and_negative_reals(self) -> None:
        result = compile_source(
            "begin integer result; real x; "
            "x := 0.0 - 2.1; "
            "result := entier(2.9) * 10 + entier(x) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [17]

    def test_sqrt_builtin_executes_with_integer_and_real_arguments(self) -> None:
        result = compile_source(
            "begin integer result; real x; "
            "x := sqrt(9); "
            "result := entier(x) * 10 + entier(sqrt(0.25) * 10) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [35]

    def test_sqrt_negative_argument_returns_zero_through_runtime_failure(self) -> None:
        result = compile_source(
            "begin integer result; real x; "
            "x := sqrt(0.0 - 1.0); "
            "result := 7 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [0]

    def test_standard_real_math_builtins_execute_through_host_imports(self) -> None:
        result = compile_source(
            "begin integer result; real x; "
            "x := sin(0) + cos(0) + arctan(1) + ln(exp(1)); "
            "result := entier(x * 100) "
            "end"
        )
        assert WasmRuntime(host=WasiHost()).load_and_run(
            result.binary, "_start", []
        ) == [278]

    def test_ln_nonpositive_argument_returns_zero_through_runtime_failure(self) -> None:
        result = compile_source(
            "begin integer result; real x; "
            "x := ln(0); "
            "result := 7 "
            "end"
        )
        assert WasmRuntime(host=WasiHost()).load_and_run(
            result.binary, "_start", []
        ) == [0]

    def test_entier_out_of_i32_range_returns_zero_without_trapping(self) -> None:
        result = compile_source(
            "begin integer result; result := entier(1.0E100) end"
        )

        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [0]

    def test_entier_nan_returns_zero_without_trapping(self) -> None:
        result = compile_source(
            "begin integer result; real x; "
            "x := (1.0E308 * 10.0) * 0.0; "
            "result := entier(x) "
            "end"
        )

        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [0]

    def test_entier_keeps_i32_boundary_floor_semantics(self) -> None:
        result = compile_source(
            "begin integer result, floor, top; "
            "floor := entier(-2147483647.5) + 2147483647; "
            "top := entier(2147483647.9); "
            "if (floor = -1) and (top = 2147483647) "
            "then result := 7 else result := 0 "
            "end"
        )

        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_builtin_print_string_literal_writes_stdout(self) -> None:
        result = compile_source("begin integer result; print('Hi'); result := 7 end")
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [7]
        assert "".join(captured) == "Hi"

    def test_mixed_case_builtin_output_writes_stdout(self) -> None:
        result = compile_source("begin integer result; PRINT('Hi'); result := 7 end")
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [7]
        assert "".join(captured) == "Hi"

    def test_builtin_print_integer_and_boolean_write_stdout(self) -> None:
        result = compile_source(
            "begin integer result; print(42); output(true); result := 7 end"
        )
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [7]
        assert "".join(captured) == "42true"

    def test_builtin_print_negative_integer_writes_stdout(self) -> None:
        result = compile_source(
            "begin integer result; print(-12); result := 3 end"
        )
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [3]
        assert "".join(captured) == "-12"

    def test_builtin_print_real_writes_fixed_three_decimal_stdout(self) -> None:
        result = compile_source(
            "begin integer result; print(3.5); output(-0.125); result := 4 end"
        )
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [4]
        assert "".join(captured) == "3.500-0.125"

    def test_builtin_print_multiple_arguments_writes_stdout_in_order(self) -> None:
        result = compile_source(
            "begin integer result; real x; boolean ok; string msg; "
            "x := 1.5; ok := true; msg := 'Hi'; "
            "print(msg, ' ', result + 1, ' ', ok, ' ', x); result := 6 "
            "end"
        )
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [6]
        assert "".join(captured) == "Hi 1 true 1.500"

    def test_builtin_print_infinite_real_returns_zero_without_stdout(self) -> None:
        result = compile_source(
            "begin integer result; real x; "
            "x := 1.0E308 * 10.0; "
            "print(x); "
            "result := 7 "
            "end"
        )
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [0]
        assert "".join(captured) == ""

    def test_builtin_print_negative_infinite_real_writes_no_partial_sign(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; real x; "
            "x := 0.0 - (1.0E308 * 10.0); "
            "print(x); "
            "result := 7 "
            "end"
        )
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [0]
        assert "".join(captured) == ""

    def test_builtin_print_nan_real_returns_zero_without_stdout(self) -> None:
        result = compile_source(
            "begin integer result; real x; "
            "x := (1.0E308 * 10.0) * 0.0; "
            "print(x); "
            "result := 7 "
            "end"
        )
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [0]
        assert "".join(captured) == ""

    def test_string_variable_assignment_and_output_write_stdout(self) -> None:
        result = compile_source(
            "begin string msg; integer result; "
            "msg := 'Hi'; print(msg); result := 7 "
            "end"
        )
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [7]
        assert "".join(captured) == "Hi"

    def test_string_variable_copy_preserves_pointer_value(self) -> None:
        result = compile_source(
            "begin string first, second; integer result; "
            "first := 'OK'; second := first; output(second); result := 8 "
            "end"
        )
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [8]
        assert "".join(captured) == "OK"

    def test_empty_string_output_writes_nothing(self) -> None:
        result = compile_source(
            "begin string msg; integer result; msg := ''; print(msg); result := 9 end"
        )
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [9]
        assert "".join(captured) == ""

    def test_oversized_string_output_returns_zero_without_writing_stdout(self) -> None:
        oversized = "A" * (_MAX_STRING_OUTPUT_BYTES + 1)
        result = compile_source(
            f"begin integer result; print('{oversized}'); result := 9 end"
        )
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [0]
        assert "".join(captured) == ""

    def test_total_output_budget_caps_multiple_print_calls(self) -> None:
        chunk = "A" * _MAX_STRING_OUTPUT_BYTES
        result = compile_source(
            "begin integer result; "
            f"print('{chunk}'); "
            f"print('{chunk}'); "
            "print('B'); "
            "result := 9 end"
        )
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [0]
        assert "".join(captured) == "A" * _MAX_TOTAL_OUTPUT_BYTES

    def test_own_integer_persists_across_procedure_calls(self) -> None:
        result = compile_source(
            "begin own integer counter; integer result; "
            "procedure bump; begin integer local; "
            "local := 1; "
            "counter := counter + 1; "
            "result := local * 10 + counter "
            "end; "
            "bump; bump "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [12]

    def test_own_integer_array_persists_across_procedure_calls(self) -> None:
        result = compile_source(
            "begin own integer array counts[1:1]; integer result; "
            "procedure bump; "
            "begin counts[1] := counts[1] + 1; result := counts[1] end; "
            "counts[1] := 4; bump; bump "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [6]

    def test_local_integer_array_is_fresh_each_activation(self) -> None:
        result = compile_source(
            "begin integer result, n; "
            "procedure probe; begin integer array a[1:n]; "
            "a[1] := a[1] + 1; result := result * 10 + a[1] end; "
            "n := 1; probe; n := 3; probe "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [11]

    def test_own_integer_array_keeps_first_entry_bounds(self) -> None:
        result = compile_source(
            "begin integer result, n; "
            "procedure probe; begin own integer array a[1:n]; "
            "a[1] := a[1] + 1; result := result * 10 + a[1] end; "
            "n := 1; probe; n := 0; probe "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [12]

    def test_own_real_boolean_and_string_scalars_persist_across_calls(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure tick; "
            "begin own real scalar; own boolean seen; own string marker; "
            "if not seen then "
            "begin scalar := 1.5; seen := true; marker := 'OK'; result := 1 end "
            "else "
            "begin scalar := scalar + 1.0; "
            "if seen and (marker = 'OK') and (scalar > 2.0) then result := 7 "
            "else result := 0 "
            "end "
            "end; "
            "tick; tick "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_own_real_boolean_and_string_arrays_persist_across_calls(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure tick; "
            "begin own real array totals[1:1]; "
            "own boolean array ready[1:1]; "
            "own string array labels[1:1]; "
            "if not ready[1] then "
            "begin totals[1] := 2.5; ready[1] := true; "
            "labels[1] := 'ARR'; result := 1 end "
            "else "
            "begin totals[1] := totals[1] + 1.0; "
            "if ready[1] and (labels[1] = 'ARR') and (totals[1] > 3.0) "
            "then result := 8 else result := 0 "
            "end "
            "end; "
            "tick; tick "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [8]

    def test_for_control_by_name_writes_caller_storage(self) -> None:
        result = compile_source(
            "begin integer result, i; "
            "procedure run(k); integer k; "
            "begin for k := 1 step 1 until 3 do result := result + k end; "
            "i := 0; result := 0; run(i); result := result * 10 + i "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [64]

    def test_for_control_value_parameter_does_not_write_caller_storage(self) -> None:
        result = compile_source(
            "begin integer result, i; "
            "procedure run(k); value k; integer k; "
            "begin for k := 1 step 1 until 3 do result := result + k end; "
            "i := 9; result := 0; run(i); result := result * 10 + i "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [69]

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

    def test_boolean_implication_truth_table(self) -> None:
        result = compile_source(
            "begin integer result; "
            "result := 0; "
            "if true impl false then result := result + 8 else result := result; "
            "if true impl true then result := result + 4 else result := result; "
            "if false impl false then result := result + 2 else result := result; "
            "if false impl true then result := result + 1 else result := result "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_boolean_and_or_impl_short_circuit_rhs(self) -> None:
        result = compile_source(
            "begin integer result; "
            "boolean procedure mark; "
            "begin result := result + 100; mark := false end; "
            "result := 0; "
            "if false and mark then result := result + 1 "
            "else result := result + 7; "
            "if true or mark then result := result + 11 "
            "else result := result + 13; "
            "if false impl mark then result := result + 17 "
            "else result := result + 19 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [35]

    def test_boolean_equivalence_truth_table(self) -> None:
        result = compile_source(
            "begin integer result; "
            "result := 0; "
            "if true eqv true then result := result + 8 else result := result; "
            "if true eqv false then result := result + 4 else result := result; "
            "if false eqv false then result := result + 2 else result := result; "
            "if false eqv true then result := result + 1 else result := result "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [10]

    def test_boolean_equivalence_remains_strict(self) -> None:
        result = compile_source(
            "begin integer result; "
            "boolean procedure mark; "
            "begin result := result + 100; mark := false end; "
            "result := 0; "
            "if true eqv mark then result := result + 1 "
            "else result := result + 7 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [107]

    def test_or_binds_tighter_than_implication(self) -> None:
        result = compile_source(
            "begin integer result; "
            "if true or false impl false then result := 1 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [0]

    def test_real_variable_assignment_drives_comparison(self) -> None:
        result = compile_source(
            "begin integer result; "
            "real x; "
            "x := 1.5; "
            "if x > 1.0 then result := 7 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_integer_to_real_assignment_promotes_before_store(self) -> None:
        result = compile_source(
            "begin integer result; "
            "real x; "
            "x := 1; "
            "if x = 1.0 then result := 9 else result := 0 "
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

    def test_go_to_spelling_skips_statements_until_label(self) -> None:
        result = compile_source(
            "begin integer result; "
            "go to done; "
            "result := 99; "
            "done: result := 7 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_forward_goto_targets_second_label_on_statement(self) -> None:
        result = compile_source(
            "begin integer result; "
            "goto second; "
            "first: second: result := 7 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_forward_goto_targets_first_label_on_shared_statement(self) -> None:
        result = compile_source(
            "begin integer result; "
            "goto first; "
            "first: second: result := 8 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [8]

    def test_forward_goto_targets_multiple_terminal_labels(self) -> None:
        result = compile_source(
            "begin integer result; "
            "result := 3; "
            "goto done2; "
            "result := 99; "
            "done1: done2: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [3]

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

    def test_runtime_instruction_budget_stops_nonterminating_goto_loop(self) -> None:
        result = compile_source("begin integer result; loop: goto loop end")
        runtime = WasmRuntime(limits=WasmExecutionLimits(max_instructions=128))

        with pytest.raises(TrapError, match="instruction budget exhausted"):
            runtime.load_and_run(result.binary, "_start", [])

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

    def test_procedure_crossing_goto_can_target_first_label(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure escape; begin result := 4; goto done; result := 99 end; "
            "result := 0; escape; result := 1; "
            "done: result := result + 3 "
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

    def test_nonlocal_switch_designational_goto_selects_outer_entry(self) -> None:
        result = compile_source(
            "begin integer result, i; "
            "switch s := first, second; "
            "i := 2; "
            "begin goto s[i] end; "
            "first: result := 1; goto done; "
            "second: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_nested_switch_selection_entry_dispatches_through_inner_switch(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result, i; "
            "switch inner := first, second; "
            "switch outer := inner[i]; "
            "i := 2; goto outer[1]; "
            "first: result := 1; goto done; "
            "second: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_self_recursive_switch_selection_entry_dispatches_at_runtime(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result, i; "
            "switch s := done, if i = 0 then done else s[i]; "
            "i := 1; goto s[2]; "
            "result := 99; "
            "done: result := 7 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_switch_entry_can_target_nonlocal_label(self) -> None:
        result = compile_source(
            "begin integer result; "
            "result := 0; "
            "begin switch s := done; result := 5; goto s[1]; result := 99 end; "
            "done: result := result + 2 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_procedure_switch_entry_can_escape_to_outer_label(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure escape; "
            "begin switch s := done; result := 4; goto s[1]; result := 99 end; "
            "result := 0; escape; result := 1; "
            "done: result := result + 3 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

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

    def test_nonlocal_conditional_designational_goto_selects_outer_label(self) -> None:
        result = compile_source(
            "begin integer result, flag; "
            "begin goto if flag = 0 then left else right end; "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [1]

    def test_procedure_crossing_conditional_designational_goto_selects_outer_label(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result, flag; "
            "procedure escape; begin goto if flag = 0 then left else right end; "
            "flag := 1; escape; result := 0; "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_procedure_crossing_nonlocal_switch_selection_uses_declaring_scope(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result, flag; "
            "switch s := if flag = 0 then left else right; "
            "procedure escape; begin flag := 1; goto s[1] end; "
            "escape; "
            "result := 0; "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_switch_parameter_designational_goto_uses_caller_switch_scope(self) -> None:
        result = compile_source(
            "begin integer result, flag; "
            "procedure escape(sw); switch sw; begin goto sw[1] end; "
            "switch s := if flag = 0 then left else right; "
            "flag := 1; escape(s); result := 0; "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_switch_parameter_accepts_conditional_switch_actual(self) -> None:
        result = compile_source(
            "begin integer result; boolean flag; "
            "switch a := left; switch b := right; "
            "procedure escape(sw); switch sw; begin goto sw[1] end; "
            "flag := false; escape(if flag then a else b); result := 0; "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_by_name_switch_parameter_re_evaluates_conditional_actual(self) -> None:
        result = compile_source(
            "begin integer result, flag; "
            "switch a := left; switch b := right; "
            "procedure escape(sw); switch sw; begin flag := 1; goto sw[1] end; "
            "flag := 0; escape(if flag = 0 then a else b); result := 0; "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_value_switch_parameter_snapshots_conditional_actual(self) -> None:
        result = compile_source(
            "begin integer result, flag; "
            "switch a := left; switch b := right; "
            "procedure escape(sw); value sw; switch sw; "
            "begin flag := 1; goto sw[1] end; "
            "flag := 0; escape(if flag = 0 then a else b); result := 0; "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [1]

    def test_conditional_switch_actual_forwards_switch_parameters(self) -> None:
        result = compile_source(
            "begin integer result; boolean flag; "
            "procedure escape(sw); switch sw; begin goto sw[1] end; "
            "procedure select(a, b); switch a, b; "
            "begin escape(if flag then a else b) end; "
            "switch leftSwitch := left; switch rightSwitch := right; "
            "flag := false; select(leftSwitch, rightSwitch); result := 0; "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_forwarded_by_name_switch_parameter_preserves_lazy_actual(self) -> None:
        result = compile_source(
            "begin integer result, flag; "
            "switch a := left; switch b := right; "
            "procedure escape(sw); switch sw; begin flag := 1; goto sw[1] end; "
            "procedure relay(sw); switch sw; begin escape(sw) end; "
            "flag := 0; relay(if flag = 0 then a else b); result := 0; "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_forwarded_value_switch_parameter_snapshots_lazy_actual(self) -> None:
        result = compile_source(
            "begin integer result, flag; "
            "switch a := left; switch b := right; "
            "procedure escape(sw); value sw; switch sw; "
            "begin flag := 1; goto sw[1] end; "
            "procedure relay(sw); switch sw; begin escape(sw) end; "
            "flag := 0; relay(if flag = 0 then a else b); result := 0; "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [1]

    def test_forwarded_switch_parameter_propagates_descriptor_through_calls(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure escape(sw); switch sw; begin goto sw[1] end; "
            "procedure relay(sw); switch sw; begin escape(sw) end; "
            "switch s := second; "
            "relay(s); result := 0; "
            "first: result := 1; goto done; "
            "second: result := 8; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [8]

    def test_value_switch_parameter_dispatches_actual(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure escape(sw); value sw; switch sw; begin goto sw[1] end; "
            "switch s := second; "
            "escape(s); result := 0; "
            "first: result := 1; goto done; "
            "second: result := 8; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [8]

    def test_switch_parameter_dispatch_uses_selected_entry_result(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure escape(sw, fallback); switch sw; label fallback; "
            "begin goto sw[1] end; "
            "switch s := left, 90; "
            "escape(s, fallback); result := 99; goto done; "
            "fallback: result := 3; goto done; "
            "left: result := 11; go to done; "
            "90: result := 90; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [11]

    def test_switch_index_out_of_range_returns_zero(self) -> None:
        result = compile_source(
            "begin integer result; switch exits := done; "
            "goto exits[2]; result := 9; done: result := 7 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [0]

    def test_procedure_parameter_statement_call_dispatches_actual(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure twice(p); procedure p; begin p; p end; "
            "procedure bump; begin result := result + 1 end; "
            "result := 0; twice(bump) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_forwarded_procedure_parameter_propagates_descriptor_through_calls(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure invoke(p); procedure p; begin p end; "
            "procedure relay(p); procedure p; begin invoke(p) end; "
            "procedure bump; begin result := result + 3 end; "
            "result := 2; relay(bump) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [5]

    def test_value_procedure_parameter_statement_call_dispatches_actual(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure twice(p); value p; procedure p; begin p; p end; "
            "procedure bump; begin result := result + 1 end; "
            "result := 0; twice(bump) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_value_procedure_parameter_statement_call_passes_value_argument(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure invoke(p); value p; procedure p; begin p(7) end; "
            "procedure set(x); value x; integer x; begin result := x end; "
            "invoke(set) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_procedure_parameter_statement_call_passes_value_argument(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure invoke(p); procedure p; begin p(7) end; "
            "procedure set(x); value x; integer x; begin result := x end; "
            "invoke(set) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_procedure_parameter_statement_call_passes_read_only_by_name_argument(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure invoke(p); procedure p; begin p(result + 3) end; "
            "procedure set(x); integer x; begin result := x end; "
            "result := 2; invoke(set) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [5]

    def test_procedure_parameter_statement_call_passes_writable_by_name_argument(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure invoke(p); procedure p; "
            "begin integer y; y := 3; p(y); result := y end; "
            "procedure bump(x); integer x; begin x := x + 4 end; "
            "invoke(bump) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_procedure_parameter_statement_call_passes_array_element_argument(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; integer array a[1:1]; "
            "procedure invoke(p); procedure p; begin p(a[1]) end; "
            "procedure set(x); integer x; begin x := x + 7 end; "
            "a[1] := 0; invoke(set); result := a[1] "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_procedure_parameter_statement_call_passes_real_array_element_argument(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; real array a[1:1]; "
            "procedure invoke(p); procedure p; begin p(a[1]) end; "
            "procedure set(x); real x; begin x := 2.5 end; "
            "a[1] := 0.0; invoke(set); result := entier(a[1] * 10) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [25]

    def test_procedure_parameter_statement_call_passes_boolean_array_element_argument(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; boolean array flags[1:1]; "
            "procedure invoke(p); procedure p; begin p(flags[1]) end; "
            "procedure set(x); boolean x; begin x := true end; "
            "flags[1] := false; invoke(set); "
            "if flags[1] then result := 7 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_procedure_parameter_statement_call_passes_string_array_element_argument(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; string array messages[1:1]; "
            "procedure invoke(p); procedure p; begin p(messages[1]) end; "
            "procedure set(x); string x; begin x := 'OK' end; "
            "messages[1] := 'NO'; invoke(set); print(messages[1]); result := 7 "
            "end"
        )
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [7]
        assert "".join(captured) == "OK"

    def test_procedure_parameter_statement_call_passes_array_argument(self) -> None:
        result = compile_source(
            "begin integer result; integer array a[1:2]; "
            "procedure invoke(p); procedure p; begin p(a) end; "
            "procedure first(xs); integer xs; array xs; "
            "begin result := xs[1] end; "
            "a[1] := 9; invoke(first) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [9]

    def test_report_style_typed_array_parameter_specifier_executes(self) -> None:
        result = compile_source(
            "begin integer result; integer array a[1:2]; "
            "procedure first(xs); integer array xs; "
            "begin result := xs[1] end; "
            "a[1] := 9; first(a) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [9]

    def test_formal_procedure_array_argument_honors_value_array_copy(self) -> None:
        result = compile_source(
            "begin integer result; integer array a[1:2]; "
            "procedure invoke(p); procedure p; begin p(a) end; "
            "procedure mutate(xs); value xs; integer xs; array xs; "
            "begin xs[1] := 5 end; "
            "a[1] := 9; invoke(mutate); result := a[1] "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [9]

    def test_procedure_parameter_statement_call_passes_label_argument(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure invoke(p); procedure p; begin p(done) end; "
            "procedure jump(l); label l; begin result := 9; goto l end; "
            "invoke(jump); result := 0; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [9]

    def test_procedure_parameter_statement_call_passes_switch_argument(self) -> None:
        result = compile_source(
            "begin integer result; switch s := left, right; "
            "procedure invoke(p); procedure p; begin p(s) end; "
            "procedure jump(sw); switch sw; begin goto sw[2] end; "
            "invoke(jump); result := 0; "
            "left: result := 1; goto done; "
            "right: result := 8; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [8]

    def test_procedure_parameter_statement_call_passes_procedure_argument(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure bump; begin result := result + 1 end; "
            "procedure invoke(p); procedure p; begin p(bump) end; "
            "procedure use(q); procedure q; begin q; q end; "
            "result := 0; invoke(use) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_formal_procedure_nested_procedure_argument_rejects_mismatch(
        self,
    ) -> None:
        with pytest.raises(AlgolWasmError) as excinfo:
            compile_source(
                "begin integer result; "
                "procedure bump; begin result := result + 1 end; "
                "procedure invoke(p); procedure p; begin p(bump) end; "
                "procedure use(q); procedure q; begin q(1) end; "
                "invoke(use) "
                "end"
            )

        assert "accepting 1 argument(s), got 0" in str(excinfo.value)

    def test_formal_procedure_by_name_argument_remains_lazy(self) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure next; begin result := result + 1; next := result end; "
            "procedure invoke(p); procedure p; begin p(next) end; "
            "procedure use(x); integer x; begin result := x + x end; "
            "result := 0; invoke(use) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [3]

    def test_forwarded_procedure_parameter_with_argument_uses_static_link(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result, base; "
            "procedure invoke(p); procedure p; begin p(7) end; "
            "procedure relay(p); procedure p; begin invoke(p) end; "
            "procedure add(x); value x; integer x; begin result := base + x end; "
            "base := 5; relay(add) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [12]

    def test_procedure_parameter_dispatch_coerces_integer_to_real(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure invoke(p); procedure p; begin p(1) end; "
            "procedure set(x); value x; real x; "
            "begin if x = 1 then result := 9 end; "
            "invoke(set) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [9]

    def test_typed_procedure_parameter_expression_call_returns_real(self) -> None:
        result = compile_source(
            "begin integer result; real y; "
            "procedure invoke(f); real f; procedure f; "
            "begin y := f(2); if y = 4 then result := 1 else result := 0 end; "
            "real procedure twice(x); value x; real x; begin twice := x * 2 end; "
            "invoke(twice) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [1]

    def test_report_style_typed_procedure_parameter_specifier_executes(self) -> None:
        result = compile_source(
            "begin integer result; real y; "
            "procedure invoke(f); real procedure f; "
            "begin y := f(2); if y = 4 then result := 1 else result := 0 end; "
            "real procedure twice(x); value x; real x; begin twice := x * 2 end; "
            "invoke(twice) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [1]

    def test_typed_procedure_parameter_expression_call_passes_array_argument(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; integer array a[1:2]; "
            "procedure invoke(f); integer f; procedure f; "
            "begin result := f(a) end; "
            "integer procedure first(xs); integer xs; array xs; "
            "begin first := xs[1] end; "
            "a[1] := 11; invoke(first) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [11]

    def test_formal_procedure_call_passes_typed_procedure_argument(self) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure twice(x); value x; integer x; "
            "begin twice := x * 2 end; "
            "procedure invoke(p); procedure p; begin p(twice) end; "
            "procedure use(f); integer f; procedure f; "
            "begin result := f(4) end; "
            "invoke(use) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [8]

    def test_formal_procedure_call_accepts_read_only_forwarded_expression(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure id(x); value x; integer x; begin id := x end; "
            "integer procedure apply(f, x); integer f, x; procedure f; "
            "begin apply := f(x) end; "
            "result := apply(id, 3 + 4) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_formal_procedure_call_writes_forwarded_array_element_actual(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result, i; integer array a[1:1]; "
            "integer procedure inc(x); integer x; "
            "begin x := x + 1; inc := x end; "
            "integer procedure apply(f, x); integer f, x; procedure f; "
            "begin apply := f(x) end; "
            "a[1] := 3; i := 1; "
            "result := apply(inc, a[i]) * 10 + a[1] "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [44]

    def test_wrapped_formal_procedure_call_accepts_read_only_expression(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure id(x); value x; integer x; begin id := x end; "
            "integer procedure apply(f, x); integer f, x; procedure f; "
            "begin apply := f(x) end; "
            "integer procedure relay(g, y); integer g, y; procedure g; "
            "begin relay := apply(g, y) end; "
            "result := relay(id, 3 + 4) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_wrapped_formal_procedure_call_keeps_writable_actual_path(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure inc(x); integer x; "
            "begin x := x + 1; inc := x end; "
            "integer procedure apply(f, x); integer f, x; procedure f; "
            "begin apply := f(x) end; "
            "integer procedure relay(g, y); integer g, y; procedure g; "
            "begin relay := apply(g, y) end; "
            "result := 3; result := relay(inc, result) * 10 + result "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [44]

    def test_formal_procedure_actual_shape_accepts_nested_read_only_actual(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure id(x); value x; integer x; begin id := x end; "
            "integer procedure relay(g, y); integer g, y; procedure g; "
            "begin relay := g(y) end; "
            "procedure invoke(p); integer p; procedure p; "
            "begin result := p(id, 3 + 4) end; "
            "invoke(relay) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_formal_procedure_actual_shape_keeps_nested_writable_actual(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure inc(x); integer x; "
            "begin x := x + 1; inc := x end; "
            "integer procedure relay(g, y); integer g, y; procedure g; "
            "begin relay := g(y) end; "
            "procedure invoke(p); integer p; procedure p; "
            "begin result := 3; result := p(inc, result) * 10 + result end; "
            "invoke(relay) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [44]

    def test_formal_procedure_forwards_formal_procedure_actual_read_only(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure id(x); value x; integer x; begin id := x end; "
            "integer procedure relay1(g, y); integer g, y; procedure g; "
            "begin relay1 := g(y) end; "
            "integer procedure relay2(p, h, z); integer p, h, z; "
            "procedure p, h; begin relay2 := p(h, z) end; "
            "procedure invoke(q); integer q; procedure q; "
            "begin result := q(relay1, id, 3 + 4) end; "
            "invoke(relay2) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_formal_procedure_forwards_formal_procedure_actual_writable(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure inc(x); integer x; "
            "begin x := x + 1; inc := x end; "
            "integer procedure relay1(g, y); integer g, y; procedure g; "
            "begin relay1 := g(y) end; "
            "integer procedure relay2(p, h, z); integer p, h, z; "
            "procedure p, h; begin relay2 := p(h, z) end; "
            "procedure invoke(q); integer q; procedure q; "
            "begin result := 3; "
            "result := q(relay1, inc, result) * 10 + result end; "
            "invoke(relay2) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [44]

    def test_real_procedure_parameter_accepts_integer_return_actual(self) -> None:
        result = compile_source(
            "begin integer result; real y; "
            "procedure invoke(f); real f; procedure f; "
            "begin y := f(2); if y = 4.0 then result := 1 else result := 0 end; "
            "integer procedure twice(x); value x; integer x; "
            "begin twice := x * 2 end; "
            "invoke(twice) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [1]

    def test_real_procedure_can_call_integer_procedure(self) -> None:
        result = compile_source(
            "begin integer result; real y; "
            "integer procedure twice(x); value x; integer x; "
            "begin twice := x * 2 end; "
            "real procedure wrap(x); value x; integer x; "
            "begin wrap := twice(x) end; "
            "y := wrap(2); "
            "if y = 4.0 then result := 1 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [1]

    def test_value_typed_procedure_parameter_expression_call_returns_real(self) -> None:
        result = compile_source(
            "begin integer result; real y; "
            "procedure invoke(f); value f; real f; procedure f; "
            "begin y := f(2); if y = 4 then result := 1 else result := 0 end; "
            "real procedure twice(x); value x; real x; begin twice := x * 2 end; "
            "invoke(twice) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [1]

    def test_forwarded_typed_procedure_parameter_expression_uses_static_link(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result, base; real y; "
            "procedure invoke(f); real f; procedure f; "
            "begin y := f(2); if y = 7 then result := 1 else result := 0 end; "
            "procedure relay(f); real f; procedure f; begin invoke(f) end; "
            "real procedure addbase(x); value x; real x; "
            "begin addbase := base + x end; "
            "base := 5; relay(addbase) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [1]

    def test_procedure_call_can_be_relation_operand(self) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure twice(x); value x; integer x; begin twice := x * 2 end; "
            "if twice(2) = 4 then result := 1 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [1]

    def test_procedure_call_can_be_array_subscript(self) -> None:
        result = compile_source(
            "begin integer result; integer array a[1:4]; "
            "integer procedure idx(x); value x; integer x; begin idx := x end; "
            "a[idx(2)] := 7; result := a[idx(2)] "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

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

    def test_bare_no_argument_typed_procedure_expression_runs(self) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure seven; begin seven := 7 end; "
            "result := seven "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_explicit_empty_no_argument_typed_procedure_expression_runs(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure seven(); begin seven := 7 end; "
            "result := seven() "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_explicit_empty_no_argument_statement_call_runs(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure mark(); begin result := 9 end; "
            "mark() "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [9]

    def test_bare_procedure_expression_by_name_re_evaluates_each_read(self) -> None:
        result = compile_source(
            "begin integer result, calls; "
            "integer procedure next; begin calls := calls + 1; next := calls end; "
            "integer procedure pair(x); integer x; begin pair := x * 10 + x end; "
            "result := pair(next) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [12]

    def test_boolean_value_procedure_returns_boolean_result(self) -> None:
        result = compile_source(
            "begin integer result; "
            "boolean procedure negate(x); value x; boolean x; "
            "begin negate := not x end; "
            "if negate(false) then result := 7 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_real_value_procedure_returns_real_result(self) -> None:
        result = compile_source(
            "begin integer result; real y; "
            "real procedure half(x); value x; real x; "
            "begin half := x / 2 end; "
            "y := half(3); "
            "if y > 1.0 then result := 7 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_string_value_procedure_returns_string_result(self) -> None:
        result = compile_source(
            "begin string msg; integer result; "
            "string procedure id(x); value x; string x; "
            "begin id := x end; "
            "msg := id('Hi'); print(msg); result := 7 "
            "end"
        )
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [7]
        assert "".join(captured) == "Hi"

    def test_integer_actual_promotes_for_real_value_parameter(self) -> None:
        result = compile_source(
            "begin integer result; real y; "
            "real procedure id(x); value x; real x; "
            "begin id := x end; "
            "y := id(1); "
            "if y = 1.0 then result := 9 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [9]

    def test_void_procedure_statement_writes_outer_frame(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure setresult(x); value x; integer x; "
            "begin result := x end; "
            "setresult(6) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [6]

    def test_forward_sibling_procedure_call_executes(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure first; begin second end; "
            "procedure second; begin result := 7 end; "
            "first "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_forward_read_only_by_name_callee_accepts_expression_actual(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure relay(x); integer x; begin emit(x) end; "
            "procedure emit(y); integer y; begin result := y end; "
            "relay(3 + 4) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_procedure_body_sees_later_block_declarations(self) -> None:
        result = compile_source(
            "begin "
            "procedure set; begin result := 7 end; "
            "integer result; "
            "switch route := done; "
            "procedure jump; begin goto route[1] end; "
            "set; jump; "
            "done: result := result + 1 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [8]

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

    def test_string_by_name_parameter_assignment_writes_back(self) -> None:
        result = compile_source(
            "begin string msg; integer result; "
            "procedure setmsg(x); string x; begin x := 'OK' end; "
            "msg := 'Hi'; setmsg(msg); print(msg); result := 8 "
            "end"
        )
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [8]
        assert "".join(captured) == "OK"

    def test_integer_array_parameter_writes_back_through_descriptor(self) -> None:
        result = compile_source(
            "begin integer array xs[1:2]; integer result; "
            "procedure setfirst(a); integer a; array a; begin a[1] := 9 end; "
            "xs[1] := 4; setfirst(xs); result := xs[1] "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [9]

    def test_value_array_parameter_copies_descriptor_and_elements(self) -> None:
        result = compile_source(
            "begin integer array xs[2:3]; integer result; "
            "procedure setfirst(a); value a; integer a; array a; "
            "begin result := a[2]; a[2] := 9; result := result * 10 + a[2] end; "
            "xs[2] := 4; setfirst(xs); result := result * 10 + xs[2] "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [494]

    def test_real_value_array_parameter_copies_without_aliasing(self) -> None:
        result = compile_source(
            "begin real array xs[1:1]; integer result; "
            "procedure bump(a); value a; real a; array a; "
            "begin a[1] := a[1] + 1.5; if a[1] > 3.0 then result := 7 end; "
            "xs[1] := 2.0; bump(xs); "
            "if xs[1] < 3.0 then result := result + 1 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [8]

    def test_boolean_and_string_value_array_parameters_copy_without_aliasing(
        self,
    ) -> None:
        result = compile_source(
            "begin boolean array flags[1:1]; string array words[1:1]; "
            "integer result; "
            "procedure mutate(flagCopy, wordCopy); "
            "value flagCopy, wordCopy; "
            "boolean flagCopy; string wordCopy; array flagCopy, wordCopy; "
            "begin flagCopy[1] := false; wordCopy[1] := 'copy'; "
            "if (not flagCopy[1]) and (wordCopy[1] = 'copy') "
            "then result := 10 else result := 0 "
            "end; "
            "flags[1] := true; words[1] := 'orig'; mutate(flags, words); "
            "if flags[1] and (words[1] = 'orig') then result := result + 1 "
            "else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [11]

    def test_array_parameter_runtime_dimension_mismatch_returns_from_callee(
        self,
    ) -> None:
        result = compile_source(
            "begin integer array xs[1:2]; integer result; "
            "procedure probe(a); integer a; array a; begin result := a[1, 1] end; "
            "probe(xs); result := 1 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [1]

    def test_label_parameter_jumps_to_caller_label(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure jump(target); label target; begin goto target end; "
            "jump(done); result := 1; "
            "done: result := 7 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_label_parameter_accepts_conditional_designational_actual(self) -> None:
        result = compile_source(
            "begin integer result; boolean flag; "
            "procedure jump(target); label target; begin goto target end; "
            "flag := false; jump(if flag then left else right); "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_by_name_label_parameter_re_evaluates_conditional_actual(self) -> None:
        result = compile_source(
            "begin integer result, flag; "
            "procedure jump(target); label target; begin flag := 1; goto target end; "
            "flag := 0; jump(if flag = 0 then left else right); "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_label_parameter_accepts_numeric_label_actual(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure jump(target); label target; begin goto target end; "
            "jump(10); result := 1; "
            "10: result := 7 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_label_parameter_accepts_switch_selection_actual(self) -> None:
        result = compile_source(
            "begin integer result, i; switch s := left, right; "
            "procedure jump(target); label target; begin goto target end; "
            "i := 2; jump(s[i]); "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_value_label_parameter_jumps_to_caller_label(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure jump(target); value target; label target; "
            "begin goto target end; "
            "jump(done); result := 1; "
            "done: result := 7 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_value_label_parameter_snapshots_conditional_actual(self) -> None:
        result = compile_source(
            "begin integer result, flag; "
            "procedure jump(target); value target; label target; "
            "begin flag := 1; goto target end; "
            "flag := 0; jump(if flag = 0 then left else right); "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [1]

    def test_forwarded_label_parameter_propagates_through_intermediate_procedure(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure jump(target); label target; begin goto target end; "
            "procedure relay(target); label target; begin jump(target) end; "
            "relay(done); result := 1; "
            "done: result := 8 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [8]

    def test_forwarded_by_name_label_parameter_preserves_lazy_actual(self) -> None:
        result = compile_source(
            "begin integer result, flag; "
            "procedure jump(target); label target; begin flag := 1; goto target end; "
            "procedure relay(target); label target; begin jump(target) end; "
            "flag := 0; relay(if flag = 0 then left else right); "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_formal_procedure_call_passes_conditional_label_argument(self) -> None:
        result = compile_source(
            "begin integer result; boolean flag; "
            "procedure invoke(p); procedure p; "
            "begin p(if flag then left else right) end; "
            "procedure jump(target); label target; begin goto target end; "
            "flag := false; invoke(jump); "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_formal_procedure_label_argument_remains_lazy_until_goto(self) -> None:
        result = compile_source(
            "begin integer result, flag; "
            "procedure invoke(p); procedure p; "
            "begin p(if flag = 0 then left else right) end; "
            "procedure jump(target); label target; "
            "begin flag := 1; goto target end; "
            "flag := 0; invoke(jump); "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_formal_procedure_value_label_argument_snapshots_before_call(self) -> None:
        result = compile_source(
            "begin integer result, flag; "
            "procedure invoke(p); procedure p; "
            "begin p(if flag = 0 then left else right) end; "
            "procedure jump(target); value target; label target; "
            "begin flag := 1; goto target end; "
            "flag := 0; invoke(jump); "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [1]

    def test_formal_procedure_call_passes_switch_selection_label_argument(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result, i; switch s := left, right; "
            "procedure invoke(p); procedure p; begin p(s[i]) end; "
            "procedure jump(target); label target; begin goto target end; "
            "i := 2; invoke(jump); "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_formal_procedure_call_passes_conditional_switch_argument(self) -> None:
        result = compile_source(
            "begin integer result; boolean flag; "
            "switch a := left; switch b := right; "
            "procedure invoke(p); procedure p; "
            "begin p(if flag then a else b) end; "
            "procedure escape(sw); switch sw; begin goto sw[1] end; "
            "flag := false; invoke(escape); result := 0; "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_formal_procedure_switch_argument_remains_lazy_until_selection(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result, flag; "
            "switch a := left; switch b := right; "
            "procedure invoke(p); procedure p; "
            "begin p(if flag = 0 then a else b) end; "
            "procedure escape(sw); switch sw; begin flag := 1; goto sw[1] end; "
            "flag := 0; invoke(escape); result := 0; "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [2]

    def test_formal_procedure_value_switch_argument_snapshots_before_call(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result, flag; "
            "switch a := left; switch b := right; "
            "procedure invoke(p); procedure p; "
            "begin p(if flag = 0 then a else b) end; "
            "procedure escape(sw); value sw; switch sw; "
            "begin flag := 1; goto sw[1] end; "
            "flag := 0; invoke(escape); result := 0; "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
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

    def test_read_only_real_by_name_expression_runs_through_eval_thunk(self) -> None:
        result = compile_source(
            "begin integer result; real y; "
            "real procedure id(x); real x; begin id := x end; "
            "y := id(1.5); "
            "if y > 1.0 then result := 7 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_read_only_boolean_by_name_expression_runs_through_eval_thunk(
        self,
    ) -> None:
        result = compile_source(
            "begin integer result; boolean flag; "
            "procedure test(b); boolean b; "
            "begin if b then result := 9 else result := 0 end; "
            "flag := false; test(not flag) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [9]

    def test_read_only_string_by_name_literal_runs_through_eval_thunk(self) -> None:
        result = compile_source(
            "begin integer result; "
            "procedure emit(s); string s; begin print(s); result := 7 end; "
            "emit('Hi') "
            "end"
        )
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [7]
        assert "".join(captured) == "Hi"

    def test_real_by_name_scalar_write_through_storage_pointer(self) -> None:
        result = compile_source(
            "begin integer result; real y; "
            "procedure bump(x); real x; begin x := x + 1.5 end; "
            "y := 2.0; "
            "bump(y); "
            "if y > 3.0 then result := 9 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [9]

    def test_real_array_element_by_name_uses_eval_and_store_thunks(self) -> None:
        result = compile_source(
            "begin integer result; real array a[1:1]; "
            "procedure bump(x); real x; begin x := x + 1.5 end; "
            "a[1] := 2.0; "
            "bump(a[1]); "
            "if a[1] > 3.0 then result := 5 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [5]

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

    def test_mutually_recursive_typed_procedures_execute(self) -> None:
        result = compile_source(
            "begin integer result; "
            "integer procedure even(n); value n; integer n; "
            "begin if n = 0 then even := 1 else even := odd(n - 1) end; "
            "integer procedure odd(n); value n; integer n; "
            "begin if n = 0 then odd := 0 else odd := even(n - 1) end; "
            "result := odd(5) "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [1]

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

    def test_real_array_element_store_and_load(self) -> None:
        result = compile_source(
            "begin integer result; real array a[1:3]; "
            "a[2] := 1.5; "
            "if a[2] > 1.0 then result := 7 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_boolean_array_element_store_and_load(self) -> None:
        result = compile_source(
            "begin integer result; boolean array flags[1:2]; "
            "flags[1] := true; "
            "if flags[1] then result := 7 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_boolean_equality_comparison(self) -> None:
        result = compile_source(
            "begin integer result; boolean flag; "
            "flag := true; "
            "if flag = true then result := 7 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [7]

    def test_string_array_element_store_and_output(self) -> None:
        result = compile_source(
            "begin integer result; string array messages[1:2]; "
            "messages[1] := 'Hi'; print(messages[1]); result := 8 "
            "end"
        )
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [8]
        assert "".join(captured) == "Hi"

    def test_string_array_element_by_name_uses_word_thunks(self) -> None:
        result = compile_source(
            "begin integer result; string array messages[1:1]; "
            "procedure setmsg(x); string x; begin x := 'Bye' end; "
            "messages[1] := 'Hi'; setmsg(messages[1]); print(messages[1]); result := 9 "
            "end"
        )
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [9]
        assert "".join(captured) == "Bye"

    def test_string_equality_comparison_uses_descriptor_contents(self) -> None:
        result = compile_source(
            "begin integer result; string msg; "
            "string procedure memo(s); value s; string s; "
            "begin own string saved; "
            "if saved = '' then saved := s; "
            "memo := saved "
            "end; "
            "msg := memo('Hi'); "
            "if msg = 'Hi' then result := 1 else result := 100; "
            "if memo('Bye') != 'Bye' then result := result + 6 "
            "else result := result + 100; "
            "print(msg) "
            "end"
        )
        captured: list[str] = []
        runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

        assert runtime.load_and_run(result.binary, "_start", []) == [7]
        assert "".join(captured) == "Hi"

    def test_multidimensional_real_array_uses_row_major_offsets(self) -> None:
        result = compile_source(
            "begin integer result; real array a[1:2, 1:2]; "
            "a[2, 2] := 3.5; "
            "if a[2, 2] > 3.0 then result := 9 else result := 0 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [9]

    def test_dynamic_array_bounds_are_evaluated_at_block_entry(self) -> None:
        result = compile_source(
            "begin integer result, lo, hi; "
            "lo := 2; hi := 4; "
            "begin integer array a[lo:hi]; a[3] := 8; result := a[3] end "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [8]

    def test_terminal_label_can_end_nested_block(self) -> None:
        result = compile_source(
            "begin integer result; "
            "begin result := 13; goto done; result := 0; done: end "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [13]

    def test_out_of_bounds_array_access_returns_zero(self) -> None:
        result = compile_source(
            "begin integer result; integer array a[1:2]; "
            "a[3] := 9; result := 1 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [0]

    def test_integer_division_by_zero_returns_zero(self) -> None:
        result = compile_source(
            "begin integer result, divisor; divisor := 0; result := 10 div divisor end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [0]

    def test_integer_modulo_by_zero_returns_zero(self) -> None:
        result = compile_source(
            "begin integer result, divisor; divisor := 0; result := 10 mod divisor end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [0]

    def test_integer_division_overflow_returns_zero(self) -> None:
        result = compile_source(
            "begin integer result, low, divisor; "
            "low := 0 - 2147483647 - 1; "
            "divisor := 0 - 1; "
            "result := low div divisor "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [0]

    def test_integer_modulo_overflow_returns_zero(self) -> None:
        result = compile_source(
            "begin integer result, low, divisor; "
            "low := 0 - 2147483647 - 1; "
            "divisor := 0 - 1; "
            "result := low mod divisor "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [0]

    def test_real_division_by_zero_returns_zero(self) -> None:
        result = compile_source(
            "begin integer result; real x, divisor; "
            "divisor := 0.0; x := 1.0 / divisor; result := 7 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [0]

    def test_zero_real_base_negative_exponent_returns_zero(self) -> None:
        result = compile_source(
            "begin integer result; real x; "
            "x := 0.0 ^ (0 - 1); result := 7 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [0]

    def test_real_pow_domain_error_returns_zero(self) -> None:
        result = compile_source(
            "begin integer result; real x; "
            "x := (0.0 - 1.0) ^ 0.5; result := 7 "
            "end"
        )
        assert WasmRuntime(host=WasiHost()).load_and_run(
            result.binary, "_start", []
        ) == [0]

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

    def test_array_allocation_element_cap_returns_zero(self) -> None:
        result = compile_source(
            "begin integer result; integer array a[1:4097]; result := 1 end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [0]

    def test_heap_exhaustion_returns_zero(self) -> None:
        result = compile_source(
            "begin integer result; "
            "real array a[1:4096], b[1:4096], c[1:4096]; "
            "result := 1 "
            "end"
        )
        assert WasmRuntime().load_and_run(result.binary, "_start", []) == [0]
