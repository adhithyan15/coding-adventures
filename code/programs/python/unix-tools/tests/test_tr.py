"""Tests for the tr tool.

=== What These Tests Verify ===

These tests exercise the tr implementation, including:

1. Spec loading and CLI Builder integration
2. Character translation
3. Character deletion (-d)
4. Squeeze repeats (-s)
5. Complement (-c)
6. Character set expansion (ranges, classes)
7. Business logic functions
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "tr.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from tr_tool import (
    complement_set,
    expand_set,
    tr_delete,
    tr_squeeze_only,
    tr_translate,
)


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


class TestSpecLoading:
    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_basic_parse(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["tr", "a-z", "A-Z"])
        assert isinstance(result, ParseResult)


class TestFlags:
    def test_delete_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["tr", "-d", "aeiou"])
        assert isinstance(result, ParseResult)
        assert result.flags["delete"] is True

    def test_squeeze_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["tr", "-s", " "])
        assert isinstance(result, ParseResult)
        assert result.flags["squeeze_repeats"] is True

    def test_complement_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["tr", "-c", "a-z", "?"])
        assert isinstance(result, ParseResult)
        assert result.flags["complement"] is True


class TestHelpVersion:
    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["tr", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["tr", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestExpandSet:
    def test_literal_chars(self) -> None:
        assert expand_set("abc") == "abc"

    def test_range(self) -> None:
        result = expand_set("a-e")
        assert result == "abcde"

    def test_digit_range(self) -> None:
        result = expand_set("0-9")
        assert result == "0123456789"

    def test_upper_class(self) -> None:
        result = expand_set("[:upper:]")
        assert result == "ABCDEFGHIJKLMNOPQRSTUVWXYZ"

    def test_lower_class(self) -> None:
        result = expand_set("[:lower:]")
        assert result == "abcdefghijklmnopqrstuvwxyz"

    def test_escape_newline(self) -> None:
        result = expand_set("\\n")
        assert result == "\n"

    def test_escape_tab(self) -> None:
        result = expand_set("\\t")
        assert result == "\t"


class TestTranslate:
    def test_lowercase_to_uppercase(self) -> None:
        result = tr_translate("hello", "abcdefghijklmnopqrstuvwxyz",
                              "ABCDEFGHIJKLMNOPQRSTUVWXYZ", squeeze=False)
        assert result == "HELLO"

    def test_partial_translation(self) -> None:
        result = tr_translate("abc", "ab", "xy", squeeze=False)
        assert result == "xyc"

    def test_set2_shorter(self) -> None:
        # When SET2 is shorter, its last char is repeated.
        result = tr_translate("abc", "abc", "x", squeeze=False)
        assert result == "xxx"

    def test_squeeze_after_translate(self) -> None:
        result = tr_translate("aabbcc", "abc", "xyz", squeeze=True)
        assert result == "xyz"


class TestDelete:
    def test_delete_vowels(self) -> None:
        result = tr_delete("hello world", "aeiou", squeeze=False, squeeze_set="")
        assert result == "hll wrld"

    def test_delete_nothing(self) -> None:
        result = tr_delete("hello", "", squeeze=False, squeeze_set="")
        assert result == "hello"

    def test_delete_and_squeeze(self) -> None:
        result = tr_delete("aabbcc", "a", squeeze=True, squeeze_set="bc")
        assert result == "bc"


class TestSqueezeOnly:
    def test_squeeze_spaces(self) -> None:
        result = tr_squeeze_only("hello   world", " ")
        assert result == "hello world"

    def test_squeeze_letters(self) -> None:
        result = tr_squeeze_only("aabbbcccc", "abc")
        assert result == "abc"

    def test_no_squeeze_needed(self) -> None:
        result = tr_squeeze_only("hello", "aeiou")
        assert result == "hello"


class TestComplement:
    def test_complement_contains_other_chars(self) -> None:
        comp = complement_set("abc")
        assert "a" not in comp
        assert "b" not in comp
        assert "d" in comp
        assert "z" in comp
