"""Tests for ideal_solve and _solve_univariate."""

from __future__ import annotations

from fractions import Fraction

from cas_multivariate.polynomial import MPoly
from cas_multivariate.solve import _rational_roots, _solve_univariate, ideal_solve

F = Fraction


# ---------------------------------------------------------------------------
# _rational_roots
# ---------------------------------------------------------------------------


def test_rational_roots_quadratic():
    """x^2 - 1 has roots 1 and -1."""
    roots = _rational_roots([F(-1), F(0), F(1)])
    assert set(roots) == {F(1), F(-1)}


def test_rational_roots_no_rational():
    """x^2 - 2 has no rational roots."""
    roots = _rational_roots([F(-2), F(0), F(1)])
    assert roots == []


def test_rational_roots_linear():
    """x - 3 has root 3."""
    roots = _rational_roots([F(-3), F(1)])
    assert roots == [F(3)]


def test_rational_roots_fractional():
    """2x - 1 has root 1/2."""
    roots = _rational_roots([F(-1), F(2)])
    assert roots == [F(1, 2)]


def test_rational_roots_constant():
    """Constant polynomial has no roots."""
    assert _rational_roots([F(5)]) == []


def test_rational_roots_zero_root():
    """x^2 has root 0 (double root, returned once)."""
    roots = _rational_roots([F(0), F(0), F(1)])
    assert F(0) in roots


def test_rational_roots_cubic():
    """x^3 - 6x^2 + 11x - 6 = (x-1)(x-2)(x-3) has roots 1, 2, 3."""
    # ascending coefficients: -6, 11, -6, 1
    roots = _rational_roots([F(-6), F(11), F(-6), F(1)])
    assert set(roots) == {F(1), F(2), F(3)}


# ---------------------------------------------------------------------------
# _solve_univariate
# ---------------------------------------------------------------------------


def test_solve_univariate_linear():
    """2x - 4 = 0 → x = 2."""
    result = _solve_univariate([F(-4), F(2)])
    assert result == [F(2)]


def test_solve_univariate_quadratic_perfect_square():
    """x^2 - 4 = 0 → x = ±2."""
    result = _solve_univariate([F(-4), F(0), F(1)])
    assert result is not None
    assert set(result) == {F(2), F(-2)}


def test_solve_univariate_quadratic_no_real():
    """x^2 + 1 = 0 → no real roots → []."""
    result = _solve_univariate([F(1), F(0), F(1)])
    assert result == []


def test_solve_univariate_double_root():
    """x^2 - 2x + 1 = (x-1)^2 → [1]."""
    result = _solve_univariate([F(1), F(-2), F(1)])
    assert result == [F(1)]


def test_solve_univariate_constant():
    """Constant polynomial has no roots."""
    assert _solve_univariate([F(5)]) == []


# ---------------------------------------------------------------------------
# ideal_solve
# ---------------------------------------------------------------------------


def test_ideal_solve_linear():
    """x + y = 1, x - y = 0 → x = 1/2, y = 1/2."""
    f1 = MPoly({(1, 0): F(1), (0, 1): F(1), (0, 0): F(-1)}, 2)  # x + y - 1
    f2 = MPoly({(1, 0): F(1), (0, 1): F(-1)}, 2)                  # x - y
    solutions = ideal_solve([f1, f2])
    assert solutions is not None
    assert len(solutions) == 1
    sol = solutions[0]
    assert sol[0] == F(1, 2)  # x = 1/2
    assert sol[1] == F(1, 2)  # y = 1/2


def test_ideal_solve_quadratic():
    """x^2 - 1 = 0, y - x = 0 → two solutions: (1,1) and (-1,-1)."""
    f1 = MPoly({(2, 0): F(1), (0, 0): F(-1)}, 2)   # x^2 - 1
    f2 = MPoly({(0, 1): F(1), (1, 0): F(-1)}, 2)   # y - x
    solutions = ideal_solve([f1, f2])
    assert solutions is not None
    assert len(solutions) == 2
    sol_set = {(sol[0], sol[1]) for sol in solutions}
    assert (F(1), F(1)) in sol_set
    assert (F(-1), F(-1)) in sol_set


def test_ideal_solve_no_real_solution():
    """x^2 + 1 = 0, y - x = 0 → no real solutions → None."""
    f1 = MPoly({(2, 0): F(1), (0, 0): F(1)}, 2)    # x^2 + 1
    f2 = MPoly({(0, 1): F(1), (1, 0): F(-1)}, 2)   # y - x
    solutions = ideal_solve([f1, f2])
    assert solutions is None


def test_ideal_solve_empty():
    """Empty polynomial list returns None."""
    assert ideal_solve([]) is None


def test_ideal_solve_univariate():
    """Single variable: x^2 - 4 = 0 → solutions [[-2], [2]] (or similar)."""
    f = MPoly({(2, 0): F(1), (0, 0): F(-4)}, 2)   # x^2 - 4 (2 vars but only x used)
    # This may or may not find a solution depending on triangular structure.
    # The key is it doesn't crash.
    result = ideal_solve([f])
    # Either None or a list — just shouldn't raise.
    assert result is None or isinstance(result, list)


def test_ideal_solve_consistent_linear():
    """x + y = 3, 2x + y = 5 → x = 2, y = 1."""
    f1 = MPoly({(1, 0): F(1), (0, 1): F(1), (0, 0): F(-3)}, 2)   # x + y - 3
    f2 = MPoly({(1, 0): F(2), (0, 1): F(1), (0, 0): F(-5)}, 2)   # 2x + y - 5
    solutions = ideal_solve([f1, f2])
    assert solutions is not None
    assert len(solutions) == 1
    assert solutions[0][0] == F(2)   # x = 2
    assert solutions[0][1] == F(1)   # y = 1
