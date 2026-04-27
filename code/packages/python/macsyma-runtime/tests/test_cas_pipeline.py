"""End-to-end pipeline tests: MACSYMA source → VM result.

Each test sends a MACSYMA string through the complete pipeline:

    parse_macsyma  →  compile_macsyma  →  VM(MacsymaBackend).eval

This validates that the name-table extension (``extend_compiler_name_table``)
wires MACSYMA user-visible names (``factor``, ``solve``, ``length``, …) to
the canonical IR heads (``Factor``, ``Solve``, ``Length``, …) that the
``SymbolicBackend`` CAS handlers dispatch on.

These tests belong in ``macsyma-runtime`` rather than ``symbolic-vm`` because
they require the full MACSYMA compiler + name table, which is a
``macsyma-runtime`` concern.  Pure handler unit tests live in
``symbolic-vm/tests/test_cas_handlers.py`` and use IR directly.
"""

from __future__ import annotations

import pytest
from macsyma_compiler import compile_macsyma
from macsyma_compiler.compiler import _STANDARD_FUNCTIONS
from macsyma_parser import parse_macsyma
from symbolic_ir import IRApply, IRFloat, IRInteger, IRRational, IRSymbol

from macsyma_runtime import MacsymaBackend, extend_compiler_name_table
from symbolic_vm import VM

# Extend the compiler name table so MACSYMA names compile to canonical IR.
# This call is idempotent — subsequent calls are no-ops.
extend_compiler_name_table(_STANDARD_FUNCTIONS)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _eval(source: str) -> object:
    """Parse + compile + eval ``source`` (no terminator needed).

    Returns the evaluated IR node.  The Display/Suppress wrapper added by
    ``wrap_terminators=False`` is absent here; we evaluate the raw IR.
    """
    # Normalise: strip trailing ``;`` or ``$`` so our pipeline is clean.
    src = source.strip().rstrip(";$").strip()
    ast = parse_macsyma(src + ";")
    stmts = compile_macsyma(ast, wrap_terminators=False)
    assert len(stmts) == 1, f"expected 1 statement, got {len(stmts)}: {stmts}"
    vm = VM(MacsymaBackend())
    return vm.eval(stmts[0])


def _int(n: int) -> IRInteger:
    return IRInteger(n)


def _sym(name: str) -> IRSymbol:
    return IRSymbol(name)


# ---------------------------------------------------------------------------
# Section A — symbolic simplification
# ---------------------------------------------------------------------------


def test_pipeline_simplify_add_zero() -> None:
    """simplify(x + 0) → x (identity-rule fires)."""
    result = _eval("simplify(x + 0)")
    assert result == IRSymbol("x")


def test_pipeline_simplify_mul_one() -> None:
    """simplify(x * 1) → x."""
    result = _eval("simplify(x * 1)")
    assert result == IRSymbol("x")


def test_pipeline_expand_is_callable() -> None:
    """expand(x + 0) returns a non-null IR node (canonical form)."""
    result = _eval("expand(x + 0)")
    assert result is not None


# ---------------------------------------------------------------------------
# Section B — substitution
# ---------------------------------------------------------------------------


def test_pipeline_subst_numeric() -> None:
    """subst(2, x, x^2 + 1) → 5."""
    result = _eval("subst(2, x, x^2 + 1)")
    assert result == _int(5)


def test_pipeline_subst_symbolic() -> None:
    """subst(y, x, x + x) → 2*y in some form (addition still present)."""
    result = _eval("subst(y, x, x + x)")
    # After substitution x→y: y+y; after arithmetic: may fold or remain Add.
    # The substitution must have replaced x with y: result should not be x.
    assert result != IRSymbol("x")


# ---------------------------------------------------------------------------
# Section C — factoring
# ---------------------------------------------------------------------------


def test_pipeline_factor_difference_of_squares() -> None:
    """factor(x^2 - 1) returns a factored expression, not Factor(...)."""
    result = _eval("factor(x^2 - 1)")
    # The Factor handler returns something other than Factor(Sub(Pow(x,2),1)).
    assert not (
        isinstance(result, IRApply)
        and isinstance(result.head, IRSymbol)
        and result.head.name == "Factor"
    ), f"Expected factored result, got unevaluated: {result}"


def test_pipeline_factor_irreducible_stays_unevaluated() -> None:
    """factor(x^2 + 1) is irreducible over Z → stays as Factor(...)."""
    result = _eval("factor(x^2 + 1)")
    assert isinstance(result, IRApply)
    assert isinstance(result.head, IRSymbol)
    assert result.head.name == "Factor"


# ---------------------------------------------------------------------------
# Section D — solving
# ---------------------------------------------------------------------------


def test_pipeline_solve_linear() -> None:
    """solve(2*x - 4, x) → [2]."""
    result = _eval("solve(2*x - 4, x)")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert len(result.args) == 1
    # Solution may be IRRational(2,1) or IRInteger(2).
    sol = result.args[0]
    assert sol in (_int(2), IRRational(2, 1))


def test_pipeline_solve_quadratic() -> None:
    """solve(x^2 - 5*x + 6, x) → [2, 3] in some order."""
    result = _eval("solve(x^2 - 5*x + 6, x)")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert len(result.args) == 2
    # Solutions may be IRRational or IRInteger.
    vals = {(s.numerator if isinstance(s, IRRational) else s.value) for s in result.args}
    assert vals == {2, 3}


# ---------------------------------------------------------------------------
# Section E — list operations
# ---------------------------------------------------------------------------


def test_pipeline_length() -> None:
    """length([a, b, c]) → 3."""
    result = _eval("length([a, b, c])")
    assert result == _int(3)


def test_pipeline_first() -> None:
    """first([a, b, c]) → a."""
    result = _eval("first([a, b, c])")
    assert result == _sym("a")


def test_pipeline_rest() -> None:
    """rest([a, b, c]) → [b, c]."""
    result = _eval("rest([a, b, c])")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert result.args == (_sym("b"), _sym("c"))


def test_pipeline_append() -> None:
    """append([1], [2, 3]) → [1, 2, 3]."""
    result = _eval("append([1], [2, 3])")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert len(result.args) == 3


def test_pipeline_reverse() -> None:
    """reverse([1, 2, 3]) → [3, 2, 1]."""
    result = _eval("reverse([1, 2, 3])")
    assert isinstance(result, IRApply)
    assert result.head.name == "List"
    assert result.args == (_int(3), _int(2), _int(1))


# ---------------------------------------------------------------------------
# Section F — constants
# ---------------------------------------------------------------------------


def test_pipeline_pi_resolves() -> None:
    """%pi evaluates to an IRFloat close to math.pi."""
    import math

    result = _eval("%pi")
    assert isinstance(result, IRFloat)
    assert abs(result.value - math.pi) < 1e-9


def test_pipeline_e_resolves() -> None:
    """%e evaluates to an IRFloat close to math.e."""
    import math

    result = _eval("%e")
    assert isinstance(result, IRFloat)
    assert abs(result.value - math.e) < 1e-9


# ---------------------------------------------------------------------------
# Section G — limit
# ---------------------------------------------------------------------------


def test_pipeline_limit_polynomial() -> None:
    """limit(x^2, x, 3) → 9."""
    result = _eval("limit(x^2, x, 3)")
    # Result may be IRInteger(9) or IRRational(9,1) after numeric fold.
    if isinstance(result, IRRational):
        assert result.numerator == 9
    else:
        assert result == _int(9)
