"""Tests for coding-adventures-sir-runtime-oop."""

from __future__ import annotations

import pytest
from coding_adventures_sir_runtime_core import Closure
from coding_adventures_sir_runtime_exceptions import SirError

import coding_adventures_sir_runtime_oop as oop
from coding_adventures_sir_runtime_oop import Val


@pytest.fixture(autouse=True)
def _reset() -> None:
    oop.reset_oop()


# ── class registry + ancestry ─────────────────────────────────────────────────


def test_records_superclass_and_resolves_direct_is_a() -> None:
    oop.define_class("Animal", None)
    oop.define_class("Dog", "Animal")
    assert oop.superclass_of("Dog") == "Animal"
    assert oop.superclass_of("Animal") is None
    d = oop.new_instance("Dog")
    assert oop.is_a(d, "Dog") is True
    assert oop.is_a(d, "Animal") is True
    assert oop.is_a(d, "Cat") is False


def test_object_basicobject_match_everything_and_cycles_terminate() -> None:
    oop.define_class("A", "B")
    oop.define_class("B", "A")  # pathological cycle
    a = oop.new_instance("A")
    assert oop.is_a(a, "Object") is True
    assert oop.is_a(a, "BasicObject") is True
    assert oop.is_a(a, "A") is True
    assert oop.is_a(a, "B") is True
    assert oop.is_a(a, "Z") is False


def test_redefining_a_class_replaces_its_registration() -> None:
    oop.define_class("C", "X")
    oop.define_class("C", "Y")
    assert oop.superclass_of("C") == "Y"


# ── class_of primitive mapping ────────────────────────────────────────────────


def test_class_of_maps_python_values_to_ruby_class_names() -> None:
    assert oop.class_of(None) == "NilClass"
    assert oop.class_of(True) == "TrueClass"
    assert oop.class_of(False) == "FalseClass"
    assert oop.class_of(3) == "Integer"
    assert oop.class_of(3.5) == "Float"
    assert oop.class_of("hi") == "String"
    assert oop.class_of([1, 2]) == "Array"
    assert oop.class_of({"a": 1}) == "Hash"
    assert oop.class_of(oop.new_instance("Foo")) == "Foo"


def test_is_a_handles_primitives_and_the_numeric_umbrella() -> None:
    assert oop.is_a(3, "Integer") is True
    assert oop.is_a(3, "Numeric") is True
    assert oop.is_a(3.5, "Numeric") is True
    assert oop.is_a("x", "Numeric") is False
    assert oop.is_a("x", "String") is True


# ── instance-variable store via current-self stack ──────────────────────────


def test_ivar_reads_nil_before_set_then_round_trips_on_default_self() -> None:
    assert oop.ivar_get("@x") is None
    assert oop.ivar_set("@x", 7) == 7
    assert oop.ivar_get("@x") == 7


def test_push_pop_self_isolates_instance_variables_per_object() -> None:
    a = oop.new_instance("Foo")
    b = oop.new_instance("Foo")
    oop.push_self(a)
    oop.ivar_set("@v", "a")
    oop.pop_self()
    oop.push_self(b)
    oop.ivar_set("@v", "b")
    assert oop.ivar_get("@v") == "b"
    oop.pop_self()
    oop.push_self(a)
    assert oop.ivar_get("@v") == "a"
    oop.pop_self()


def test_pop_self_on_empty_stack_is_safe() -> None:
    oop.pop_self()  # no-op, must not raise
    assert oop.ivar_get("@anything") is None


# ── class-variable store ─────────────────────────────────────────────────────


def test_cvar_reads_nil_before_set_then_round_trips() -> None:
    assert oop.cvar_get("@@count") is None
    assert oop.cvar_set("@@count", 0) == 0
    assert oop.cvar_set("@@count", 1) == 1
    assert oop.cvar_get("@@count") == 1


# ── method dispatch ──────────────────────────────────────────────────────────


def test_is_a_kind_of_accept_a_class_name_string_or_value() -> None:
    oop.define_class("Animal", None)
    oop.define_class("Dog", "Animal")
    d = oop.new_instance("Dog")
    assert oop.call_method(d, "is_a?", "Animal") is True
    assert oop.call_method(d, "kind_of?", "Dog") is True
    assert oop.call_method(3, "is_a?", "Integer") is True
    assert oop.call_method(3, "is_a?", 99) is True  # class taken from the value


def test_instance_of_requires_exact_non_ancestor_match() -> None:
    oop.define_class("Animal", None)
    oop.define_class("Dog", "Animal")
    d = oop.new_instance("Dog")
    assert oop.call_method(d, "instance_of?", "Dog") is True
    assert oop.call_method(d, "instance_of?", "Animal") is False


def test_class_returns_class_name_and_unknown_methods_raise_no_method_error() -> None:
    assert oop.call_method(oop.new_instance("Foo"), "class") == "Foo"
    assert oop.call_method(3, "class") == "Integer"
    # A genuinely unknown method now raises a typed NoMethodError (T1), not the
    # old nil floor — so a Ruby ``rescue NoMethodError`` catches it.
    with pytest.raises(SirError) as excinfo:
        oop.call_method(3, "no_such_method")
    assert excinfo.value.sir_class == "NoMethodError"
    assert excinfo.value.args[0] == "undefined method 'no_such_method' for Integer"


def test_define_method_backs_the_dispatch_fallback() -> None:
    oop.define_method("double", lambda recv, args: recv * 2)
    assert oop.call_method(21, "double") == 42


def test_sir_instance_carries_class_tag_and_empty_ivar_bag() -> None:
    i = oop.SirInstance("Widget")
    assert i.sir_class == "Widget"
    assert i.ivars == {}


# ── built-in method catalog: non-block Array (M1a) ────────────────────────────


def test_array_length_size_count() -> None:
    assert oop.call_method([1, 2, 3], "length") == 3
    assert oop.call_method([1, 2, 3], "size") == 3
    assert oop.call_method([1, 2, 3], "count") == 3
    assert oop.call_method([1, 2, 2, 3], "count", 2) == 2


def test_array_first_last_with_and_without_count() -> None:
    assert oop.call_method([1, 2, 3], "first") == 1
    assert oop.call_method([1, 2, 3], "last") == 3
    assert oop.call_method([1, 2, 3], "first", 2) == [1, 2]
    assert oop.call_method([1, 2, 3], "last", 2) == [2, 3]
    assert oop.call_method([], "first") is None
    assert oop.call_method([], "last") is None
    assert oop.call_method([1, 2], "last", 0) == []


def test_array_include_and_index() -> None:
    assert oop.call_method([1, 2, 3], "include?", 2) is True
    assert oop.call_method([1, 2, 3], "include?", 9) is False
    assert oop.call_method([1, 2, 3], "index", 3) == 2
    assert oop.call_method([1, 2, 3], "index", 9) is None


def test_array_fetch_in_range_and_negative_index() -> None:
    # ``Array#fetch`` returns the element for an in-range index (negative counts
    # from the end), like ``arr[i]`` — the difference is only on out-of-bounds.
    assert oop.call_method([10, 20, 30], "fetch", 0) == 10
    assert oop.call_method([10, 20, 30], "fetch", 2) == 30
    assert oop.call_method([10, 20, 30], "fetch", -1) == 30


def test_array_fetch_out_of_bounds_with_default_returns_default() -> None:
    # A second argument supplies the value returned instead of raising.
    assert oop.call_method([10, 20], "fetch", 5, 99) == 99


def test_array_fetch_out_of_bounds_raises_index_error() -> None:
    # Ruby ``Array#fetch`` out of bounds with no default raises IndexError (T1) —
    # unlike ``arr[i]``, which returns nil.
    with pytest.raises(SirError) as excinfo:
        oop.call_method([10, 20], "fetch", 100)
    assert excinfo.value.sir_class == "IndexError"
    assert excinfo.value.args[0] == "index 100 outside of array bounds: -2...2"


def test_array_take_and_drop_clamp_count() -> None:
    # ``take(n)``/``drop(n)`` split the array at ``n``, which is clamped to
    # ``[0, len]``: an over-long count saturates and a negative count folds to 0.
    assert oop.call_method([1, 2, 3, 4], "take", 2) == [1, 2]
    assert oop.call_method([1, 2, 3, 4], "drop", 2) == [3, 4]
    assert oop.call_method([1, 2, 3], "take", 0) == []
    assert oop.call_method([1, 2, 3], "drop", 0) == [1, 2, 3]
    assert oop.call_method([1, 2, 3], "take", 99) == [1, 2, 3]
    assert oop.call_method([1, 2, 3], "drop", 99) == []
    assert oop.call_method([1, 2, 3], "take", -5) == []
    assert oop.call_method([1, 2, 3], "drop", -5) == [1, 2, 3]


def test_array_values_at_selects_and_folds_negatives() -> None:
    # ``values_at(*idxs)`` returns one element per index; a negative index folds
    # from the end once, and an out-of-range index yields ``None`` (never raises).
    assert oop.call_method([10, 20, 30], "values_at", 0, 2) == [10, 30]
    assert oop.call_method([10, 20, 30], "values_at", -1, -2) == [30, 20]
    assert oop.call_method([10, 20, 30], "values_at", 5, -9) == [None, None]
    assert oop.call_method([10, 20, 30], "values_at") == []


