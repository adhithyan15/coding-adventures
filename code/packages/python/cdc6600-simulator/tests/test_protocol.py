"""SIM00 protocol compliance tests for CDC6600Simulator."""

from simulator_protocol import ExecutionResult, Simulator, StepTrace

from cdc6600_simulator import HALT, CDC6600Simulator, CDC6600State, long_instr
from cdc6600_simulator.simulator import F_LDXI

# ── Helpers ────────────────────────────────────────────────────────────────────

def simple_prog() -> bytes:
    """LDXI X1, 7  then HALT — smallest non-trivial program."""
    return long_instr(F_LDXI, 1, 0, 7) + HALT


# ── isinstance checks ──────────────────────────────────────────────────────────

def test_is_simulator():
    sim = CDC6600Simulator()
    assert isinstance(sim, Simulator)


def test_is_concrete_simulator():
    sim = CDC6600Simulator()
    assert isinstance(sim, CDC6600Simulator)


# ── reset() ───────────────────────────────────────────────────────────────────

def test_reset_clears_registers():
    sim = CDC6600Simulator()
    sim.execute(simple_prog())
    sim.reset()
    s = sim.get_state()
    assert s.x == tuple(0 for _ in range(8))
    assert s.a == tuple(0 for _ in range(8))
    assert s.b == tuple(0 for _ in range(8))


def test_reset_clears_memory():
    sim = CDC6600Simulator()
    sim.execute(simple_prog())
    sim.reset()
    s = sim.get_state()
    assert all(w == 0 for w in s.memory)


def test_reset_sets_p_zero():
    sim = CDC6600Simulator()
    sim.execute(simple_prog())
    sim.reset()
    assert sim.get_state().p == 0


def test_reset_clears_halted():
    sim = CDC6600Simulator()
    sim.execute(simple_prog())
    assert sim.get_state().halted
    sim.reset()
    assert not sim.get_state().halted


# ── load() ────────────────────────────────────────────────────────────────────

def test_load_resets_state():
    sim = CDC6600Simulator()
    sim.execute(simple_prog())
    sim.load(simple_prog())
    s = sim.get_state()
    assert s.p == 0
    assert not s.halted


def test_load_places_bytes_in_memory():
    sim = CDC6600Simulator()
    sim.load(simple_prog())
    s = sim.get_state()
    # The word 0 should be non-zero (contains the LDXI instruction + HALT parcel)
    assert s.memory[0] != 0


# ── step() ────────────────────────────────────────────────────────────────────

def test_step_returns_step_trace():
    sim = CDC6600Simulator()
    sim.load(simple_prog())
    trace = sim.step()
    assert isinstance(trace, StepTrace)


def test_step_trace_has_pc_before():
    sim = CDC6600Simulator()
    sim.load(simple_prog())
    trace = sim.step()
    assert trace.pc_before == 0


def test_step_trace_has_pc_after():
    sim = CDC6600Simulator()
    sim.load(simple_prog())
    trace = sim.step()
    # LDXI is a 30-bit (2-parcel) instruction → P advances by 2
    assert trace.pc_after == 2


def test_step_trace_has_mnemonic():
    sim = CDC6600Simulator()
    sim.load(simple_prog())
    trace = sim.step()
    assert "LDXI" in trace.mnemonic


def test_step_trace_description_not_empty():
    sim = CDC6600Simulator()
    sim.load(simple_prog())
    trace = sim.step()
    assert trace.description


def test_step_when_halted_returns_halt_trace():
    sim = CDC6600Simulator()
    sim.execute(simple_prog())
    trace = sim.step()
    assert "HALT" in trace.mnemonic
    assert trace.pc_before == trace.pc_after


def test_step_advances_p():
    sim = CDC6600Simulator()
    sim.load(simple_prog())
    sim.step()   # LDXI (2 parcels)
    assert sim.get_state().p == 2


# ── execute() ─────────────────────────────────────────────────────────────────

def test_execute_returns_execution_result():
    sim = CDC6600Simulator()
    result = sim.execute(simple_prog())
    assert isinstance(result, ExecutionResult)


def test_execute_halted_true():
    sim = CDC6600Simulator()
    result = sim.execute(simple_prog())
    assert result.halted


def test_execute_ok():
    sim = CDC6600Simulator()
    result = sim.execute(simple_prog())
    assert result.ok


def test_execute_steps_positive():
    sim = CDC6600Simulator()
    result = sim.execute(simple_prog())
    assert result.steps > 0


def test_execute_final_state_is_cdc6600state():
    sim = CDC6600Simulator()
    result = sim.execute(simple_prog())
    assert isinstance(result.final_state, CDC6600State)


def test_execute_x1_set():
    sim = CDC6600Simulator()
    result = sim.execute(simple_prog())
    assert result.final_state.x1 == 7


def test_execute_traces_list():
    sim = CDC6600Simulator()
    result = sim.execute(simple_prog())
    assert isinstance(result.traces, list)
    assert len(result.traces) > 0


def test_execute_no_error_on_success():
    sim = CDC6600Simulator()
    result = sim.execute(simple_prog())
    assert result.error is None


def test_execute_max_steps_exceeded():
    sim = CDC6600Simulator()
    from cdc6600_simulator.simulator import F_JMP
    inf_loop = long_instr(F_JMP, 0, 0, 0)  # jump to parcel 0 — infinite loop
    result = sim.execute(inf_loop, max_steps=10)
    assert not result.ok
    assert result.error is not None


# ── get_state() ───────────────────────────────────────────────────────────────

def test_get_state_returns_cdc6600state():
    sim = CDC6600Simulator()
    assert isinstance(sim.get_state(), CDC6600State)


def test_get_state_snapshot_not_mutated():
    sim = CDC6600Simulator()
    sim.load(simple_prog())
    snap = sim.get_state()
    p_before = snap.p
    sim.step()
    # The snapshot is frozen — its p is unchanged
    assert snap.p == p_before


def test_get_state_is_frozen():
    sim = CDC6600Simulator()
    s = sim.get_state()
    raised = False
    try:
        s.p = 99  # type: ignore[misc]
    except (AttributeError, TypeError):
        raised = True
    assert raised, "Frozen dataclass should not allow attribute assignment"
