"""Tests for the Intel 4004 gate-level simulator.

These tests verify that every instruction works correctly when routed
through real logic gates. The test structure mirrors the behavioral
simulator's tests — same programs, same expected results.
"""

from intel4004_gatelevel import Intel4004GateLevel

# ===================================================================
# Basic instructions
# ===================================================================


class TestNOP:
    def test_nop_does_nothing(self) -> None:
        cpu = Intel4004GateLevel()
        traces = cpu.run(bytes([0x00, 0x01]))
        assert cpu.accumulator == 0
        assert traces[0].mnemonic == "NOP"

    def test_multiple_nops(self) -> None:
        cpu = Intel4004GateLevel()
        traces = cpu.run(bytes([0x00, 0x00, 0x00, 0x01]))
        assert len(traces) == 4


class TestHLT:
    def test_hlt_stops(self) -> None:
        cpu = Intel4004GateLevel()
        traces = cpu.run(bytes([0x01]))
        assert cpu.halted is True
        assert len(traces) == 1


class TestLDM:
    def test_ldm_values(self) -> None:
        for n in range(16):
            cpu = Intel4004GateLevel()
            cpu.run(bytes([0xD0 | n, 0x01]))
            assert cpu.accumulator == n


class TestLD:
    def test_ld_reads_register(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xD7, 0xB0, 0xA0, 0x01]))  # LDM 7, XCH R0, LD R0
        assert cpu.accumulator == 7


class TestXCH:
    def test_xch_swaps(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xD7, 0xB0, 0x01]))
        assert cpu.registers[0] == 7
        assert cpu.accumulator == 0


class TestINC:
    def test_inc_wraps(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xDF, 0xB0, 0x60, 0x01]))  # LDM 15, XCH R0, INC R0
        assert cpu.registers[0] == 0

    def test_inc_no_carry(self) -> None:
        cpu = Intel4004GateLevel()
        # Set carry, then INC — carry should stay
        cpu.run(bytes([0xDF, 0xB1, 0xDF, 0x81, 0x60, 0x01]))
        assert cpu.carry is True


# ===================================================================
# Arithmetic
# ===================================================================


class TestADD:
    def test_add_basic(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xD3, 0xB0, 0xD2, 0x80, 0x01]))
        assert cpu.accumulator == 5
        assert cpu.carry is False

    def test_add_overflow(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xD1, 0xB0, 0xDF, 0x80, 0x01]))
        assert cpu.accumulator == 0
        assert cpu.carry is True

    def test_add_carry_in(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([
            0xDF, 0xB0, 0xDF, 0x80,  # 15+15 → carry=1
            0xD1, 0xB1, 0xD1, 0x81,  # 1+1+carry = 3
            0x01,
        ]))
        assert cpu.accumulator == 3


class TestSUB:
    def test_sub_basic(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xD3, 0xB0, 0xD5, 0x90, 0x01]))
        assert cpu.accumulator == 2
        assert cpu.carry is True

    def test_sub_underflow(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xD1, 0xB0, 0xD0, 0x90, 0x01]))
        assert cpu.accumulator == 15
        assert cpu.carry is False


# ===================================================================
# Accumulator operations
# ===================================================================


