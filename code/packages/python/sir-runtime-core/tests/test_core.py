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


def test_ne_is_exact_negation_of_eq() -> None:
    # `!=` must never disagree with `==`: symbol-aware, and native otherwise.
    assert sir.ne(sir.intern("a"), sir.intern("a")) is False
    assert sir.ne(sir.intern("a"), sir.intern("b")) is True
    assert sir.ne(1, 2) is True
    assert sir.ne(1, 1) is False
    assert sir.ne(1, 1.0) is False  # cross int/float equality


def test_le_and_ge() -> None:
    assert sir.le(1, 2) is True
    assert sir.le(2, 2) is True
    assert sir.le(3, 2) is False
    assert sir.le(1, 1.0) is True  # int vs float compares by value
    assert sir.ge(2, 1) is True
    assert sir.ge(2, 2) is True
    assert sir.ge(1, 2) is False


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


def test_display_convention_ruby_booleans() -> None:
    # The default convention is Lisp (`#t`/`#f`); a Ruby-sourced program
    # selects `true`/`false`.  Restore the default afterwards so the module
    # state does not leak into other tests.
    try:
        sir.set_display_convention("ruby")
        assert sir.to_display(True) == "true"
        assert sir.to_display(False) == "false"
        # Non-boolean forms are convention-independent.
        assert sir.to_display(None) == "nil"
        assert sir.to_display(sir.intern("sym")) == "sym"
        # An unrecognised convention falls back to the Lisp default.
        sir.set_display_convention("klingon")
        assert sir.to_display(True) == "#t"
    finally:
        sir.set_display_convention("lisp")
    assert sir.to_display(True) == "#t"


def test_print_uses_display(capsys: pytest.CaptureFixture[str]) -> None:
    assert sir.print(None) is None
    out = capsys.readouterr().out
    assert out == "nil\n"


# --- puts (Ruby semantics) -------------------------------------------------


def test_puts_no_args_prints_single_newline(
    capsys: pytest.CaptureFixture[str],
) -> None:
    assert sir.sir_puts() is None
    assert capsys.readouterr().out == "\n"


def test_puts_string_appends_newline(capsys: pytest.CaptureFixture[str]) -> None:
    sir.sir_puts("hello")
    assert capsys.readouterr().out == "hello\n"


def test_puts_does_not_double_trailing_newline(
    capsys: pytest.CaptureFixture[str],
) -> None:
    # Ruby: a value already ending in "\n" is not double-spaced.
    sir.sir_puts("x\n")
    assert capsys.readouterr().out == "x\n"


def test_puts_multiple_args_one_per_line(
    capsys: pytest.CaptureFixture[str],
) -> None:
    sir.sir_puts("a", "b")
    assert capsys.readouterr().out == "a\nb\n"


def test_puts_array_flattens_one_element_per_line(
    capsys: pytest.CaptureFixture[str],
) -> None:
    sir.sir_puts([1, 2, 3])
    assert capsys.readouterr().out == "1\n2\n3\n"
    # Nested arrays are flattened recursively.
    sir.sir_puts([1, [2, 3]])
    assert capsys.readouterr().out == "1\n2\n3\n"


def test_puts_empty_array_prints_single_newline(
    capsys: pytest.CaptureFixture[str],
) -> None:
    sir.sir_puts([])
    assert capsys.readouterr().out == "\n"


def test_puts_nil_prints_blank_line(capsys: pytest.CaptureFixture[str]) -> None:
    # `puts nil` is a blank line — NOT the display form "nil".
    sir.sir_puts(None)
    assert capsys.readouterr().out == "\n"


def test_puts_reference_program(capsys: pytest.CaptureFixture[str]) -> None:
    # The canonical execution-proof program: `puts "hello"; puts; puts [1,2,3]`
    sir.sir_puts("hello")
    sir.sir_puts()
    sir.sir_puts([1, 2, 3])
    assert capsys.readouterr().out == "hello\n\n1\n2\n3\n"


def test_puts_routes_through_call_builtin(
    capsys: pytest.CaptureFixture[str],
) -> None:
    # The backends dispatch `puts` by name through `call_builtin`.
    assert sir.call_builtin("puts", ["hi"]) is None
    assert capsys.readouterr().out == "hi\n"


def test_puts_self_referential_array_terminates(
    capsys: pytest.CaptureFixture[str],
) -> None:
    # Regression (security, CWE-674 uncontrolled recursion): `a = []; a << a;
    # puts a` in Ruby prints `[...]` and terminates.  Without the cycle guard
    # the element-per-line flatten recurses forever and raises RecursionError
    # (a DoS).  The guard must terminate AND render the cycle as Ruby's
    # `[...]`.
    a: list = []
    a.append(a)
    assert sir.sir_puts(a) is None
    assert capsys.readouterr().out == "[...]\n"


