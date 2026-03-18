"""Comprehensive tests for the CLR IL Compiler.

These tests verify that the CLR compiler correctly translates AST nodes into
real CLR IL bytecode bytes. We test the same categories as the JVM compiler:

1. **Number encoding** — Verify the tiered encoding (ldc.i4.N, ldc.i4.s, ldc.i4).
2. **Local variable encoding** — Verify short and long forms (stloc.N, stloc.s).
3. **Arithmetic operations** — Verify add, sub, mul, div opcodes.
4. **End-to-end** — Full AST to bytecode verification.
5. **Edge cases** — Inline encoding, many variables, etc.
"""

from __future__ import annotations

import struct

import pytest

from lang_parser import (
    Assignment,
    BinaryOp,
    Name,
    NumberLiteral,
    Program,
    StringLiteral,
)

from bytecode_compiler.clr_compiler import (
    ADD,
    CLRCodeObject,
    CLRCompiler,
    DIV,
    LDC_I4,
    LDC_I4_0,
    LDC_I4_1,
    LDC_I4_8,
    LDC_I4_S,
    LDLOC_0,
    LDLOC_1,
    LDLOC_S,
    MUL,
    POP,
    RET,
    STLOC_0,
    STLOC_1,
    STLOC_3,
    STLOC_S,
    SUB,
)


# =========================================================================
# Helpers
# =========================================================================


def compile_ast(program: Program) -> CLRCodeObject:
    """Shortcut: compile a Program AST into a CLRCodeObject."""
    return CLRCompiler().compile(program)


# =========================================================================
# Number encoding tests
# =========================================================================


class TestNumberEncoding:
    """Verify the CLR's tiered number encoding: ldc.i4.N, ldc.i4.s, ldc.i4."""

    def test_ldc_i4_0(self) -> None:
        """Number 0 should use ldc.i4.0 (single byte 0x16)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=0))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == LDC_I4_0

    def test_ldc_i4_1(self) -> None:
        """Number 1 should use ldc.i4.1 (single byte 0x17)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == LDC_I4_1

    def test_ldc_i4_8(self) -> None:
        """Number 8 should use ldc.i4.8 (single byte 0x1E)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=8))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == LDC_I4_8

    def test_ldc_i4_s_for_9(self) -> None:
        """Number 9 exceeds ldc.i4.N range, should use ldc.i4.s (0x1F, 0x09)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=9))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == LDC_I4_S
        assert code.bytecode[1] == 9

    def test_ldc_i4_s_for_100(self) -> None:
        """Number 100 should use ldc.i4.s (0x1F, 0x64)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=100))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == LDC_I4_S
        assert code.bytecode[1] == 100

    def test_ldc_i4_s_for_127(self) -> None:
        """Number 127 (max positive signed byte) should use ldc.i4.s."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=127))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == LDC_I4_S
        assert code.bytecode[1] == 127

    def test_ldc_i4_s_for_negative(self) -> None:
        """Negative numbers in -128 to -1 range should use ldc.i4.s."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=-1))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == LDC_I4_S
        assert code.bytecode[1] == 0xFF  # -1 as unsigned byte

    def test_ldc_i4_for_128(self) -> None:
        """Number 128 exceeds ldc.i4.s range, should use ldc.i4 (5 bytes)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=128))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == LDC_I4
        # Next 4 bytes should be 128 as little-endian int32
        value_bytes = code.bytecode[1:5]
        assert struct.unpack("<i", value_bytes)[0] == 128

    def test_ldc_i4_for_large_number(self) -> None:
        """Large numbers should use ldc.i4 with 4-byte little-endian encoding."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=100000))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == LDC_I4
        value_bytes = code.bytecode[1:5]
        assert struct.unpack("<i", value_bytes)[0] == 100000

    def test_ldc_i4_for_negative_129(self) -> None:
        """Number -129 exceeds ldc.i4.s range, should use ldc.i4."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=-129))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == LDC_I4
        value_bytes = code.bytecode[1:5]
        assert struct.unpack("<i", value_bytes)[0] == -129


# =========================================================================
# Local variable encoding tests
# =========================================================================


