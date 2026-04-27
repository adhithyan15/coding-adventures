"""End-to-end tests for the Dartmouth BASIC → GE-225 compiled pipeline.

Every test compiles BASIC source text and executes it on the GE-225 simulator.
The tests exercise the full pipeline:

  BASIC source
      → dartmouth_basic_parser      (AST)
      → dartmouth_basic_ir_compiler (IrProgram)
      → ir_to_ge225_compiler        (GE-225 binary)
      → ge225_simulator             (execution)

This is exactly the experience that Dartmouth students had in 1964:
type a program, press RETURN, receive output within seconds.
"""

from __future__ import annotations

import pytest

from dartmouth_basic_ge225_compiler import BasicError, RunResult, run_basic


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def output(source: str) -> str:
    """Compile and run source; return the typewriter output string."""
    return run_basic(source).output


def result(source: str) -> RunResult:
    """Compile and run source; return the full RunResult."""
    return run_basic(source)


# ---------------------------------------------------------------------------
# LET — variable assignment and arithmetic
# ---------------------------------------------------------------------------


class TestLet:
    """Variables are stored in spill slots; we verify via var_values."""

    def test_let_constant(self) -> None:
        r = result("10 LET A = 42\n20 END\n")
        assert r.var_values["A"] == 42

    def test_let_addition(self) -> None:
        r = result("10 LET A = 3 + 4\n20 END\n")
        assert r.var_values["A"] == 7

    def test_let_subtraction(self) -> None:
        r = result("10 LET A = 10 - 3\n20 END\n")
        assert r.var_values["A"] == 7

    def test_let_multiplication(self) -> None:
        r = result("10 LET A = 6 * 7\n20 END\n")
        assert r.var_values["A"] == 42

    def test_let_division(self) -> None:
        r = result("10 LET A = 100 / 4\n20 END\n")
        assert r.var_values["A"] == 25

    def test_let_chained_variables(self) -> None:
        r = result("10 LET A = 5\n20 LET B = A * 2\n30 LET C = B + A\n40 END\n")
        assert r.var_values["A"] == 5
        assert r.var_values["B"] == 10
        assert r.var_values["C"] == 15

    def test_let_negative_result(self) -> None:
        r = result("10 LET A = 3 - 10\n20 END\n")
        assert r.var_values["A"] == -7

    def test_let_unary_minus(self) -> None:
        r = result("10 LET A = -9\n20 END\n")
        assert r.var_values["A"] == -9

    def test_let_complex_expression(self) -> None:
        # (2 + 3) * (10 - 4) = 5 * 6 = 30
        r = result("10 LET A = (2 + 3) * (10 - 4)\n20 END\n")
        assert r.var_values["A"] == 30

    def test_let_overwrites_previous_value(self) -> None:
        r = result("10 LET A = 1\n20 LET A = 99\n30 END\n")
        assert r.var_values["A"] == 99


# ---------------------------------------------------------------------------
# PRINT — string output
# ---------------------------------------------------------------------------


class TestPrintString:
    """PRINT with string literals produces typewriter output."""

    def test_hello_world(self) -> None:
        assert output("10 PRINT \"HELLO WORLD\"\n20 END\n") == "HELLO WORLD\n"

    def test_print_appends_newline(self) -> None:
        # Each PRINT ends with GE-225 carriage return → converted to \n
        assert output("10 PRINT \"A\"\n20 PRINT \"B\"\n30 END\n") == "A\nB\n"

    def test_print_empty_string(self) -> None:
        # PRINT "" emits just the carriage return
        assert output("10 PRINT \"\"\n20 END\n") == "\n"

    def test_print_bare(self) -> None:
        # Bare PRINT with no argument emits only the carriage return
        assert output("10 PRINT\n20 END\n") == "\n"

    def test_print_digits_as_string(self) -> None:
        assert output("10 PRINT \"123\"\n20 END\n") == "123\n"

    def test_print_lowercase_uppercased(self) -> None:
        # GE-225 typewriter is uppercase only; lowercase is promoted
        assert output("10 PRINT \"hello\"\n20 END\n") == "HELLO\n"

    def test_print_multiple_lines(self) -> None:
        src = "10 PRINT \"LINE 1\"\n20 PRINT \"LINE 2\"\n30 PRINT \"LINE 3\"\n40 END\n"
        assert output(src) == "LINE 1\nLINE 2\nLINE 3\n"


# ---------------------------------------------------------------------------
# PRINT — numeric output
# ---------------------------------------------------------------------------


