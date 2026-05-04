"""Tests for VM handler functions (IR conversion + dispatch)."""

from __future__ import annotations

from fractions import Fraction

import pytest
from symbolic_ir import (
    ADD,
    LIST,
    MUL,
    NEG,
    POW,
    SUB,
    IRApply,
    IRInteger,
    IRRational,
    IRSymbol,
)

from cas_multivariate.handlers import (
    GROEBNER,
    IDEAL_SOLVE,
    POLY_REDUCE,
    ConversionError,
    _extract_poly_list,
    _extract_var_list,
    _frac_to_ir,
    _ir_to_mpoly,
    _mpoly_to_ir,
    build_multivariate_handler_table,
    groebner_handler,
    ideal_solve_handler,
    poly_reduce_handler,
)
from cas_multivariate.polynomial import MPoly

F = Fraction

# ---------------------------------------------------------------------------
# Fake VM for handler testing
# ---------------------------------------------------------------------------


class _FakeVM:
    """Minimal stub for the VM parameter in handler calls."""

    def eval(self, node):
        return node


_VM = _FakeVM()

# ---------------------------------------------------------------------------
# IR helpers
# ---------------------------------------------------------------------------

_X = IRSymbol("x")
_Y = IRSymbol("y")


def _var_list_ir(*names: str) -> IRApply:
    return IRApply(LIST, tuple(IRSymbol(n) for n in names))


# ---------------------------------------------------------------------------
# _frac_to_ir
# ---------------------------------------------------------------------------


def test_frac_to_ir_integer():
    """Fraction with denominator 1 → IRInteger."""
    result = _frac_to_ir(F(3))
    assert result == IRInteger(3)


def test_frac_to_ir_rational():
    """Fraction 1/2 → IRRational(1,2)."""
    result = _frac_to_ir(F(1, 2))
    assert result == IRRational(1, 2)


def test_frac_to_ir_negative():
    """Fraction -1/2 → IRRational(-1,2)."""
    result = _frac_to_ir(F(-1, 2))
    assert result == IRRational(-1, 2)


# ---------------------------------------------------------------------------
# _ir_to_mpoly
# ---------------------------------------------------------------------------


def test_ir_to_mpoly_integer():
    """IRInteger(3) → constant MPoly with value 3."""
    p = _ir_to_mpoly(IRInteger(3), ["x", "y"])
    assert p.coeffs == {(0, 0): F(3)}


def test_ir_to_mpoly_rational():
    """IRRational(1,2) → constant MPoly with value 1/2."""
    p = _ir_to_mpoly(IRRational(1, 2), ["x", "y"])
    assert p.coeffs == {(0, 0): F(1, 2)}


def test_ir_to_mpoly_symbol_x():
    """IRSymbol('x') → x in Q[x,y]."""
    p = _ir_to_mpoly(_X, ["x", "y"])
    assert p.coeffs == {(1, 0): F(1)}


def test_ir_to_mpoly_symbol_y():
    """IRSymbol('y') → y in Q[x,y]."""
    p = _ir_to_mpoly(_Y, ["x", "y"])
    assert p.coeffs == {(0, 1): F(1)}


def test_ir_to_mpoly_add():
    """Add(x, y) → x + y."""
    node = IRApply(ADD, (_X, _Y))
    p = _ir_to_mpoly(node, ["x", "y"])
    assert p.coeffs == {(1, 0): F(1), (0, 1): F(1)}


def test_ir_to_mpoly_mul():
    """Mul(x, y) → x*y."""
    node = IRApply(MUL, (_X, _Y))
    p = _ir_to_mpoly(node, ["x", "y"])
    assert p.coeffs == {(1, 1): F(1)}


def test_ir_to_mpoly_pow():
    """Pow(x, 2) → x^2."""
    node = IRApply(POW, (_X, IRInteger(2)))
    p = _ir_to_mpoly(node, ["x", "y"])
    assert p.coeffs == {(2, 0): F(1)}


def test_ir_to_mpoly_pow_zero():
    """Pow(x, 0) → 1."""
    node = IRApply(POW, (_X, IRInteger(0)))
    p = _ir_to_mpoly(node, ["x", "y"])
    assert p.coeffs == {(0, 0): F(1)}


def test_ir_to_mpoly_neg():
    """Neg(x) → -x."""
    node = IRApply(NEG, (_X,))
    p = _ir_to_mpoly(node, ["x", "y"])
    assert p.coeffs == {(1, 0): F(-1)}


def test_ir_to_mpoly_sub():
    """Sub(x, y) → x - y."""
    node = IRApply(SUB, (_X, _Y))
    p = _ir_to_mpoly(node, ["x", "y"])
    assert p.coeffs == {(1, 0): F(1), (0, 1): F(-1)}


