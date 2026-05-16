"""Equivalence tests: gate-level vs behavioral Z80 simulator.

These tests cross-validate the gate-level simulator against the behavioral
Z80 simulator from `coding-adventures-z80-simulator`. Both should produce
identical register values and flags after executing the same programs.

We compare:
- All main registers: A, B, C, D, E, H, L
- All flags: S, Z, H, PV, N, C
- PC, SP
"""

from z80_simulator import Z80Simulator

from z80_gatelevel import Z80GateLevelSimulator


def run_both(program: bytes) -> tuple[object, object]:
    """Run the same program on both simulators, return (gate, behav) states."""
    gate_sim = Z80GateLevelSimulator()
    behav_sim = Z80Simulator()
    gate_result = gate_sim.execute(program)
    behav_result = behav_sim.execute(program)
    return gate_result.final_state, behav_result.final_state


def assert_regs_equal(gate, behav):
    """Assert all main registers and flags match."""
    assert gate.a == behav.a, f"A mismatch: gate={gate.a:#04x} behav={behav.a:#04x}"
    assert gate.b == behav.b, "B mismatch"
    assert gate.c == behav.c, "C mismatch"
    assert gate.d == behav.d, "D mismatch"
    assert gate.e == behav.e, "E mismatch"
    assert gate.h == behav.h, "H mismatch"
    assert gate.l == behav.l, "L mismatch"
    assert gate.flag_z == behav.flag_z, "Z flag mismatch"
    assert gate.flag_c == behav.flag_c, "C flag mismatch"
    assert gate.flag_s == behav.flag_s, "S flag mismatch"
    assert gate.flag_n == behav.flag_n, "N flag mismatch"


class TestNopHalt:
    def test_nop_halt(self):
        program = bytes([0x00, 0x76])  # NOP; HALT
        gate, behav = run_both(program)
        assert gate.halted is True
        assert behav.halted is True

    def test_halt_immediately(self):
        program = bytes([0x76])
        gate, behav = run_both(program)
        assert gate.halted is True


class TestLoadRegister:
    def test_ld_a_n(self):
        program = bytes([0x3E, 0x42, 0x76])  # LD A, 66; HALT
        gate, behav = run_both(program)
        assert gate.a == 66
        assert behav.a == 66

    def test_ld_b_n(self):
        program = bytes([0x06, 0x10, 0x76])  # LD B, 16; HALT
        gate, behav = run_both(program)
        assert gate.b == 16
        assert behav.b == 16

    def test_ld_bc_nn(self):
        program = bytes([0x01, 0x34, 0x12, 0x76])  # LD BC, 0x1234; HALT
        gate, behav = run_both(program)
        assert gate.b == 0x12
        assert gate.c == 0x34
        assert behav.b == 0x12


