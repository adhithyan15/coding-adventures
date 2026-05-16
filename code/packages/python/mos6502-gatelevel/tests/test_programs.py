"""End-to-end programs for the MOS 6502 gate-level simulator."""

from __future__ import annotations

import pytest

from mos6502_gatelevel import MOS6502GateLevelSimulator


@pytest.fixture()
def sim():
    return MOS6502GateLevelSimulator()


# ── Utility ───────────────────────────────────────────────────────────────────

def run(program, origin=0):
    s = MOS6502GateLevelSimulator()
    return s.execute(bytes(program), origin)


# ── Basic programs ────────────────────────────────────────────────────────────

class TestBasicPrograms:
    def test_load_and_halt(self, sim):
        result = sim.execute(bytes([0xA9, 0x42, 0x00]))
        assert result.final_state.a == 0x42
        assert result.halted is True

    def test_nop_then_halt(self, sim):
        result = sim.execute(bytes([0xEA, 0xEA, 0x00]))
        assert result.halted is True

    def test_add_two_numbers(self, sim):
        result = sim.execute(bytes([
            0xA9, 0x0A,   # LDA #10
            0x18,          # CLC
            0x69, 0x05,   # ADC #5
            0x00,
        ]))
        assert result.final_state.a == 15

    def test_subtract_two_numbers(self, sim):
        result = sim.execute(bytes([
            0xA9, 0x0A,   # LDA #10
            0x38,          # SEC
            0xE9, 0x03,   # SBC #3
            0x00,
        ]))
        assert result.final_state.a == 7
        assert result.final_state.flag_c is True

    def test_store_and_load(self, sim):
        result = sim.execute(bytes([
            0xA9, 0x55,   # LDA #0x55
            0x85, 0x20,   # STA $20
            0xA9, 0x00,   # LDA #0
            0xA5, 0x20,   # LDA $20
            0x00,
        ]))
        assert result.final_state.a == 0x55


# ── Loop programs ─────────────────────────────────────────────────────────────

class TestLoopPrograms:
    def test_count_down_loop(self, sim):
        # Count X from 5 down to 0
        prog = [
            0xA2, 0x05,   # LDX #5
            # Loop:
            0xCA,          # DEX
            0xD0, 0xFD,   # BNE -3 (back to DEX)
            0x00,
        ]
        result = sim.execute(bytes(prog))
        assert result.final_state.x == 0
        assert result.final_state.flag_z is True

    def test_sum_1_to_5(self, sim):
        # Sum 1+2+3+4+5 = 15
        prog = bytearray(20)
        prog[0] = 0xA9; prog[1] = 0x00   # LDA #0 (sum)
        prog[2] = 0xA2; prog[3] = 0x05   # LDX #5 (counter)
        # Loop:
        prog[4] = 0x18                   # CLC
        prog[5] = 0x86; prog[6] = 0x10   # STX $10
        prog[7] = 0x65; prog[8] = 0x10   # ADC $10 (A += X)
        prog[9] = 0xCA                   # DEX
        prog[10] = 0xD0; prog[11] = 0xF8  # BNE back to CLC
        prog[12] = 0x00
        result = sim.execute(bytes(prog))
        assert result.final_state.a == 15

    def test_multiply_by_shifting(self, sim):
        # Multiply 3 by 4 using ASL (left shift = multiply by 2)
        prog = [
            0xA9, 0x03,   # LDA #3
            0x0A,          # ASL  (3 << 1 = 6)
            0x0A,          # ASL  (6 << 1 = 12)
            0x00,
        ]
        result = sim.execute(bytes(prog))
        assert result.final_state.a == 12

    def test_indexed_loop_fill(self, sim):
        # Fill memory[$10..$14] with 0x55 using indexed loop
        # X goes from 0 to 4, incrementing
        prog = bytearray(20)
        prog[0] = 0xA9; prog[1] = 0x55   # LDA #0x55
        prog[2] = 0xA2; prog[3] = 0x00   # LDX #0
        # Loop:
        prog[4] = 0x95; prog[5] = 0x10   # STA $10,X
        prog[6] = 0xE8                   # INX
        prog[7] = 0xE0; prog[8] = 0x05   # CPX #5
        prog[9] = 0xD0; prog[10] = 0xF9  # BNE -7 (back to STA)
        prog[11] = 0x00
        result = sim.execute(bytes(prog))
        for i in range(5):
            assert result.final_state.memory[0x10 + i] == 0x55


# ── Subroutine programs ────────────────────────────────────────────────────────

