"""Tests for the JVM bytecode simulator.

These tests verify every instruction in our JVM subset, including edge cases
like stack underflow, division by zero, and invalid opcodes. They also include
end-to-end programs that demonstrate how real Java expressions compile to
JVM bytecode.
"""

import pytest

from jvm_simulator.simulator import (
    JVMOpcode,
    JVMSimulator,
    JVMTrace,
    assemble_jvm,
    encode_iconst,
    encode_iload,
    encode_istore,
)


# ===========================================================================
# Helper encoding tests
# ===========================================================================


class TestEncodeIconst:
    """Test the encode_iconst helper for pushing integer constants."""

    def test_iconst_0(self) -> None:
        """iconst_0 is a single byte: 0x03."""
        assert encode_iconst(0) == bytes([0x03])

    def test_iconst_1(self) -> None:
        """iconst_1 is a single byte: 0x04."""
        assert encode_iconst(1) == bytes([0x04])

    def test_iconst_5(self) -> None:
        """iconst_5 is a single byte: 0x08."""
        assert encode_iconst(5) == bytes([0x08])

    def test_iconst_42_uses_bipush(self) -> None:
        """Values 6-127 fall back to bipush (2 bytes)."""
        result = encode_iconst(42)
        assert result == bytes([0x10, 42])

    def test_iconst_negative_uses_bipush(self) -> None:
        """Negative values use bipush with signed encoding."""
        result = encode_iconst(-1)
        assert result == bytes([0x10, 0xFF])

    def test_iconst_negative_128(self) -> None:
        """Minimum signed byte value (-128) uses bipush."""
        result = encode_iconst(-128)
        assert result == bytes([0x10, 0x80])

    def test_iconst_out_of_range_raises(self) -> None:
        """Values outside signed byte range raise ValueError."""
        with pytest.raises(ValueError, match="outside signed byte range"):
            encode_iconst(128)
        with pytest.raises(ValueError, match="outside signed byte range"):
            encode_iconst(-129)


class TestEncodeIstore:
    """Test the encode_istore helper for storing to local variable slots."""

    def test_istore_0_uses_shortcut(self) -> None:
        """Slot 0 uses the single-byte istore_0 shortcut."""
        assert encode_istore(0) == bytes([0x3B])

    def test_istore_3_uses_shortcut(self) -> None:
        """Slot 3 uses the single-byte istore_3 shortcut."""
        assert encode_istore(3) == bytes([0x3E])

    def test_istore_5_uses_generic(self) -> None:
        """Slot 5 uses the 2-byte generic istore form."""
        assert encode_istore(5) == bytes([0x36, 0x05])


class TestEncodeIload:
    """Test the encode_iload helper for loading from local variable slots."""

    def test_iload_0_uses_shortcut(self) -> None:
        """Slot 0 uses the single-byte iload_0 shortcut."""
        assert encode_iload(0) == bytes([0x1A])

    def test_iload_3_uses_shortcut(self) -> None:
        """Slot 3 uses the single-byte iload_3 shortcut."""
        assert encode_iload(3) == bytes([0x1D])

    def test_iload_5_uses_generic(self) -> None:
        """Slot 5 uses the 2-byte generic iload form."""
        assert encode_iload(5) == bytes([0x15, 0x05])


