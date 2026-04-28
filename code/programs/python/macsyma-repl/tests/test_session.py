"""End-to-end MACSYMA REPL session tests.

Drive the language plugin via ``Repl.run_with_io`` so no real terminal
is needed. Each test queues up a list of inputs, captures the outputs,
and verifies the recorded transcript.
"""

from __future__ import annotations

import sys
from pathlib import Path

# Add the program's root to sys.path so language/prompt/main are importable
# under pytest, which runs from the package's ``tests/`` directory.
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from coding_adventures_repl import Repl  # noqa: E402

from language import MacsymaLanguage  # noqa: E402
from prompt import MacsymaPrompt  # noqa: E402


def _run(inputs: list[str]) -> list[str]:
    """Drive a session with the given input lines; return collected outputs.

    The framework writes the prompt followed by a newline before each
    read, then writes each ``ok``/``error`` value on its own line.
    Synchronous mode keeps tests fast and deterministic.
    """
    language = MacsymaLanguage()
    prompt = MacsymaPrompt(history=language.history)
    queue = iter(inputs)
    outputs: list[str] = []

    def input_fn() -> str | None:
        return next(queue, None)

    Repl.run_with_io(
        language=language,
        prompt=prompt,
        input_fn=input_fn,
        output_fn=outputs.append,
        mode="sync",
    )
    return outputs


# ---------------------------------------------------------------------------
# Basic arithmetic and persistence
# ---------------------------------------------------------------------------


def test_simple_arithmetic() -> None:
    out = _run(["2 + 3;", ":quit"])
    # Output sequence: "(%i1) " (prompt), "(%o1) 5" (result), "(%i2) " (next prompt)
    assert "(%o1) 5" in out


def test_variable_persistence_across_turns() -> None:
    out = _run(["x: 5$", "x + 1;", ":quit"])
    # x:5$ suppresses output. Next turn shows (%o2) 6.
    assert any(line == "(%o2) 6" for line in out)


def test_function_definition_and_call() -> None:
    out = _run(["f(x) := x^2$", "f(3);", ":quit"])
    assert any(line == "(%o2) 9" for line in out)


# ---------------------------------------------------------------------------
# Display / Suppress
# ---------------------------------------------------------------------------


def test_dollar_suppresses_output() -> None:
    """``42$`` records to history but emits no displayed line."""
    out = _run(["42$", ":quit"])
    # No (%o1) line should appear.
    assert not any(line.startswith("(%o1) ") for line in out)


def test_semicolon_displays_output() -> None:
    out = _run(["42;", ":quit"])
    assert any(line == "(%o1) 42" for line in out)


def test_mixed_terminators_in_one_line() -> None:
    """``a:1$ a + 2;`` — first stmt suppressed, second displayed."""
    out = _run(["a:1$ a + 2;", ":quit"])
    # The first statement's output is suppressed; the second shows (%o2) 3.
    assert any(line == "(%o2) 3" for line in out)


# ---------------------------------------------------------------------------
# History references
# ---------------------------------------------------------------------------


def test_percent_resolves_to_last_output() -> None:
    out = _run(["2 + 3;", "% * 2;", ":quit"])
    assert any(line == "(%o2) 10" for line in out)


def test_percent_oN_resolves_named_output() -> None:
    out = _run(["10;", "20;", "%o1 + %o2;", ":quit"])
    assert any(line == "(%o3) 30" for line in out)


# ---------------------------------------------------------------------------
# Quit and error handling
# ---------------------------------------------------------------------------


def test_colon_quit_ends_session() -> None:
    """``:quit`` ends the session immediately."""
    out = _run([":quit"])
    # No output expected besides the initial prompt.
    assert all(not line.startswith("(%o") for line in out)


def test_quit_keyword_ends_session() -> None:
    out = _run(["quit;"])
    assert all(not line.startswith("(%o") for line in out)


def test_parse_error_does_not_kill_session() -> None:
    """A bad input emits an error line and the session continues."""
    out = _run(["1 +;", "2 + 3;", ":quit"])
    # An error message appears.
    assert any("Error" in line or "error" in line.lower() for line in out)
    # And the next valid input still works.
    assert any(line.endswith(") 5") for line in out)


def test_blank_line_is_ignored() -> None:
    out = _run(["", "2 + 3;", ":quit"])
    assert any(line == "(%o1) 5" for line in out)


def test_auto_terminator_appended() -> None:
    """Input without a trailing ``;`` or ``$`` is treated as ``;``."""
    out = _run(["2 + 3", ":quit"])
    assert any(line == "(%o1) 5" for line in out)


# ---------------------------------------------------------------------------
# CAS operations smoke tests — full pipeline through pretty printer
# ---------------------------------------------------------------------------


def test_factor_difference_of_squares_repl() -> None:
    """factor(x^2 - 1) produces a factored form, not unevaluated Factor(…)."""
    out = _run(["factor(x^2 - 1);", ":quit"])
    result_lines = [line for line in out if line.startswith("(%o1)")]
    assert len(result_lines) == 1
    line = result_lines[0]
    # Must not come back as the unevaluated head.
    assert "Factor(" not in line
    # Must contain a * (product of two factors).
    assert "*" in line