def test_puts_mutually_recursive_arrays_terminate(
    capsys: pytest.CaptureFixture[str],
) -> None:
    # A cycle through two arrays (a -> b -> a) is still a cycle on the flatten
    # path.  `puts a` flattens a's element (b, not yet seen), then b's element
    # (a, already on the path) → `[...]`.  Terminates rather than diverging.
    a: list = []
    b: list = [a]
    a.append(b)
    assert sir.sir_puts(a) is None
    assert capsys.readouterr().out == "[...]\n"


# --- arithmetic ------------------------------------------------------------


def test_variadic_arithmetic() -> None:
    assert sir.add(1, 2, 3) == 6
    assert sir.add() == 0
    assert sir.sub(10, 3, 2) == 5
    assert sir.sub(5) == -5
    assert sir.sub() == 0
    assert sir.mul(2, 3, 4) == 24
    assert sir.mul() == 1


def test_add_string_concat() -> None:
    # Ruby's `+` is polymorphic: two strings concatenate.
    assert sir.add("a", "b") == "ab"
    assert sir.add("foo", "bar", "baz") == "foobarbaz"
    # A single string argument is returned unchanged (fold of one).
    assert sir.add("solo") == "solo"


def test_add_array_concat() -> None:
    # SIR arrays are plain Python lists, so `+` concatenates them.
    assert sir.add([1], [2]) == [1, 2]
    assert sir.add([1, 2], [3], [4, 5]) == [1, 2, 3, 4, 5]


def test_add_array_concat_does_not_alias_operands() -> None:
    # Ruby's Array#+ is non-destructive: the inputs must be untouched.
    a = [1]
    b = [2]
    result = sir.add(a, b)
    assert result == [1, 2]
    result.append(99)
    assert a == [1]  # first operand not mutated
    assert b == [2]  # second operand not mutated


def test_mul_string_repeat() -> None:
    # "ab" * 3 -> "ababab"; non-positive counts -> "".
    assert sir.mul("ab", 3) == "ababab"
    assert sir.mul("ab", 0) == ""
    assert sir.mul("ab", -1) == ""


def test_mul_array_repeat() -> None:
    # [0] * 3 -> [0, 0, 0]; a fresh list, non-positive count -> [].
    assert sir.mul([0], 3) == [0, 0, 0]
    assert sir.mul([1, 2], 2) == [1, 2, 1, 2]
    assert sir.mul([1], 0) == []


def test_mul_array_repeat_does_not_alias_operand() -> None:
    src = [1]
    result = sir.mul(src, 3)
    assert result == [1, 1, 1]
    result.append(9)
    assert src == [1]  # source list untouched


def test_mul_array_join_with_string() -> None:
    # [1, 2] * ", " -> "1, 2" (element to_s via the canonical SIR display).
    assert sir.mul([1, 2], ", ") == "1, 2"
    assert sir.mul(["a", "b", "c"], "-") == "a-b-c"
    assert sir.mul([], ", ") == ""
    # Join uses to_display, not repr: nil renders as "nil", not "None".
    assert sir.mul([None, True], "|") == "nil|#t"


def test_mul_numeric_fold_unchanged() -> None:
    # Regression: the pure-numeric variadic fold is preserved exactly.
    assert sir.mul() == 1
    assert sir.mul(2, 3) == 6
    assert sir.mul(2, 3, 4) == 24
    assert sir.mul(5) == 5
    # Float promotion still works.
    assert sir.mul(2, 2.5) == 5.0


def test_mul_bool_count_is_not_treated_as_a_repeat() -> None:
    # bool is an int subclass in Python but NOT a SIR number (see
    # values.is_number), so the string/array repeat arms must exclude it: a
    # bool count is not an integer repeat count. `"ab" * True` therefore does
    # NOT go through the repeat arm — it falls through to the numeric fold,
    # where Python's `1 * "ab" * True` yields the string once ("ab"), which is
    # distinct from the repeat arm's `"ab" * True` (which would also be "ab"
    # here, but for larger truthy-int-like values the distinction matters and
    # the arm intentionally never fires on a bool).
    assert sir.mul("ab", True) == "ab"


def test_integer_division_floors_and_float_true_divides() -> None:
    # Ruby ``Integer#/`` floors toward −∞ (SIR21 §E3 DivOp::Floor) — every sign
    # combination, matching the Rust oracle exactly.
    assert sir.div(7, 2) == 3
    assert sir.div(-7, 2) == -4  # floors toward −∞, NOT truncate-to-zero (−3)
    assert sir.div(7, -2) == -4
    assert sir.div(-7, -2) == 3
    assert sir.div(-6, 2) == -3  # exact division: floor == trunc
    # ``Float#/`` true-divides — the old ``int(a / b)`` wrongly floored this to 3.
    assert sir.div(7.0, 2) == 3.5
    assert sir.div(7, 2.0) == 3.5
    # Variadic fold + identity element are unchanged.
    assert sir.div() == 0
    assert sir.div(9, 3, 1) == 3


