"""End-to-end tests for the Dartmouth BASIC → JVM pipeline.

Every test compiles a BASIC program to JVM bytecode and runs it through the
JVM simulator, asserting on stdout output.  This mirrors the WASM integration
tests but targets the JVM backend.
"""

from __future__ import annotations

import pytest
from unittest.mock import patch

from dartmouth_basic_jvm_compiler import BasicError, RunResult, run_basic


# ---------------------------------------------------------------------------
# LET / arithmetic
# ---------------------------------------------------------------------------


class TestLet:
    """Variable assignment and arithmetic are compiled and executed correctly."""

    def test_let_constant(self) -> None:
        result = run_basic("10 LET A = 42\n20 PRINT A\n30 END\n")
        assert result.output == "42\n"

    def test_let_addition(self) -> None:
        result = run_basic("10 LET A = 3\n20 LET B = 4\n30 LET C = A + B\n40 PRINT C\n50 END\n")
        assert result.output == "7\n"

    def test_let_subtraction(self) -> None:
        result = run_basic("10 LET A = 10\n20 LET B = 3\n30 PRINT A - B\n40 END\n")
        assert result.output == "7\n"

    def test_let_multiplication(self) -> None:
        result = run_basic("10 LET A = 6\n20 LET B = 7\n30 PRINT A * B\n40 END\n")
        assert result.output == "42\n"

    def test_let_division(self) -> None:
        result = run_basic("10 LET A = 10\n20 LET B = 2\n30 PRINT A / B\n40 END\n")
        assert result.output == "5\n"

    def test_let_chained_variables(self) -> None:
        result = run_basic(
            "10 LET A = 5\n20 LET B = A * 2\n30 LET C = B + 1\n40 PRINT C\n50 END\n"
        )
        assert result.output == "11\n"

    def test_let_negative_result(self) -> None:
        result = run_basic("10 LET A = 3 - 10\n20 PRINT A\n30 END\n")
        assert result.output == "-7\n"

    def test_let_complex_expression(self) -> None:
        result = run_basic("10 LET A = (2 + 3) * (4 - 1)\n20 PRINT A\n30 END\n")
        assert result.output == "15\n"

    def test_let_overwrites_previous_value(self) -> None:
        result = run_basic(
            "10 LET A = 1\n20 LET A = 2\n30 LET A = 3\n40 PRINT A\n50 END\n"
        )
        assert result.output == "3\n"


# ---------------------------------------------------------------------------
# PRINT — string literals
# ---------------------------------------------------------------------------


class TestPrintString:
    """PRINT of string literals produces correct ASCII output."""

    def test_hello_world(self) -> None:
        result = run_basic("10 PRINT \"HELLO WORLD\"\n20 END\n")
        assert result.output == "HELLO WORLD\n"

    def test_print_appends_newline(self) -> None:
        result = run_basic("10 PRINT \"X\"\n20 END\n")
        assert result.output.endswith("\n")

    def test_print_empty_string(self) -> None:
        result = run_basic("10 PRINT \"\"\n20 END\n")
        assert result.output == "\n"

    def test_print_bare(self) -> None:
        result = run_basic("10 PRINT\n20 END\n")
        assert result.output == "\n"

    def test_print_digits_as_string(self) -> None:
        result = run_basic("10 PRINT \"123\"\n20 END\n")
        assert result.output == "123\n"

    def test_print_multiple_lines(self) -> None:
        result = run_basic(
            "10 PRINT \"FIRST\"\n20 PRINT \"SECOND\"\n30 END\n"
        )
        assert result.output == "FIRST\nSECOND\n"


# ---------------------------------------------------------------------------
# PRINT — numeric expressions
# ---------------------------------------------------------------------------


