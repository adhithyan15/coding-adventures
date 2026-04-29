"""Compile-only tests for ``twig-jvm-compiler``.

These stop at the IR stage — no JVM invocation.  End-to-end tests
that actually run on real ``java`` live in :mod:`test_real_jvm`.
"""

from __future__ import annotations

import pytest
from compiler_ir import IrOp
from twig.errors import TwigCompileError

from twig_jvm_compiler import compile_to_ir


def test_integer_literal_emits_load_imm() -> None:
    prog = compile_to_ir("42")
    assert IrOp.LOAD_IMM in [i.opcode for i in prog.instructions]
    assert IrOp.HALT in [i.opcode for i in prog.instructions]


def test_addition_emits_add() -> None:
    prog = compile_to_ir("(+ 1 2)")
    assert IrOp.ADD in [i.opcode for i in prog.instructions]


def test_subtraction_emits_sub() -> None:
    assert IrOp.SUB in [i.opcode for i in compile_to_ir("(- 10 3)").instructions]


def test_multiplication_emits_mul() -> None:
    assert IrOp.MUL in [i.opcode for i in compile_to_ir("(* 3 4)").instructions]


def test_division_emits_div() -> None:
    assert IrOp.DIV in [i.opcode for i in compile_to_ir("(/ 20 4)").instructions]


def test_eq_lt_gt_emit_cmp() -> None:
    p = compile_to_ir("(= 1 1)")
    assert IrOp.CMP_EQ in [i.opcode for i in p.instructions]
    p = compile_to_ir("(< 1 2)")
    assert IrOp.CMP_LT in [i.opcode for i in p.instructions]
    p = compile_to_ir("(> 2 1)")
    assert IrOp.CMP_GT in [i.opcode for i in p.instructions]


def test_if_emits_branch_and_jump() -> None:
    prog = compile_to_ir("(if (= 1 1) 100 200)")
    ops = [i.opcode for i in prog.instructions]
    assert IrOp.BRANCH_Z in ops
    assert IrOp.JUMP in ops
    assert IrOp.LABEL in ops


def test_let_compiles() -> None:
    prog = compile_to_ir("(let ((x 5)) (* x x))")
    assert IrOp.MUL in [i.opcode for i in prog.instructions]


def test_begin_compiles() -> None:
    prog = compile_to_ir("(begin 1 2 3)")
    assert IrOp.HALT in [i.opcode for i in prog.instructions]


def test_top_level_value_define_inlined() -> None:
    """``(define x 42)`` folds to the literal at every reference."""
    prog = compile_to_ir("(define x 42) (+ x x)")
    # No globals machinery — both x references should appear as
    # LOAD_IMM 42 in the program.
    load_imms = [
        instr
        for instr in prog.instructions
        if instr.opcode == IrOp.LOAD_IMM
    ]
    forty_two_loads = [
        i for i in load_imms
        if any(getattr(o, "value", None) == 42 for o in i.operands)
    ]
    assert len(forty_two_loads) >= 2


def test_top_level_function_define_emits_callable_region() -> None:
    """``(define (f x) ...)`` emits a labelled IR region."""
    prog = compile_to_ir("(define (square x) (* x x)) (square 3)")
    labels = [
        instr.operands[0].name  # type: ignore[attr-defined]
        for instr in prog.instructions
        if instr.opcode == IrOp.LABEL
    ]
    assert "square" in labels
    assert "_start" in labels


def test_function_call_emits_call_op() -> None:
    prog = compile_to_ir("(define (f x) x) (f 7)")
    assert IrOp.CALL in [i.opcode for i in prog.instructions]


def test_recursion_compiles() -> None:
    """``fact`` references itself — compile shouldn't fail on
    forward reference because we pre-collect function names."""
    prog = compile_to_ir(
        "(define (fact n) (if (= n 0) 1 (* n (fact (- n 1))))) (fact 4)"
    )
    assert IrOp.CALL in [i.opcode for i in prog.instructions]
    # The recursive call inside fact's body references "fact"
    # before fact's region is fully emitted — this works because
    # we pre-scan all top-level defines first.


def test_mutual_recursion_compiles() -> None:
    src = """
    (define (even? n) (if (= n 0) 1 (odd? (- n 1))))
    (define (odd? n) (if (= n 0) 0 (even? (- n 1))))
    (even? 4)
    """
    prog = compile_to_ir(src)
    assert IrOp.CALL in [i.opcode for i in prog.instructions]


# ---------------------------------------------------------------------------
# Rejections
# ---------------------------------------------------------------------------


def test_lambda_rejected() -> None:
    with pytest.raises(TwigCompileError, match="lambda"):
        compile_to_ir("(lambda (x) x)")


def test_cons_rejected() -> None:
    with pytest.raises(TwigCompileError, match="cons"):
        compile_to_ir("(cons 1 2)")


def test_print_rejected() -> None:
    with pytest.raises(TwigCompileError, match="print"):
        compile_to_ir("(print 1)")


def test_nil_rejected() -> None:
    with pytest.raises(TwigCompileError, match="nil"):
        compile_to_ir("nil")


def test_quoted_symbol_rejected() -> None:
    with pytest.raises(TwigCompileError, match="symbols"):
        compile_to_ir("'foo")


def test_unbound_name_rejected() -> None:
    with pytest.raises(TwigCompileError, match="unbound"):
        compile_to_ir("undefined")


def test_non_literal_value_define_rejected() -> None:
    """``(define x (+ 1 2))`` is out of scope until TW02.5."""
    with pytest.raises(TwigCompileError, match="literal RHS"):
        compile_to_ir("(define x (+ 1 2))")


def test_wrong_arity_rejected() -> None:
    with pytest.raises(TwigCompileError, match="2 arguments"):
        compile_to_ir("(+ 1 2 3)")


def test_function_arity_mismatch_rejected() -> None:
    with pytest.raises(TwigCompileError, match="takes 1 arguments"):
        compile_to_ir("(define (f x) x) (f 1 2)")
