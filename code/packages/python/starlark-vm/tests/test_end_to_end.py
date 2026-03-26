"""End-to-end tests: Starlark source → compile → execute → verify.

These tests exercise the complete pipeline:
    source code → starlark_lexer → tokens → starlark_parser → AST
    → starlark_ast_to_bytecode_compiler → bytecode → starlark_vm → result

Each test passes Starlark source code into the pipeline and checks
the final variable state and/or output.
"""

from __future__ import annotations

import pytest

from starlark_ast_to_bytecode_compiler import compile_starlark, create_starlark_ast_to_bytecode_compiler, Op
from starlark_vm import create_starlark_vm, StarlarkResult


# =========================================================================
# Helper
# =========================================================================


def run(source: str) -> StarlarkResult:
    """Compile and execute Starlark source, returning the result."""
    code = compile_starlark(source)
    vm = create_starlark_vm()
    traces = vm.execute(code)
    return StarlarkResult(
        variables=dict(vm.variables),
        output=list(vm.output),
        traces=traces,
    )


# =========================================================================
# Test: Basic Assignments
# =========================================================================


class TestBasicAssignment:
    """Test simple variable assignment."""

    def test_assign_int(self):
        result = run("x = 42\n")
        assert result.variables["x"] == 42

    def test_assign_float(self):
        result = run("x = 3.14\n")
        assert result.variables["x"] == 3.14

    def test_assign_string(self):
        result = run('x = "hello"\n')
        assert result.variables["x"] == "hello"

    def test_assign_true(self):
        result = run("x = True\n")
        assert result.variables["x"] is True

    def test_assign_false(self):
        result = run("x = False\n")
        assert result.variables["x"] is False

    def test_assign_none(self):
        result = run("x = None\n")
        assert result.variables["x"] is None

    def test_multiple_assignments(self):
        result = run("x = 1\ny = 2\nz = 3\n")
        assert result.variables["x"] == 1
        assert result.variables["y"] == 2
        assert result.variables["z"] == 3

    def test_reassignment(self):
        result = run("x = 1\nx = 2\n")
        assert result.variables["x"] == 2


# =========================================================================
# Test: Arithmetic
# =========================================================================


class TestArithmetic:
    """Test arithmetic operations."""

    def test_add(self):
        result = run("x = 1 + 2\n")
        assert result.variables["x"] == 3

    def test_subtract(self):
        result = run("x = 10 - 3\n")
        assert result.variables["x"] == 7

    def test_multiply(self):
        result = run("x = 4 * 5\n")
        assert result.variables["x"] == 20

    def test_divide(self):
        result = run("x = 10 / 4\n")
        assert result.variables["x"] == 2.5

    def test_floor_divide(self):
        result = run("x = 10 // 3\n")
        assert result.variables["x"] == 3

    def test_modulo(self):
        result = run("x = 10 % 3\n")
        assert result.variables["x"] == 1

    def test_power(self):
        result = run("x = 2 ** 10\n")
        assert result.variables["x"] == 1024

    def test_complex_expression(self):
        result = run("x = 2 + 3 * 4\n")
        assert result.variables["x"] == 14  # Precedence: 2 + (3*4)

    def test_parenthesized_expression(self):
        result = run("x = (2 + 3) * 4\n")
        assert result.variables["x"] == 20

    def test_unary_negate(self):
        result = run("x = -5\n")
        assert result.variables["x"] == -5

    def test_string_concatenation(self):
        result = run('x = "hello" + " " + "world"\n')
        assert result.variables["x"] == "hello world"

    def test_string_repeat(self):
        result = run('x = "ab" * 3\n')
        assert result.variables["x"] == "ababab"

    def test_variable_arithmetic(self):
        result = run("a = 10\nb = 20\nc = a + b\n")
        assert result.variables["c"] == 30


# =========================================================================
# Test: Comparisons
# =========================================================================


class TestComparisons:
    """Test comparison operations."""

    def test_equal(self):
        result = run("x = 1 == 1\n")
        assert result.variables["x"] is True

    def test_not_equal(self):
        result = run("x = 1 != 2\n")
        assert result.variables["x"] is True

    def test_less_than(self):
        result = run("x = 1 < 2\n")
        assert result.variables["x"] is True

    def test_greater_than(self):
        result = run("x = 2 > 1\n")
        assert result.variables["x"] is True

    def test_less_equal(self):
        result = run("x = 2 <= 2\n")
        assert result.variables["x"] is True

    def test_greater_equal(self):
        result = run("x = 3 >= 2\n")
        assert result.variables["x"] is True


# =========================================================================
# Test: Boolean Logic
# =========================================================================


