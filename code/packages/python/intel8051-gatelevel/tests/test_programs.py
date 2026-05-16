"""Full program tests for intel8051_gatelevel.Intel8051GateLevelSimulator.

Each test encodes a complete program as raw 8051 machine code and verifies
the expected final state after execution.

These tests exercise the simulator end-to-end:
  1. Sum 1..10 via DJNZ loop
  2. MUL AB: 12 × 17 = 204
  3. DIV AB: 100 / 7 = quotient 14, remainder 2
  4. DA A: BCD arithmetic
  5. Bit-addressable: set/clear ACC bit 5, check via JB
  6. PUSH/POP stack round-trip
  7. LCALL/RET subroutine
  8. CJNE comparison loop
  9. MOVC table lookup via @A+DPTR
"""

import pytest
from intel8051_simulator.state import SFR_ACC, SFR_B

from intel8051_gatelevel import Intel8051GateLevelSimulator

HALT = 0xA5


@pytest.fixture
def sim():
    return Intel8051GateLevelSimulator()


class TestSumLoop:
    """Test 1: Sum 1+2+...+10 = 55 using DJNZ."""

    def test_sum_1_to_10(self, sim):
        # ACC = 0, R0 = 10
        # Loop: ACC += R0; DJNZ R0, loop
        # Result: 10+9+8+7+6+5+4+3+2+1 = 55
        prog = bytes([
            0x74, 0x00,   # MOV A, #0      (sum = 0)
            0x78, 0x0A,   # MOV R0, #10    (counter = 10)
            # loop: (address 4)
            0x28,         # ADD A, R0      (sum += counter)
            0xD8, 0xFD,   # DJNZ R0, -3   (back to ADD at offset -3)
            HALT,
        ])
        result = sim.execute(prog)
        assert result.halted
        assert result.final_state.acc == 55


class TestMultiply:
    """Test 2: MUL AB — 12 × 17 = 204."""

    def test_mul_12_times_17(self, sim):
        prog = bytes([
            0x74, 12,            # MOV A, #12
            0x75, SFR_B, 17,    # MOV B, #17
            0xA4,                # MUL AB
            HALT,
        ])
        result = sim.execute(prog)
        assert result.halted
        state = result.final_state
        assert state.acc == 204
        assert state.b == 0
        assert not state.cy   # CY always 0 after MUL
        assert not state.ov   # result ≤ 255, no overflow

    def test_mul_overflow(self, sim):
        prog = bytes([
            0x74, 0xFF,          # MOV A, #255
            0x75, SFR_B, 0xFF,  # MOV B, #255
            0xA4,                # MUL AB → 65025 = 0xFE01
            HALT,
        ])
        result = sim.execute(prog)
        assert result.halted
        state = result.final_state
        assert state.acc == 0x01
        assert state.b == 0xFE
        assert state.ov   # overflow set when B != 0


class TestDivide:
    """Test 3: DIV AB — 100 / 7 = quotient 14, remainder 2."""

    def test_div_100_by_7(self, sim):
        prog = bytes([
            0x74, 100,           # MOV A, #100
            0x75, SFR_B, 7,     # MOV B, #7
            0x84,                # DIV AB
            HALT,
        ])
        result = sim.execute(prog)
        assert result.halted
        state = result.final_state
        assert state.acc == 14
        assert state.b == 2
        assert not state.cy
        assert not state.ov

    def test_div_by_zero(self, sim):
        prog = bytes([
            0x74, 0xFF,          # MOV A, #255
            0x75, SFR_B, 0x00,  # MOV B, #0 (divide by zero)
            0x84,                # DIV AB → OV = 1
            HALT,
        ])
        result = sim.execute(prog)
        assert result.halted
        state = result.final_state
        assert state.ov  # divide by zero sets OV

    def test_div_exact(self, sim):
        prog = bytes([
            0x74, 0x14,          # MOV A, #20
            0x75, SFR_B, 0x04,  # MOV B, #4
            0x84,                # DIV AB → A=5, B=0
            HALT,
        ])
        result = sim.execute(prog)
        state = result.final_state
        assert state.acc == 5
        assert state.b == 0


