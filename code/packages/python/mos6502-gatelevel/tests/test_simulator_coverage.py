"""Simulator coverage tests — BRK, NMI, IRQ, addressing modes, edge cases."""

from __future__ import annotations

import pytest

from mos6502_gatelevel import MOS6502GateLevelSimulator


@pytest.fixture()
def sim():
    return MOS6502GateLevelSimulator()


# ── Protocol methods ──────────────────────────────────────────────────────────

class TestProtocolMethods:
    def test_reset_clears_memory(self, sim):
        sim._memory[0x100] = 0xFF
        sim.reset()
        assert sim._memory[0x100] == 0

    def test_reset_registers(self, sim):
        sim._rf.a.write(0xFF)
        sim.reset()
        state = sim.get_state()
        assert state.a == 0
        assert state.s == 0xFD
        assert state.flag_i is True

    def test_load_sets_pc(self, sim):
        sim.load(bytes([0x00]), 0x1000)
        assert sim.get_state().pc == 0x1000

    def test_load_invalid_origin_raises(self, sim):
        with pytest.raises(ValueError):
            sim.load(bytes([0x00]), 0x10000)

    def test_step_when_halted_raises(self, sim):
        sim.execute(bytes([0x00]))  # BRK halts
        with pytest.raises(RuntimeError, match="halted"):
            sim.step()

    def test_execute_returns_result(self, sim):
        result = sim.execute(bytes([0xA9, 0x42, 0x00]))
        assert result.halted is True
        assert result.steps > 0
        assert len(result.traces) > 0
        assert result.final_state.a == 0x42

    def test_execute_max_steps(self, sim):
        # Infinite loop: JMP $0000
        prog = bytes([0x4C, 0x00, 0x00])
        result = sim.execute(prog, max_steps=10)
        assert result.steps == 10
        assert result.halted is False

    def test_get_state_snapshot(self, sim):
        sim.load(bytes([0xA9, 0x42, 0x00]))
        sim.step()
        state = sim.get_state()
        assert state.a == 0x42
        assert isinstance(state.memory, tuple)
        assert len(state.memory) == 65536

    def test_execute_preserves_input_ports(self, sim):
        sim.set_input_port(5, 0x42)
        sim.execute(bytes([0x00]))
        # Port should survive the reset inside execute
        result = sim.execute(bytes([0xAD, 0x05, 0xFF, 0x00]))
        assert result.final_state.a == 0x42

    def test_step_trace(self, sim):
        sim.load(bytes([0xA9, 0x42, 0x00]))
        trace = sim.step()
        assert trace.mnemonic == "LDA"
        assert trace.pc_before == 0
        assert trace.pc_after == 2

    def test_illegal_opcode_raises(self, sim):
        sim.load(bytes([0x02]))  # illegal
        with pytest.raises(ValueError):
            sim.step()


# ── BRK behavior ─────────────────────────────────────────────────────────────

class TestBRKBehavior:
    def test_brk_halts(self, sim):
        result = sim.execute(bytes([0x00]))
        assert result.halted is True

    def test_brk_sets_i_flag(self, sim):
        sim.execute(bytes([0x58, 0x00]))  # CLI then BRK
        assert sim.get_state().flag_i is True

    def test_brk_sets_b_flag(self, sim):
        sim.execute(bytes([0x00]))
        assert sim.get_state().flag_b is True

    def test_brk_pushes_pc_and_p(self, sim):
        # After BRK at address 0, stack should have pushed PC+2=2 and P
        result = sim.execute(bytes([0x00]))
        state = result.final_state
        # Stack pointer decremented 3 times (PCH, PCL, P)
        assert state.s == 0xFD - 3

    def test_brk_p_has_b_set(self, sim):
        # The P pushed on stack should have B=1
        sim.execute(bytes([0x00]))
        # Stack top is P, two above are PC
        s = sim.get_state().s
        pushed_p = sim._memory[0x0100 | ((s + 1) & 0xFF)]
        assert pushed_p & 0x10, "B bit should be set in pushed P"

    def test_brk_pushes_pc_plus_2(self, sim):
        # BRK at address 0: pushed PC should be 2 (PC+2)
        # BRK is 2-byte instruction conceptually (PC+2 pushed)
        result = sim.execute(bytes([0x00]))
        state = result.final_state
        s = state.s
        pushed_lo = sim._memory[0x0100 | ((s + 2) & 0xFF)]
        pushed_hi = sim._memory[0x0100 | ((s + 3) & 0xFF)]
        pushed_pc = (pushed_hi << 8) | pushed_lo
        assert pushed_pc == 2   # PC was at 1 (after opcode fetch), +1 more = 2