def test_ir_to_mpoly_complex_expr():
    """x^2 + 2*x + 1 = (x+1)^2."""
    # Add(Pow(x,2), Mul(2,x), 1)
    node = IRApply(ADD, (
        IRApply(POW, (_X, IRInteger(2))),
        IRApply(MUL, (IRInteger(2), _X)),
        IRInteger(1),
    ))
    p = _ir_to_mpoly(node, ["x"])
    assert p.coeffs == {(2,): F(1), (1,): F(2), (0,): F(1)}


def test_ir_to_mpoly_unknown_symbol_raises():
    """Unknown symbol (not in var_list) raises ConversionError."""
    with pytest.raises(ConversionError):
        _ir_to_mpoly(IRSymbol("z"), ["x", "y"])


def test_ir_to_mpoly_negative_pow_raises():
    """Negative exponent raises ConversionError."""
    with pytest.raises(ConversionError):
        _ir_to_mpoly(IRApply(POW, (_X, IRInteger(-1))), ["x", "y"])


def test_ir_to_mpoly_non_integer_pow_raises():
    """Non-integer exponent raises ConversionError."""
    with pytest.raises(ConversionError):
        _ir_to_mpoly(IRApply(POW, (_X, _Y)), ["x", "y"])


def test_ir_to_mpoly_unsupported_raises():
    """Unsupported node type raises ConversionError."""
    from symbolic_ir import SIN
    with pytest.raises(ConversionError):
        _ir_to_mpoly(IRApply(SIN, (_X,)), ["x", "y"])


# ---------------------------------------------------------------------------
# _mpoly_to_ir
# ---------------------------------------------------------------------------


def test_mpoly_to_ir_zero():
    """Zero polynomial → IRInteger(0)."""
    result = _mpoly_to_ir(MPoly.zero(2), [_X, _Y])
    assert result == IRInteger(0)


def test_mpoly_to_ir_constant():
    """Constant polynomial → IRInteger or IRRational."""
    p = MPoly({(0, 0): F(5)}, 2)
    result = _mpoly_to_ir(p, [_X, _Y])
    assert result == IRInteger(5)


def test_mpoly_to_ir_single_var():
    """x → IRSymbol('x')."""
    p = MPoly({(1, 0): F(1)}, 2)
    result = _mpoly_to_ir(p, [_X, _Y])
    assert result == _X


def test_mpoly_to_ir_roundtrip():
    """IR → MPoly → IR should produce a semantically equivalent polynomial."""
    # x^2 + y - 1
    original = IRApply(ADD, (
        IRApply(POW, (_X, IRInteger(2))),
        _Y,
        IRInteger(-1),
    ))
    p = _ir_to_mpoly(original, ["x", "y"])
    # Convert back to IR and re-convert to MPoly for comparison.
    back_ir = _mpoly_to_ir(p, [_X, _Y])
    p2 = _ir_to_mpoly(back_ir, ["x", "y"])
    assert p == p2


# ---------------------------------------------------------------------------
# _extract_var_list
# ---------------------------------------------------------------------------


def test_extract_var_list_basic():
    node = _var_list_ir("x", "y")
    assert _extract_var_list(node) == ["x", "y"]


def test_extract_var_list_not_list():
    assert _extract_var_list(_X) is None


def test_extract_var_list_non_symbol():
    node = IRApply(LIST, (IRInteger(1),))
    assert _extract_var_list(node) is None


# ---------------------------------------------------------------------------
# _extract_poly_list
# ---------------------------------------------------------------------------


def test_extract_poly_list_basic():
    # List(x, y) interpreted as polynomials
    node = IRApply(LIST, (_X, _Y))
    polys = _extract_poly_list(node, ["x", "y"])
    assert polys is not None
    assert len(polys) == 2


def test_extract_poly_list_conversion_failure():
    from symbolic_ir import SIN
    node = IRApply(LIST, (IRApply(SIN, (_X,)),))
    assert _extract_poly_list(node, ["x"]) is None


# ---------------------------------------------------------------------------
# Groebner handler
# ---------------------------------------------------------------------------


def test_groebner_handler_basic():
    """groebner([x+y-1, x-y], [x,y]) should return a List of IR polynomials."""
    # List(Add(x, y, -1), Sub(x, y))
    poly_list = IRApply(LIST, (
        IRApply(ADD, (_X, _Y, IRInteger(-1))),
        IRApply(SUB, (_X, _Y)),
    ))
    var_list = _var_list_ir("x", "y")
    expr = IRApply(GROEBNER, (poly_list, var_list))
    result = groebner_handler(_VM, expr)
    assert isinstance(result, IRApply)
    assert result.head == LIST
    assert len(result.args) >= 1