class TestBCDArithmetic:
    """Test 4: DA A — decimal adjust after BCD addition."""

    def test_bcd_29_plus_47(self, sim):
        # BCD 29 + BCD 47 = BCD 76
        # Binary: 0x29 + 0x47 = 0x70, but AC=1 (carry from bit 3 to bit 4)
        # DA A: AC=1 triggers low nibble correction: 0x70 + 6 = 0x76
        # High nibble 7 ≤ 9 and CY=0 → no high correction → result = 0x76 (BCD 76)
        prog = bytes([
            0x74, 0x29,   # MOV A, #0x29 (BCD 29)
            0x24, 0x47,   # ADD A, #0x47 (binary add: 0x29+0x47=0x70, AC=1)
            0xD4,         # DA A         (AC=1 → +6 to low nibble → 0x76)
            HALT,
        ])
        result = sim.execute(prog)
        assert result.halted
        state = result.final_state
        assert state.acc == 0x76  # BCD 76 (the correct decimal result for 29+47)

    def test_bcd_adjustment_needed(self, sim):
        # 0x09 (BCD 9) + 0x01 (BCD 1) = 0x0A, but A>9 so DA adds 6 → 0x10 (BCD 10)
        prog = bytes([
            0x74, 0x09,
            0x24, 0x01,   # ADD: 0x09 + 0x01 = 0x0A, AC=1
            0xD4,         # DA A: low nibble A > 9 → +6 → 0x10
            HALT,
        ])
        result = sim.execute(prog)
        state = result.final_state
        assert state.acc == 0x10  # BCD 10


class TestBitAddressable:
    """Test 5: Bit-addressable — set/clear ACC bit 5, check via JB."""

    def test_setb_acc_bit5_and_jb(self, sim):
        # ACC bit 5 has bit address 0xE5
        prog = bytes([
            0xE4,             # CLR A (ACC = 0x00)
            0xD2, 0xE5,       # SETB 0xE5 (ACC bit 5 = 1 → ACC = 0x20)
            # JB ACC.5, +2 — branch if bit 5 is set
            0x20, 0xE5, 0x01,  # JB ACC.5, +1 (skip next byte)
            0x00,             # NOP (skipped)
            0x74, 0xFF,       # MOV A, #0xFF (reached if branch taken)
            HALT,
        ])
        result = sim.execute(prog)
        assert result.halted
        state = result.final_state
        assert state.acc == 0xFF  # JB was taken (branch executed)

    def test_clr_acc_bit5(self, sim):
        prog = bytes([
            0x74, 0x20,       # MOV A, #0x20 (set bit 5)
            0xC2, 0xE5,       # CLR ACC.5 → A = 0x00
            HALT,
        ])
        result = sim.execute(prog)
        state = result.final_state
        assert state.acc == 0x00


class TestStackOperations:
    """Test 6: PUSH/POP stack round-trip."""

    def test_push_pop_roundtrip(self, sim):
        prog = bytes([
            0x74, 0x42,   # MOV A, #0x42
            0xF5, SFR_ACC,  # MOV dir, A (copy A to direct)
            0xC0, SFR_ACC,  # PUSH ACC
            0x74, 0x00,   # MOV A, #0   (corrupt A)
            0xD0, SFR_ACC,  # POP ACC    (restore from stack)
            HALT,
        ])
        result = sim.execute(prog)
        assert result.halted
        state = result.final_state
        assert state.acc == 0x42

    def test_stack_preserves_order(self, sim):
        # Push 0x11 then 0x22, pop should give 0x22 first (LIFO)
        prog = bytes([
            0x75, SFR_ACC, 0x11,   # MOV ACC, #0x11
            0xC0, SFR_ACC,          # PUSH ACC
            0x75, SFR_ACC, 0x22,   # MOV ACC, #0x22
            0xC0, SFR_ACC,          # PUSH ACC (stack: 0x11, 0x22)
            0xD0, SFR_ACC,          # POP ACC → 0x22
            HALT,
        ])
        result = sim.execute(prog)
        state = result.final_state
        assert state.acc == 0x22

    def test_sp_incremented(self, sim):
        prog = bytes([
            0x74, 0xAB,
            0xC0, SFR_ACC,    # PUSH ACC → SP = 0x08
            HALT,
        ])
        result = sim.execute(prog)
        state = result.final_state
        assert state.sp == 0x08  # SP was 0x07, pushed 1 byte → 0x08