class TestLocalVariableEncoding:
    """Verify the CLR's tiered local variable encoding."""

    def test_stloc_0(self) -> None:
        """First variable should use stloc.0 (0x0A)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[1] == STLOC_0

    def test_stloc_1(self) -> None:
        """Second variable should use stloc.1 (0x0B)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1)),
                Assignment(target=Name(name="y"), value=NumberLiteral(value=2)),
            ]
        )
        code = compile_ast(program)
        # bytecode: ldc.i4.1, stloc.0, ldc.i4.2, stloc.1, ret
        assert code.bytecode[3] == STLOC_1

    def test_stloc_3(self) -> None:
        """Fourth variable should use stloc.3 (0x0D)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="a"), value=NumberLiteral(value=0)),
                Assignment(target=Name(name="b"), value=NumberLiteral(value=1)),
                Assignment(target=Name(name="c"), value=NumberLiteral(value=2)),
                Assignment(target=Name(name="d"), value=NumberLiteral(value=3)),
            ]
        )
        code = compile_ast(program)
        # Each assignment: 1 byte ldc.i4.N + 1 byte stloc.N = 2 bytes
        # d is at byte offset 7
        assert code.bytecode[7] == STLOC_3

    def test_stloc_s_for_slot_4(self) -> None:
        """Fifth+ variable should use stloc.s (0x13) + index byte."""
        program = Program(
            statements=[
                Assignment(target=Name(name="a"), value=NumberLiteral(value=0)),
                Assignment(target=Name(name="b"), value=NumberLiteral(value=1)),
                Assignment(target=Name(name="c"), value=NumberLiteral(value=2)),
                Assignment(target=Name(name="d"), value=NumberLiteral(value=3)),
                Assignment(target=Name(name="e"), value=NumberLiteral(value=4)),
            ]
        )
        code = compile_ast(program)
        # First 4 assignments: 8 bytes, then ldc.i4.4 (1 byte), stloc.s, 4
        assert code.bytecode[9] == STLOC_S
        assert code.bytecode[10] == 4

    def test_ldloc_0(self) -> None:
        """Loading first variable should use ldloc.0 (0x06)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1)),
                Assignment(target=Name(name="y"), value=Name(name="x")),
            ]
        )
        code = compile_ast(program)
        # bytecode: ldc.i4.1, stloc.0, ldloc.0, stloc.1, ret
        assert code.bytecode[2] == LDLOC_0

    def test_ldloc_1(self) -> None:
        """Loading second variable should use ldloc.1 (0x07)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1)),
                Assignment(target=Name(name="y"), value=NumberLiteral(value=2)),
                Assignment(target=Name(name="z"), value=Name(name="y")),
            ]
        )
        code = compile_ast(program)
        # bytecode: ldc.i4.1, stloc.0, ldc.i4.2, stloc.1, ldloc.1, stloc.2, ret
        assert code.bytecode[4] == LDLOC_1

    def test_ldloc_s_for_slot_4(self) -> None:
        """Loading 5th+ variable should use ldloc.s (0x11) + index byte."""
        program = Program(
            statements=[
                Assignment(target=Name(name="a"), value=NumberLiteral(value=0)),
                Assignment(target=Name(name="b"), value=NumberLiteral(value=1)),
                Assignment(target=Name(name="c"), value=NumberLiteral(value=2)),
                Assignment(target=Name(name="d"), value=NumberLiteral(value=3)),
                Assignment(target=Name(name="e"), value=NumberLiteral(value=4)),
                Assignment(target=Name(name="f"), value=Name(name="e")),
            ]
        )
        code = compile_ast(program)
        # After 5 assignments (11 bytes), we have ldloc.s 4, stloc.s 5
        assert code.bytecode[11] == LDLOC_S
        assert code.bytecode[12] == 4


# =========================================================================
# Arithmetic operation tests
# =========================================================================


class TestArithmeticOps:
    """Verify that binary operations emit correct CLR opcodes."""

    def test_add(self) -> None:
        """Addition should emit add (0x58)."""
        program = Program(
            statements=[
                Assignment(
                    target=Name(name="x"),
                    value=BinaryOp(
                        left=NumberLiteral(value=1),
                        op="+",
                        right=NumberLiteral(value=2),
                    ),
                )
            ]
        )
        code = compile_ast(program)
        assert ADD in code.bytecode

    def test_sub(self) -> None:
        """Subtraction should emit sub (0x59)."""
        program = Program(
            statements=[
                Assignment(
                    target=Name(name="x"),
                    value=BinaryOp(
                        left=NumberLiteral(value=5),
                        op="-",
                        right=NumberLiteral(value=3),
                    ),
                )
            ]
        )
        code = compile_ast(program)
        assert SUB in code.bytecode

    def test_mul(self) -> None:
        """Multiplication should emit mul (0x5A)."""
        program = Program(
            statements=[
                Assignment(
                    target=Name(name="x"),
                    value=BinaryOp(
                        left=NumberLiteral(value=4),
                        op="*",
                        right=NumberLiteral(value=3),
                    ),
                )
            ]
        )
        code = compile_ast(program)
        assert MUL in code.bytecode

    def test_div(self) -> None:
        """Division should emit div (0x5B)."""
        program = Program(
            statements=[
                Assignment(
                    target=Name(name="x"),
                    value=BinaryOp(
                        left=NumberLiteral(value=10),
                        op="/",
                        right=NumberLiteral(value=2),
                    ),
                )
            ]
        )
        code = compile_ast(program)
        assert DIV in code.bytecode


# =========================================================================
# End-to-end bytecode verification
# =========================================================================


class TestEndToEnd:
    """Verify complete bytecode sequences for known programs."""

    def test_x_equals_1_plus_2(self) -> None:
        """``x = 1 + 2`` should produce: ldc.i4.1, ldc.i4.2, add, stloc.0, ret."""
        program = Program(
            statements=[
                Assignment(
                    target=Name(name="x"),
                    value=BinaryOp(
                        left=NumberLiteral(value=1),
                        op="+",
                        right=NumberLiteral(value=2),
                    ),
                )
            ]
        )
        code = compile_ast(program)

        expected = bytes([
            LDC_I4_1,   # ldc.i4.1
            LDC_I4_1 + 1,  # ldc.i4.2
            ADD,        # add
            STLOC_0,    # stloc.0
            RET,        # ret
        ])
        assert code.bytecode == expected

    def test_x_equals_100(self) -> None:
        """``x = 100`` should use ldc.i4.s encoding."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=100))
            ]
        )
        code = compile_ast(program)

        expected = bytes([
            LDC_I4_S,   # ldc.i4.s
            100,        # value 100
            STLOC_0,    # stloc.0
            RET,        # ret
        ])
        assert code.bytecode == expected

    def test_expression_statement_emits_pop(self) -> None:
        """A bare expression statement should emit pop (0x26) after evaluation."""
        program = Program(
            statements=[
                BinaryOp(
                    left=NumberLiteral(value=1),
                    op="+",
                    right=NumberLiteral(value=2),
                )
            ]
        )
        code = compile_ast(program)

        expected = bytes([
            LDC_I4_1,      # ldc.i4.1
            LDC_I4_1 + 1,  # ldc.i4.2
            ADD,           # add
            POP,           # pop (discard result)
            RET,           # ret
        ])
        assert code.bytecode == expected

    def test_two_assignments(self) -> None:
        """Two assignments use different local slots."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1)),
                Assignment(target=Name(name="y"), value=NumberLiteral(value=2)),
            ]
        )
        code = compile_ast(program)

        expected = bytes([
            LDC_I4_1,      # ldc.i4.1
            STLOC_0,       # stloc.0 (x)
            LDC_I4_1 + 1,  # ldc.i4.2
            STLOC_1,       # stloc.1 (y)
            RET,           # ret
        ])
        assert code.bytecode == expected

    def test_empty_program(self) -> None:
        """An empty program should just produce ret."""
        program = Program(statements=[])
        code = compile_ast(program)

        assert code.bytecode == bytes([RET])
        assert code.num_locals == 0
        assert code.local_names == []

    def test_ends_with_ret(self) -> None:
        """Every compiled program should end with ret (0x2A)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=42))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[-1] == RET

    def test_ldc_i4_end_to_end(self) -> None:
        """``x = 1000`` should use ldc.i4 with 4-byte little-endian."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1000))
            ]
        )
        code = compile_ast(program)

        expected = bytes([LDC_I4]) + struct.pack("<i", 1000) + bytes([STLOC_0, RET])
        assert code.bytecode == expected


# =========================================================================
# Local names mapping tests
# =========================================================================


class TestLocalNames:
    """Verify that local_names correctly maps slot indices to variable names."""

    def test_single_variable(self) -> None:
        """One variable: slot 0 = x."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1))
            ]
        )
        code = compile_ast(program)
        assert code.local_names == ["x"]
        assert code.num_locals == 1

    def test_multiple_variables(self) -> None:
        """Multiple variables: slots assigned in order of first appearance."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1)),
                Assignment(target=Name(name="y"), value=NumberLiteral(value=2)),
                Assignment(target=Name(name="z"), value=NumberLiteral(value=3)),
            ]
        )
        code = compile_ast(program)
        assert code.local_names == ["x", "y", "z"]
        assert code.num_locals == 3

    def test_reassignment_reuses_slot(self) -> None:
        """Reassigning a variable should reuse its existing slot."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1)),
                Assignment(target=Name(name="x"), value=NumberLiteral(value=2)),
            ]
        )
        code = compile_ast(program)
        assert code.local_names == ["x"]
        assert code.num_locals == 1