def test_diff_monomial_repl() -> None:
    """diff(x^2, x) → 2*x (not the unevaluated D(…))."""
    out = _run(["diff(x^2, x);", ":quit"])
    result_lines = [line for line in out if line.startswith("(%o1)")]
    assert len(result_lines) == 1
    line = result_lines[0]
    assert "D(" not in line
    assert "2" in line and "x" in line


def test_solve_linear_repl() -> None:
    """solve(2*x - 4, x) → [2]."""
    out = _run(["solve(2*x - 4, x);", ":quit"])
    result_lines = [line for line in out if line.startswith("(%o1)")]
    assert len(result_lines) == 1
    line = result_lines[0]
    assert "2" in line


def test_integrate_power_repl() -> None:
    """integrate(x, x) → x^2/2 or 1/2*x^2 (power rule, not unevaluated)."""
    out = _run(["integrate(x, x);", ":quit"])
    result_lines = [line for line in out if line.startswith("(%o1)")]
    assert len(result_lines) == 1
    line = result_lines[0]
    assert "Integrate(" not in line
    assert "x" in line


def test_limit_polynomial_repl() -> None:
    """limit(x^2 + 1, x, 2) → 5."""
    out = _run(["limit(x^2 + 1, x, 2);", ":quit"])
    result_lines = [line for line in out if line.startswith("(%o1)")]
    assert len(result_lines) == 1
    assert "5" in result_lines[0]


def test_simplify_repl() -> None:
    """simplify(x + 0) → x (identity elimination)."""
    out = _run(["simplify(x + 0);", ":quit"])
    result_lines = [line for line in out if line.startswith("(%o1)")]
    assert len(result_lines) == 1
    line = result_lines[0]
    assert "x" in line


# ---------------------------------------------------------------------------
# Regression tests for REPL quality fixes
# ---------------------------------------------------------------------------


def test_is_prime_alias_repl() -> None:
    """is_prime(17) → True  (alias for primep, not unevaluated)."""
    out = _run(["is_prime(17);", ":quit"])
    result_lines = [line for line in out if line.startswith("(%o1)")]
    assert len(result_lines) == 1
    assert "True" in result_lines[0]


def test_map_lambda_repl() -> None:
    """map(lambda([z], z^2), [1, 2, 3]) → [1, 4, 9]."""
    out = _run(["map(lambda([z], z^2), [1, 2, 3]);", ":quit"])
    result_lines = [line for line in out if line.startswith("(%o1)")]
    assert len(result_lines) == 1
    line = result_lines[0]
    # Must not come back as the unevaluated Map(lambda(...), ...) form.
    assert "lambda" not in line
    assert "1" in line and "4" in line and "9" in line


def test_taylor_sin_repl() -> None:
    """taylor(sin(y), y, 0, 3) produces a polynomial, not unevaluated."""
    out = _run(["taylor(sin(y), y, 0, 3);", ":quit"])
    result_lines = [line for line in out if line.startswith("(%o1)")]
    assert len(result_lines) == 1
    line = result_lines[0]
    # Must not come back as the unevaluated Taylor(sin(y), y, 0, 3).
    assert "Taylor(" not in line
    assert "y" in line  # Should contain y (the linear/cubic terms)


def test_diff_product_pretty_output() -> None:
    """diff(sin(y)*cos(y), y) uses subtraction, not ``+`` followed by a minus."""
    out = _run(["diff(sin(y)*cos(y), y);", ":quit"])
    result_lines = [line for line in out if line.startswith("(%o1)")]
    assert len(result_lines) == 1
    line = result_lines[0]
    # The result should contain subtraction somewhere.
    assert " - " in line, f"Expected subtraction in output, got: {line!r}"
    # Should not contain the ugly 'f*-g' pattern.
    assert "*-" not in line, f"Unexpected raw '*-' in output: {line!r}"


# ---------------------------------------------------------------------------
# Phase G — control flow (while / for / block / return / if)
# ---------------------------------------------------------------------------