class TestSubroutine:
    """Test 7: LCALL/RET subroutine call."""

    def test_lcall_ret(self, sim):
        # Main: MOV A, #0; LCALL sub; (A should be 0x42 after call); HALT
        # Sub at address 0x10: MOV A, #0x42; RET
        prog = bytearray(0x20)
        prog[0] = 0x74
        prog[1] = 0x00  # MOV A, #0
        prog[2] = 0x12
        prog[3] = 0x00
        prog[4] = 0x10  # LCALL 0x0010
        prog[5] = HALT
        # Subroutine at 0x10
        prog[0x10] = 0x74
        prog[0x11] = 0x42  # MOV A, #0x42
        prog[0x12] = 0x22                       # RET

        result = sim.execute(bytes(prog))
        assert result.halted
        state = result.final_state
        assert state.acc == 0x42

    def test_ret_returns_to_caller(self, sim):
        prog = bytearray(0x20)
        # Main code: LCALL sub, then MOV A, #0x55, then HALT
        prog[0] = 0x12
        prog[1] = 0x00
        prog[2] = 0x10  # LCALL 0x0010
        prog[3] = 0x74
        prog[4] = 0x55  # MOV A, #0x55 (executed after RET)
        prog[5] = HALT
        # Sub: immediately RET
        prog[0x10] = 0x22  # RET

        result = sim.execute(bytes(prog))
        state = result.final_state
        assert state.acc == 0x55  # MOV after LCALL was executed


class TestCJNE:
    """Test 8: CJNE comparison loop."""

    def test_cjne_loop_counts_down(self, sim):
        # Load R0 = 0, loop while R0 != 5 (increment each time)
        # Actually: CJNE A, #imm: jump if A != imm
        prog = bytes([
            0x74, 0x00,   # MOV A, #0
            # loop: (offset 2)
            0xB4, 0x05, 0x01,  # CJNE A, #5, +1 (skip INC if A==5)
            HALT,
            0x04,         # INC A
            0x80, 0xF8,   # SJMP -8 (back to CJNE)
            HALT,
        ])
        result = sim.execute(prog)
        assert result.halted
        state = result.final_state
        assert state.acc == 5

    def test_cjne_not_taken_when_equal(self, sim):
        prog = bytes([
            0x74, 0x0A,   # MOV A, #10
            0xB4, 0x0A, 0x01,  # CJNE A, #10, +1 → NOT taken (A==10)
            HALT,
            0x74, 0xFF,   # MOV A, #0xFF (would be reached if branch taken)
            HALT,
        ])
        result = sim.execute(prog)
        # Branch not taken, so we hit the first HALT
        assert result.halted
        assert result.final_state.acc == 0x0A  # unchanged

    def test_cjne_sets_cy_when_less(self, sim):
        prog = bytes([
            0xC3,         # CLR C
            0x74, 0x05,   # MOV A, #5
            # CJNE A, #10, rel: A < imm → CY = 1
            0xB4, 0x0A, 0x00,  # CJNE A, #10, +0 (branch target = next instr)
            HALT,
        ])
        result = sim.execute(prog)
        assert result.final_state.cy  # CY set because 5 < 10


class TestMOVC:
    """Test 9: MOVC table lookup via @A+DPTR."""

    def test_movc_table_lookup(self, sim):
        # Table at offset 0x10 from program start:
        # Table: [0x11, 0x22, 0x33, 0x44]
        # Load A=2 (index), DPTR=0x10 (table base), MOVC A, @A+DPTR → A = table[2] = 0x33
        table_offset = 0x10
        prog = bytearray(table_offset + 4)
        prog[0] = 0x74
        prog[1] = 0x02  # MOV A, #2 (index)
        prog[2] = 0x90
        prog[3] = 0x00
        prog[4] = table_offset  # MOV DPTR, #table_offset
        prog[5] = 0x93   # MOVC A, @A+DPTR
        prog[6] = HALT
        # Table data
        prog[table_offset + 0] = 0x11
        prog[table_offset + 1] = 0x22
        prog[table_offset + 2] = 0x33
        prog[table_offset + 3] = 0x44

        result = sim.execute(bytes(prog))
        assert result.halted
        assert result.final_state.acc == 0x33

    def test_movc_index_zero(self, sim):
        table_offset = 0x10
        prog = bytearray(table_offset + 4)
        prog[0] = 0x74
        prog[1] = 0x00  # MOV A, #0 (index 0)
        prog[2] = 0x90
        prog[3] = 0x00
        prog[4] = table_offset
        prog[5] = 0x93   # MOVC A, @A+DPTR
        prog[6] = HALT
        prog[table_offset + 0] = 0xAB
        prog[table_offset + 1] = 0xCD

        result = sim.execute(bytes(prog))
        assert result.final_state.acc == 0xAB