def test_array_rotate_wraps_left_and_right() -> None:
    # ``rotate(n=1)`` rotates left by ``n``; a negative ``n`` rotates right and the
    # modulo wraps any magnitude.  Empty array stays empty.
    assert oop.call_method([1, 2, 3, 4], "rotate") == [2, 3, 4, 1]
    assert oop.call_method([1, 2, 3, 4], "rotate", 2) == [3, 4, 1, 2]
    assert oop.call_method([1, 2, 3, 4], "rotate", -1) == [4, 1, 2, 3]
    assert oop.call_method([1, 2, 3, 4], "rotate", 6) == [3, 4, 1, 2]
    assert oop.call_method([1, 2, 3], "rotate", 0) == [1, 2, 3]
    assert oop.call_method([], "rotate", 3) == []


def test_array_zip_pads_and_truncates_to_receiver_length() -> None:
    # ``zip(*others)`` yields one tuple per receiver element; a shorter operand pads
    # with ``None`` and a longer one is truncated to the receiver length.
    assert oop.call_method([1, 2, 3], "zip", [4, 5, 6]) == [[1, 4], [2, 5], [3, 6]]
    assert oop.call_method([1, 2, 3], "zip", [4, 5]) == [[1, 4], [2, 5], [3, None]]
    assert oop.call_method([1, 2], "zip", [3, 4, 5]) == [[1, 3], [2, 4]]
    assert oop.call_method([1, 2], "zip", [3, 4], [5, 6]) == [[1, 3, 5], [2, 4, 6]]
    assert oop.call_method([1, 2], "zip", 99) == [[1, None], [2, None]]
    assert oop.call_method([1, 2], "zip") == [[1], [2]]


def test_array_each_slice_each_cons_chunk_while() -> None:
    # `each_slice(n)` splits into consecutive <=n-size chunks (last may be short).
    assert oop.call_method([1, 2, 3, 4, 5], "each_slice", 2) == [[1, 2], [3, 4], [5]]
    assert oop.call_method([1, 2, 3, 4], "each_slice", 2) == [[1, 2], [3, 4]]
    assert oop.call_method([1, 2, 3], "each_slice", 5) == [[1, 2, 3]]
    assert oop.call_method([], "each_slice", 2) == []
    # n <= 0 → [] (Ruby raises ArgumentError; never-raise floor yields empty).
    assert oop.call_method([1, 2], "each_slice", 0) == []
    # `each_cons(n)` yields every consecutive n-element sliding window.
    assert oop.call_method([1, 2, 3, 4], "each_cons", 2) == [[1, 2], [2, 3], [3, 4]]
    assert oop.call_method([1, 2, 3], "each_cons", 3) == [[1, 2, 3]]
    # window larger than the array (or n <= 0) → [].
    assert oop.call_method([1, 2], "each_cons", 3) == []
    assert oop.call_method([1, 2], "each_cons", 0) == []
    # `chunk_while { |a, b| pred }` splits into runs while the adjacent-pair
    # predicate holds; a falsy result starts a new run.
    assert oop.call_method(
        [1, 2, 4, 5, 7], "chunk_while", Closure(lambda a, b: b - a == 1)
    ) == [[1, 2], [4, 5], [7]]
    # all-truthy → one run; all-falsy → singletons.
    assert oop.call_method([1, 2, 3], "chunk_while", Closure(lambda a, b: True)) == [[1, 2, 3]]
    assert oop.call_method([1, 2, 3], "chunk_while", Closure(lambda a, b: False)) == [[1], [2], [3]]
    # empty → []; single element → [[x]].
    assert oop.call_method([], "chunk_while", Closure(lambda a, b: True)) == []
    assert oop.call_method([9], "chunk_while", Closure(lambda a, b: True)) == [[9]]
    # respond_to? advertises the new methods.
    assert oop.call_method([1], "respond_to?", "each_slice") is True
    assert oop.call_method([1], "respond_to?", "each_cons") is True
    assert oop.call_method([1], "respond_to?", "chunk_while") is True


def test_array_tally() -> None:
    # `tally` → a Hash of element → occurrence count, in first-seen key order
    # (a dict preserves insertion order, matching Ruby).  Brings Python level
    # with the Go/Rust runtimes, which already ship `tally`.
    assert oop.call_method(["a", "b", "a", "c", "a"], "tally") == {"a": 3, "b": 1, "c": 1}
    assert oop.call_method([1, 1, 2, 3, 3, 3], "tally") == {1: 2, 2: 1, 3: 3}
    # empty array → empty hash.
    assert oop.call_method([], "tally") == {}
    # respond_to? advertises it.
    assert oop.call_method([1], "respond_to?", "tally") is True


def test_array_slice_when() -> None:
    # `slice_when { |a, b| pred }` is the INVERSE of `chunk_while`: it starts a
    # NEW run BETWEEN an adjacent pair exactly WHERE the block is truthy.
    # `b - a > 1` splits only on an UPWARD gap; (12, 0) has b - a == -12, so 0
    # stays in the preceding run.
    assert oop.call_method(
        [1, 2, 4, 9, 10, 11, 12, 0], "slice_when", Closure(lambda a, b: b - a > 1)
    ) == [[1, 2], [4], [9, 10, 11, 12, 0]]
    # A non-monotonic split predicate (`b < a`) breaks each descent.
    assert oop.call_method(
        [1, 4, 2, 3, 1], "slice_when", Closure(lambda a, b: b < a)
    ) == [[1, 4], [2, 3], [1]]
    # all-truthy → every element its own singleton run; all-falsy → one run.
    assert oop.call_method([1, 2, 3], "slice_when", Closure(lambda a, b: True)) == [[1], [2], [3]]
    assert oop.call_method([1, 2, 3], "slice_when", Closure(lambda a, b: False)) == [[1, 2, 3]]
    # empty → []; single element → [[x]].
    assert oop.call_method([], "slice_when", Closure(lambda a, b: True)) == []
    assert oop.call_method([9], "slice_when", Closure(lambda a, b: True)) == [[9]]
    # respond_to? advertises it.
    assert oop.call_method([1], "respond_to?", "slice_when") is True


def test_array_cycle() -> None:
    # `cycle(n) { |x| … }` iterates the array n full passes in order, yielding
    # each element on every pass; it always returns nil.
    seen: list[int] = []
    assert (
        oop.call_method([1, 2, 3], "cycle", 2, Closure(lambda x: seen.append(x)))
        is None
    )
    assert seen == [1, 2, 3, 1, 2, 3]
    # A single pass is just the array in order.
    once: list[int] = []
    oop.call_method([7, 8], "cycle", 1, Closure(lambda x: once.append(x)))
    assert once == [7, 8]
    # n <= 0 yields nothing (and still returns nil).
    zero: list[int] = []
    assert oop.call_method([1, 2], "cycle", 0, Closure(lambda x: zero.append(x))) is None
    assert zero == []
    neg: list[int] = []
    oop.call_method([1, 2], "cycle", -3, Closure(lambda x: neg.append(x)))
    assert neg == []
    # An empty receiver yields nothing no matter how many passes are requested.
    empty: list[int] = []
    oop.call_method([], "cycle", 5, Closure(lambda x: empty.append(x)))
    assert empty == []
    # respond_to? advertises it; a still-uncatalogued method reports False.
    assert oop.call_method([1], "respond_to?", "cycle") is True
    assert oop.call_method([1], "respond_to?", "combination") is False


def test_array_mutating_push_pop_shift_unshift() -> None:
    a = [1, 2]
    assert oop.call_method(a, "push", 3) == [1, 2, 3]
    assert a == [1, 2, 3]
    assert oop.call_method(a, "<<", 4) == [1, 2, 3, 4]
    assert oop.call_method(a, "pop") == 4
    assert a == [1, 2, 3]
    assert oop.call_method(a, "shift") == 1
    assert a == [2, 3]
    assert oop.call_method(a, "unshift", 0) == [0, 2, 3]


def test_array_reverse_sort_minmax_sum() -> None:
    assert oop.call_method([1, 2, 3], "reverse") == [3, 2, 1]
    assert oop.call_method([3, 1, 2], "sort") == [1, 2, 3]
    assert oop.call_method([3, 1, 2], "min") == 1
    assert oop.call_method([3, 1, 2], "max") == 3
    assert oop.call_method([1, 2, 3], "sum") == 6
    assert oop.call_method([1, 2, 3], "sum", 10) == 16
    assert oop.call_method([], "min") is None
    # `minmax` → [min, max] in one call; empty → [nil, nil].
    assert oop.call_method([3, 1, 2], "minmax") == [1, 3]
    assert oop.call_method(["b", "a", "c"], "minmax") == ["a", "c"]
    assert oop.call_method([], "minmax") == [None, None]
    assert oop.call_method([1], "respond_to?", "minmax") is True


def test_array_reverse_is_nonmutating() -> None:
    a = [1, 2, 3]
    assert oop.call_method(a, "reverse") == [3, 2, 1]
    assert a == [1, 2, 3]


def test_array_uniq_flatten_compact_empty() -> None:
    assert oop.call_method([1, 1, 2, 3, 3], "uniq") == [1, 2, 3]
    assert oop.call_method([1, [2, [3, 4]], 5], "flatten") == [1, 2, 3, 4, 5]
    assert oop.call_method([1, None, 2, None], "compact") == [1, 2]
    assert oop.call_method([], "empty?") is True
    assert oop.call_method([1], "empty?") is False


# ── built-in method catalog: universal Object (M1a) ───────────────────────────


