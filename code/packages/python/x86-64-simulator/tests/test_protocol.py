"""Tests for SIM00 protocol compliance."""

from x86_64_simulator import MEM_SIZE, X86_64Simulator, X86_64State


def test_reset_zeroes_all_registers():
    sim = X86_64Simulator()
    # Load something to dirty state
    sim._cpu.gpr[0] = 0xDEADBEEF
    sim._cpu.rflags = 0xFF
    sim.reset()
    assert all(sim._cpu.gpr[i] == 0 for i in range(16) if i != 4)  # RSP=0xFFF8
    assert sim._cpu.rflags == 0
    assert sim._cpu.pc == 0
    assert not sim._cpu.halted


def test_reset_sets_rsp():
    sim = X86_64Simulator()
    sim.reset()
    assert sim._cpu.gpr[4] == 0xFFF8  # RSP = top-of-memory - 8


def test_reset_zeroes_memory():
    sim = X86_64Simulator()
    sim._cpu.memory[100] = 0xAB
    sim.reset()
    assert sim._cpu.memory[100] == 0


def test_load_copies_program():
    sim = X86_64Simulator()
    prog = bytes([0x48, 0xB8, 1, 0, 0, 0, 0, 0, 0, 0, 0xF4])
    sim.load(prog)
    assert sim._cpu.memory[0] == 0x48
    assert sim._cpu.memory[1] == 0xB8
    assert sim._cpu.pc == 0
    assert not sim._cpu.halted


def test_get_state_returns_frozen_snapshot():
    sim = X86_64Simulator()
    state = sim.get_state()
    assert isinstance(state, X86_64State)
    assert state.pc == 0
    assert len(state.gpr) == 16
    assert len(state.memory) == MEM_SIZE
    assert not state.halted


def test_execute_returns_state():
    sim = X86_64Simulator()
    prog = bytes([0xF4])  # HLT
    state = sim.execute(prog)
    assert isinstance(state, X86_64State)
    assert state.halted


def test_step_returns_step_trace():
    from x86_64_simulator import StepTrace
    sim = X86_64Simulator()
    sim.load(bytes([0x90, 0xF4]))  # NOP, HLT
    trace = sim.step()
    assert isinstance(trace, StepTrace)
    assert trace.pc_before == 0
    assert not trace.halted


def test_step_on_hlt_sets_halted():
    sim = X86_64Simulator()
    sim.load(bytes([0xF4]))
    trace = sim.step()
    assert trace.halted
    assert sim._cpu.halted


def test_step_after_halt_does_not_advance_pc():
    sim = X86_64Simulator()
    sim.load(bytes([0xF4]))
    sim.step()   # execute HLT
    pc_after_hlt = sim._cpu.pc
    sim.step()   # step again — should stay put
    assert sim._cpu.pc == pc_after_hlt


def test_execute_stops_at_max_steps():
    sim = X86_64Simulator()
    prog = bytes([0x90] * 1000 + [0xF4])  # 1000 NOPs then HLT
    state = sim.execute(prog, max_steps=5)
    assert not state.halted  # didn't reach HLT in 5 steps


def test_iop_stubs_do_not_crash():
    sim = X86_64Simulator()
    sim.set_input_port(0, 0)
    assert sim.get_output_port(0) == 0
    sim.interrupt(0)
    sim.nmi()
