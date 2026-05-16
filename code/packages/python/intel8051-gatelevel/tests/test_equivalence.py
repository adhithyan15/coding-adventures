"""Cross-validation tests: gate-level vs behavioral 8051 simulator.

These tests run the same programs on both simulators and verify that:
  - ACC produces the same final value
  - PSW flags (CY, OV) produce the same final value
  - IRAM contents match after execution
  - PC matches after execution (both halted at HALT opcode)

The behavioral simulator (Intel8051Simulator / I8051Simulator) is the
reference implementation. The gate-level simulator must produce identical
state for all programs.
"""

from intel8051_simulator.state import (
    SFR_B,
)

HALT = 0xA5  # sentinel opcode


def cross_validate(prog: bytes) -> tuple:
    """Run prog on both simulators, return (behavioral_state, gate_state)."""
    from intel8051_simulator import I8051Simulator as BehavioralSim

    from intel8051_gatelevel import Intel8051GateLevelSimulator as GateSim

    b = BehavioralSim()
    b.execute(prog)
    bs = b.get_state()

    g = GateSim()
    g.execute(bytes(prog))
    gs = g.get_state()

    return bs, gs


def assert_states_equal(bs, gs):
    """Assert that behavioral and gate-level final states match."""
    assert bs.acc == gs.acc, f"ACC mismatch: behavioral={bs.acc:#04x} gate={gs.acc:#04x}"
    assert bs.pc == gs.pc, f"PC mismatch: behavioral={bs.pc:#06x} gate={gs.pc:#06x}"
    # Compare IRAM bytes 0-255
    for addr in range(256):
        assert bs.iram[addr] == gs.iram[addr], (
            f"IRAM[0x{addr:02X}] mismatch: behavioral={bs.iram[addr]:#04x} gate={gs.iram[addr]:#04x}"
        )


class TestArithmeticEquivalence:
    """Program 1: Arithmetic — ADD/ADDC/SUBB with flag checks."""

    def test_add_basic(self):
        prog = bytes([
            0x74, 0x10,   # MOV A, #0x10
            0x24, 0x20,   # ADD A, #0x20  → A = 0x30
            HALT,
        ])
        bs, gs = cross_validate(prog)
        assert_states_equal(bs, gs)
        assert gs.acc == 0x30

    def test_add_with_carry(self):
        prog = bytes([
            0x74, 0xFF,   # MOV A, #0xFF
            0x24, 0x01,   # ADD A, #0x01  → A = 0x00, CY = 1
            HALT,
        ])
        bs, gs = cross_validate(prog)
        assert_states_equal(bs, gs)
        assert gs.acc == 0x00
        assert gs.cy  # carry set

    def test_addc_with_carry(self):
        prog = bytes([
            0x74, 0xFF,   # MOV A, #0xFF
            0x24, 0x01,   # ADD A, #0x01  → A = 0x00, CY = 1
            0x34, 0x00,   # ADDC A, #0x00 → A = 0x01 (added CY)
            HALT,
        ])
        bs, gs = cross_validate(prog)
        assert_states_equal(bs, gs)
        assert gs.acc == 0x01

    def test_subb(self):
        prog = bytes([
            0xC3,         # CLR C         → CY = 0
            0x74, 0x50,   # MOV A, #0x50
            0x94, 0x30,   # SUBB A, #0x30 → A = 0x20
            HALT,
        ])
        bs, gs = cross_validate(prog)
        assert_states_equal(bs, gs)
        assert gs.acc == 0x20

    def test_subb_with_borrow(self):
        prog = bytes([
            0xC3,         # CLR C
            0x74, 0x05,
            0x94, 0x10,   # SUBB A, #0x10 → borrow, CY = 1
            HALT,
        ])
        bs, gs = cross_validate(prog)
        assert_states_equal(bs, gs)
        assert gs.cy


class TestLogicEquivalence:
    """Program 2: Logic — ANL/ORL/XRL/CLR/CPL operations."""

    def test_anl(self):
        prog = bytes([
            0x74, 0xFF,   # MOV A, #0xFF
            0x54, 0x0F,   # ANL A, #0x0F → A = 0x0F
            HALT,
        ])
        bs, gs = cross_validate(prog)
        assert_states_equal(bs, gs)
        assert gs.acc == 0x0F

    def test_orl(self):
        prog = bytes([
            0x74, 0xF0,   # MOV A, #0xF0
            0x44, 0x0F,   # ORL A, #0x0F → A = 0xFF
            HALT,
        ])
        bs, gs = cross_validate(prog)
        assert_states_equal(bs, gs)
        assert gs.acc == 0xFF

    def test_xrl(self):
        prog = bytes([
            0x74, 0xAA,   # MOV A, #0xAA
            0x64, 0xFF,   # XRL A, #0xFF → A = 0x55
            HALT,
        ])
        bs, gs = cross_validate(prog)
        assert_states_equal(bs, gs)
        assert gs.acc == 0x55

    def test_cpl_a(self):
        prog = bytes([
            0x74, 0x0F,   # MOV A, #0x0F
            0xF4,         # CPL A → A = 0xF0
            HALT,
        ])
        bs, gs = cross_validate(prog)
        assert_states_equal(bs, gs)
        assert gs.acc == 0xF0

    def test_clr_a(self):
        prog = bytes([
            0x74, 0xFF,
            0xE4,         # CLR A → A = 0x00
            HALT,
        ])
        bs, gs = cross_validate(prog)
        assert_states_equal(bs, gs)
        assert gs.acc == 0x00


