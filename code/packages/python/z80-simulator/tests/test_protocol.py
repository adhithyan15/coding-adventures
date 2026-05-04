"""Tests for the SIM00 Simulator protocol implementation.

Covers: reset, load, step, execute, get_state,
        set_input_port, get_output_port, halted behaviour.
"""

import pytest

from z80_simulator import Z80Simulator, Z80State

# ── Construction ──────────────────────────────────────────────────────────────

class TestConstruction:
    def test_initial_state_registers_zero(self):
        sim = Z80Simulator()
        s = sim.get_state()
        assert s.a == 0
        assert s.b == 0
        assert s.c == 0
        assert s.d == 0
        assert s.e == 0
        assert s.h == 0
        assert s.l == 0

    def test_initial_state_special_regs_zero(self):
        sim = Z80Simulator()
        s = sim.get_state()
        assert s.ix == 0
        assert s.iy == 0
        assert s.sp == 0
        assert s.pc == 0
        assert s.i == 0
        assert s.r == 0

    def test_initial_flags_all_set(self):
        # Z80 power-on: F = 0xFF (all flags set)
        sim = Z80Simulator()
        s = sim.get_state()
        assert s.flag_s is True
        assert s.flag_z is True
        assert s.flag_h is True
        assert s.flag_pv is True
        assert s.flag_n is True
        assert s.flag_c is True

    def test_initial_interrupts_disabled(self):
        sim = Z80Simulator()
        s = sim.get_state()
        assert s.iff1 is False
        assert s.iff2 is False
        assert s.im == 0

    def test_initial_not_halted(self):
        sim = Z80Simulator()
        assert sim.get_state().halted is False

    def test_memory_zeroed(self):
        sim = Z80Simulator()
        s = sim.get_state()
        assert all(b == 0 for b in s.memory)


# ── reset ─────────────────────────────────────────────────────────────────────

class TestReset:
    def test_reset_clears_registers(self):
        sim = Z80Simulator()
        # Dirty the state
        sim.execute(bytes([0x3E, 0x42, 0x76]))   # LD A,0x42; HALT
        s1 = sim.get_state()
        assert s1.a == 0x42
        sim.reset()
        s2 = sim.get_state()
        assert s2.a == 0
        assert s2.pc == 0
        assert s2.halted is False

    def test_reset_zeros_memory(self):
        sim = Z80Simulator()
        sim.load(bytes([0xAA, 0xBB]), 0x1000)
        sim.reset()
        s = sim.get_state()
        assert s.memory[0x1000] == 0
        assert s.memory[0x1001] == 0


# ── load ──────────────────────────────────────────────────────────────────────

class TestLoad:
    def test_load_default_origin(self):
        sim = Z80Simulator()
        prog = bytes([0x3E, 0x42, 0x76])
        sim.load(prog)
        s = sim.get_state()
        assert s.memory[0] == 0x3E
        assert s.memory[1] == 0x42
        assert s.memory[2] == 0x76
        assert s.pc == 0

    def test_load_custom_origin(self):
        sim = Z80Simulator()
        sim.load(bytes([0x01, 0x02, 0x03]), origin=0x2000)
        s = sim.get_state()
        assert s.memory[0x2000] == 0x01
        assert s.pc == 0x2000

    def test_load_invalid_origin(self):
        sim = Z80Simulator()
        with pytest.raises(ValueError):
            sim.load(bytes([0x76]), origin=0x10000)

    def test_load_clears_halted(self):
        sim = Z80Simulator()
        sim.execute(bytes([0x76]))
        assert sim.get_state().halted is True
        sim.load(bytes([0x76]))
        assert sim.get_state().halted is False


# ── step ──────────────────────────────────────────────────────────────────────

class TestStep:
    def test_step_advances_pc(self):
        sim = Z80Simulator()
        sim.load(bytes([0x00, 0x00, 0x76]))   # NOP NOP HALT
        trace = sim.step()
        assert trace.pc_before == 0
        assert trace.pc_after == 1
        assert trace.mnemonic == "NOP"

    def test_step_raises_when_halted(self):
        sim = Z80Simulator()
        sim.execute(bytes([0x76]))
        with pytest.raises(RuntimeError):
            sim.step()

    def test_step_returns_step_trace(self):
        sim = Z80Simulator()
        sim.load(bytes([0x3E, 0x05, 0x76]))   # LD A, 5
        trace = sim.step()
        assert trace.pc_before == 0
        assert trace.pc_after == 2


