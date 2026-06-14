"""Shared test helpers for the AArch64 simulator test suite."""

import pytest

from aarch64_simulator import AArch64Simulator, AArch64State


def run_from_current(sim: AArch64Simulator) -> tuple[AArch64State, int]:
    """
    Step `sim` until HALT or 100 000 steps, starting from its current state.

    Returns (final_state, step_count).  Used by tests that pre-configure
    registers before running — call sim.load() first (to zero memory), then
    set sim._state, then call this helper.
    """
    steps = 0
    max_steps = 100_000
    while steps < max_steps:
        trace = sim.step()
        steps += 1
        if trace.mnemonic.startswith("ERROR:"):
            break
        if sim.get_state().halted:
            break
    return sim.get_state(), steps


@pytest.fixture
def sim() -> AArch64Simulator:
    """Return a freshly reset AArch64Simulator."""
    return AArch64Simulator()
