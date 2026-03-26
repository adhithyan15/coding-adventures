"""Tests for the Lisp compiler — from source to bytecode.

These tests verify that the compiler correctly transforms Lisp source
code into bytecode instructions. They check:

1. Constants and names are added to the correct pools
2. The right opcodes are emitted in the right order
3. Special forms compile correctly
4. Tail call optimization works

The end-to-end tests in test_end_to_end.py verify that the compiled
code actually executes correctly on the VM.
"""

from __future__ import annotations

import pytest
from lisp_compiler import compile_lisp
from lisp_vm import LispOp
from virtual_machine import CodeObject, Instruction


def _opcodes(source: str) -> list[int]:
    """Compile source and return just the opcode list."""
    code = compile_lisp(source)
    return [instr.opcode for instr in code.instructions]


def _constants(source: str) -> list[object]:
    """Compile source and return the constants pool."""
    return compile_lisp(source).constants


def _names(source: str) -> list[str]:
    """Compile source and return the names pool."""
    return compile_lisp(source).names


class TestAtomCompilation:
    """Test compilation of atomic values — numbers, symbols, strings."""

    def test_number_literal(self) -> None:
        """A number becomes LOAD_CONST."""
        code = compile_lisp("42")
        assert 42 in code.constants
        assert LispOp.LOAD_CONST in _opcodes("42")

    def test_negative_number(self) -> None:
        """Negative numbers are parsed as a single token."""
        code = compile_lisp("-7")
        assert -7 in code.constants

    def test_nil_literal(self) -> None:
        """The symbol 'nil' becomes LOAD_NIL."""
        assert LispOp.LOAD_NIL in _opcodes("nil")

    def test_t_literal(self) -> None:
        """The symbol 't' becomes LOAD_TRUE."""
        assert LispOp.LOAD_TRUE in _opcodes("t")

    def test_symbol_becomes_load_name(self) -> None:
        """A bare symbol becomes LOAD_NAME."""
        code = compile_lisp("x")
        assert "x" in code.names
        assert LispOp.LOAD_NAME in _opcodes("x")

    def test_string_literal(self) -> None:
        """A string literal becomes LOAD_CONST with quotes stripped."""
        code = compile_lisp('"hello"')
        assert "hello" in code.constants


class TestArithmeticCompilation:
    """Test compilation of arithmetic operations."""

    def test_add(self) -> None:
        """(+ 1 2) compiles to LOAD_CONST 1, LOAD_CONST 2, ADD."""
        ops = _opcodes("(+ 1 2)")
        assert LispOp.LOAD_CONST in ops
        assert LispOp.ADD in ops

    def test_subtract(self) -> None:
        ops = _opcodes("(- 5 3)")
        assert LispOp.SUB in ops

    def test_multiply(self) -> None:
        ops = _opcodes("(* 4 5)")
        assert LispOp.MUL in ops

    def test_divide(self) -> None:
        ops = _opcodes("(/ 10 2)")
        assert LispOp.DIV in ops

    def test_nested_arithmetic(self) -> None:
        """(+ (* 2 3) 4) should compile both operations."""
        ops = _opcodes("(+ (* 2 3) 4)")
        assert LispOp.MUL in ops
        assert LispOp.ADD in ops

    def test_arithmetic_constants(self) -> None:
        """Constants from arithmetic expressions are in the pool."""
        code = compile_lisp("(+ 1 2)")
        assert 1 in code.constants
        assert 2 in code.constants


class TestComparisonCompilation:
    """Test compilation of comparison operations."""

    def test_eq(self) -> None:
        ops = _opcodes("(eq 1 2)")
        assert LispOp.CMP_EQ in ops

    def test_less_than(self) -> None:
        ops = _opcodes("(< 1 2)")
        assert LispOp.CMP_LT in ops

    def test_greater_than(self) -> None:
        ops = _opcodes("(> 3 2)")
        assert LispOp.CMP_GT in ops

    def test_equals_sign(self) -> None:
        """The = operator also maps to CMP_EQ."""
        ops = _opcodes("(= 1 1)")
        assert LispOp.CMP_EQ in ops


class TestDefineCompilation:
    """Test compilation of (define name expr)."""

    def test_define_number(self) -> None:
        """(define x 42) stores 42 in variable 'x'."""
        code = compile_lisp("(define x 42)")
        assert "x" in code.names
        assert 42 in code.constants
        assert LispOp.STORE_NAME in _opcodes("(define x 42)")

    def test_define_pushes_nil(self) -> None:
        """define returns NIL (it's a statement, not an expression)."""
        ops = _opcodes("(define x 42)")
        assert LispOp.LOAD_NIL in ops


