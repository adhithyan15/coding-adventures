"""SIM00 protocol compliance tests for PowerPC601Simulator."""

from simulator_protocol import ExecutionResult, Simulator, StepTrace

from powerpc601_simulator import (
    HALT,
    PowerPC601Simulator,
    PowerPC601State,
    d_form,
    i_form,
)
from powerpc601_simulator.simulator import PO_ADDI, PO_B

# ── Helpers ────────────────────────────────────────────────────────────────────


def simple_prog() -> bytes:
    """addi r3, 0, 42  then HALT — smallest non-trivial program."""
    return d_form(PO_ADDI, 3, 0, 42) + HALT


# ── isinstance checks ──────────────────────────────────────────────────────────


def test_is_simulator():
    sim = PowerPC601Simulator()
    assert isinstance(sim, Simulator)


def test_is_concrete_simulator():
    sim = PowerPC601Simulator()
    assert isinstance(sim, PowerPC601Simulator)


# ── reset() ───────────────────────────────────────────────────────────────────


def test_reset_clears_gprs():
    sim = PowerPC601Simulator()
    sim.execute(simple_prog())
    sim.reset()
    s = sim.get_state()
    assert s.gpr == tuple(0 for _ in range(32))


def test_reset_clears_special_regs():
    sim = PowerPC601Simulator()
    sim.execute(simple_prog())
    sim.reset()
    s = sim.get_state()
    assert s.lr == 0
    assert s.ctr == 0
    assert s.xer == 0
    assert s.cr == 0


def test_reset_clears_memory():
    sim = PowerPC601Simulator()
    sim.execute(simple_prog())
    sim.reset()
    s = sim.get_state()
    assert all(b == 0 for b in s.memory)


def test_reset_sets_cia_zero():
    sim = PowerPC601Simulator()
    sim.execute(simple_prog())
    sim.reset()
    assert sim.get_state().cia == 0


def test_reset_clears_halted():
    sim = PowerPC601Simulator()
    sim.execute(simple_prog())
    assert sim.get_state().halted
    sim.reset()
    assert not sim.get_state().halted


# ── load() ────────────────────────────────────────────────────────────────────


def test_load_resets_state():
    sim = PowerPC601Simulator()
    sim.execute(simple_prog())
    sim.load(simple_prog())
    s = sim.get_state()
    assert s.cia == 0
    assert not s.halted


def test_load_places_bytes_in_memory():
    sim = PowerPC601Simulator()
    sim.load(simple_prog())
    s = sim.get_state()
    # First 4 bytes encode the addi instruction — memory[0] must be non-zero
    assert s.memory[0] != 0


# ── step() ────────────────────────────────────────────────────────────────────


def test_step_returns_step_trace():
    sim = PowerPC601Simulator()
    sim.load(simple_prog())
    trace = sim.step()
    assert isinstance(trace, StepTrace)


def test_step_trace_has_pc_before():
    sim = PowerPC601Simulator()
    sim.load(simple_prog())
    trace = sim.step()
    assert trace.pc_before == 0


def test_step_trace_has_pc_after():
    sim = PowerPC601Simulator()
    sim.load(simple_prog())
    trace = sim.step()
    # addi is a single 32-bit instruction → CIA advances by 4
    assert trace.pc_after == 4


def test_step_trace_has_mnemonic():
    sim = PowerPC601Simulator()
    sim.load(simple_prog())
    trace = sim.step()
    assert "addi" in trace.mnemonic


def test_step_trace_description_not_empty():
    sim = PowerPC601Simulator()
    sim.load(simple_prog())
    trace = sim.step()
    assert trace.description


def test_step_when_halted_returns_halt_trace():
    sim = PowerPC601Simulator()
    sim.execute(simple_prog())
    trace = sim.step()
    assert "HALT" in trace.mnemonic
    assert trace.pc_before == trace.pc_after


def test_step_advances_cia():
    sim = PowerPC601Simulator()
    sim.load(simple_prog())
    sim.step()   # addi (4 bytes)
    assert sim.get_state().cia == 4


# ── execute() ─────────────────────────────────────────────────────────────────


def test_execute_returns_execution_result():
    sim = PowerPC601Simulator()
    result = sim.execute(simple_prog())
    assert isinstance(result, ExecutionResult)


def test_execute_halted_true():
    sim = PowerPC601Simulator()
    result = sim.execute(simple_prog())
    assert result.halted


def test_execute_ok():
    sim = PowerPC601Simulator()
    result = sim.execute(simple_prog())
    assert result.ok


def test_execute_steps_positive():
    sim = PowerPC601Simulator()
    result = sim.execute(simple_prog())
    assert result.steps > 0


def test_execute_final_state_is_powerpc601state():
    sim = PowerPC601Simulator()
    result = sim.execute(simple_prog())
    assert isinstance(result.final_state, PowerPC601State)


def test_execute_r3_set():
    sim = PowerPC601Simulator()
    result = sim.execute(simple_prog())
    assert result.final_state.r3 == 42


def test_execute_traces_list():
    sim = PowerPC601Simulator()
    result = sim.execute(simple_prog())
    assert isinstance(result.traces, list)
    assert len(result.traces) > 0


def test_execute_no_error_on_success():
    sim = PowerPC601Simulator()
    result = sim.execute(simple_prog())
    assert result.error is None


def test_execute_max_steps_exceeded():
    sim = PowerPC601Simulator()
    # Infinite loop: b 0 (branch to self)
    inf_loop = i_form(PO_B, 0)   # b 0 (branch to CIA+0 = itself)
    result = sim.execute(inf_loop, max_steps=10)
    assert not result.ok
    assert result.error is not None


# ── get_state() ───────────────────────────────────────────────────────────────


def test_get_state_returns_powerpc601state():
    sim = PowerPC601Simulator()
    assert isinstance(sim.get_state(), PowerPC601State)


def test_get_state_snapshot_not_mutated():
    sim = PowerPC601Simulator()
    sim.load(simple_prog())
    snap = sim.get_state()
    cia_before = snap.cia
    sim.step()
    assert snap.cia == cia_before


def test_get_state_is_frozen():
    sim = PowerPC601Simulator()
    s = sim.get_state()
    raised = False
    try:
        s.cia = 99  # type: ignore[misc]
    except (AttributeError, TypeError):
        raised = True
    assert raised, "Frozen dataclass should not allow attribute assignment"