class TestAssembleJvm:
    """Test the assemble_jvm helper for building bytecode programs."""

    def test_simple_program(self) -> None:
        """Assemble iconst_1 + iconst_2 + iadd + istore_0 + return."""
        bytecode = assemble_jvm(
            (JVMOpcode.ICONST_1,),
            (JVMOpcode.ICONST_2,),
            (JVMOpcode.IADD,),
            (JVMOpcode.ISTORE_0,),
            (JVMOpcode.RETURN,),
        )
        assert bytecode == bytes([0x04, 0x05, 0x60, 0x3B, 0xB1])

    def test_bipush_assembly(self) -> None:
        """bipush 42 assembles to 2 bytes."""
        bytecode = assemble_jvm((JVMOpcode.BIPUSH, 42))
        assert bytecode == bytes([0x10, 42])

    def test_bipush_negative(self) -> None:
        """bipush -5 assembles with signed byte encoding."""
        bytecode = assemble_jvm((JVMOpcode.BIPUSH, -5))
        assert bytecode == bytes([0x10, 0xFB])

    def test_goto_assembly(self) -> None:
        """goto with offset 3 assembles to 3 bytes."""
        bytecode = assemble_jvm((JVMOpcode.GOTO, 3))
        assert bytecode == bytes([0xA7, 0x00, 0x03])

    def test_goto_negative_offset(self) -> None:
        """goto with negative offset assembles correctly."""
        bytecode = assemble_jvm((JVMOpcode.GOTO, -5))
        assert bytecode == bytes([0xA7, 0xFF, 0xFB])

    def test_ldc_assembly(self) -> None:
        """ldc with index 3 assembles to 2 bytes."""
        bytecode = assemble_jvm((JVMOpcode.LDC, 3))
        assert bytecode == bytes([0x12, 0x03])

    def test_iload_generic_assembly(self) -> None:
        """iload with slot 5 assembles to 2 bytes."""
        bytecode = assemble_jvm((JVMOpcode.ILOAD, 5))
        assert bytecode == bytes([0x15, 0x05])

    def test_istore_generic_assembly(self) -> None:
        """istore with slot 7 assembles to 2 bytes."""
        bytecode = assemble_jvm((JVMOpcode.ISTORE, 7))
        assert bytecode == bytes([0x36, 0x07])

    def test_missing_operand_raises(self) -> None:
        """Opcodes that need operands raise ValueError if missing."""
        with pytest.raises(ValueError, match="requires an operand"):
            assemble_jvm((JVMOpcode.BIPUSH,))

    def test_missing_offset_raises(self) -> None:
        """Branch opcodes raise ValueError if missing offset."""
        with pytest.raises(ValueError, match="requires an offset"):
            assemble_jvm((JVMOpcode.GOTO,))


# ===========================================================================
# iconst_N instruction tests
# ===========================================================================


class TestIconst:
    """Test iconst_0 through iconst_5: push small integer constants."""

    def test_iconst_0(self) -> None:
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.ICONST_0,), (JVMOpcode.RETURN,)))
        traces = sim.run()
        assert sim.stack == [0]  # iconst_0 pushed 0, return just halts
        assert traces[0].stack_after == [0]
        assert traces[0].opcode == "iconst_0"

    def test_iconst_1(self) -> None:
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.ICONST_1,), (JVMOpcode.RETURN,)))
        sim.run()
        assert sim.stack == [1]  # return doesn't pop

    def test_iconst_2(self) -> None:
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.ICONST_2,), (JVMOpcode.RETURN,)))
        sim.run()
        assert sim.stack == [2]

    def test_iconst_3(self) -> None:
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.ICONST_3,), (JVMOpcode.RETURN,)))
        sim.run()
        assert sim.stack == [3]

    def test_iconst_4(self) -> None:
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.ICONST_4,), (JVMOpcode.RETURN,)))
        sim.run()
        assert sim.stack == [4]

    def test_iconst_5(self) -> None:
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.ICONST_5,), (JVMOpcode.RETURN,)))
        sim.run()
        assert sim.stack == [5]

    def test_iconst_trace_description(self) -> None:
        """Trace should describe what value was pushed."""
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.ICONST_3,), (JVMOpcode.RETURN,)))
        traces = sim.run()
        assert traces[0].description == "push 3"


# ===========================================================================
# bipush instruction tests
# ===========================================================================


class TestBipush:
    """Test bipush: push a signed byte value (-128 to 127)."""

    def test_bipush_positive(self) -> None:
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.BIPUSH, 42), (JVMOpcode.RETURN,)))
        sim.run()
        assert sim.stack == [42]

    def test_bipush_zero(self) -> None:
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.BIPUSH, 0), (JVMOpcode.RETURN,)))
        sim.run()
        assert sim.stack == [0]

    def test_bipush_negative(self) -> None:
        """bipush -1 should push -1 (signed byte 0xFF)."""
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.BIPUSH, -1), (JVMOpcode.RETURN,)))
        sim.run()
        assert sim.stack == [-1]

    def test_bipush_min_value(self) -> None:
        """bipush -128 (minimum signed byte)."""
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.BIPUSH, -128), (JVMOpcode.RETURN,)))
        sim.run()
        assert sim.stack == [-128]

    def test_bipush_max_value(self) -> None:
        """bipush 127 (maximum signed byte)."""
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.BIPUSH, 127), (JVMOpcode.RETURN,)))
        sim.run()
        assert sim.stack == [127]


