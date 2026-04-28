"""Unit tests for ``SlotState`` — the V8 Ignition-style feedback slot.

The state machine is the core of the LANG17 metrics surface, so these
tests cover each transition explicitly plus a few corner cases around
the MEGAMORPHIC cap.  A companion test module
(``test_instr_observation.py``) exercises the *integration* with
``IIRInstr.record_observation``.
"""

from __future__ import annotations

from interpreter_ir import (
    MAX_POLYMORPHIC_OBSERVATIONS,
    SlotKind,
    SlotState,
)

# ---------------------------------------------------------------------------
# Defaults
# ---------------------------------------------------------------------------


def test_default_state_is_uninitialized() -> None:
    slot = SlotState()
    assert slot.kind is SlotKind.UNINITIALIZED
    assert slot.observations == []
    assert slot.count == 0


def test_uninitialized_is_not_specialisable() -> None:
    slot = SlotState()
    assert slot.is_specialisable() is False
    assert slot.dominant_type() is None


# ---------------------------------------------------------------------------
# MONOMORPHIC transitions
# ---------------------------------------------------------------------------


def test_first_observation_goes_monomorphic() -> None:
    slot = SlotState()
    slot.record("u8")
    assert slot.kind is SlotKind.MONOMORPHIC
    assert slot.observations == ["u8"]
    assert slot.count == 1


def test_repeat_of_same_type_stays_monomorphic() -> None:
    slot = SlotState()
    for _ in range(10):
        slot.record("u8")
    assert slot.kind is SlotKind.MONOMORPHIC
    assert slot.observations == ["u8"]
    assert slot.count == 10


def test_monomorphic_is_specialisable() -> None:
    slot = SlotState()
    slot.record("u8")
    assert slot.is_specialisable() is True
    assert slot.dominant_type() == "u8"


# ---------------------------------------------------------------------------
# POLYMORPHIC transitions
# ---------------------------------------------------------------------------


def test_second_distinct_type_goes_polymorphic() -> None:
    slot = SlotState()
    slot.record("u8")
    slot.record("str")
    assert slot.kind is SlotKind.POLYMORPHIC
    assert slot.observations == ["u8", "str"]
    assert slot.count == 2


def test_polymorphic_stays_polymorphic_up_to_four_distinct() -> None:
    slot = SlotState()
    for ty in ["int", "str", "bool", "float"]:
        slot.record(ty)
    assert slot.kind is SlotKind.POLYMORPHIC
    assert slot.observations == ["int", "str", "bool", "float"]
    assert len(slot.observations) == MAX_POLYMORPHIC_OBSERVATIONS


def test_polymorphic_is_not_specialisable() -> None:
    slot = SlotState()
    slot.record("u8")
    slot.record("str")
    assert slot.is_specialisable() is False
    assert slot.dominant_type() is None


def test_polymorphic_revisit_existing_type() -> None:
    """Seeing a type already in observations updates count but not kind."""
    slot = SlotState()
    slot.record("int")
    slot.record("str")
    slot.record("int")  # revisit
    assert slot.kind is SlotKind.POLYMORPHIC
    assert slot.observations == ["int", "str"]
    assert slot.count == 3


# ---------------------------------------------------------------------------
# MEGAMORPHIC transitions
# ---------------------------------------------------------------------------


def test_fifth_distinct_type_goes_megamorphic() -> None:
    slot = SlotState()
    for ty in ["int", "str", "bool", "float", "list"]:
        slot.record(ty)
    assert slot.kind is SlotKind.MEGAMORPHIC
    # Observations list is discarded on transition to bound memory.
    assert slot.observations == []
    assert slot.count == 5


def test_megamorphic_is_sticky() -> None:
    """Once MEGAMORPHIC, never downgrades — even if subsequent records
    happen to all be the same type."""
    slot = SlotState()
    for ty in ["a", "b", "c", "d", "e"]:
        slot.record(ty)
    assert slot.kind is SlotKind.MEGAMORPHIC
    for _ in range(100):
        slot.record("a")  # any further observations
    assert slot.kind is SlotKind.MEGAMORPHIC
    assert slot.observations == []
    assert slot.count == 105


def test_megamorphic_helpers() -> None:
    slot = SlotState()
    for ty in ["a", "b", "c", "d", "e"]:
        slot.record(ty)
    assert slot.is_megamorphic() is True
    assert slot.is_specialisable() is False
    assert slot.dominant_type() is None


# ---------------------------------------------------------------------------
# Language-agnosticism — the state machine does not inspect string contents
# ---------------------------------------------------------------------------


def test_works_with_lisp_style_type_names() -> None:
    slot = SlotState()
    for ty in ["cons", "symbol", "closure", "nil"]:
        slot.record(ty)
    assert slot.kind is SlotKind.POLYMORPHIC
    assert slot.observations == ["cons", "symbol", "closure", "nil"]


def test_works_with_javascript_style_type_names() -> None:
    slot = SlotState()
    for ty in ["Number", "String"]:
        slot.record(ty)
    assert slot.kind is SlotKind.POLYMORPHIC
    assert slot.dominant_type() is None


def test_works_with_python_style_type_names() -> None:
    slot = SlotState()
    slot.record("int")
    slot.record("int")
    assert slot.kind is SlotKind.MONOMORPHIC
    assert slot.dominant_type() == "int"


def test_arbitrary_strings_treated_by_equality() -> None:
    """Whitespace and casing are significant — the state machine doesn't
    normalise.  (Frontends are expected to be consistent in their naming.)"""
    slot = SlotState()
    slot.record("int")
    slot.record("Int")
    slot.record("INT")
    assert slot.kind is SlotKind.POLYMORPHIC
    assert slot.observations == ["int", "Int", "INT"]


# ---------------------------------------------------------------------------
# The max-observations constant is the documented cap
# ---------------------------------------------------------------------------


def test_max_polymorphic_observations_is_four() -> None:
    """Four is V8 Ignition's value; keep it pinned for external consumers."""
    assert MAX_POLYMORPHIC_OBSERVATIONS == 4
