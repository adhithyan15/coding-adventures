"""Comprehensive tests for the CLR IL Simulator.

These tests verify every instruction in our CLR IL subset, including edge
cases, error conditions, and end-to-end programs. The test organization
mirrors the instruction categories:

    1. Constant loading (ldc.i4.*)
    2. Local variable access (ldloc.*, stloc.*)
    3. Arithmetic (add, sub, mul, div)
    4. Comparison (ceq, cgt, clt — two-byte opcodes)
    5. Branching (br.s, brfalse.s, brtrue.s)
    6. Miscellaneous (nop, ldnull, ret)
    7. Helper functions (encode_*, assemble_clr)
    8. End-to-end programs
    9. Error cases
"""

import struct

import pytest
from clr_bytecode_disassembler import CLRInstruction, CLRMethodBody

from clr_simulator.simulator import (
    CEQ_BYTE,
    CGT_BYTE,
    CLT_BYTE,
    CLROpcode,
    CLRSimulator,
    assemble_clr,
    encode_ldc_i4,
    encode_ldloc,
    encode_stloc,
)

# ===================================================================
# Fixtures
# ===================================================================


@pytest.fixture
def sim() -> CLRSimulator:
    """Create a fresh CLR simulator for each test."""
    return CLRSimulator()


# ===================================================================
# 1. Constant loading: ldc.i4.0 through ldc.i4.8
# ===================================================================


class TestLdcI4ShortForms:
    """Test the short-form constant loaders (ldc.i4.0 through ldc.i4.8).

    The CLR provides 9 single-byte opcodes for pushing small constants,
    more than the JVM's 6 (iconst_0 through iconst_5). These are the
    most commonly used constants in real programs.
    """

    @pytest.mark.parametrize("value", range(9))
    def test_ldc_i4_short_forms(self, sim: CLRSimulator, value: int) -> None:
        """Each ldc.i4.N pushes the corresponding integer N onto the stack."""
        bytecode = assemble_clr(encode_ldc_i4(value), (CLROpcode.RET,))
        sim.load(bytecode)
        traces = sim.run()

        # The constant should be on the stack before ret
        assert sim.stack == [value]
        # First trace should show the push
        assert traces[0].opcode == f"ldc.i4.{value}"
        assert traces[0].stack_after == [value]
        assert traces[0].description == f"push {value}"

    def test_ldc_i4_0_is_one_byte(self) -> None:
        """ldc.i4.0 encodes as a single byte (0x16)."""
        encoded = encode_ldc_i4(0)
        assert encoded == bytes([0x16])
        assert len(encoded) == 1

    def test_ldc_i4_8_is_one_byte(self) -> None:
        """ldc.i4.8 encodes as a single byte (0x1E)."""
        encoded = encode_ldc_i4(8)
        assert encoded == bytes([0x1E])
        assert len(encoded) == 1


# ===================================================================
# 2. Constant loading: ldc.i4.s (signed int8)
# ===================================================================


class TestLdcI4S:
    """Test the medium-form constant loader (ldc.i4.s).

    ldc.i4.s uses 2 bytes: the opcode (0x1F) + a signed int8 value.
    It handles values from -128 to 127 that don't have short forms.
    """

    def test_ldc_i4_s_positive(self, sim: CLRSimulator) -> None:
        """ldc.i4.s can push positive values (9-127)."""
        bytecode = assemble_clr(encode_ldc_i4(42), (CLROpcode.RET,))
        sim.load(bytecode)
        traces = sim.run()
        assert sim.stack == [42]
        assert traces[0].opcode == "ldc.i4.s"

    def test_ldc_i4_s_negative(self, sim: CLRSimulator) -> None:
        """ldc.i4.s can push negative values (-128 to -1)."""
        bytecode = assemble_clr(encode_ldc_i4(-1), (CLROpcode.RET,))
        sim.load(bytecode)
        traces = sim.run()
        assert sim.stack == [-1]
        assert traces[0].opcode == "ldc.i4.s"

    def test_ldc_i4_s_min_value(self, sim: CLRSimulator) -> None:
        """ldc.i4.s can push -128 (minimum signed int8)."""
        bytecode = assemble_clr(encode_ldc_i4(-128), (CLROpcode.RET,))
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [-128]

    def test_ldc_i4_s_max_value(self, sim: CLRSimulator) -> None:
        """ldc.i4.s can push 127 (maximum signed int8)."""
        bytecode = assemble_clr(encode_ldc_i4(127), (CLROpcode.RET,))
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [127]

    def test_ldc_i4_s_encoding(self) -> None:
        """ldc.i4.s is exactly 2 bytes: opcode + signed int8."""
        encoded = encode_ldc_i4(42)
        assert len(encoded) == 2
        assert encoded[0] == 0x1F  # ldc.i4.s opcode
        assert encoded[1] == 42

    def test_ldc_i4_s_negative_encoding(self) -> None:
        """ldc.i4.s encodes negative values as two's complement."""
        encoded = encode_ldc_i4(-1)
        assert len(encoded) == 2
        assert encoded[0] == 0x1F
        assert encoded[1] == 0xFF  # -1 as unsigned byte


# ===================================================================
# 3. Constant loading: ldc.i4 (int32)
# ===================================================================


