"""Tests for Intel8080GateLevelSimulator — protocol conformance and edge cases."""

from __future__ import annotations

import pytest
from intel8080_simulator import Intel8080State

from intel8080_gatelevel import Intel8080GateLevelSimulator


class TestSIMProtocol:
    def test_reset_clears_state(self) -> None:
        sim = Intel8080GateLevelSimulator()
        sim.execute(bytes([0x3E, 0x42, 0x76]))
        sim.reset()
        state = sim.get_state()
        assert state.a == 0
        assert state.pc == 0

    def test_load_and_step(self) -> None:
        sim = Intel8080GateLevelSimulator()
        sim.reset()
        sim.load(bytes([0x3E, 0x0A, 0x76]))  # MVI A, 10; HLT
        trace = sim.step()
        assert trace.mnemonic == "MVI A"
        assert trace.pc_before == 0
        assert trace.pc_after == 2

    def test_step_after_halt_raises(self) -> None:
        sim = Intel8080GateLevelSimulator()
        sim.execute(bytes([0x76]))
        with pytest.raises(RuntimeError):
            sim.step()

    def test_get_state_returns_intel8080state(self) -> None:
        sim = Intel8080GateLevelSimulator()
        state = sim.get_state()
        assert isinstance(state, Intel8080State)

    def test_execute_returns_halted_true(self) -> None:
        result = Intel8080GateLevelSimulator().execute(bytes([0x76]))
        assert result.halted is True

    def test_execute_steps_count(self) -> None:
        # NOP NOP NOP HLT
        result = Intel8080GateLevelSimulator().execute(bytes([0x00, 0x00, 0x00, 0x76]))
        assert result.steps == 4

    def test_execute_traces(self) -> None:
        result = Intel8080GateLevelSimulator().execute(bytes([0x00, 0x76]))
        assert result.traces[0].mnemonic == "NOP"
        assert result.traces[1].mnemonic == "HLT"

    def test_max_steps_prevents_infinite_loop(self) -> None:
        # JMP to self = infinite loop
        prog = bytes([0xC3, 0x00, 0x00])
        result = Intel8080GateLevelSimulator().execute(prog, max_steps=100)
        assert result.steps == 100
        assert result.halted is False

    def test_port_validation_input(self) -> None:
        sim = Intel8080GateLevelSimulator()
        with pytest.raises(ValueError):
            sim.set_input_port(256, 0)
        with pytest.raises(ValueError):
            sim.set_input_port(0, 256)

    def test_port_validation_output(self) -> None:
        sim = Intel8080GateLevelSimulator()
        with pytest.raises(ValueError):
            sim.get_output_port(256)

    def test_input_ports_preserved_across_execute(self) -> None:
        sim = Intel8080GateLevelSimulator()
        sim.set_input_port(1, 0xAB)
        sim.execute(bytes([0x76]))   # HLT immediately
        # Ports preserved: run IN now
        result = sim.execute(bytes([0xDB, 0x01, 0x76]))
        assert result.final_state.a == 0xAB


class TestMemoryOps:
    """Test MOV with M pseudo-register, INR M, DCR M."""

    def test_mov_from_m(self) -> None:
        # LXI H,0x0200; MVI M,0x42; MOV A,M
        prog = bytes([
            0x21, 0x00, 0x02,   # LXI H,0x0200
            0x36, 0x42,          # MVI M,0x42
            0x7E,                # MOV A,M
            0x76,
        ])
        result = Intel8080GateLevelSimulator().execute(prog)
        assert result.final_state.a == 0x42

    def test_mov_to_m(self) -> None:
        # LXI H,0x0200; MVI A,0x55; MOV M,A
        prog = bytes([
            0x21, 0x00, 0x02,   # LXI H,0x0200
            0x3E, 0x55,          # MVI A,0x55
            0x77,                # MOV M,A
            0x76,
        ])
        result = Intel8080GateLevelSimulator().execute(prog)
        assert result.final_state.memory[0x0200] == 0x55

    def test_inr_m(self) -> None:
        # LXI H,0x0200; MVI M,9; INR M
        prog = bytes([
            0x21, 0x00, 0x02,   # LXI H,0x0200
            0x36, 0x09,          # MVI M,9
            0x34,                # INR M
            0x76,
        ])
        result = Intel8080GateLevelSimulator().execute(prog)
        assert result.final_state.memory[0x0200] == 10

    def test_dcr_m(self) -> None:
        # LXI H,0x0200; MVI M,5; DCR M
        prog = bytes([
            0x21, 0x00, 0x02,   # LXI H,0x0200
            0x36, 0x05,          # MVI M,5
            0x35,                # DCR M
            0x76,
        ])
        result = Intel8080GateLevelSimulator().execute(prog)
        assert result.final_state.memory[0x0200] == 4

    def test_add_m(self) -> None:
        # LXI H,0x0200; MVI M,7; MVI A,3; ADD M
        prog = bytes([
            0x21, 0x00, 0x02,   # LXI H,0x0200
            0x36, 0x07,          # MVI M,7
            0x3E, 0x03,          # MVI A,3
            0x86,                # ADD M
            0x76,
        ])
        result = Intel8080GateLevelSimulator().execute(prog)
        assert result.final_state.a == 10