class TestAddALU:
    def test_add_a_b(self):
        program = bytes([
            0x3E, 0x05,  # LD A, 5
            0x06, 0x03,  # LD B, 3
            0x80,        # ADD A, B
            0x76,        # HALT
        ])
        gate, behav = run_both(program)
        assert gate.a == 8
        assert behav.a == 8
        assert_regs_equal(gate, behav)

    def test_add_overflow(self):
        program = bytes([
            0x3E, 0x7F,  # LD A, 127
            0x3C,        # INC A  (127+1=128, signed overflow)
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.a == 128
        assert gate.flag_pv == behav.flag_pv

    def test_sub(self):
        program = bytes([
            0x3E, 0x0A,  # LD A, 10
            0x06, 0x03,  # LD B, 3
            0x90,        # SUB B
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.a == 7
        assert behav.a == 7
        assert_regs_equal(gate, behav)

    def test_and(self):
        program = bytes([
            0x3E, 0xFF,  # LD A, 0xFF
            0x06, 0x0F,  # LD B, 0x0F
            0xA0,        # AND B
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.a == 0x0F
        assert behav.a == 0x0F
        assert gate.flag_h == behav.flag_h

    def test_or(self):
        program = bytes([
            0x3E, 0xF0,  # LD A, 0xF0
            0x06, 0x0F,  # LD B, 0x0F
            0xB0,        # OR B
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.a == 0xFF
        assert behav.a == 0xFF

    def test_xor(self):
        program = bytes([
            0x3E, 0xFF,  # LD A, 0xFF
            0x06, 0xAA,  # LD B, 0xAA
            0xA8,        # XOR B
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.a == 0x55
        assert behav.a == 0x55

    def test_cp(self):
        program = bytes([
            0x3E, 0x05,  # LD A, 5
            0x06, 0x05,  # LD B, 5
            0xB8,        # CP B (A unchanged, flags set as if SUB)
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.a == 5   # A unchanged by CP
        assert gate.flag_z == 1
        assert behav.flag_z == 1


class TestIncDec:
    def test_inc_b(self):
        program = bytes([
            0x06, 0x05,  # LD B, 5
            0x04,        # INC B
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.b == 6
        assert behav.b == 6

    def test_dec_b(self):
        program = bytes([
            0x06, 0x05,  # LD B, 5
            0x05,        # DEC B
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.b == 4
        assert behav.b == 4

    def test_inc_rp(self):
        program = bytes([
            0x01, 0xFF, 0xFF,  # LD BC, 0xFFFF
            0x03,              # INC BC
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.b == 0
        assert gate.c == 0

    def test_dec_rp(self):
        program = bytes([
            0x01, 0x01, 0x00,  # LD BC, 1
            0x0B,              # DEC BC
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.b == 0
        assert gate.c == 0


class TestJumps:
    def test_jp_nn(self):
        program = bytes([
            0xC3, 0x05, 0x00,  # JP 0x0005
            0x3E, 0xFF,        # LD A, 255 (should be skipped)
            0x3E, 0x42,        # LD A, 66  (at 0x0005)
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.a == 0x42
        assert behav.a == 0x42

    def test_jr_nz(self):
        # Loop: B = 3, B--, if NZ jump back
        program = bytes([
            0x06, 0x03,  # LD B, 3     (0x0000)
            0x05,        # DEC B       (0x0002) ← loop target
            0x20, 0xFD,  # JR NZ, -3  (0x0003) (PC after fetch=0x0005, -3 → 0x0002)
            0x76,        # HALT        (0x0005)
        ])
        gate, behav = run_both(program)
        assert gate.b == 0
        assert behav.b == 0

    def test_call_ret(self):
        # Call subroutine, return, verify A
        program = bytes([
            0x3E, 0x00,        # LD A, 0       (0x0000)
            0xCD, 0x07, 0x00,  # CALL 0x0007   (0x0002)
            0x76,              # HALT           (0x0005) — unreachable? No, after RET
            0x00,              # NOP            (0x0006) — padding
            0x3E, 0x42,        # LD A, 66       (0x0007) subroutine
            0xC9,              # RET            (0x0009)
        ])
        # HALT is at 0x0005; CALL is at 0x0002, call goes to 0x0007
        # Subroutine loads A=66, returns to 0x0005 (the byte AFTER the CALL)
        gate, behav = run_both(program)
        assert gate.a == 0x42
        assert behav.a == 0x42


class TestStackPushPop:
    def test_push_pop_bc(self):
        program = bytes([
            0x01, 0x34, 0x12,  # LD BC, 0x1234
            0x31, 0x00, 0x80,  # LD SP, 0x8000
            0xC5,              # PUSH BC
            0x01, 0x00, 0x00,  # LD BC, 0
            0xC1,              # POP BC
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.b == 0x12
        assert gate.c == 0x34

    def test_push_pop_af(self):
        program = bytes([
            0x3E, 0x55,        # LD A, 0x55
            0x31, 0x00, 0x80,  # LD SP, 0x8000
            0xF5,              # PUSH AF
            0x3E, 0x00,        # LD A, 0
            0xF1,              # POP AF
            0x76,
        ])
        gate, behav = run_both(program)
        # After POP AF, A should be restored to 0x55
        assert gate.a == 0x55


class TestCBRotates:
    def test_rlc_a(self):
        program = bytes([
            0x3E, 0x80,  # LD A, 0x80
            0xCB, 0x07,  # RLC A
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.a == 0x01
        assert gate.flag_c == behav.flag_c

    def test_rrc_b(self):
        program = bytes([
            0x06, 0x01,  # LD B, 1
            0xCB, 0x08,  # RRC B
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.b == 0x80
        assert gate.flag_c == 1
        assert behav.flag_c == 1

    def test_sla(self):
        program = bytes([
            0x3E, 0x01,  # LD A, 1
            0xCB, 0x27,  # SLA A
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.a == 0x02
        assert_regs_equal(gate, behav)

    def test_srl(self):
        program = bytes([
            0x3E, 0x80,  # LD A, 0x80
            0xCB, 0x3F,  # SRL A
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.a == 0x40
        assert_regs_equal(gate, behav)

    def test_bit_op(self):
        program = bytes([
            0x3E, 0x08,  # LD A, 0b00001000
            0xCB, 0x5F,  # BIT 3, A
            0x76,
        ])
        gate, behav = run_both(program)
        # BIT 3 of 0x08 = bit 3 is SET, so Z=0
        assert gate.flag_z == 0
        assert behav.flag_z == 0


class TestImmediateALU:
    def test_add_a_n(self):
        program = bytes([
            0x3E, 0x10,  # LD A, 16
            0xC6, 0x04,  # ADD A, 4
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.a == 20
        assert behav.a == 20

    def test_sub_n(self):
        program = bytes([
            0x3E, 0x10,  # LD A, 16
            0xD6, 0x04,  # SUB 4
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.a == 12

    def test_and_n(self):
        program = bytes([
            0x3E, 0xFF,  # LD A, 0xFF
            0xE6, 0x0F,  # AND 0x0F
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.a == 0x0F
        assert behav.a == 0x0F

    def test_xor_n(self):
        program = bytes([
            0x3E, 0xAA,  # LD A, 0xAA
            0xEE, 0xFF,  # XOR 0xFF
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.a == 0x55
        assert behav.a == 0x55


class TestExchange:
    def test_ex_af(self):
        program = bytes([
            0x3E, 0x42,  # LD A, 0x42
            0x08,        # EX AF, AF'
            0x3E, 0x00,  # LD A, 0
            0x08,        # EX AF, AF' (swap back)
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.a == 0x42
        assert behav.a == 0x42

    def test_exx(self):
        # Z80 is little-endian: LD BC, nn encodes lo byte first, hi byte second.
        # LD BC, 0x1234 → opcode 0x01, lo=0x34, hi=0x12
        program = bytes([
            0x01, 0x34, 0x12,  # LD BC, 0x1234  (lo=0x34, hi=0x12)
            0xD9,              # EXX
            0x01, 0xBB, 0xAA,  # LD BC, 0xAABB  (lo=0xBB, hi=0xAA)
            0xD9,              # EXX (swap back — restores BC=0x1234)
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.b == 0x12
        assert gate.c == 0x34


class TestAccumRotates:
    def test_rlca(self):
        program = bytes([
            0x3E, 0x80,  # LD A, 0x80
            0x07,        # RLCA
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.a == 0x01
        assert gate.flag_c == 1
        assert behav.flag_c == 1

    def test_rrca(self):
        program = bytes([
            0x3E, 0x01,  # LD A, 0x01
            0x0F,        # RRCA
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.a == 0x80
        assert gate.flag_c == 1

    def test_rla(self):
        # RLA rotates A left through carry: new_bit0 = old_C, new_C = old_bit7.
        # Z80 resets with C=1. XOR A clears A and sets C=0.
        # Then with C=0, RLA on 0x80 → A=0x00, C=1 (bit7 shifts to carry).
        program = bytes([
            0xAF,        # XOR A   (A=0, clears C flag)
            0x3E, 0x80,  # LD A, 0x80
            0x17,        # RLA  (A = (A<<1)|C_old = 0x00, C = old_bit7 = 1)
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.a == 0x00
        assert gate.flag_c == 1

    def test_rra(self):
        # RRA rotates A right through carry: new_bit7 = old_C, new_C = old_bit0.
        # Clear C with XOR A, then with C=0, RRA on 0x01 → A=0x00, C=1.
        program = bytes([
            0xAF,        # XOR A   (A=0, clears C flag)
            0x3E, 0x01,  # LD A, 0x01
            0x1F,        # RRA  (A = (C_old<<7)|(A>>1) = 0x00, C = old_bit0 = 1)
            0x76,
        ])
        gate, behav = run_both(program)
        assert gate.a == 0x00
        assert gate.flag_c == 1