# ===========================================================================
# ldc instruction tests
# ===========================================================================


class TestLdc:
    """Test ldc: load a constant from the constant pool."""

    def test_ldc_integer(self) -> None:
        sim = JVMSimulator()
        sim.load(
            assemble_jvm((JVMOpcode.LDC, 0), (JVMOpcode.RETURN,)),
            constants=[999],
        )
        sim.run()
        assert sim.stack == [999]

    def test_ldc_multiple_constants(self) -> None:
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.LDC, 0),
                (JVMOpcode.LDC, 1),
                (JVMOpcode.RETURN,),
            ),
            constants=[100, 200],
        )
        sim.run()
        assert sim.stack == [100, 200]

    def test_ldc_out_of_range_raises(self) -> None:
        """Accessing an out-of-range constant pool index raises RuntimeError."""
        sim = JVMSimulator()
        sim.load(
            assemble_jvm((JVMOpcode.LDC, 5), (JVMOpcode.RETURN,)),
            constants=[42],
        )
        with pytest.raises(RuntimeError, match="Constant pool index"):
            sim.run()

    def test_ldc_trace_description(self) -> None:
        sim = JVMSimulator()
        sim.load(
            assemble_jvm((JVMOpcode.LDC, 0), (JVMOpcode.RETURN,)),
            constants=[42],
        )
        traces = sim.run()
        assert "constant[0] = 42" in traces[0].description


# ===========================================================================
# iload / istore instruction tests
# ===========================================================================


class TestIloadIstore:
    """Test loading and storing local variables."""

    def test_istore_0_iload_0(self) -> None:
        """Store 5 in slot 0, load it back."""
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_5,),
                (JVMOpcode.ISTORE_0,),
                (JVMOpcode.ILOAD_0,),
                (JVMOpcode.RETURN,),
            )
        )
        sim.run()
        assert sim.stack == [5]
        assert sim.locals[0] == 5

    def test_istore_1_iload_1(self) -> None:
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_3,),
                (JVMOpcode.ISTORE_1,),
                (JVMOpcode.ILOAD_1,),
                (JVMOpcode.RETURN,),
            )
        )
        sim.run()
        assert sim.stack == [3]
        assert sim.locals[1] == 3

    def test_istore_2_iload_2(self) -> None:
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_2,),
                (JVMOpcode.ISTORE_2,),
                (JVMOpcode.ILOAD_2,),
                (JVMOpcode.RETURN,),
            )
        )
        sim.run()
        assert sim.stack == [2]
        assert sim.locals[2] == 2

    def test_istore_3_iload_3(self) -> None:
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_4,),
                (JVMOpcode.ISTORE_3,),
                (JVMOpcode.ILOAD_3,),
                (JVMOpcode.RETURN,),
            )
        )
        sim.run()
        assert sim.stack == [4]
        assert sim.locals[3] == 4

    def test_istore_generic_iload_generic(self) -> None:
        """Test generic istore/iload with slot > 3."""
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.BIPUSH, 99),
                (JVMOpcode.ISTORE, 5),
                (JVMOpcode.ILOAD, 5),
                (JVMOpcode.RETURN,),
            )
        )
        sim.run()
        assert sim.stack == [99]
        assert sim.locals[5] == 99

    def test_iload_uninitialized_raises(self) -> None:
        """Loading from an uninitialized local raises RuntimeError."""
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.ILOAD_0,), (JVMOpcode.RETURN,)))
        with pytest.raises(RuntimeError, match="not been initialized"):
            sim.run()

    def test_istore_underflow_raises(self) -> None:
        """Storing from an empty stack raises RuntimeError."""
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.ISTORE_0,), (JVMOpcode.RETURN,)))
        with pytest.raises(RuntimeError, match="Stack underflow"):
            sim.run()


