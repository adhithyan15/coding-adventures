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


# --- sir_write (SIR28 §2.1: __sys_write__) ----------------------------------
#
# SIR28 §7: the old `sir_print`/`sir_puts` functions (and their dedicated
# tests) are gone — every frontend now emits `__sys_write__`, which this
# package implements as `sir_write` below. `test_write_terminator_none_*`
# covers `print`'s old shape (`terminator: "none"`); `test_write_terminator_
# per_value_*`/`test_write_per_value_*`/`test_write_self_referential_array_
# terminates` cover `puts`'s old shape (`terminator: "per_value"`,
# `unpack_arrays: True`), including its array-flattening and cycle-safety
# behavior.


def test_write_terminator_none_writes_values_back_to_back(
    capsys: pytest.CaptureFixture[str],
) -> None:
    assert sir.sir_write("stdout", "none", False, "a", "b") is None
    assert capsys.readouterr().out == "ab"


def test_write_terminator_per_value_writes_one_newline_per_value(
    capsys: pytest.CaptureFixture[str],
) -> None:
    sir.sir_write("stdout", "per_value", False, 1, 2)
    assert capsys.readouterr().out == "1\n2\n"


def test_write_terminator_once_space_joins_with_one_trailing_newline(
    capsys: pytest.CaptureFixture[str],
) -> None:
    sir.sir_write("stdout", "once", False, 1, 2)
    assert capsys.readouterr().out == "1 2\n"


def test_write_per_value_with_unpack_arrays_flattens_nested_array(
    capsys: pytest.CaptureFixture[str],
) -> None:
    sir.sir_write("stdout", "per_value", True, [1, [2, 3], 4])
    assert capsys.readouterr().out == "1\n2\n3\n4\n"


def test_write_per_value_without_unpack_arrays_bracket_displays_array(
    capsys: pytest.CaptureFixture[str],
) -> None:
    sir.sir_write("stdout", "per_value", False, [1, 2])
    assert capsys.readouterr().out == "[1, 2]\n"


def test_write_stream_stderr_writes_to_stderr_not_stdout(
    capsys: pytest.CaptureFixture[str],
) -> None:
    sir.sir_write("stderr", "once", False, "oops")
    captured = capsys.readouterr()
    assert captured.out == ""
    assert captured.err == "oops\n"


def test_write_per_value_with_zero_values_writes_a_single_blank_line(
    capsys: pytest.CaptureFixture[str],
) -> None:
    sir.sir_write("stdout", "per_value", False)
    assert capsys.readouterr().out == "\n"


def test_write_does_not_suppress_trailing_newline_unlike_puts(
    capsys: pytest.CaptureFixture[str],
) -> None:
    # Deliberate divergence from sir_puts: __sys_write__ always appends
    # exactly one newline per value under "per_value", even when the
    # value's display form already ends in one (SIR28 §2.1's table, not
    # sir_puts's extra suppression nuance).
    sir.sir_write("stdout", "per_value", False, "x\n")
    assert capsys.readouterr().out == "x\n\n"


def test_write_self_referential_array_terminates(
    capsys: pytest.CaptureFixture[str],
) -> None:
    a: list = []
    a.append(a)
    assert sir.sir_write("stdout", "per_value", True, a) is None
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


def test_shift_left_integer() -> None:
    assert sir.shift_left(5, 2) == 20
    assert sir.shift_left(1, 0) == 1
    assert sir.shift_left(0, 5) == 0


def test_shift_left_negative_amount_reverses_direction() -> None:
    # Ruby: a negative shift amount reverses direction (a right shift).
    assert sir.shift_left(5, -1) == 2
    assert sir.shift_left(-8, -1) == -4


def test_shift_left_no_saturation_unlike_fixed_width_backends() -> None:
    # Python ints are arbitrary precision, so 1 << 63 is the TRUE
    # mathematical result -- unlike the C/Go/Rust backends, which saturate
    # at INT64_MAX as a documented v0 limitation. This is the MORE faithful
    # match to real Ruby's own bignum-growing `<<`.
    assert sir.shift_left(1, 63) == 9223372036854775808


def test_shift_left_array_pushes_in_place() -> None:
    # Ruby's Array#<< MUTATES the receiver (unlike `+`, which is
    # non-destructive) and chains: `a << 1 << 2` pushes both (the frontend
    # lowers a `<<` chain to one variadic call).
    a = [1, 2]
    result = sir.shift_left(a, 3, 4)
    assert result is a
    assert a == [1, 2, 3, 4]


def test_shift_left_string_concatenates_to_a_new_string() -> None:
    a = "ab"
    result = sir.shift_left(a, "cd")
    assert result == "abcd"
    assert a == "ab"  # original string untouched (str is immutable anyway)


