"""Tests for SIM00 protocol compliance — ARMv7-A simulator."""

from armv7a_simulator import CPSR_T, MEM_SIZE, SP, ARMv7ASimulator, ARMv7AState


def test_reset_zeroes_registers():
    sim = ARMv7ASimulator()
    sim._cpu.gpr[0] = 0xDEADBEEF
    sim._cpu.gpr[7] = 0x12345678
    sim.reset()
    for i in range(16):
        if i != SP:
            assert sim._cpu.gpr[i] == 0, f"R{i} not zeroed after reset"


def test_reset_sets_sp():
    sim = ARMv7ASimulator()
    sim.reset()
    assert sim._cpu.gpr[SP] == 0xFFF8


def test_reset_sets_thumb_bit():
    sim = ARMv7ASimulator()
    sim.reset()
    assert (sim._cpu.cpsr >> CPSR_T) & 1 == 1, "CPSR.T should be 1 after reset"


def test_reset_clears_flags():
    sim = ARMv7ASimulator()
    sim._cpu.cpsr = 0xF800_0000   # all flags set
    sim.reset()
    # Only T bit should be set, not N/Z/C/V
    assert not sim.get_state().n
    assert not sim.get_state().z
    assert not sim.get_state().c
    assert not sim.get_state().v


def test_reset_zeroes_memory():
    sim = ARMv7ASimulator()
    sim._cpu.memory[42] = 0xFF
    sim.reset()
    assert sim._cpu.memory[42] == 0


def test_load_copies_program():
    sim = ARMv7ASimulator()
    prog = bytes([0x00, 0x20, 0x00, 0x00])  # MOV R0, #0 + halt
    sim.load(prog)
    assert sim._cpu.memory[0] == 0x00
    assert sim._cpu.memory[1] == 0x20
    assert sim._cpu.pc == 0
    assert not sim._cpu.halted


def test_get_state_returns_frozen_snapshot():
    sim = ARMv7ASimulator()
    state = sim.get_state()
    assert isinstance(state, ARMv7AState)
    assert state.pc == 0
    assert len(state.gpr) == 16
    assert len(state.memory) == MEM_SIZE
    assert not state.halted


def test_execute_returns_state():
    sim = ARMv7ASimulator()
    state = sim.execute(b"\x00\x00")  # halt immediately
    assert isinstance(state, ARMv7AState)
    assert state.halted


def test_step_returns_step_trace():
    from armv7a_simulator import ARMv7ASimulator, StepTrace

    sim = ARMv7ASimulator()
    # MOV R0, #42 (0x2A00 → little-endian: 0x00, 0x2A) then halt
    sim.load(bytes([0x2A, 0x20, 0x00, 0x00]))
    trace = sim.step()
    assert isinstance(trace, StepTrace)
    assert trace.pc_before == 0
    assert not trace.halted


def test_step_on_halt_sets_halted():
    sim = ARMv7ASimulator()
    sim.load(bytes([0x00, 0x00]))  # halt immediately
    trace = sim.step()
    assert trace.halted
    assert sim._cpu.halted


def test_step_after_halt_does_not_advance_pc():
    sim = ARMv7ASimulator()
    sim.load(bytes([0x00, 0x00]))
    sim.step()
    pc_after = sim._cpu.pc
    sim.step()
    assert sim._cpu.pc == pc_after


def test_execute_stops_at_max_steps():
    sim = ARMv7ASimulator()
    # 1000 NOPs (each NOP = 0xBF00, little-endian: 0x00, 0xBF), then halt
    prog = bytes([0x00, 0xBF] * 500 + [0x00, 0x00])
    state = sim.execute(prog, max_steps=5)
    assert not state.halted  # didn't reach halt in 5 steps


def test_iop_stubs_do_not_crash():
    sim = ARMv7ASimulator()
    sim.set_input_port(0, 0)
    assert sim.get_output_port(0) == 0
    sim.interrupt(0)
    sim.nmi()
