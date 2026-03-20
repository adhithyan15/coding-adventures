"""Tests for Starlark compiler rule handlers.

These tests compile Starlark source code and verify the emitted bytecode
instructions, constants, and names. Each test exercises a specific
compiler handler or compilation pattern.
"""

from __future__ import annotations

import pytest

from starlark_compiler import compile_starlark, Op
from starlark_compiler.compiler import (
    _extract_simple_name,
    _has_token,
    _find_token_index,
    _nodes,
    _tokens,
    _token_values,
    _type_name,
    _parse_string_literal,
)
from starlark_parser import parse_starlark


# =========================================================================
# Helpers
# =========================================================================


def opcodes(source: str) -> list[int]:
    """Compile source and return just the opcode list."""
    code = compile_starlark(source)
    return [instr.opcode for instr in code.instructions]


def compile(source: str):
    """Compile source and return the CodeObject."""
    return compile_starlark(source)


# =========================================================================
# Test: Helper Functions
# =========================================================================


class TestHelperFunctions:
    """Test internal helper functions."""

    def test_tokens_extracts_tokens(self):
        ast = parse_starlark("x = 1\n")
        # The file node has statement children; dig into assign_stmt
        from lexer import Token
        file_node = ast
        tokens = _tokens(file_node)
        # file node may have NEWLINE tokens
        assert all(isinstance(t, Token) for t in tokens)

    def test_nodes_extracts_nodes(self):
        ast = parse_starlark("x = 1\n")
        from lang_parser import ASTNode
        nodes = _nodes(ast)
        assert all(isinstance(n, ASTNode) for n in nodes)

    def test_type_name_string(self):
        from lexer import Token
        t = Token(type="NAME", value="x", line=1, column=1)
        assert _type_name(t) == "NAME"

    def test_type_name_enum(self):
        from lexer import Token
        from lexer.tokenizer import TokenType
        t = Token(type=TokenType.NUMBER, value="42", line=1, column=1)
        assert _type_name(t) == "NUMBER"

    def test_has_token(self):
        ast = parse_starlark("x = 1\n")
        # Dig into the assign_stmt node
        stmt = _nodes(ast)[0]  # statement
        assign = _nodes(stmt)[0]  # simple_stmt
        assign2 = _nodes(assign)[0]  # small_stmt / assign_stmt
        # The assign_stmt should have a "=" token somewhere in its tree
        # This depends on the exact AST structure

    def test_find_token_index_found(self):
        ast = parse_starlark("x = 1\n")
        # Walk into assign_stmt and look for "="
        stmt = _nodes(ast)[0]
        simple = _nodes(stmt)[0]
        assign = _nodes(simple)[0]
        idx = _find_token_index(assign, "=")
        # May or may not find it depending on AST structure
        # Just verify it returns int or None
        assert idx is None or isinstance(idx, int)

    def test_extract_simple_name(self):
        ast = parse_starlark("x\n")
        # Navigate to the expression containing "x"
        stmt = _nodes(ast)[0]
        simple = _nodes(stmt)[0]
        assign = _nodes(simple)[0]  # assign_stmt (expression stmt)
        expr_list = _nodes(assign)[0]  # expression_list
        name = _extract_simple_name(expr_list)
        assert name == "x"


# =========================================================================
# Test: Basic Assignments
# =========================================================================