class TestSubroutinePrograms:
    def test_simple_subroutine(self, sim):
        prog = bytearray(256)
        # Main: LDA #5; JSR $0010; BRK
        prog[0] = 0xA9; prog[1] = 0x05
        prog[2] = 0x20; prog[3] = 0x10; prog[4] = 0x00
        prog[5] = 0x00
        # Subroutine at $10: ADC #5; RTS
        prog[0x10] = 0x18
        prog[0x11] = 0x69; prog[0x12] = 0x05
        prog[0x13] = 0x60
        result = sim.execute(bytes(prog))
        assert result.final_state.a == 10

    def test_subroutine_with_arguments_in_memory(self, sim):
        prog = bytearray(256)
        # Store arg at $10
        prog[0] = 0xA9; prog[1] = 0x07
        prog[2] = 0x85; prog[3] = 0x10
        # Call multiply_by_3 at $20
        prog[4] = 0x20; prog[5] = 0x20; prog[6] = 0x00
        prog[7] = 0x00
        # Subroutine: A = $10 * 3 (via 2 additions)
        prog[0x20] = 0xA5; prog[0x21] = 0x10   # LDA arg
        prog[0x22] = 0x18; prog[0x23] = 0x65; prog[0x24] = 0x10  # ADC arg
        prog[0x25] = 0x18; prog[0x26] = 0x65; prog[0x27] = 0x10  # ADC arg
        prog[0x28] = 0x60
        result = sim.execute(bytes(prog))
        assert result.final_state.a == 21   # 7 * 3

    def test_stack_preserved_across_subroutine(self, sim):
        prog = bytearray(256)
        prog[0] = 0xA9; prog[1] = 0xAB    # LDA #0xAB
        prog[2] = 0x48                     # PHA
        prog[3] = 0x20; prog[4] = 0x10; prog[5] = 0x00  # JSR $0010
        prog[6] = 0x68                     # PLA
        prog[7] = 0x00                     # BRK
        prog[0x10] = 0xA9; prog[0x11] = 0x00  # LDA #0 (trash A)
        prog[0x12] = 0x60                  # RTS
        result = sim.execute(bytes(prog))
        assert result.final_state.a == 0xAB   # PLA restored pushed value


# ── BCD programs ─────────────────────────────────────────────────────────────

class TestBCDPrograms:
    def test_bcd_add_9_plus_1(self, sim):
        prog = [0xF8, 0xA9, 0x09, 0x18, 0x69, 0x01, 0x00]
        result = sim.execute(bytes(prog))
        assert result.final_state.a == 0x10   # BCD 9+1=10

    def test_bcd_add_with_carry(self, sim):
        prog = [0xF8, 0xA9, 0x99, 0x18, 0x69, 0x01, 0x00]
        result = sim.execute(bytes(prog))
        assert result.final_state.a == 0x00
        assert result.final_state.flag_c is True  # BCD carry

    def test_bcd_sub(self, sim):
        prog = [0xF8, 0xA9, 0x10, 0x38, 0xE9, 0x01, 0x00]
        result = sim.execute(bytes(prog))
        assert result.final_state.a == 0x09

    def test_bcd_exit_mode(self, sim):
        # Enter BCD, do operation, exit BCD
        prog = [0xF8, 0xA9, 0x09, 0x18, 0x69, 0x01, 0xD8, 0x00]
        result = sim.execute(bytes(prog))
        assert result.final_state.flag_d is False


# ── Index register programs ────────────────────────────────────────────────────

class TestIndexPrograms:
    def test_array_copy_with_x(self, sim):
        # Copy 5 bytes from $50–$54 to $60–$64 using X-indexed loop
        prog = bytearray(256)
        # Pre-load source data at $50-$54
        for i, v in enumerate([0x11, 0x22, 0x33, 0x44, 0x55]):
            prog[0x50 + i] = v
        # Copy loop (starts at $00)
        prog[0x00] = 0xA2; prog[0x01] = 0x04   # LDX #4
        prog[0x02] = 0xBD; prog[0x03] = 0x50; prog[0x04] = 0x00  # LDA $0050,X
        prog[0x05] = 0x9D; prog[0x06] = 0x60; prog[0x07] = 0x00  # STA $0060,X
        prog[0x08] = 0xCA                       # DEX
        prog[0x09] = 0x10; prog[0x0A] = 0xF7   # BPL -9 (back to LDA)
        prog[0x0B] = 0x00                       # BRK
        result = sim.execute(bytes(prog), origin=0)
        assert result.halted
        for i, v in enumerate([0x11, 0x22, 0x33, 0x44, 0x55]):
            assert result.final_state.memory[0x60 + i] == v

    def test_y_indexed_store(self, sim):
        prog = bytearray(20)
        prog[0] = 0xA9; prog[1] = 0xAA   # LDA #0xAA
        prog[2] = 0xA0; prog[3] = 0x03   # LDY #3
        prog[4] = 0x99; prog[5] = 0x00; prog[6] = 0x02  # STA $0200,Y
        prog[7] = 0x00
        result = sim.execute(bytes(prog))
        assert result.final_state.memory[0x0203] == 0xAA