class TestPrintNumeric:
    """PRINT of numeric expressions produces correct decimal ASCII output."""

    def test_print_zero(self) -> None:
        result = run_basic("10 PRINT 0\n20 END\n")
        assert result.output == "0\n"

    def test_print_positive_integer(self) -> None:
        result = run_basic("10 PRINT 42\n20 END\n")
        assert result.output == "42\n"

    def test_print_negative_integer(self) -> None:
        result = run_basic("10 PRINT -7\n20 END\n")
        assert result.output == "-7\n"

    def test_print_variable(self) -> None:
        result = run_basic("10 LET X = 99\n20 PRINT X\n30 END\n")
        assert result.output == "99\n"

    def test_print_expression_result(self) -> None:
        result = run_basic("10 PRINT 3 * 3 + 1\n20 END\n")
        assert result.output == "10\n"

    def test_print_large_number(self) -> None:
        result = run_basic("10 PRINT 12345\n20 END\n")
        assert result.output == "12345\n"

    def test_print_powers_of_ten(self) -> None:
        result = run_basic(
            "10 PRINT 1\n20 PRINT 10\n30 PRINT 100\n40 PRINT 1000\n50 END\n"
        )
        assert result.output == "1\n10\n100\n1000\n"

    def test_print_leading_zero_suppressed(self) -> None:
        result = run_basic("10 PRINT 5\n20 END\n")
        assert result.output == "5\n"
        assert not result.output.startswith("0")

    def test_print_mixed_string_and_number(self) -> None:
        result = run_basic(
            "10 LET X = 42\n20 PRINT \"ANSWER IS \", X\n30 END\n"
        )
        assert result.output == "ANSWER IS 42\n"

    def test_print_arithmetic_in_print(self) -> None:
        result = run_basic("10 PRINT 2 + 2\n20 END\n")
        assert result.output == "4\n"


# ---------------------------------------------------------------------------
# FOR / NEXT
# ---------------------------------------------------------------------------


class TestForNext:
    """FOR/NEXT loops execute the correct number of iterations."""

    def test_for_prints_sequence(self) -> None:
        result = run_basic(
            "10 FOR I = 1 TO 5\n20 PRINT I\n30 NEXT I\n40 END\n"
        )
        assert result.output == "1\n2\n3\n4\n5\n"

    def test_for_sum_one_to_ten(self) -> None:
        result = run_basic(
            "10 LET S = 0\n20 FOR I = 1 TO 10\n30 LET S = S + I\n"
            "40 NEXT I\n50 PRINT S\n60 END\n"
        )
        assert result.output == "55\n"

    def test_for_sum_one_to_hundred(self) -> None:
        result = run_basic(
            "10 LET S = 0\n20 FOR I = 1 TO 100\n30 LET S = S + I\n"
            "40 NEXT I\n50 PRINT S\n60 END\n"
        )
        assert result.output == "5050\n"

    def test_for_with_step(self) -> None:
        result = run_basic(
            "10 FOR I = 0 TO 10 STEP 2\n20 PRINT I\n30 NEXT I\n40 END\n"
        )
        assert result.output == "0\n2\n4\n6\n8\n10\n"

    def test_for_loop_zero_iterations(self) -> None:
        result = run_basic(
            "10 LET X = 0\n20 FOR I = 5 TO 1\n30 LET X = X + 1\n"
            "40 NEXT I\n50 PRINT X\n60 END\n"
        )
        assert result.output == "0\n"

    def test_factorial_5(self) -> None:
        result = run_basic(
            "10 LET F = 1\n20 FOR I = 1 TO 5\n30 LET F = F * I\n"
            "40 NEXT I\n50 PRINT F\n60 END\n"
        )
        assert result.output == "120\n"


# ---------------------------------------------------------------------------
# IF / THEN
# ---------------------------------------------------------------------------