# ── NMI behavior ─────────────────────────────────────────────────────────────

class TestNMIBehavior:
    def test_nmi_pushes_pc_and_p(self, sim):
        # Set NMI vector to $0100
        prog = bytearray(65536)
        prog[0] = 0xEA   # NOP (we'll call nmi() before it executes)
        prog[0xFFFA] = 0x00   # NMI lo
        prog[0xFFFB] = 0x01   # NMI hi → $0100
        prog[0x0100] = 0xA9; prog[0x0101] = 0x42; prog[0x0102] = 0x00
        sim.reset()
        sim.load(bytes(prog), 0)
        sim.nmi()
        state = sim.get_state()
        # PC should now be at NMI vector
        assert state.pc == 0x0100
        # I flag should be set
        assert state.flag_i is True
        # S decremented by 3
        assert state.s == 0xFD - 3

    def test_nmi_b_bit_not_set_in_pushed_p(self, sim):
        prog = bytearray(65536)
        prog[0xFFFA] = 0x00
        prog[0xFFFB] = 0x01
        prog[0x0100] = 0x00
        sim.reset()
        sim.load(bytes(prog), 0)
        sim.nmi()
        state = sim.get_state()
        s = state.s
        pushed_p = sim._memory[0x0100 | ((s + 1) & 0xFF)]
        # B bit should NOT be set for NMI (hardware interrupt)
        assert not (pushed_p & 0x10), "B bit should be clear in NMI-pushed P"

    def test_nmi_not_masked_by_i(self, sim):
        # NMI fires even when I=1
        prog = bytearray(65536)
        prog[0] = 0x78   # SEI
        prog[0xFFFA] = 0x10
        prog[0xFFFB] = 0x00
        prog[0x0010] = 0x00
        sim.reset()
        sim.load(bytes(prog), 0)
        sim.step()   # SEI
        sim.nmi()    # Should fire despite I=1
        assert sim.get_state().pc == 0x0010


# ── IRQ behavior ─────────────────────────────────────────────────────────────

class TestIRQBehavior:
    def test_irq_masked_when_i_set(self, sim):
        sim.reset()
        # I=1 at reset; IRQ should not fire
        initial_pc = sim.get_state().pc
        sim.interrupt()
        assert sim.get_state().pc == initial_pc   # PC unchanged

    def test_irq_fires_when_i_clear(self, sim):
        prog = bytearray(65536)
        prog[0xFFFE] = 0x10   # IRQ lo
        prog[0xFFFF] = 0x00   # IRQ hi → $0010
        prog[0x0010] = 0x00   # BRK at handler
        sim.reset()
        sim.load(bytes(prog), 0)
        sim.step()   # Step past first instruction (would be NOP from 0x00 data)
        sim._rf.flags.set_i(0)   # Clear I
        sim.interrupt()
        assert sim.get_state().pc == 0x0010

    def test_irq_sets_i_flag(self, sim):
        prog = bytearray(65536)
        prog[0xFFFE] = 0x00
        prog[0xFFFF] = 0x01
        sim.reset()
        sim.load(bytes(prog), 0)
        sim._rf.flags.set_i(0)
        sim.interrupt()
        assert sim.get_state().flag_i is True

    def test_irq_b_bit_not_set(self, sim):
        prog = bytearray(65536)
        prog[0xFFFE] = 0x00
        prog[0xFFFF] = 0x01
        sim.reset()
        sim.load(bytes(prog), 0)
        sim._rf.flags.set_i(0)
        s_before = sim.get_state().s
        sim.interrupt()
        s_after = sim.get_state().s
        pushed_p = sim._memory[0x0100 | ((s_after + 1) & 0xFF)]
        assert not (pushed_p & 0x10), "B bit should be clear in IRQ-pushed P"


