"""Tests for SIM00 protocol compliance of the AArch64 simulator."""

from aarch64_simulator import (
    HALT,
    MEM_SIZE,
    AArch64Simulator,
    dp_imm,
    make_initial_state,
)

# ── reset() ───────────────────────────────────────────────────────────────────


def test_reset_zeros_all_registers():
    """After reset, all GPRs, SP, PC, NZCV, and memory are zero."""
    sim = AArch64Simulator()
    sim.load(dp_imm(1, 0, 0, 42, 0, 0, 1) + HALT)  # ADD X1, X0, #42
    sim.execute(dp_imm(1, 0, 0, 42, 0, 0, 1) + HALT)
    sim.reset()
    s = sim.get_state()
    assert s.pc == 0
    assert all(r == 0 for r in s.gpr)
    assert s.sp == 0
    assert s.nzcv == 0
    assert all(b == 0 for b in s.memory)
    assert not s.halted


def test_reset_memory_size():
    """Memory tuple has exactly MEM_SIZE bytes after reset."""
    sim = AArch64Simulator()
    sim.reset()
    assert len(sim.get_state().memory) == MEM_SIZE


# ── load() ────────────────────────────────────────────────────────────────────


def test_load_copies_program_bytes():
    """load() copies the program bytes starting at address 0."""
    sim = AArch64Simulator()
    prog = b"\x01\x02\x03\x04"
    sim.load(prog)
    s = sim.get_state()
    assert s.memory[0] == 0x01
    assert s.memory[1] == 0x02
    assert s.memory[2] == 0x03
    assert s.memory[3] == 0x04


def test_load_resets_before_copy():
    """load() resets state before copying so leftover state is cleared."""
    sim = AArch64Simulator()
    sim.execute(dp_imm(1, 0, 0, 99, 0, 0, 1) + HALT)
    assert sim.get_state().x1 == 99
    sim.load(HALT)
    assert sim.get_state().x1 == 0


def test_load_preserves_unwritten_memory_as_zero():
    """Bytes after the program are zeroed (from reset)."""
    sim = AArch64Simulator()
    sim.load(b"\xAB\xCD" + HALT)
    s = sim.get_state()
    assert s.memory[0] == 0xAB
    assert s.memory[1] == 0xCD
    assert s.memory[4] == 0x00   # HALT
    assert s.memory[100] == 0x00


# ── step() ────────────────────────────────────────────────────────────────────


def test_step_returns_step_trace():
    """step() returns a StepTrace with pc_before, pc_after, mnemonic, ok."""
    from simulator_protocol import StepTrace
    sim = AArch64Simulator()
    sim.load(dp_imm(1, 0, 0, 1, 0, 0, 0) + HALT)  # ADD X0, X0, #1
    trace = sim.step()
    assert isinstance(trace, StepTrace)
    assert not trace.mnemonic.startswith("ERROR:")
    assert trace.pc_before == 0
    assert trace.pc_after == 4
    assert "ADD" in trace.mnemonic


def test_step_advances_pc_by_4():
    """Each step advances PC by 4."""
    sim = AArch64Simulator()
    sim.load(dp_imm(1, 0, 0, 1, 0, 0, 0) + dp_imm(1, 0, 0, 2, 0, 0, 0) + HALT)
    sim.step()
    assert sim.get_state().pc == 4
    sim.step()
    assert sim.get_state().pc == 8


def test_step_on_halt_does_not_advance():
    """step() on a halted simulator returns HALT trace and stays halted."""
    sim = AArch64Simulator()
    sim.execute(HALT)
    trace = sim.step()
    assert "HALT" in trace.mnemonic
    assert trace.pc_before == trace.pc_after
    assert not trace.mnemonic.startswith("ERROR:")


def test_step_on_halt_stays_halted():
    """Stepping a halted simulator does not un-halt it."""
    sim = AArch64Simulator()
    sim.execute(HALT)
    assert sim.get_state().halted
    sim.step()
    assert sim.get_state().halted


# ── execute() ─────────────────────────────────────────────────────────────────


def test_execute_returns_execution_result():
    """execute() returns an ExecutionResult."""
    from simulator_protocol import ExecutionResult
    sim = AArch64Simulator()
    result = sim.execute(HALT)
    assert isinstance(result, ExecutionResult)


def test_execute_halts_on_halt_word():
    """execute() halts when it encounters 0x00000000."""
    sim = AArch64Simulator()
    result = sim.execute(HALT)
    assert result.halted
    assert result.ok


def test_execute_runs_to_completion():
    """execute() steps through a simple program."""
    sim = AArch64Simulator()
    prog = dp_imm(1, 0, 0, 7, 0, 0, 1) + HALT   # ADD X1, X0, #7
    result = sim.execute(prog)
    assert result.halted
    assert result.final_state.x1 == 7


def test_execute_resets_between_calls():
    """Calling execute() twice resets state between them."""
    sim = AArch64Simulator()
    sim.execute(dp_imm(1, 0, 0, 99, 0, 0, 1) + HALT)
    assert sim.get_state().x1 == 99
    sim.execute(HALT)
    assert sim.get_state().x1 == 0


def test_execute_max_steps():
    """execute() stops after max_steps even without a HALT."""
    from aarch64_simulator import branch_imm
    sim = AArch64Simulator()
    loop = branch_imm(0, 0)   # B #0  — branches to itself (infinite loop)
    result = sim.execute(loop, max_steps=10)
    assert result.steps == 10
    assert not result.halted


# ── get_state() ───────────────────────────────────────────────────────────────


def test_get_state_returns_frozen():
    """get_state() returns a frozen dataclass (writes to fields raise errors)."""
    sim = AArch64Simulator()
    s = sim.get_state()
    raised = False
    try:
        s.pc = 4  # type: ignore[misc]
    except (AttributeError, TypeError):
        raised = True
    assert raised


def test_get_state_gpr_is_tuple():
    """gpr field is a tuple, not a list."""
    sim = AArch64Simulator()
    s = sim.get_state()
    assert isinstance(s.gpr, tuple)
    assert len(s.gpr) == 32


def test_get_state_memory_is_tuple():
    """memory field is a tuple of MEM_SIZE ints."""
    sim = AArch64Simulator()
    s = sim.get_state()
    assert isinstance(s.memory, tuple)
    assert len(s.memory) == MEM_SIZE


def test_snapshot_independence():
    """Snapshots taken at different times are independent."""
    sim = AArch64Simulator()
    sim.load(dp_imm(1, 0, 0, 1, 0, 0, 1) + HALT)  # ADD X1, X0, #1
    snap1 = sim.get_state()
    sim.step()
    snap2 = sim.get_state()
    assert snap1.pc == 0
    assert snap2.pc == 4
    assert snap1.x1 == 0
    assert snap2.x1 == 1


# ── make_initial_state() ──────────────────────────────────────────────────────


def test_make_initial_state_is_all_zero():
    """make_initial_state() returns an all-zero state."""
    s = make_initial_state()
    assert s.pc == 0
    assert all(r == 0 for r in s.gpr)
    assert s.sp == 0
    assert s.nzcv == 0
    assert len(s.memory) == MEM_SIZE
    assert all(b == 0 for b in s.memory)
    assert not s.halted