def test_object_nil_eq_and_identity() -> None:
    assert oop.call_method(None, "nil?") is True
    assert oop.call_method(0, "nil?") is False
    assert oop.call_method([1, 2], "==", [1, 2]) is True
    assert oop.call_method([1, 2], "==", [1, 3]) is False
    assert oop.call_method(1, "!=", 2) is True
    x = [1]
    assert oop.call_method(x, "equal?", x) is True
    assert oop.call_method([1], "equal?", [1]) is False


def test_object_dup_clone_itself_freeze() -> None:
    a = [1, 2]
    dup = oop.call_method(a, "dup")
    assert dup == [1, 2]
    assert dup is not a
    assert oop.call_method(5, "itself") == 5
    assert oop.call_method(a, "freeze") is a
    assert oop.call_method(5, "frozen?") is True
    assert oop.call_method([1], "frozen?") is False


def test_to_a_on_nil_and_array() -> None:
    assert oop.call_method(None, "to_a") == []
    a = [1, 2]
    assert oop.call_method(a, "to_a") is a


# ── respond_to? honesty + nil floor (M1a) ─────────────────────────────────────


def test_respond_to_reports_catalog_membership() -> None:
    assert oop.call_method([1], "respond_to?", "reverse") is True
    assert oop.call_method([1], "respond_to?", "nil?") is True
    assert oop.call_method([1], "respond_to?", "is_a?") is True
    assert oop.call_method([1], "respond_to?", "map") is True  # block method (M1b)
    # An out-of-catalog method (`chunk` is the non-`_while` variant, uncatalogued):
    assert oop.call_method([1], "respond_to?", "chunk") is False


def test_known_block_method_without_block_still_returns_nil() -> None:
    # A *known* block method called WITHOUT a block bottoms out at nil (Ruby
    # returns an Enumerator; v0 floor is nil — see spec).  This is a wrong-shape
    # invocation of a catalogued method, NOT a missing method, so it must NOT
    # raise NoMethodError (T1): ``_responds_to`` reports ``map``/``times`` as
    # known, keeping the nil floor.
    assert oop.call_method([1, 2, 3], "map") is None
    assert oop.call_method(5, "times") is None


def test_out_of_catalog_method_raises_no_method_error() -> None:
    # An out-of-catalog String method (scan needs a regex engine — later PR) is a
    # genuinely unknown method → typed NoMethodError (T1).
    with pytest.raises(SirError) as excinfo:
        oop.call_method("hi", "scan")
    assert excinfo.value.sir_class == "NoMethodError"
    assert excinfo.value.args[0] == "undefined method 'scan' for String"


# ── built-in method catalog: block-taking Array/Enumerable (M1b) ──────────────


def test_each_runs_block_and_returns_receiver() -> None:
    seen: list[Val] = []
    a = [1, 2, 3]
    result = oop.call_method(a, "each", Closure(seen.append))
    assert seen == [1, 2, 3]
    assert result is a


def test_each_with_index() -> None:
    pairs: list[Val] = []
    oop.call_method(["a", "b"], "each_with_index", Closure(lambda x, i: pairs.append((x, i))))
    assert pairs == [("a", 0), ("b", 1)]


def test_map_collect_and_select_filter_reject() -> None:
    assert oop.call_method([1, 2, 3], "map", Closure(lambda x: x * 2)) == [2, 4, 6]
    assert oop.call_method([1, 2, 3], "collect", Closure(lambda x: x + 1)) == [2, 3, 4]
    assert oop.call_method([1, 2, 3, 4], "select", Closure(lambda x: x % 2 == 0)) == [2, 4]
    assert oop.call_method([1, 2, 3, 4], "filter", Closure(lambda x: x > 2)) == [3, 4]
    assert oop.call_method([1, 2, 3, 4], "reject", Closure(lambda x: x % 2 == 0)) == [1, 3]


def test_reduce_inject_with_and_without_initial() -> None:
    assert oop.call_method([1, 2, 3, 4], "reduce", Closure(lambda a, b: a + b)) == 10
    assert oop.call_method([1, 2, 3], "inject", 100, Closure(lambda a, b: a + b)) == 106
    assert oop.call_method([], "reduce", Closure(lambda a, b: a + b)) is None


def test_find_detect_and_flat_map() -> None:
    assert oop.call_method([1, 2, 3, 4], "find", Closure(lambda x: x > 2)) == 3
    assert oop.call_method([1, 2], "detect", Closure(lambda x: x > 9)) is None
    assert oop.call_method([1, 2, 3], "flat_map", Closure(lambda x: [x, x * 10])) == [
        1,
        10,
        2,
        20,
        3,
        30,
    ]


def test_array_block_breadth() -> None:
    # sort_by / min_by / max_by — keyed by the block result.
    assert oop.call_method([3, 1, 2], "sort_by", Closure(lambda x: x)) == [1, 2, 3]
    assert oop.call_method(["aaa", "a", "aa"], "sort_by", Closure(len)) == ["a", "aa", "aaa"]
    assert oop.call_method([3, 1, 2], "min_by", Closure(lambda x: x)) == 1
    assert oop.call_method([3, 1, 2], "max_by", Closure(lambda x: x)) == 3
    assert oop.call_method([], "min_by", Closure(lambda x: x)) is None
    # group_by — a Hash of key -> matching elements.
    assert oop.call_method([1, 2, 3, 4], "group_by", Closure(lambda x: x % 2 == 0)) == {
        False: [1, 3],
        True: [2, 4],
    }
    # partition — [matching, non_matching].
    assert oop.call_method([1, 2, 3, 4], "partition", Closure(lambda x: x % 2 == 0)) == [
        [2, 4],
        [1, 3],
    ]
    # collect_concat is an alias of flat_map.
    assert oop.call_method([1, 2], "collect_concat", Closure(lambda x: [x, x])) == [1, 1, 2, 2]
    # take_while / drop_while — leading truthy run and the remainder.
    assert oop.call_method([1, 2, 3, 4], "take_while", Closure(lambda x: x < 3)) == [1, 2]
    assert oop.call_method([1, 2, 3, 4], "drop_while", Closure(lambda x: x < 3)) == [3, 4]
    # count { block } — number of truthy results.
    assert oop.call_method([1, 2, 3, 4], "count", Closure(lambda x: x % 2 == 0)) == 2
    # each_with_object — folds into and returns the memo.
    assert oop.call_method(
        [1, 2, 3], "each_with_object", [], Closure(lambda x, o: o.append(x * 10))
    ) == [10, 20, 30]


def test_any_all_none_use_sir_truthiness() -> None:
    assert oop.call_method([1, 2, 3], "any?", Closure(lambda x: x > 2)) is True
    assert oop.call_method([1, 2, 3], "any?", Closure(lambda x: x > 9)) is False
    assert oop.call_method([2, 4, 6], "all?", Closure(lambda x: x % 2 == 0)) is True
    assert oop.call_method([2, 3], "all?", Closure(lambda x: x % 2 == 0)) is False
    assert oop.call_method([1, 2, 3], "none?", Closure(lambda x: x > 9)) is True
    assert oop.call_method([1, 2, 3], "none?", Closure(lambda x: x > 2)) is False


def test_select_uses_sir_truthiness_not_python() -> None:
    # SIR truthiness: only False/None are falsy, so 0 and "" are KEPT.
    assert oop.call_method([0, 1, None, 2], "select", Closure(lambda x: x)) == [0, 1, 2]


# ── built-in method catalog: Hash (M1c) ───────────────────────────────────────


def test_hash_keys_values_size_empty() -> None:
    h = {"a": 1, "b": 2}
    assert oop.call_method(h, "keys") == ["a", "b"]
    assert oop.call_method(h, "values") == [1, 2]
    assert oop.call_method(h, "size") == 2
    assert oop.call_method(h, "length") == 2
    assert oop.call_method(h, "empty?") is False
    assert oop.call_method({}, "empty?") is True


def test_hash_key_value_membership() -> None:
    h = {"a": 1}
    assert oop.call_method(h, "has_key?", "a") is True
    assert oop.call_method(h, "key?", "z") is False
    assert oop.call_method(h, "include?", "a") is True
    assert oop.call_method(h, "member?", "a") is True
    assert oop.call_method(h, "has_value?", 1) is True
    assert oop.call_method(h, "value?", 9) is False


def test_hash_fetch_dig_to_a() -> None:
    h = {"a": 1, "b": 2}
    assert oop.call_method(h, "fetch", "a") == 1
    # Missing key with an explicit default returns the default (no raise).
    assert oop.call_method(h, "fetch", "z", 99) == 99
    assert oop.call_method(h, "dig", "b") == 2
    assert oop.call_method(h, "to_a") == [["a", 1], ["b", 2]]


def test_hash_fetch_missing_key_raises_key_error() -> None:
    # Ruby ``Hash#fetch`` on a missing key with no default raises KeyError (T1) —
    # unlike ``hash[k]``, which returns nil.
    with pytest.raises(SirError) as excinfo:
        oop.call_method({"a": 1}, "fetch", "z")
    assert excinfo.value.sir_class == "KeyError"
    assert excinfo.value.args[0] == 'key not found: "z"'


def test_hash_store_merge_delete_clear_invert() -> None:
    h = {"a": 1}
    assert oop.call_method(h, "store", "b", 2) == 2
    assert h == {"a": 1, "b": 2}
    assert oop.call_method(h, "[]=", "c", 3) == 3
    assert oop.call_method({"a": 1}, "merge", {"b": 2}) == {"a": 1, "b": 2}
    assert oop.call_method(h, "delete", "a") == 1
    assert "a" not in h
    assert oop.call_method({"a": 1, "b": 2}, "invert") == {1: "a", 2: "b"}
    cleared = {"a": 1}
    assert oop.call_method(cleared, "clear") == {}