# ===========================================================================
# Arithmetic instruction tests
# ===========================================================================


class TestArithmetic:
    """Test iadd, isub, imul, idiv."""

    def test_iadd(self) -> None:
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_3,),
                (JVMOpcode.ICONST_4,),
                (JVMOpcode.IADD,),
                (JVMOpcode.RETURN,),
            )
        )
        sim.run()
        assert sim.stack == [7]

    def test_isub(self) -> None:
        """isub: second-to-top minus top. 5 - 3 = 2."""
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_5,),
                (JVMOpcode.ICONST_3,),
                (JVMOpcode.ISUB,),
                (JVMOpcode.RETURN,),
            )
        )
        sim.run()
        assert sim.stack == [2]

    def test_imul(self) -> None:
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_3,),
                (JVMOpcode.ICONST_4,),
                (JVMOpcode.IMUL,),
                (JVMOpcode.RETURN,),
            )
        )
        sim.run()
        assert sim.stack == [12]

    def test_idiv(self) -> None:
        """idiv: 5 / 2 = 2 (integer division truncates toward zero)."""
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_5,),
                (JVMOpcode.ICONST_2,),
                (JVMOpcode.IDIV,),
                (JVMOpcode.RETURN,),
            )
        )
        sim.run()
        assert sim.stack == [2]

    def test_idiv_by_zero_raises(self) -> None:
        """Division by zero raises RuntimeError (like Java ArithmeticException)."""
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_5,),
                (JVMOpcode.ICONST_0,),
                (JVMOpcode.IDIV,),
                (JVMOpcode.RETURN,),
            )
        )
        with pytest.raises(RuntimeError, match="division by zero"):
            sim.run()

    def test_iadd_trace(self) -> None:
        """Verify trace captures stack states for iadd."""
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_1,),
                (JVMOpcode.ICONST_2,),
                (JVMOpcode.IADD,),
                (JVMOpcode.RETURN,),
            )
        )
        traces = sim.run()
        # The iadd trace should be the third instruction (index 2)
        iadd_trace = traces[2]
        assert iadd_trace.opcode == "iadd"
        assert iadd_trace.stack_after == [3]
        assert "pop 2 and 1, push 3" in iadd_trace.description

    def test_arithmetic_underflow_raises(self) -> None:
        """Arithmetic on empty stack raises RuntimeError."""
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.IADD,), (JVMOpcode.RETURN,)))
        with pytest.raises(RuntimeError, match="Stack underflow"):
            sim.run()

    def test_isub_negative_result(self) -> None:
        """isub can produce negative results: 2 - 5 = -3."""
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_2,),
                (JVMOpcode.ICONST_5,),
                (JVMOpcode.ISUB,),
                (JVMOpcode.RETURN,),
            )
        )
        sim.run()
        assert sim.stack == [-3]


# ===========================================================================
# Control flow instruction tests
# ===========================================================================


