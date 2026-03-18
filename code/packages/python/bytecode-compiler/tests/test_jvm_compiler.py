"""Comprehensive tests for the JVM Bytecode Compiler.

These tests verify that the JVM compiler correctly translates AST nodes into
real JVM bytecode bytes. We test at multiple levels:

1. **Number encoding** — Verify the tiered encoding (iconst, bipush, ldc).
2. **Local variable encoding** — Verify short and long forms (istore_N, istore).
3. **Arithmetic operations** — Verify iadd, isub, imul, idiv opcodes.
4. **End-to-end** — Full AST to bytecode verification.
5. **Edge cases** — Constant deduplication, many variables, etc.
"""

from __future__ import annotations

import pytest

from lang_parser import (
    Assignment,
    BinaryOp,
    Name,
    NumberLiteral,
    Program,
    StringLiteral,
)

from bytecode_compiler.jvm_compiler import (
    BIPUSH,
    IADD,
    ICONST_0,
    ICONST_1,
    ICONST_5,
    IDIV,
    ILOAD,
    ILOAD_0,
    ILOAD_1,
    IMUL,
    ISTORE,
    ISTORE_0,
    ISTORE_1,
    ISTORE_3,
    ISUB,
    JVMCodeObject,
    JVMCompiler,
    LDC,
    POP,
    RETURN,
)


# =========================================================================
# Helpers
# =========================================================================


def compile_ast(program: Program) -> JVMCodeObject:
    """Shortcut: compile a Program AST into a JVMCodeObject."""
    return JVMCompiler().compile(program)


# =========================================================================
# Number encoding tests
# =========================================================================


class TestNumberEncoding:
    """Verify the JVM's tiered number encoding: iconst, bipush, ldc."""

    def test_iconst_0(self) -> None:
        """Number 0 should use iconst_0 (single byte 0x03)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=0))
            ]
        )
        code = compile_ast(program)

        # iconst_0 (0x03), istore_0 (0x3B), return (0xB1)
        assert code.bytecode[0] == ICONST_0

    def test_iconst_1(self) -> None:
        """Number 1 should use iconst_1 (single byte 0x04)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == ICONST_1

    def test_iconst_5(self) -> None:
        """Number 5 should use iconst_5 (single byte 0x08)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=5))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == ICONST_5

    def test_bipush_for_6(self) -> None:
        """Number 6 exceeds iconst range, should use bipush (0x10, 0x06)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=6))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == BIPUSH
        assert code.bytecode[1] == 6

    def test_bipush_for_100(self) -> None:
        """Number 100 should use bipush (0x10, 0x64)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=100))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == BIPUSH
        assert code.bytecode[1] == 100

    def test_bipush_for_127(self) -> None:
        """Number 127 (max bipush positive) should use bipush."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=127))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == BIPUSH
        assert code.bytecode[1] == 127

    def test_bipush_for_negative(self) -> None:
        """Negative numbers in -128 to -1 range should use bipush."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=-1))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == BIPUSH
        assert code.bytecode[1] == 0xFF  # -1 as unsigned byte

    def test_ldc_for_128(self) -> None:
        """Number 128 exceeds bipush range, should use ldc + constant pool."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=128))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == LDC
        assert code.bytecode[1] == 0  # constant pool index 0
        assert code.constants == [128]

    def test_ldc_for_large_number(self) -> None:
        """Large numbers should use ldc with constant pool."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1000))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == LDC
        assert 1000 in code.constants

    def test_ldc_for_negative_129(self) -> None:
        """Number -129 exceeds bipush range, should use ldc."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=-129))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == LDC
        assert -129 in code.constants


# =========================================================================
# Local variable encoding tests
# =========================================================================