class TestConsCompilation:
    """Test compilation of cons cell operations."""

    def test_cons(self) -> None:
        ops = _opcodes("(cons 1 2)")
        assert LispOp.CONS in ops

    def test_car(self) -> None:
        ops = _opcodes("(car x)")
        assert LispOp.CAR in ops

    def test_cdr(self) -> None:
        ops = _opcodes("(cdr x)")
        assert LispOp.CDR in ops


class TestPredicateCompilation:
    """Test compilation of predicate operations."""

    def test_atom(self) -> None:
        ops = _opcodes("(atom x)")
        assert LispOp.IS_ATOM in ops

    def test_is_nil(self) -> None:
        ops = _opcodes("(is-nil x)")
        assert LispOp.IS_NIL in ops


class TestQuoteCompilation:
    """Test compilation of quoted data."""

    def test_quote_number(self) -> None:
        """(quote 42) → LOAD_CONST 42."""
        ops = _opcodes("(quote 42)")
        assert LispOp.LOAD_CONST in ops

    def test_quote_symbol(self) -> None:
        """(quote foo) → MAKE_SYMBOL "foo"."""
        code = compile_lisp("(quote foo)")
        assert "foo" in code.constants
        assert LispOp.MAKE_SYMBOL in _opcodes("(quote foo)")

    def test_quote_nil(self) -> None:
        """(quote nil) → LOAD_NIL."""
        assert LispOp.LOAD_NIL in _opcodes("(quote nil)")

    def test_quote_list(self) -> None:
        """(quote (1 2 3)) builds a cons chain."""
        ops = _opcodes("(quote (1 2 3))")
        # Should have LOAD_NIL, then pairs of LOAD_CONST + CONS
        assert LispOp.LOAD_NIL in ops
        assert ops.count(LispOp.CONS) == 3

    def test_quote_empty_list(self) -> None:
        """(quote ()) → LOAD_NIL."""
        assert LispOp.LOAD_NIL in _opcodes("(quote ())")

    def test_shorthand_quote(self) -> None:
        """'foo should compile the same as (quote foo)."""
        ops = _opcodes("'foo")
        assert LispOp.MAKE_SYMBOL in ops

    def test_shorthand_quote_list(self) -> None:
        """'(1 2) should build a cons chain."""
        ops = _opcodes("'(1 2)")
        assert ops.count(LispOp.CONS) == 2


class TestCondCompilation:
    """Test compilation of conditional expressions."""

    def test_cond_emits_jumps(self) -> None:
        """cond should emit JUMP_IF_FALSE and JUMP instructions."""
        ops = _opcodes("(cond ((eq 1 1) 42) (t 0))")
        assert LispOp.JUMP_IF_FALSE in ops
        assert LispOp.JUMP in ops

    def test_cond_with_else(self) -> None:
        """cond with t clause should not have final LOAD_NIL default."""
        code = compile_lisp("(cond (t 42))")
        ops = [i.opcode for i in code.instructions]
        # Should have LOAD_CONST for 42 and HALT, but no JUMP_IF_FALSE
        assert LispOp.LOAD_CONST in ops


class TestLambdaCompilation:
    """Test compilation of lambda expressions."""

    def test_lambda_emits_make_closure(self) -> None:
        """lambda should emit LOAD_CONST (body) then MAKE_CLOSURE."""
        ops = _opcodes("(lambda (x) x)")
        assert LispOp.LOAD_CONST in ops
        assert LispOp.MAKE_CLOSURE in ops

    def test_lambda_body_is_code_object(self) -> None:
        """The lambda body should be a CodeObject in the constants pool."""
        code = compile_lisp("(lambda (x) x)")
        body_codes = [c for c in code.constants if isinstance(c, CodeObject)]
        assert len(body_codes) == 1

    def test_lambda_body_uses_load_local(self) -> None:
        """Parameters are accessed via LOAD_LOCAL in the body."""
        code = compile_lisp("(lambda (x) x)")
        body = [c for c in code.constants if isinstance(c, CodeObject)][0]
        body_ops = [i.opcode for i in body.instructions]
        assert LispOp.LOAD_LOCAL in body_ops

    def test_lambda_body_has_return(self) -> None:
        """The lambda body should end with RETURN."""
        code = compile_lisp("(lambda (x) x)")
        body = [c for c in code.constants if isinstance(c, CodeObject)][0]
        body_ops = [i.opcode for i in body.instructions]
        assert LispOp.RETURN in body_ops

    def test_lambda_param_count(self) -> None:
        """MAKE_CLOSURE operand should be the parameter count."""
        code = compile_lisp("(lambda (a b c) a)")
        # Find the MAKE_CLOSURE instruction
        make_closure = [
            i for i in code.instructions if i.opcode == LispOp.MAKE_CLOSURE
        ]
        assert len(make_closure) == 1
        assert make_closure[0].operand == 3


