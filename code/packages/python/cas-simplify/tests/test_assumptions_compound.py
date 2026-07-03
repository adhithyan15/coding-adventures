"""Tests for compound-relation support in :class:`AssumptionContext`
(Track G1, macsyma-truly-finish-plan).

Until Track G1 the assumption store only understood plain-symbol-vs-zero
relations (``assume(x > 0)``).  Compound relations such as
``assume(a^2 > b^2)`` were silently dropped, which prevented the
symbolic-coefficient Weierzstrass integrator from learning the
discriminant sign at integration time.

These tests exercise the new compound-relation path end-to-end at the
``AssumptionContext`` API level — they neither require nor exercise the
VM.  The integrator-side behaviour is tested in
``symbolic-vm/tests/test_weierstrass_symbolic_coefficients.py``.
"""

from __future__ import annotations

from symbolic_ir import (
    EQUAL,
    GREATER,
    GREATER_EQUAL,
    LESS,
    LESS_EQUAL,
    NOT_EQUAL,
    POW,
    IRApply,
    IRInteger,
    IRSymbol,
)

from cas_simplify.assumptions import AssumptionContext


# Test fixtures — ``a^2`` and ``b^2`` as IR.  We build them once at module
# scope because :class:`IRApply` is a frozen dataclass with structural
# equality, so the same node identity flows through every test.
A = IRSymbol("a")
B = IRSymbol("b")
TWO = IRInteger(2)
A_SQ = IRApply(POW, (A, TWO))
B_SQ = IRApply(POW, (B, TWO))


def _gt(lhs, rhs):
    return IRApply(GREATER, (lhs, rhs))


def _lt(lhs, rhs):
    return IRApply(LESS, (lhs, rhs))


def _ge(lhs, rhs):
    return IRApply(GREATER_EQUAL, (lhs, rhs))


def _le(lhs, rhs):
    return IRApply(LESS_EQUAL, (lhs, rhs))


def _eq(lhs, rhs):
    return IRApply(EQUAL, (lhs, rhs))


def _ne(lhs, rhs):
    return IRApply(NOT_EQUAL, (lhs, rhs))


# ---------------------------------------------------------------------------
# Direct lookups — assume(rel); is(rel) is True.
# ---------------------------------------------------------------------------


def test_assume_compound_greater_direct() -> None:
    """``assume(a^2 > b^2)`` then ``is(a^2 > b^2)`` must return True."""
    ctx = AssumptionContext()
    ctx.assume_relation(_gt(A_SQ, B_SQ))
    assert ctx.is_true_relation(_gt(A_SQ, B_SQ)) is True


def test_assume_compound_equal_direct() -> None:
    """``assume(a^2 = b^2)`` then ``is(a^2 = b^2)`` must return True."""
    ctx = AssumptionContext()
    ctx.assume_relation(_eq(A_SQ, B_SQ))
    assert ctx.is_true_relation(_eq(A_SQ, B_SQ)) is True


def test_assume_compound_less_direct() -> None:
    """``assume(a^2 < b^2)`` then ``is(a^2 < b^2)`` must return True."""
    ctx = AssumptionContext()
    ctx.assume_relation(_lt(A_SQ, B_SQ))
    assert ctx.is_true_relation(_lt(A_SQ, B_SQ)) is True


def test_assume_compound_greater_equal_direct() -> None:
    """``assume(a^2 >= b^2)`` then ``is(a^2 >= b^2)`` must return True."""
    ctx = AssumptionContext()
    ctx.assume_relation(_ge(A_SQ, B_SQ))
    assert ctx.is_true_relation(_ge(A_SQ, B_SQ)) is True


def test_assume_compound_not_equal_direct() -> None:
    """``assume(a^2 != b^2)`` then ``is(a^2 != b^2)`` must return True."""
    ctx = AssumptionContext()
    ctx.assume_relation(_ne(A_SQ, B_SQ))
    assert ctx.is_true_relation(_ne(A_SQ, B_SQ)) is True


# ---------------------------------------------------------------------------
# Commutative / dual rewrites — assume(rel); is(dual(rel)) is True.
# ---------------------------------------------------------------------------


def test_assume_greater_commutes_to_less() -> None:
    """``assume(a^2 > b^2)`` implies ``is(b^2 < a^2)`` is True."""
    ctx = AssumptionContext()
    ctx.assume_relation(_gt(A_SQ, B_SQ))
    assert ctx.is_true_relation(_lt(B_SQ, A_SQ)) is True