class TestBooleanLogic:
    """Test boolean operations and short-circuit evaluation."""

    def test_not_true(self):
        result = run("x = not True\n")
        assert result.variables["x"] is False

    def test_not_false(self):
        result = run("x = not False\n")
        assert result.variables["x"] is True

    def test_and_true(self):
        result = run("x = True and True\n")
        assert result.variables["x"] is True

    def test_and_false(self):
        result = run("x = True and False\n")
        assert result.variables["x"] is False

    def test_or_true(self):
        result = run("x = False or True\n")
        assert result.variables["x"] is True

    def test_or_false(self):
        result = run("x = False or False\n")
        assert result.variables["x"] is False

    def test_and_short_circuit(self):
        """and returns first falsy value."""
        result = run("x = 0 and 42\n")
        assert result.variables["x"] == 0

    def test_or_short_circuit(self):
        """or returns first truthy value."""
        result = run("x = 0 or 42\n")
        assert result.variables["x"] == 42


# =========================================================================
# Test: Lists
# =========================================================================


class TestLists:
    """Test list operations."""

    def test_empty_list(self):
        result = run("x = []\n")
        assert result.variables["x"] == []

    def test_list_literal(self):
        result = run("x = [1, 2, 3]\n")
        assert result.variables["x"] == [1, 2, 3]

    def test_list_concatenation(self):
        result = run("x = [1, 2] + [3, 4]\n")
        assert result.variables["x"] == [1, 2, 3, 4]

    def test_list_repetition(self):
        result = run("x = [0] * 3\n")
        assert result.variables["x"] == [0, 0, 0]

    def test_list_indexing(self):
        result = run("x = [10, 20, 30]\ny = x[1]\n")
        assert result.variables["y"] == 20


# =========================================================================
# Test: Dicts
# =========================================================================


class TestDicts:
    """Test dictionary operations."""

    def test_empty_dict(self):
        result = run("x = {}\n")
        assert result.variables["x"] == {}

    def test_dict_literal(self):
        result = run('x = {"a": 1, "b": 2}\n')
        assert result.variables["x"] == {"a": 1, "b": 2}

    def test_dict_subscript(self):
        result = run('x = {"a": 42}\ny = x["a"]\n')
        assert result.variables["y"] == 42


# =========================================================================
# Test: Tuples
# =========================================================================


class TestTuples:
    """Test tuple operations."""

    def test_empty_tuple(self):
        result = run("x = ()\n")
        assert result.variables["x"] == ()

    def test_single_element_tuple(self):
        result = run("x = (1,)\n")
        assert result.variables["x"] == (1,)

    def test_multi_element_tuple(self):
        result = run("x = (1, 2, 3)\n")
        assert result.variables["x"] == (1, 2, 3)


# =========================================================================
# Test: If/Elif/Else
# =========================================================================


class TestIfStatements:
    """Test conditional execution."""

    def test_if_true(self):
        result = run("x = 0\nif True:\n    x = 1\n")
        assert result.variables["x"] == 1

    def test_if_false(self):
        result = run("x = 0\nif False:\n    x = 1\n")
        assert result.variables["x"] == 0

    def test_if_else_true(self):
        result = run("if True:\n    x = 1\nelse:\n    x = 2\n")
        assert result.variables["x"] == 1

    def test_if_else_false(self):
        result = run("if False:\n    x = 1\nelse:\n    x = 2\n")
        assert result.variables["x"] == 2

    def test_if_elif_else(self):
        src = "n = 2\nif n == 1:\n    x = 10\nelif n == 2:\n    x = 20\nelse:\n    x = 30\n"
        result = run(src)
        assert result.variables["x"] == 20

    def test_ternary_expression(self):
        result = run("x = 1 if True else 2\n")
        assert result.variables["x"] == 1

    def test_ternary_expression_false(self):
        result = run("x = 1 if False else 2\n")
        assert result.variables["x"] == 2


# =========================================================================
# Test: For Loops
# =========================================================================


class TestForLoops:
    """Test for loop execution."""

    def test_for_loop_sum(self):
        result = run("total = 0\nfor x in [1, 2, 3]:\n    total = total + x\n")
        assert result.variables["total"] == 6

    def test_for_loop_accumulate(self):
        result = run('result = ""\nfor c in ["a", "b", "c"]:\n    result = result + c\n')
        assert result.variables["result"] == "abc"

    def test_nested_for_loops(self):
        src = (
            "total = 0\n"
            "for i in [1, 2]:\n"
            "    for j in [10, 20]:\n"
            "        total = total + i * j\n"
        )
        result = run(src)
        # 1*10 + 1*20 + 2*10 + 2*20 = 10+20+20+40 = 90
        assert result.variables["total"] == 90


