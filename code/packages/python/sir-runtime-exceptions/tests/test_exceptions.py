"""Unit tests for the SIR Python exception runtime."""

from __future__ import annotations

import pytest

from coding_adventures_sir_runtime_exceptions import (
    SirError,
    class_of_thrown,
    raise_error,
    rescue_matches,
)


class TestSirError:
    def test_tags_exception_with_sir_class_and_message(self) -> None:
        e = SirError("ArgumentError", "bad arg")
        assert e.sir_class == "ArgumentError"
        assert str(e) == "bad arg"

    def test_message_defaults_to_class_name(self) -> None:
        assert str(SirError("RuntimeError")) == "RuntimeError"
        assert str(SirError("RuntimeError", None)) == "RuntimeError"

    def test_stringifies_non_string_message(self) -> None:
        assert str(SirError("E", 42)) == "42"

    def test_is_a_real_exception(self) -> None:
        assert isinstance(SirError("E"), Exception)


class TestRaiseError:
    def test_raises_sir_error_of_named_class_with_message(self) -> None:
        with pytest.raises(SirError) as info:
            raise_error("TypeError", "nope")
        assert info.value.sir_class == "TypeError"
        assert str(info.value) == "nope"

    def test_bare_reraise_defaults_to_runtime_error(self) -> None:
        with pytest.raises(SirError) as info:
            raise_error()
        assert info.value.sir_class == "RuntimeError"


class TestClassOfThrown:
    def test_reports_sir_error_tag(self) -> None:
        assert class_of_thrown(SirError("KeyError")) == "KeyError"

    def test_buckets_native_errors_as_standard_error(self) -> None:
        assert class_of_thrown(ValueError("native")) == "StandardError"
        assert class_of_thrown(RuntimeError()) == "StandardError"


class TestRescueMatches:
    def test_empty_list_is_catch_all(self) -> None:
        assert rescue_matches(SirError("Anything"), []) is True
        assert rescue_matches(ValueError("x"), []) is True

    def test_matches_exact_class_name(self) -> None:
        assert rescue_matches(SirError("ArgumentError"), ["ArgumentError"]) is True

    def test_matches_ancestor_via_builtin_hierarchy(self) -> None:
        assert rescue_matches(SirError("ArgumentError"), ["StandardError"]) is True
        assert rescue_matches(SirError("KeyError"), ["IndexError"]) is True
        assert rescue_matches(SirError("NoMethodError"), ["NameError"]) is True

    def test_exception_is_universal_root(self) -> None:
        assert rescue_matches(SirError("WhateverError"), ["Exception"]) is True
        assert rescue_matches(ValueError("native"), ["Exception"]) is True

    def test_standard_error_catches_native_python_errors(self) -> None:
        assert rescue_matches(ValueError("native"), ["StandardError"]) is True

    def test_does_not_match_unrelated_class(self) -> None:
        assert rescue_matches(SirError("TypeError"), ["ArgumentError"]) is False
        assert rescue_matches(SirError("RuntimeError"), ["KeyError"]) is False

    def test_matches_when_any_listed_class_matches(self) -> None:
        assert rescue_matches(SirError("TypeError"), ["KeyError", "TypeError"]) is True

    def test_user_class_matches_only_by_exact_name(self) -> None:
        assert rescue_matches(SirError("MyError"), ["MyError"]) is True
        assert rescue_matches(SirError("MyError"), ["StandardError"]) is False