class TestControlFlow:
    """Test goto, if_icmpeq, if_icmpgt."""

    def test_goto_forward(self) -> None:
        """goto +5 should skip 2 bytes past itself to the next instruction.

        Layout:
            PC=0: iconst_1     (1 byte)
            PC=1: goto +5      (3 bytes) -> jumps to PC=6
            PC=4: iconst_2     (1 byte)  <- skipped
            PC=5: return       (1 byte)  <- skipped
            PC=6: iconst_3     (1 byte)
            PC=7: return       (1 byte)
        """
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_1,),  # PC=0
                (JVMOpcode.GOTO, 5),  # PC=1, offset +5 -> target PC=6
                (JVMOpcode.ICONST_2,),  # PC=4, skipped
                (JVMOpcode.RETURN,),  # PC=5, skipped
                (JVMOpcode.ICONST_3,),  # PC=6
                (JVMOpcode.RETURN,),  # PC=7
            )
        )
        sim.run()
        # Should have 1 and 3 on stack (2 was skipped)
        assert sim.stack == [1, 3]

    def test_if_icmpeq_taken(self) -> None:
        """if_icmpeq should branch when two values are equal.

        Layout:
            PC=0: iconst_3     (1 byte)
            PC=1: iconst_3     (1 byte)
            PC=2: if_icmpeq +6 (3 bytes) -> target PC=8
            PC=5: iconst_1     (1 byte)  <- skipped
            PC=6: return       (1 byte)  <- skipped
            PC=7: return       (1 byte)  <- skipped (padding)
            PC=8: iconst_5     (1 byte)
            PC=9: return       (1 byte)
        """
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_3,),  # PC=0
                (JVMOpcode.ICONST_3,),  # PC=1
                (JVMOpcode.IF_ICMPEQ, 6),  # PC=2, branch if equal -> PC=8
                (JVMOpcode.ICONST_1,),  # PC=5, skipped
                (JVMOpcode.RETURN,),  # PC=6, skipped
                (JVMOpcode.RETURN,),  # PC=7, padding
                (JVMOpcode.ICONST_5,),  # PC=8
                (JVMOpcode.RETURN,),  # PC=9
            )
        )
        sim.run()
        assert sim.stack == [5]

    def test_if_icmpeq_not_taken(self) -> None:
        """if_icmpeq should fall through when values are not equal."""
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_3,),  # PC=0
                (JVMOpcode.ICONST_4,),  # PC=1
                (JVMOpcode.IF_ICMPEQ, 6),  # PC=2, not taken -> fall through to PC=5
                (JVMOpcode.ICONST_1,),  # PC=5, executed
                (JVMOpcode.RETURN,),  # PC=6
            )
        )
        sim.run()
        assert sim.stack == [1]

    def test_if_icmpgt_taken(self) -> None:
        """if_icmpgt should branch when first > second.

        push 5, push 3 -> 5 > 3 is true -> branch taken.
        """
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_5,),  # PC=0
                (JVMOpcode.ICONST_3,),  # PC=1
                (JVMOpcode.IF_ICMPGT, 6),  # PC=2, 5 > 3 -> branch to PC=8
                (JVMOpcode.ICONST_0,),  # PC=5, skipped
                (JVMOpcode.RETURN,),  # PC=6, skipped
                (JVMOpcode.RETURN,),  # PC=7, padding
                (JVMOpcode.ICONST_1,),  # PC=8
                (JVMOpcode.RETURN,),  # PC=9
            )
        )
        sim.run()
        assert sim.stack == [1]

    def test_if_icmpgt_not_taken(self) -> None:
        """if_icmpgt should fall through when first <= second.

        push 3, push 5 -> 3 > 5 is false -> fall through.
        """
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_3,),  # PC=0
                (JVMOpcode.ICONST_5,),  # PC=1
                (JVMOpcode.IF_ICMPGT, 6),  # PC=2, 3 > 5 is false -> PC=5
                (JVMOpcode.ICONST_0,),  # PC=5, executed
                (JVMOpcode.RETURN,),  # PC=6
            )
        )
        sim.run()
        assert sim.stack == [0]

    def test_if_icmpgt_equal_not_taken(self) -> None:
        """if_icmpgt should fall through when values are equal (> not >=)."""
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_3,),  # PC=0
                (JVMOpcode.ICONST_3,),  # PC=1
                (JVMOpcode.IF_ICMPGT, 6),  # PC=2, 3 > 3 is false -> PC=5
                (JVMOpcode.ICONST_0,),  # PC=5, executed
                (JVMOpcode.RETURN,),  # PC=6
            )
        )
        sim.run()
        assert sim.stack == [0]


# ===========================================================================
# Return instruction tests
# ===========================================================================