def test_hash_block_each_map_select_reject() -> None:
    seen: list[Val] = []
    h = {"a": 1, "b": 2}
    result = oop.call_method(h, "each", Closure(lambda k, v: seen.append((k, v))))
    assert seen == [("a", 1), ("b", 2)]
    assert result is h
    assert oop.call_method(h, "map", Closure(lambda k, v: f"{k}={v}")) == ["a=1", "b=2"]
    assert oop.call_method(h, "select", Closure(lambda k, v: v > 1)) == {"b": 2}
    assert oop.call_method(h, "reject", Closure(lambda k, v: v > 1)) == {"a": 1}


def test_hash_each_key_each_value() -> None:
    ks: list[Val] = []
    vs: list[Val] = []
    h = {"a": 1, "b": 2}
    oop.call_method(h, "each_key", Closure(ks.append))
    oop.call_method(h, "each_value", Closure(vs.append))
    assert ks == ["a", "b"]
    assert vs == [1, 2]


def test_hash_transform_values_and_keys() -> None:
    h = {"a": 1, "b": 2}
    # transform_values: new hash, values mapped, keys unchanged, non-mutating.
    assert oop.call_method(h, "transform_values", Closure(lambda v: v * 10)) == {"a": 10, "b": 20}
    assert h == {"a": 1, "b": 2}
    # transform_keys: new hash, keys mapped, values unchanged.
    assert oop.call_method(h, "transform_keys", Closure(lambda k: k.upper())) == {"A": 1, "B": 2}
    # transform_keys collision: the LAST pair wins.
    assert oop.call_method({"a": 1, "b": 2}, "transform_keys", Closure(lambda _k: "x")) == {"x": 2}


def test_hash_enumerable_aggregates() -> None:
    # Hash mixes in Enumerable: the block is yielded [key, value] and the
    # "element" an aggregate returns is the two-element [key, value] pair.
    h = {"a": 1, "b": 2, "c": 3}
    # find/detect → first matching [k, v] pair (or nil).
    assert oop.call_method(h, "find", Closure(lambda k, v: v > 1)) == ["b", 2]
    assert oop.call_method(h, "detect", Closure(lambda k, v: v > 9)) is None
    # any?/all?/none? over block([k, v]).
    assert oop.call_method(h, "any?", Closure(lambda k, v: v == 2)) is True
    assert oop.call_method(h, "all?", Closure(lambda k, v: v > 0)) is True
    assert oop.call_method(h, "none?", Closure(lambda k, v: v > 9)) is True
    # count { |k, v| pred } → number of truthy pairs.
    assert oop.call_method(h, "count", Closure(lambda k, v: v % 2 == 1)) == 2
    # sort_by → NEW array of [k, v] pairs ordered by the block key.
    assert oop.call_method(h, "sort_by", Closure(lambda k, v: -v)) == [
        ["c", 3],
        ["b", 2],
        ["a", 1],
    ]
    # min_by / max_by → the extremal [k, v] pair.
    assert oop.call_method(h, "min_by", Closure(lambda k, v: v)) == ["a", 1]
    assert oop.call_method(h, "max_by", Closure(lambda k, v: v)) == ["c", 3]
    # min_by / max_by on an empty hash → nil.
    assert oop.call_method({}, "min_by", Closure(lambda k, v: v)) is None
    # respond_to? advertises the new aggregates.
    assert oop.call_method(h, "respond_to?", "min_by") is True
    assert oop.call_method(h, "respond_to?", "sort_by") is True


def test_hash_enumerable_grouping_folding() -> None:
    # Enumerable breadth part 2: group_by/partition/flat_map/reduce/sum. The
    # block is yielded [key, value] and results carry [key, value] pairs;
    # reduce follows Ruby's memo convention (memo, pair).
    h = {"a": 1, "b": 2, "c": 3, "d": 4}
    # group_by { |k, v| v.even? } → {False: [[a,1],[c,3]], True: [[b,2],[d,4]]}
    assert oop.call_method(h, "group_by", Closure(lambda k, v: v % 2 == 0)) == {
        False: [["a", 1], ["c", 3]],
        True: [["b", 2], ["d", 4]],
    }
    # partition { |k, v| v > 2 } → [[[c,3],[d,4]], [[a,1],[b,2]]]
    assert oop.call_method(h, "partition", Closure(lambda k, v: v > 2)) == [
        [["c", 3], ["d", 4]],
        [["a", 1], ["b", 2]],
    ]
    # flat_map { |k, v| [k, v] } → [a, 1, b, 2, c, 3, d, 4]  (one-level flatten)
    assert oop.call_method(h, "flat_map", Closure(lambda k, v: [k, v])) == [
        "a", 1, "b", 2, "c", 3, "d", 4,
    ]
    # collect_concat alias behaves identically.
    assert oop.call_method({"a": 1}, "collect_concat", Closure(lambda k, v: [v])) == [1]
    # reduce(0) { |sum, (k, v)| sum + v } → 10
    assert oop.call_method(h, "reduce", 0, Closure(lambda acc, pair: acc + pair[1])) == 10
    # inject (seedless) starts the memo from the first [k, v] pair, then folds
    # later pairs; here it appends each later value → [a, 1, 2].
    assert (
        oop.call_method({"a": 1, "b": 2}, "inject", Closure(lambda acc, pair: acc + [pair[1]]))
        == ["a", 1, 2]
    )
    # empty seedless reduce → nil
    assert oop.call_method({}, "reduce", Closure(lambda acc, pair: acc)) is None
    # sum(0) { |k, v| v } → 10 ; sum(100) { |k, v| v } → 110
    assert oop.call_method(h, "sum", 0, Closure(lambda k, v: v)) == 10
    assert oop.call_method(h, "sum", 100, Closure(lambda k, v: v)) == 110
    # respond_to? advertises the new methods; receiver unchanged throughout.
    assert oop.call_method(h, "respond_to?", "group_by") is True
    assert oop.call_method(h, "respond_to?", "reduce") is True
    assert list(h.items()) == [("a", 1), ("b", 2), ("c", 3), ("d", 4)]


def test_hash_to_h_and_indexed_iteration() -> None:
    # `to_h` (no block) returns a shallow copy; mutating it leaves the source
    # untouched.  `to_h { |k, v| [nk, nv] }` re-maps each pair.  Indexed/object
    # iteration (`each_with_index`, `each_with_object`) yields the [k, v] pair
    # as ONE argument alongside the index/memo (the second block param), matching
    # Ruby's Enumerable convention (contrast `each`'s two-arg (k, v) yield).
    h = {"a": 1, "b": 2, "c": 3}
    # to_h, no block → a fresh equal dict that does not alias the receiver.
    copy = oop.call_method(h, "to_h")
    assert copy == {"a": 1, "b": 2, "c": 3}
    copy["z"] = 99
    assert "z" not in h
    # to_h with a block re-maps each [k, v] → [new_k, new_v]; here upcase the key
    # and double the value.
    assert oop.call_method(h, "to_h", Closure(lambda k, v: [k.upper(), v * 2])) == {
        "A": 2,
        "B": 4,
        "C": 6,
    }
    # to_h block whose new keys collide → the LAST pair wins (Ruby's rule).
    assert oop.call_method(h, "to_h", Closure(lambda k, v: ["k", v])) == {"k": 3}
    # each_with_index yields ([k, v], i) and returns the receiver.
    seen: list[Val] = []
    result = oop.call_method(
        h, "each_with_index", Closure(lambda pair, i: seen.append([pair, i]))
    )
    assert seen == [[["a", 1], 0], [["b", 2], 1], [["c", 3], 2]]
    assert result is h
    # each_with_object(memo) yields ([k, v], memo) and returns the memo; here we
    # accumulate the values into a running total wrapped in a list.
    total = oop.call_method(
        h,
        "each_with_object",
        [0],
        Closure(lambda pair, memo: memo.__setitem__(0, memo[0] + pair[1])),
    )
    assert total == [6]
    # each_with_object with NO memo argument returns the receiver unchanged.
    assert oop.call_method(h, "each_with_object", Closure(lambda pair, memo: None)) is h
    # respond_to? advertises the new methods; source hash unchanged throughout.
    assert oop.call_method(h, "respond_to?", "to_h") is True
    assert oop.call_method(h, "respond_to?", "each_with_index") is True
    assert oop.call_method(h, "respond_to?", "each_with_object") is True
    assert h == {"a": 1, "b": 2, "c": 3}


def test_hash_respond_to_and_no_method_error_floor() -> None:
    assert oop.call_method({"a": 1}, "respond_to?", "keys") is True
    assert oop.call_method({"a": 1}, "respond_to?", "each") is True
    # `transform_keys!` (the in-place bang variant) is still out of catalog.
    assert oop.call_method({"a": 1}, "respond_to?", "transform_keys!") is False
    # An out-of-catalog Hash method is genuinely unknown → NoMethodError (T1).
    with pytest.raises(SirError) as excinfo:
        oop.call_method({"a": 1}, "transform_keys!")
    assert excinfo.value.sir_class == "NoMethodError"
    assert excinfo.value.args[0] == "undefined method 'transform_keys!' for Hash"
    # Universal Object methods still resolve on a Hash receiver.
    assert oop.call_method({"a": 1}, "nil?") is False


# ── built-in method catalog: String (M1c) ─────────────────────────────────────


