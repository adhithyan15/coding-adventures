"""Shared test helpers for the PowerPC 601 simulator test suite."""

from powerpc601_simulator import PowerPC601Simulator, PowerPC601State


def run_from_current(
    sim: PowerPC601Simulator, max_steps: int = 1000
) -> tuple[PowerPC601State, str | None]:
    """Step an already-loaded simulator without re-loading (preserves preset registers)."""
    for _ in range(max_steps):
        if sim.get_state().halted:
            break
        trace = sim.step()
        if sim.get_state().halted:
            break
        if trace.mnemonic.startswith("ERROR:"):
            return sim.get_state(), trace.mnemonic
    return sim.get_state(), None