# ── Typed division-by-zero (T1) ───────────────────────────────────────────────


def test_int_division_by_zero_raises_typed_zero_division_error() -> None:
    # Ruby's ``1 / 0`` raises ZeroDivisionError.  We re-raise Python's native
    # fault as a *typed* SirError so a Ruby ``rescue ZeroDivisionError`` matches
    # it (not merely the over-broad StandardError).
    from coding_adventures_sir_runtime_exceptions import SirError

    with pytest.raises(SirError) as excinfo:
        sir.div(1, 0)
    assert excinfo.value.sir_class == "ZeroDivisionError"
    assert excinfo.value.args[0] == "divided by 0"


def test_float_division_by_zero_raises_typed_zero_division_error() -> None:
    # Per the SIR error spec, ``1.0 / 0`` is also a ZeroDivisionError (Python's
    # ``1.0 / 0`` raises natively too, so the same wrap applies).
    from coding_adventures_sir_runtime_exceptions import SirError

    with pytest.raises(SirError) as excinfo:
        sir.div(1.0, 0)
    assert excinfo.value.sir_class == "ZeroDivisionError"


def test_division_by_zero_reports_the_step_that_faulted() -> None:
    # A variadic fold that only hits the zero divisor mid-chain still raises.
    from coding_adventures_sir_runtime_exceptions import SirError

    with pytest.raises(SirError) as excinfo:
        sir.div(10, 2, 0)
    assert excinfo.value.sir_class == "ZeroDivisionError"


def test_zero_division_error_is_rescue_matchable() -> None:
    # End-to-end: the typed error the runtime raises is caught by a rescue clause
    # naming ZeroDivisionError, its StandardError ancestor, or a bare rescue —
    # exactly the ``begin; 1/0; rescue ZeroDivisionError; end`` shape.
    from coding_adventures_sir_runtime_exceptions import rescue_matches

    try:
        sir.div(1, 0)
        raise AssertionError("div(1, 0) did not raise")  # pragma: no cover
    except Exception as exc:  # noqa: BLE001 - emitted rescue catches broadly then dispatches
        assert rescue_matches(exc, ["ZeroDivisionError"]) is True
        assert rescue_matches(exc, ["StandardError"]) is True  # ancestry walk
        assert rescue_matches(exc, []) is True  # bare rescue
        assert rescue_matches(exc, ["KeyError"]) is False  # unrelated clause misses


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


# --- proc-vs-lambda arity (Q10g) -------------------------------------------


def test_block_drops_extra_args() -> None:
    # A one-param block yielded two values binds the first, drops the rest
    # (Ruby proc/block leniency) rather than raising.
    c = sir.make_closure(lambda x: x, [])
    assert sir.apply(c, [1, 2, 3]) == 1


def test_block_pads_missing_args_with_nil() -> None:
    # Too few args → the missing trailing params become nil (None).
    c = sir.make_closure(lambda x, y: (x, y), [])
    assert sir.apply(c, [7]) == (7, None)


def test_block_arity_accounts_for_captures() -> None:
    # make_closure subtracts the capture count: this block's own arity is 1.
    c = sir.make_closure(lambda cap, x: (cap, x), [99])
    assert sir.apply(c, [1, 2, 3]) == (99, 1)


def test_variadic_block_is_not_adjusted() -> None:
    # A *rest block keeps every argument (arity is None → no trimming).
    c = sir.make_closure(lambda *xs: list(xs), [])
    assert sir.apply(c, [1, 2, 3]) == [1, 2, 3]


def test_lambda_is_strict_on_too_many_args() -> None:
    # as_lambda marks the closure strict, so a mismatch raises (the analogue
    # of Ruby's ArgumentError) instead of silently dropping args.
    c = sir.as_lambda(sir.make_closure(lambda x: x, []))
    assert sir.apply(c, [5]) == 5
    with pytest.raises(TypeError):
        sir.apply(c, [1, 2])


def test_lambda_is_strict_on_too_few_args() -> None:
    c = sir.as_lambda(sir.make_closure(lambda x, y: (x, y), []))
    with pytest.raises(TypeError):
        sir.apply(c, [1])


def test_as_lambda_returns_same_closure_and_sets_flag() -> None:
    c = sir.make_closure(lambda x: x, [])
    assert c.is_lambda is False
    assert sir.as_lambda(c) is c
    assert c.is_lambda is True


def test_as_lambda_passes_non_closure_through() -> None:
    assert sir.as_lambda(42) == 42