def test_string_length_case_reverse() -> None:
    assert oop.call_method("hello", "length") == 5
    assert oop.call_method("hello", "size") == 5
    assert oop.call_method("hello", "upcase") == "HELLO"
    assert oop.call_method("HELLO", "downcase") == "hello"
    assert oop.call_method("hello world", "capitalize") == "Hello world"
    assert oop.call_method("abc", "reverse") == "cba"


def test_string_justify_and_swapcase() -> None:
    # ljust/rjust/center pad to `width` chars with a cyclic pad; center's odd
    # extra pad goes on the RIGHT (Ruby's rule); width <= len is a no-op.
    assert oop.call_method("hi", "ljust", 5) == "hi   "
    assert oop.call_method("hi", "ljust", 5, "*") == "hi***"
    assert oop.call_method("hi", "rjust", 5, "*") == "***hi"
    assert oop.call_method("hi", "center", 6, "*") == "**hi**"
    assert oop.call_method("hi", "center", 5, "*") == "*hi**"
    assert oop.call_method("abc", "ljust", 1) == "abc"
    assert oop.call_method("abcdef", "ljust", 10, "xy") == "abcdefxyxy"
    # swapcase flips each ASCII letter, leaving other characters untouched.
    assert oop.call_method("Hello World", "swapcase") == "hELLO wORLD"
    assert oop.call_method("a1B!c", "swapcase") == "A1b!C"


def test_string_strip_family_and_chomp() -> None:
    assert oop.call_method("  hi  ", "strip") == "hi"
    assert oop.call_method("  hi  ", "lstrip") == "hi  "
    assert oop.call_method("  hi  ", "rstrip") == "  hi"
    assert oop.call_method("line\n", "chomp") == "line"
    assert oop.call_method("line\r\n", "chomp") == "line"
    assert oop.call_method("hello", "chomp", "lo") == "hel"
    assert oop.call_method("hello", "chomp") == "hello"


def test_string_chars_bytes_split() -> None:
    assert oop.call_method("abc", "chars") == ["a", "b", "c"]
    assert oop.call_method("AB", "bytes") == [65, 66]
    assert oop.call_method("a,b,c", "split", ",") == ["a", "b", "c"]
    assert oop.call_method("a  b\tc", "split") == ["a", "b", "c"]


def test_string_predicates_and_index() -> None:
    assert oop.call_method("hello", "include?", "ell") is True
    assert oop.call_method("hello", "start_with?", "he") is True
    assert oop.call_method("hello", "end_with?", "lo") is True
    assert oop.call_method("hello", "index", "l") == 2
    assert oop.call_method("hello", "index", "z") is None
    assert oop.call_method("", "empty?") is True
    assert oop.call_method("x", "empty?") is False


def test_string_replace_sub_gsub_are_literal() -> None:
    assert oop.call_method("old", "replace", "new") == "new"
    assert oop.call_method("a.a.a", "sub", "a", "X") == "X.a.a"
    assert oop.call_method("a.a.a", "gsub", "a", "X") == "X.X.X"
    # Literal — a replacement containing regex/backref syntax is inserted verbatim.
    assert oop.call_method("ab", "gsub", "a", "$&") == "$&b"


def test_string_to_i_to_f_to_sym() -> None:
    assert oop.call_method("42abc", "to_i") == 42
    assert oop.call_method("  -7", "to_i") == -7
    assert oop.call_method("nope", "to_i") == 0
    assert oop.call_method("3.14xyz", "to_f") == 3.14
    assert oop.call_method("nope", "to_f") == 0.0
    sym = oop.call_method("name", "to_sym")
    assert getattr(sym, "name", None) == "name"


def test_string_repeat_and_concat() -> None:
    assert oop.call_method("ab", "*", 3) == "ababab"
    assert oop.call_method("foo", "+", "bar") == "foobar"
    # Non-positive counts yield "" (never raise); a hostile count is capped.
    assert oop.call_method("ab", "*", 0) == ""
    assert oop.call_method("ab", "*", -5) == ""
    assert len(oop.call_method("ab", "*", 10**9)) <= 100_000_000


def test_string_tr_translates_and_deletes() -> None:
    # `tr(from, to)`: position-wise char map; a shorter `to` repeats its last
    # char; an empty `to` deletes matching chars; last mapping wins on a repeat.
    assert oop.call_method("hello", "tr", "el", "ip") == "hippo"
    assert oop.call_method("hello", "tr", "aeiou", "*") == "h*ll*"
    assert oop.call_method("hello", "tr", "l", "") == "heo"
    assert oop.call_method("hello", "tr", "xyz", "abc") == "hello"


def test_string_count_delete_squeeze_char_sets() -> None:
    assert oop.call_method("hello", "count", "l") == 2
    assert oop.call_method("hello", "count", "lo") == 3
    assert oop.call_method("hello", "count", "xyz") == 0
    assert oop.call_method("hello", "delete", "l") == "heo"
    assert oop.call_method("hello", "delete", "aeiou") == "hll"
    # squeeze: no arg collapses every run; with a set, only those chars
    assert oop.call_method("mississippi", "squeeze") == "misisipi"
    assert oop.call_method("aaabbbccc", "squeeze", "a") == "abbbccc"


def test_string_each_char_block() -> None:
    seen: list[Val] = []
    result = oop.call_method("abc", "each_char", Closure(seen.append))
    assert seen == ["a", "b", "c"]
    assert result == "abc"


def test_string_respond_to_and_no_method_error_floor() -> None:
    assert oop.call_method("x", "respond_to?", "upcase") is True
    assert oop.call_method("x", "respond_to?", "each_char") is True
    assert oop.call_method("x", "respond_to?", "scan") is False
    # An out-of-catalog String method is genuinely unknown → NoMethodError (T1).
    with pytest.raises(SirError) as excinfo:
        oop.call_method("x", "scan")
    assert excinfo.value.sir_class == "NoMethodError"
    # Universal Object methods still resolve on a String receiver.
    assert oop.call_method("x", "nil?") is False
    assert oop.call_method("x", "class") == "String"


# ── built-in method catalog: Numeric (Integer/Float) (M1c) ────────────────────


def test_numeric_predicates_and_sign() -> None:
    assert oop.call_method(4, "even?") is True
    assert oop.call_method(3, "odd?") is True
    assert oop.call_method(0, "zero?") is True
    assert oop.call_method(5, "positive?") is True
    assert oop.call_method(-5, "negative?") is True
    assert oop.call_method(-7, "abs") == 7
    assert oop.call_method(-3.5, "abs") == 3.5


def test_numeric_conversions_and_steps() -> None:
    assert oop.call_method(3.9, "to_i") == 3
    assert oop.call_method(4, "to_f") == 4.0
    assert oop.call_method(5, "succ") == 6
    assert oop.call_method(5, "next") == 6
    assert oop.call_method(5, "pred") == 4


def test_numeric_floor_ceil_round() -> None:
    assert oop.call_method(3.2, "floor") == 3
    assert oop.call_method(3.2, "ceil") == 4
    assert oop.call_method(7, "floor") == 7
    # Ruby rounds half away from zero (unlike Python banker's rounding).
    assert oop.call_method(2.5, "round") == 3
    assert oop.call_method(-2.5, "round") == -3
    assert oop.call_method(5, "round") == 5


def test_numeric_gcd_pow_digits() -> None:
    assert oop.call_method(12, "gcd", 18) == 6
    assert oop.call_method(2, "**", 10) == 1024
    assert oop.call_method(2, "pow", 5) == 32
    assert oop.call_method(123, "digits") == [3, 2, 1]
    assert oop.call_method(0, "digits") == [0]


def test_numeric_round_ndigits() -> None:
    # A positive ndigits rounds a Float to that many decimals, half away from zero.
    assert oop.call_method(3.14159, "round", 2) == 3.14
    assert oop.call_method(2.675, "round", 2) == 2.68
    assert oop.call_method(-2.5, "round", 0) == -3
    # ndigits <= 0 rounds to an Integer power of ten.
    assert oop.call_method(1234, "round", -2) == 1200
    assert oop.call_method(1250, "round", -2) == 1300  # half away from zero
    assert oop.call_method(1234.5, "round", -1) == 1230
    # A rounding place that dwarfs the value is 0 (Ruby parity).
    assert oop.call_method(1234, "round", -10) == 0
    assert oop.call_method(1234.5, "round", -10) == 0
    # DoS guard: a hostile magnitude must short-circuit, not build a bignum.
    assert oop.call_method(1234, "round", -1_000_000_000) == 0
    assert oop.call_method(1234.5, "round", -1_000_000_000) == 0
    # Positive ndigits past Float precision returns the value unchanged (no
    # 10.0 ** ndigits OverflowError).
    assert oop.call_method(3.14, "round", 1_000_000) == 3.14
    # A non-finite ndigits argument degrades to 0 rather than raising an untyped
    # int(inf)/int(nan) error.
    assert oop.call_method(5, "round", float("inf")) == 5
    assert oop.call_method(5, "round", float("nan")) == 5
    # A huge integer receiver must round via integer arithmetic (no float
    # OverflowError from `recv / factor`).
    assert oop.call_method(10**309, "round", -1) == 10**309
    assert oop.call_method(5 * 10**400, "round", -3) == 5 * 10**400
    # A near-max Float with a positive ndigits must not overflow the scale-up
    # (recv * 10**ndigits → inf → _ruby_round(inf) OverflowError); return as-is.
    assert oop.call_method(1.7e308, "round", 5) == 1.7e308
    assert oop.call_method(1e300, "round", 17) == 1e300