class TestReturn:
    """Test return and ireturn instructions."""

    def test_return_halts(self) -> None:
        """return should halt the simulator."""
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.RETURN,)))
        sim.run()
        assert sim.halted is True
        assert sim.return_value is None

    def test_ireturn_halts_with_value(self) -> None:
        """ireturn should halt and capture the return value."""
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_5,),
                (JVMOpcode.IRETURN,),
            )
        )
        sim.run()
        assert sim.halted is True
        assert sim.return_value == 5

    def test_ireturn_pops_stack(self) -> None:
        """ireturn should pop the value from the stack."""
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_1,),
                (JVMOpcode.ICONST_5,),
                (JVMOpcode.IRETURN,),
            )
        )
        sim.run()
        assert sim.stack == [1]  # Only 5 was popped
        assert sim.return_value == 5

    def test_return_trace_description(self) -> None:
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.RETURN,)))
        traces = sim.run()
        assert traces[0].description == "return void"

    def test_ireturn_trace_description(self) -> None:
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_3,),
                (JVMOpcode.IRETURN,),
            )
        )
        traces = sim.run()
        assert traces[1].description == "return 3"


# ===========================================================================
# End-to-end program tests
# ===========================================================================


class TestEndToEnd:
    """Test complete programs that demonstrate real JVM bytecode patterns."""

    def test_x_equals_1_plus_2(self) -> None:
        """The classic 'x = 1 + 2' program.

        This is the JVM equivalent of:
            int x = 1 + 2;

        Bytecode:
            iconst_1      push 1
            iconst_2      push 2
            iadd           pop 2 and 1, push 3
            istore_0      pop 3, store in local 0 (x)
            return         halt
        """
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_1,),
                (JVMOpcode.ICONST_2,),
                (JVMOpcode.IADD,),
                (JVMOpcode.ISTORE_0,),
                (JVMOpcode.RETURN,),
            )
        )
        traces = sim.run()
        assert sim.locals[0] == 3
        assert len(traces) == 5

    def test_x_equals_3_plus_4_times_2(self) -> None:
        """x = (3 + 4) * 2

        Bytecode:
            iconst_3      push 3
            iconst_4      push 4
            iadd           pop 4 and 3, push 7
            iconst_2      push 2
            imul           pop 2 and 7, push 14
            istore_0      pop 14, store in local 0 (x)
            return         halt
        """
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_3,),
                (JVMOpcode.ICONST_4,),
                (JVMOpcode.IADD,),
                (JVMOpcode.ICONST_2,),
                (JVMOpcode.IMUL,),
                (JVMOpcode.ISTORE_0,),
                (JVMOpcode.RETURN,),
            )
        )
        sim.run()
        assert sim.locals[0] == 14

    def test_swap_two_variables(self) -> None:
        """Swap a and b using a temp variable.

        Java equivalent:
            int a = 3, b = 5;
            int temp = a;
            a = b;
            b = temp;
        """
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                # a = 3
                (JVMOpcode.ICONST_3,),
                (JVMOpcode.ISTORE_0,),
                # b = 5
                (JVMOpcode.ICONST_5,),
                (JVMOpcode.ISTORE_1,),
                # temp = a
                (JVMOpcode.ILOAD_0,),
                (JVMOpcode.ISTORE_2,),
                # a = b
                (JVMOpcode.ILOAD_1,),
                (JVMOpcode.ISTORE_0,),
                # b = temp
                (JVMOpcode.ILOAD_2,),
                (JVMOpcode.ISTORE_1,),
                (JVMOpcode.RETURN,),
            )
        )
        sim.run()
        assert sim.locals[0] == 5  # a = 5
        assert sim.locals[1] == 3  # b = 3

    def test_ireturn_value(self) -> None:
        """A method that returns 1 + 2 = 3."""
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_1,),
                (JVMOpcode.ICONST_2,),
                (JVMOpcode.IADD,),
                (JVMOpcode.IRETURN,),
            )
        )
        sim.run()
        assert sim.return_value == 3

    def test_trace_stack_states(self) -> None:
        """Verify that traces capture accurate stack snapshots."""
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_1,),
                (JVMOpcode.ICONST_2,),
                (JVMOpcode.IADD,),
                (JVMOpcode.RETURN,),
            )
        )
        traces = sim.run()

        # iconst_1: [] -> [1]
        assert traces[0].stack_before == []
        assert traces[0].stack_after == [1]

        # iconst_2: [1] -> [1, 2]
        assert traces[1].stack_before == [1]
        assert traces[1].stack_after == [1, 2]

        # iadd: [1, 2] -> [3]
        assert traces[2].stack_after == [3]

    def test_trace_locals_snapshot(self) -> None:
        """Verify that traces capture locals state after istore."""
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_5,),
                (JVMOpcode.ISTORE_0,),
                (JVMOpcode.RETURN,),
            )
        )
        traces = sim.run()
        # After istore_0, locals[0] should be 5
        assert traces[1].locals_snapshot[0] == 5
        assert traces[1].opcode == "istore_0"