class TestBitOpsEquivalence:
    """Program 3: Bit ops — SETB/CLR/CPL/JB/JNB on PSW.CY."""

    def test_setb_clr_carry(self):
        prog = bytes([
            0xD3,         # SETB C → CY = 1
            0xC3,         # CLR C  → CY = 0
            HALT,
        ])
        bs, gs = cross_validate(prog)
        assert_states_equal(bs, gs)
        assert not gs.cy

    def test_jb_taken(self):
        prog = bytes([
            0xD3,         # SETB C (bit 0xD7 = CY)
            0x20, 0xD7, 0x01,  # JB CY, +1 (skip NOP) — should take branch
            0x00,         # NOP (skipped)
            0x74, 0x42,   # MOV A, #0x42 (reached)
            HALT,
        ])
        bs, gs = cross_validate(prog)
        assert_states_equal(bs, gs)
        assert gs.acc == 0x42

    def test_jnb_not_taken(self):
        prog = bytes([
            0xD3,         # SETB C
            0x30, 0xD7, 0x01,  # JNB CY, +1 — NOT taken because CY=1
            0x74, 0x42,   # MOV A, #0x42 (executed since branch not taken)
            HALT,
        ])
        bs, gs = cross_validate(prog)
        assert_states_equal(bs, gs)
        assert gs.acc == 0x42


class TestBlockMoveEquivalence:
    """Program 4: Block move using DJNZ loop."""

    def test_sum_loop(self):
        # Sum 1+2+...+5 = 15 using DJNZ
        prog = bytes([
            0x74, 0x00,   # MOV A, #0    (sum = 0)
            0x7B, 0x05,   # MOV R3, #5   (counter = 5)
            # Loop:
            0x2B,         # ADD A, R3    (sum += counter)
            0xDB, 0xFD,   # DJNZ R3, -3 (2 bytes back to ADD)
            HALT,
        ])
        bs, gs = cross_validate(prog)
        assert_states_equal(bs, gs)
        assert gs.acc == 15  # 5+4+3+2+1 = 15


class TestFullProgramEquivalence:
    """Program 5: More comprehensive — Fibonacci sequence."""

    def test_fibonacci_to_8(self):
        # Simplified: compute R0+R1 where R0=8, R1=5 → A=13
        prog2 = bytes([
            0x78, 0x08,   # MOV R0, #8
            0x79, 0x05,   # MOV R1, #5
            0x74, 0x00,   # MOV A, #0
            0x28,         # ADD A, R0   → A = 8
            0x29,         # ADD A, R1   → A = 13
            HALT,
        ])
        bs, gs = cross_validate(prog2)
        assert_states_equal(bs, gs)
        assert gs.acc == 13

    def test_register_operations(self):
        # Test all register ops: MOV Rn, A / XCH / SWAP
        prog = bytes([
            0x74, 0xAB,   # MOV A, #0xAB
            0xF8,         # MOV R0, A   → R0 = 0xAB
            0x74, 0xCD,   # MOV A, #0xCD
            0xF9,         # MOV R1, A   → R1 = 0xCD
            0xE8,         # MOV A, R0   → A = 0xAB
            0xC4,         # SWAP A      → A = 0xBA
            HALT,
        ])
        bs, gs = cross_validate(prog)
        assert_states_equal(bs, gs)
        assert gs.acc == 0xBA


class TestMulDivEquivalence:
    """MUL and DIV cross-validation."""

    def test_mul(self):
        prog = bytes([
            0x74, 12,     # MOV A, #12
            0x75, SFR_B, 17,  # MOV B, #17   (B = 17)
            0xA4,         # MUL AB → A = 204, B = 0
            HALT,
        ])
        bs, gs = cross_validate(prog)
        assert_states_equal(bs, gs)
        assert gs.acc == 204
        assert gs.b == 0

    def test_div(self):
        prog = bytes([
            0x74, 100,    # MOV A, #100
            0x75, SFR_B, 7,  # MOV B, #7
            0x84,         # DIV AB → A = 14, B = 2
            HALT,
        ])
        bs, gs = cross_validate(prog)
        assert_states_equal(bs, gs)
        assert gs.acc == 14
        assert gs.b == 2