def test_numeric_divmod_fdiv() -> None:
    assert oop.call_method(13, "divmod", 4) == [3, 1]
    # Remainder takes the divisor's sign (Ruby/Python floored division).
    assert oop.call_method(13, "divmod", -4) == [-4, -3]
    assert oop.call_method(7, "fdiv", 2) == 3.5
    # fdiv never raises: dividing by zero yields Infinity / NaN.
    assert oop.call_method(1, "fdiv", 0) == float("inf")
    assert oop.call_method(-1, "fdiv", 0) == float("-inf")
    import math as _math

    assert _math.isnan(oop.call_method(0, "fdiv", 0))
    # divmod by zero raises a typed ZeroDivisionError.
    with pytest.raises(SirError):
        oop.call_method(1, "divmod", 0)
    # A non-numeric divisor degrades rather than raising an untyped error:
    # divmod → typed ZeroDivisionError, fdiv → Infinity (0 divisor).
    with pytest.raises(SirError):
        oop.call_method(1, "divmod", "x")
    assert oop.call_method(1, "fdiv", "x") == float("inf")
    # A bignum receiver/arg must saturate to ±inf, not raise an untyped
    # OverflowError from float(bignum).
    big = 2**5000
    assert oop.call_method(big, "fdiv", 2) == float("inf")
    assert oop.call_method(3, "fdiv", big) == 0.0
    # Mixed int-receiver / float-divisor divmod on a bignum must not raise an
    # untyped OverflowError; the receiver saturates to inf, so divmod degrades
    # to NaN (never-raise floor) rather than crashing.
    q, r = oop.call_method(big, "divmod", 0.5)
    assert _math.isnan(q) and _math.isnan(r)


def test_numeric_clamp_between() -> None:
    assert oop.call_method(5, "clamp", 1, 10) == 5
    assert oop.call_method(-3, "clamp", 1, 10) == 1
    assert oop.call_method(99, "clamp", 1, 10) == 10
    assert oop.call_method(5, "between?", 1, 10) is True
    assert oop.call_method(0, "between?", 1, 10) is False
    assert oop.call_method(10, "between?", 1, 10) is True


def test_numeric_to_s() -> None:
    assert oop.call_method(42, "to_s") == "42"
    assert oop.call_method(3.14, "to_s") == "3.14"
    assert oop.call_method(7, "inspect") == "7"


def test_numeric_block_times_upto_downto_step() -> None:
    seen: list[Val] = []
    assert oop.call_method(3, "times", Closure(seen.append)) == 3
    assert seen == [0, 1, 2]
    up: list[Val] = []
    oop.call_method(1, "upto", 4, Closure(up.append))
    assert up == [1, 2, 3, 4]
    down: list[Val] = []
    oop.call_method(3, "downto", 1, Closure(down.append))
    assert down == [3, 2, 1]
    step: list[Val] = []
    oop.call_method(0, "step", 10, 5, Closure(step.append))
    assert step == [0, 5, 10]


def test_numeric_respond_to_and_floor() -> None:
    assert oop.call_method(5, "respond_to?", "even?") is True
    assert oop.call_method(5, "respond_to?", "times") is True
    assert oop.call_method(5, "respond_to?", "bit_length") is False
    # An out-of-catalog numeric method is genuinely unknown → NoMethodError (T1).
    with pytest.raises(SirError) as excinfo:
        oop.call_method(5, "bit_length")
    assert excinfo.value.sir_class == "NoMethodError"
    assert excinfo.value.args[0] == "undefined method 'bit_length' for Integer"
    # A *known* block method called without a block bottoms out at the nil floor.
    assert oop.call_method(5, "times") is None


# ── built-in method catalog: Symbol (M1c) ─────────────────────────────────────


def test_symbol_methods() -> None:
    sym = oop.call_method("hello", "to_sym")
    assert oop.call_method(sym, "to_s") == "hello"
    assert oop.call_method(sym, "length") == 5
    assert oop.call_method(sym, "size") == 5
    assert oop.call_method(sym, "inspect") == ":hello"
    assert oop.call_method(sym, "empty?") is False
    up = oop.call_method(sym, "upcase")
    assert getattr(up, "name", None) == "HELLO"
    down = oop.call_method(oop.call_method("ABC", "to_sym"), "downcase")
    assert getattr(down, "name", None) == "abc"
    assert oop.call_method(sym, "to_sym") is sym


# ── built-in method catalog: nil / true / false + to_s/inspect (M1c) ──────────


def test_nil_true_false_to_s_inspect() -> None:
    assert oop.call_method(None, "to_s") == ""
    assert oop.call_method(None, "inspect") == "nil"
    assert oop.call_method(None, "to_a") == []
    assert oop.call_method(True, "to_s") == "true"
    assert oop.call_method(False, "to_s") == "false"
    assert oop.call_method(True, "inspect") == "true"
    # bool resolves only Object methods, never the numeric catalog — so a
    # numeric method on a boolean is unknown → NoMethodError (T1).
    assert oop.call_method(True, "respond_to?", "even?") is False
    with pytest.raises(SirError) as excinfo:
        oop.call_method(True, "even?")
    assert excinfo.value.sir_class == "NoMethodError"
    assert excinfo.value.args[0] == "undefined method 'even?' for TrueClass"


def test_object_to_s_inspect_collections() -> None:
    assert oop.call_method([1, 2, 3], "to_s") == "[1, 2, 3]"
    assert oop.call_method(["a", "b"], "inspect") == '["a", "b"]'
    assert oop.call_method("hi", "inspect") == '"hi"'
    assert oop.call_method("hi", "to_s") == "hi"


def test_array_join() -> None:
    assert oop.call_method([1, 2, 3], "join") == "123"
    assert oop.call_method([1, 2, 3], "join", "-") == "1-2-3"
    assert oop.call_method(["a", "b"], "join", ", ") == "a, b"


def test_pow_and_digits_bound_hostile_bignums() -> None:
    # A hostile exponent would allocate gigabytes; pow refuses (0) rather than
    # hanging, and digits never builds a giant list — neither raises.
    assert oop.call_method(2, "**", 10**9) == 0
    assert oop.call_method(2, "pow", 64) == 2**64  # legitimate values still work
    over_budget = 1 << (1 << 20)  # a >1M-bit integer, past the digits budget
    assert oop.call_method(over_budget, "digits") == [0]  # refused
    assert oop.call_method(123, "digits") == [3, 2, 1]


def test_numeric_methods_never_raise_on_non_finite() -> None:
    inf = float("inf")
    # int-coercing methods degrade gracefully on inf/nan instead of raising.
    assert oop.call_method(inf, "to_i") == 0
    assert oop.call_method(inf, "even?") is False
    assert oop.call_method(inf, "gcd", 6) == 0
    assert oop.call_method(inf, "floor") == inf
    assert oop.call_method(inf, "round") == inf
    assert oop.call_method(inf, "digits") == [0]


def test_inspect_handles_cycles_without_raising() -> None:
    a: list[Val] = []
    a.append(a)  # self-referential array
    assert oop.call_method(a, "inspect") == "[[...]]"
    h: dict[Val, Val] = {}
    h["self"] = h
    assert oop.call_method(h, "inspect") == '{"self"=>{...}}'


# ── Symbol#to_proc (&:sym) — M2 ───────────────────────────────────────────────


def test_sym_to_proc_maps_over_array_via_apply() -> None:
    from coding_adventures_sir_runtime_core import apply, intern

    proc = oop.sym_to_proc(intern("to_s"))
    assert isinstance(proc, Closure)
    # [1, 2, 3].map(&:to_s) — map drives the proc through apply with one arg.
    assert [apply(proc, [x]) for x in [1, 2, 3]] == ["1", "2", "3"]


def test_sym_to_proc_forwards_extra_args_to_method() -> None:
    from coding_adventures_sir_runtime_core import apply, intern

    # A two-arg apply binds the first as receiver and forwards the rest as
    # method arguments: ["hello", "ell"] → "hello".include?("ell").  (This is
    # the arity shape `inject`/`each_with_index` blocks rely on; arithmetic
    # operators like `&:+` are native, not in the dispatch catalog — out of
    # scope for M2.)
    proc = oop.sym_to_proc(intern("include?"))
    assert apply(proc, ["hello", "ell"]) is True
    assert apply(proc, ["hello", "xyz"]) is False


def test_sym_to_proc_accepts_bare_string_name() -> None:
    from coding_adventures_sir_runtime_core import apply

    proc = oop.sym_to_proc("upcase")
    assert apply(proc, ["hi"]) == "HI"


def test_sym_to_proc_out_of_catalog_method_raises_no_method_error() -> None:
    from coding_adventures_sir_runtime_core import apply, intern

    # ``(&:no_such_method)`` applied to a value dispatches an unknown method, so
    # it raises NoMethodError (T1) just like a direct ``x.no_such_method`` —
    # matching Ruby, where ``[42].map(&:no_such_method)`` raises NoMethodError.
    proc = oop.sym_to_proc(intern("no_such_method"))
    with pytest.raises(SirError) as excinfo:
        apply(proc, [42])
    assert excinfo.value.sir_class == "NoMethodError"


def test_sym_to_proc_drives_array_block_method_dispatch() -> None:
    from coding_adventures_sir_runtime_core import intern

    # End-to-end through call_method: [1, 2, 3].map(&:to_s).
    proc = oop.sym_to_proc(intern("to_s"))
    assert oop.call_method([1, 2, 3], "map", proc) == ["1", "2", "3"]


