"""Tests for the JVM bytecode simulator.

These tests verify every instruction in our JVM subset, including edge cases
like stack underflow, division by zero, and invalid opcodes. They also include
end-to-end programs that demonstrate how real Java expressions compile to
JVM bytecode.
"""

import pytest
from jvm_bytecode_disassembler import disassemble_method_body

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


class _FakeMethodRef:
    def __init__(self, descriptor: str) -> None:
        self.descriptor = descriptor


class _FakeHost:
    def __init__(self) -> None:
        self.calls: list[tuple[str, object, list[object]]] = []

    def get_static(self, reference: object) -> object:
        self.calls.append(("get_static", reference, []))
        return {"kind": "print-stream"}

    def invoke_virtual(
        self,
        reference: object,
        receiver: object,
        args: list[object],
    ) -> object | None:
        self.calls.append(("invoke_virtual", receiver, args))
        return None


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

    def test_iconst_128_uses_sipush(self) -> None:
        """Values beyond bipush range fall back to sipush."""
        assert encode_iconst(128) == bytes([0x11, 0x00, 0x80])

    def test_iconst_negative_129_uses_sipush(self) -> None:
        """Negative values below -128 also use sipush when possible."""
        assert encode_iconst(-129) == bytes([0x11, 0xFF, 0x7F])

    def test_iconst_out_of_range_raises(self) -> None:
        """Values outside signed short range still raise ValueError."""
        with pytest.raises(ValueError, match="outside signed short range"):
            encode_iconst(32768)
        with pytest.raises(ValueError, match="outside signed short range"):
            encode_iconst(-32769)


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


# ===========================================================================
# simulator-protocol conformance tests
# ===========================================================================
# These tests verify that JVMSimulator satisfies the Simulator[JVMState]
# protocol: get_state(), execute(), and reset() behave correctly and
# the returned types match the protocol contract.