# =========================================================================
# Return type tests
# =========================================================================


class TestReturnType:
    """Verify the compiler returns proper CLRCodeObject instances."""

    def test_returns_clr_code_object(self) -> None:
        """compile() should return a CLRCodeObject instance."""
        program = Program(statements=[NumberLiteral(value=1)])
        code = compile_ast(program)
        assert isinstance(code, CLRCodeObject)

    def test_bytecode_is_bytes(self) -> None:
        """The bytecode field should be immutable bytes, not bytearray."""
        program = Program(statements=[NumberLiteral(value=1)])
        code = compile_ast(program)
        assert isinstance(code.bytecode, bytes)


# =========================================================================
# Error handling tests
# =========================================================================


class TestErrorHandling:
    """Verify the compiler raises appropriate errors."""

    def test_unknown_expression_raises_type_error(self) -> None:
        """Passing an unrecognized AST node should raise TypeError."""

        class FakeNode:
            pass

        compiler = CLRCompiler()
        with pytest.raises(TypeError, match="Unknown expression type"):
            compiler._compile_expression(FakeNode())  # type: ignore[arg-type]

    def test_string_literal_raises_type_error(self) -> None:
        """String literals are not yet supported in the CLR compiler."""
        program = Program(
            statements=[
                Assignment(
                    target=Name(name="x"), value=StringLiteral(value="hello")
                )
            ]
        )
        with pytest.raises(TypeError, match="CLR compiler does not support string"):
            compile_ast(program)