# ── case-equality (M5) ─────────────────────────────────────────────────────────


def test_case_eq_regex_matches_string() -> None:
    import re

    pat = re.compile("ell")
    assert oop.case_eq(pat, "hello") is True
    assert oop.case_eq(pat, "world") is False


def test_case_eq_regex_non_string_never_matches() -> None:
    import re

    # A non-String scrutinee never matches a regex (Ruby returns false).
    assert oop.case_eq(re.compile("1"), 1) is False


def test_case_eq_range_membership() -> None:
    # A Range is detected structurally (class name + `includes`), so a stand-in
    # named `Range` exercises the path without importing sir-runtime-range.
    class Range:
        def __init__(self, lo: int, hi: int) -> None:
            self.lo, self.hi = lo, hi

        def includes(self, value: Val) -> bool:
            return self.lo <= value <= self.hi

    r = Range(1, 5)
    assert oop.case_eq(r, 3) is True
    assert oop.case_eq(r, 9) is False
    # Ruby `(1..5) === "x"` is false, not an error — the int/str comparison
    # raises TypeError inside `includes`, which case_eq swallows.
    assert oop.case_eq(r, "x") is False


def test_case_eq_falls_back_to_equality() -> None:
    # A plain literal pattern uses value equality (the `==` floor).
    assert oop.case_eq(5, 5) is True
    assert oop.case_eq(5, 6) is False
    assert oop.case_eq("a", "a") is True


# ── Kernel flow-control + boolean operators (M6) ──────────────────────────────


def test_send_routes_to_named_method() -> None:
    # `x.send(:meth, *args)` is exactly `x.meth(*args)`; the method name may
    # arrive as a Symbol (the emitted form) or a bare string.
    from coding_adventures_sir_runtime_core import intern

    assert oop.call_method("hello", "send", "upcase") == "HELLO"
    assert oop.call_method([3, 1, 2], "send", "sort") == [1, 2, 3]
    # A Symbol method name (interned) routes identically.
    assert oop.call_method("hi", "send", intern("upcase")) == "HI"
    # `__send__` is the alias used when `send` itself is shadowed in real Ruby.
    assert oop.call_method("hi", "__send__", "reverse") == "ih"
    # Sanity: plain dispatch of a non-send method is unaffected.
    assert oop.call_method("__send__", "size") == 8


def test_send_forwards_arguments_and_block() -> None:
    # Extra arguments forward through send.
    assert oop.call_method("a,b,c", "send", "split", ",") == ["a", "b", "c"]
    # A trailing block survives send and reaches the block-taking method.
    seen: list[Val] = []
    oop.call_method([1, 2], "send", "each", Closure(seen.append))
    assert seen == [1, 2]


def test_user_defined_send_override_wins() -> None:
    # The user define_method table is consulted first (resolution order #2), so a
    # user-defined `send` override takes precedence over the built-in routing.
    oop.define_method("send", lambda recv, args: "overridden")
    assert oop.call_method("x", "send", "upcase") == "overridden"


def test_send_without_method_name_is_nil() -> None:
    # `send` with no method name bottoms out at the nil floor (never raises).
    assert oop.call_method("x", "send") is None


def test_tap_yields_receiver_and_returns_it() -> None:
    captured: list[Val] = []
    result = oop.call_method([1, 2, 3], "tap", Closure(captured.append))
    assert captured == [[1, 2, 3]]
    assert result == [1, 2, 3]
    # Block-less tap returns the receiver (v0 floor).
    assert oop.call_method(42, "tap") == 42


def test_then_returns_block_result() -> None:
    # then/yield_self replace the value with the block's result.
    assert oop.call_method(5, "then", Closure(lambda x: x * 2)) == 10
    assert oop.call_method("hi", "yield_self", Closure(lambda s: s + "!")) == "hi!"
    # Block-less then returns the receiver.
    assert oop.call_method(7, "then") == 7


def test_bool_logical_operators() -> None:
    # Eager (non-short-circuit) logical operators on booleans.
    assert oop.call_method(True, "&", True) is True
    assert oop.call_method(True, "&", False) is False
    assert oop.call_method(False, "|", True) is True
    assert oop.call_method(True, "^", True) is False
    assert oop.call_method(True, "^", False) is True
    # Ruby truthiness on the argument: nil is falsy, 0/"" are truthy.
    assert oop.call_method(True, "&", None) is False
    assert oop.call_method(False, "|", 0) is True
    assert oop.call_method(False, "|", "") is True


def test_kernel_respond_to_is_honest() -> None:
    # tap/then/send resolve on every receiver; bool operators on bools only.
    assert oop.call_method(1, "respond_to?", "tap") is True
    assert oop.call_method("x", "respond_to?", "then") is True
    assert oop.call_method([], "respond_to?", "send") is True
    assert oop.call_method(True, "respond_to?", "&") is True
    # A non-bool receiver does not respond to the boolean operators.
    assert oop.call_method(5, "respond_to?", "^") is False
    # An out-of-catalog name is both NoMethodError (T1) and respond_to? == False.
    with pytest.raises(SirError) as excinfo:
        oop.call_method(True, "nonexistent_method")
    assert excinfo.value.sir_class == "NoMethodError"
    assert oop.call_method(True, "respond_to?", "nonexistent_method") is False


# ── O1: user method tables, call_new / call_super / self / class methods ──────


def test_call_new_runs_initialize_and_binds_self() -> None:
    # Dog.new("Rex") — initialize sets @name on the freshly allocated object.
    oop.define_class("Dog", None)

    def _init(name: Val) -> Val:
        return oop.ivar_set("@name", name)

    oop.def_method("Dog", "initialize", Closure(_init))
    dog = oop.call_new("Dog", "Rex")
    assert isinstance(dog, oop.SirInstance)
    assert dog.sir_class == "Dog"
    assert dog.ivars["@name"] == "Rex"
    # The self-stack is balanced after construction (initialize popped its self).
    assert oop.current_self() is None


def test_call_new_without_initialize_is_plain_allocation() -> None:
    oop.define_class("Empty", None)
    obj = oop.call_new("Empty")
    assert isinstance(obj, oop.SirInstance)
    assert obj.ivars == {}


def test_call_new_inherits_initialize_from_ancestor() -> None:
    oop.define_class("Base", None)
    oop.define_class("Derived", "Base")
    oop.def_method("Base", "initialize", Closure(lambda v: oop.ivar_set("@v", v)))
    obj = oop.call_new("Derived", 7)
    assert obj.sir_class == "Derived"
    assert obj.ivars["@v"] == 7


def test_call_method_dispatches_user_instance_method() -> None:
    # Dog#speak reads @name via the current self pushed by call_method.
    oop.define_class("Dog", None)
    oop.def_method("Dog", "initialize", Closure(lambda n: oop.ivar_set("@name", n)))
    oop.def_method(
        "Dog", "speak", Closure(lambda: oop.ivar_get("@name") + " says woof")
    )
    dog = oop.call_new("Dog", "Rex")
    assert oop.call_method(dog, "speak") == "Rex says woof"
    # Self is balanced after dispatch.
    assert oop.current_self() is None


def test_call_method_walks_ancestry_for_user_method() -> None:
    oop.define_class("Animal", None)
    oop.define_class("Cat", "Animal")
    oop.def_method("Animal", "legs", Closure(lambda: 4))
    cat = oop.call_new("Cat")
    assert oop.call_method(cat, "legs") == 4


def test_call_method_falls_through_to_builtins_for_instances() -> None:
    # A SirInstance with no user `class`/`is_a?` still resolves the reflective
    # built-ins (no regression of the universal Object surface).
    oop.define_class("Widget", None)
    w = oop.call_new("Widget")
    assert oop.call_method(w, "class") == "Widget"
    assert oop.call_method(w, "is_a?", "Widget") is True
    assert oop.call_method(w, "nil?") is False


def test_call_super_walks_to_parent_implementation() -> None:
    # Cat#describe calls super (Animal#describe) with the current self bound.
    oop.define_class("Animal", None)
    oop.define_class("Cat", "Animal")
    oop.def_method(
        "Animal", "describe", Closure(lambda: oop.ivar_get("@name") + " with 4 legs")
    )

    def _cat_describe() -> Val:
        # super — same receiver, no new self.
        return oop.call_super("describe", "Cat")

    oop.def_method("Cat", "initialize", Closure(lambda n: oop.ivar_set("@name", n)))
    oop.def_method("Cat", "describe", Closure(_cat_describe))
    cat = oop.call_new("Cat", "Tom")
    assert oop.call_method(cat, "describe") == "Tom with 4 legs"


def test_call_super_returns_nil_when_no_ancestor_method() -> None:
    oop.define_class("Lonely", None)
    # No superclass at all → nil floor.
    assert oop.call_super("whatever", "Lonely") is None
    # Superclass exists but does not define the method → nil floor.
    oop.define_class("Base", None)
    oop.define_class("Sub", "Base")
    assert oop.call_super("missing", "Sub") is None


def test_call_class_method_dispatch_and_ancestry() -> None:
    # Counter.zero — a `def self.zero` class method.
    oop.define_class("Counter", None)
    oop.def_class_method("Counter", "zero", Closure(lambda: 0))
    assert oop.call_class_method("Counter", "zero") == 0
    # Inherited class method resolves through the ancestry walk.
    oop.define_class("Sub", "Counter")
    assert oop.call_class_method("Sub", "zero") == 0
    # Unknown class method → nil floor.
    assert oop.call_class_method("Counter", "nope") is None