class TestMiscInstructions:
    """Additional coverage tests for various instruction groups."""

    def test_inc_dec(self, sim):
        prog = bytes([
            0x74, 0x05,   # MOV A, #5
            0x04,         # INC A → 6
            0x04,         # INC A → 7
            0x14,         # DEC A → 6
            HALT,
        ])
        result = sim.execute(prog)
        assert result.final_state.acc == 6

    def test_rotates(self, sim):
        prog = bytes([
            0x74, 0x01,   # MOV A, #0x01
            0x23,         # RL A → 0x02
            0x23,         # RL A → 0x04
            0x23,         # RL A → 0x08
            HALT,
        ])
        result = sim.execute(prog)
        assert result.final_state.acc == 0x08

    def test_swap(self, sim):
        prog = bytes([
            0x74, 0xAB,   # MOV A, #0xAB
            0xC4,         # SWAP A → 0xBA
            HALT,
        ])
        result = sim.execute(prog)
        assert result.final_state.acc == 0xBA

    def test_xch(self, sim):
        prog = bytes([
            0x74, 0x42,   # MOV A, #0x42
            0xF8,         # MOV R0, A      → R0 = 0x42
            0x74, 0xAB,   # MOV A, #0xAB
            0xC8,         # XCH A, R0      → A = 0x42, R0 = 0xAB
            HALT,
        ])
        result = sim.execute(prog)
        assert result.final_state.acc == 0x42

    def test_mov_dptr_and_inc(self, sim):
        prog = bytes([
            0x90, 0x12, 0x34,  # MOV DPTR, #0x1234
            0xA3,              # INC DPTR → 0x1235
            HALT,
        ])
        result = sim.execute(prog)
        state = result.final_state
        assert state.dptr == 0x1235

    def test_sjmp_forward(self, sim):
        prog = bytes([
            0x80, 0x02,   # SJMP +2 (skip 2 bytes)
            0x74, 0xDE,   # MOV A, #0xDE (skipped)
            0x74, 0x42,   # MOV A, #0x42 (reached)
            HALT,
        ])
        result = sim.execute(prog)
        assert result.final_state.acc == 0x42

    def test_movx_xdata(self, sim):
        # Write 0xBE to XDATA[0] via MOVX @R0, A
        prog = bytes([
            0x78, 0x00,   # MOV R0, #0    (XDATA address 0)
            0x74, 0xBE,   # MOV A, #0xBE
            0xF2,         # MOVX @R0, A   (write)
            0x74, 0x00,   # MOV A, #0     (clear A)
            0xE2,         # MOVX A, @R0   (read back)
            HALT,
        ])
        result = sim.execute(prog)
        assert result.final_state.acc == 0xBE

    def test_execute_with_origin(self, sim):
        # Load program at offset 0x100
        prog = bytes([
            0x74, 0x55,   # MOV A, #0x55
            HALT,
        ])
        result = sim.execute(prog, origin=0x100)
        assert result.halted
        assert result.final_state.acc == 0x55

    def test_reset_clears_state(self, sim):
        prog = bytes([0x74, 0x42, HALT])
        sim.execute(prog)
        sim.reset()
        state = sim.get_state()
        assert state.acc == 0

    def test_port_operations(self, sim):
        sim.set_input_port(0, 0xAB)
        assert sim.get_output_port(0) == 0xAB
        sim.set_input_port(3, 0xCD)
        assert sim.get_output_port(3) == 0xCD

    def test_nop(self, sim):
        prog = bytes([0x00, 0x00, 0x00, HALT])  # 3 NOPs then HALT
        result = sim.execute(prog)
        assert result.halted
        assert result.steps == 4

    def test_step_halted(self, sim):
        prog = bytes([HALT])
        sim.execute(prog)
        # Already halted — step should return HALT trace
        trace = sim.step()
        assert trace.mnemonic == "HALT"

    def test_load_too_large_raises(self, sim):
        with pytest.raises(ValueError):
            sim.load(bytes(65537))

    def test_get_state_after_load(self, sim):
        prog = bytes([0x74, 0x42, HALT])
        sim.load(prog)
        state = sim.get_state()
        assert state.pc == 0
        assert not state.halted

    def test_rrc_chain(self, sim):
        prog = bytes([
            0xC3,         # CLR C (CY=0)
            0x74, 0x01,   # MOV A, #0x01
            0x13,         # RRC A → bit 0 goes to CY, CY goes to bit 7: A=0x00, CY=1
            0x13,         # RRC A → CY (1) → bit 7: A=0x80, CY=0
            HALT,
        ])
        result = sim.execute(prog)
        assert result.final_state.acc == 0x80
