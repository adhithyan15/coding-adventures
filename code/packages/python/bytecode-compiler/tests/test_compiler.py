"""Comprehensive tests for the Bytecode Compiler.

These tests verify that the compiler correctly translates AST nodes into
bytecode instructions. We test at two levels:

1. **Unit tests** — Feed hand-built AST nodes into the compiler and verify
   the exact instructions, constants, and names that come out. These tests
   are precise and isolated from the lexer/parser.

2. **End-to-end tests** — Use ``compile_source`` to go from source code all
   the way to a CodeObject, then execute it on the VM and check the results.
   These tests verify that the full pipeline works together.
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
from virtual_machine import CodeObject, Instruction, OpCode, VirtualMachine

from bytecode_compiler import BytecodeCompiler, compile_source


# =========================================================================
# Helpers
# =========================================================================


def opcodes(code: CodeObject) -> list[OpCode]:
    """Extract just the opcodes from a CodeObject, for quick comparison."""
    return [instr.opcode for instr in code.instructions]


def operands(code: CodeObject) -> list[int | str | None]:
    """Extract just the operands from a CodeObject, for quick comparison."""
    return [instr.operand for instr in code.instructions]


def compile_ast(program: Program) -> CodeObject:
    """Shortcut: compile a Program AST into a CodeObject."""
    return BytecodeCompiler().compile(program)


# =========================================================================
# Unit tests — AST node to bytecode
# =========================================================================


class TestNumberLiteral:
    """Compiling a bare number literal like ``42``."""

    def test_number_literal_produces_load_const_pop_halt(self) -> None:
        """A number expression statement should: load the constant, pop it
        (because it's not assigned), then halt."""
        program = Program(statements=[NumberLiteral(value=42)])
        code = compile_ast(program)

        assert opcodes(code) == [OpCode.LOAD_CONST, OpCode.POP, OpCode.HALT]
        assert operands(code) == [0, None, None]
        assert code.constants == [42]
        assert code.names == []

    def test_number_literal_zero(self) -> None:
        """Zero is a valid constant and should be handled normally."""
        program = Program(statements=[NumberLiteral(value=0)])
        code = compile_ast(program)

        assert code.constants == [0]
        assert opcodes(code) == [OpCode.LOAD_CONST, OpCode.POP, OpCode.HALT]


class TestStringLiteral:
    """Compiling a bare string literal like ``"hello"``."""

    def test_string_literal_produces_load_const_pop_halt(self) -> None:
        """A string expression statement: load, pop (unused), halt."""
        program = Program(statements=[StringLiteral(value="hello")])
        code = compile_ast(program)

        assert opcodes(code) == [OpCode.LOAD_CONST, OpCode.POP, OpCode.HALT]
        assert operands(code) == [0, None, None]
        assert code.constants == ["hello"]
        assert code.names == []

    def test_empty_string(self) -> None:
        """Empty strings are valid constants."""
        program = Program(statements=[StringLiteral(value="")])
        code = compile_ast(program)

        assert code.constants == [""]


class TestNameReference:
    """Compiling a variable reference like ``x``."""

    def test_name_produces_load_name_pop_halt(self) -> None:
        """A bare name reference: look up the variable, pop the result, halt."""
        program = Program(statements=[Name(name="x")])
        code = compile_ast(program)

        assert opcodes(code) == [OpCode.LOAD_NAME, OpCode.POP, OpCode.HALT]
        assert operands(code) == [0, None, None]
        assert code.constants == []
        assert code.names == ["x"]


class TestAssignment:
    """Compiling assignments like ``x = 42``."""

    def test_simple_assignment(self) -> None:
        """``x = 42`` should: load 42, store in x, halt. No POP needed because
        STORE_NAME already pops the value."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=42))
            ]
        )
        code = compile_ast(program)

        assert opcodes(code) == [OpCode.LOAD_CONST, OpCode.STORE_NAME, OpCode.HALT]
        assert operands(code) == [0, 0, None]
        assert code.constants == [42]
        assert code.names == ["x"]

    def test_assignment_with_string(self) -> None:
        """``name = "alice"`` should store a string constant."""
        program = Program(
            statements=[
                Assignment(
                    target=Name(name="name"),
                    value=StringLiteral(value="alice"),
                )
            ]
        )
        code = compile_ast(program)

        assert code.constants == ["alice"]
        assert code.names == ["name"]
        assert opcodes(code) == [OpCode.LOAD_CONST, OpCode.STORE_NAME, OpCode.HALT]


class TestBinaryOp:
    """Compiling binary operations like ``1 + 2``."""

    def test_addition(self) -> None:
        """``1 + 2`` as an expression statement: load both, add, pop, halt."""
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

        assert opcodes(code) == [
            OpCode.LOAD_CONST,
            OpCode.LOAD_CONST,
            OpCode.ADD,
            OpCode.POP,
            OpCode.HALT,
        ]
        assert code.constants == [1, 2]

    def test_subtraction(self) -> None:
        """``5 - 3`` should emit SUB."""
        program = Program(
            statements=[
                BinaryOp(
                    left=NumberLiteral(value=5),
                    op="-",
                    right=NumberLiteral(value=3),
                )
            ]
        )
        code = compile_ast(program)

        assert OpCode.SUB in opcodes(code)

    def test_multiplication(self) -> None:
        """``4 * 7`` should emit MUL."""
        program = Program(
            statements=[
                BinaryOp(
                    left=NumberLiteral(value=4),
                    op="*",
                    right=NumberLiteral(value=7),
                )
            ]
        )
        code = compile_ast(program)

        assert OpCode.MUL in opcodes(code)

    def test_division(self) -> None:
        """``10 / 2`` should emit DIV."""
        program = Program(
            statements=[
                BinaryOp(
                    left=NumberLiteral(value=10),
                    op="/",
                    right=NumberLiteral(value=2),
                )
            ]
        )
        code = compile_ast(program)

        assert OpCode.DIV in opcodes(code)


class TestComplexExpressions:
    """Compiling nested expressions that test precedence and structure."""

    def test_assignment_with_binary_op(self) -> None:
        """``x = 1 + 2`` should compile the addition, then store in x."""
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

        assert opcodes(code) == [
            OpCode.LOAD_CONST,  # 1
            OpCode.LOAD_CONST,  # 2
            OpCode.ADD,
            OpCode.STORE_NAME,  # x
            OpCode.HALT,
        ]
        assert code.constants == [1, 2]
        assert code.names == ["x"]

    def test_nested_binary_ops_respects_tree_structure(self) -> None:
        """``x = 1 + 2 * 3`` — the parser builds the tree with * binding tighter,
        so the compiler should emit the multiplication before the addition.

        AST:
            Assignment(x, BinaryOp(1, +, BinaryOp(2, *, 3)))

        Expected:
            LOAD_CONST 0 (1)
            LOAD_CONST 1 (2)
            LOAD_CONST 2 (3)
            MUL
            ADD
            STORE_NAME 0 (x)
            HALT
        """
        program = Program(
            statements=[
                Assignment(
                    target=Name(name="x"),
                    value=BinaryOp(
                        left=NumberLiteral(value=1),
                        op="+",
                        right=BinaryOp(
                            left=NumberLiteral(value=2),
                            op="*",
                            right=NumberLiteral(value=3),
                        ),
                    ),
                )
            ]
        )
        code = compile_ast(program)

        assert opcodes(code) == [
            OpCode.LOAD_CONST,  # 1
            OpCode.LOAD_CONST,  # 2
            OpCode.LOAD_CONST,  # 3
            OpCode.MUL,
            OpCode.ADD,
            OpCode.STORE_NAME,  # x
            OpCode.HALT,
        ]
        assert code.constants == [1, 2, 3]

    def test_binary_op_with_name_operands(self) -> None:
        """``a + b`` should emit LOAD_NAME for both operands."""
        program = Program(
            statements=[
                BinaryOp(
                    left=Name(name="a"),
                    op="+",
                    right=Name(name="b"),
                )
            ]
        )
        code = compile_ast(program)

        assert opcodes(code) == [
            OpCode.LOAD_NAME,
            OpCode.LOAD_NAME,
            OpCode.ADD,
            OpCode.POP,
            OpCode.HALT,
        ]
        assert code.names == ["a", "b"]


class TestMultipleStatements:
    """Programs with more than one statement."""

    def test_two_assignments(self) -> None:
        """``x = 1`` then ``y = 2`` — each gets its own constant and name."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1)),
                Assignment(target=Name(name="y"), value=NumberLiteral(value=2)),
            ]
        )
        code = compile_ast(program)

        assert opcodes(code) == [
            OpCode.LOAD_CONST,  # 1
            OpCode.STORE_NAME,  # x
            OpCode.LOAD_CONST,  # 2
            OpCode.STORE_NAME,  # y
            OpCode.HALT,
        ]
        assert code.constants == [1, 2]
        assert code.names == ["x", "y"]

    def test_assignment_then_expression(self) -> None:
        """``x = 42`` then ``x`` — the second statement is an expression
        that should be popped."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=42)),
                Name(name="x"),
            ]
        )
        code = compile_ast(program)

        assert opcodes(code) == [
            OpCode.LOAD_CONST,  # 42
            OpCode.STORE_NAME,  # x
            OpCode.LOAD_NAME,   # x
            OpCode.POP,
            OpCode.HALT,
        ]


class TestDeduplication:
    """Constant and name pools should deduplicate entries."""

    def test_constant_deduplication(self) -> None:
        """Using the same number twice should reuse the same constant index."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1)),
                Assignment(target=Name(name="y"), value=NumberLiteral(value=1)),
            ]
        )
        code = compile_ast(program)

        # Both LOAD_CONST instructions should reference index 0
        assert code.constants == [1]
        load_consts = [
            i for i in code.instructions if i.opcode == OpCode.LOAD_CONST
        ]
        assert all(i.operand == 0 for i in load_consts)

    def test_name_deduplication(self) -> None:
        """Referencing the same variable twice should reuse the same name index."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=1)),
                Assignment(target=Name(name="x"), value=NumberLiteral(value=2)),
            ]
        )
        code = compile_ast(program)

        # Both STORE_NAME instructions should reference index 0
        assert code.names == ["x"]
        store_names = [
            i for i in code.instructions if i.opcode == OpCode.STORE_NAME
        ]
        assert all(i.operand == 0 for i in store_names)

    def test_mixed_deduplication(self) -> None:
        """Constants and names are deduplicated independently."""
        program = Program(
            statements=[
                Assignment(target=Name(name="x"), value=NumberLiteral(value=5)),
                Assignment(
                    target=Name(name="y"),
                    value=BinaryOp(
                        left=Name(name="x"),
                        op="+",
                        right=NumberLiteral(value=5),
                    ),
                ),
            ]
        )
        code = compile_ast(program)

        # 5 appears twice but should be stored once
        assert code.constants == [5]
        # x appears in both statements (store and load), should be stored once
        assert "x" in code.names
        assert code.names.count("x") == 0 or code.names.count("x") == 1
        # Actually check the name pool has no duplicates
        assert len(code.names) == len(set(code.names))


class TestEmptyProgram:
    """Edge case: a program with no statements."""

    def test_empty_program_produces_just_halt(self) -> None:
        """An empty program should still produce a valid CodeObject with HALT."""
        program = Program(statements=[])
        code = compile_ast(program)

        assert opcodes(code) == [OpCode.HALT]
        assert code.constants == []
        assert code.names == []


class TestCompilerReturnType:
    """Verify the compiler returns proper CodeObject instances."""

    def test_returns_code_object(self) -> None:
        """compile() should return a CodeObject instance."""
        program = Program(statements=[NumberLiteral(value=1)])
        code = compile_ast(program)

        assert isinstance(code, CodeObject)

    def test_code_object_has_instructions(self) -> None:
        """The CodeObject should have an instructions list."""
        program = Program(statements=[NumberLiteral(value=1)])
        code = compile_ast(program)

        assert isinstance(code.instructions, list)
        assert all(isinstance(i, Instruction) for i in code.instructions)


class TestUnknownExpressionType:
    """The compiler should raise TypeError for unknown AST nodes."""

    def test_unknown_expression_raises_type_error(self) -> None:
        """Passing an unrecognized AST node should raise TypeError."""

        class FakeNode:
            pass

        compiler = BytecodeCompiler()
        with pytest.raises(TypeError, match="Unknown expression type"):
            compiler._compile_expression(FakeNode())  # type: ignore[arg-type]


# =========================================================================
# End-to-end tests — source code -> VM execution
# =========================================================================


class TestEndToEnd:
    """Full pipeline: source code -> lexer -> parser -> compiler -> VM."""

    def test_simple_assignment(self) -> None:
        """``x = 1 + 2`` should result in x == 3."""
        code = compile_source("x = 1 + 2")
        vm = VirtualMachine()
        vm.execute(code)

        assert vm.variables["x"] == 3

    def test_multiple_assignments(self) -> None:
        """Multiple assignments should all be accessible in the VM."""
        code = compile_source("a = 10\nb = 20\nc = a + b")
        vm = VirtualMachine()
        vm.execute(code)

        assert vm.variables["a"] == 10
        assert vm.variables["b"] == 20
        assert vm.variables["c"] == 30

    def test_arithmetic_operations(self) -> None:
        """Test all four arithmetic operations end-to-end."""
        code = compile_source("a = 10 + 5\nb = 10 - 5\nc = 10 * 5\nd = 10 / 5")
        vm = VirtualMachine()
        vm.execute(code)

        assert vm.variables["a"] == 15
        assert vm.variables["b"] == 5
        assert vm.variables["c"] == 50
        assert vm.variables["d"] == 2

    def test_expression_with_precedence(self) -> None:
        """``x = 2 + 3 * 4`` should respect multiplication precedence."""
        code = compile_source("x = 2 + 3 * 4")
        vm = VirtualMachine()
        vm.execute(code)

        assert vm.variables["x"] == 14  # 2 + (3 * 4) = 14, not (2+3)*4 = 20

    def test_variable_reuse(self) -> None:
        """A variable can be assigned, then used in a later expression."""
        code = compile_source("x = 10\ny = x + 5")
        vm = VirtualMachine()
        vm.execute(code)

        assert vm.variables["x"] == 10
        assert vm.variables["y"] == 15

    def test_variable_reassignment(self) -> None:
        """A variable can be reassigned to a new value."""
        code = compile_source("x = 1\nx = 2")
        vm = VirtualMachine()
        vm.execute(code)

        assert vm.variables["x"] == 2

    def test_compile_source_returns_code_object(self) -> None:
        """compile_source should return a CodeObject."""
        code = compile_source("x = 42")
        assert isinstance(code, CodeObject)

    def test_compile_source_with_keywords(self) -> None:
        """compile_source should accept optional keywords parameter."""
        # Keywords shouldn't affect simple expressions, but the parameter
        # should be accepted without error.
        code = compile_source("x = 1", keywords=["if", "else"])
        vm = VirtualMachine()
        vm.execute(code)

        assert vm.variables["x"] == 1

    def test_chain_of_operations(self) -> None:
        """A longer program exercising multiple features."""
        source = "a = 1\nb = 2\nc = 3\nresult = a + b * c"
        code = compile_source(source)
        vm = VirtualMachine()
        vm.execute(code)

        # b * c = 6, a + 6 = 7
        assert vm.variables["result"] == 7


class TestCompileSourceConvenience:
    """Tests specifically for the compile_source helper function."""

    def test_basic_usage(self) -> None:
        """Simplest possible usage."""
        code = compile_source("42")
        assert isinstance(code, CodeObject)
        assert len(code.instructions) > 0

    def test_ends_with_halt(self) -> None:
        """Every compiled program should end with HALT."""
        code = compile_source("x = 1")
        assert code.instructions[-1].opcode == OpCode.HALT
