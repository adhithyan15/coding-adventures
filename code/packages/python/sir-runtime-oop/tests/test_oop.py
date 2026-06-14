"""Tests for coding-adventures-sir-runtime-oop."""

from __future__ import annotations

import pytest

import coding_adventures_sir_runtime_oop as oop


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
