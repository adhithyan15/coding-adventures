"""Tests for the LANG16 heap / GC additions to InterpreterIR.

These verify the *schema* contract — the new opcodes appear in the
right opcode-category sets, ``ref<T>`` types parse and round-trip
correctly, and the new ``IIRInstr.may_alloc`` field defaults to
False so existing programs see no behaviour change.

Runtime semantics (when does a collection actually fire, what does
``alloc`` return, etc.) live in vm-core and the GC algorithms; tests
for those land in their own packages.
"""

from __future__ import annotations

from interpreter_ir import (
    ALL_OPS,
    ALLOCATING_OPS,
    HEAP_OPS,
    SIDE_EFFECT_OPS,
    VALUE_OPS,
    IIRInstr,
    is_ref_type,
    make_ref_type,
    unwrap_ref_type,
)

# ---------------------------------------------------------------------------
# Heap opcodes — schema membership
# ---------------------------------------------------------------------------


def test_heap_opcodes_are_in_all_ops() -> None:
    """Every HEAP_OPS member is also in ALL_OPS."""
    for op in HEAP_OPS:
        assert op in ALL_OPS


def test_heap_ops_canonical_set() -> None:
    """The seven LANG16 heap opcodes are present and the set is exact."""
    assert frozenset({
        "alloc",
        "box",
        "unbox",
        "field_load",
        "field_store",
        "is_null",
        "safepoint",
    }) == HEAP_OPS


def test_value_producing_heap_ops_are_in_value_ops() -> None:
    """``alloc``, ``box``, ``unbox``, ``field_load``, ``is_null``
    produce values and so must appear in ``VALUE_OPS``."""
    for op in ("alloc", "box", "unbox", "field_load", "is_null"):
        assert op in VALUE_OPS


def test_side_effect_heap_ops_are_in_side_effect_ops() -> None:
    """``field_store`` mutates the heap and ``safepoint`` may yield
    to the GC; both must appear in ``SIDE_EFFECT_OPS``."""
    assert "field_store" in SIDE_EFFECT_OPS
    assert "safepoint" in SIDE_EFFECT_OPS


def test_allocating_ops_subset_of_heap_ops() -> None:
    """``ALLOCATING_OPS`` must be a subset of ``HEAP_OPS``."""
    assert ALLOCATING_OPS.issubset(HEAP_OPS)


def test_allocating_ops_canonical_set() -> None:
    """Allocations happen at ``alloc``, ``box``, and ``safepoint``."""
    assert frozenset({"alloc", "box", "safepoint"}) == ALLOCATING_OPS


# ---------------------------------------------------------------------------
# ref<T> type encoding
# ---------------------------------------------------------------------------


def test_is_ref_type_recognises_ref_strings() -> None:
    assert is_ref_type("ref<u8>") is True
    assert is_ref_type("ref<any>") is True
    assert is_ref_type("ref<ref<any>>") is True
    assert is_ref_type("ref<some-language-type>") is True


def test_is_ref_type_rejects_plain_types() -> None:
    assert is_ref_type("u8") is False
    assert is_ref_type("any") is False
    assert is_ref_type("bool") is False
    assert is_ref_type("") is False


def test_unwrap_ref_type_returns_inner() -> None:
    assert unwrap_ref_type("ref<u8>") == "u8"
    assert unwrap_ref_type("ref<any>") == "any"


def test_unwrap_ref_type_handles_nested_refs() -> None:
    """``ref<ref<any>>`` unwraps once to ``ref<any>``."""
    assert unwrap_ref_type("ref<ref<any>>") == "ref<any>"
    # And once more.
    assert unwrap_ref_type("ref<any>") == "any"


def test_unwrap_ref_type_returns_none_for_non_ref() -> None:
    assert unwrap_ref_type("u8") is None
    assert unwrap_ref_type("any") is None
    assert unwrap_ref_type("ref") is None  # not "ref<...>"


def test_make_and_unwrap_round_trip() -> None:
    """``unwrap(make(t)) == t`` for any base type."""
    for t in ("u8", "u16", "any", "str", "ref<any>", "cons", "symbol"):
        assert unwrap_ref_type(make_ref_type(t)) == t