class TestPhaseGControlFlow:
    """End-to-end REPL tests for Phase G grammar-level control-flow keywords.

    These constructs are compiled by the Phase G grammar and evaluated by the
    Phase G VM handlers in ``symbolic-vm`` 0.32.0. No name-table additions are
    needed; the keywords are handled at the grammar / compiler layer.
    """

    def test_while_loop_sum(self) -> None:
        """``while s < 5 do s: s + 1`` increments s from 0 to 5."""
        out = _run(["s: 0$", "while s < 5 do s: s + 1;", ":quit"])
        combined = "\n".join(out)
        assert "5" in combined

    def test_for_range_sum(self) -> None:
        """``for i thru 5 do s:s+i`` inside a block sums 0+1+2+3+4+5 = 15."""
        out = _run(["block([s:0], for i thru 5 do s:s+i, s);", ":quit"])
        combined = "\n".join(out)
        assert "15" in combined

    def test_for_each_applies_body(self) -> None:
        """``for x in [1,2,3] do s:s+x`` accumulates to 6."""
        out = _run(["block([s:0], for x in [1,2,3] do s:s+x, s);", ":quit"])
        combined = "\n".join(out)
        assert "6" in combined

    def test_if_then_else_true(self) -> None:
        """``if 2 > 1 then 99 else 0`` takes the true branch → 99."""
        out = _run(["if 2 > 1 then 99 else 0;", ":quit"])
        result_lines = [line for line in out if line.startswith("(%o1)")]
        assert len(result_lines) == 1
        assert "99" in result_lines[0]

    def test_if_then_else_false(self) -> None:
        """``if 1 > 2 then 99 else 0`` takes the false branch → 0."""
        out = _run(["if 1 > 2 then 99 else 0;", ":quit"])
        result_lines = [line for line in out if line.startswith("(%o1)")]
        assert len(result_lines) == 1
        assert "0" in result_lines[0]

    def test_if_then_no_else_miss(self) -> None:
        """``if false then 99`` with no else clause → false (unmatched branch)."""
        out = _run(["if false then 99;", ":quit"])
        combined = "\n".join(out)
        assert "false" in combined.lower()

    def test_block_local_scope(self) -> None:
        """Local ``x:99`` inside a block does not overwrite the outer ``x:10``."""
        out = _run(["x: 10$", "block([x:99], x);", "x;", ":quit"])
        # (%o2) must be 99 (inside block), (%o3) must be 10 (outer scope restored).
        assert any("(%o2)" in line and "99" in line for line in out), (
            f"Expected (%o2) 99 in output; got: {out}"
        )
        assert any("(%o3)" in line and "10" in line for line in out), (
            f"Expected (%o3) 10 in output; got: {out}"
        )

    def test_return_from_block(self) -> None:
        """``return(42)`` inside a block short-circuits the block to 42."""
        out = _run(["block([x:1], x:2, return(42), x);", ":quit"])
        result_lines = [line for line in out if line.startswith("(%o1)")]
        assert len(result_lines) == 1
        assert "42" in result_lines[0]


# ---------------------------------------------------------------------------
# Phase 13 — hyperbolic functions (sinh / cosh / tanh and calculus)
# ---------------------------------------------------------------------------


class TestPhase13Hyperbolic:
    """End-to-end REPL tests for Phase 13 hyperbolic functions.

    ``sinh``, ``cosh``, ``tanh``, ``asinh``, ``acosh``, ``atanh`` were added
    to the symbolic VM in 0.32.0.  The compiler's ``_STANDARD_FUNCTIONS``
    already maps these names to canonical IR heads, so no name-table additions
    are required in ``macsyma-runtime``.
    """

    def test_sinh_zero(self) -> None:
        """``sinh(0)`` evaluates to the exact integer 0."""
        out = _run(["sinh(0);", ":quit"])
        result_lines = [line for line in out if line.startswith("(%o1)")]
        assert len(result_lines) == 1
        assert "0" in result_lines[0]

    def test_cosh_zero(self) -> None:
        """``cosh(0)`` evaluates to the exact integer 1."""
        out = _run(["cosh(0);", ":quit"])
        result_lines = [line for line in out if line.startswith("(%o1)")]
        assert len(result_lines) == 1
        assert "1" in result_lines[0]

    def test_tanh_zero(self) -> None:
        """``tanh(0)`` evaluates to the exact integer 0."""
        out = _run(["tanh(0);", ":quit"])
        result_lines = [line for line in out if line.startswith("(%o1)")]
        assert len(result_lines) == 1
        assert "0" in result_lines[0]

    def test_sinh_numeric(self) -> None:
        """``ev(sinh(1), numer)`` → a decimal close to 1.1752."""
        out = _run(["ev(sinh(1), numer);", ":quit"])
        result_lines = [line for line in out if line.startswith("(%o1)")]
        assert len(result_lines) == 1
        # sinh(1) ≈ 1.1752011936438014 — check the leading digits appear.
        assert "1.1" in result_lines[0]

    def test_diff_sinh(self) -> None:
        """``diff(sinh(x), x)`` → cosh(x) (derivative of sinh is cosh)."""
        out = _run(["diff(sinh(x), x);", ":quit"])
        result_lines = [line for line in out if line.startswith("(%o1)")]
        assert len(result_lines) == 1
        assert "cosh" in result_lines[0].lower()

    def test_integrate_sinh(self) -> None:
        """``integrate(sinh(x), x)`` → cosh(x) (antiderivative of sinh is cosh)."""
        out = _run(["integrate(sinh(x), x);", ":quit"])
        result_lines = [line for line in out if line.startswith("(%o1)")]
        assert len(result_lines) == 1
        assert "cosh" in result_lines[0].lower()