class TestPrintNumeric:
    """PRINT with numeric expressions prints their decimal representation."""

    def test_print_zero(self) -> None:
        assert output("10 PRINT 0\n20 END\n") == "0\n"

    def test_print_positive_integer(self) -> None:
        assert output("10 PRINT 42\n20 END\n") == "42\n"

    def test_print_negative_integer(self) -> None:
        assert output("10 LET X = -7\n20 PRINT X\n30 END\n") == "-7\n"

    def test_print_variable(self) -> None:
        assert output("10 LET A = 99\n20 PRINT A\n30 END\n") == "99\n"

    def test_print_expression_result(self) -> None:
        assert output("10 PRINT 3 + 4\n20 END\n") == "7\n"

    def test_print_large_number(self) -> None:
        assert output("10 LET X = 12345\n20 PRINT X\n30 END\n") == "12345\n"

    def test_print_max_20bit(self) -> None:
        # Maximum positive 20-bit signed integer: 2^19 - 1 = 524287
        assert output("10 LET X = 524287\n20 PRINT X\n30 END\n") == "524287\n"

    def test_print_powers_of_ten(self) -> None:
        for power, expected in [(1, "1"), (10, "10"), (100, "100"),
                                 (1000, "1000"), (10000, "10000"), (100000, "100000")]:
            r = output(f"10 LET X = {power}\n20 PRINT X\n30 END\n")
            assert r == expected + "\n", f"failed for power {power}"

    def test_print_leading_zero_suppressed(self) -> None:
        # 7 should print as "7", not "000007"
        assert output("10 PRINT 7\n20 END\n") == "7\n"

    def test_print_mixed_string_and_number(self) -> None:
        # PRINT "LABEL", value on one line
        src = "10 LET N = 99\n20 PRINT \"N IS \", N\n30 END\n"
        assert output(src) == "N IS 99\n"

    def test_print_arithmetic_in_print(self) -> None:
        # 6 * 7 = 42
        src = "10 LET X = 6\n20 LET Y = 7\n30 PRINT \"PRODUCT IS \", X * Y\n40 END\n"
        assert output(src) == "PRODUCT IS 42\n"


# ---------------------------------------------------------------------------
# FOR / NEXT loops
# ---------------------------------------------------------------------------


class TestForNext:
    """FOR/NEXT loops with variable bounds, steps, and accumulation."""

    def test_for_prints_sequence(self) -> None:
        src = "10 FOR I = 1 TO 5\n20 PRINT I\n30 NEXT I\n40 END\n"
        assert output(src) == "1\n2\n3\n4\n5\n"

    def test_for_sum_one_to_ten(self) -> None:
        src = (
            "10 LET S = 0\n"
            "20 FOR I = 1 TO 10\n"
            "30 LET S = S + I\n"
            "40 NEXT I\n"
            "50 PRINT S\n"
            "60 END\n"
        )
        r = result(src)
        assert r.var_values["S"] == 55
        assert r.output == "55\n"

    def test_for_sum_one_to_hundred(self) -> None:
        src = (
            "10 LET S = 0\n"
            "20 FOR I = 1 TO 100\n"
            "30 LET S = S + I\n"
            "40 NEXT I\n"
            "50 PRINT S\n"
            "60 END\n"
        )
        assert result(src).var_values["S"] == 5050

    def test_for_with_step(self) -> None:
        # 0, 2, 4, 6, 8 → sum = 20
        src = (
            "10 LET S = 0\n"
            "20 FOR I = 0 TO 8 STEP 2\n"
            "30 LET S = S + I\n"
            "40 NEXT I\n"
            "50 PRINT S\n"
            "60 END\n"
        )
        assert result(src).var_values["S"] == 20

    def test_nested_loops_multiplication_table_cell(self) -> None:
        # Compute 3 × 4 = 12 via nested loop counting
        src = (
            "10 LET P = 0\n"
            "20 FOR I = 1 TO 3\n"
            "30 FOR J = 1 TO 4\n"
            "40 LET P = P + 1\n"
            "50 NEXT J\n"
            "60 NEXT I\n"
            "70 PRINT P\n"
            "80 END\n"
        )
        assert result(src).var_values["P"] == 12

    def test_for_loop_zero_iterations(self) -> None:
        # FOR I = 5 TO 1: body never executes, S stays 0
        src = "10 LET S = 0\n20 FOR I = 5 TO 1\n30 LET S = 99\n40 NEXT I\n50 END\n"
        assert result(src).var_values["S"] == 0

    def test_factorial_5(self) -> None:
        src = (
            "10 LET F = 1\n"
            "20 FOR I = 1 TO 5\n"
            "30 LET F = F * I\n"
            "40 NEXT I\n"
            "50 PRINT F\n"
            "60 END\n"
        )
        r = result(src)
        assert r.var_values["F"] == 120
        assert r.output == "120\n"