class TestAccumOps:
    def test_clb(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xDF, 0xB0, 0xDF, 0x80, 0xF0, 0x01]))
        assert cpu.accumulator == 0
        assert cpu.carry is False

    def test_clc(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xDF, 0xB0, 0xDF, 0x80, 0xF1, 0x01]))
        assert cpu.carry is False

    def test_iac(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xD5, 0xF2, 0x01]))
        assert cpu.accumulator == 6

    def test_iac_overflow(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xDF, 0xF2, 0x01]))
        assert cpu.accumulator == 0
        assert cpu.carry is True

    def test_cmc(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xF3, 0x01]))
        assert cpu.carry is True

    def test_cma(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xD5, 0xF4, 0x01]))
        assert cpu.accumulator == 10

    def test_ral(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xD5, 0xF5, 0x01]))  # 0101 → 1010
        assert cpu.accumulator == 0b1010

    def test_rar(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xD4, 0xF6, 0x01]))  # 0100 → 0010
        assert cpu.accumulator == 2

    def test_tcc(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xFA, 0xF7, 0x01]))
        assert cpu.accumulator == 1
        assert cpu.carry is False

    def test_dac(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xD5, 0xF8, 0x01]))
        assert cpu.accumulator == 4
        assert cpu.carry is True

    def test_dac_zero(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xD0, 0xF8, 0x01]))
        assert cpu.accumulator == 15
        assert cpu.carry is False

    def test_tcs(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xFA, 0xF9, 0x01]))
        assert cpu.accumulator == 10

    def test_stc(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xFA, 0x01]))
        assert cpu.carry is True

    def test_daa(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xDC, 0xFB, 0x01]))
        assert cpu.accumulator == 2
        assert cpu.carry is True

    def test_kbp_all_values(self) -> None:
        expected = {0: 0, 1: 1, 2: 2, 4: 3, 8: 4, 3: 15, 15: 15}
        for inp, out in expected.items():
            cpu = Intel4004GateLevel()
            cpu.run(bytes([0xD0 | inp, 0xFC, 0x01]))
            assert cpu.accumulator == out, (
                f"KBP({inp})={cpu.accumulator}, expected {out}"
            )

    def test_dcl(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xD2, 0xFD, 0x01]))
        assert cpu.ram_bank == 2


# ===================================================================
# Jump instructions
# ===================================================================


class TestJumps:
    def test_jun(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0x40, 0x04, 0xD5, 0x01, 0x01]))
        assert cpu.accumulator == 0  # LDM 5 skipped

    def test_jcn_zero(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0x14, 0x04, 0xD5, 0x01, 0x01]))
        assert cpu.accumulator == 0  # A==0 → jump

    def test_jcn_nonzero_no_jump(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xD3, 0x14, 0x06, 0xD5, 0x01, 0x01, 0x01]))
        assert cpu.accumulator == 5

    def test_jcn_invert(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xD3, 0x1C, 0x06, 0xD5, 0x01, 0x01, 0x01]))
        assert cpu.accumulator == 3  # A!=0 → jump (invert zero test)

    def test_isz_loop(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xDE, 0xB0, 0x70, 0x02, 0x01]))
        assert cpu.registers[0] == 0


# ===================================================================
# Subroutines
# ===================================================================


class TestSubroutines:
    def test_jms_bbl(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([
            0x50, 0x04,  # JMS 0x004
            0x01,        # HLT (returned here)
            0x00,        # padding
            0xC5,        # BBL 5
        ]))
        assert cpu.accumulator == 5

    def test_nested(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([
            0x50, 0x06,  # JMS sub1
            0xB0, 0x01,  # XCH R0, HLT
            0x00, 0x00,  # padding
            0x50, 0x0C,  # sub1: JMS sub2
            0xB1,        # XCH R1
            0xD9, 0xC0,  # LDM 9, BBL 0
            0x00,        # padding
            0xC3,        # sub2: BBL 3
        ]))
        assert cpu.registers[1] == 3


# ===================================================================
# Register pairs
# ===================================================================


class TestPairs:
    def test_fim(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0x20, 0xAB, 0x01]))
        assert cpu.registers[0] == 0xA
        assert cpu.registers[1] == 0xB

    def test_src_wrm_rdm(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([
            0x20, 0x00, 0x21, 0xD7, 0xE0,  # SRC P0, LDM 7, WRM
            0xD0,                            # LDM 0
            0x20, 0x00, 0x21, 0xE9,          # SRC P0, RDM
            0x01,
        ]))
        assert cpu.accumulator == 7

    def test_jin(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0x22, 0x06, 0x33, 0xD5, 0x01, 0x00, 0x01]))
        assert cpu.accumulator == 0  # LDM 5 skipped


# ===================================================================
# RAM I/O
# ===================================================================


