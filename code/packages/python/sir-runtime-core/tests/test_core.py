"""Tests for coding-adventures-sir-runtime-core."""

from __future__ import annotations

import pytest

import coding_adventures_sir_runtime_core as sir

# --- truthiness (false/nil-only) -------------------------------------------


def test_only_false_and_nil_are_falsy() -> None:
    assert sir.truthy(False) is False
    assert sir.truthy(None) is False


@pytest.mark.parametrize("v", [0, 0.0, "", [], {}, 1, "x", [1], sir.intern("s")])
def test_everything_else_is_truthy(v: object) -> None:
    # The whole point of the library: 0/""/[]/{} are TRUE under SIR.
    assert sir.truthy(v) is True


# --- symbols (interned identity) -------------------------------------------


def test_interned_symbols_share_identity() -> None:
    assert sir.intern("a") is sir.intern("a")
    assert sir.intern("a") is not sir.intern("b")


def test_symbol_equality_and_repr() -> None:
    assert sir.intern("a") == sir.Symbol("a")
    assert repr(sir.intern("foo")) == "foo"
    assert hash(sir.intern("a")) == hash(sir.Symbol("a"))


def test_symbol_not_equal_to_string() -> None:
    assert sir.intern("a") != "a"


# --- pairs -----------------------------------------------------------------


def test_cons_car_cdr() -> None:
    p = sir.cons(1, 2)
    assert sir.car(p) == 1
    assert sir.cdr(p) == 2
    assert sir.is_pair(p) is True
    assert sir.is_pair(1) is False


def test_car_cdr_reject_non_pair() -> None:
    with pytest.raises(TypeError):
        sir.car(1)
    with pytest.raises(TypeError):
        sir.cdr(1)


def test_pair_list_display() -> None:
    proper = sir.cons(1, sir.cons(2, sir.cons(3, None)))
    assert sir.to_display(proper) == "(1 2 3)"
    dotted = sir.cons(1, 2)
    assert sir.to_display(dotted) == "(1 . 2)"


# --- equality + predicates -------------------------------------------------


def test_eq_symbol_aware() -> None:
    assert sir.eq(sir.intern("a"), sir.intern("a")) is True
    assert sir.eq(sir.intern("a"), sir.intern("b")) is False
    assert sir.eq(1, 1) is True
    assert sir.eq(1, 2) is False


def test_predicates() -> None:
    assert sir.is_null(None) is True
    assert sir.is_null(0) is False
    assert sir.is_number(3) is True
    assert sir.is_number(True) is False  # bool is not a SIR number
    assert sir.is_symbol(sir.intern("x")) is True
    assert sir.is_symbol("x") is False


# --- display ---------------------------------------------------------------


def test_to_display_forms() -> None:
    assert sir.to_display(None) == "nil"
    assert sir.to_display(True) == "#t"
    assert sir.to_display(False) == "#f"
    assert sir.to_display(sir.intern("sym")) == "sym"
    assert sir.to_display(42) == "42"
    assert sir.to_display("hi") == "hi"


def test_print_uses_display(capsys: pytest.CaptureFixture[str]) -> None:
    assert sir.print(None) is None
    out = capsys.readouterr().out
    assert out == "nil\n"


# --- arithmetic ------------------------------------------------------------


def test_variadic_arithmetic() -> None:
    assert sir.add(1, 2, 3) == 6
    assert sir.add() == 0
    assert sir.sub(10, 3, 2) == 5
    assert sir.sub(5) == -5
    assert sir.sub() == 0
    assert sir.mul(2, 3, 4) == 24
    assert sir.mul() == 1


def test_truncating_division() -> None:
    assert sir.div(7, 2) == 3
    assert sir.div(-7, 2) == -3  # truncates toward zero, not floor
    assert sir.div() == 0
    assert sir.div(9, 3, 1) == 3


def test_comparisons() -> None:
    assert sir.lt(1, 2) is True
    assert sir.gt(2, 1) is True
    assert sir.lt(2, 1) is False


# --- closures --------------------------------------------------------------


def test_make_closure_prepends_captures() -> None:
    def f(a: int, b: int, c: int) -> int:
        return a + b + c

    c = sir.make_closure(f, [10, 20])
    assert sir.apply(c, [5]) == 35


def test_apply_rejects_non_closure() -> None:
    with pytest.raises(TypeError):
        sir.apply(42, [])


def test_apply_nil_target_raises_local_jump_error() -> None:
    # No-block-given case: a `yield` reached through a nil block parameter
    # (apply target is None) raises the dedicated LocalJumpError, not the
    # generic non-closure TypeError.
    with pytest.raises(sir.LocalJumpError):
        sir.apply(None, [1, 2])


def test_local_jump_error_message_mentions_no_block() -> None:
    with pytest.raises(sir.LocalJumpError, match="no block given"):
        sir.apply(None, [])


def test_local_jump_error_is_distinct_from_type_error() -> None:
    # The nil case must NOT be a TypeError (so the two failure modes stay
    # distinguishable for callers / future rescue mapping).
    assert not issubclass(sir.LocalJumpError, TypeError)


# --- globals ---------------------------------------------------------------


def test_global_store_roundtrip() -> None:
    sir.global_set("g1", 99)
    assert sir.global_get("g1") == 99
    assert sir.global_get_static("g1") == 99
    sir.global_set(sir.intern("g2"), 7)
    assert sir.global_get(sir.intern("g2")) == 7


def test_global_get_undefined_errors() -> None:
    with pytest.raises(NameError):
        sir.global_get("never_defined")
    with pytest.raises(NameError):
        sir.global_get_static("also_never")


# --- builtin dispatch ------------------------------------------------------


def test_call_builtin() -> None:
    assert sir.call_builtin("+", [1, 2, 3]) == 6
    assert sir.call_builtin("print", [None]) is None


def test_call_unknown_builtin_errors() -> None:
    with pytest.raises(NameError):
        sir.call_builtin("nope", [])


def test_builtin_closure_is_callable_handle() -> None:
    plus = sir.builtin_closure("+")
    assert sir.apply(plus, [4, 5]) == 9
