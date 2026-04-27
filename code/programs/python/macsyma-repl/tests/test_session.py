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