# =========================================================================
# Test: Bitwise Operations
# =========================================================================


class TestBitwiseOps:
    """Test bitwise operations."""

    def test_bit_and(self):
        result = run("x = 6 & 3\n")
        assert result.variables["x"] == 2

    def test_bit_or(self):
        result = run("x = 6 | 3\n")
        assert result.variables["x"] == 7

    def test_bit_xor(self):
        result = run("x = 6 ^ 3\n")
        assert result.variables["x"] == 5

    def test_bit_not(self):
        result = run("x = ~0\n")
        assert result.variables["x"] == -1

    def test_left_shift(self):
        result = run("x = 1 << 3\n")
        assert result.variables["x"] == 8

    def test_right_shift(self):
        result = run("x = 8 >> 2\n")
        assert result.variables["x"] == 2


# =========================================================================
# Test: Keyword Arguments (CALL_FUNCTION_KW)
# =========================================================================


class TestKeywordArguments:
    """Test that keyword arguments work for user-defined Starlark functions.

    This exercises the full pipeline: the compiler produces CALL_FUNCTION_KW
    bytecode with a keyword names tuple on the stack, and the VM handler
    unpacks the kwargs and maps them to the correct parameter positions.
    """

    def test_single_keyword_arg(self):
        """f(x=1) — one keyword argument, no positional."""
        result = run("def f(x):\n    return x\nresult = f(x=42)\n")
        assert result.variables["result"] == 42

    def test_multiple_keyword_args(self):
        """f(x=1, y=2) — multiple keyword arguments."""
        result = run(
            "def f(x, y):\n    return x + y\n"
            "result = f(x=10, y=20)\n"
        )
        assert result.variables["result"] == 30

    def test_keyword_args_out_of_order(self):
        """f(y=2, x=1) — keyword arguments can be in any order."""
        result = run(
            "def f(x, y):\n    return x * 10 + y\n"
            "result = f(y=3, x=7)\n"
        )
        assert result.variables["result"] == 73

    def test_mixed_positional_and_keyword(self):
        """f(1, y=2) — mix of positional and keyword arguments."""
        result = run(
            "def f(x, y):\n    return x * 10 + y\n"
            "result = f(1, y=2)\n"
        )
        assert result.variables["result"] == 12

    def test_keyword_args_with_three_params(self):
        """f(a=1, b=2, c=3) — three keyword arguments."""
        result = run(
            "def f(a, b, c):\n    return a * 100 + b * 10 + c\n"
            "result = f(a=4, b=5, c=6)\n"
        )
        assert result.variables["result"] == 456

    def test_keyword_args_build_file_style(self):
        """Simulate BUILD-file-style calls: rule(name='foo', deps=['bar']).

        This is the primary use case for CALL_FUNCTION_KW — it's how BUILD
        file rules like py_library(name='foo', deps=['bar']) work.
        """
        result = run(
            "def py_library(name, deps):\n"
            "    return name + \":\" + str(len(deps))\n"
            "result = py_library(name=\"mylib\", deps=[\"dep1\", \"dep2\"])\n"
        )
        assert result.variables["result"] == "mylib:2"

    def test_keyword_args_with_list_value(self):
        """Keyword argument whose value is a list."""
        result = run(
            "def f(items):\n    return len(items)\n"
            "result = f(items=[1, 2, 3])\n"
        )
        assert result.variables["result"] == 3

    def test_keyword_args_with_string_value(self):
        """Keyword argument whose value is a string."""
        result = run(
            "def greet(name):\n    return \"hello \" + name\n"
            "result = greet(name=\"world\")\n"
        )
        assert result.variables["result"] == "hello world"


# =========================================================================
# Test: Starlark Opcodes Enum
# =========================================================================


class TestOpcodeEnum:
    """Verify opcode values match the spec."""

    def test_stack_opcodes(self):
        assert Op.LOAD_CONST == 0x01
        assert Op.POP == 0x02
        assert Op.DUP == 0x03

    def test_variable_opcodes(self):
        assert Op.STORE_NAME == 0x10
        assert Op.LOAD_NAME == 0x11

    def test_arithmetic_opcodes(self):
        assert Op.ADD == 0x20
        assert Op.SUB == 0x21
        assert Op.MUL == 0x22

    def test_comparison_opcodes(self):
        assert Op.CMP_EQ == 0x30
        assert Op.CMP_IN == 0x36

    def test_control_flow_opcodes(self):
        assert Op.JUMP == 0x40
        assert Op.JUMP_IF_FALSE == 0x41

    def test_halt(self):
        assert Op.HALT == 0xFF
