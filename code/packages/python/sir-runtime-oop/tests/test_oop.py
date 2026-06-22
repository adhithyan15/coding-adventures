"""Tests for coding-adventures-sir-runtime-oop."""

from __future__ import annotations

import pytest
from coding_adventures_sir_runtime_core import Closure

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


def test_class_returns_class_name_and_unknown_methods_return_nil() -> None:
    assert oop.call_method(oop.new_instance("Foo"), "class") == "Foo"
    assert oop.call_method(3, "class") == "Integer"
    assert oop.call_method(3, "no_such_method") is None


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
    # An out-of-catalog method:
    assert oop.call_method([1], "respond_to?", "each_slice") is False


def test_unknown_method_returns_nil_not_raise() -> None:
    # A block method called WITHOUT a block bottoms out at nil (Ruby returns an
    # Enumerator; v0 floor is nil — see spec).
    assert oop.call_method([1, 2, 3], "map") is None
    # An out-of-catalog String method (scan needs a regex engine — later PR).
    assert oop.call_method("hi", "scan") is None
    # Numeric has no catalog yet (M1c-Numeric), so every method is the nil floor.
    assert oop.call_method(5, "times") is None


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
    assert oop.call_method(h, "fetch", "z") is None
    assert oop.call_method(h, "fetch", "z", 99) == 99
    assert oop.call_method(h, "dig", "b") == 2
    assert oop.call_method(h, "to_a") == [["a", 1], ["b", 2]]


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


def test_hash_respond_to_and_nil_floor() -> None:
    assert oop.call_method({"a": 1}, "respond_to?", "keys") is True
    assert oop.call_method({"a": 1}, "respond_to?", "each") is True
    assert oop.call_method({"a": 1}, "respond_to?", "transform_keys") is False
    assert oop.call_method({"a": 1}, "transform_keys") is None
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


def test_string_each_char_block() -> None:
    seen: list[Val] = []
    result = oop.call_method("abc", "each_char", Closure(seen.append))
    assert seen == ["a", "b", "c"]
    assert result == "abc"


def test_string_respond_to_and_nil_floor() -> None:
    assert oop.call_method("x", "respond_to?", "upcase") is True
    assert oop.call_method("x", "respond_to?", "each_char") is True
    assert oop.call_method("x", "respond_to?", "scan") is False
    assert oop.call_method("x", "scan") is None
    # Universal Object methods still resolve on a String receiver.
    assert oop.call_method("x", "nil?") is False
    assert oop.call_method("x", "class") == "String"
