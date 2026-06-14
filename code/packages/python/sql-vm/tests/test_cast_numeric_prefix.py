"""SQLite-compatible string-to-number coercion in ``CAST``.

Python's ``int()`` / ``float()`` reject any string with trailing
non-numeric characters: ``float('1.5abc')`` raises.  SQLite's CAST
takes a different approach — it greedily matches the longest valid
numeric prefix and discards anything that follows.  So
``CAST('1.5abc' AS REAL)`` is ``1.5``, and ``CAST('123abc' AS INTEGER)``
is ``123``.

Two corollaries that catch most engines off-guard:

1. The literal text ``"inf"`` and ``"nan"`` coerce to ``0.0`` because
   they have no leading digit — there is no numeric prefix at all.
   Python's ``float('inf')`` returns infinity, which mini-sqlite used
   to surface to callers (wrong).

2. The INTEGER cast extracts only the *integer* prefix, not the float
   prefix.  ``CAST('1.5abc' AS INTEGER)`` is ``1`` (just the ``1``),
   not ``1`` from truncating ``1.5``.  Similarly ``CAST('1e5' AS
   INTEGER)`` is ``1`` — the cast stops at the decimal/exponent
   marker.

The new helpers ``_sqlite_str_to_int`` and ``_sqlite_str_to_real`` use
regex prefix matching to encode SQLite's rule directly.
"""

from __future__ import annotations

import math

import pytest

from sql_vm.scalar_functions import call


def _cast(value: object, target: str) -> object:
    return call("cast", [value, target])


# ---------------------------------------------------------------------------
# REAL — float prefix, no Python-style special keywords
# ---------------------------------------------------------------------------


class TestCastReal:
    @pytest.mark.parametrize(
        ("text", "expected"),
        [
            # Special-keyword strings — Python would accept these, SQLite
            # rejects them (no leading digit → empty prefix → 0.0).
            ("inf", 0.0),
            ("Inf", 0.0),
            ("infinity", 0.0),
            ("-inf", 0.0),
            ("nan", 0.0),
            ("NaN", 0.0),
            # Non-numeric prefix.
            ("abc", 0.0),
            ("", 0.0),
            # Valid number followed by garbage — keep the prefix.
            ("1.5abc", 1.5),
            ("123abc", 123.0),
            ("1e5abc", 100000.0),
            # Whitespace handling.
            ("  1.5  ", 1.5),
            # Leading sign, decimal-only, exponent.
            ("+1.5", 1.5),
            ("-1.5", -1.5),
            (".5", 0.5),
            ("1e10", 10_000_000_000.0),
        ],
    )
    def test_string_to_real(self, text: str, expected: float) -> None:
        result = _cast(text, "REAL")
        assert result == expected
        assert isinstance(result, float)

    def test_overflow_to_inf(self) -> None:
        # ``1e500`` overflows IEEE 754 double → +inf; SQLite agrees.
        result = _cast("1e500", "REAL")
        assert math.isinf(result) and result > 0  # type: ignore[arg-type]

    def test_int_input_unchanged(self) -> None:
        assert _cast(42, "REAL") == 42.0

    def test_float_input_unchanged(self) -> None:
        assert _cast(3.14, "REAL") == 3.14

    def test_bool_to_real(self) -> None:
        assert _cast(True, "REAL") == 1.0
        assert _cast(False, "REAL") == 0.0

    def test_null(self) -> None:
        assert _cast(None, "REAL") is None


# ---------------------------------------------------------------------------
# INTEGER — int prefix only (no decimal, no exponent)
# ---------------------------------------------------------------------------


class TestCastInteger:
    @pytest.mark.parametrize(
        ("text", "expected"),
        [
            # Plain integers.
            ("0", 0),
            ("42", 42),
            ("-42", -42),
            ("+42", 42),
            # Garbage after integer prefix is discarded.
            ("123abc", 123),
            ("-42abc", -42),
            # Floats become their integer prefix, NOT the truncated float.
            # 1.5abc → integer prefix is "1" → result 1.  This is subtle:
            # SQLite's INTEGER cast doesn't honour the decimal point at
            # all, so the answer is the same as for 1abc.
            ("1.5abc", 1),
            ("1.5", 1),
            ("1.9", 1),  # NOT 2 (no truncation; just prefix match)
            ("-1.9", -1),
            # Exponent markers are also not part of the int prefix.
            ("1e5", 1),
            ("1e5abc", 1),
            # Empty / non-numeric.
            ("", 0),
            ("abc", 0),
            # Leading whitespace.
            ("  -7  ", -7),
        ],
    )
    def test_string_to_integer(self, text: str, expected: int) -> None:
        result = _cast(text, "INTEGER")
        assert result == expected
        assert isinstance(result, int)

    def test_float_input_truncates(self) -> None:
        # When the input is already a float (not a string), Python's
        # ``int()`` truncates toward zero — same as SQLite.
        assert _cast(1.9, "INTEGER") == 1
        assert _cast(-1.9, "INTEGER") == -1

    def test_bool_to_integer(self) -> None:
        assert _cast(True, "INTEGER") == 1
        assert _cast(False, "INTEGER") == 0

    def test_null(self) -> None:
        assert _cast(None, "INTEGER") is None


# ---------------------------------------------------------------------------
# Type-name aliases — int8, smallint, double precision, etc. — all map
# ---------------------------------------------------------------------------


class TestCastTypeAliases:
    @pytest.mark.parametrize("alias", ["INT", "int8", "bigint", "smallint", "mediumint"])
    def test_int_aliases(self, alias: str) -> None:
        assert _cast("42abc", alias) == 42

    @pytest.mark.parametrize("alias", ["FLOAT", "double", "double precision", "numeric"])
    def test_real_aliases(self, alias: str) -> None:
        assert _cast("3.14abc", alias) == 3.14