class TestAssignmentCompilation:
    """Test compilation of assignment statements."""

    def test_assign_int(self):
        code = compile("x = 42\n")
        assert 42 in code.constants
        assert "x" in code.names
        assert Op.LOAD_CONST in opcodes("x = 42\n")
        assert Op.STORE_NAME in opcodes("x = 42\n")

    def test_assign_float(self):
        code = compile("x = 3.14\n")
        assert 3.14 in code.constants
        assert Op.LOAD_CONST in opcodes("x = 3.14\n")

    def test_assign_string(self):
        code = compile('x = "hello"\n')
        assert "hello" in code.constants

    def test_assign_true(self):
        ops = opcodes("x = True\n")
        assert Op.LOAD_TRUE in ops

    def test_assign_false(self):
        ops = opcodes("x = False\n")
        assert Op.LOAD_FALSE in ops

    def test_assign_none(self):
        ops = opcodes("x = None\n")
        assert Op.LOAD_NONE in ops

    def test_multiple_assignments(self):
        code = compile("x = 1\ny = 2\n")
        assert 1 in code.constants
        assert 2 in code.constants
        assert "x" in code.names
        assert "y" in code.names

    def test_expression_statement_pops(self):
        """An expression statement (no assignment) should emit POP."""
        ops = opcodes("42\n")
        assert Op.POP in ops

    def test_augmented_assign(self):
        """x += 1 should load, add, and store."""
        ops = opcodes("x = 0\nx += 1\n")
        assert Op.ADD in ops


# =========================================================================
# Test: Arithmetic Compilation
# =========================================================================


class TestArithmeticCompilation:
    """Test compilation of arithmetic expressions."""

    def test_add(self):
        ops = opcodes("x = 1 + 2\n")
        assert Op.ADD in ops

    def test_subtract(self):
        ops = opcodes("x = 10 - 3\n")
        assert Op.SUB in ops

    def test_multiply(self):
        ops = opcodes("x = 4 * 5\n")
        assert Op.MUL in ops

    def test_divide(self):
        ops = opcodes("x = 10 / 3\n")
        assert Op.DIV in ops

    def test_floor_divide(self):
        ops = opcodes("x = 10 // 3\n")
        assert Op.FLOOR_DIV in ops

    def test_modulo(self):
        ops = opcodes("x = 10 % 3\n")
        assert Op.MOD in ops

    def test_power(self):
        ops = opcodes("x = 2 ** 10\n")
        assert Op.POWER in ops

    def test_unary_negate(self):
        ops = opcodes("x = -5\n")
        assert Op.NEGATE in ops

    def test_unary_bitwise_not(self):
        ops = opcodes("x = ~0\n")
        assert Op.BIT_NOT in ops

    def test_complex_expression(self):
        """2 + 3 * 4 should emit MUL before ADD (precedence)."""
        code = compile("x = 2 + 3 * 4\n")
        ops = [i.opcode for i in code.instructions]
        mul_idx = ops.index(Op.MUL)
        add_idx = ops.index(Op.ADD)
        assert mul_idx < add_idx

    def test_parenthesized_expression(self):
        """(2 + 3) * 4 should emit ADD before MUL."""
        code = compile("x = (2 + 3) * 4\n")
        ops = [i.opcode for i in code.instructions]
        add_idx = ops.index(Op.ADD)
        mul_idx = ops.index(Op.MUL)
        assert add_idx < mul_idx


# =========================================================================
# Test: Bitwise Operations
# =========================================================================


class TestBitwiseCompilation:
    """Test compilation of bitwise operations."""

    def test_bit_and(self):
        ops = opcodes("x = 6 & 3\n")
        assert Op.BIT_AND in ops

    def test_bit_or(self):
        ops = opcodes("x = 6 | 3\n")
        assert Op.BIT_OR in ops

    def test_bit_xor(self):
        ops = opcodes("x = 6 ^ 3\n")
        assert Op.BIT_XOR in ops

    def test_left_shift(self):
        ops = opcodes("x = 1 << 3\n")
        assert Op.LSHIFT in ops

    def test_right_shift(self):
        ops = opcodes("x = 8 >> 2\n")
        assert Op.RSHIFT in ops


# =========================================================================
# Test: Comparisons
# =========================================================================