def test_shift_left_string_non_string_operand_raises_type_error() -> None:
    # Python's own `+` already raises TypeError for a non-str RHS, matching
    # Ruby's TypeError for `<<` on an incompatible operand -- no explicit
    # check needed, mirroring `add`'s String arm.
    with pytest.raises(TypeError):
        sir.shift_left("ab", 1)


def test_shift_left_bool_amount_is_not_treated_as_a_shift_count() -> None:
    # bool is an int subclass in Python but is NOT a SIR shift amount (same
    # exclusion `mul`'s string/array repeat arms apply) -- contributes a 0
    # shift, matching the C/Go/Rust backends' catch-all.
    assert sir.shift_left(5, True) == 5


def test_shift_left_float_amount_truncates_toward_zero() -> None:
    assert sir.shift_left(1, 2.9) == 4  # int(2.9) == 2
    assert sir.shift_left(1, -2.9) == 0  # int(-2.9) == -2, right shift by 2


def test_shift_left_no_args_returns_zero() -> None:
    assert sir.shift_left() == 0


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


# ── SIR21 T3b-2: `trunc_div` / `utrunc_div` / `true_div` ─────────────────────
#
# `div` above IS `div_floor` (Ruby's floor semantics) — no separate test
# needed for that name. These three are genuinely new.


def test_trunc_div_truncates_toward_zero() -> None:
    # Unlike `div`'s floor semantics, every sign combination rounds toward
    # zero (matches C's integer `/`), verified against the same worked
    # example `div`'s own floor test uses so the divergence is explicit.
    assert sir.trunc_div(7, 2) == 3
    assert sir.trunc_div(-7, 2) == -3  # trunc, NOT floor's -4
    assert sir.trunc_div(7, -2) == -3
    assert sir.trunc_div(-7, -2) == 3
    assert sir.trunc_div(-6, 2) == -3  # exact division: trunc == floor


def test_trunc_div_exact_for_python_bignums() -> None:
    # `divmod` operates on Python's arbitrary-precision int directly, so
    # (unlike a fixed-width i64/int64 backend) there is no float-precision
    # loss and no MIN/-1 overflow edge case to guard. Truncation toward zero
    # is symmetric (trunc(-x / y) == -trunc(x / y) for y > 0); for a
    # POSITIVE dividend trunc and floor agree, so the positive side is
    # exactly `huge // 3` and the negative side must be its negation.
    huge = 10**30 + 7  # not exactly divisible by 3 -- exercises the adjustment
    assert huge % 3 != 0
    assert sir.trunc_div(huge, 3) == huge // 3
    assert sir.trunc_div(-huge, 3) == -(huge // 3)


def test_trunc_div_by_zero_raises_typed_zero_division_error() -> None:
    from coding_adventures_sir_runtime_exceptions import SirError

    with pytest.raises(SirError) as excinfo:
        sir.trunc_div(1, 0)
    assert excinfo.value.sir_class == "ZeroDivisionError"
    assert excinfo.value.args[0] == "divided by 0"


def test_utrunc_div_matches_trunc_div_python_has_no_fixed_width() -> None:
    # Python's int has no separate signed/unsigned representation, so
    # (unlike C/Go/Rust, which reinterpret bits) this backend's udiv_trunc
    # is identical to trunc_div — documented, not a bug.
    assert sir.utrunc_div(7, 2) == sir.trunc_div(7, 2) == 3
    assert sir.utrunc_div(-7, 2) == sir.trunc_div(-7, 2) == -3


def test_utrunc_div_by_zero_raises_typed_zero_division_error() -> None:
    from coding_adventures_sir_runtime_exceptions import SirError

    with pytest.raises(SirError) as excinfo:
        sir.utrunc_div(1, 0)
    assert excinfo.value.sir_class == "ZeroDivisionError"


def test_true_div_always_true_divides_even_on_integer_operands() -> None:
    # `div`'s own floor test asserts `sir.div(7, 2) == 3` (an int); true_div
    # on the SAME operands must be the float `3.5` — the entire point of
    # having a separate always-float op.
    assert sir.true_div(7, 2) == 3.5
    assert sir.true_div(-7, 2) == -3.5
    assert sir.true_div(6, 3) == 2.0
    assert isinstance(sir.true_div(6, 3), float)


def test_true_div_by_zero_raises_typed_zero_division_error() -> None:
    from coding_adventures_sir_runtime_exceptions import SirError

    with pytest.raises(SirError) as excinfo:
        sir.true_div(1, 0)
    assert excinfo.value.sir_class == "ZeroDivisionError"
    assert excinfo.value.args[0] == "divided by 0"


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
