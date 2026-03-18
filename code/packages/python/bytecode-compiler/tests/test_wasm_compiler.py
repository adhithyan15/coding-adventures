"""Comprehensive tests for the WASM Bytecode Compiler.

These tests verify that the WASM compiler correctly translates AST nodes into
real WebAssembly bytecode bytes. The WASM compiler is simpler than JVM/CLR
because WASM uses uniform encoding (no short forms), but we still test:

1. **Number encoding** — Verify i32.const with 4-byte little-endian.
2. **Local variable encoding** — Verify local.get/local.set with index byte.
3. **Arithmetic operations** — Verify i32.add, i32.sub, i32.mul, i32.div_s.
4. **End-to-end** — Full AST to bytecode verification.
5. **WASM-specific** — No pop for expression statements, end instruction.
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

from bytecode_compiler.wasm_compiler import (
    END,
    I32_ADD,
    I32_CONST,
    I32_DIV_S,
    I32_MUL,
    I32_SUB,
    LOCAL_GET,
    LOCAL_SET,
    WASMCodeObject,
    WASMCompiler,
)


# =========================================================================
# Helpers
# =========================================================================


def compile_ast(program: Program) -> WASMCodeObject:
    """Shortcut: compile a Program AST into a WASMCodeObject."""
    return WASMCompiler().compile(program)


def i32_const_bytes(value: int) -> bytes:
    """Build the expected bytes for an i32.const instruction."""
    return bytes([I32_CONST]) + struct.pack("<i", value)


# =========================================================================
# Number encoding tests
# =========================================================================


class TestNumberEncoding:
    """Verify WASM's uniform number encoding: always i32.const + 4 bytes."""

    def test_i32_const_0(self) -> None:
        """Number 0 should use i32.const + 4 zero bytes (no short form in WASM)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=0))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == I32_CONST
        value_bytes = code.bytecode[1:5]
        assert struct.unpack("<i", value_bytes)[0] == 0

    def test_i32_const_1(self) -> None:
        """Number 1 should use i32.const + 4-byte encoding."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == I32_CONST
        value_bytes = code.bytecode[1:5]
        assert struct.unpack("<i", value_bytes)[0] == 1

    def test_i32_const_42(self) -> None:
        """Number 42 should use i32.const + 4-byte encoding."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=42))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[:5] == i32_const_bytes(42)

    def test_i32_const_large(self) -> None:
        """Large number should use i32.const + 4-byte encoding."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=100000))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[:5] == i32_const_bytes(100000)

    def test_i32_const_negative(self) -> None:
        """Negative numbers should use i32.const with signed encoding."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=-1))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[0] == I32_CONST
        value_bytes = code.bytecode[1:5]
        assert struct.unpack("<i", value_bytes)[0] == -1

    def test_i32_const_max_negative(self) -> None:
        """Large negative numbers should work with signed 32-bit encoding."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=-1000))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[:5] == i32_const_bytes(-1000)


# =========================================================================
# Local variable encoding tests
# =========================================================================


class TestLocalVariableEncoding:
    """Verify WASM local variable access encoding."""

    def test_local_set_0(self) -> None:
        """First variable should use local.set 0 (0x21, 0x00)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1))
            ]
        )
        code = compile_ast(program)
        # i32.const 1 (5 bytes), local.set 0 (2 bytes), end (1 byte)
        assert code.bytecode[5] == LOCAL_SET
        assert code.bytecode[6] == 0

    def test_local_set_1(self) -> None:
        """Second variable should use local.set 1 (0x21, 0x01)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1)),
                Assignment(target=Name(name="y"), value=NumberLiteral(value=2)),
            ]
        )
        code = compile_ast(program)
        # First: i32.const(5) + local.set(2) = 7, then i32.const(5) + local.set(2)
        assert code.bytecode[12] == LOCAL_SET
        assert code.bytecode[13] == 1

    def test_local_get_0(self) -> None:
        """Loading first variable should use local.get 0 (0x20, 0x00)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1)),
                Assignment(target=Name(name="y"), value=Name(name="x")),
            ]
        )
        code = compile_ast(program)
        # First assignment: i32.const(5) + local.set(2) = 7 bytes
        # Then: local.get 0 (2 bytes)
        assert code.bytecode[7] == LOCAL_GET
        assert code.bytecode[8] == 0

    def test_local_get_1(self) -> None:
        """Loading second variable should use local.get 1."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1)),
                Assignment(target=Name(name="y"), value=NumberLiteral(value=2)),
                Assignment(target=Name(name="z"), value=Name(name="y")),
            ]
        )
        code = compile_ast(program)
        # Two assignments: 7 + 7 = 14 bytes, then local.get 1
        assert code.bytecode[14] == LOCAL_GET
        assert code.bytecode[15] == 1

    def test_many_locals(self) -> None:
        """Variables beyond the first few use increasing slot indices."""
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
        # Fifth variable should use slot 4
        # 4 assignments * 7 bytes = 28 bytes, then i32.const(5), local.set
        assert code.bytecode[33] == LOCAL_SET
        assert code.bytecode[34] == 4