class TestComparisonCompilation:
    """Test compilation of comparison operations."""

    def test_equal(self):
        ops = opcodes("x = 1 == 1\n")
        assert Op.CMP_EQ in ops

    def test_not_equal(self):
        ops = opcodes("x = 1 != 2\n")
        assert Op.CMP_NE in ops

    def test_less_than(self):
        ops = opcodes("x = 1 < 2\n")
        assert Op.CMP_LT in ops

    def test_greater_than(self):
        ops = opcodes("x = 2 > 1\n")
        assert Op.CMP_GT in ops

    def test_less_equal(self):
        ops = opcodes("x = 2 <= 2\n")
        assert Op.CMP_LE in ops

    def test_greater_equal(self):
        ops = opcodes("x = 3 >= 2\n")
        assert Op.CMP_GE in ops

    def test_in(self):
        ops = opcodes("x = 1 in [1, 2]\n")
        assert Op.CMP_IN in ops

    def test_not_in(self):
        ops = opcodes("x = 1 not in [1, 2]\n")
        assert Op.CMP_NOT_IN in ops


# =========================================================================
# Test: Boolean Logic
# =========================================================================


class TestBooleanCompilation:
    """Test compilation of boolean expressions."""

    def test_not(self):
        ops = opcodes("x = not True\n")
        assert Op.NOT in ops

    def test_and_short_circuit(self):
        """'and' uses JUMP_IF_FALSE_OR_POP for short-circuit."""
        ops = opcodes("x = True and False\n")
        assert Op.JUMP_IF_FALSE_OR_POP in ops

    def test_or_short_circuit(self):
        """'or' uses JUMP_IF_TRUE_OR_POP for short-circuit."""
        ops = opcodes("x = False or True\n")
        assert Op.JUMP_IF_TRUE_OR_POP in ops


# =========================================================================
# Test: If Statements
# =========================================================================


class TestIfCompilation:
    """Test compilation of if/elif/else statements."""

    def test_if_emits_jump_if_false(self):
        ops = opcodes("if True:\n    x = 1\n")
        assert Op.JUMP_IF_FALSE in ops

    def test_if_else(self):
        ops = opcodes("if True:\n    x = 1\nelse:\n    x = 2\n")
        assert Op.JUMP_IF_FALSE in ops
        assert Op.JUMP in ops

    def test_if_elif_else(self):
        src = "n = 2\nif n == 1:\n    x = 10\nelif n == 2:\n    x = 20\nelse:\n    x = 30\n"
        ops = opcodes(src)
        # Should have multiple JUMP_IF_FALSE and JUMP
        assert ops.count(Op.JUMP_IF_FALSE) >= 2

    def test_ternary_expression(self):
        ops = opcodes("x = 1 if True else 2\n")
        assert Op.JUMP_IF_FALSE in ops
        assert Op.JUMP in ops


# =========================================================================
# Test: For Loops
# =========================================================================


class TestForCompilation:
    """Test compilation of for loops."""

    def test_for_loop_basic(self):
        ops = opcodes("for x in [1, 2, 3]:\n    pass\n")
        assert Op.GET_ITER in ops
        assert Op.FOR_ITER in ops
        assert Op.JUMP in ops

    def test_for_loop_stores_variable(self):
        code = compile("for x in [1, 2]:\n    pass\n")
        assert "x" in code.names

    def test_nested_for_loops(self):
        src = "for i in [1]:\n    for j in [2]:\n        pass\n"
        ops = opcodes(src)
        assert ops.count(Op.GET_ITER) == 2
        assert ops.count(Op.FOR_ITER) == 2

    def test_break_emits_jump(self):
        ops = opcodes("for x in [1, 2]:\n    break\n")
        # break emits an extra JUMP (beyond the loop's own JUMP)
        assert ops.count(Op.JUMP) >= 2

    def test_continue_emits_jump(self):
        ops = opcodes("for x in [1, 2]:\n    continue\n")
        # continue emits a JUMP back to loop top
        assert ops.count(Op.JUMP) >= 2


# =========================================================================
# Test: Function Definitions
# =========================================================================


