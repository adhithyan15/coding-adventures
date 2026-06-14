"""Exhaustive edge-case tests for ``SUBSTR(x, y[, z])``.

SQLite's substr has three corners that catch most implementations:

1. ``y = 0`` is one position *to the left* of the first character — not
   a sentinel for "start of string".  Combined with positive ``z``, the
   request span includes position 0 (invalid) so the returned string is
   one character shorter than ``z``.

2. **Negative ``y``** counts from the end: ``y = -1`` is the last
   character.  Concretely ``y`` resolves to ``N + 1 + y`` (so on an
   empty string, ``y = -1`` resolves to 0 — same as the previous
   corner).

3. **Negative ``z``** asks for ``|z|`` characters *preceding* position
   ``y`` — useful for "take everything up to but not including this
   point".

Before this PR mini-sqlite got every one of those wrong (e.g.
``substr('hello', 0, 3)`` returned ``'hel'`` instead of ``'he'``).  The
new implementation models the requested character range as a closed
1-indexed interval ``[lo, hi]``, clips to ``[1, N]``, and converts to a
Python slice — uniform handling that doesn't accumulate per-branch
fixups.
"""

from __future__ import annotations

import pytest

from sql_vm.scalar_functions import call


def _s(*args: object) -> object:
    return call("substr", list(args))


# ---------------------------------------------------------------------------
# Standard positive-y, positive-z — sanity baseline
# ---------------------------------------------------------------------------


class TestSubstrBaseline:
    def test_basic(self) -> None:
        assert _s("hello", 1, 3) == "hel"

    def test_from_middle(self) -> None:
        assert _s("hello", 2, 3) == "ell"

    def test_to_end(self) -> None:
        assert _s("hello", 2) == "ello"

    def test_overshoot_length(self) -> None:
        assert _s("hello", 2, 100) == "ello"

    def test_zero_length(self) -> None:
        assert _s("hello", 2, 0) == ""


# ---------------------------------------------------------------------------
# Negative ``y`` — count from end (resolved as N + 1 + y)
# ---------------------------------------------------------------------------


class TestNegativeStart:
    def test_minus_one_is_last_char(self) -> None:
        assert _s("hello", -1) == "o"

    def test_minus_three_from_end(self) -> None:
        assert _s("hello", -3) == "llo"

    def test_minus_n_is_whole_string(self) -> None:
        assert _s("hello", -5) == "hello"

    def test_negative_overshoot_no_length(self) -> None:
        # y = -100 → resolved y = -94; from -94 to end means clipped to
        # [1, 5] which is the whole string.
        assert _s("hello", -100) == "hello"


# ---------------------------------------------------------------------------
# ``y = 0`` — one position before the string
# ---------------------------------------------------------------------------


class TestZeroStart:
    def test_zero_with_length_3(self) -> None:
        # Span: positions 0,1,2 → ∩ [1,5] = positions 1,2 → 'he'.
        assert _s("hello", 0, 3) == "he"

    def test_zero_with_length_1(self) -> None:
        # Span: position 0 → ∩ [1,5] = empty.
        assert _s("hello", 0, 1) == ""

    def test_zero_with_length_5(self) -> None:
        # Span: positions 0..4 → ∩ [1,5] = positions 1..4 → 'hell'.
        assert _s("hello", 0, 5) == "hell"

    def test_zero_with_zero_length(self) -> None:
        assert _s("hello", 0, 0) == ""

    def test_zero_no_length(self) -> None:
        # y=0, no length → from 0 to end → clipped to [1, 5] → 'hello'.
        assert _s("hello", 0) == "hello"


# ---------------------------------------------------------------------------
# Negative ``z`` — characters preceding position y
# ---------------------------------------------------------------------------


class TestNegativeLength:
    def test_minus_one_takes_one_before(self) -> None:
        # 1 char before position 2 = position 1 = 'h'.
        assert _s("hello", 2, -1) == "h"

    def test_minus_three_clipped(self) -> None:
        # 3 chars before position 2 = positions -1, 0, 1 → only 1 valid → 'h'.
        assert _s("hello", 2, -3) == "h"

    def test_minus_two_at_position_three(self) -> None:
        # 2 chars before position 3 = positions 1, 2 = 'he'.
        assert _s("hello", 3, -2) == "he"

    def test_negative_length_from_end(self) -> None:
        # y=5 is last char; -2 means positions 3, 4 = 'll'.
        assert _s("hello", 5, -2) == "ll"

    def test_negative_length_negative_start(self) -> None:
        # y=-2 → position 4. z=-2 → positions 2, 3 = 'el'.
        assert _s("hello", -2, -2) == "el"


# ---------------------------------------------------------------------------
# Out-of-range — start way before or way after the string
# ---------------------------------------------------------------------------


class TestOutOfRange:
    def test_far_negative_short_length(self) -> None:
        # y=-100 → resolved -94. z=5. Span [-94, -90] entirely left of [1, 5].
        assert _s("hello", -100, 5) == ""

    def test_far_negative_long_length(self) -> None:
        # y=-94, z=102. Span [-94, 7] ∩ [1, 5] = [1, 5] → 'hello'.
        assert _s("hello", -100, 102) == "hello"

    def test_start_past_end(self) -> None:
        assert _s("hello", 6) == ""
        assert _s("hello", 100) == ""

    def test_start_past_end_with_length(self) -> None:
        assert _s("hello", 100, 5) == ""


# ---------------------------------------------------------------------------
# Empty input
# ---------------------------------------------------------------------------


class TestEmptyInput:
    def test_empty_string(self) -> None:
        assert _s("", 1) == ""
        assert _s("", 1, 5) == ""
        assert _s("", -1) == ""

    def test_null_input(self) -> None:
        assert _s(None, 1) is None
        assert _s(None, 1, 3) is None


# ---------------------------------------------------------------------------
# Blob inputs — operate on bytes with the same algorithm
# ---------------------------------------------------------------------------


class TestBlobSubstr:
    def test_blob_basic(self) -> None:
        assert _s(b"hello", 1, 3) == b"hel"

    def test_blob_zero_start(self) -> None:
        assert _s(b"hello", 0, 3) == b"he"

    def test_blob_negative_start(self) -> None:
        assert _s(b"hello", -3) == b"llo"

    def test_blob_negative_length(self) -> None:
        assert _s(b"hello", 3, -2) == b"he"


# ---------------------------------------------------------------------------
# substring alias (SQL-standard spelling)
# ---------------------------------------------------------------------------


class TestSubstringAlias:
    @pytest.mark.parametrize(
        ("args", "expected"),
        [
            (("hello", 1, 3), "hel"),
            (("hello", 0, 3), "he"),
            (("hello", -3), "llo"),
            (("hello", 3, -2), "he"),
        ],
    )
    def test_substring_matches_substr(self, args: tuple, expected: str) -> None:
        assert call("substring", list(args)) == expected