class TestIfThen:
    """Conditional branches take/skip correctly for all six relational operators."""

    def test_if_eq_true_branches(self) -> None:
        result = run_basic(
            "10 LET A = 5\n20 IF A = 5 THEN 40\n30 PRINT \"NO\"\n"
            "40 PRINT \"YES\"\n50 END\n"
        )
        assert result.output == "YES\n"

    def test_if_eq_false_falls_through(self) -> None:
        result = run_basic(
            "10 LET A = 3\n20 IF A = 5 THEN 40\n30 PRINT \"NO\"\n"
            "40 END\n"
        )
        assert result.output == "NO\n"

    def test_if_lt_true(self) -> None:
        result = run_basic(
            "10 IF 3 < 5 THEN 30\n20 PRINT \"NO\"\n30 PRINT \"YES\"\n40 END\n"
        )
        assert result.output == "YES\n"

    def test_if_gt_true(self) -> None:
        result = run_basic(
            "10 IF 7 > 2 THEN 30\n20 PRINT \"NO\"\n30 PRINT \"YES\"\n40 END\n"
        )
        assert result.output == "YES\n"

    def test_if_ne_true(self) -> None:
        result = run_basic(
            "10 IF 3 <> 4 THEN 30\n20 PRINT \"NO\"\n30 PRINT \"YES\"\n40 END\n"
        )
        assert result.output == "YES\n"

    def test_if_le_true(self) -> None:
        result = run_basic(
            "10 IF 5 <= 5 THEN 30\n20 PRINT \"NO\"\n30 PRINT \"YES\"\n40 END\n"
        )
        assert result.output == "YES\n"

    def test_if_ge_true(self) -> None:
        result = run_basic(
            "10 IF 6 >= 6 THEN 30\n20 PRINT \"NO\"\n30 PRINT \"YES\"\n40 END\n"
        )
        assert result.output == "YES\n"

    def test_if_used_to_implement_max(self) -> None:
        result = run_basic(
            "10 LET A = 7\n20 LET B = 3\n30 LET M = A\n"
            "40 IF B > A THEN 60\n50 GOTO 70\n60 LET M = B\n"
            "70 PRINT M\n80 END\n"
        )
        assert result.output == "7\n"


# ---------------------------------------------------------------------------
# GOTO
# ---------------------------------------------------------------------------


class TestGoto:
    """GOTO branches forward and backward correctly."""

    def test_goto_skips_code(self) -> None:
        result = run_basic(
            "10 GOTO 30\n20 PRINT \"SKIP\"\n30 PRINT \"AFTER\"\n40 END\n"
        )
        assert result.output == "AFTER\n"

    def test_goto_backward_counts(self) -> None:
        result = run_basic(
            "10 LET N = 3\n20 PRINT N\n30 LET N = N - 1\n"
            "40 IF N > 0 THEN 20\n50 END\n"
        )
        assert result.output == "3\n2\n1\n"


# ---------------------------------------------------------------------------
# Classic programs
# ---------------------------------------------------------------------------


class TestClassicPrograms:
    """Complete programs that exercise multiple BASIC features together."""

    def test_fibonacci_first_ten(self) -> None:
        result = run_basic(
            "10 LET A = 0\n20 LET B = 1\n"
            "30 FOR I = 1 TO 10\n"
            "40 LET C = A + B\n50 LET A = B\n60 LET B = C\n"
            "70 NEXT I\n80 PRINT B\n90 END\n"
        )
        assert result.output == "89\n"

    def test_gauss_sum(self) -> None:
        result = run_basic(
            "10 LET S = 0\n20 FOR I = 1 TO 100\n30 LET S = S + I\n"
            "40 NEXT I\n50 PRINT S\n60 END\n"
        )
        assert result.output == "5050\n"

    def test_multiplication_table_row(self) -> None:
        result = run_basic(
            "10 FOR I = 1 TO 5\n20 PRINT 3 * I\n30 NEXT I\n40 END\n"
        )
        assert result.output == "3\n6\n9\n12\n15\n"

    def test_countdown(self) -> None:
        result = run_basic(
            "10 LET N = 5\n20 PRINT N\n30 LET N = N - 1\n"
            "40 IF N > 0 THEN 20\n50 END\n"
        )
        assert result.output == "5\n4\n3\n2\n1\n"

    def test_collatz_steps_for_6(self) -> None:
        result = run_basic(
            "10 LET N = 6\n20 LET S = 0\n"
            "30 IF N = 1 THEN 80\n"
            "40 LET R = N / 2\n"
            "50 LET R = R * 2\n"
            "60 IF N = R THEN 70\n"
            "61 LET N = 3 * N + 1\n62 GOTO 65\n"
            "70 LET N = N / 2\n"
            "65 LET S = S + 1\n"
            "66 GOTO 30\n"
            "80 PRINT S\n90 END\n"
        )
        assert result.output == "8\n"

    def test_rem_is_comment(self) -> None:
        result = run_basic(
            "10 REM THIS IS A COMMENT\n20 PRINT \"OK\"\n30 END\n"
        )
        assert result.output == "OK\n"

    def test_stop_halts_like_end(self) -> None:
        result = run_basic(
            "10 PRINT \"BEFORE\"\n20 STOP\n30 PRINT \"AFTER\"\n40 END\n"
        )
        assert result.output == "BEFORE\n"

    def test_hello_world_jvm_style(self) -> None:
        result = run_basic(
            "10 PRINT \"HELLO FROM JVM\"\n20 END\n"
        )
        assert result.output == "HELLO FROM JVM\n"


