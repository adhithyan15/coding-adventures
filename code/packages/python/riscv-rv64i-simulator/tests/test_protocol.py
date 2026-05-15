"""Tests for SIM00 protocol compliance — RV64I simulator."""

from riscv_rv64i_simulator import MEM_SIZE, SP, RV64ISimulator, RV64IState


def test_reset_zeroes_registers():
    sim = RV64ISimulator()
    sim._cpu.gpr[1] = 0xDEADBEEF
    sim._cpu.gpr[10] = 0x12345678
    sim.reset()
    for i in range(32):
        if i != SP:
            assert sim._cpu.gpr[i] == 0, f"x{i} not zeroed after reset"


def test_reset_sets_sp():
    sim = RV64ISimulator()
    sim.reset()
    assert sim._cpu.gpr[SP] == 0xFFF8


def test_reset_zeroes_memory():
    sim = RV64ISimulator()
    sim._cpu.memory[100] = 0xFF
    sim.reset()
    assert sim._cpu.memory[100] == 0


def test_reset_sets_pc_zero():
    sim = RV64ISimulator()
    sim._cpu.pc = 0x1234
    sim.reset()
    assert sim._cpu.pc == 0


def test_reset_clears_halted():
    sim = RV64ISimulator()
    sim._cpu.halted = True
    sim.reset()
    assert not sim._cpu.halted


def test_load_copies_program():
    sim = RV64ISimulator()
    prog = bytes([0x13, 0x05, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00])
    sim.load(prog)
    assert sim._cpu.memory[0] == 0x13
    assert sim._cpu.memory[1] == 0x05
    assert sim._cpu.pc == 0
    assert not sim._cpu.halted


def test_get_state_returns_frozen_snapshot():
    sim = RV64ISimulator()
    state = sim.get_state()
    assert isinstance(state, RV64IState)
    assert state.pc == 0
    assert len(state.gpr) == 32
    assert len(state.memory) == MEM_SIZE
    assert not state.halted


def test_x0_always_zero_in_state():
    sim = RV64ISimulator()
    sim._cpu.gpr[0] = 0xDEAD   # bypass write_reg directly
    state = sim.get_state()
    # x0 in gpr might be dirty but read_reg enforces 0; the state
    # snapshot captures raw gpr, but RV64IState.zero is always 0
    assert state.zero == 0


def test_execute_returns_state():
    sim = RV64ISimulator()
    state = sim.execute(b"\x00\x00\x00\x00")  # halt immediately
    assert isinstance(state, RV64IState)
    assert state.halted


def test_step_returns_step_trace():
    from riscv_rv64i_simulator import RV64ISimulator, StepTrace
    sim = RV64ISimulator()
    # ADDI x10, x0, 42 then halt
    prog = bytes([0x13, 0x05, 0xA0, 0x02,   # addi x10, x0, 42
                  0x00, 0x00, 0x00, 0x00])   # halt
    sim.load(prog)
    trace = sim.step()
    assert isinstance(trace, StepTrace)
    assert trace.pc_before == 0
    assert trace.pc_after == 4
    assert not trace.halted


def test_step_on_halt_sets_halted():
    sim = RV64ISimulator()
    sim.load(b"\x00\x00\x00\x00")
    trace = sim.step()
    assert trace.halted
    assert sim._cpu.halted


def test_step_after_halt_does_not_advance_pc():
    sim = RV64ISimulator()
    sim.load(b"\x00\x00\x00\x00")
    sim.step()
    pc_after = sim._cpu.pc
    sim.step()
    assert sim._cpu.pc == pc_after


def test_execute_stops_at_max_steps():
    sim = RV64ISimulator()
    # 500 NOPs then halt — should stop before reaching halt with max_steps=5
    nop = bytes([0x13, 0x00, 0x00, 0x00])   # addi x0, x0, 0 (NOP)
    prog = nop * 500 + b"\x00\x00\x00\x00"
    state = sim.execute(prog, max_steps=5)
    assert not state.halted


def test_iop_stubs_do_not_crash():
    sim = RV64ISimulator()
    sim.set_input_port(0, 0)
    assert sim.get_output_port(0) == 0
    sim.interrupt(0)
    sim.nmi()


def test_state_register_properties():
    sim = RV64ISimulator()
    sim._cpu.gpr[1]  = 0x100   # ra
    sim._cpu.gpr[2]  = 0x200   # sp
    sim._cpu.gpr[10] = 0x42    # a0
    state = sim.get_state()
    assert state.ra == 0x100
    assert state.sp == 0x200
    assert state.a0 == 0x42
