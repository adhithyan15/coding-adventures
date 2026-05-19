"""Oracle tests for recently-added scalar functions.

This file pins behaviour for the newly registered scalar functions against
real ``sqlite3``.  The functions live in :mod:`sql_vm.scalar_functions`;
this module exercises them end-to-end through the mini-sqlite pipeline so
we catch any registration/dispatch issues, not just unit-level bugs.

Coverage:

1. **Hyperbolic trigonometry** — ``sinh``, ``cosh``, ``tanh``, ``asinh``,
   ``acosh``, ``atanh``.  Standard SQLite math-function-library entries.

2. **``trunc(X)``** — truncate toward zero (differs from ``floor`` in
   sign handling for negative inputs).

3. **Optimizer hints** — ``likely(X)``, ``unlikely(X)``, and
   ``likelihood(X, Y)``.  All pass *X* through unchanged in real SQLite
   (they exist only to inform the planner's branch-probability estimates).

4. **Compile-time option probes** — ``sqlite_compileoption_used(name)``
   returns 0 in mini-sqlite (no SQLite compile-time options are defined);
   ``sqlite_compileoption_get(N)`` returns NULL.  Both match SQLite's
   contract for a build with no options enabled.

Every assertion compares against the real ``sqlite3`` module so we know
the answer matches the reference implementation exactly.
"""

from __future__ import annotations

import math
import sqlite3

import mini_sqlite

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _one(sql: str):
    """Return the single scalar from ``SELECT <expr>`` on mini-sqlite."""
    return mini_sqlite.connect(":memory:").execute(sql).fetchone()[0]


def _ref(sql: str):
    """Return the single scalar from ``SELECT <expr>`` on real sqlite3."""
    return sqlite3.connect(":memory:").execute(sql).fetchone()[0]


def _check(sql: str) -> None:
    """Assert mini-sqlite matches real sqlite3 for *sql*."""
    m, r = _one(sql), _ref(sql)
    if isinstance(m, float) and isinstance(r, float):
        # Floating-point comparisons need tolerance (libm differences).
        assert math.isclose(m, r, rel_tol=1e-12, abs_tol=1e-12), (
            f"SQL: {sql!r}\n  mini: {m}\n  ref:  {r}"
        )
    else:
        assert m == r, f"SQL: {sql!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# Hyperbolic trig
# ---------------------------------------------------------------------------


class TestHyperbolicTrig:
    def test_sinh_zero(self) -> None:
        _check("SELECT sinh(0)")

    def test_sinh_one(self) -> None:
        _check("SELECT sinh(1)")

    def test_cosh_zero(self) -> None:
        _check("SELECT cosh(0)")

    def test_cosh_two(self) -> None:
        _check("SELECT cosh(2)")

    def test_tanh_one(self) -> None:
        _check("SELECT tanh(1)")

    def test_tanh_saturates(self) -> None:
        # |tanh(x)| → 1 as |x| → ∞; both engines should agree to 1.0
        _check("SELECT tanh(50)")

    def test_asinh_inverse_sinh(self) -> None:
        _check("SELECT asinh(sinh(1.5))")

    def test_acosh_one_is_zero(self) -> None:
        _check("SELECT acosh(1)")

    def test_acosh_two(self) -> None:
        _check("SELECT acosh(2)")

    def test_atanh_zero(self) -> None:
        _check("SELECT atanh(0)")

    def test_atanh_half(self) -> None:
        _check("SELECT atanh(0.5)")

    def test_sinh_null(self) -> None:
        _check("SELECT sinh(NULL)")


# ---------------------------------------------------------------------------
# trunc(X)
# ---------------------------------------------------------------------------


class TestTrunc:
    def test_trunc_positive_real(self) -> None:
        _check("SELECT trunc(3.7)")

    def test_trunc_negative_real(self) -> None:
        # Distinct from floor(-3.7) = -4
        _check("SELECT trunc(-3.7)")

    def test_trunc_zero(self) -> None:
        _check("SELECT trunc(0)")

    def test_trunc_integer(self) -> None:
        _check("SELECT trunc(5)")

    def test_trunc_negative_small(self) -> None:
        _check("SELECT trunc(-0.5)")

    def test_trunc_null(self) -> None:
        _check("SELECT trunc(NULL)")


# ---------------------------------------------------------------------------
# Optimizer hints — likely / unlikely / likelihood
# ---------------------------------------------------------------------------


class TestOptimizerHints:
    def test_likely_integer(self) -> None:
        _check("SELECT likely(42)")

    def test_likely_text(self) -> None:
        _check("SELECT likely('hello')")

    def test_likely_null(self) -> None:
        _check("SELECT likely(NULL)")

    def test_unlikely_integer(self) -> None:
        _check("SELECT unlikely(7)")

    def test_unlikely_in_where(self) -> None:
        # Real-world usage pattern: WHERE unlikely(rare_condition)
        conn_mini = mini_sqlite.connect(":memory:")
        conn_mini.execute("CREATE TABLE t (n INTEGER)")
        conn_mini.executemany("INSERT INTO t VALUES (?)", [(1,), (2,), (3,)])
        rows_mini = conn_mini.execute(
            "SELECT n FROM t WHERE unlikely(n = 2)"
        ).fetchall()

        conn_ref = sqlite3.connect(":memory:")
        conn_ref.execute("CREATE TABLE t (n INTEGER)")
        conn_ref.executemany("INSERT INTO t VALUES (?)", [(1,), (2,), (3,)])
        rows_ref = conn_ref.execute(
            "SELECT n FROM t WHERE unlikely(n = 2)"
        ).fetchall()

        assert rows_mini == rows_ref == [(2,)]

    def test_likelihood_integer(self) -> None:
        _check("SELECT likelihood(99, 0.5)")

    def test_likelihood_text(self) -> None:
        _check("SELECT likelihood('x', 0.99)")

    def test_likelihood_null(self) -> None:
        _check("SELECT likelihood(NULL, 0.01)")


# ---------------------------------------------------------------------------
# sqlite_compileoption_used / sqlite_compileoption_get
# ---------------------------------------------------------------------------
#
# These return implementation-defined values in real SQLite (depending on
# which compile flags were set in the build).  We can't compare directly
# against real sqlite3's output because the Python sqlite3 module's build
# has options enabled and ours doesn't.  Instead we assert mini-sqlite's
# documented contract: 0 from _used (no options defined) and NULL from
# _get.


class TestSqliteCompileOptions:
    def test_compileoption_used_returns_zero(self) -> None:
        # Any name returns 0 — mini-sqlite has no compile options.
        assert _one("SELECT sqlite_compileoption_used('THREADSAFE')") == 0
        assert _one("SELECT sqlite_compileoption_used('ENABLE_RTREE')") == 0

    def test_compileoption_get_returns_null(self) -> None:
        assert _one("SELECT sqlite_compileoption_get(0)") is None
        assert _one("SELECT sqlite_compileoption_get(100)") is None