class TestRAMIO:
    def test_status_write_read(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([
            0x20, 0x00, 0x21,  # SRC P0
            0xD3, 0xE4,        # LDM 3, WR0
            0xD0,              # LDM 0
            0x20, 0x00, 0x21,  # SRC P0
            0xEC,              # RD0
            0x01,
        ]))
        assert cpu.accumulator == 3

    def test_wrr_rdr(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xDB, 0xE2, 0xD0, 0xEA, 0x01]))
        assert cpu.accumulator == 11

    def test_ram_banking(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([
            0xD0, 0xFD,        # DCL bank 0
            0x20, 0x00, 0x21,  # SRC P0
            0xD5, 0xE0,        # LDM 5, WRM
            0xD1, 0xFD,        # DCL bank 1
            0x20, 0x00, 0x21,
            0xD9, 0xE0,        # LDM 9, WRM
            0xD0, 0xFD,        # DCL bank 0
            0x20, 0x00, 0x21,
            0xE9,              # RDM
            0x01,
        ]))
        assert cpu.accumulator == 5


# ===================================================================
# End-to-end programs
# ===================================================================


class TestEndToEnd:
    def test_x_equals_1_plus_2(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01]))
        assert cpu.registers[1] == 3
        assert cpu.halted is True

    def test_multiply_3x4(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([
            0xD3, 0xB0, 0xDC, 0xB1,
            0xD0, 0x80, 0x71, 0x05,
            0xB2, 0x01,
        ]))
        assert cpu.registers[2] == 12

    def test_bcd_7_plus_8(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([
            0xD8, 0xB0, 0xD7, 0x80, 0xFB, 0x01,
        ]))
        assert cpu.accumulator == 5
        assert cpu.carry is True

    def test_countdown(self) -> None:
        cpu = Intel4004GateLevel()
        cpu.run(bytes([0xD5, 0xF8, 0x1C, 0x01, 0x01]))
        assert cpu.accumulator == 0

    def test_max_steps(self) -> None:
        cpu = Intel4004GateLevel()
        traces = cpu.run(bytes([0x40, 0x00]), max_steps=10)
        assert len(traces) == 10

    def test_gate_count(self) -> None:
        cpu = Intel4004GateLevel()
        count = cpu.gate_count()
        assert count > 500  # Sanity check


# ===================================================================
# Component tests
# ===================================================================


class TestComponents:
    def test_bits_roundtrip(self) -> None:
        from intel4004_gatelevel.bits import bits_to_int, int_to_bits

        for val in range(16):
            assert bits_to_int(int_to_bits(val, 4)) == val

        for val in range(4096):
            assert bits_to_int(int_to_bits(val, 12)) == val

    def test_alu_add(self) -> None:
        from intel4004_gatelevel.alu import GateALU

        alu = GateALU()
        result, carry = alu.add(5, 3, 0)
        assert result == 8
        assert not carry

    def test_alu_sub(self) -> None:
        from intel4004_gatelevel.alu import GateALU

        alu = GateALU()
        result, carry = alu.subtract(5, 3, 1)
        assert result == 2
        assert carry  # no borrow

    def test_register_file(self) -> None:
        from intel4004_gatelevel.registers import RegisterFile

        rf = RegisterFile()
        rf.write(5, 11)
        assert rf.read(5) == 11
        assert rf.read(0) == 0

    def test_pc_increment(self) -> None:
        from intel4004_gatelevel.pc import ProgramCounter

        pc = ProgramCounter()
        assert pc.read() == 0
        pc.increment()
        assert pc.read() == 1
        pc.increment()
        assert pc.read() == 2

    def test_stack_push_pop(self) -> None:
        from intel4004_gatelevel.stack import HardwareStack

        stack = HardwareStack()
        stack.push(0x100)
        stack.push(0x200)
        assert stack.pop() == 0x200
        assert stack.pop() == 0x100

    def test_decoder(self) -> None:
        from intel4004_gatelevel.decoder import decode

        d = decode(0xD5)
        assert d.is_ldm == 1
        assert d.immediate == 5

        d = decode(0x80)
        assert d.is_add == 1
        assert d.reg_index == 0
