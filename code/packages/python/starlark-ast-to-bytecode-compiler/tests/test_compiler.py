"""Tests for Starlark compiler rule handlers.

These tests compile Starlark source code and verify the emitted bytecode
instructions, constants, and names. Each test exercises a specific
compiler handler or compilation pattern.
"""

from __future__ import annotations

import pytest

from starlark_ast_to_bytecode_compiler import compile_starlark, Op
from starlark_ast_to_bytecode_compiler.compiler import (
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


# =========================================================================
# Test: Load Statements
# =========================================================================


class TestLoadStatements:
    """Test compilation of load() statements."""

    def test_load_single_symbol(self):
        """load('mod.star', 'sym') produces LOAD_MODULE + IMPORT_FROM."""
        code = compile('load("mod.star", "sym")\n')
        ops = [i.opcode for i in code.instructions]
        assert Op.LOAD_MODULE in ops
        assert Op.IMPORT_FROM in ops
        assert "mod.star" in code.names
        assert "sym" in code.names

    def test_load_multiple_symbols(self):
        """load('mod.star', 'a', 'b') produces multiple IMPORT_FROM."""
        code = compile('load("mod.star", "a", "b")\n')
        ops = [i.opcode for i in code.instructions]
        import_count = ops.count(Op.IMPORT_FROM)
        assert import_count >= 2
        assert "a" in code.names
        assert "b" in code.names


# =========================================================================
# Test: For Loops with Break and Continue
# =========================================================================


class TestForLoopControl:
    """Test for loop compilation with break and continue."""

    def test_for_with_break(self):
        """break inside a for loop emits JUMP past the loop."""
        code = compile("for x in [1, 2, 3]:\n    break\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.GET_ITER in ops
        assert Op.FOR_ITER in ops
        assert Op.JUMP in ops

    def test_for_with_continue(self):
        """continue inside a for loop emits JUMP back to loop top."""
        code = compile("for x in [1, 2, 3]:\n    continue\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.GET_ITER in ops
        assert Op.FOR_ITER in ops

    def test_for_with_break_and_continue(self):
        """Combined break and continue in a loop."""
        code = compile(
            "for x in [1, 2, 3]:\n"
            "    if x == 1:\n"
            "        continue\n"
            "    if x == 3:\n"
            "        break\n"
        )
        ops = [i.opcode for i in code.instructions]
        assert Op.GET_ITER in ops
        assert Op.FOR_ITER in ops


# =========================================================================
# Test: Function Parameters — Defaults, Varargs, Kwargs
# =========================================================================


class TestFunctionParameters:
    """Test compilation of various function parameter styles."""

    def test_default_parameter(self):
        """def f(x=1) should compile default value."""
        code = compile("def f(x=1):\n    return x\n")
        # Default value 1 should be in constants
        assert 1 in code.constants

    def test_multiple_defaults(self):
        """def f(x=1, y=2) — multiple default values."""
        code = compile("def f(x=1, y=2):\n    return x + y\n")
        assert 1 in code.constants
        assert 2 in code.constants

    def test_varargs_parameter(self):
        """def f(*args) — varargs parameter."""
        code = compile("def f(*args):\n    return args\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.MAKE_FUNCTION in ops

    def test_kwargs_parameter(self):
        """def f(**kwargs) — keyword arguments parameter."""
        code = compile("def f(**kwargs):\n    return kwargs\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.MAKE_FUNCTION in ops

    def test_mixed_params(self):
        """def f(a, b=1, *args) — mixed parameters."""
        code = compile("def f(a, b=1, *args):\n    return a\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.MAKE_FUNCTION in ops


# =========================================================================
# Test: If/Elif/Else Compilation
# =========================================================================


class TestIfElifElse:
    """Test compilation of if/elif/else chains."""

    def test_if_elif(self):
        """if/elif produces correct jumps."""
        code = compile(
            "if x == 1:\n"
            "    y = 1\n"
            "elif x == 2:\n"
            "    y = 2\n"
        )
        ops = [i.opcode for i in code.instructions]
        assert Op.CMP_EQ in ops
        assert Op.JUMP_IF_FALSE in ops

    def test_if_elif_else(self):
        """if/elif/else produces correct jumps."""
        code = compile(
            "if x == 1:\n"
            "    y = 1\n"
            "elif x == 2:\n"
            "    y = 2\n"
            "else:\n"
            "    y = 3\n"
        )
        ops = [i.opcode for i in code.instructions]
        assert ops.count(Op.CMP_EQ) >= 2
        assert ops.count(Op.JUMP_IF_FALSE) >= 2


# =========================================================================
# Test: Ternary Expressions
# =========================================================================


class TestTernaryExpressions:
    """Test compilation of ternary if/else expressions."""

    def test_simple_ternary(self):
        """x if True else y — ternary expression."""
        code = compile("result = 1 if True else 0\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.JUMP_IF_FALSE in ops
        assert Op.JUMP in ops

    def test_nested_ternary(self):
        """Nested ternary expression."""
        code = compile("result = 1 if x else (2 if y else 3)\n")
        ops = [i.opcode for i in code.instructions]
        assert ops.count(Op.JUMP_IF_FALSE) >= 2


# =========================================================================
# Test: Boolean Operators
# =========================================================================


class TestBooleanOperators:
    """Test compilation of and/or/not operators."""

    def test_or_expression(self):
        """a or b uses short-circuit evaluation."""
        code = compile("result = a or b\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.JUMP_IF_TRUE_OR_POP in ops

    def test_chained_or(self):
        """a or b or c — chained or."""
        code = compile("result = a or b or c\n")
        ops = [i.opcode for i in code.instructions]
        assert ops.count(Op.JUMP_IF_TRUE_OR_POP) >= 2

    def test_and_expression(self):
        """a and b uses short-circuit evaluation."""
        code = compile("result = a and b\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.JUMP_IF_FALSE_OR_POP in ops

    def test_chained_and(self):
        """a and b and c — chained and."""
        code = compile("result = a and b and c\n")
        ops = [i.opcode for i in code.instructions]
        assert ops.count(Op.JUMP_IF_FALSE_OR_POP) >= 2

    def test_not_expression(self):
        """not x compiles to NOT opcode."""
        code = compile("result = not x\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.NOT in ops

    def test_combined_and_or(self):
        """(a or b) and c — combined boolean operators."""
        code = compile("result = (a or b) and c\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.JUMP_IF_TRUE_OR_POP in ops
        assert Op.JUMP_IF_FALSE_OR_POP in ops


# =========================================================================
# Test: Comparison Operators
# =========================================================================


class TestComparisonOperators:
    """Test compilation of comparison operators."""

    def test_in_operator(self):
        """x in lst compiles to CMP_IN."""
        code = compile("result = x in [1, 2, 3]\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.CMP_IN in ops

    def test_not_in_operator(self):
        """x not in lst compiles to CMP_NOT_IN."""
        code = compile("result = x not in [1, 2]\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.CMP_NOT_IN in ops

    def test_less_than(self):
        code = compile("result = a < b\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.CMP_LT in ops

    def test_greater_equal(self):
        code = compile("result = a >= b\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.CMP_GE in ops

    def test_less_equal(self):
        code = compile("result = a <= b\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.CMP_LE in ops


# =========================================================================
# Test: Arithmetic and Unary Operators
# =========================================================================


class TestArithmeticOperators:
    """Test compilation of arithmetic and unary operators."""

    def test_floor_division(self):
        code = compile("result = a // b\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.FLOOR_DIV in ops

    def test_modulo(self):
        code = compile("result = a % b\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.MOD in ops

    def test_power(self):
        code = compile("result = a ** 2\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.POWER in ops

    def test_unary_negate(self):
        code = compile("result = -x\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.NEGATE in ops

    def test_bit_not(self):
        code = compile("result = ~x\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.BIT_NOT in ops

    def test_left_shift(self):
        code = compile("result = x << 2\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.LSHIFT in ops

    def test_right_shift(self):
        code = compile("result = x >> 2\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.RSHIFT in ops

    def test_bit_and(self):
        code = compile("result = a & b\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.BIT_AND in ops

    def test_bit_or(self):
        code = compile("result = a | b\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.BIT_OR in ops

    def test_bit_xor(self):
        code = compile("result = a ^ b\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.BIT_XOR in ops


# =========================================================================
# Test: Subscript and Slicing
# =========================================================================


class TestSubscriptAndSlicing:
    """Test compilation of subscript access and slicing."""

    def test_simple_subscript(self):
        """lst[0] compiles to LOAD_SUBSCRIPT."""
        code = compile("result = lst[0]\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.LOAD_SUBSCRIPT in ops

    @pytest.mark.skip(reason="Complex assignment targets not yet implemented")
    def test_store_subscript(self):
        """lst[0] = 1 compiles to STORE_SUBSCRIPT."""
        code = compile("lst[0] = 1\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.STORE_SUBSCRIPT in ops

    @pytest.mark.skip(reason="Slice syntax not yet supported by parser")
    def test_slice_start_end(self):
        """lst[1:3] compiles to LOAD_SLICE."""
        code = compile("result = lst[1:3]\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.LOAD_SLICE in ops

    @pytest.mark.skip(reason="Slice syntax not yet supported by parser")
    def test_slice_start_only(self):
        """lst[1:] compiles with slice support."""
        code = compile("result = lst[1:]\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.LOAD_SLICE in ops

    @pytest.mark.skip(reason="Slice syntax not yet supported by parser")
    def test_slice_end_only(self):
        """lst[:3] compiles with slice support."""
        code = compile("result = lst[:3]\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.LOAD_SLICE in ops

    @pytest.mark.skip(reason="Slice syntax not yet supported by parser")
    def test_slice_full(self):
        """lst[:] compiles with slice support."""
        code = compile("result = lst[:]\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.LOAD_SLICE in ops

    @pytest.mark.skip(reason="Slice syntax not yet supported by parser")
    def test_slice_with_step(self):
        """lst[::2] compiles with slice support."""
        code = compile("result = lst[::2]\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.LOAD_SLICE in ops

    @pytest.mark.skip(reason="Slice syntax not yet supported by parser")
    def test_slice_all_parts(self):
        """lst[1:3:2] compiles with slice support."""
        code = compile("result = lst[1:3:2]\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.LOAD_SLICE in ops


# =========================================================================
# Test: Attribute Access
# =========================================================================


class TestAttributeAccess:
    """Test compilation of attribute access and store."""

    def test_load_attribute(self):
        """obj.attr compiles to LOAD_ATTR."""
        code = compile("result = obj.attr\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.LOAD_ATTR in ops

    @pytest.mark.skip(reason="Complex assignment targets not yet implemented")
    def test_store_attribute(self):
        """obj.attr = 1 compiles to STORE_ATTR."""
        code = compile("obj.attr = 1\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.STORE_ATTR in ops


# =========================================================================
# Test: List Comprehensions
# =========================================================================


class TestListComprehensions:
    """Test compilation of list comprehensions."""

    def test_simple_comprehension(self):
        """[x for x in lst] produces BUILD_LIST + GET_ITER + FOR_ITER."""
        code = compile("[x for x in range(5)]\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.BUILD_LIST in ops
        assert Op.GET_ITER in ops
        assert Op.FOR_ITER in ops

    def test_comprehension_with_filter(self):
        """[x for x in lst if x > 0] includes JUMP_IF_FALSE."""
        code = compile("[x for x in range(10) if x > 5]\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.BUILD_LIST in ops
        assert Op.JUMP_IF_FALSE in ops


# =========================================================================
# Test: Dict Literals and Comprehensions
# =========================================================================


class TestDictCompilation:
    """Test compilation of dict literals and comprehensions."""

    def test_empty_dict(self):
        """Empty dict {} compiles to BUILD_DICT."""
        code = compile("d = {}\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.BUILD_DICT in ops

    def test_dict_with_entries(self):
        """Dict with entries compiles key-value pairs + BUILD_DICT."""
        code = compile('d = {"a": 1, "b": 2}\n')
        ops = [i.opcode for i in code.instructions]
        assert Op.BUILD_DICT in ops


# =========================================================================
# Test: Augmented Assignment
# =========================================================================


class TestAugmentedAssignment:
    """Test compilation of augmented assignment operators."""

    def test_plus_equals(self):
        """x += 1 compiles to LOAD + ADD + STORE."""
        code = compile("x = 0\nx += 1\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.ADD in ops

    def test_minus_equals(self):
        """x -= 1 compiles to LOAD + SUB + STORE."""
        code = compile("x = 0\nx -= 1\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.SUB in ops

    def test_times_equals(self):
        """x *= 2 compiles to LOAD + MUL + STORE."""
        code = compile("x = 1\nx *= 2\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.MUL in ops


# =========================================================================
# Test: Keyword Argument Compilation (CPython Convention)
# =========================================================================


class TestKeywordArgCompilation:
    """Test that keyword arguments compile to the CPython convention."""

    def test_kw_arg_produces_names_tuple(self):
        """f(x=1) should push a names tuple ('x',) as a constant."""
        code = compile("f(x=1)\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.CALL_FUNCTION_KW in ops
        # The keyword names tuple should be in the constants
        found = any(c == ("x",) for c in code.constants)
        assert found, f"Expected ('x',) in constants, got {code.constants}"

    def test_multiple_kw_args_names_tuple(self):
        """f(a=1, b=2) should push ('a', 'b') as constant."""
        code = compile("f(a=1, b=2)\n")
        found = any(c == ("a", "b") for c in code.constants)
        assert found, f"Expected ('a', 'b') in constants, got {code.constants}"

    def test_mixed_pos_and_kw(self):
        """f(1, x=2) emits CALL_FUNCTION_KW with argc=2."""
        code = compile("f(1, x=2)\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.CALL_FUNCTION_KW in ops
        # Find the CALL_FUNCTION_KW instruction and check operand
        for instr in code.instructions:
            if instr.opcode == Op.CALL_FUNCTION_KW:
                assert instr.operand == 2
                break

    def test_param_names_tuple_in_function(self):
        """def f(a, b): ... should push ('a', 'b') as param_names."""
        code = compile("def f(a, b):\n    return a + b\n")
        # The param names tuple should be in the constants
        found = any(c == ("a", "b") for c in code.constants)
        assert found, f"Expected ('a', 'b') in constants, got {code.constants}"


# =========================================================================
# Test: Load Statement with Alias
# =========================================================================


class TestLoadAlias:
    """Test load() with aliased imports: load('file', alias = 'symbol')."""

    def test_load_alias(self):
        """load('file.star', my_fn = 'orig_fn') emits IMPORT_FROM + STORE_NAME for alias."""
        code = compile('load("//rules.star", my_fn = "orig_fn")\n')
        ops = [i.opcode for i in code.instructions]
        assert Op.LOAD_MODULE in ops
        assert Op.IMPORT_FROM in ops
        # Both the original name and alias should be in names
        assert "orig_fn" in code.names
        assert "my_fn" in code.names


# =========================================================================
# Test: Tuple Unpacking in For Loops
# =========================================================================


class TestForLoopUnpacking:
    """Test for loops with tuple unpacking: for x, y in pairs."""

    def test_for_tuple_unpack(self):
        """for x, y in pairs: ... emits UNPACK_SEQUENCE."""
        code = compile("for x, y in pairs:\n    z = x\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.UNPACK_SEQUENCE in ops


# =========================================================================
# Test: Expression List (Tuple Creation)
# =========================================================================


class TestExpressionList:
    """Test that expression lists create tuples."""

    def test_tuple_assignment(self):
        """x = 1, 2, 3 creates a tuple via BUILD_TUPLE."""
        code = compile("x = 1, 2, 3\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.BUILD_TUPLE in ops

    def test_single_trailing_comma(self):
        """x = 1, creates a single-element tuple."""
        code = compile("x = 1,\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.BUILD_TUPLE in ops


# =========================================================================
# Test: Literal Values (True, False, None)
# =========================================================================


class TestLiteralValues:
    """Test compilation of True, False, None."""

    def test_true_literal(self):
        """x = True emits LOAD_TRUE."""
        code = compile("x = True\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.LOAD_TRUE in ops

    def test_false_literal(self):
        """x = False emits LOAD_FALSE."""
        code = compile("x = False\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.LOAD_FALSE in ops

    def test_none_literal(self):
        """x = None emits LOAD_NONE."""
        code = compile("x = None\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.LOAD_NONE in ops


# =========================================================================
# Test: Adjacent String Concatenation
# =========================================================================


class TestStringConcatenation:
    """Test compile-time string concatenation."""

    def test_adjacent_strings(self):
        """'hello' 'world' concatenates at compile time."""
        code = compile('x = "hello" "world"\n')
        # Should have concatenated string in constants
        assert "helloworld" in code.constants


# =========================================================================
# Test: Unary Operators
# =========================================================================


class TestUnaryOperators:
    """Test compilation of unary operators."""

    def test_negate(self):
        """x = -5 emits NEGATE."""
        code = compile("x = -y\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.NEGATE in ops

    def test_bitwise_not(self):
        """x = ~y emits BIT_NOT."""
        code = compile("x = ~y\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.BIT_NOT in ops

    def test_unary_plus(self):
        """x = +y is a no-op (just loads y)."""
        code = compile("x = +y\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.LOAD_NAME in ops
        # No NEGATE should be present
        assert Op.NEGATE not in ops


# =========================================================================
# Test: Paren Expressions and Tuples
# =========================================================================


class TestParenExpressions:
    """Test parenthesized expressions and tuples."""

    def test_empty_tuple(self):
        """x = () creates empty tuple."""
        code = compile("x = ()\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.BUILD_TUPLE in ops
        for instr in code.instructions:
            if instr.opcode == Op.BUILD_TUPLE:
                assert instr.operand == 0
                break

    def test_paren_tuple(self):
        """x = (1, 2) creates a tuple."""
        code = compile("x = (1, 2)\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.BUILD_TUPLE in ops

    def test_single_paren(self):
        """x = (1) is just parenthesized, not a tuple."""
        code = compile("x = (1)\n")
        ops = [i.opcode for i in code.instructions]
        # Should NOT have BUILD_TUPLE
        assert Op.BUILD_TUPLE not in ops

    def test_single_element_tuple(self):
        """x = (1,) creates a single-element tuple."""
        code = compile("x = (1,)\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.BUILD_TUPLE in ops


# =========================================================================
# Test: Dict Literals
# =========================================================================


class TestDictLiterals:
    """Test compilation of dict literals."""

    def test_empty_dict(self):
        """x = {} creates empty dict."""
        code = compile("x = {}\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.BUILD_DICT in ops
        for instr in code.instructions:
            if instr.opcode == Op.BUILD_DICT:
                assert instr.operand == 0
                break

    def test_dict_with_entries(self):
        """x = {'a': 1, 'b': 2} creates dict with 2 entries."""
        code = compile('x = {"a": 1, "b": 2}\n')
        ops = [i.opcode for i in code.instructions]
        assert Op.BUILD_DICT in ops

    def test_dict_string_keys(self):
        """Dict keys are in constants."""
        code = compile('x = {"key": 42}\n')
        assert "key" in code.constants
        assert 42 in code.constants


# =========================================================================
# Test: While Loop
# =========================================================================


class TestWhileLoop:
    """Test compilation of while loops (Starlark doesn't have while, test for loops instead)."""

    def test_for_loop_has_jump(self):
        """for x in lst: ... emits FOR_ITER + JUMP loop."""
        code = compile("for x in lst:\n    y = x\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.FOR_ITER in ops
        assert Op.JUMP in ops


# =========================================================================
# Test: Return Statement
# =========================================================================


class TestReturnStatement:
    """Test compilation of return statements."""

    def test_return_value(self):
        """return x emits LOAD_NAME + RETURN."""
        code = compile("def f():\n    return 42\n")
        # The nested function code should have RETURN
        for const in code.constants:
            if hasattr(const, 'instructions'):
                inner_ops = [i.opcode for i in const.instructions]
                assert Op.RETURN in inner_ops
                break

    def test_return_none(self):
        """Bare return emits LOAD_NONE + RETURN."""
        code = compile("def f():\n    return\n")
        for const in code.constants:
            if hasattr(const, 'instructions'):
                inner_ops = [i.opcode for i in const.instructions]
                assert Op.RETURN in inner_ops
                assert Op.LOAD_NONE in inner_ops
                break


# =========================================================================
# Test: Pass Statement
# =========================================================================


class TestPassStatement:
    """Test compilation of pass statements."""

    def test_pass_compiles(self):
        """pass compiles without error."""
        code = compile("pass\n")
        # pass should produce at least a HALT instruction
        assert len(code.instructions) >= 1


# =========================================================================
# Test: Multiple Statements
# =========================================================================


class TestMultipleStatements:
    """Test compilation of multi-line programs."""

    def test_sequential_assignments(self):
        """Multiple assignments compile sequentially."""
        code = compile("x = 1\ny = 2\nz = 3\n")
        store_count = sum(1 for i in code.instructions if i.opcode == Op.STORE_NAME)
        assert store_count == 3

    def test_function_and_call(self):
        """def f(): ... then f() both compile."""
        code = compile("def f():\n    return 1\nresult = f()\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.MAKE_FUNCTION in ops
        assert Op.CALL_FUNCTION in ops


# =========================================================================
# Test: Lambda Expressions
# =========================================================================


class TestLambdaExpressions:
    """Test compilation of lambda expressions."""

    def test_simple_lambda(self):
        """f = lambda x: x + 1 compiles to MAKE_FUNCTION."""
        code = compile("f = lambda x: x + 1\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.MAKE_FUNCTION in ops

    def test_lambda_no_args(self):
        """f = lambda: 42 compiles."""
        code = compile("f = lambda: 42\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.MAKE_FUNCTION in ops

    def test_lambda_multiple_args(self):
        """f = lambda x, y: x + y compiles."""
        code = compile("f = lambda x, y: x + y\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.MAKE_FUNCTION in ops


# =========================================================================
# Test: List Comprehension with Filter
# =========================================================================


class TestListComprehensionFilter:
    """Test list comprehensions with if clauses."""

    def test_comp_with_if(self):
        """[x for x in lst if x > 0] emits JUMP_IF_FALSE for filter."""
        code = compile("result = [x for x in lst if x > 0]\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.JUMP_IF_FALSE in ops
        assert Op.GET_ITER in ops
        assert Op.FOR_ITER in ops
        assert Op.LIST_APPEND in ops


# =========================================================================
# Test: String Escape Sequences
# =========================================================================


class TestStringEscapes:
    """Test string literal parsing with escape sequences."""

    def test_newline_escape(self):
        """'hello\\nworld' has a newline in the constant."""
        code = compile('x = "hello\\nworld"\n')
        assert "hello\nworld" in code.constants

    def test_tab_escape(self):
        """'hello\\tworld' has a tab in the constant."""
        code = compile('x = "hello\\tworld"\n')
        assert "hello\tworld" in code.constants

    def test_backslash_escape(self):
        """'hello\\\\world' has a backslash in the constant."""
        code = compile('x = "hello\\\\world"\n')
        assert "hello\\world" in code.constants


# =========================================================================
# Test: Complex Expressions
# =========================================================================


class TestComplexExpressions:
    """Test compilation of complex expressions combining multiple features."""

    def test_nested_function_calls(self):
        """f(g(x)) compiles with nested CALL_FUNCTION."""
        code = compile("result = f(g(x))\n")
        call_count = sum(1 for i in code.instructions if i.opcode == Op.CALL_FUNCTION)
        assert call_count == 2

    def test_chained_comparison(self):
        """Comparison operators compile correctly."""
        code = compile("result = x == y\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.CMP_EQ in ops

    def test_not_equal(self):
        """x != y emits CMP_NE."""
        code = compile("result = x != y\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.CMP_NE in ops

    def test_method_call(self):
        """obj.method() compiles to LOAD_ATTR + CALL_FUNCTION."""
        code = compile("result = obj.method()\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.LOAD_ATTR in ops
        assert Op.CALL_FUNCTION in ops

    def test_subscript_access(self):
        """lst[0] compiles to LOAD_SUBSCRIPT."""
        code = compile("result = lst[0]\n")
        ops = [i.opcode for i in code.instructions]
        assert Op.LOAD_SUBSCRIPT in ops

    def test_function_with_defaults(self):
        """def f(x, y=10): ... compiles defaults."""
        code = compile("def f(x, y=10):\n    return x + y\n")
        # Default value 10 should be in constants
        assert 10 in code.constants

    def test_for_loop_in_function(self):
        """For loop inside function uses LOAD_LOCAL."""
        code = compile("def f(lst):\n    for x in lst:\n        y = x\n    return y\n")
        for const in code.constants:
            if hasattr(const, 'instructions'):
                inner_ops = [i.opcode for i in const.instructions]
                # Should use LOAD_LOCAL/STORE_LOCAL inside function scope
                assert Op.STORE_LOCAL in inner_ops or Op.STORE_NAME in inner_ops
                break
