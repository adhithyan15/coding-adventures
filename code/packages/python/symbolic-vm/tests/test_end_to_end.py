"""End-to-end integration tests — MACSYMA source → evaluated result.

These tests exercise the complete pipeline:

    source text
        → macsyma-lexer (GrammarLexer)
        → macsyma-parser (GrammarParser → ASTNode)
        → macsyma-compiler (ASTNode → symbolic IR)
        → symbolic-vm (IR → IR via a Backend)

Each test constructs a MACSYMA source string and asserts on the final
evaluated IR. The goal is not to stress any one component; it's to
prove the whole stack composes as intended.
"""

from __future__ import annotations

import pytest
from macsyma_compiler import compile_macsyma
from macsyma_parser import parse_macsyma
from symbolic_ir import (
    COS,
    IRApply,
    IRFloat,
    IRInteger,
    IRRational,
    IRSymbol,
)

from symbolic_vm import VM, StrictBackend, SymbolicBackend


def run_strict(source: str):
    """Compile and evaluate ``source`` in strict mode; return final value."""
    statements = compile_macsyma(parse_macsyma(source))
    return VM(StrictBackend()).eval_program(statements)


def run_symbolic(source: str):
    """Compile and evaluate ``source`` in symbolic mode; return final value."""
    statements = compile_macsyma(parse_macsyma(source))
    return VM(SymbolicBackend()).eval_program(statements)


# ---------------------------------------------------------------------------
# Calculator mode (StrictBackend)
# ---------------------------------------------------------------------------


def test_strict_simple_arithmetic() -> None:
    assert run_strict("1 + 2 * 3;") == IRInteger(7)


def test_strict_parentheses() -> None:
    assert run_strict("(1 + 2) * 3;") == IRInteger(9)


def test_strict_rationals_stay_exact() -> None:
    assert run_strict("1 / 2 + 1 / 3;") == IRRational(5, 6)


def test_strict_float_contamination() -> None:
    assert run_strict("1 + 0.5;") == IRFloat(1.5)


def test_strict_assignment_and_use() -> None:
    result = run_strict("a : 3; b : 4; a^2 + b^2;")
    assert result == IRInteger(25)


def test_strict_user_function() -> None:
    source = "square(x) := x^2; square(6);"
    assert run_strict(source) == IRInteger(36)


def test_strict_user_function_two_args() -> None:
    source = "hyp(a, b) := a^2 + b^2; hyp(3, 4);"
    assert run_strict(source) == IRInteger(25)


def test_strict_undefined_raises() -> None:
    with pytest.raises(NameError):
        run_strict("x + 1;")


def test_strict_comparison() -> None:
    assert run_strict("5 > 3;") == IRSymbol("True")


# ---------------------------------------------------------------------------
# Symbolic mode (SymbolicBackend)
# ---------------------------------------------------------------------------


def test_symbolic_free_variable_survives() -> None:
    # x is never bound, so it stays symbolic.
    result = run_symbolic("x + 0;")
    assert result == IRSymbol("x")


def test_symbolic_identity_collapses() -> None:
    # x * 1 → x, x + 0 → x
    result = run_symbolic("x * 1 + 0;")
    assert result == IRSymbol("x")


def test_symbolic_partial_numeric_fold() -> None:
    # x + (2 + 3) → x + 5
    from symbolic_ir import ADD

    result = run_symbolic("x + (2 + 3);")
    assert result == IRApply(ADD, (IRSymbol("x"), IRInteger(5)))


def test_symbolic_differentiation_polynomial() -> None:
    # diff(x^2, x) → 2*x
    from symbolic_ir import MUL

    result = run_symbolic("diff(x^2, x);")
    assert result == IRApply(MUL, (IRInteger(2), IRSymbol("x")))


def test_symbolic_differentiation_trig() -> None:
    # diff(sin(x), x) → cos(x)
    result = run_symbolic("diff(sin(x), x);")
    assert result == IRApply(COS, (IRSymbol("x"),))


def test_symbolic_function_then_diff() -> None:
    # f(x) := x^3; diff(f(x), x) → 3*x^2
    from symbolic_ir import MUL, POW

    source = "f(x) := x^3; diff(f(x), x);"
    result = run_symbolic(source)
    assert result == IRApply(
        MUL,
        (IRInteger(3), IRApply(POW, (IRSymbol("x"), IRInteger(2)))),
    )


def test_symbolic_mixed_program() -> None:
    # Assign a numeric, define a function, then apply both.
    # k : 10; f(x) := k * x; f(3)  →  30
    source = "k : 10; f(x) := k * x; f(3);"
    assert run_symbolic(source) == IRInteger(30)


def test_symbolic_integrate_polynomial() -> None:
    # integrate(x^2, x) → (1/3)·x^3
    from symbolic_ir import MUL, POW, IRRational

    result = run_symbolic("integrate(x^2, x);")
    expected = IRApply(
        MUL,
        (
            IRRational(1, 3),
            IRApply(POW, (IRSymbol("x"), IRInteger(3))),
        ),
    )
    assert result == expected


def test_symbolic_integrate_trig() -> None:
    # integrate(sin(x), x) → -cos(x)
    from symbolic_ir import NEG

    result = run_symbolic("integrate(sin(x), x);")
    assert result == IRApply(NEG, (IRApply(COS, (IRSymbol("x"),)),))


def test_symbolic_integrate_then_diff_exp() -> None:
    # diff(integrate(exp(x), x), x) → exp(x)  — fundamental theorem
    # works cleanly for functions whose derivative chain doesn't leave
    # rational-coefficient cancellation behind.
    from symbolic_ir import EXP

    result = run_symbolic("diff(integrate(exp(x), x), x);")
    assert result == IRApply(EXP, (IRSymbol("x"),))