# ---------------------------------------------------------------------------
# Error paths
# ---------------------------------------------------------------------------


class TestErrors:
    """BasicError is raised for unsupported features and bad input."""

    def test_gosub_raises(self) -> None:
        with pytest.raises(BasicError):
            run_basic("10 GOSUB 100\n20 END\n100 RETURN\n")

    def test_unsupported_char_raises(self) -> None:
        with pytest.raises(BasicError):
            run_basic("10 PRINT \"@#$\"\n20 END\n")

    def test_parse_error_wraps_as_basic_error(self) -> None:
        """A parser failure in stage 1 is wrapped as BasicError."""
        with patch("dartmouth_basic_parser.parse_dartmouth_basic", side_effect=ValueError("bad input")):
            with pytest.raises(BasicError, match="parse error"):
                run_basic("10 END\n")

    def test_jvm_lowering_error_wraps_as_basic_error(self) -> None:
        """A JvmBackendError in stage 3 is wrapped as BasicError."""
        with patch(
            "ir_to_jvm_class_file.lower_ir_to_jvm_class_file",
            side_effect=RuntimeError("lowering failed"),
        ):
            with pytest.raises(BasicError):
                run_basic("10 END\n")

    def test_runtime_error_wraps_as_basic_error(self) -> None:
        """An unexpected exception from the JVM runtime in stage 4 is wrapped as BasicError."""
        with patch(
            "jvm_runtime.JVMRuntime.run_method",
            side_effect=RuntimeError("execution fault"),
        ):
            with pytest.raises(BasicError, match="runtime error"):
                run_basic("10 END\n")

    def test_basic_error_from_runtime_propagates_unchanged(self) -> None:
        """A BasicError raised during stage 4 is re-raised without wrapping."""
        original = BasicError("inner error")
        with patch(
            "jvm_runtime.JVMRuntime.run_method",
            side_effect=original,
        ):
            with pytest.raises(BasicError) as exc_info:
                run_basic("10 END\n")
        assert exc_info.value is original


# ---------------------------------------------------------------------------
# RunResult fields
# ---------------------------------------------------------------------------


class TestRunResult:
    """RunResult has the expected field values for the JVM backend."""

    def test_output_is_string(self) -> None:
        result = run_basic("10 PRINT \"HI\"\n20 END\n")
        assert isinstance(result.output, str)

    def test_var_values_is_empty(self) -> None:
        result = run_basic("10 LET A = 5\n20 END\n")
        assert result.var_values == {}

    def test_steps_is_zero(self) -> None:
        result = run_basic("10 END\n")
        assert result.steps == 0

    def test_halt_address_is_zero(self) -> None:
        result = run_basic("10 END\n")
        assert result.halt_address == 0