# ── execute ───────────────────────────────────────────────────────────────────

class TestExecute:
    def test_execute_runs_to_halt(self):
        sim = Z80Simulator()
        result = sim.execute(bytes([
            0x3E, 0x0A,   # LD A, 10
            0xC6, 0x05,   # ADD A, 5
            0x76,         # HALT
        ]))
        assert result.halted is True
        assert result.final_state.a == 15

    def test_execute_respects_max_steps(self):
        sim = Z80Simulator()
        # Infinite NOP loop (JR -2 loops forever)
        result = sim.execute(bytes([0x18, 0xFE]), max_steps=50)
        assert result.halted is False
        assert result.steps == 50

    def test_execute_returns_traces(self):
        sim = Z80Simulator()
        result = sim.execute(bytes([0x00, 0x00, 0x76]))   # NOP NOP HALT
        assert len(result.traces) == 3

    def test_execute_preserves_input_ports(self):
        sim = Z80Simulator()
        sim.set_input_port(0x10, 0xAB)
        sim.execute(bytes([0xDB, 0x10, 0x76]))   # IN A,(0x10); HALT
        assert sim.get_state().a == 0xAB

    def test_execute_custom_origin(self):
        sim = Z80Simulator()
        result = sim.execute(
            bytes([0x3E, 0x07, 0x76]),   # LD A,7; HALT
            origin=0x4000
        )
        assert result.final_state.a == 7
        assert result.final_state.pc == 0x4003


# ── get_state ─────────────────────────────────────────────────────────────────

class TestGetState:
    def test_get_state_returns_z80state(self):
        sim = Z80Simulator()
        s = sim.get_state()
        assert isinstance(s, Z80State)

    def test_state_is_frozen(self):
        sim = Z80Simulator()
        s = sim.get_state()
        with pytest.raises(Exception):   # frozen dataclass raises FrozenInstanceError
            s.a = 99  # type: ignore[misc]

    def test_state_memory_is_tuple(self):
        sim = Z80Simulator()
        assert isinstance(sim.get_state().memory, tuple)
        assert len(sim.get_state().memory) == 65536


# ── I/O ports ─────────────────────────────────────────────────────────────────

class TestIOPorts:
    def test_set_and_get_input_port(self):
        sim = Z80Simulator()
        sim.set_input_port(0, 0x55)
        sim.execute(bytes([0xDB, 0x00, 0x76]))   # IN A,(0)
        assert sim.get_state().a == 0x55

    def test_output_port_written(self):
        sim = Z80Simulator()
        sim.execute(bytes([0x3E, 0xAB, 0xD3, 0x07, 0x76]))   # LD A,0xAB; OUT(7),A
        assert sim.get_output_port(7) == 0xAB

    def test_port_out_of_range_raises(self):
        sim = Z80Simulator()
        with pytest.raises(ValueError):
            sim.set_input_port(256, 0)

    def test_port_value_out_of_range_raises(self):
        sim = Z80Simulator()
        with pytest.raises(ValueError):
            sim.set_input_port(0, 256)

    def test_get_output_port_out_of_range(self):
        sim = Z80Simulator()
        with pytest.raises(ValueError):
            sim.get_output_port(256)


# ── Z80State helpers ──────────────────────────────────────────────────────────

class TestZ80StateHelpers:
    def test_f_byte_packs_flags(self):
        sim = Z80Simulator()
        sim.execute(bytes([
            0xAF,   # XOR A → A=0, Z=1, N=0, C=0, H=0, PV (parity) = even = 1
            0x76,
        ]))
        s = sim.get_state()
        # XOR A: Z=1, S=0, H=0, N=0, C=0, PV = parity(0) = True
        # f_byte = 0b01000100 = 0x44
        assert s.f_byte() == 0x44

    def test_bc_property(self):
        sim = Z80Simulator()
        sim.execute(bytes([0x01, 0x34, 0x12, 0x76]))   # LD BC, 0x1234
        s = sim.get_state()
        assert s.bc == 0x1234

    def test_de_property(self):
        sim = Z80Simulator()
        sim.execute(bytes([0x11, 0x78, 0x56, 0x76]))   # LD DE, 0x5678
        s = sim.get_state()
        assert s.de == 0x5678

    def test_hl_property(self):
        sim = Z80Simulator()
        sim.execute(bytes([0x21, 0xBC, 0x9A, 0x76]))   # LD HL, 0x9ABC
        s = sim.get_state()
        assert s.hl == 0x9ABC