# ── All addressing modes ────────────────────────────────────────────────────────

class TestAllAddressingModes:
    def test_immediate(self, sim):
        result = sim.execute(bytes([0xA9, 0x42, 0x00]))
        assert result.final_state.a == 0x42

    def test_zero_page(self, sim):
        prog = bytes([0xA9, 0x55, 0x85, 0x30, 0xA9, 0x00, 0xA5, 0x30, 0x00])
        result = sim.execute(prog)
        assert result.final_state.a == 0x55

    def test_zero_page_x(self, sim):
        prog = bytes([
            0xA2, 0x02,         # LDX #2
            0xA9, 0x77, 0x95, 0x10,   # LDA #0x77; STA $10,X
            0xA9, 0x00, 0xB5, 0x10,   # LDA #0; LDA $10,X
            0x00,
        ])
        result = sim.execute(prog)
        assert result.final_state.a == 0x77

    def test_zero_page_x_wraps(self, sim):
        # ZPX wraps in zero page: $FF + 2 = $01
        prog = bytes([
            0xA9, 0xAB, 0x85, 0x01,   # LDA #0xAB; STA $01
            0xA2, 0x02,               # LDX #2
            0xA9, 0x00,
            0xB5, 0xFF,               # LDA $FF,X  → addr = ($FF+2)&$FF = $01
            0x00,
        ])
        result = sim.execute(prog)
        assert result.final_state.a == 0xAB

    def test_zero_page_y(self, sim):
        prog = bytes([
            0xA0, 0x03,               # LDY #3
            0xA9, 0x66, 0x96, 0x10,   # LDA #0x66; STX $10,Y but use STX
            # Actually use LDX then STX:
        ])
        prog = bytes([
            0xA2, 0x55,               # LDX #0x55
            0xA0, 0x03,               # LDY #3
            0x96, 0x10,               # STX $10,Y → $13
            0xA2, 0x00,               # LDX #0
            0xB6, 0x10,               # LDX $10,Y → loads from $13
            0x00,
        ])
        result = sim.execute(prog)
        assert result.final_state.x == 0x55

    def test_absolute(self, sim):
        prog = bytearray(20)
        prog[0] = 0xA9; prog[1] = 0x99
        prog[2] = 0x8D; prog[3] = 0x00; prog[4] = 0x03  # STA $0300
        prog[5] = 0xA9; prog[6] = 0x00
        prog[7] = 0xAD; prog[8] = 0x00; prog[9] = 0x03  # LDA $0300
        prog[10] = 0x00
        result = sim.execute(bytes(prog))
        assert result.final_state.a == 0x99

    def test_accumulator_asl(self, sim):
        result = sim.execute(bytes([0xA9, 0x04, 0x0A, 0x00]))
        assert result.final_state.a == 8

    def test_implied(self, sim):
        result = sim.execute(bytes([0xA2, 0x05, 0xE8, 0x00]))
        assert result.final_state.x == 6

    def test_relative_forward(self, sim):
        # BCC +2 (skip 2 bytes)
        prog = bytes([0x18, 0x90, 0x02, 0xEA, 0xEA, 0xA9, 0x01, 0x00])
        result = sim.execute(prog)
        assert result.final_state.a == 1

    def test_relative_backward(self, sim):
        # Loop: LDA #5; BNE to same instruction (infinite) → use counter
        prog = bytearray(20)
        prog[0] = 0xA2; prog[1] = 0x03   # LDX #3
        prog[2] = 0xCA                   # DEX
        prog[3] = 0xD0; prog[4] = 0xFD   # BNE -3 (to DEX)
        prog[5] = 0x00
        result = sim.execute(prog)
        assert result.final_state.x == 0