def test_current_self_reflects_stack_top() -> None:
    assert oop.current_self() is None
    obj = oop.new_instance("Thing")
    oop.push_self(obj)
    assert oop.current_self() is obj
    oop.pop_self()
    assert oop.current_self() is None


def test_self_return_chaining() -> None:
    # Counter#inc returns self (current_self) for method chaining.
    oop.define_class("Counter", None)
    oop.def_method("Counter", "initialize", Closure(lambda: oop.ivar_set("@n", 0)))

    def _inc() -> Val:
        oop.ivar_set("@n", oop.ivar_get("@n") + 1)
        return oop.current_self()

    oop.def_method("Counter", "inc", Closure(_inc))
    oop.def_method("Counter", "count", Closure(lambda: oop.ivar_get("@n")))
    c = oop.call_new("Counter")
    chained = oop.call_method(oop.call_method(c, "inc"), "inc")
    assert chained is c
    assert oop.call_method(c, "count") == 2


def test_reset_oop_clears_method_tables() -> None:
    oop.define_class("Dog", None)
    oop.def_method("Dog", "speak", Closure(lambda: "woof"))
    oop.def_class_method("Dog", "make", Closure(lambda: "made"))
    oop.reset_oop()
    # After reset the tables are empty → nil floor.
    assert oop.call_class_method("Dog", "make") is None
    assert oop.call_super("speak", "Dog") is None


# ── Typed runtime errors: end-to-end rescue-matchability (T1) ─────────────────
#
# These prove the *emitted rescue shape* works: a Ruby ``begin; <op>; rescue
# <Class> => e; end`` lowers to a native ``try/except`` that catches broadly and
# then calls ``rescue_matches(exc, [<Class>])`` per clause.  A faulting op must
# raise a typed SirError so the right clause fires (and unrelated clauses miss).


def test_array_fetch_oob_is_rescued_as_index_error() -> None:
    from coding_adventures_sir_runtime_exceptions import rescue_matches

    try:
        oop.call_method([1, 2], "fetch", 100)
        raise AssertionError("fetch did not raise")  # pragma: no cover
    except Exception as exc:  # noqa: BLE001 - mirrors the emitted broad catch
        assert rescue_matches(exc, ["IndexError"]) is True
        assert rescue_matches(exc, ["StandardError"]) is True
        assert rescue_matches(exc, ["KeyError"]) is False  # sibling clause misses


def test_hash_fetch_miss_is_rescued_as_key_error() -> None:
    from coding_adventures_sir_runtime_exceptions import rescue_matches

    try:
        oop.call_method({"a": 1}, "fetch", "z")
        raise AssertionError("fetch did not raise")  # pragma: no cover
    except Exception as exc:  # noqa: BLE001
        assert rescue_matches(exc, ["KeyError"]) is True
        # KeyError < IndexError < StandardError in the built-in ancestry, so a
        # ``rescue IndexError`` also catches a raised KeyError (Ruby semantics).
        assert rescue_matches(exc, ["IndexError"]) is True
        assert rescue_matches(exc, ["StandardError"]) is True


def test_unknown_method_is_rescued_as_no_method_error() -> None:
    from coding_adventures_sir_runtime_exceptions import rescue_matches

    try:
        oop.call_method(oop.new_instance("Widget"), "undefined")
        raise AssertionError("dispatch did not raise")  # pragma: no cover
    except Exception as exc:  # noqa: BLE001
        assert rescue_matches(exc, ["NoMethodError"]) is True
        # NoMethodError < NameError < StandardError — a ``rescue NameError``
        # catches it too (Ruby semantics).
        assert rescue_matches(exc, ["NameError"]) is True
        assert rescue_matches(exc, ["StandardError"]) is True


def test_nil_unknown_method_is_no_method_error() -> None:
    # ``nil.foo`` raises NoMethodError in Ruby (not the old silent nil).
    with pytest.raises(SirError) as excinfo:
        oop.call_method(None, "foo")
    assert excinfo.value.sir_class == "NoMethodError"
    assert excinfo.value.args[0] == "undefined method 'foo' for NilClass"


def test_index_operator_still_returns_nil_no_over_raise() -> None:
    # REGRESSION: ``.fetch`` raises, but the plain index operator ``arr[i]`` /
    # ``hash[k]`` must NOT — Ruby returns nil there.  The backend emits ``[]`` as
    # a *native* Python subscript that never routes through ``call_method`` /
    # ``.fetch``, so the two paths stay independent.  A missing hash key via the
    # ``dict.get`` semantics the ``[]`` lowering relies on still yields nil, and
    # the catalogued (non-fetch) accessors keep their nil returns.
    assert {"a": 1}.get("missing") is None  # the [] lowering's miss → nil
    assert oop.call_method([], "first") is None  # empty-array accessor stays nil
    assert oop.call_method({"a": 1}, "dig", "z") is None  # dig miss stays nil


# ── MX2: mixins — include / extend / MRO ──────────────────────────────────────


def test_include_makes_module_instance_method_reachable() -> None:
    # module Greetable; def greet; "hi"; end; end + class Robot; include Greetable.
    oop.define_class("Robot", None)
    oop.def_method("Greetable", "greet", Closure(lambda: "hi"))
    oop.include_module("Robot", "Greetable")
    robot = oop.call_new("Robot")
    assert oop.call_method(robot, "greet") == "hi"


def test_class_own_method_shadows_included_module() -> None:
    # A method the class defines itself wins over the module's (class-first MRO).
    oop.define_class("Widget", None)
    oop.def_method("Nameable", "name", Closure(lambda: "module"))
    oop.include_module("Widget", "Nameable")
    oop.def_method("Widget", "name", Closure(lambda: "class"))
    widget = oop.call_new("Widget")
    assert oop.call_method(widget, "name") == "class"


def test_most_recently_included_module_wins() -> None:
    # include A, then include B: B (most recent) shadows A for a shared method.
    oop.define_class("C", None)
    oop.def_method("A", "who", Closure(lambda: "A"))
    oop.def_method("B", "who", Closure(lambda: "B"))
    oop.include_module("C", "A")
    oop.include_module("C", "B")
    c = oop.call_new("C")
    assert oop.call_method(c, "who") == "B"


def test_module_shadows_superclass() -> None:
    # A module method is found before the superclass's (module precedes super in MRO).
    oop.define_class("Base", None)
    oop.define_class("Derived", "Base")
    oop.def_method("Base", "kind", Closure(lambda: "base"))
    oop.def_method("Mix", "kind", Closure(lambda: "mix"))
    oop.include_module("Derived", "Mix")
    obj = oop.call_new("Derived")
    assert oop.call_method(obj, "kind") == "mix"


def test_included_module_can_access_ivars_via_current_self() -> None:
    # A mixed-in method runs with the receiver pushed as current self.
    oop.define_class("Account", None)
    oop.def_method("Account", "initialize", Closure(lambda bal: oop.ivar_set("@bal", bal)))
    oop.def_method("Reportable", "report", Closure(lambda: oop.ivar_get("@bal")))
    oop.include_module("Account", "Reportable")
    acct = oop.call_new("Account", 42)
    assert oop.call_method(acct, "report") == 42


def test_diamond_include_resolves_once() -> None:
    # M includes Base; C includes M and Base directly (two paths to Base).  The
    # MRO lists Base exactly once and resolution finds Base#origin.
    oop.define_class("C", None)
    oop.def_method("Base", "origin", Closure(lambda: "base"))
    oop.include_module("M", "Base")
    oop.include_module("C", "Base")
    oop.include_module("C", "M")
    mro = oop.oop._owner_mro("C")
    assert mro.count("Base") == 1
    c = oop.call_new("C")
    assert oop.call_method(c, "origin") == "base"


def test_self_including_module_terminates() -> None:
    # A module that (pathologically) includes itself must not loop the MRO walk.
    oop.define_class("C", None)
    oop.def_method("Loopy", "go", Closure(lambda: "ok"))
    oop.include_module("Loopy", "Loopy")  # cycle
    oop.include_module("C", "Loopy")
    c = oop.call_new("C")
    assert oop.call_method(c, "go") == "ok"


def test_extend_registers_module_method_as_class_method() -> None:
    # extend M mixes M's instance methods in as class/singleton methods.
    oop.define_class("Widget", None)
    oop.def_method("Counting", "count", Closure(lambda: 7))
    oop.extend_module("Widget", "Counting")
    assert oop.call_class_method("Widget", "count") == 7


def test_extend_does_not_add_instance_method() -> None:
    # extend attaches to the class, NOT to instances — an instance call is a
    # NoMethodError (the method is not on the instance MRO).
    oop.define_class("Widget", None)
    oop.def_method("Counting", "count", Closure(lambda: 7))
    oop.extend_module("Widget", "Counting")
    w = oop.call_new("Widget")
    with pytest.raises(SirError) as excinfo:
        oop.call_method(w, "count")
    assert excinfo.value.sir_class == "NoMethodError"


def test_include_unknown_method_still_raises_no_method_error() -> None:
    # include adds behaviour but does not swallow genuinely missing methods.
    oop.define_class("Robot", None)
    oop.def_method("Greetable", "greet", Closure(lambda: "hi"))
    oop.include_module("Robot", "Greetable")
    robot = oop.call_new("Robot")
    with pytest.raises(SirError) as excinfo:
        oop.call_method(robot, "nope")
    assert excinfo.value.sir_class == "NoMethodError"