class TestLdcI4:
    """Test the general-form constant loader (ldc.i4).

    ldc.i4 uses 5 bytes: the opcode (0x20) + a little-endian signed int32.
    It handles any 32-bit integer value.
    """

    def test_ldc_i4_large_positive(self, sim: CLRSimulator) -> None:
        """ldc.i4 can push large positive values."""
        bytecode = assemble_clr(encode_ldc_i4(1000), (CLROpcode.RET,))
        sim.load(bytecode)
        traces = sim.run()
        assert sim.stack == [1000]
        assert traces[0].opcode == "ldc.i4"

    def test_ldc_i4_large_negative(self, sim: CLRSimulator) -> None:
        """ldc.i4 can push large negative values."""
        bytecode = assemble_clr(encode_ldc_i4(-1000), (CLROpcode.RET,))
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [-1000]

    def test_ldc_i4_max_int32(self, sim: CLRSimulator) -> None:
        """ldc.i4 can push the maximum int32 value."""
        bytecode = assemble_clr(encode_ldc_i4(2_147_483_647), (CLROpcode.RET,))
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [2_147_483_647]

    def test_ldc_i4_min_int32(self, sim: CLRSimulator) -> None:
        """ldc.i4 can push the minimum int32 value."""
        bytecode = assemble_clr(encode_ldc_i4(-2_147_483_648), (CLROpcode.RET,))
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [-2_147_483_648]

    def test_ldc_i4_encoding(self) -> None:
        """ldc.i4 is exactly 5 bytes: opcode + LE int32."""
        encoded = encode_ldc_i4(1000)
        assert len(encoded) == 5
        assert encoded[0] == 0x20  # ldc.i4 opcode
        # 1000 in little-endian: 0xE8 0x03 0x00 0x00
        assert encoded[1:] == struct.pack("<i", 1000)

    def test_ldc_i4_value_128(self, sim: CLRSimulator) -> None:
        """Value 128 requires ldc.i4 (doesn't fit in signed int8)."""
        bytecode = assemble_clr(encode_ldc_i4(128), (CLROpcode.RET,))
        sim.load(bytecode)
        traces = sim.run()
        assert sim.stack == [128]
        # 128 > 127, so it can't use ldc.i4.s — must use ldc.i4
        assert traces[0].opcode == "ldc.i4"

    def test_ldc_i4_value_minus_129(self, sim: CLRSimulator) -> None:
        """Value -129 requires ldc.i4 (doesn't fit in signed int8)."""
        bytecode = assemble_clr(encode_ldc_i4(-129), (CLROpcode.RET,))
        sim.load(bytecode)
        traces = sim.run()
        assert sim.stack == [-129]
        assert traces[0].opcode == "ldc.i4"


# ===================================================================
# 4. Local variables: ldloc and stloc (short forms)
# ===================================================================