def test_make_ref_type_format() -> None:
    assert make_ref_type("u8") == "ref<u8>"
    assert make_ref_type("any") == "ref<any>"


# ---------------------------------------------------------------------------
# IIRInstr — may_alloc field
# ---------------------------------------------------------------------------


def test_may_alloc_defaults_to_false() -> None:
    """Existing callers that don't set ``may_alloc`` see the safe default."""
    instr = IIRInstr("add", "v0", ["a", "b"], "u8")
    assert instr.may_alloc is False


def test_may_alloc_is_settable() -> None:
    """Frontends set ``may_alloc=True`` for allocating instructions."""
    instr = IIRInstr("alloc", "v0", [16, 0], "ref<any>", may_alloc=True)
    assert instr.may_alloc is True


def test_may_alloc_does_not_affect_equality() -> None:
    """``may_alloc`` is ``compare=False`` so it does not affect ``==``."""
    a = IIRInstr("add", "v0", ["a", "b"], "u8")
    b = IIRInstr("add", "v0", ["a", "b"], "u8", may_alloc=True)
    assert a == b


# ---------------------------------------------------------------------------
# is_typed() recognises ref<T> as typed
# ---------------------------------------------------------------------------


def test_is_typed_true_for_ref_types() -> None:
    """``ref<T>`` is a concrete type — JITs should not run the
    profiler on it."""
    instr = IIRInstr("alloc", "v0", [16, 0], "ref<any>")
    assert instr.is_typed() is True


def test_is_typed_false_for_any() -> None:
    instr = IIRInstr("add", "v0", ["a", "b"], "any")
    assert instr.is_typed() is False


def test_is_typed_true_for_plain_concrete() -> None:
    """Sanity: existing concrete types still report typed."""
    instr = IIRInstr("add", "v0", ["a", "b"], "u8")
    assert instr.is_typed() is True


# ---------------------------------------------------------------------------
# Heap-opcode IIRInstr smoke tests — construction must not raise
# ---------------------------------------------------------------------------


def test_alloc_instr() -> None:
    """``alloc size, kind`` produces a ``ref<any>``."""
    instr = IIRInstr(
        "alloc",
        "cell",
        [16, 0],            # size, kind_id
        "ref<any>",
        may_alloc=True,
    )
    assert instr.op == "alloc"
    assert instr.may_alloc is True
    assert is_ref_type(instr.type_hint)


def test_box_instr() -> None:
    """``box value`` boxes a primitive into a single-slot heap cell."""
    instr = IIRInstr("box", "p", ["x"], "ref<u8>", may_alloc=True)
    assert instr.op == "box"
    assert unwrap_ref_type(instr.type_hint) == "u8"


def test_unbox_instr() -> None:
    """``unbox ref`` derefs a single-slot box."""
    instr = IIRInstr("unbox", "x", ["p"], "u8")
    assert instr.op == "unbox"
    assert instr.may_alloc is False


def test_field_load_instr() -> None:
    """``field_load ref, offset`` reads one field from a heap object."""
    instr = IIRInstr("field_load", "car", ["cell", 0], "any")
    assert instr.op == "field_load"


def test_field_store_instr() -> None:
    """``field_store ref, offset, value`` writes one field; the field
    is in ``srcs`` because ``field_store`` is void."""
    instr = IIRInstr(
        "field_store",
        None,
        ["cell", 1, "newval"],
        "void",
    )
    assert instr.op == "field_store"
    assert instr.dest is None


def test_is_null_instr() -> None:
    """``is_null ref`` returns a bool."""
    instr = IIRInstr("is_null", "empty", ["cell"], "bool")
    assert instr.op == "is_null"


def test_safepoint_instr() -> None:
    """``safepoint`` is void and may trigger a collection."""
    instr = IIRInstr("safepoint", None, [], "void", may_alloc=True)
    assert instr.op == "safepoint"
    assert instr.dest is None
    assert instr.may_alloc is True
