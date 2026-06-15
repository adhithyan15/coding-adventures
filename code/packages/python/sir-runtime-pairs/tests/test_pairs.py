"""Unit tests for the SIR Python cons-pair runtime."""

from __future__ import annotations

from typing import Any

import pytest

import coding_adventures_sir_runtime_pairs.pairs as pairs_mod
from coding_adventures_sir_runtime_pairs import (
    Pair,
    car,
    cdr,
    cons,
    is_pair,
    set_display,
)


class TestConsCarCdr:
    def test_cons_builds_a_pair_and_roundtrips(self) -> None:
        p = cons(1, 2)
        assert isinstance(p, Pair)
        assert car(p) == 1
        assert cdr(p) == 2

    def test_cons_fields_are_directly_accessible(self) -> None:
        p = cons("a", "b")
        assert p.car == "a"
        assert p.cdr == "b"

    def test_car_on_non_pair_raises_type_error(self) -> None:
        with pytest.raises(TypeError, match="car on non-pair"):
            car(42)

    def test_cdr_on_non_pair_raises_type_error(self) -> None:
        with pytest.raises(TypeError, match="cdr on non-pair"):
            cdr(None)


class TestIsPair:
    def test_true_for_a_pair(self) -> None:
        assert is_pair(cons(1, 2)) is True

    def test_false_for_non_pairs(self) -> None:
        assert is_pair(1) is False
        assert is_pair(None) is False
        assert is_pair([1, 2]) is False


class TestDefaultDisplay:
    def test_proper_list_repr(self) -> None:
        # cons(1, cons(2, cons(3, None))) is the proper list (1 2 3).
        p = cons(1, cons(2, cons(3, None)))
        assert repr(p) == "(1 2 3)"

    def test_single_element_proper_list(self) -> None:
        assert repr(cons(1, None)) == "(1)"

    def test_dotted_pair_repr(self) -> None:
        assert repr(cons(1, 2)) == "(1 . 2)"

    def test_nested_pair_repr(self) -> None:
        # A pair whose car is itself a pair: ((1 2) 3).
        inner = cons(1, cons(2, None))
        outer = cons(inner, cons(3, None))
        assert repr(outer) == "((1 2) 3)"

    def test_improper_tail_after_several_elements(self) -> None:
        # cons(1, cons(2, 3)) -> (1 2 . 3)
        assert repr(cons(1, cons(2, 3))) == "(1 2 . 3)"


class TestSetDisplay:
    def test_set_display_swaps_the_renderer(self) -> None:
        # Inject a renderer that maps None -> "nil" (Lisp-style) and otherwise
        # falls back to str.  This exercises set_display and proves the hook is
        # actually consulted per element.  (A None *tail* is always omitted by
        # the proper-list rule regardless of the hook, so we observe the swap on
        # a None *car* and on the body elements instead.)
        def custom(v: Any) -> str:
            return "nil" if v is None else f"<{v}>"

        original = pairs_mod._display
        try:
            set_display(custom)
            # A None car now renders as "nil"; the int tail goes through <...>.
            assert repr(cons(None, 2)) == "(nil . <2>)"
            # Every body element is routed through the injected renderer.
            assert repr(cons(1, cons(2, None))) == "(<1> <2>)"
        finally:
            # Restore the default renderer so other tests see plain str output.
            set_display(original)

    def test_default_display_restored_after_swap(self) -> None:
        # After the swap test restores the original, the default str renderer is
        # back: elements print via plain str, not the custom <...> form.
        assert repr(cons(1, cons(2, None))) == "(1 2)"
        assert pairs_mod._display is pairs_mod._default_display
