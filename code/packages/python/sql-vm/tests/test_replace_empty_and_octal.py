"""Two corner-case fixes in the scalar function registry.

1. ``REPLACE(x, "", y)`` is a no-op in SQLite — empty needle matches
   nothing.  Python's ``str.replace("", X)`` inserts ``X`` between every
   character (and at both ends), which is almost never what a SQL caller
   wants.  Mini-sqlite previously delegated to Python and produced
   ``"XhXeXlXlXoX"`` from ``replace("hello", "", "X")``.  The fix is a
   one-line short-circuit.

2. ``printf("%#o", val)`` uses C's classic ``0`` prefix, not Python's
   modern ``0o`` prefix.  Combined with width/zero-pad flags SQLite has
   subtle rules (the ``0`` prefix is *omitted* when ``val == 0`` and is
   placed *after* leading spaces in right-aligned widths).  The new
   implementation strips ``#`` from the Python format, lets Python compute
   the basic width/padding, then prepends ``0`` into the correct column.
"""

from __future__ import annotations

import pytest

from sql_vm.scalar_functions import call

# ---------------------------------------------------------------------------
# REPLACE(x, "", y) — empty needle is a no-op
# ---------------------------------------------------------------------------


class TestReplaceEmptyNeedle:
    def test_empty_needle_no_change(self) -> None:
        # Was 'XhXeXlXlXoX'; SQLite returns 'hello'.
        assert call("replace", ["hello", "", "X"]) == "hello"

    def test_empty_needle_empty_haystack(self) -> None:
        assert call("replace", ["", "", "X"]) == ""

    def test_empty_needle_empty_replacement(self) -> None:
        # Both empty: still a no-op.
        assert call("replace", ["hello", "", ""]) == "hello"

    def test_non_empty_needle_still_works(self) -> None:
        # Regression guard: the early-return only fires for empty needle.
        assert call("replace", ["hello", "l", "L"]) == "heLLo"

    def test_replace_with_longer_string(self) -> None:
        assert call("replace", ["aaa", "a", "bb"]) == "bbbbbb"


# ---------------------------------------------------------------------------
# printf("%#o", …) — C-style ``0`` prefix, not Python's ``0o``
# ---------------------------------------------------------------------------


class TestPrintfHashOctal:
    @pytest.mark.parametrize(
        ("fmt", "value", "expected"),
        [
            ("%#o", 0, "0"),       # zero gets no prefix
            ("%#o", 8, "010"),
            ("%#o", 64, "0100"),
            ("%#o", 1, "01"),
            # Right-aligned width: prefix sits *after* leading spaces.
            ("%#5o", 0, "    0"),
            ("%#5o", 8, "  010"),
            ("%#5o", 64, " 0100"),
            # Zero-padded width: total width grows by the prefix.
            ("%#05o", 0, "00000"),
            ("%#05o", 8, "000010"),
            ("%#05o", 64, "000100"),
        ],
    )
    def test_hash_octal(self, fmt: str, value: int, expected: str) -> None:
        assert call("printf", [fmt, value]) == expected

    @pytest.mark.parametrize(
        ("fmt", "value", "expected"),
        [
            # Without ``#`` flag — unchanged behaviour.
            ("%o", 8, "10"),
            ("%5o", 8, "   10"),
            ("%05o", 8, "00010"),
        ],
    )
    def test_plain_octal_unchanged(self, fmt: str, value: int, expected: str) -> None:
        assert call("printf", [fmt, value]) == expected

    def test_hash_hex_unaffected(self) -> None:
        # ``%#x`` was already producing ``0x…``, which matches SQLite.
        assert call("printf", ["%#x", 255]) == "0xff"

    def test_hash_HEX_uppercase(self) -> None:
        assert call("printf", ["%#X", 255]) == "0XFF"