def test_assume_less_commutes_to_greater() -> None:
    """``assume(a^2 < b^2)`` implies ``is(b^2 > a^2)`` is True."""
    ctx = AssumptionContext()
    ctx.assume_relation(_lt(A_SQ, B_SQ))
    assert ctx.is_true_relation(_gt(B_SQ, A_SQ)) is True


def test_assume_ge_commutes_to_le() -> None:
    """``assume(a^2 >= b^2)`` implies ``is(b^2 <= a^2)`` is True."""
    ctx = AssumptionContext()
    ctx.assume_relation(_ge(A_SQ, B_SQ))
    assert ctx.is_true_relation(_le(B_SQ, A_SQ)) is True


def test_assume_equal_is_commutative() -> None:
    """``assume(a^2 = b^2)`` implies ``is(b^2 = a^2)`` is True."""
    ctx = AssumptionContext()
    ctx.assume_relation(_eq(A_SQ, B_SQ))
    assert ctx.is_true_relation(_eq(B_SQ, A_SQ)) is True


def test_assume_ne_is_commutative() -> None:
    """``assume(a^2 != b^2)`` implies ``is(b^2 != a^2)`` is True."""
    ctx = AssumptionContext()
    ctx.assume_relation(_ne(A_SQ, B_SQ))
    assert ctx.is_true_relation(_ne(B_SQ, A_SQ)) is True


# ---------------------------------------------------------------------------
# Unknown / no-negative-inference cases.
# ---------------------------------------------------------------------------


def test_unknown_compound_returns_none() -> None:
    """No assertion at all → query returns None (not False)."""
    ctx = AssumptionContext()
    c, d = IRSymbol("c"), IRSymbol("d")
    c_sq = IRApply(POW, (c, TWO))
    d_sq = IRApply(POW, (d, TWO))
    assert ctx.is_true_relation(_gt(c_sq, d_sq)) is None


def test_assume_greater_does_not_imply_less_false() -> None:
    """``assume(a^2 > b^2)`` does NOT imply ``is(a^2 < b^2)`` is False —
    we return None because compound facts deliberately avoid negative
    inference (the user might also assert the opposite later)."""
    ctx = AssumptionContext()
    ctx.assume_relation(_gt(A_SQ, B_SQ))
    assert ctx.is_true_relation(_lt(A_SQ, B_SQ)) is None


# ---------------------------------------------------------------------------
# Forget path — compound relations are removable.
# ---------------------------------------------------------------------------


def test_forget_compound_relation() -> None:
    """``forget_relation(a^2 > b^2)`` removes the previously stored fact."""
    ctx = AssumptionContext()
    ctx.assume_relation(_gt(A_SQ, B_SQ))
    assert ctx.is_true_relation(_gt(A_SQ, B_SQ)) is True
    ctx.forget_relation(_gt(A_SQ, B_SQ))
    assert ctx.is_true_relation(_gt(A_SQ, B_SQ)) is None


def test_forget_all_clears_compound_relations() -> None:
    """``forget_all`` clears both plain-symbol facts AND compound
    relations — i.e. the assumption store is fully reset."""
    ctx = AssumptionContext()
    ctx.assume_relation(_gt(A_SQ, B_SQ))
    x = IRSymbol("x")
    ctx.assume_relation(_gt(x, IRInteger(0)))
    ctx.forget_all()
    assert ctx.is_true_relation(_gt(A_SQ, B_SQ)) is None
    assert ctx.is_positive("x") is None


# ---------------------------------------------------------------------------
# Plain-symbol path unchanged — make sure the extension doesn't regress.
# ---------------------------------------------------------------------------


def test_plain_symbol_path_still_works() -> None:
    """``assume(x > 0)`` still threads through the per-symbol fact table."""
    ctx = AssumptionContext()
    x = IRSymbol("x")
    ctx.assume_relation(_gt(x, IRInteger(0)))
    assert ctx.is_positive("x") is True
    assert ctx.is_true_relation(_gt(x, IRInteger(0))) is True
    assert ctx.is_true_relation(_lt(x, IRInteger(0))) is False


def test_assume_compound_dedupes() -> None:
    """Asserting the same compound fact twice should not grow the store."""
    ctx = AssumptionContext()
    ctx.assume_relation(_gt(A_SQ, B_SQ))
    ctx.assume_relation(_gt(A_SQ, B_SQ))
    # Re-assert via the commuted form too — canonicalisation should fold.
    ctx.assume_relation(_lt(B_SQ, A_SQ))
    # We can't directly inspect ``_general_relations`` size from the
    # public API; instead we round-trip a forget and make sure a SINGLE
    # forget zeroes out the fact.
    ctx.forget_relation(_gt(A_SQ, B_SQ))
    assert ctx.is_true_relation(_gt(A_SQ, B_SQ)) is None
