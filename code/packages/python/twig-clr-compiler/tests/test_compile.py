"""Compile-only tests for ``twig-clr-compiler``.

These tests stop at the IR stage — they verify the AST → IrProgram
walker produces the expected shapes without exercising the
downstream optimiser / CIL lowering / assembly pipeline.  Pipeline
tests live in :mod:`test_integration`.
"""

from __future__ import annotations

import pytest
from compiler_ir import IrOp
from twig.errors import TwigCompileError

from twig_clr_compiler import compile_to_ir

# ---------------------------------------------------------------------------
# Atoms
# ---------------------------------------------------------------------------


def test_integer_literal_emits_load_imm() -> None:
    prog = compile_to_ir("42")
    ops = [i.opcode for i in prog.instructions]
    # LABEL _start, LOAD_IMM, ADD_IMM (move to reg 1), HALT
    assert IrOp.LOAD_IMM in ops
    assert ops[-1] == IrOp.HALT


def test_boolean_literal_lowered_as_int() -> None:
    prog = compile_to_ir("#t")
    # #t becomes LOAD_IMM 1
    has_load_imm_1 = any(
        i.opcode == IrOp.LOAD_IMM
        and len(i.operands) >= 2
        and getattr(i.operands[1], "value", None) == 1
        for i in prog.instructions
    )
    assert has_load_imm_1


def test_negative_integer() -> None:
    prog = compile_to_ir("-7")
    assert IrOp.LOAD_IMM in [i.opcode for i in prog.instructions]


# ---------------------------------------------------------------------------
# Arithmetic
# ---------------------------------------------------------------------------


def test_addition_emits_add() -> None:
    prog = compile_to_ir("(+ 1 2)")
    ops = [i.opcode for i in prog.instructions]
    assert IrOp.ADD in ops


def test_subtraction_emits_sub() -> None:
    prog = compile_to_ir("(- 10 3)")
    assert IrOp.SUB in [i.opcode for i in prog.instructions]


def test_multiplication_emits_mul() -> None:
    prog = compile_to_ir("(* 6 7)")
    assert IrOp.MUL in [i.opcode for i in prog.instructions]


def test_division_emits_div() -> None:
    prog = compile_to_ir("(/ 20 4)")
    assert IrOp.DIV in [i.opcode for i in prog.instructions]


def test_comparison_eq() -> None:
    prog = compile_to_ir("(= 1 1)")
    assert IrOp.CMP_EQ in [i.opcode for i in prog.instructions]


def test_comparison_lt() -> None:
    prog = compile_to_ir("(< 1 2)")
    assert IrOp.CMP_LT in [i.opcode for i in prog.instructions]


def test_comparison_gt() -> None:
    prog = compile_to_ir("(> 2 1)")
    assert IrOp.CMP_GT in [i.opcode for i in prog.instructions]


# ---------------------------------------------------------------------------
# Control flow
# ---------------------------------------------------------------------------


def test_if_emits_branch_z_and_jump() -> None:
    prog = compile_to_ir("(if (= 1 1) 100 200)")
    ops = [i.opcode for i in prog.instructions]
    assert IrOp.BRANCH_Z in ops
    assert IrOp.JUMP in ops
    assert IrOp.LABEL in ops


def test_let_binds_locals() -> None:
    """A ``let`` body that references the binding compiles cleanly."""
    prog = compile_to_ir("(let ((x 5)) (* x x))")
    assert IrOp.MUL in [i.opcode for i in prog.instructions]


def test_nested_let() -> None:
    prog = compile_to_ir("(let ((a 1)) (let ((b 2)) (+ a b)))")
    assert IrOp.ADD in [i.opcode for i in prog.instructions]


def test_begin_returns_last() -> None:
    """``(begin 1 2 3)`` — last expression's value is the result."""
    prog = compile_to_ir("(begin 1 2 3)")
    # Should compile without error; final HALT loads from the last
    # expression's register.
    assert IrOp.HALT in [i.opcode for i in prog.instructions]


# ---------------------------------------------------------------------------
# Rejections (out-of-scope for v1)
# ---------------------------------------------------------------------------


def test_lambda_is_rejected() -> None:
    with pytest.raises(TwigCompileError, match="lambda"):
        compile_to_ir("(lambda (x) x)")


def test_define_is_rejected() -> None:
    with pytest.raises(TwigCompileError, match="define"):
        compile_to_ir("(define x 42)")


def test_cons_is_rejected() -> None:
    with pytest.raises(TwigCompileError, match="cons"):
        compile_to_ir("(cons 1 2)")


def test_print_is_rejected() -> None:
    with pytest.raises(TwigCompileError, match="print"):
        compile_to_ir("(print 1)")


def test_nil_is_rejected() -> None:
    with pytest.raises(TwigCompileError, match="nil"):
        compile_to_ir("nil")


def test_quoted_symbol_is_rejected() -> None:
    with pytest.raises(TwigCompileError, match="symbols"):
        compile_to_ir("'foo")


def test_unbound_name_is_rejected() -> None:
    with pytest.raises(TwigCompileError, match="unbound"):
        compile_to_ir("undefined_name")


def test_unknown_function_is_rejected() -> None:
    """Names other than the v1 binary builtins are not callable."""
    with pytest.raises(TwigCompileError):
        compile_to_ir("(let ((f 1)) (f 2))")


def test_wrong_arg_count_is_rejected() -> None:
    with pytest.raises(TwigCompileError, match="2 arguments"):
        compile_to_ir("(+ 1 2 3)")


def test_empty_program_compiles_to_zero() -> None:
    """An empty source file should produce a valid program returning 0."""
    prog = compile_to_ir("")
    ops = [i.opcode for i in prog.instructions]
    assert IrOp.HALT in ops
    assert IrOp.LOAD_IMM in ops