# ── Memory I/O programs ───────────────────────────────────────────────────────

class TestIOPrograms:
    def test_read_input_port(self, sim):
        sim.set_input_port(0, 0x42)
        result = sim.execute(bytes([
            0xAD, 0x00, 0xFF,  # LDA $FF00 (port 0)
            0x00,
        ]))
        assert result.final_state.a == 0x42

    def test_write_output_port(self, sim):
        sim.execute(bytes([
            0xA9, 0x77,
            0x8D, 0x01, 0xFF,  # STA $FF01 (port 1)
            0x00,
        ]))
        assert sim.get_output_port(1) == 0x77

    def test_port_range(self, sim):
        sim.set_input_port(239, 0xFF)
        result = sim.execute(bytes([
            0xAD, 0xEF, 0xFF,  # LDA $FFEF (port 239)
            0x00,
        ]))
        assert result.final_state.a == 0xFF

    def test_invalid_port_raises(self, sim):
        with pytest.raises(ValueError):
            sim.set_input_port(240, 0)

    def test_invalid_port_get_raises(self, sim):
        with pytest.raises(ValueError):
            sim.get_output_port(240)


# ── JMP programs ─────────────────────────────────────────────────────────────

class TestJMPPrograms:
    def test_jmp_absolute(self, sim):
        prog = bytearray(256)
        prog[0] = 0x4C; prog[1] = 0x10; prog[2] = 0x00  # JMP $0010
        prog[3] = 0xEA                                    # NOP (should be skipped)
        prog[0x10] = 0xA9; prog[0x11] = 0x42             # LDA #0x42
        prog[0x12] = 0x00
        result = sim.execute(bytes(prog))
        assert result.final_state.a == 0x42

    def test_jmp_indirect_basic(self, sim):
        prog = bytearray(256)
        # Store target $0020 at $10/$11
        prog[0] = 0xA9; prog[1] = 0x20
        prog[2] = 0x85; prog[3] = 0x10   # $10 = 0x20 (lo)
        prog[4] = 0xA9; prog[5] = 0x00
        prog[6] = 0x85; prog[7] = 0x11   # $11 = 0x00 (hi)
        prog[8] = 0x6C; prog[9] = 0x10; prog[10] = 0x00  # JMP ($0010)
        prog[0x20] = 0xA9; prog[0x21] = 0x55
        prog[0x22] = 0x00
        result = sim.execute(bytes(prog))
        assert result.final_state.a == 0x55

    def test_jmp_indirect_page_wrap_bug(self, sim):
        # The classic 6502 JMP ($xxFF) bug
        # Use a pointer within page 0: $00FF/$0000 wraps to same page
        prog = bytearray(65536)
        # Place vector at $00FF/$0000 (not $00FF/$0100)
        prog[0x00FF] = 0x20   # lo byte of target: $0020
        prog[0x0000] = 0x00   # hi byte read due to bug (page wrap)
        prog[0x0100] = 0x40   # Would be hi byte if no bug ($4020)
        # JMP ($00FF) — the indirect address
        prog[3] = 0x6C; prog[4] = 0xFF; prog[5] = 0x00  # JMP ($00FF)
        # Target at $0020:
        prog[0x0020] = 0xA9; prog[0x0021] = 0x42
        prog[0x0022] = 0x00
        # But $0000 = 0x6C (the JMP instruction itself), wait - conflict!
        # Use simpler approach: vector at $10FF/$1000 (page 0x10)
        prog2 = bytearray(65536)
        prog2[0x10FF] = 0x20   # lo byte of target
        prog2[0x1000] = 0x00   # hi byte due to bug — target = $0020
        prog2[0x1100] = 0x40   # Would be hi byte without bug — target = $4020
        prog2[0x0020] = 0xA9; prog2[0x0021] = 0x42
        prog2[0x0022] = 0x00
        prog2[0] = 0x6C; prog2[1] = 0xFF; prog2[2] = 0x10   # JMP ($10FF)
        result = sim.execute(bytes(prog2))
        # Due to bug, hi byte from $1000 = 0x00, lo from $10FF = 0x20
        # Target = $0020, not $4020
        assert result.final_state.a == 0x42