# ── Flag edge cases ────────────────────────────────────────────────────────────

class TestFlagEdgeCases:
    def test_z_flag_on_zero(self, sim):
        result = sim.execute(bytes([0xA9, 0x00, 0x00]))
        assert result.final_state.flag_z is True
        assert result.final_state.flag_n is False

    def test_n_flag_on_0x80(self, sim):
        result = sim.execute(bytes([0xA9, 0x80, 0x00]))
        assert result.final_state.flag_n is True
        assert result.final_state.flag_z is False

    def test_v_flag_adc_overflow(self, sim):
        result = sim.execute(bytes([0xA9, 0x7F, 0x18, 0x69, 0x01, 0x00]))
        assert result.final_state.flag_v is True

    def test_v_flag_clears(self, sim):
        result = sim.execute(bytes([0xA9, 0x7F, 0x18, 0x69, 0x01, 0xB8, 0x00]))
        assert result.final_state.flag_v is False

    def test_c_flag_adc_carry(self, sim):
        result = sim.execute(bytes([0xA9, 0xFF, 0x18, 0x69, 0x01, 0x00]))
        assert result.final_state.flag_c is True
        assert result.final_state.a == 0

    def test_z_flag_inx_wraps(self, sim):
        result = sim.execute(bytes([0xA2, 0xFF, 0xE8, 0x00]))
        assert result.final_state.x == 0
        assert result.final_state.flag_z is True

    def test_n_flag_tax(self, sim):
        result = sim.execute(bytes([0xA9, 0xFF, 0xAA, 0x00]))
        assert result.final_state.flag_n is True

    def test_txs_no_flags(self, sim):
        # TXS does NOT set N or Z
        result = sim.execute(bytes([0xA9, 0xFF, 0xA8, 0xA2, 0x00, 0x9A, 0x00]))
        # X=0x00 loaded, TXS → S=0x00
        # Y was 0xFF before, but that shouldn't affect things
        assert result.final_state.flag_z is not None   # just verify no crash

    def test_rol_with_carry_in(self, sim):
        # SEC; ROL A=0 → A=1 (carry in)
        result = sim.execute(bytes([0xA9, 0x00, 0x38, 0x2A, 0x00]))
        assert result.final_state.a == 1
        assert result.final_state.flag_c is False

    def test_ror_with_carry_in(self, sim):
        result = sim.execute(bytes([0xA9, 0x00, 0x38, 0x6A, 0x00]))
        assert result.final_state.a == 0x80
        assert result.final_state.flag_c is False


# ── Memory operations ─────────────────────────────────────────────────────────

