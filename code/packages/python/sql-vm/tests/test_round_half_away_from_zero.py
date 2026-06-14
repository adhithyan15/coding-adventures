"""Oracle tests for ``ROUND(x[, n])`` — half-away-from-zero semantics.

SQLite specifies a school-arithmetic tie-breaker for ``round``: a value
exactly halfway between two integers (in decimal) rounds *away from
zero*.  Mini-sqlite was previously delegating to Python's built-in
``round`` which uses *banker's rounding* (round half to even); that
gave subtly wrong answers for any half-integer input::

    Python:  round(0.5) == 0,  round(2.5) == 2
    SQLite:  round(0.5) == 1,  round(2.5) == 3

This regression was particularly nasty because most one-off tests pass
(``round(1.5) == 2`` is true under both rules).  The discrepancy only
shows up on even half-integers.

Two-argument form is also now correct: it uses ``Decimal.quantize``
with ``ROUND_HALF_UP`` on the exact IEEE 754 representation of *x*,
which is equivalent to SQLite's internal ``printf("%.*f", n, x)``
behaviour because both work in decimal space on the stored double.

Additional fix: ``round(x, NULL)`` now returns NULL (SQLite short-
circuits on NULL digits) instead of treating NULL as the default 0.
"""

from __future__ import annotations

import pytest

from sql_vm.scalar_functions import call


def _r(*args: object) -> object:
    return call("round", list(args))


# ---------------------------------------------------------------------------
# One-argument form — half-away-from-zero
# ---------------------------------------------------------------------------


class TestRoundOneArgHalfAwayFromZero:
    @pytest.mark.parametrize(
        ("x", "expected"),
        [
            (0.5, 1.0),
            (1.5, 2.0),
            (2.5, 3.0),   # was 2.0 under banker's
            (3.5, 4.0),
            (4.5, 5.0),   # was 4.0 under banker's
            (-0.5, -1.0),
            (-1.5, -2.0),
            (-2.5, -3.0), # was -2.0 under banker's
            (1.0, 1.0),
            (0.0, 0.0),
            (1.4, 1.0),
            (1.6, 2.0),
        ],
    )
    def test_half_away_from_zero(self, x: float, expected: float) -> None:
        assert _r(x) == expected


# ---------------------------------------------------------------------------
# Two-argument form — round at the n-th decimal place, half-up on the
# exact IEEE 754 representation
# ---------------------------------------------------------------------------


class TestRoundTwoArg:
    @pytest.mark.parametrize(
        ("x", "n", "expected"),
        [
            # Tie cases where the stored double exactly equals the displayed
            # decimal → round half up.
            (0.25, 1, 0.3),
            (1.25, 1, 1.3),
            (1.35, 1, 1.4),
            (2.5, 0, 3.0),
            # Tie cases where the stored double is slightly below the
            # displayed decimal → rounds down because the underlying value
            # is not actually halfway.
            (2.355, 2, 2.35),
            (1.005, 2, 1.0),
            (1.045, 2, 1.04),
            (0.15, 1, 0.1),
            # Tie cases where the stored double is slightly above → up.
            (2.345, 2, 2.35),
            # SQLite clamps the digits argument to [0, 30]; negative
            # values become 0 (no rounding to the left of the decimal
            # point — this is SQLite-specific, not standard SQL).
            (1234.5, -1, 1235.0),
            (1235.5, -1, 1236.0),
            (1250.0, -2, 1250.0),
            # Values above 30 are clamped down to 30 digits of precision.
            (1.5, 35, 1.5),
        ],
    )
    def test_two_arg(self, x: float, n: int, expected: float) -> None:
        assert _r(x, n) == expected


# ---------------------------------------------------------------------------
# NULL handling — propagate, including for NULL digits
# ---------------------------------------------------------------------------


class TestRoundNullHandling:
    def test_null_x_one_arg(self) -> None:
        assert _r(None) is None

    def test_null_x_two_arg(self) -> None:
        assert _r(None, 2) is None

    def test_null_digits(self) -> None:
        # Regression: SQLite short-circuits on NULL digits → NULL.
        # mini-sqlite previously coerced NULL → 0 and returned a value.
        assert _r(1.5, None) is None

    def test_null_both(self) -> None:
        assert _r(None, None) is None


# ---------------------------------------------------------------------------
# Integer inputs — no change in value
# ---------------------------------------------------------------------------


class TestRoundIntegerInputs:
    def test_int_one_arg(self) -> None:
        # int input with default digits → preserved as a whole-number float
        assert _r(5) == 5.0

    def test_int_two_arg(self) -> None:
        assert _r(5, 2) == 5.0

    def test_int_negative_digits_clamped(self) -> None:
        # Negative n is clamped to 0 by SQLite, so the result is the
        # original integer (as a double).
        assert _r(125, -1) == 125.0