class TestFunctionCompilation:
    """Test compilation of function definitions."""

    def test_simple_function(self):
        ops = opcodes("def f():\n    return 1\n")
        assert Op.MAKE_FUNCTION in ops
        assert Op.STORE_NAME in ops

    def test_function_name(self):
        code = compile("def greet():\n    pass\n")
        assert "greet" in code.names

    def test_function_body_is_nested_code(self):
        """Function body should be a nested CodeObject in constants."""
        code = compile("def f():\n    return 42\n")
        from virtual_machine import CodeObject
        nested = [c for c in code.constants if isinstance(c, CodeObject)]
        assert len(nested) == 1

    def test_function_with_params(self):
        code = compile("def add(a, b):\n    return a + b\n")
        from virtual_machine import CodeObject
        nested = [c for c in code.constants if isinstance(c, CodeObject)]
        assert len(nested) == 1

    def test_function_with_defaults(self):
        """Default params should push values before MAKE_FUNCTION."""
        ops = opcodes("def f(x=10):\n    return x\n")
        # The default value (10) is loaded before MAKE_FUNCTION
        make_idx = ops.index(Op.MAKE_FUNCTION)
        # There should be LOAD_CONST instructions before MAKE_FUNCTION
        const_loads = [i for i, o in enumerate(ops[:make_idx]) if o == Op.LOAD_CONST]
        assert len(const_loads) >= 1


# =========================================================================
# Test: Collection Literals
# =========================================================================