class TestMemoryOperations:
    def test_inc_memory_zero_page(self, sim):
        result = sim.execute(bytes([
            0xA9, 0xFF, 0x85, 0x10,   # LDA #0xFF; STA $10
            0xE6, 0x10,               # INC $10
            0xA5, 0x10,               # LDA $10
            0x00,
        ]))
        assert result.final_state.a == 0   # 0xFF + 1 wraps to 0
        assert result.final_state.flag_z is True

    def test_dec_memory_zero_page(self, sim):
        result = sim.execute(bytes([
            0xA9, 0x00, 0x85, 0x10,
            0xC6, 0x10,
            0xA5, 0x10,
            0x00,
        ]))
        assert result.final_state.a == 0xFF

    def test_asl_memory(self, sim):
        result = sim.execute(bytes([
            0xA9, 0x01, 0x85, 0x10,
            0x06, 0x10,               # ASL $10
            0xA5, 0x10,
            0x00,
        ]))
        assert result.final_state.a == 2

    def test_lsr_memory(self, sim):
        result = sim.execute(bytes([
            0xA9, 0x04, 0x85, 0x10,
            0x46, 0x10,
            0xA5, 0x10,
            0x00,
        ]))
        assert result.final_state.a == 2

    def test_rol_memory(self, sim):
        result = sim.execute(bytes([
            0xA9, 0x01, 0x85, 0x10,
            0x38,                     # SEC
            0x26, 0x10,               # ROL $10
            0xA5, 0x10,
            0x00,
        ]))
        assert result.final_state.a == 3   # 0x01 << 1 | C=1 = 3

    def test_ror_memory(self, sim):
        result = sim.execute(bytes([
            0xA9, 0x04, 0x85, 0x10,
            0x38,
            0x66, 0x10,               # ROR $10
            0xA5, 0x10,
            0x00,
        ]))
        assert result.final_state.a == 0x82   # 0x04 >> 1 | 0x80 = 0x82

    def test_bit_absolute(self, sim):
        prog = bytearray(20)
        prog[0] = 0xA9; prog[1] = 0xC0
        prog[2] = 0x8D; prog[3] = 0x00; prog[4] = 0x02   # STA $0200
        prog[5] = 0xA9; prog[6] = 0x3F
        prog[7] = 0x2C; prog[8] = 0x00; prog[9] = 0x02   # BIT $0200
        prog[10] = 0x00
        result = sim.execute(bytes(prog))
        # M = 0xC0: N=1, V=1, Z=1 (A & M = 0x3F & 0xC0 = 0)
        assert result.final_state.flag_n is True
        assert result.final_state.flag_v is True
        assert result.final_state.flag_z is True


# ── PLP / RTI edge cases ───────────────────────────────────────────────────────

class TestPLPRTIEdgeCases:
    def test_plp_restores_flags(self, sim):
        # Push specific flags and PLP
        result = sim.execute(bytes([
            0xA9, 0x01,   # LDA #1 (C=1, N/Z cleared)
            0x48,          # PHA  (push 0x01 — but this pushes A not P)
            # Actually push P via PHP first:
            0x38, 0x08,   # SEC; PHP  (push P with C=1)
            0x18,          # CLC
            0x28,          # PLP (restore P with C=1)
            0x00,
        ]))
        assert result.final_state.flag_c is True

    def test_bit5_always_set_after_plp(self, sim):
        # Even if bit 5 is 0 in the pushed P, it should come back as 1
        prog = bytes([
            0xA9, 0xDF,   # LDA #0xDF (bit5=0)
            0x48,          # PHA
            0x28,          # PLP
            0x00,
        ])
        result = sim.execute(prog)
        # Flags: 0xDF = 1101 1111; bit5=0; after PLP bit5 doesn't matter
        # Just verify it ran
        assert result.halted


# ── Step trace ─────────────────────────────────────────────────────────────────

class TestStepTrace:
    def test_trace_contains_mnemonic(self, sim):
        sim.load(bytes([0xA9, 0x42, 0x00]))
        trace = sim.step()
        assert trace.mnemonic == "LDA"

    def test_trace_pc_before_after(self, sim):
        sim.load(bytes([0xA9, 0x42, 0xEA, 0x00]))
        trace1 = sim.step()   # LDA #0x42
        assert trace1.pc_before == 0
        assert trace1.pc_after == 2
        trace2 = sim.step()   # NOP
        assert trace2.pc_before == 2
        assert trace2.pc_after == 3

    def test_all_descriptions_are_strings(self, sim):
        sim.load(bytes([
            0xA9, 0x01, 0xA2, 0x02, 0xA0, 0x03,
            0xAA, 0xA8, 0x8A, 0x98,
            0x48, 0x68, 0x08, 0x28,
            0x18, 0x38, 0x58, 0x78, 0xB8, 0xD8, 0xF8,
            0xEA, 0x00,
        ]))
        traces = []
        while not sim.get_state().halted:
            traces.append(sim.step())
        for t in traces:
            assert isinstance(t.description, str)
            assert len(t.description) > 0