# =========================================================================
# Arithmetic operation tests
# =========================================================================


class TestArithmeticOps:
    """Verify that binary operations emit correct WASM opcodes."""

    def test_i32_add(self) -> None:
        """Addition should emit i32.add (0x6A)."""
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
        assert I32_ADD in code.bytecode

    def test_i32_sub(self) -> None:
        """Subtraction should emit i32.sub (0x6B)."""
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
        assert I32_SUB in code.bytecode

    def test_i32_mul(self) -> None:
        """Multiplication should emit i32.mul (0x6C)."""
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
        assert I32_MUL in code.bytecode

    def test_i32_div_s(self) -> None:
        """Division should emit i32.div_s (0x6D)."""
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
        assert I32_DIV_S in code.bytecode


# =========================================================================
# End-to-end bytecode verification
# =========================================================================


class TestEndToEnd:
    """Verify complete bytecode sequences for known programs."""

    def test_x_equals_1_plus_2(self) -> None:
        """``x = 1 + 2`` should produce correct WASM bytecode."""
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

        expected = (
            i32_const_bytes(1)     # i32.const 1 (5 bytes)
            + i32_const_bytes(2)   # i32.const 2 (5 bytes)
            + bytes([I32_ADD])     # i32.add (1 byte)
            + bytes([LOCAL_SET, 0])  # local.set 0 (2 bytes)
            + bytes([END])         # end (1 byte)
        )
        assert code.bytecode == expected

    def test_no_pop_for_expression_statement(self) -> None:
        """WASM doesn't need pop for expression statements."""
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

        # No pop instruction — WASM handles stack at function boundary
        expected = (
            i32_const_bytes(1)
            + i32_const_bytes(2)
            + bytes([I32_ADD])
            + bytes([END])
        )
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

        expected = (
            i32_const_bytes(1)       # i32.const 1
            + bytes([LOCAL_SET, 0])   # local.set 0 (x)
            + i32_const_bytes(2)     # i32.const 2
            + bytes([LOCAL_SET, 1])   # local.set 1 (y)
            + bytes([END])           # end
        )
        assert code.bytecode == expected

    def test_empty_program(self) -> None:
        """An empty program should just produce end."""
        program = Program(statements=[])
        code = compile_ast(program)

        assert code.bytecode == bytes([END])
        assert code.num_locals == 0
        assert code.local_names == []

    def test_ends_with_end(self) -> None:
        """Every compiled program should end with end (0x0B)."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=42))
            ]
        )
        code = compile_ast(program)
        assert code.bytecode[-1] == END

    def test_variable_load_and_store(self) -> None:
        """``x = 1; y = x`` should load x and store into y."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1)),
                Assignment(target=Name(name="y"), value=Name(name="x")),
            ]
        )
        code = compile_ast(program)

        expected = (
            i32_const_bytes(1)       # i32.const 1
            + bytes([LOCAL_SET, 0])   # local.set 0 (x)
            + bytes([LOCAL_GET, 0])   # local.get 0 (x)
            + bytes([LOCAL_SET, 1])   # local.set 1 (y)
            + bytes([END])           # end
        )
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
    """Verify the compiler returns proper WASMCodeObject instances."""

    def test_returns_wasm_code_object(self) -> None:
        """compile() should return a WASMCodeObject instance."""
        program = Program(statements=[NumberLiteral(value=1)])
        code = compile_ast(program)
        assert isinstance(code, WASMCodeObject)

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

        compiler = WASMCompiler()
        with pytest.raises(TypeError, match="Unknown expression type"):
            compiler._compile_expression(FakeNode())  # type: ignore[arg-type]

    def test_string_literal_raises_type_error(self) -> None:
        """String literals are not yet supported in the WASM compiler."""
        program = Program(
            statements=[
                Assignment(
                    target=Name(name="x"), value=StringLiteral(value="hello")
                )
            ]
        )
        with pytest.raises(TypeError, match="WASM compiler does not support string"):
            compile_ast(program)