# ---------------------------------------------------------------------------
# IF / THEN conditionals
# ---------------------------------------------------------------------------


class TestIfThen:
    """IF with all six relational operators."""

    def test_if_eq_true_branches(self) -> None:
        src = "10 LET A = 5\n20 IF A = 5 THEN 40\n30 LET A = 0\n40 END\n"
        assert result(src).var_values["A"] == 5

    def test_if_eq_false_falls_through(self) -> None:
        src = "10 LET A = 5\n20 IF A = 9 THEN 40\n30 LET A = 0\n40 END\n"
        assert result(src).var_values["A"] == 0

    def test_if_lt_true(self) -> None:
        src = "10 LET R = 0\n20 IF 3 < 5 THEN 40\n30 LET R = 1\n40 END\n"
        assert result(src).var_values["R"] == 0

    def test_if_gt_true(self) -> None:
        src = "10 LET R = 0\n20 IF 10 > 3 THEN 40\n30 LET R = 1\n40 END\n"
        assert result(src).var_values["R"] == 0

    def test_if_ne_true(self) -> None:
        src = "10 LET A = 7\n20 IF A <> 5 THEN 40\n30 LET A = 0\n40 END\n"
        assert result(src).var_values["A"] == 7

    def test_if_le_true(self) -> None:
        src = "10 LET R = 0\n20 IF 5 <= 5 THEN 40\n30 LET R = 1\n40 END\n"
        assert result(src).var_values["R"] == 0

    def test_if_ge_true(self) -> None:
        src = "10 LET R = 0\n20 IF 6 >= 5 THEN 40\n30 LET R = 1\n40 END\n"
        assert result(src).var_values["R"] == 0

    def test_if_used_to_implement_max(self) -> None:
        src = (
            "10 LET A = 3\n"
            "20 LET B = 7\n"
            "30 LET M = A\n"
            "40 IF B > A THEN 60\n"
            "50 GOTO 70\n"
            "60 LET M = B\n"
            "70 END\n"
        )
        assert result(src).var_values["M"] == 7


# ---------------------------------------------------------------------------
# GOTO
# ---------------------------------------------------------------------------


class TestGoto:
    """GOTO for unconditional jumps."""

    def test_goto_skips_code(self) -> None:
        src = "10 GOTO 30\n20 LET A = 99\n30 END\n"
        assert result(src).var_values["A"] == 0

    def test_goto_backward_counts(self) -> None:
        # Manual countdown loop via GOTO
        src = (
            "10 LET N = 5\n"
            "20 LET N = N - 1\n"
            "30 IF N > 0 THEN 20\n"
            "40 END\n"
        )
        assert result(src).var_values["N"] == 0


# ---------------------------------------------------------------------------
# Classic BASIC programs
# ---------------------------------------------------------------------------


