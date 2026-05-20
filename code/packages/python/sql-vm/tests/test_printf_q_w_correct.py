"""Oracle tests for SQLite's ``%q``, ``%Q``, and ``%w`` printf conversions.

SQLite's printf has three SQL-specific conversions that don't exist in
C's printf — and mini-sqlite had previously confused ``%q`` with
``%Q`` and not implemented ``%w`` at all.

Reference docs: https://sqlite.org/printf.html

  ``%q``  SQL string-literal escape.  Doubles every single quote;
           **no** surrounding quotes are added.  Designed for
           interpolation inside a single-quoted string literal that
           the caller is writing — e.g.
           ``"INSERT INTO t VALUES('" || printf('%q', x) || "')"``.
           NULL becomes the literal ``"(NULL)"``.

  ``%Q``  Complete SQL string literal.  Like ``%q`` but wraps in single
           quotes — so ``printf('%Q', x)`` is the SQL literal form of
           *x*.  NULL becomes the literal ``"NULL"`` (no quotes).

  ``%w``  SQL identifier escape.  Doubles every *double* quote (the
           identifier-quoting character in standard SQL); **no**
           surrounding quotes are added.  Designed for interpolation
           inside a ``"…"`` quoted identifier — e.g.
           ``"SELECT \"" || printf('%w', col) || \"\" FROM t"``.
           NULL becomes the literal ``"(NULL)"``.

Before this PR mini-sqlite implemented ``%q`` exactly the same as
``%Q`` (wrapping in single quotes) and silently passed ``%w`` through
as the literal text ``"%w"``.  Both are now fixed and the docstrings
explicitly describe the difference so future readers don't fall into
the same trap.
"""

from __future__ import annotations

import pytest

from sql_vm.scalar_functions import call


def _p(*args: object) -> object:
    return call("printf", list(args))


# ---------------------------------------------------------------------------
# %q — SQL string-literal escape, no surrounding quotes
# ---------------------------------------------------------------------------


class TestPercentQ:
    @pytest.mark.parametrize(
        ("arg", "expected"),
        [
            ("hello", "hello"),         # no quotes → unchanged
            ("it's", "it''s"),           # one single quote → doubled
            ("a'b'c", "a''b''c"),        # multiple quotes
            ("''", "''''"),              # two adjacent quotes → four
            ('"hello"', '"hello"'),      # double quotes left alone
            ("", ""),                    # empty string → empty
        ],
    )
    def test_q_strings(self, arg: str, expected: str) -> None:
        assert _p("%q", arg) == expected

    def test_q_null(self) -> None:
        # Real sqlite3 emits the literal text "(NULL)" — not an empty
        # string, not Python's None.  This avoids silently collapsing a
        # NULL into nothing in the middle of a SQL string the caller is
        # building.
        assert _p("%q", None) == "(NULL)"


# ---------------------------------------------------------------------------
# %Q — complete SQL string literal (wrapped in single quotes)
# ---------------------------------------------------------------------------


class TestPercentBigQ:
    @pytest.mark.parametrize(
        ("arg", "expected"),
        [
            ("hello", "'hello'"),
            ("it's", "'it''s'"),
            ("", "''"),
        ],
    )
    def test_big_q_strings(self, arg: str, expected: str) -> None:
        assert _p("%Q", arg) == expected

    def test_big_q_null(self) -> None:
        # %Q is meant to produce a syntactically valid SQL expression,
        # so NULL renders as the literal text "NULL" — no quotes.
        assert _p("%Q", None) == "NULL"


# ---------------------------------------------------------------------------
# %w — SQL identifier escape, no surrounding quotes
# ---------------------------------------------------------------------------


class TestPercentW:
    @pytest.mark.parametrize(
        ("arg", "expected"),
        [
            ("hello", "hello"),         # plain identifier
            ('say"hi', 'say""hi'),       # one double quote → doubled
            ('a"b"c', 'a""b""c'),        # multiple double quotes
            ("it's", "it's"),            # single quotes left alone
            ("", ""),
        ],
    )
    def test_w_strings(self, arg: str, expected: str) -> None:
        assert _p("%w", arg) == expected

    def test_w_null(self) -> None:
        # Mirrors %q — NULL → the literal text "(NULL)".
        assert _p("%w", None) == "(NULL)"


# ---------------------------------------------------------------------------
# Composition with other conversions
# ---------------------------------------------------------------------------


class TestComposition:
    def test_q_in_full_literal(self) -> None:
        # Typical use: build an SQL literal by hand.
        assert _p("VALUES('%q')", "it's") == "VALUES('it''s')"

    def test_big_q_inline(self) -> None:
        assert _p("VALUES(%Q)", None) == "VALUES(NULL)"
        assert _p("VALUES(%Q)", "x") == "VALUES('x')"

    def test_w_in_quoted_identifier(self) -> None:
        # Typical use: build a "schema"."col" reference.
        assert _p('SELECT "%w" FROM t', 'odd"col') == 'SELECT "odd""col" FROM t'

    def test_q_and_s_together(self) -> None:
        # %q escapes; %s passes through unchanged.
        assert _p("%s = %q", "name", "O'Brien") == "name = O''Brien"