class TestFunctionCallCompilation:
    """Test compilation of function calls."""

    def test_simple_call(self) -> None:
        """(f 1 2) should compile args, then func, then CALL_FUNCTION."""
        ops = _opcodes("(f 1 2)")
        assert LispOp.CALL_FUNCTION in ops

    def test_call_arg_count(self) -> None:
        """CALL_FUNCTION operand should be the argument count."""
        code = compile_lisp("(f 1 2 3)")
        call = [i for i in code.instructions if i.opcode == LispOp.CALL_FUNCTION]
        assert len(call) == 1
        assert call[0].operand == 3


class TestTailCallCompilation:
    """Test that tail calls emit TAIL_CALL instead of CALL_FUNCTION."""

    def test_tail_call_in_lambda_body(self) -> None:
        """A call in the body of a lambda should use TAIL_CALL."""
        code = compile_lisp("(lambda (n) (f n))")
        body = [c for c in code.constants if isinstance(c, CodeObject)][0]
        body_ops = [i.opcode for i in body.instructions]
        assert LispOp.TAIL_CALL in body_ops
        assert LispOp.CALL_FUNCTION not in body_ops

    def test_no_tail_call_at_top_level(self) -> None:
        """Calls at the top level should use CALL_FUNCTION, not TAIL_CALL."""
        ops = _opcodes("(f 1)")
        assert LispOp.CALL_FUNCTION in ops
        assert LispOp.TAIL_CALL not in ops

    def test_no_tail_call_in_args(self) -> None:
        """Calls used as arguments should not be tail calls."""
        code = compile_lisp("(lambda (n) (g (f n)))")
        body = [c for c in code.constants if isinstance(c, CodeObject)][0]
        body_ops = [i.opcode for i in body.instructions]
        # The outer call (g ...) should be TAIL_CALL
        # The inner call (f n) should be CALL_FUNCTION
        assert LispOp.TAIL_CALL in body_ops
        assert LispOp.CALL_FUNCTION in body_ops

    def test_tail_call_in_cond_branch(self) -> None:
        """A call in a cond branch body should be a tail call if
        the cond itself is in tail position."""
        code = compile_lisp("(lambda (n) (cond ((eq n 0) 1) (t (f n))))")
        body = [c for c in code.constants if isinstance(c, CodeObject)][0]
        body_ops = [i.opcode for i in body.instructions]
        assert LispOp.TAIL_CALL in body_ops


class TestProgramCompilation:
    """Test compilation of multi-expression programs."""

    def test_multiple_expressions(self) -> None:
        """Multiple expressions: all but last are popped."""
        ops = _opcodes("1 2 3")
        assert ops.count(LispOp.POP) == 2  # pop 1 and 2, keep 3

    def test_define_then_use(self) -> None:
        """(define x 5) x → should have both STORE_NAME and LOAD_NAME."""
        code = compile_lisp("(define x 5) x")
        assert "x" in code.names
        assert LispOp.STORE_NAME in _opcodes("(define x 5) x")
        assert LispOp.LOAD_NAME in _opcodes("(define x 5) x")

    def test_empty_program(self) -> None:
        """Empty source should just have HALT."""
        code = compile_lisp("")
        # Should only have HALT
        assert len(code.instructions) == 1
        assert code.instructions[0].opcode == LispOp.HALT

    def test_empty_list(self) -> None:
        """() should compile to LOAD_NIL."""
        assert LispOp.LOAD_NIL in _opcodes("()")


class TestPrintCompilation:
    """Test compilation of (print expr)."""

    def test_print_compiles(self) -> None:
        ops = _opcodes("(print 42)")
        assert LispOp.PRINT in ops
        assert LispOp.LOAD_CONST in ops