# ===========================================================================
# Error handling tests
# ===========================================================================


class TestErrors:
    """Test error conditions and edge cases."""

    def test_invalid_opcode_raises(self) -> None:
        """An unknown opcode byte should raise RuntimeError."""
        sim = JVMSimulator()
        sim.load(bytes([0xFF]))  # 0xFF is not a valid JVM opcode
        with pytest.raises(RuntimeError, match="Unknown JVM opcode"):
            sim.step()

    def test_step_after_halt_raises(self) -> None:
        """Stepping after halt should raise RuntimeError."""
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.RETURN,)))
        sim.run()
        with pytest.raises(RuntimeError, match="halted"):
            sim.step()

    def test_pc_past_end_raises(self) -> None:
        """PC past bytecode end should raise RuntimeError."""
        sim = JVMSimulator()
        sim.load(b"")
        with pytest.raises(RuntimeError, match="past end of bytecode"):
            sim.step()

    def test_ireturn_empty_stack_raises(self) -> None:
        """ireturn on empty stack should raise RuntimeError."""
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.IRETURN,)))
        with pytest.raises(RuntimeError, match="Stack underflow"):
            sim.run()

    def test_if_icmpeq_underflow_raises(self) -> None:
        """if_icmpeq with insufficient stack raises RuntimeError."""
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_1,),
                (JVMOpcode.IF_ICMPEQ, 3),
            )
        )
        with pytest.raises(RuntimeError, match="Stack underflow"):
            sim.run()

    def test_load_resets_state(self) -> None:
        """Loading new bytecode should reset all state."""
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_5,),
                (JVMOpcode.ISTORE_0,),
                (JVMOpcode.RETURN,),
            )
        )
        sim.run()
        assert sim.locals[0] == 5
        assert sim.halted is True

        # Load new program -- state should reset
        sim.load(assemble_jvm((JVMOpcode.RETURN,)))
        assert sim.locals[0] is None
        assert sim.halted is False
        assert sim.stack == []
        assert sim.pc == 0

    def test_max_steps_safety(self) -> None:
        """run() should stop after max_steps even if not halted."""
        sim = JVMSimulator()
        # Infinite loop: goto -3 (jump back to itself)
        # But since goto offset is from its own PC, we need goto 0
        sim.load(assemble_jvm((JVMOpcode.GOTO, 0)))
        traces = sim.run(max_steps=5)
        assert len(traces) == 5
        assert not sim.halted


# ===========================================================================
# Simulator step() method tests
# ===========================================================================


class TestStep:
    """Test the step() method directly for finer control."""

    def test_step_returns_trace(self) -> None:
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.ICONST_1,), (JVMOpcode.RETURN,)))
        trace = sim.step()
        assert isinstance(trace, JVMTrace)
        assert trace.pc == 0
        assert trace.opcode == "iconst_1"

    def test_step_advances_pc(self) -> None:
        sim = JVMSimulator()
        sim.load(
            assemble_jvm(
                (JVMOpcode.ICONST_1,),
                (JVMOpcode.ICONST_2,),
                (JVMOpcode.RETURN,),
            )
        )
        sim.step()
        assert sim.pc == 1  # iconst_1 is 1 byte
        sim.step()
        assert sim.pc == 2  # iconst_2 is 1 byte

    def test_step_bipush_advances_pc_by_2(self) -> None:
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.BIPUSH, 42), (JVMOpcode.RETURN,)))
        sim.step()
        assert sim.pc == 2  # bipush is 2 bytes