def test_groebner_handler_wrong_arity():
    """Wrong number of args → unevaluated."""
    expr = IRApply(GROEBNER, (_var_list_ir("x"),))
    result = groebner_handler(_VM, expr)
    assert result is expr


def test_groebner_handler_non_list_vars():
    """Non-List second argument → unevaluated."""
    poly_list = IRApply(LIST, (_X,))
    expr = IRApply(GROEBNER, (poly_list, _X))
    result = groebner_handler(_VM, expr)
    assert result is expr


def test_groebner_handler_bad_poly():
    """Non-polynomial expression → unevaluated."""
    from symbolic_ir import SIN
    bad = IRApply(LIST, (IRApply(SIN, (_X,)),))
    expr = IRApply(GROEBNER, (bad, _var_list_ir("x")))
    result = groebner_handler(_VM, expr)
    assert result is expr


# ---------------------------------------------------------------------------
# PolyReduce handler
# ---------------------------------------------------------------------------


def test_poly_reduce_handler_basic():
    """poly_reduce(x^2, [x - 1], [x]) → 1 (since x^2 mod (x-1) = 1)."""
    f_ir = IRApply(POW, (_X, IRInteger(2)))           # x^2
    g_list = IRApply(LIST, (IRApply(SUB, (_X, IRInteger(1))),))  # [x - 1]
    var_list = _var_list_ir("x")
    expr = IRApply(POLY_REDUCE, (f_ir, g_list, var_list))
    result = poly_reduce_handler(_VM, expr)
    # x^2 reduced by [x-1]: x^2 = x*(x-1) + x; x = 1*(x-1) + 1 → remainder 1
    # Result should be IRInteger(1)
    assert result == IRInteger(1)


def test_poly_reduce_handler_wrong_arity():
    """Wrong arity → unevaluated."""
    expr = IRApply(POLY_REDUCE, (_X, _var_list_ir("x")))
    result = poly_reduce_handler(_VM, expr)
    assert result is expr


def test_poly_reduce_handler_bad_f():
    """Non-polynomial f → unevaluated."""
    from symbolic_ir import SIN
    bad_f = IRApply(SIN, (_X,))
    g_list = IRApply(LIST, (_X,))
    expr = IRApply(POLY_REDUCE, (bad_f, g_list, _var_list_ir("x")))
    result = poly_reduce_handler(_VM, expr)
    assert result is expr


# ---------------------------------------------------------------------------
# IdealSolve handler
# ---------------------------------------------------------------------------


def test_ideal_solve_handler_linear():
    """ideal_solve([x+y-1, x-y], [x,y]) → List(List(Rule(x,1/2), Rule(y,1/2)))."""
    poly_list = IRApply(LIST, (
        IRApply(ADD, (_X, _Y, IRInteger(-1))),
        IRApply(SUB, (_X, _Y)),
    ))
    var_list = _var_list_ir("x", "y")
    expr = IRApply(IDEAL_SOLVE, (poly_list, var_list))
    result = ideal_solve_handler(_VM, expr)
    assert isinstance(result, IRApply)
    assert result.head == LIST
    assert len(result.args) == 1  # One solution
    sol = result.args[0]
    assert isinstance(sol, IRApply)
    assert sol.head == LIST
    # Each element is Rule(var, val)
    rules = {r.args[0].name: r.args[1] for r in sol.args}  # type: ignore[union-attr]
    assert rules["x"] == IRRational(1, 2)
    assert rules["y"] == IRRational(1, 2)


def test_ideal_solve_handler_wrong_arity():
    """Wrong arity → unevaluated."""
    expr = IRApply(IDEAL_SOLVE, (_var_list_ir("x"),))
    result = ideal_solve_handler(_VM, expr)
    assert result is expr


def test_ideal_solve_handler_no_solution():
    """x^2+1=0 has no real solutions → unevaluated."""
    poly_list = IRApply(LIST, (
        IRApply(ADD, (IRApply(POW, (_X, IRInteger(2))), IRInteger(1))),
    ))
    var_list = _var_list_ir("x")
    expr = IRApply(IDEAL_SOLVE, (poly_list, var_list))
    result = ideal_solve_handler(_VM, expr)
    assert result is expr


# ---------------------------------------------------------------------------
# build_multivariate_handler_table
# ---------------------------------------------------------------------------


def test_build_handler_table():
    """Handler table has all three expected keys."""
    table = build_multivariate_handler_table()
    assert "Groebner" in table
    assert "PolyReduce" in table
    assert "IdealSolve" in table
    assert callable(table["Groebner"])
    assert callable(table["PolyReduce"])
    assert callable(table["IdealSolve"])
