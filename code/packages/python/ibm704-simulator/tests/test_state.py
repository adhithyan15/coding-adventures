"""Tests for IBM704State — a frozen, immutable snapshot."""

from __future__ import annotations

from dataclasses import FrozenInstanceError

import pytest

from ibm704_simulator import IBM704Simulator, IBM704State


def _zero_memory() -> tuple[int, ...]:
    return tuple([0] * 32768)


def _make_state(**overrides: object) -> IBM704State:
    base: dict[str, object] = dict(
        accumulator_sign=False,
        accumulator_p=False,
        accumulator_q=False,
        accumulator_magnitude=0,
        mq=0,
        mq_sign=False,
        mq_magnitude=0,
        index_a=0,
        index_b=0,
        index_c=0,
        pc=0,
        halted=False,
        overflow_trigger=False,
        divide_check_trigger=False,
        memory=_zero_memory(),
    )
    base.update(overrides)
    return IBM704State(**base)  # type: ignore[arg-type]


def test_state_is_frozen() -> None:
    state = _make_state()
    with pytest.raises(FrozenInstanceError):
        state.accumulator_magnitude = 99  # type: ignore[misc]


def test_state_memory_is_immutable_tuple() -> None:
    state = _make_state()
    assert isinstance(state.memory, tuple)
    # Tuples don't have item assignment.
    with pytest.raises(TypeError):
        state.memory[0] = 1  # type: ignore[index]


def test_get_state_returns_copy_not_reference() -> None:
    sim = IBM704Simulator()
    snapshot_before = sim.get_state()
    # Mutate live simulator state directly.
    sim._memory[42] = 0xDEADBEEF  # noqa: SLF001
    assert snapshot_before.memory[42] == 0


def test_get_state_reflects_current_state() -> None:
    sim = IBM704Simulator()
    sim._memory[100] = 0x123456789  # noqa: SLF001
    state = sim.get_state()
    assert state.memory[100] == 0x123456789


def test_state_has_all_required_fields() -> None:
    state = _make_state()
    # Spot check the fields the SIM00 protocol cares about.
    assert hasattr(state, "pc")
    assert hasattr(state, "halted")
    assert hasattr(state, "memory")