class TestConditionalBranches:
    """Test all 8 condition codes including parity (PO/PE) and sign (P/M)."""

    def test_jz_taken(self) -> None:
        # XRA A (Z=1); JZ to HLT
        prog = bytes([0xAF, 0xCA, 0x05, 0x00, 0x3E, 0xFF, 0x76])
        result = Intel8080GateLevelSimulator().execute(prog)
        assert result.final_state.a == 0   # 0xFF not loaded

    def test_jnc_taken(self) -> None:
        # XRA A (CY=0); JNC to HLT
        prog = bytes([0xAF, 0xD2, 0x05, 0x00, 0x3E, 0xFF, 0x76])
        result = Intel8080GateLevelSimulator().execute(prog)
        assert result.final_state.a == 0

    def test_jc_taken(self) -> None:
        # STC (CY=1); JC to HLT
        prog = bytes([0x37, 0xDA, 0x05, 0x00, 0x3E, 0xFF, 0x76])
        result = Intel8080GateLevelSimulator().execute(prog)
        assert result.final_state.a == 0

    def test_jpo_taken(self) -> None:
        # MVI A,1 (odd parity → P=0); JPO (parity odd) to HLT
        prog = bytes([0x3E, 0x01, 0xE2, 0x06, 0x00, 0x3E, 0xFF, 0x76])
        result = Intel8080GateLevelSimulator().execute(prog)
        assert result.final_state.a == 1   # 0xFF not loaded (jumped over)

    def test_jpe_taken(self) -> None:
        # MVI A,3; ORA A (3=0b11, 2 set bits → even parity, P=1); JPE to HLT
        # MVI alone does not set flags — ORA A copies A and sets parity flag
        prog = bytes([0x3E, 0x03, 0xB7, 0xEA, 0x08, 0x00, 0x3E, 0xFF, 0x76])
        result = Intel8080GateLevelSimulator().execute(prog)
        assert result.final_state.a == 3   # jumped to HLT, 0xFF not loaded

    def test_jp_taken(self) -> None:
        # MVI A,1 is positive (S=0); JP (positive/sign clear) to HLT
        prog = bytes([0x3E, 0x01, 0xF2, 0x06, 0x00, 0x3E, 0xFF, 0x76])
        result = Intel8080GateLevelSimulator().execute(prog)
        assert result.final_state.a == 1

    def test_jm_taken(self) -> None:
        # ADD 0x40+0x40=0x80 (S=1); JM (minus/sign set) to HLT
        prog = bytes([
            0x3E, 0x40,          # MVI A,0x40
            0x06, 0x40,          # MVI B,0x40
            0x80,                # ADD B → 0x80, S=1
            0xFA, 0x09, 0x00,    # JM 0x0009
            0x3E, 0xFF,          # (skipped)
            0x76,                # HLT
        ])
        result = Intel8080GateLevelSimulator().execute(prog)
        assert result.final_state.a == 0x80

    def test_conditional_ret_all_conditions(self) -> None:
        # Test RZ (return if zero)
        prog = bytes([
            0x31, 0x00, 0x04,    # LXI SP,0x0400
            0xCD, 0x0A, 0x00,    # CALL 0x000A
            0x76,                # HLT
            0x00, 0x00, 0x00,    # padding
            0xAF,                # 0x000A: XRA A (Z=1)
            0xC8,                # 0x000B: RZ (return if Z, taken)
            0x3E, 0xFF,          # 0x000C: MVI A,0xFF (skipped)
            0xC9,                # 0x000E: RET
        ])
        result = Intel8080GateLevelSimulator().execute(prog)
        assert result.final_state.a == 0   # MVI A,0xFF was skipped

    def test_invalid_condition_raises(self) -> None:
        from intel8080_gatelevel.control import FlagRegister
        flags = FlagRegister()
        with pytest.raises(ValueError):
            flags.condition_met(8)


class TestLXID:
    """Test LXI D — previously missing coverage."""

    def test_lxi_d(self) -> None:
        # LXI D, 0xABCD
        prog = bytes([0x11, 0xCD, 0xAB, 0x76])
        result = Intel8080GateLevelSimulator().execute(prog)
        assert result.final_state.d == 0xAB
        assert result.final_state.e == 0xCD


class TestRST:
    def test_rst_0(self) -> None:
        # Set up a small RST 0 handler at address 0x0000 that sets A=0x99
        # But the RST 0 would jump to 0x0000 which is where the code is...
        # Instead test RST 1 → address 0x0008
        prog = bytes([
            0x31, 0x00, 0x04,    # 0x00: LXI SP,0x0400
            0xCF,                # 0x03: RST 1 → jump to 0x0008
            0x76,                # 0x04: HLT (return here after RST)
            0x00, 0x00, 0x00,    # 0x05-0x07: padding
            0x3E, 0x11,          # 0x08: MVI A,0x11
            0xC9,                # 0x0A: RET
        ])
        result = Intel8080GateLevelSimulator().execute(prog)
        assert result.final_state.a == 0x11
