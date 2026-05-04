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


def test_print_rejected() -> None:
    with pytest.raises(TwigCompileError, match="print"):
        compile_to_ir("(print 1)")


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


# ── Closures (JVM02 Phase 2d) ─────────────────────────────────────────────


def test_lambda_lifts_to_top_level_region() -> None:
    """An anonymous lambda becomes a fresh ``_lambda_N`` region
    and the use site emits MAKE_CLOSURE referencing it."""
    from compiler_ir import IrLabel, IrOp
    ir = compile_to_ir(
        "(define (make-adder n) (lambda (x) (+ x n))) (make-adder 7)"
    )
    labels = [
        ins.operands[0].name
        for ins in ir.instructions
        if ins.opcode is IrOp.LABEL
        and isinstance(ins.operands[0], IrLabel)
    ]
    assert "make-adder" in labels
    assert "_start" in labels
    assert "_lambda_0" in labels

    mk = [i for i in ir.instructions if i.opcode is IrOp.MAKE_CLOSURE]
    assert len(mk) == 1
    assert mk[0].operands[1].name == "_lambda_0"
    assert mk[0].operands[2].value == 1


def test_closure_call_emits_apply_closure() -> None:
    """A call whose function position is itself an Apply (so
    the result is a closure value, not a known top-level
    function) lowers to APPLY_CLOSURE."""
    from compiler_ir import IrOp
    ir = compile_to_ir(
        "(define (make-adder n) (lambda (x) (+ x n)))"
        "((make-adder 7) 35)"
    )
    ap = [i for i in ir.instructions if i.opcode is IrOp.APPLY_CLOSURE]
    assert len(ap) == 1
    assert ap[0].operands[2].value == 1


# ── Heap primitives (TW03 Phase 3e) ───────────────────────────────────────


def test_nil_emits_load_nil() -> None:
    """A bare ``nil`` literal compiles to ``LOAD_NIL``."""
    from compiler_ir import IrOp
    ir = compile_to_ir("nil")
    assert IrOp.LOAD_NIL in [i.opcode for i in ir.instructions]


def test_quoted_symbol_emits_make_symbol() -> None:
    """``'foo`` (or ``(quote foo)``) compiles to ``MAKE_SYMBOL``
    with the symbol name as an ``IrLabel``."""
    from compiler_ir import IrLabel, IrOp
    ir = compile_to_ir("'foo")
    mks = [i for i in ir.instructions if i.opcode is IrOp.MAKE_SYMBOL]
    assert len(mks) == 1
    assert isinstance(mks[0].operands[1], IrLabel)
    assert mks[0].operands[1].name == "foo"


def test_cons_emits_make_cons() -> None:
    from compiler_ir import IrOp
    ir = compile_to_ir("(cons 1 nil)")
    assert IrOp.MAKE_CONS in [i.opcode for i in ir.instructions]


def test_car_emits_car() -> None:
    from compiler_ir import IrOp
    ir = compile_to_ir("(car (cons 1 nil))")
    assert IrOp.CAR in [i.opcode for i in ir.instructions]


def test_cdr_emits_cdr() -> None:
    from compiler_ir import IrOp
    ir = compile_to_ir("(cdr (cons 1 nil))")
    assert IrOp.CDR in [i.opcode for i in ir.instructions]


def test_null_predicate_emits_is_null() -> None:
    from compiler_ir import IrOp
    ir = compile_to_ir("(null? nil)")
    assert IrOp.IS_NULL in [i.opcode for i in ir.instructions]


def test_pair_predicate_emits_is_pair() -> None:
    from compiler_ir import IrOp
    ir = compile_to_ir("(pair? (cons 1 nil))")
    assert IrOp.IS_PAIR in [i.opcode for i in ir.instructions]


def test_symbol_predicate_emits_is_symbol() -> None:
    from compiler_ir import IrOp
    ir = compile_to_ir("(symbol? 'foo)")
    assert IrOp.IS_SYMBOL in [i.opcode for i in ir.instructions]


def test_cons_arity_validation() -> None:
    """``cons`` takes exactly 2 args."""
    with pytest.raises(TwigCompileError, match="cons"):
        compile_to_ir("(cons 1)")


def test_car_arity_validation() -> None:
    with pytest.raises(TwigCompileError, match="car"):
        compile_to_ir("(car)")