class TestClassicPrograms:
    """End-to-end tests with canonical Dartmouth BASIC programs."""

    def test_fibonacci_first_ten(self) -> None:
        """Compute Fibonacci numbers F(1)..F(10) and verify F(10) = 55."""
        src = (
            "10 LET A = 1\n"
            "20 LET B = 1\n"
            "30 FOR I = 3 TO 10\n"
            "40 LET C = A + B\n"
            "50 LET A = B\n"
            "60 LET B = C\n"
            "70 NEXT I\n"
            "80 PRINT B\n"
            "90 END\n"
        )
        r = result(src)
        assert r.var_values["B"] == 55
        assert r.output == "55\n"

    def test_gauss_sum(self) -> None:
        """The young Gauss story: sum 1..100 = 5050."""
        src = (
            "10 LET S = 0\n"
            "20 FOR I = 1 TO 100\n"
            "30 LET S = S + I\n"
            "40 NEXT I\n"
            "50 PRINT S\n"
            "60 END\n"
        )
        assert result(src).output == "5050\n"

    def test_multiplication_table_row(self) -> None:
        """Print 3 × 1 through 3 × 5."""
        src = (
            "10 LET N = 3\n"
            "20 FOR I = 1 TO 5\n"
            "30 PRINT N * I\n"
            "40 NEXT I\n"
            "50 END\n"
        )
        assert result(src).output == "3\n6\n9\n12\n15\n"

    def test_countdown(self) -> None:
        """Count down from 5 to 1 using IF/THEN/GOTO."""
        src = (
            "10 LET I = 5\n"
            "20 PRINT I\n"
            "30 LET I = I - 1\n"
            "40 IF I > 0 THEN 20\n"
            "50 END\n"
        )
        assert result(src).output == "5\n4\n3\n2\n1\n"

    def test_collatz_steps_for_6(self) -> None:
        """
        Collatz (hailstone) sequence from 6: 6→3→10→5→16→8→4→2→1.
        Count steps until reaching 1.

        In BASIC without MOD we use: oddness via (N / 2) * 2 <> N trick.
        Instead we implement a simplified version: count steps for N=6.
        """
        # N=6: steps = 6,3,10,5,16,8,4,2,1 → 8 steps to reach 1
        src = (
            "10 LET N = 6\n"
            "20 LET K = 0\n"
            "30 IF N = 1 THEN 100\n"
            "40 LET H = N / 2\n"
            "50 LET E = H * 2\n"
            "60 IF E = N THEN 80\n"
            "70 LET N = 3 * N + 1\n"
            "75 LET K = K + 1\n"
            "76 GOTO 30\n"
            "80 LET N = N / 2\n"
            "85 LET K = K + 1\n"
            "90 GOTO 30\n"
            "100 PRINT K\n"
            "110 END\n"
        )
        r = result(src)
        assert r.output == "8\n"

    def test_rem_is_comment(self) -> None:
        """REM statements produce no output and do not affect variables."""
        src = (
            "10 REM THIS IS A COMMENT\n"
            "20 LET A = 5\n"
            "30 REM ANOTHER REMARK\n"
            "40 END\n"
        )
        r = result(src)
        assert r.var_values["A"] == 5
        assert r.output == ""

    def test_stop_halts_like_end(self) -> None:
        """STOP halts execution; code after it is never reached."""
        src = "10 LET A = 1\n20 STOP\n30 LET A = 99\n"
        assert result(src).var_values["A"] == 1

    def test_hello_world_1964_style(self) -> None:
        """The spirit of Kemeny & Kurtz's original Dartmouth BASIC demo."""
        src = (
            "10 PRINT \"HELLO FROM THE GE-225\"\n"
            "20 PRINT \"DARTMOUTH BASIC 1964\"\n"
            "30 END\n"
        )
        assert result(src).output == "HELLO FROM THE GE-225\nDARTMOUTH BASIC 1964\n"


# ---------------------------------------------------------------------------
# Error handling
# ---------------------------------------------------------------------------


class TestErrors:
    """BasicError is raised for unsupported features and runtime failures."""

    def test_gosub_raises(self) -> None:
        with pytest.raises(BasicError, match="GOSUB"):
            run_basic("10 GOSUB 50\n20 END\n50 RETURN\n")

    def test_unsupported_char_raises(self) -> None:
        with pytest.raises(BasicError):
            run_basic("10 PRINT \"A@B\"\n20 END\n")

    def test_max_steps_raises(self) -> None:
        # Infinite loop: only 10 steps allowed
        with pytest.raises(BasicError, match="did not halt"):
            run_basic("10 GOTO 10\n", max_steps=10)

    def test_division_by_zero_raises(self) -> None:
        with pytest.raises(BasicError, match="[Zz]ero"):
            run_basic("10 LET A = 5 / 0\n20 END\n")


# ---------------------------------------------------------------------------
# RunResult metadata
# ---------------------------------------------------------------------------


class TestRunResult:
    """Verify RunResult fields beyond just output."""

    def test_steps_is_positive(self) -> None:
        r = result("10 LET A = 1\n20 END\n")
        assert r.steps > 0

    def test_halt_address_is_nonzero(self) -> None:
        # The halt stub lives after the TON prologue, so address > 0
        r = result("10 END\n")
        assert r.halt_address > 0

    def test_all_variables_initialised_to_zero(self) -> None:
        # Variables that are never written should read as 0 from the spill area
        r = result("10 END\n")
        for var in "ABCDEFGHIJKLMNOPQRSTUVWXYZ":
            assert r.var_values[var] == 0, f"expected 0 for {var}"

    def test_steps_reflects_loop_count(self) -> None:
        # A 10-iteration loop should take more steps than a 1-iteration loop
        r1 = result("10 FOR I = 1 TO 1\n20 NEXT I\n30 END\n")
        r10 = result("10 FOR I = 1 TO 10\n20 NEXT I\n30 END\n")
        assert r10.steps > r1.steps