class TestSimulatorProtocolConformance:
    """Verify Simulator[JVMState] protocol conformance for JVMSimulator."""

    def test_get_state_returns_jvm_state(self) -> None:
        """get_state() returns a JVMState frozen dataclass with correct field types."""
        from jvm_simulator.state import JVMState

        sim = JVMSimulator()
        state = sim.get_state()

        assert isinstance(state, JVMState)
        assert isinstance(state.stack, tuple)
        assert isinstance(state.locals, tuple)
        assert isinstance(state.constants, tuple)
        assert isinstance(state.pc, int)
        assert isinstance(state.halted, bool)

    def test_get_state_is_immutable_snapshot(self) -> None:
        """get_state() snapshots are independent — mutating sim does not affect them."""
        sim = JVMSimulator()
        sim.load(assemble_jvm((JVMOpcode.ICONST_1,), (JVMOpcode.RETURN,)))
        state_before = sim.get_state()
        sim.step()  # push 1 onto stack
        state_after = sim.get_state()

        # The snapshot taken before step() must NOT reflect the new stack
        assert state_before.stack == ()
        assert state_after.stack == (1,)

    def test_execute_simple_program_ok(self) -> None:
        """execute() runs x = 1 + 2 and returns ok=True with correct final state."""
        from simulator_protocol import ExecutionResult

        sim = JVMSimulator()
        program = assemble_jvm(
            (JVMOpcode.ICONST_1,),
            (JVMOpcode.ICONST_2,),
            (JVMOpcode.IADD,),
            (JVMOpcode.ISTORE_0,),
            (JVMOpcode.RETURN,),
        )
        result = sim.execute(program)

        assert isinstance(result, ExecutionResult)
        assert result.ok
        assert result.halted
        assert result.error is None
        assert result.final_state.locals[0] == 3
        assert result.steps == 5

    def test_execute_ireturn_captures_return_value(self) -> None:
        """execute() stores the IRETURN return value in final_state.return_value."""
        sim = JVMSimulator()
        program = assemble_jvm(
            (JVMOpcode.BIPUSH, 42),
            (JVMOpcode.IRETURN,),
        )
        result = sim.execute(program)

        assert result.ok
        assert result.final_state.return_value == 42

    def test_execute_traces_contain_step_traces(self) -> None:
        """execute() populates result.traces with one StepTrace per instruction."""
        from simulator_protocol import StepTrace

        sim = JVMSimulator()
        program = assemble_jvm(
            (JVMOpcode.ICONST_3,),
            (JVMOpcode.ISTORE_0,),
            (JVMOpcode.RETURN,),
        )
        result = sim.execute(program)

        assert len(result.traces) == 3
        for trace in result.traces:
            assert isinstance(trace, StepTrace)
            assert isinstance(trace.mnemonic, str)
            assert len(trace.mnemonic) > 0

    def test_reset_clears_state(self) -> None:
        """reset() restores the simulator to its initial power-on state."""
        sim = JVMSimulator()
        program = assemble_jvm(
            (JVMOpcode.ICONST_5,),
            (JVMOpcode.ISTORE_0,),
            (JVMOpcode.RETURN,),
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
        assert sim.return_value is None


class TestDisassembledMethodFacade:
    """Verify the simulator's disassembled-bytecode API."""

    def test_load_method_executes_disassembled_body(self) -> None:
        method = disassemble_method_body(
            assemble_jvm(
                (JVMOpcode.ICONST_1,),
                (JVMOpcode.ICONST_2,),
                (JVMOpcode.IADD,),
                (JVMOpcode.IRETURN,),
            ),
            max_stack=2,
            max_locals=0,
        )

        sim = JVMSimulator()
        sim.load_method(method)
        traces = sim.run()

        assert sim.halted is True
        assert sim.return_value == 3
        assert traces[-1].opcode == "ireturn"

    def test_load_method_supports_real_constant_pool_indices(self) -> None:
        method = disassemble_method_body(
            assemble_jvm((JVMOpcode.LDC, 9), (JVMOpcode.IRETURN,)),
            max_stack=1,
            max_locals=0,
            constant_pool={9: 300},
        )

        sim = JVMSimulator()
        sim.load_method(method)
        sim.run()

        assert sim.return_value == 300

    def test_load_method_with_host_executes_getstatic_and_invokevirtual(self) -> None:
        host = _FakeHost()
        method = disassemble_method_body(
            assemble_jvm(
                (JVMOpcode.GETSTATIC, 7),
                (JVMOpcode.LDC, 13),
                (JVMOpcode.INVOKEVIRTUAL, 15),
                (JVMOpcode.RETURN,),
            ),
            max_stack=2,
            max_locals=0,
            constant_pool={
                7: "java/lang/System.out",
                13: "Hello, world!",
                15: _FakeMethodRef("(Ljava/lang/String;)V"),
            },
        )

        sim = JVMSimulator(host=host)
        sim.load_method(method)
        traces = sim.run()

        assert sim.halted is True
        assert host.calls == [
            ("get_static", "java/lang/System.out", []),
            ("invoke_virtual", {"kind": "print-stream"}, ["Hello, world!"]),
        ]
        assert [trace.opcode for trace in traces] == [
            "getstatic",
            "ldc",
            "invokevirtual",
            "return",
        ]

    def test_host_required_for_getstatic(self) -> None:
        method = disassemble_method_body(
            assemble_jvm((JVMOpcode.GETSTATIC, 7), (JVMOpcode.RETURN,)),
            constant_pool={7: "java/lang/System.out"},
        )
        sim = JVMSimulator()
        sim.load_method(method)

        with pytest.raises(RuntimeError, match="No JVM host is configured"):
            sim.step()


# ===========================================================================
# New opcode tests (v0.2.0)
# ===========================================================================


class TestIconstM1:
    """iconst_m1 pushes the integer -1."""

    def test_iconst_m1_pushes_minus_one(self) -> None:
        sim = JVMSimulator()
        sim.load(bytes([0x02, 0xAC]))  # iconst_m1, ireturn
        sim.run()
        assert sim.return_value == -1

    def test_iconst_m1_trace_shows_minus_one(self) -> None:
        sim = JVMSimulator()
        sim.load(bytes([0x02, 0xB1]))  # iconst_m1, return
        traces = sim.run()
        assert traces[0].stack_after == [-1]


class TestNop:
    """nop (0x00) advances PC without touching the stack."""

    def test_nop_does_not_change_stack(self) -> None:
        sim = JVMSimulator()
        sim.load(bytes([0x00, 0x04, 0xAC]))  # nop, iconst_1, ireturn
        sim.run()
        assert sim.return_value == 1


class TestPop:
    """pop discards the top of the operand stack."""

    def test_pop_removes_top(self) -> None:
        sim = JVMSimulator()
        sim.load(bytes([0x04, 0x05, 0x57, 0xAC]))  # iconst_1, iconst_2, pop, ireturn
        sim.run()
        assert sim.return_value == 1  # iconst_1 survives; iconst_2 was popped

    def test_pop_stack_underflow_raises(self) -> None:
        sim = JVMSimulator()
        sim.load(bytes([0x57, 0xB1]))  # pop on empty stack, return
        with pytest.raises(RuntimeError, match="Stack underflow"):
            sim.step()


class TestBitwiseOps:
    """ishl, ishr, iand, ior perform bitwise integer operations."""

    def test_ishl(self) -> None:
        # 1 << 3 = 8
        sim = JVMSimulator()
        sim.load(bytes([0x04, 0x06, 0x78, 0xAC]))  # iconst_1, iconst_3, ishl, ireturn
        sim.run()
        assert sim.return_value == 8

    def test_ishr(self) -> None:
        # 8 >> 2 = 2
        sim = JVMSimulator()
        sim.load(bytes([0x10, 0x08, 0x05, 0x7A, 0xAC]))  # bipush 8, iconst_2, ishr, ireturn
        sim.run()
        assert sim.return_value == 2

    def test_iand(self) -> None:
        # 5 & 3 = 1
        sim = JVMSimulator()
        sim.load(bytes([0x08, 0x06, 0x7E, 0xAC]))  # iconst_5, iconst_3
        sim.run()
        assert sim.return_value == 1

    def test_ior(self) -> None:
        # 5 | 2 = 7
        sim = JVMSimulator()
        sim.load(bytes([0x08, 0x05, 0x80, 0xAC]))  # iconst_5, iconst_2, ior, ireturn
        sim.run()
        assert sim.return_value == 7


class TestI2B:
    """i2b narrows an int to a signed byte (sign-extends lowest 8 bits)."""

    def test_i2b_positive_unchanged(self) -> None:
        # 65 stays 65
        sim = JVMSimulator()
        sim.load(bytes([0x10, 0x41, 0x91, 0xAC]))  # bipush 65, i2b, ireturn
        sim.run()
        assert sim.return_value == 65

    def test_i2b_wraps_to_negative(self) -> None:
        # 0xFF = 255 → signed byte = -1
        sim = JVMSimulator()
        sim.load(bytes([0x10, 0xFF, 0x91, 0xAC]))  # bipush 0xFF, i2b, ireturn
        sim.run()
        assert sim.return_value == -1

    def test_i2b_128_becomes_negative_128(self) -> None:
        sim = JVMSimulator()
        sim.load(bytes([0x10, 0x80, 0x91, 0xAC]))  # bipush 0x80, i2b, ireturn
        sim.run()
        assert sim.return_value == -128


class TestIfeqIfne:
    """ifeq / ifne branch on the top-of-stack value compared to 0."""

    def test_ifeq_branches_when_zero(self) -> None:
        # iconst_0; ifeq +3 (skip next); iconst_1; return
        bytecode = bytes([
            0x03,        # iconst_0
            0x99, 0x00, 0x05,  # ifeq +5  (jump to return)
            0x04,        # iconst_1
            0xAC,        # ireturn  ← SKIPPED
            0x03,        # iconst_0  ← TAKEN
            0xAC,        # ireturn
        ])
        sim = JVMSimulator()
        sim.load(bytecode)
        sim.run()
        assert sim.return_value == 0  # branched to iconst_0+ireturn

    def test_ifeq_falls_through_when_nonzero(self) -> None:
        bytecode = bytes([
            0x04,               # iconst_1
            0x99, 0x00, 0x05,   # ifeq +5
            0x05,               # iconst_2  ← TAKEN (falls through)
            0xAC,               # ireturn
            0x03,               # iconst_0
            0xAC,               # ireturn
        ])
        sim = JVMSimulator()
        sim.load(bytecode)
        sim.run()
        assert sim.return_value == 2

    def test_ifne_branches_when_nonzero(self) -> None:
        bytecode = bytes([
            0x04,               # iconst_1
            0x9A, 0x00, 0x05,   # ifne +5  (jump to second ireturn)
            0x03,               # iconst_0
            0xAC,               # ireturn  ← SKIPPED
            0x05,               # iconst_2  ← TAKEN
            0xAC,               # ireturn
        ])
        sim = JVMSimulator()
        sim.load(bytecode)
        sim.run()
        assert sim.return_value == 2

    def test_ifeq_stack_underflow_raises(self) -> None:
        sim = JVMSimulator()
        sim.load(bytes([0x99, 0x00, 0x03, 0xB1]))  # ifeq with empty stack
        with pytest.raises(RuntimeError, match="Stack underflow"):
            sim.step()


class TestIfIcmpLtNe:
    """if_icmplt and if_icmpne branch on two-operand comparisons."""

    def test_if_icmplt_branches_when_less_than(self) -> None:
        # 2 < 5 → branch
        bytecode = bytes([
            0x05, 0x08,           # iconst_2, iconst_5
            0xA1, 0x00, 0x05,     # if_icmplt +5
            0x04, 0xAC,           # iconst_1, ireturn
            0x05, 0xAC,           # iconst_2, ireturn  ← TAKEN
        ])
        sim = JVMSimulator()
        sim.load(bytecode)
        sim.run()
        assert sim.return_value == 2

    def test_if_icmplt_falls_through_when_not_less(self) -> None:
        # 5 < 2 → false → fall through
        bytecode = bytes([
            0x08, 0x05,           # iconst_5, iconst_2
            0xA1, 0x00, 0x05,     # if_icmplt +5
            0x04, 0xAC,           # iconst_1, ireturn  ← TAKEN
            0x05, 0xAC,           # iconst_2, ireturn
        ])
        sim = JVMSimulator()
        sim.load(bytecode)
        sim.run()
        assert sim.return_value == 1

    def test_if_icmpne_branches_when_not_equal(self) -> None:
        # 3 != 4 → branch
        bytecode = bytes([
            0x06, 0x07,           # iconst_3, iconst_4
            0xA0, 0x00, 0x05,     # if_icmpne +5
            0x04, 0xAC,           # iconst_1, ireturn
            0x05, 0xAC,           # iconst_2, ireturn  ← TAKEN
        ])
        sim = JVMSimulator()
        sim.load(bytecode)
        sim.run()
        assert sim.return_value == 2

    def test_if_icmpne_falls_through_when_equal(self) -> None:
        # 4 == 4 → fall through
        bytecode = bytes([
            0x07, 0x07,           # iconst_4, iconst_4
            0xA0, 0x00, 0x05,     # if_icmpne +5
            0x04, 0xAC,           # iconst_1, ireturn  ← TAKEN
            0x05, 0xAC,           # iconst_2, ireturn
        ])
        sim = JVMSimulator()
        sim.load(bytecode)
        sim.run()
        assert sim.return_value == 1



# ---------------------------------------------------------------------------
# Mock field/method reference objects.  The simulator treats these as opaque
# dict keys and only reads the `descriptor` attribute on method references
# for argument-count inference.  We keep them here rather than importing
# jvm_class_file so this package does not need jvm_class_file in its BUILD.
# ---------------------------------------------------------------------------

from dataclasses import dataclass as _dc


@_dc(frozen=True)
class _FieldRef:
    class_name: str
    name: str
    descriptor: str


@_dc(frozen=True)
class _MethodRef:
    class_name: str
    name: str
    descriptor: str


class TestStaticFields:
    """putstatic stores values in static_fields; getstatic reads them back."""

    def test_putstatic_stores_value(self) -> None:
        field_ref = _FieldRef("TestClass", "myField", "I")
        host = _FakeHost()
        method = disassemble_method_body(
            assemble_jvm(
                (JVMOpcode.ICONST_5,),
                (JVMOpcode.PUTSTATIC, 10),
                (JVMOpcode.RETURN,),
            ),
            constant_pool={10: field_ref},
        )
        sim = JVMSimulator(host=host)
        sim.load_method(method)
        sim.run()
        assert sim.static_fields[field_ref] == 5

    def test_getstatic_reads_from_static_fields_first(self) -> None:
        """getstatic should return the value in static_fields before trying host."""
        field_ref = _FieldRef("TestClass", "counter", "I")
        method = disassemble_method_body(
            assemble_jvm(
                (JVMOpcode.GETSTATIC, 10),
                (JVMOpcode.IRETURN,),
            ),
            constant_pool={10: field_ref},
        )
        sim = JVMSimulator()
        sim.static_fields[field_ref] = 42
        sim.load_method(method)
        sim.run()
        assert sim.return_value == 42

    def test_static_fields_survive_load_method(self) -> None:
        """load_method must not reset static_fields."""
        sim = JVMSimulator()
        ref = _FieldRef("A", "x", "I")
        sim.static_fields[ref] = 99
        sim.load(bytes([0xB1]))  # return
        assert sim.static_fields[ref] == 99

    def test_static_fields_survive_reset(self) -> None:
        """reset() clears per-method state but must NOT wipe static_fields."""
        sim = JVMSimulator()
        ref = _FieldRef("A", "y", "I")
        sim.static_fields[ref] = 7
        sim.reset()
        assert sim.static_fields[ref] == 7

    def test_shared_static_fields_dict_is_visible_to_multiple_simulators(self) -> None:
        """Two simulators sharing the same dict see each other's writes."""
        shared: dict = {}
        ref = _FieldRef("Foo", "val", "I")

        writer = JVMSimulator(static_fields=shared)
        writer.static_fields[ref] = 123

        reader = JVMSimulator(static_fields=shared)
        assert reader.static_fields[ref] == 123


class TestInvokestatic:
    """invokestatic dispatches to host.invoke_static with args and static_fields."""

    class _InvokeStaticHost:
        """Records invokestatic calls and returns a fixed value."""

        def __init__(self, return_value: object | None = None) -> None:
            self.calls: list[tuple] = []
            self._return_value = return_value

        def get_static(self, reference: object) -> object:
            return {"stream": True}

        def invoke_static(
            self,
            reference: object,
            static_fields: dict,
            args: list[object],
        ) -> object | None:
            self.calls.append((reference, static_fields, list(args)))
            return self._return_value

    def test_invokestatic_void_method_passes_args(self) -> None:
        method_ref = _MethodRef("Foo", "bar", "(II)V")
        host = self._InvokeStaticHost(return_value=None)

        method = disassemble_method_body(
            assemble_jvm(
                (JVMOpcode.ICONST_3,),
                (JVMOpcode.ICONST_4,),
                (JVMOpcode.INVOKESTATIC, 5),
                (JVMOpcode.RETURN,),
            ),
            constant_pool={5: method_ref},
        )
        sim = JVMSimulator(host=host)
        sim.load_method(method)
        sim.run()
        assert len(host.calls) == 1
        ref, sf, args = host.calls[0]
        assert ref is method_ref
        assert args == [3, 4]

    def test_invokestatic_with_return_value_pushes_result(self) -> None:
        method_ref = _MethodRef("Foo", "getValue", "()I")
        host = self._InvokeStaticHost(return_value=99)

        method = disassemble_method_body(
            assemble_jvm(
                (JVMOpcode.INVOKESTATIC, 5),
                (JVMOpcode.IRETURN,),
            ),
            constant_pool={5: method_ref},
        )
        sim = JVMSimulator(host=host)
        sim.load_method(method)
        sim.run()
        assert sim.return_value == 99

    def test_invokestatic_passes_static_fields_dict(self) -> None:
        method_ref = _MethodRef("X", "m", "()V")
        host = self._InvokeStaticHost(return_value=None)
        shared: dict = {}
        ref = _FieldRef("X", "f", "I")
        shared[ref] = 77

        method = disassemble_method_body(
            assemble_jvm((JVMOpcode.INVOKESTATIC, 5), (JVMOpcode.RETURN,)),
            constant_pool={5: method_ref},
        )
        sim = JVMSimulator(host=host, static_fields=shared)
        sim.load_method(method)
        sim.run()
        _, sf, _ = host.calls[0]
        assert sf is shared

    def test_invokestatic_no_host_raises(self) -> None:
        method_ref = _MethodRef("X", "m", "()V")
        method = disassemble_method_body(
            assemble_jvm((JVMOpcode.INVOKESTATIC, 5), (JVMOpcode.RETURN,)),
            constant_pool={5: method_ref},
        )
        sim = JVMSimulator()  # no host
        sim.load_method(method)
        with pytest.raises(RuntimeError, match="No JVM host is configured for invokestatic"):
            sim.step()


class TestArrayOps:
    """newarray, iaload, iastore, baload, bastore operate on Python lists."""

    def test_newarray_creates_array_of_correct_size(self) -> None:
        # iconst_3; newarray T_INT; istore_0; iload_0; iconst_2; iaload; ireturn
        # Array of size 3, access element at index 2 (default 0)
        bytecode = bytearray()
        bytecode += bytes([0x06])        # iconst_3
        bytecode += bytes([0xBC, 0x0A])  # newarray T_INT=10
        bytecode += bytes([0x3B])        # istore_0
        bytecode += bytes([0x1A])        # iload_0
        bytecode += bytes([0x05])        # iconst_2
        bytecode += bytes([0x2E])        # iaload
        bytecode += bytes([0xAC])        # ireturn
        sim = JVMSimulator()
        sim.load(bytes(bytecode), num_locals=4)
        sim.run()
        assert sim.return_value == 0  # default value

    def test_newarray_and_iastore_and_iaload(self) -> None:
        # new int[3]; a[1] = 42; return a[1]
        bytecode = bytearray()
        bytecode += bytes([0x06])        # iconst_3
        bytecode += bytes([0xBC, 0x0A])  # newarray T_INT=10
        bytecode += bytes([0x3B])        # istore_0
        bytecode += bytes([0x1A])        # iload_0
        bytecode += bytes([0x04])        # iconst_1
        bytecode += bytes([0x10, 42])    # bipush 42
        bytecode += bytes([0x4F])        # iastore
        bytecode += bytes([0x1A])        # iload_0
        bytecode += bytes([0x04])        # iconst_1
        bytecode += bytes([0x2E])        # iaload
        bytecode += bytes([0xAC])        # ireturn
        sim = JVMSimulator()
        sim.load(bytes(bytecode), num_locals=4)
        sim.run()
        assert sim.return_value == 42

    def test_newarray_and_bastore_and_baload(self) -> None:
        # new byte[4]; b[2] = 0xFF; return b[2] (sign-extended = -1)
        bytecode = bytearray()
        bytecode += bytes([0x07])        # iconst_4
        bytecode += bytes([0xBC, 0x08])  # newarray T_BYTE=8
        bytecode += bytes([0x3B])        # istore_0
        bytecode += bytes([0x1A])        # iload_0
        bytecode += bytes([0x05])        # iconst_2
        bytecode += bytes([0x10, 0xFF])  # bipush 0xFF
        bytecode += bytes([0x54])        # bastore
        bytecode += bytes([0x1A])        # iload_0
        bytecode += bytes([0x05])        # iconst_2
        bytecode += bytes([0x33])        # baload
        bytecode += bytes([0xAC])        # ireturn
        sim = JVMSimulator()
        sim.load(bytes(bytecode), num_locals=4)
        sim.run()
        assert sim.return_value == -1  # 0xFF sign-extended

    def test_iaload_out_of_bounds_raises(self) -> None:
        bytecode = bytearray()
        bytecode += bytes([0x03])        # iconst_0 (size = 0)
        bytecode += bytes([0xBC, 0x0A])  # newarray T_INT
        bytecode += bytes([0x03])        # iconst_0 (index 0 is out of bounds for empty array)
        bytecode += bytes([0x2E])        # iaload
        bytecode += bytes([0xAC])
        sim = JVMSimulator()
        sim.load(bytes(bytecode))
        with pytest.raises(RuntimeError, match="ArrayIndexOutOfBoundsException"):
            sim.run()

    def test_iastore_out_of_bounds_raises(self) -> None:
        bytecode = bytearray()
        bytecode += bytes([0x03])        # iconst_0 (size = 0)
        bytecode += bytes([0xBC, 0x0A])  # newarray T_INT
        bytecode += bytes([0x03])        # iconst_0 (index)
        bytecode += bytes([0x04])        # iconst_1 (value)
        bytecode += bytes([0x4F])        # iastore
        bytecode += bytes([0xB1])
        sim = JVMSimulator()
        sim.load(bytes(bytecode))
        with pytest.raises(RuntimeError, match="ArrayIndexOutOfBoundsException"):
            sim.run()
