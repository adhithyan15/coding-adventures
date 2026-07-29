"""Unit tests for the SIR Python exception runtime."""

from __future__ import annotations

import pytest

from coding_adventures_sir_runtime_exceptions import (
    SirError,
    ancestry_chain,
    class_of_thrown,
    raise_error,
    register_ancestry,
    rescue_matches,
)
from coding_adventures_sir_runtime_exceptions import exceptions as _exc_mod


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
        # Without a registered ancestry edge, a user class matches only itself.
        assert rescue_matches(SirError("MyError"), ["MyError"]) is True
        assert rescue_matches(SirError("MyError"), ["StandardError"]) is False


@pytest.fixture(autouse=True)
def _restore_ancestry() -> None:
    """Snapshot and restore the live ancestry so a `register_ancestry` in one
    test cannot leak edges into another (the table is module-global mutable)."""
    saved = dict(_exc_mod._ANCESTRY)
    yield
    _exc_mod._ANCESTRY.clear()
    _exc_mod._ANCESTRY.update(saved)


class TestRegisterAncestry:
    """E2: user `class Child < Parent` edges threaded from the backend."""

    def test_registered_user_edge_matches_builtin_ancestor(self) -> None:
        # Before registration: user class matches only by exact name.
        assert rescue_matches(SirError("UserErr"), ["StandardError"]) is False
        register_ancestry({"UserErr": "StandardError"})
        # After: it descends from StandardError, and on up to Exception.
        assert rescue_matches(SirError("UserErr"), ["StandardError"]) is True
        assert rescue_matches(SirError("UserErr"), ["Exception"]) is True
        # Exact name still matches.
        assert rescue_matches(SirError("UserErr"), ["UserErr"]) is True

    def test_unrelated_user_class_still_not_matched(self) -> None:
        register_ancestry({"UserErr": "StandardError"})
        # A different user class with no edge does not match StandardError.
        assert rescue_matches(SirError("OtherErr"), ["StandardError"]) is False
        # And UserErr does not match an unrelated built-in it does not descend
        # from.
        assert rescue_matches(SirError("UserErr"), ["TypeError"]) is False

    def test_multi_level_user_chain(self) -> None:
        # Grandchild -> Child -> RuntimeError -> StandardError -> Exception.
        register_ancestry({"Child": "RuntimeError", "Grandchild": "Child"})
        assert rescue_matches(SirError("Grandchild"), ["RuntimeError"]) is True
        assert rescue_matches(SirError("Grandchild"), ["StandardError"]) is True
        assert rescue_matches(SirError("Grandchild"), ["Child"]) is True

    def test_registration_is_additive_and_idempotent(self) -> None:
        register_ancestry({"UserErr": "StandardError"})
        register_ancestry({"UserErr": "StandardError"})  # no-op re-register
        # Built-in edges are untouched.
        assert rescue_matches(SirError("ArgumentError"), ["StandardError"]) is True
        assert rescue_matches(SirError("UserErr"), ["StandardError"]) is True

    def test_self_referential_edge_does_not_loop(self) -> None:
        # A malformed self-edge must not hang the matcher (cycle guard).
        register_ancestry({"Loopy": "Loopy"})
        assert rescue_matches(SirError("Loopy"), ["Loopy"]) is True
        assert rescue_matches(SirError("Loopy"), ["StandardError"]) is False


class TestAncestryChain:
    """`ancestry_chain` exposes the ordered ancestry the OOP runtime's
    `is_a?` walks to look for an included module on each link (a question
    `rescue_matches`, a boolean name-walk, cannot answer)."""

    def test_builtin_chain_is_class_then_ancestors_in_order(self) -> None:
        assert ancestry_chain("ArgumentError") == [
            "ArgumentError",
            "StandardError",
            "Exception",
        ]

    def test_leaf_root_is_a_singleton_chain(self) -> None:
        assert ancestry_chain("Exception") == ["Exception"]

    def test_unregistered_class_is_its_own_only_link(self) -> None:
        assert ancestry_chain("TotallyUnknown") == ["TotallyUnknown"]

    def test_user_edge_extends_the_chain(self) -> None:
        register_ancestry({"UserErr": "RuntimeError"})
        assert ancestry_chain("UserErr") == [
            "UserErr",
            "RuntimeError",
            "StandardError",
            "Exception",
        ]

    def test_self_referential_edge_does_not_loop(self) -> None:
        # Cycle guard: a malformed self-edge terminates rather than hanging,
        # and the class appears at most once.
        register_ancestry({"Loopy": "Loopy"})
        assert ancestry_chain("Loopy") == ["Loopy"]

    def test_two_node_cycle_terminates(self) -> None:
        register_ancestry({"A": "B", "B": "A"})
        chain = ancestry_chain("A")
        assert chain[0] == "A"
        assert chain.count("A") == 1 and chain.count("B") == 1