class TestLocalVariablesShortForms:
    """Test ldloc.0-3 and stloc.0-3 (single-byte local access)."""

    @pytest.mark.parametrize("slot", range(4))
    def test_stloc_short_form(self, sim: CLRSimulator, slot: int) -> None:
        """stloc.N stores the top of stack into local variable N."""
        bytecode = assemble_clr(
            encode_ldc_i4(42),
            encode_stloc(slot),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.locals[slot] == 42

    @pytest.mark.parametrize("slot", range(4))
    def test_ldloc_short_form(self, sim: CLRSimulator, slot: int) -> None:
        """ldloc.N pushes the value of local variable N onto the stack."""
        bytecode = assemble_clr(
            encode_ldc_i4(99),
            encode_stloc(slot),
            encode_ldloc(slot),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [99]

    def test_stloc_ldloc_roundtrip(self, sim: CLRSimulator) -> None:
        """Store a value and load it back — the value should be preserved."""
        bytecode = assemble_clr(
            encode_ldc_i4(7),
            encode_stloc(0),
            encode_ldloc(0),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        traces = sim.run()
        assert sim.stack == [7]
        # Verify trace mnemonics
        assert traces[0].opcode == "ldc.i4.7"
        assert traces[1].opcode == "stloc.0"
        assert traces[2].opcode == "ldloc.0"


# ===================================================================
# 5. Local variables: ldloc.s and stloc.s (generic forms)
# ===================================================================


class TestLocalVariablesGenericForms:
    """Test ldloc.s and stloc.s (two-byte local access for slots > 3)."""

    def test_stloc_s(self, sim: CLRSimulator) -> None:
        """stloc.s can store to any local variable slot."""
        bytecode = assemble_clr(
            encode_ldc_i4(55),
            encode_stloc(10),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.locals[10] == 55

    def test_ldloc_s(self, sim: CLRSimulator) -> None:
        """ldloc.s can load from any local variable slot."""
        bytecode = assemble_clr(
            encode_ldc_i4(77),
            encode_stloc(10),
            encode_ldloc(10),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [77]

    def test_stloc_s_encoding(self) -> None:
        """stloc.s for slot 10 is 2 bytes: 0x13 0x0A."""
        encoded = encode_stloc(10)
        assert encoded == bytes([0x13, 0x0A])

    def test_ldloc_s_encoding(self) -> None:
        """ldloc.s for slot 10 is 2 bytes: 0x11 0x0A."""
        encoded = encode_ldloc(10)
        assert encoded == bytes([0x11, 0x0A])

    def test_stloc_trace(self, sim: CLRSimulator) -> None:
        """stloc.s trace shows the slot and value."""
        bytecode = assemble_clr(
            encode_ldc_i4(33),
            encode_stloc(5),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        traces = sim.run()
        assert traces[1].opcode == "stloc.s"
        assert "locals[5]" in traces[1].description

    def test_ldloc_trace(self, sim: CLRSimulator) -> None:
        """ldloc.s trace shows the slot and value."""
        bytecode = assemble_clr(
            encode_ldc_i4(33),
            encode_stloc(5),
            encode_ldloc(5),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        traces = sim.run()
        assert traces[2].opcode == "ldloc.s"
        assert "locals[5]" in traces[2].description


# ===================================================================
# 6. Arithmetic: add, sub, mul, div
# ===================================================================


class TestArithmetic:
    """Test the type-inferred arithmetic instructions.

    Unlike the JVM (which has iadd, ladd, fadd, dadd), the CLR has just
    one `add` opcode — the type is inferred from the stack. Our simulator
    only handles int32 arithmetic.
    """

    def test_add(self, sim: CLRSimulator) -> None:
        """add pops two values and pushes their sum."""
        bytecode = assemble_clr(
            encode_ldc_i4(3),
            encode_ldc_i4(4),
            (CLROpcode.ADD,),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [7]

    def test_sub(self, sim: CLRSimulator) -> None:
        """sub pops two values and pushes their difference (a - b)."""
        bytecode = assemble_clr(
            encode_ldc_i4(10),
            encode_ldc_i4(3),
            (CLROpcode.SUB,),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [7]

    def test_mul(self, sim: CLRSimulator) -> None:
        """mul pops two values and pushes their product."""
        bytecode = assemble_clr(
            encode_ldc_i4(6),
            encode_ldc_i4(7),
            (CLROpcode.MUL,),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [42]

    def test_div(self, sim: CLRSimulator) -> None:
        """div pops two values and pushes their quotient (integer division)."""
        bytecode = assemble_clr(
            encode_ldc_i4(10),
            encode_ldc_i4(3),
            (CLROpcode.DIV,),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [3]  # 10 / 3 = 3 (truncated)

    def test_div_by_zero(self, sim: CLRSimulator) -> None:
        """Division by zero raises ZeroDivisionError."""
        bytecode = assemble_clr(
            encode_ldc_i4(10),
            encode_ldc_i4(0),
            (CLROpcode.DIV,),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        with pytest.raises(ZeroDivisionError, match="DivideByZeroException"):
            sim.run()

    def test_div_negative_truncation(self, sim: CLRSimulator) -> None:
        """Division truncates toward zero (C-style), not toward negative infinity."""
        bytecode = assemble_clr(
            encode_ldc_i4(-7),
            encode_ldc_i4(2),
            (CLROpcode.DIV,),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        # C-style truncation: -7 / 2 = -3 (not -4 like Python's //)
        assert sim.stack == [-3]

    def test_add_trace(self, sim: CLRSimulator) -> None:
        """add trace shows the operands and result."""
        bytecode = assemble_clr(
            encode_ldc_i4(3),
            encode_ldc_i4(4),
            (CLROpcode.ADD,),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        traces = sim.run()
        add_trace = traces[2]
        assert add_trace.opcode == "add"
        assert add_trace.stack_before == [3, 4]
        assert add_trace.stack_after == [7]
        assert "pop 4 and 3" in add_trace.description
        assert "push 7" in add_trace.description

    def test_sub_negative_result(self, sim: CLRSimulator) -> None:
        """sub can produce negative results."""
        bytecode = assemble_clr(
            encode_ldc_i4(3),
            encode_ldc_i4(10),
            (CLROpcode.SUB,),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [-7]


# ===================================================================
# 7. Comparison: ceq, cgt, clt (two-byte opcodes)
# ===================================================================


class TestComparisons:
    """Test the two-byte comparison opcodes (0xFE prefix).

    These opcodes pop two values, compare them, and push 1 (true) or
    0 (false). They use the 0xFE prefix because the single-byte opcode
    space was already full with more commonly used instructions.
    """

    def test_ceq_equal(self, sim: CLRSimulator) -> None:
        """ceq pushes 1 when values are equal."""
        bytecode = assemble_clr(
            encode_ldc_i4(5),
            encode_ldc_i4(5),
            (CLROpcode.PREFIX_FE, CEQ_BYTE),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [1]

    def test_ceq_not_equal(self, sim: CLRSimulator) -> None:
        """ceq pushes 0 when values are not equal."""
        bytecode = assemble_clr(
            encode_ldc_i4(5),
            encode_ldc_i4(3),
            (CLROpcode.PREFIX_FE, CEQ_BYTE),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [0]

    def test_cgt_greater(self, sim: CLRSimulator) -> None:
        """cgt pushes 1 when first value is greater."""
        bytecode = assemble_clr(
            encode_ldc_i4(5),
            encode_ldc_i4(3),
            (CLROpcode.PREFIX_FE, CGT_BYTE),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [1]

    def test_cgt_not_greater(self, sim: CLRSimulator) -> None:
        """cgt pushes 0 when first value is not greater."""
        bytecode = assemble_clr(
            encode_ldc_i4(3),
            encode_ldc_i4(5),
            (CLROpcode.PREFIX_FE, CGT_BYTE),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [0]

    def test_cgt_equal(self, sim: CLRSimulator) -> None:
        """cgt pushes 0 when values are equal (not strictly greater)."""
        bytecode = assemble_clr(
            encode_ldc_i4(5),
            encode_ldc_i4(5),
            (CLROpcode.PREFIX_FE, CGT_BYTE),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [0]

    def test_clt_less(self, sim: CLRSimulator) -> None:
        """clt pushes 1 when first value is less."""
        bytecode = assemble_clr(
            encode_ldc_i4(3),
            encode_ldc_i4(5),
            (CLROpcode.PREFIX_FE, CLT_BYTE),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [1]

    def test_clt_not_less(self, sim: CLRSimulator) -> None:
        """clt pushes 0 when first value is not less."""
        bytecode = assemble_clr(
            encode_ldc_i4(5),
            encode_ldc_i4(3),
            (CLROpcode.PREFIX_FE, CLT_BYTE),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [0]

    def test_ceq_trace(self, sim: CLRSimulator) -> None:
        """ceq trace shows the comparison and result."""
        bytecode = assemble_clr(
            encode_ldc_i4(5),
            encode_ldc_i4(5),
            (CLROpcode.PREFIX_FE, CEQ_BYTE),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        traces = sim.run()
        ceq_trace = traces[2]
        assert ceq_trace.opcode == "ceq"
        assert "5 == 5" in ceq_trace.description
        assert ceq_trace.stack_after == [1]


# ===================================================================
# 8. Branching: br.s, brfalse.s, brtrue.s
# ===================================================================


class TestBranching:
    """Test branch instructions with signed int8 offsets.

    Branch offsets in the CLR are relative to the NEXT instruction's PC.
    So br.s at PC=0 with offset=+2 jumps to PC = (0 + 2) + 2 = 4.
    """

    def test_br_s_forward(self, sim: CLRSimulator) -> None:
        """br.s with positive offset jumps forward."""
        # Layout:
        #   PC=0: ldc.i4.1  (1 byte)
        #   PC=1: br.s +1   (2 bytes, skip next instruction)
        #   PC=3: ldc.i4.2  (1 byte, SKIPPED)
        #   PC=4: ret        (1 byte)
        bytecode = assemble_clr(
            encode_ldc_i4(1),
            bytes([CLROpcode.BR_S, 1]),  # skip 1 byte after this instruction
            encode_ldc_i4(2),  # this gets skipped
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        # Only ldc.i4.1 should have been executed, not ldc.i4.2
        assert sim.stack == [1]

    def test_br_s_zero_offset(self, sim: CLRSimulator) -> None:
        """br.s with offset 0 falls through to the next instruction."""
        bytecode = assemble_clr(
            encode_ldc_i4(1),
            bytes([CLROpcode.BR_S, 0]),  # offset 0 = fall through
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [1]

    def test_brfalse_s_taken(self, sim: CLRSimulator) -> None:
        """brfalse.s branches when the value is zero."""
        # Layout:
        #   PC=0: ldc.i4.0  (1 byte) — push 0 (false)
        #   PC=1: brfalse.s +1 (2 bytes) — branch if false, skip next
        #   PC=3: ldc.i4.1  (1 byte, SKIPPED)
        #   PC=4: ret        (1 byte)
        bytecode = assemble_clr(
            encode_ldc_i4(0),
            bytes([CLROpcode.BRFALSE_S, 1]),  # branch taken (value is 0)
            encode_ldc_i4(1),  # skipped
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.stack == []  # ldc.i4.0 was consumed by brfalse

    def test_brfalse_s_not_taken(self, sim: CLRSimulator) -> None:
        """brfalse.s falls through when the value is nonzero."""
        bytecode = assemble_clr(
            encode_ldc_i4(1),
            bytes([CLROpcode.BRFALSE_S, 1]),  # not taken (value is 1)
            encode_ldc_i4(2),  # NOT skipped
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [2]  # ldc.i4.2 was executed

    def test_brtrue_s_taken(self, sim: CLRSimulator) -> None:
        """brtrue.s branches when the value is nonzero."""
        bytecode = assemble_clr(
            encode_ldc_i4(1),
            bytes([CLROpcode.BRTRUE_S, 1]),  # branch taken (value is 1)
            encode_ldc_i4(2),  # skipped
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.stack == []  # ldc.i4.1 was consumed by brtrue

    def test_brtrue_s_not_taken(self, sim: CLRSimulator) -> None:
        """brtrue.s falls through when the value is zero."""
        bytecode = assemble_clr(
            encode_ldc_i4(0),
            bytes([CLROpcode.BRTRUE_S, 1]),  # not taken (value is 0)
            encode_ldc_i4(2),  # NOT skipped
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [2]

    def test_br_s_backward(self, sim: CLRSimulator) -> None:
        """br.s with negative offset jumps backward (loop)."""
        # Build a simple loop: count from 0 to 3
        # Layout:
        #   PC=0: ldc.i4.0    (1 byte)   — push 0 (counter)
        #   PC=1: stloc.0     (1 byte)   — counter = 0
        #   PC=2: ldloc.0     (1 byte)   — push counter   [LOOP START]
        #   PC=3: ldc.i4.1    (1 byte)   — push 1
        #   PC=4: add         (1 byte)   — counter + 1
        #   PC=5: stloc.0     (1 byte)   — store incremented counter
        #   PC=6: ldloc.0     (1 byte)   — push counter
        #   PC=7: ldc.i4.3    (1 byte)   — push 3
        #   PC=8: ceq         (2 bytes)  — counter == 3?
        #   PC=10: brfalse.s -9 (2 bytes) — loop back if not 3
        #   PC=12: ret        (1 byte)
        bytecode = assemble_clr(
            encode_ldc_i4(0),
            encode_stloc(0),
            # Loop start (PC=2)
            encode_ldloc(0),
            encode_ldc_i4(1),
            (CLROpcode.ADD,),
            encode_stloc(0),
            encode_ldloc(0),
            encode_ldc_i4(3),
            (CLROpcode.PREFIX_FE, CEQ_BYTE),
            # brfalse.s: if counter != 3, jump back
            # After brfalse.s (PC=10, size=2), next_pc = 12
            # We want to jump to PC=2, so offset = 2 - 12 = -10
            bytes([CLROpcode.BRFALSE_S, struct.pack("b", -10)[0]]),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.locals[0] == 3

    def test_brfalse_s_trace_taken(self, sim: CLRSimulator) -> None:
        """brfalse.s trace shows 'branch taken' when value is 0."""
        bytecode = assemble_clr(
            encode_ldc_i4(0),
            bytes([CLROpcode.BRFALSE_S, 0]),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        traces = sim.run()
        assert "branch taken" in traces[1].description

    def test_brtrue_s_trace_not_taken(self, sim: CLRSimulator) -> None:
        """brtrue.s trace shows 'branch not taken' when value is 0."""
        bytecode = assemble_clr(
            encode_ldc_i4(0),
            bytes([CLROpcode.BRTRUE_S, 0]),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        traces = sim.run()
        assert "branch not taken" in traces[1].description


# ===================================================================
# 9. Miscellaneous: nop, ldnull, ret
# ===================================================================


class TestMiscellaneous:
    """Test nop, ldnull, and ret instructions."""

    def test_nop(self, sim: CLRSimulator) -> None:
        """nop does nothing — stack and locals are unchanged."""
        bytecode = assemble_clr(
            (CLROpcode.NOP,),
            encode_ldc_i4(1),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        traces = sim.run()
        assert traces[0].opcode == "nop"
        assert traces[0].stack_before == []
        assert traces[0].stack_after == []
        assert sim.stack == [1]

    def test_ldnull(self, sim: CLRSimulator) -> None:
        """ldnull pushes None (null) onto the stack."""
        bytecode = assemble_clr(
            (CLROpcode.LDNULL,),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.stack == [None]

    def test_ret_halts(self, sim: CLRSimulator) -> None:
        """ret halts the simulator."""
        bytecode = assemble_clr((CLROpcode.RET,))
        sim.load(bytecode)
        sim.run()
        assert sim.halted is True

    def test_ret_trace(self, sim: CLRSimulator) -> None:
        """ret trace shows 'return'."""
        bytecode = assemble_clr((CLROpcode.RET,))
        sim.load(bytecode)
        traces = sim.run()
        assert traces[0].opcode == "ret"
        assert traces[0].description == "return"


# ===================================================================
# 10. Helper function tests
# ===================================================================


class TestHelperFunctions:
    """Test the encode_* and assemble_clr helper functions."""

    def test_encode_ldc_i4_short_forms(self) -> None:
        """encode_ldc_i4 uses short forms for 0-8."""
        for n in range(9):
            encoded = encode_ldc_i4(n)
            assert len(encoded) == 1
            assert encoded[0] == CLROpcode.LDC_I4_0 + n

    def test_encode_ldc_i4_medium_form(self) -> None:
        """encode_ldc_i4 uses ldc.i4.s for -128 to -1 and 9 to 127."""
        for n in [9, 10, 50, 100, 127, -1, -50, -128]:
            encoded = encode_ldc_i4(n)
            assert len(encoded) == 2
            assert encoded[0] == CLROpcode.LDC_I4_S

    def test_encode_ldc_i4_general_form(self) -> None:
        """encode_ldc_i4 uses ldc.i4 for values outside int8 range."""
        for n in [128, 256, 1000, -129, -1000]:
            encoded = encode_ldc_i4(n)
            assert len(encoded) == 5
            assert encoded[0] == CLROpcode.LDC_I4

    def test_encode_stloc_short_forms(self) -> None:
        """encode_stloc uses short forms for slots 0-3."""
        for slot in range(4):
            encoded = encode_stloc(slot)
            assert len(encoded) == 1
            assert encoded[0] == CLROpcode.STLOC_0 + slot

    def test_encode_stloc_generic_form(self) -> None:
        """encode_stloc uses stloc.s for slots > 3."""
        encoded = encode_stloc(10)
        assert len(encoded) == 2
        assert encoded[0] == CLROpcode.STLOC_S
        assert encoded[1] == 10

    def test_encode_ldloc_short_forms(self) -> None:
        """encode_ldloc uses short forms for slots 0-3."""
        for slot in range(4):
            encoded = encode_ldloc(slot)
            assert len(encoded) == 1
            assert encoded[0] == CLROpcode.LDLOC_0 + slot

    def test_encode_ldloc_generic_form(self) -> None:
        """encode_ldloc uses ldloc.s for slots > 3."""
        encoded = encode_ldloc(10)
        assert len(encoded) == 2
        assert encoded[0] == CLROpcode.LDLOC_S
        assert encoded[1] == 10

    def test_assemble_clr_tuples(self) -> None:
        """assemble_clr handles tuples of ints."""
        bytecode = assemble_clr(
            (CLROpcode.LDC_I4_1,),
            (CLROpcode.RET,),
        )
        assert bytecode == bytes([0x17, 0x2A])

    def test_assemble_clr_bytes(self) -> None:
        """assemble_clr handles bytes objects."""
        bytecode = assemble_clr(
            encode_ldc_i4(42),
            bytes([CLROpcode.RET]),
        )
        assert bytecode == bytes([0x1F, 42, 0x2A])

    def test_assemble_clr_mixed(self) -> None:
        """assemble_clr handles a mix of tuples and bytes."""
        bytecode = assemble_clr(
            encode_ldc_i4(1),
            (CLROpcode.RET,),
        )
        assert bytecode == bytes([0x17, 0x2A])


# ===================================================================
# 11. End-to-end programs
# ===================================================================


class TestEndToEnd:
    """Test complete programs that exercise multiple instructions together."""

    def test_x_equals_1_plus_2(self, sim: CLRSimulator) -> None:
        """Classic test: compute x = 1 + 2, store in local 0.

        This is the CLR equivalent of "the first program" — the simplest
        meaningful computation: push two constants, add them, store the result.

        CLR IL:
            ldc.i4.1    push 1
            ldc.i4.2    push 2
            add         pop 2 and 1, push 3
            stloc.0     pop 3, store in local 0
            ret         halt
        """
        bytecode = assemble_clr(
            encode_ldc_i4(1),
            encode_ldc_i4(2),
            (CLROpcode.ADD,),
            encode_stloc(0),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        traces = sim.run()

        assert sim.locals[0] == 3
        assert len(traces) == 5
        assert sim.halted is True

    def test_x_equals_3_plus_4_times_2(self, sim: CLRSimulator) -> None:
        """Compute x = (3 + 4) * 2, store in local 0.

        CLR IL:
            ldc.i4.3    push 3
            ldc.i4.4    push 4
            add         pop 4 and 3, push 7
            ldc.i4.2    push 2
            mul         pop 2 and 7, push 14
            stloc.0     pop 14, store in local 0
            ret         halt
        """
        bytecode = assemble_clr(
            encode_ldc_i4(3),
            encode_ldc_i4(4),
            (CLROpcode.ADD,),
            encode_ldc_i4(2),
            (CLROpcode.MUL,),
            encode_stloc(0),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.locals[0] == 14

    def test_swap_two_locals(self, sim: CLRSimulator) -> None:
        """Swap local 0 and local 1 using the stack as temporary storage.

        This demonstrates the stack-based approach to variable manipulation:
        load both values, then store them in reverse order.

        CLR IL:
            ldc.i4.5    push 5
            stloc.0     local[0] = 5
            ldc.i4.8    push 8
            stloc.1     local[1] = 8
            ldloc.0     push local[0] (= 5)
            ldloc.1     push local[1] (= 8)
            stloc.0     local[0] = 8 (top of stack)
            stloc.1     local[1] = 5 (remaining value)
            ret
        """
        bytecode = assemble_clr(
            encode_ldc_i4(5),
            encode_stloc(0),
            encode_ldc_i4(8),
            encode_stloc(1),
            encode_ldloc(0),
            encode_ldloc(1),
            encode_stloc(0),
            encode_stloc(1),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        sim.run()
        assert sim.locals[0] == 8
        assert sim.locals[1] == 5

    def test_conditional_max(self, sim: CLRSimulator) -> None:
        """Compute max(5, 3) using cgt and brfalse.s.

        CLR IL:
            ldc.i4.5    push 5
            stloc.0     a = 5
            ldc.i4.3    push 3
            stloc.1     b = 3
            ldloc.0     push a
            ldloc.1     push b
            cgt         a > b? push 1 or 0
            brfalse.s +2 if false, skip to 'b is max'
            ldloc.0     push a (a is max)
            br.s +1     skip over ldloc.1
            ldloc.1     push b (b is max)
            stloc.2     store max in local 2
            ret
        """
        bytecode = assemble_clr(
            encode_ldc_i4(5),       # PC=0: push 5 (1 byte)
            encode_stloc(0),        # PC=1: a = 5 (1 byte)
            encode_ldc_i4(3),       # PC=2: push 3 (1 byte)
            encode_stloc(1),        # PC=3: b = 3 (1 byte)
            encode_ldloc(0),        # PC=4: push a (1 byte)
            encode_ldloc(1),        # PC=5: push b (1 byte)
            (CLROpcode.PREFIX_FE, CGT_BYTE),  # PC=6: cgt (2 bytes)
            bytes([CLROpcode.BRFALSE_S, 3]),        # PC=8: brfalse.s +3 (2 bytes)
            encode_ldloc(0),        # PC=10: push a (1 byte) — a is max
            bytes([CLROpcode.BR_S, 1]),  # PC=11: br.s +1 (2 bytes)
            encode_ldloc(1),        # PC=13: push b (1 byte) — b is max
            encode_stloc(2),        # PC=14: store max (1 byte)
            (CLROpcode.RET,),       # PC=15: ret (1 byte)
        )
        sim.load(bytecode)
        sim.run()
        assert sim.locals[2] == 5  # max(5, 3) = 5

    def test_trace_verification(self, sim: CLRSimulator) -> None:
        """Verify that traces accurately capture state transitions."""
        bytecode = assemble_clr(
            encode_ldc_i4(1),
            encode_ldc_i4(2),
            (CLROpcode.ADD,),
            encode_stloc(0),
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        traces = sim.run()

        # Trace 0: ldc.i4.1
        assert traces[0].pc == 0
        assert traces[0].opcode == "ldc.i4.1"
        assert traces[0].stack_before == []
        assert traces[0].stack_after == [1]

        # Trace 1: ldc.i4.2
        assert traces[1].pc == 1
        assert traces[1].opcode == "ldc.i4.2"
        assert traces[1].stack_before == [1]
        assert traces[1].stack_after == [1, 2]

        # Trace 2: add
        assert traces[2].pc == 2
        assert traces[2].opcode == "add"
        assert traces[2].stack_before == [1, 2]
        assert traces[2].stack_after == [3]

        # Trace 3: stloc.0
        assert traces[3].pc == 3
        assert traces[3].opcode == "stloc.0"
        assert traces[3].stack_before == [3]
        assert traces[3].stack_after == []
        assert traces[3].locals_snapshot[0] == 3

        # Trace 4: ret
        assert traces[4].pc == 4
        assert traces[4].opcode == "ret"


# ===================================================================
# 12. Error cases
# ===================================================================


class TestErrorCases:
    """Test error handling for invalid operations."""

    def test_step_after_halt(self, sim: CLRSimulator) -> None:
        """Stepping after ret raises RuntimeError."""
        bytecode = assemble_clr((CLROpcode.RET,))
        sim.load(bytecode)
        sim.run()
        with pytest.raises(RuntimeError, match="halted"):
            sim.step()

    def test_unknown_opcode(self, sim: CLRSimulator) -> None:
        """Unknown opcodes raise ValueError."""
        sim.load(bytes([0xFF]))  # Not a valid opcode
        with pytest.raises(ValueError, match="Unknown CLR opcode"):
            sim.step()

    def test_unknown_two_byte_opcode(self, sim: CLRSimulator) -> None:
        """Unknown two-byte opcodes raise ValueError."""
        sim.load(bytes([0xFE, 0xFF]))
        with pytest.raises(ValueError, match="Unknown two-byte opcode"):
            sim.step()

    def test_pc_beyond_bytecode(self, sim: CLRSimulator) -> None:
        """PC beyond bytecode length raises RuntimeError."""
        sim.load(b"")
        with pytest.raises(RuntimeError, match="beyond the end"):
            sim.step()

    def test_ldloc_uninitialized(self, sim: CLRSimulator) -> None:
        """Loading an uninitialized local raises RuntimeError."""
        bytecode = assemble_clr(encode_ldloc(0), (CLROpcode.RET,))
        sim.load(bytecode)
        with pytest.raises(RuntimeError, match="uninitialized"):
            sim.step()

    def test_ldloc_s_uninitialized(self, sim: CLRSimulator) -> None:
        """Loading an uninitialized local via ldloc.s raises RuntimeError."""
        bytecode = assemble_clr(encode_ldloc(10), (CLROpcode.RET,))
        sim.load(bytecode)
        with pytest.raises(RuntimeError, match="uninitialized"):
            sim.step()

    def test_load_resets_state(self, sim: CLRSimulator) -> None:
        """Loading a new program resets all state."""
        # Run first program
        bytecode1 = assemble_clr(
            encode_ldc_i4(42),
            encode_stloc(0),
            (CLROpcode.RET,),
        )
        sim.load(bytecode1)
        sim.run()
        assert sim.locals[0] == 42
        assert sim.halted is True

        # Load second program — state should be reset
        bytecode2 = assemble_clr((CLROpcode.RET,))
        sim.load(bytecode2)
        assert sim.locals[0] is None
        assert sim.halted is False
        assert sim.stack == []
        assert sim.pc == 0

    def test_incomplete_two_byte_opcode(self, sim: CLRSimulator) -> None:
        """Incomplete two-byte opcode (just 0xFE at end) raises ValueError."""
        sim.load(bytes([0xFE]))
        with pytest.raises(ValueError, match="Incomplete two-byte opcode"):
            sim.step()

    def test_brfalse_with_null(self, sim: CLRSimulator) -> None:
        """brfalse.s treats null as false (0) and branches."""
        bytecode = assemble_clr(
            (CLROpcode.LDNULL,),
            bytes([CLROpcode.BRFALSE_S, 0]),  # offset 0 = fall through
            (CLROpcode.RET,),
        )
        sim.load(bytecode)
        traces = sim.run()
        # null is treated as 0 (false), so brfalse.s should branch
        assert "branch taken" in traces[1].description

    def test_num_locals_parameter(self, sim: CLRSimulator) -> None:
        """load() respects the num_locals parameter."""
        sim.load(assemble_clr((CLROpcode.RET,)), num_locals=4)
        assert len(sim.locals) == 4


# ===========================================================================
# simulator-protocol conformance tests
# ===========================================================================
# These tests verify that CLRSimulator satisfies the Simulator[CLRState]
# protocol: get_state(), execute(), and reset() behave correctly and
# the returned types match the protocol contract.


class TestSimulatorProtocolConformance:
    """Verify Simulator[CLRState] protocol conformance for CLRSimulator."""

    def test_get_state_returns_clr_state(self) -> None:
        """get_state() returns a CLRState frozen dataclass with correct field types."""
        from clr_simulator.state import CLRState

        sim = CLRSimulator()
        state = sim.get_state()

        assert isinstance(state, CLRState)
        assert isinstance(state.stack, tuple)
        assert isinstance(state.locals, tuple)
        assert isinstance(state.pc, int)
        assert isinstance(state.halted, bool)

    def test_get_state_is_immutable_snapshot(self) -> None:
        """get_state() snapshots are independent — mutating sim does not affect them."""
        sim = CLRSimulator()
        sim.load(assemble_clr(encode_ldc_i4(1), (CLROpcode.RET,)))
        state_before = sim.get_state()
        sim.step()  # push 1 onto the evaluation stack
        state_after = sim.get_state()

        # The snapshot taken before step() must NOT reflect the new stack
        assert state_before.stack == ()
        assert state_after.stack == (1,)

    def test_execute_simple_program_ok(self) -> None:
        """execute() runs x = 1 + 2 and returns ok=True with correct final state."""
        from simulator_protocol import ExecutionResult

        sim = CLRSimulator()
        program = assemble_clr(
            encode_ldc_i4(1),
            encode_ldc_i4(2),
            (CLROpcode.ADD,),
            encode_stloc(0),
            (CLROpcode.RET,),
        )
        result = sim.execute(program)

        assert isinstance(result, ExecutionResult)
        assert result.ok
        assert result.halted
        assert result.error is None
        assert result.final_state.locals[0] == 3
        assert result.steps == 5

    def test_execute_large_constant(self) -> None:
        """execute() handles the full 5-byte ldc.i4 form (values > 127)."""
        sim = CLRSimulator()
        program = assemble_clr(
            encode_ldc_i4(1000),
            encode_stloc(0),
            (CLROpcode.RET,),
        )
        result = sim.execute(program)

        assert result.ok
        assert result.final_state.locals[0] == 1000

    def test_execute_traces_contain_step_traces(self) -> None:
        """execute() populates result.traces with one StepTrace per instruction."""
        from simulator_protocol import StepTrace

        sim = CLRSimulator()
        program = assemble_clr(
            encode_ldc_i4(3),
            encode_stloc(0),
            (CLROpcode.RET,),
        )
        result = sim.execute(program)

        assert len(result.traces) == 3
        for trace in result.traces:
            assert isinstance(trace, StepTrace)
            assert isinstance(trace.mnemonic, str)
            assert len(trace.mnemonic) > 0

    def test_reset_clears_state(self) -> None:
        """reset() restores the simulator to its initial power-on state."""
        sim = CLRSimulator()
        program = assemble_clr(
            encode_ldc_i4(5),
            encode_stloc(0),
            (CLROpcode.RET,),
        )
        sim.execute(program)

        # After execution the simulator is halted with locals[0] = 5
        assert sim.halted
        assert sim.locals[0] == 5

        sim.reset()

        assert not sim.halted
        assert sim.stack == []
        assert sim.locals[0] is None
        assert sim.pc == 0


class _FakeSignature:
    def __init__(
        self,
        parameter_types: tuple[str, ...],
        return_type: str = "void",
    ) -> None:
        self.parameter_types = parameter_types
        self.return_type = return_type


class _FakeMethodRef:
    def __init__(self) -> None:
        self.declaring_type = "System.Console"
        self.name = "WriteLine"
        self.signature = _FakeSignature(("string",))


class _FakeHost:
    def __init__(self) -> None:
        self.messages: list[str] = []

    def call_method(self, method: object, args: list[object | None]) -> None:
        _ = method
        self.messages.append(f"{args[0]}")
        return None


def test_execute_disassembled_method_body_with_host_call() -> None:
    host = _FakeHost()
    sim = CLRSimulator(host=host)
    body = CLRMethodBody(
        metadata_version="v4.0.30319",
        declaring_type="Program",
        name="Main",
        max_stack=8,
        local_count=0,
        instructions=(
            CLRInstruction(offset=0, opcode="ldstr", operand="Hello, world!", size=5),
            CLRInstruction(offset=5, opcode="call", operand=_FakeMethodRef(), size=5),
            CLRInstruction(offset=10, opcode="ret"),
        ),
        il_bytes=bytes.fromhex("7201000070280d00000a2a"),
    )

    sim.load_method_body(body)
    traces = sim.run()

    assert host.messages == ["Hello, world!"]
    assert [trace.opcode for trace in traces] == ["ldstr", "call", "ret"]