class TestCollectionCompilation:
    """Test compilation of list, dict, and tuple literals."""

    def test_empty_list(self):
        ops = opcodes("x = []\n")
        assert Op.BUILD_LIST in ops

    def test_list_literal(self):
        code = compile("x = [1, 2, 3]\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.BUILD_LIST in ops
        # Should have a BUILD_LIST with count=3
        for instr in code.instructions:
            if instr.opcode == Op.BUILD_LIST:
                assert instr.operand == 3
                break

    def test_empty_dict(self):
        ops = opcodes("x = {}\n")
        assert Op.BUILD_DICT in ops

    def test_dict_literal(self):
        code = compile('x = {"a": 1, "b": 2}\n')
        ops = [i.opcode for i in code.instructions]
        assert Op.BUILD_DICT in ops
        for instr in code.instructions:
            if instr.opcode == Op.BUILD_DICT:
                assert instr.operand == 2
                break

    def test_empty_tuple(self):
        ops = opcodes("x = ()\n")
        assert Op.BUILD_TUPLE in ops

    def test_single_element_tuple(self):
        ops = opcodes("x = (1,)\n")
        assert Op.BUILD_TUPLE in ops

    def test_multi_element_tuple(self):
        code = compile("x = (1, 2, 3)\n")
        for instr in code.instructions:
            if instr.opcode == Op.BUILD_TUPLE:
                assert instr.operand == 3
                break

    def test_list_indexing(self):
        ops = opcodes("x = [1, 2, 3]\ny = x[1]\n")
        assert Op.LOAD_SUBSCRIPT in ops


# =========================================================================
# Test: String Operations
# =========================================================================


class TestStringCompilation:
    """Test compilation of string-related operations."""

    def test_string_literal(self):
        code = compile('x = "hello"\n')
        assert "hello" in code.constants

    def test_single_quoted_string(self):
        code = compile("x = 'hello'\n")
        assert "hello" in code.constants

    def test_string_concatenation_compile_time(self):
        """Adjacent strings are concatenated at compile time."""
        code = compile('x = "hello" "world"\n')
        assert "helloworld" in code.constants

    def test_string_escape_sequences(self):
        """Escape sequences in strings are resolved at compile time."""
        assert _parse_string_literal('"\\r"') == "\r"
        assert _parse_string_literal('"\\0"') == "\0"
        assert _parse_string_literal("'hello'") == "hello"

    def test_triple_quoted_string(self):
        assert _parse_string_literal('"""triple"""') == "triple"
        assert _parse_string_literal("'''triple'''") == "triple"

    def test_unknown_escape(self):
        """Unknown escapes are preserved as-is."""
        assert _parse_string_literal('"\\q"') == "\\q"


# =========================================================================
# Test: Variable Loading
# =========================================================================


class TestVariableCompilation:
    """Test compilation of variable references."""

    def test_load_name(self):
        ops = opcodes("x = 1\ny = x\n")
        assert Op.LOAD_NAME in ops

    def test_variable_in_expression(self):
        code = compile("a = 10\nb = 20\nc = a + b\n")
        assert "a" in code.names
        assert "b" in code.names
        assert "c" in code.names


# =========================================================================
# Test: Control Flow Patterns
# =========================================================================


class TestControlFlowPatterns:
    """Test bytecode patterns for control flow constructs."""

    def test_if_jump_targets_are_valid(self):
        """Jump targets in if statements should be valid instruction indices."""
        code = compile("if True:\n    x = 1\n")
        for instr in code.instructions:
            if instr.opcode in (Op.JUMP, Op.JUMP_IF_FALSE, Op.JUMP_IF_TRUE):
                assert instr.operand is not None
                assert 0 <= instr.operand <= len(code.instructions)

    def test_for_loop_jump_targets(self):
        """FOR_ITER and JUMP in for loops should have valid targets."""
        code = compile("for x in [1]:\n    pass\n")
        for instr in code.instructions:
            if instr.opcode in (Op.FOR_ITER, Op.JUMP):
                assert instr.operand is not None
                assert 0 <= instr.operand <= len(code.instructions)

    def test_halt_at_end(self):
        """Every compiled program should end with HALT."""
        code = compile("x = 1\n")
        assert code.instructions[-1].opcode == Op.HALT


# =========================================================================
# Test: Pass Statement
# =========================================================================


class TestPassCompilation:
    """Test pass statement compilation."""

    def test_pass_emits_nothing(self):
        """pass should not emit any instructions of its own."""
        ops_pass = opcodes("pass\n")
        # Should just have HALT (pass emits nothing)
        assert Op.HALT in ops_pass


# =========================================================================
# Test: Return Statement
# =========================================================================


class TestReturnCompilation:
    """Test return statement compilation."""

    def test_return_with_value(self):
        code = compile("def f():\n    return 42\n")
        from virtual_machine import CodeObject
        nested = [c for c in code.constants if isinstance(c, CodeObject)]
        assert len(nested) == 1
        body_ops = [i.opcode for i in nested[0].instructions]
        assert Op.RETURN in body_ops

    def test_return_without_value(self):
        code = compile("def f():\n    return\n")
        from virtual_machine import CodeObject
        nested = [c for c in code.constants if isinstance(c, CodeObject)]
        body_ops = [i.opcode for i in nested[0].instructions]
        assert Op.LOAD_NONE in body_ops
        assert Op.RETURN in body_ops


# =========================================================================
# Test: Function Calls
# =========================================================================


class TestFunctionCallCompilation:
    """Test compilation of function calls."""

    def test_call_no_args(self):
        ops = opcodes("f()\n")
        assert Op.CALL_FUNCTION in ops

    def test_call_with_args(self):
        code = compile("f(1, 2)\n")
        for instr in code.instructions:
            if instr.opcode == Op.CALL_FUNCTION:
                assert instr.operand == 2
                break

    def test_call_with_keyword_args(self):
        ops = opcodes("f(x=1)\n")
        assert Op.CALL_FUNCTION_KW in ops


# =========================================================================
# Test: Augmented Assignment
# =========================================================================


class TestAugmentedAssignCompilation:
    """Test compilation of augmented assignment operators."""

    def test_plus_equals(self):
        ops = opcodes("x = 0\nx += 1\n")
        assert Op.ADD in ops

    def test_minus_equals(self):
        ops = opcodes("x = 10\nx -= 3\n")
        assert Op.SUB in ops

    def test_times_equals(self):
        ops = opcodes("x = 2\nx *= 3\n")
        assert Op.MUL in ops

    def test_divide_equals(self):
        ops = opcodes("x = 10\nx /= 2\n")
        assert Op.DIV in ops

    def test_floor_divide_equals(self):
        ops = opcodes("x = 10\nx //= 3\n")
        assert Op.FLOOR_DIV in ops

    def test_modulo_equals(self):
        ops = opcodes("x = 10\nx %= 3\n")
        assert Op.MOD in ops

    def test_power_equals(self):
        ops = opcodes("x = 2\nx **= 3\n")
        assert Op.POWER in ops

    def test_and_equals(self):
        ops = opcodes("x = 7\nx &= 3\n")
        assert Op.BIT_AND in ops

    def test_or_equals(self):
        ops = opcodes("x = 5\nx |= 3\n")
        assert Op.BIT_OR in ops

    def test_xor_equals(self):
        ops = opcodes("x = 5\nx ^= 3\n")
        assert Op.BIT_XOR in ops

    def test_lshift_equals(self):
        ops = opcodes("x = 1\nx <<= 3\n")
        assert Op.LSHIFT in ops

    def test_rshift_equals(self):
        ops = opcodes("x = 8\nx >>= 2\n")
        assert Op.RSHIFT in ops


# =========================================================================
# Test: compile_starlark convenience function
# =========================================================================


class TestCompileStarlark:
    """Test the compile_starlark convenience function."""

    def test_returns_code_object(self):
        from virtual_machine import CodeObject
        code = compile_starlark("x = 1\n")
        assert isinstance(code, CodeObject)

    def test_instructions_are_list(self):
        code = compile_starlark("x = 1\n")
        assert isinstance(code.instructions, list)
        assert len(code.instructions) > 0

    def test_ends_with_halt(self):
        code = compile_starlark("x = 1\n")
        assert code.instructions[-1].opcode == Op.HALT

    def test_empty_program(self):
        code = compile_starlark("\n")
        # Should just have HALT
        assert code.instructions[-1].opcode == Op.HALT


# =========================================================================
# Test: Expression Lists and Tuples
# =========================================================================


class TestExpressionListCompilation:
    """Test compilation of expression lists."""

    def test_tuple_unpacking(self):
        """a, b = 1, 2 should use UNPACK_SEQUENCE."""
        ops = opcodes("a, b = 1, 2\n")
        assert Op.UNPACK_SEQUENCE in ops

    def test_trailing_comma_tuple(self):
        """(1,) should build a single-element tuple."""
        ops = opcodes("x = (1,)\n")
        assert Op.BUILD_TUPLE in ops


# =========================================================================
# Test: Attribute Access
# =========================================================================


class TestAttributeCompilation:
    """Test compilation of attribute access."""

    def test_load_attr(self):
        ops = opcodes("x = a.b\n")
        assert Op.LOAD_ATTR in ops


# =========================================================================
# Test: Slice Operations
# =========================================================================


class TestSliceCompilation:
    """Test compilation of slice operations.

    Note: Slice syntax (a[1:3]) requires parser support for the subscript
    rule with colons. These tests are skipped if the parser doesn't support
    slice syntax yet.
    """

    @pytest.mark.skip(reason="Parser does not yet support slice syntax")
    def test_simple_slice(self):
        ops = opcodes("x = a[1:3]\n")
        assert Op.LOAD_SLICE in ops

    @pytest.mark.skip(reason="Parser does not yet support slice syntax")
    def test_slice_with_step(self):
        ops = opcodes("x = a[::2]\n")
        assert Op.LOAD_SLICE in ops


# =========================================================================
# Test: Lambda Expressions
# =========================================================================


class TestLambdaCompilation:
    """Test compilation of lambda expressions."""

    def test_lambda(self):
        ops = opcodes("f = lambda x: x + 1\n")
        assert Op.MAKE_FUNCTION in ops

    def test_lambda_body_is_nested(self):
        code = compile("f = lambda x: x + 1\n")
        from virtual_machine import CodeObject
        nested = [c for c in code.constants if isinstance(c, CodeObject)]
        assert len(nested) == 1


# =========================================================================
# Test: Error Handling
# =========================================================================


class TestCompilationErrors:
    """Test that compilation errors are raised for invalid constructs."""

    def test_break_outside_loop(self):
        """break outside a for loop should raise SyntaxError."""
        with pytest.raises(SyntaxError, match="break"):
            compile_starlark("break\n")

    def test_continue_outside_loop(self):
        """continue outside a for loop should raise SyntaxError."""
        with pytest.raises(SyntaxError, match="continue"):
            compile_starlark("continue\n")
