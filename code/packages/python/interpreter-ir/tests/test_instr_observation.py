"""Tests for the integration between ``IIRInstr.record_observation`` and
the new ``SlotState`` machine (LANG17).

The invariant under test: ``observed_slot``, ``observed_type``, and
``observation_count`` always stay consistent with each other after any
sequence of ``record_observation`` calls.  Callers that read the legacy
``observed_type`` view continue to see ``None`` → concrete →
``"polymorphic"``; callers that read the new ``observed_slot`` see the
full four-state machine.
"""

from __future__ import annotations

from interpreter_ir import IIRInstr, SlotKind


def _make_any_instr() -> IIRInstr:
    """Return a dynamically-typed test instruction."""
    return IIRInstr("add", "v0", ["a", "b"], "any")


# ---------------------------------------------------------------------------
# Defaults
# ---------------------------------------------------------------------------


def test_fresh_instr_has_no_observation() -> None:
    instr = _make_any_instr()
    assert instr.observed_type is None
    assert instr.observation_count == 0
    assert instr.observed_slot is None
    assert instr.has_observation() is False


# ---------------------------------------------------------------------------
# First observation — transitions slot from None to MONOMORPHIC
# ---------------------------------------------------------------------------


def test_first_observation_populates_all_three_views() -> None:
    instr = _make_any_instr()
    instr.record_observation("u8")

    # Legacy views.
    assert instr.observed_type == "u8"
    assert instr.observation_count == 1

    # New slot view.
    assert instr.observed_slot is not None
    assert instr.observed_slot.kind is SlotKind.MONOMORPHIC
    assert instr.observed_slot.observations == ["u8"]
    assert instr.observed_slot.count == 1


# ---------------------------------------------------------------------------
# Monomorphic repeat — all three views track the count but stay stable
# ---------------------------------------------------------------------------


def test_repeated_observations_of_same_type() -> None:
    instr = _make_any_instr()
    for _ in range(7):
        instr.record_observation("u8")

    assert instr.observed_type == "u8"
    assert instr.observation_count == 7
    assert instr.observed_slot is not None
    assert instr.observed_slot.kind is SlotKind.MONOMORPHIC
    assert instr.observed_slot.observations == ["u8"]
    assert instr.observed_slot.count == 7


# ---------------------------------------------------------------------------
# Polymorphic — legacy view shows "polymorphic", slot retains full list
# ---------------------------------------------------------------------------


def test_polymorphic_legacy_view_says_polymorphic() -> None:
    instr = _make_any_instr()
    instr.record_observation("u8")
    instr.record_observation("str")

    assert instr.observed_type == "polymorphic"
    assert instr.observation_count == 2
    assert instr.is_polymorphic() is True

    # The richer view retains both distinct types.
    assert instr.observed_slot.kind is SlotKind.POLYMORPHIC
    assert instr.observed_slot.observations == ["u8", "str"]


def test_polymorphic_up_to_four_types() -> None:
    instr = _make_any_instr()
    for ty in ["a", "b", "c", "d"]:
        instr.record_observation(ty)
    assert instr.observed_type == "polymorphic"
    assert instr.observed_slot.kind is SlotKind.POLYMORPHIC
    assert instr.observed_slot.observations == ["a", "b", "c", "d"]


# ---------------------------------------------------------------------------
# Megamorphic — slot transitions; legacy view still says "polymorphic"
# (because the two-state model cannot distinguish the two)
# ---------------------------------------------------------------------------


def test_megamorphic_legacy_view_still_polymorphic() -> None:
    instr = _make_any_instr()
    for ty in ["a", "b", "c", "d", "e"]:
        instr.record_observation(ty)

    # Slot is MEGAMORPHIC …
    assert instr.observed_slot.kind is SlotKind.MEGAMORPHIC
    assert instr.observed_slot.observations == []
    assert instr.observed_slot.count == 5

    # … but the legacy two-state view still says "polymorphic" (it can't
    # distinguish POLY from MEGA).  Old callers of is_polymorphic() get
    # the conservative "don't specialise" signal either way.
    assert instr.observed_type == "polymorphic"
    assert instr.is_polymorphic() is True
    assert instr.observation_count == 5


def test_megamorphic_sticky_through_more_observations() -> None:
    instr = _make_any_instr()
    for ty in ["a", "b", "c", "d", "e"]:
        instr.record_observation(ty)
    for _ in range(50):
        instr.record_observation("a")

    assert instr.observed_slot.kind is SlotKind.MEGAMORPHIC
    assert instr.observation_count == 55
    assert instr.observed_slot.count == 55


# ---------------------------------------------------------------------------
# effective_type stays consistent
# ---------------------------------------------------------------------------


def test_effective_type_monomorphic() -> None:
    instr = _make_any_instr()
    instr.record_observation("u8")
    assert instr.effective_type() == "u8"


def test_effective_type_polymorphic_falls_back_to_any() -> None:
    instr = _make_any_instr()
    instr.record_observation("u8")
    instr.record_observation("str")
    # effective_type returns "any" for polymorphic (legacy behaviour).
    assert instr.effective_type() == "any"


# ---------------------------------------------------------------------------
# Statically-typed instructions — the profiler in vm-core won't record
# these, but if a caller does record_observation directly, the state
# machine still tracks (useful for tests / tooling).
# ---------------------------------------------------------------------------


def test_record_observation_works_on_concrete_typed_instr() -> None:
    instr = IIRInstr("add", "v0", ["a", "b"], "u8")
    instr.record_observation("u8")
    # effective_type uses the concrete hint, not the observation.
    assert instr.effective_type() == "u8"
    # But the slot is still populated — useful for debug tools that want
    # to see every runtime observation.
    assert instr.observed_slot is not None
    assert instr.observed_slot.kind is SlotKind.MONOMORPHIC


# ---------------------------------------------------------------------------
# Repr remains unchanged on the happy path
# ---------------------------------------------------------------------------


def test_repr_with_observation() -> None:
    instr = _make_any_instr()
    instr.record_observation("u8")
    rendered = repr(instr)
    # The legacy repr shows the observed_type and count.
    assert "obs=" in rendered
    assert "u8" in rendered