class TestLocalVariableEncoding:
    """Verify the JVM's tiered local variable encoding."""

    def test_istore_0(self) -> None:
        """First variable should use istore_0 (0x3B)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1))
            ]
        )
        code = compile_ast(program)
        # bytecode: iconst_1, istore_0, return
        assert code.bytecode[1] == ISTORE_0

    def test_istore_1(self) -> None:
        """Second variable should use istore_1 (0x3C)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1)),
                Assignment(target=Name(name="y"), value=NumberLiteral(value=2)),
            ]
        )
        code = compile_ast(program)
        # bytecode: iconst_1, istore_0, iconst_2, istore_1, return
        assert code.bytecode[3] == ISTORE_1

    def test_istore_3(self) -> None:
        """Fourth variable should use istore_3 (0x3E)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="a"), value=NumberLiteral(value=0)),
                Assignment(target=Name(name="b"), value=NumberLiteral(value=1)),
                Assignment(target=Name(name="c"), value=NumberLiteral(value=2)),
                Assignment(target=Name(name="d"), value=NumberLiteral(value=3)),
            ]
        )
        code = compile_ast(program)
        # a=slot0, b=slot1, c=slot2, d=slot3
        assert code.bytecode[7] == ISTORE_3

    def test_istore_generic_for_slot_4(self) -> None:
        """Fifth+ variable should use istore (0x36) + index byte."""
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
        # Find the last istore sequence — should be istore 4
        # The fifth assignment: iconst_4, istore, 4
        # Previous bytecodes: a(2), b(2), c(2), d(2) = 8 bytes, then iconst_4(1)
        assert code.bytecode[9] == ISTORE
        assert code.bytecode[10] == 4

    def test_iload_0(self) -> None:
        """Loading first variable should use iload_0 (0x1A)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1)),
                Assignment(target=Name(name="y"), value=Name(name="x")),
            ]
        )
        code = compile_ast(program)
        # bytecode: iconst_1, istore_0, iload_0, istore_1, return
        assert code.bytecode[2] == ILOAD_0

    def test_iload_1(self) -> None:
        """Loading second variable should use iload_1 (0x1B)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1)),
                Assignment(target=Name(name="y"), value=NumberLiteral(value=2)),
                Assignment(target=Name(name="z"), value=Name(name="y")),
            ]
        )
        code = compile_ast(program)
        # bytecode: iconst_1, istore_0, iconst_2, istore_1, iload_1, istore_2, return
        assert code.bytecode[4] == ILOAD_1

    def test_iload_generic_for_slot_4(self) -> None:
        """Loading 5th+ variable should use iload (0x15) + index byte."""
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
        # After 5 assignments (11 bytes), we have iload 4, istore ...
        assert code.bytecode[11] == ILOAD
        assert code.bytecode[12] == 4


# =========================================================================
# Arithmetic operation tests
# =========================================================================


class TestArithmeticOps:
    """Verify that binary operations emit correct JVM opcodes."""

    def test_iadd(self) -> None:
        """Addition should emit iadd (0x60)."""
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
        assert IADD in code.bytecode

    def test_isub(self) -> None:
        """Subtraction should emit isub (0x64)."""
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
        assert ISUB in code.bytecode

    def test_imul(self) -> None:
        """Multiplication should emit imul (0x68)."""
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
        assert IMUL in code.bytecode

    def test_idiv(self) -> None:
        """Division should emit idiv (0x6C)."""
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
        assert IDIV in code.bytecode


# =========================================================================
# End-to-end bytecode verification
# =========================================================================


class TestEndToEnd:
    """Verify complete bytecode sequences for known programs."""

    def test_x_equals_1_plus_2(self) -> None:
        """``x = 1 + 2`` should produce: iconst_1, iconst_2, iadd, istore_0, return."""
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
            ICONST_1,   # iconst_1 (push 1)
            ICONST_1 + 1,  # iconst_2 (push 2)
            IADD,       # iadd (1 + 2 = 3)
            ISTORE_0,   # istore_0 (store in x)
            RETURN,     # return
        ])
        assert code.bytecode == expected

    def test_x_equals_100(self) -> None:
        """``x = 100`` should use bipush encoding."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=100))
            ]
        )
        code = compile_ast(program)

        expected = bytes([
            BIPUSH,     # bipush
            100,        # value 100
            ISTORE_0,   # istore_0
            RETURN,     # return
        ])
        assert code.bytecode == expected

    def test_expression_statement_emits_pop(self) -> None:
        """A bare expression statement should emit pop (0x57) after evaluation."""
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
            ICONST_1,   # iconst_1
            ICONST_1 + 1,  # iconst_2
            IADD,       # iadd
            POP,        # pop (discard result)
            RETURN,     # return
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
            ICONST_1,   # iconst_1
            ISTORE_0,   # istore_0 (x)
            ICONST_1 + 1,  # iconst_2
            ISTORE_1,   # istore_1 (y)
            RETURN,     # return
        ])
        assert code.bytecode == expected

    def test_empty_program(self) -> None:
        """An empty program should just produce return."""
        program = Program(statements=[])
        code = compile_ast(program)

        assert code.bytecode == bytes([RETURN])
        assert code.constants == []
        assert code.num_locals == 0
        assert code.local_names == []

    def test_ends_with_return(self) -> None:
        """Every compiled program should end with return (0xB1)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=42))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[-1] == RETURN


# =========================================================================
# Constant pool tests
# =========================================================================


class TestConstantPool:
    """Verify constant pool management and deduplication."""

    def test_constant_deduplication(self) -> None:
        """Using the same large number twice should reuse the constant pool entry."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=200)),
                Assignment(target=Name(name="y"), value=NumberLiteral(value=200)),
            ]
        )
        code = compile_ast(program)

        # 200 should appear only once in the constant pool
        assert code.constants == [200]
        # Both ldc instructions should reference index 0
        ldc_indices = [
            code.bytecode[i + 1]
            for i in range(len(code.bytecode))
            if code.bytecode[i] == LDC
        ]
        assert ldc_indices == [0, 0]

    def test_different_constants_get_different_indices(self) -> None:
        """Different large numbers get separate constant pool entries."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=200)),
                Assignment(target=Name(name="y"), value=NumberLiteral(value=300)),
            ]
        )
        code = compile_ast(program)
        assert code.constants == [200, 300]

    def test_string_in_constant_pool(self) -> None:
        """String literals should be stored in the constant pool."""
        program = Program(
            statements=[
                Assignment(
                    target=Name(name="x"), value=StringLiteral(value="hello")
                )
            ]
        )
        code = compile_ast(program)
        assert "hello" in code.constants
        assert code.bytecode[0] == LDC


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
    """Verify the compiler returns proper JVMCodeObject instances."""

    def test_returns_jvm_code_object(self) -> None:
        """compile() should return a JVMCodeObject instance."""
        program = Program(statements=[NumberLiteral(value=1)])
        code = compile_ast(program)
        assert isinstance(code, JVMCodeObject)

    def test_bytecode_is_bytes(self) -> None:
        """The bytecode field should be immutable bytes, not bytearray."""
        program = Program(statements=[NumberLiteral(value=1)])
        code = compile_ast(program)
        assert isinstance(code.bytecode, bytes)


# =========================================================================
# Error handling tests
# =========================================================================


class TestErrorHandling:
    """Verify the compiler raises appropriate errors for unsupported nodes."""

    def test_unknown_expression_raises_type_error(self) -> None:
        """Passing an unrecognized AST node should raise TypeError."""

        class FakeNode:
            pass

        compiler = JVMCompiler()
        with pytest.raises(TypeError, match="Unknown expression type"):
            compiler._compile_expression(FakeNode())  # type: ignore[arg-type]
