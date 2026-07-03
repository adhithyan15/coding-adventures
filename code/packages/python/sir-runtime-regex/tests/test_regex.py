"""Unit tests for the SIR Python regex runtime."""

from __future__ import annotations

import re

from coding_adventures_sir_runtime_regex import (
    compile,
    is_match,
    match_data,
)


class TestFlagTranslation:
    def test_no_flags_still_always_multiline(self) -> None:
        # Ruby's ^/$ are always line anchors, so MULTILINE is unconditional even
        # with an empty flag string — and nothing else is set.
        p = compile("a")
        assert p.flags & re.MULTILINE
        assert not (p.flags & re.IGNORECASE)
        assert not (p.flags & re.DOTALL)
        assert not (p.flags & re.VERBOSE)

    def test_i_maps_to_ignorecase(self) -> None:
        p = compile("a", "i")
        assert p.flags & re.IGNORECASE
        assert p.flags & re.MULTILINE  # still always on

    def test_m_maps_to_dotall_not_python_multiline_semantics(self) -> None:
        # Ruby /m means "dot matches newline" == Python DOTALL. (Python's own
        # re.MULTILINE is the always-on line-anchor flag, set independently.)
        p = compile(".", "m")
        assert p.flags & re.DOTALL
        # Proof of behaviour: with DOTALL, "." now spans a newline.
        assert p.search("\n") is not None
        # Without /m, "." would not match a newline.
        assert compile(".").search("\n") is None

    def test_x_maps_to_verbose(self) -> None:
        p = compile("a", "x")
        assert p.flags & re.VERBOSE

    def test_combined_flags(self) -> None:
        p = compile("a", "imx")
        assert p.flags & re.IGNORECASE
        assert p.flags & re.DOTALL
        assert p.flags & re.VERBOSE
        assert p.flags & re.MULTILINE

    def test_unknown_flag_chars_are_ignored(self) -> None:
        # 'z', 'o', 'u' are not in the mapping; they must contribute nothing
        # (but MULTILINE remains on, and a real flag alongside still applies).
        p = compile("a", "zoiu")
        assert p.flags & re.IGNORECASE
        assert p.flags & re.MULTILINE

    def test_verbose_ignores_unescaped_whitespace(self) -> None:
        # Under VERBOSE, the spaces in the pattern are not literal, so this still
        # matches "ab" with no spaces.
        p = compile("a b", "x")
        assert p.search("ab") is not None


class TestIsMatch:
    def test_true_unanchored_search(self) -> None:
        # Ruby =~ is an unanchored search: a hit anywhere counts, not fullmatch.
        assert is_match(r"\d+", "abc 42 xyz") is True

    def test_false_when_no_match(self) -> None:
        assert is_match(r"\d+", "no digits here") is False

    def test_accepts_a_precompiled_pattern(self) -> None:
        pat = compile(r"\d+", "")
        assert is_match(pat, "x9") is True
        assert is_match(pat, "none") is False

    def test_accepts_a_raw_string_pattern(self) -> None:
        assert is_match("foo", "a foo b") is True


class TestMatchData:
    def test_returns_group_zero(self) -> None:
        assert match_data(r"\d+", "abc 42 xyz") == "42"

    def test_returns_none_on_no_match(self) -> None:
        assert match_data(r"\d+", "no digits") is None

    def test_accepts_a_precompiled_pattern(self) -> None:
        pat = compile(r"[a-z]+", "")
        assert match_data(pat, "  HELLO world") == "world"
