"""Compile-time tests for twig-beam-compiler (TW03 Phase 1)."""

from __future__ import annotations

import pytest
from compiler_ir import IrLabel, IrOp
from twig.errors import TwigCompileError

from twig_beam_compiler import (
    BeamPackageError,
    compile_source,
    compile_to_ir,
)


class TestSupportedSurface:
    def test_int_literal_compiles(self) -> None:
        ir = compile_to_ir("42")
        ops = [ins.opcode for ins in ir.instructions]
        assert IrOp.LABEL in ops
        assert IrOp.LOAD_IMM in ops
        assert IrOp.RET in ops

    def test_addition_emits_add(self) -> None:
        ir = compile_to_ir("(+ 1 2)")
        ops = [ins.opcode for ins in ir.instructions]
        assert IrOp.ADD in ops

    @pytest.mark.parametrize(
        ("src", "op"),
        [
            ("(+ 1 2)", IrOp.ADD),
            ("(- 10 3)", IrOp.SUB),
            ("(* 6 7)", IrOp.MUL),
            ("(/ 10 2)", IrOp.DIV),
        ],
    )
    def test_arithmetic_ops(self, src: str, op: IrOp) -> None:
        ir = compile_to_ir(src)
        assert op in [ins.opcode for ins in ir.instructions]

    @pytest.mark.parametrize(
        ("src", "op"),
        [
            ("(= 1 1)", IrOp.CMP_EQ),
            ("(< 1 2)", IrOp.CMP_LT),
            ("(> 2 1)", IrOp.CMP_GT),
        ],
    )
    def test_comparison_ops(self, src: str, op: IrOp) -> None:
        ir = compile_to_ir(src)
        assert op in [ins.opcode for ins in ir.instructions]

    def test_let_compiles(self) -> None:
        ir = compile_to_ir("(let ((x 5)) (* x x))")
        assert IrOp.MUL in [ins.opcode for ins in ir.instructions]

    def test_if_emits_branch(self) -> None:
        ir = compile_to_ir("(if (= 1 1) 100 200)")
        ops = [ins.opcode for ins in ir.instructions]
        assert IrOp.BRANCH_Z in ops
        assert IrOp.JUMP in ops
        assert IrOp.CMP_EQ in ops

    def test_define_function_emits_call(self) -> None:
        ir = compile_to_ir("(define (square x) (* x x)) (square 7)")
        ops = [ins.opcode for ins in ir.instructions]
        assert IrOp.CALL in ops
        # Two callable LABELs: square and main.
        region_labels = [
            ins.operands[0].name
            for ins in ir.instructions
            if ins.opcode is IrOp.LABEL
            and isinstance(ins.operands[0], IrLabel)
            and not ins.operands[0].name.startswith("_")
        ]
        assert "square" in region_labels
        assert "main" in region_labels

    def test_recursive_function_compiles(self) -> None:
        ir = compile_to_ir(
            "(define (fact n) (if (= n 0) 1 (* n (fact (- n 1)))))"
            "(fact 5)"
        )
        ops = [ins.opcode for ins in ir.instructions]
        assert ops.count(IrOp.CALL) >= 2
        assert IrOp.BRANCH_Z in ops
        assert IrOp.MUL in ops
        assert IrOp.SUB in ops

    def test_value_define_inlined(self) -> None:
        ir = compile_to_ir("(define x 42) x")
        region_labels = [
            ins.operands[0].name
            for ins in ir.instructions
            if ins.opcode is IrOp.LABEL
            and isinstance(ins.operands[0], IrLabel)
            and not ins.operands[0].name.startswith("_")
        ]
        # Value defines fold to compile-time constants — only main.
        assert region_labels == ["main"]

    def test_lambda_lifts_to_top_level_region(self) -> None:
        """An anonymous lambda becomes a fresh ``_lambda_N`` region
        and the use site emits MAKE_CLOSURE referencing it."""
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
        assert "main" in labels
        assert "_lambda_0" in labels

        # MAKE_CLOSURE for _lambda_0 with 1 capture should appear
        # inside make-adder's body.
        mk = [i for i in ir.instructions if i.opcode is IrOp.MAKE_CLOSURE]
        assert len(mk) == 1
        assert mk[0].operands[1].name == "_lambda_0"
        # operand 2 is the IrImmediate num_captured
        assert mk[0].operands[2].value == 1

    def test_closure_call_emits_apply_closure(self) -> None:
        """A call whose function position is itself an Apply (so
        the result is a closure value, not a known top-level
        function) lowers to APPLY_CLOSURE."""
        ir = compile_to_ir(
            "(define (make-adder n) (lambda (x) (+ x n)))"
            "((make-adder 7) 35)"
        )
        ap = [i for i in ir.instructions if i.opcode is IrOp.APPLY_CLOSURE]
        assert len(ap) == 1
        # APPLY_CLOSURE dst, closure_reg, IrImmediate(num_args), arg0
        assert ap[0].operands[2].value == 1


class TestRejectedSurface:
    def test_unbound_name_rejected(self) -> None:
        with pytest.raises(TwigCompileError, match="unbound name"):
            compile_to_ir("foo")

    def test_unknown_function_rejected(self) -> None:
        with pytest.raises(TwigCompileError, match="unknown function"):
            compile_to_ir("(weirdop 1 2)")

    def test_arity_mismatch_rejected(self) -> None:
        with pytest.raises(TwigCompileError, match="expects 2 arguments"):
            compile_to_ir("(+ 1 2 3)")

    def test_function_arity_mismatch_rejected(self) -> None:
        with pytest.raises(TwigCompileError, match="takes 1 arguments"):
            compile_to_ir("(define (id x) x) (id 1 2)")

    def test_cons_rejected(self) -> None:
        with pytest.raises(TwigCompileError, match="not yet supported"):
            compile_to_ir("(cons 1 2)")

    def test_print_rejected(self) -> None:
        with pytest.raises(TwigCompileError, match="not yet supported"):
            compile_to_ir("(print 42)")

    def test_quoted_symbol_rejected(self) -> None:
        with pytest.raises(TwigCompileError, match="quoted symbols"):
            compile_to_ir("'foo")

    def test_non_literal_value_define_rejected(self) -> None:
        with pytest.raises(TwigCompileError, match="literal RHS"):
            compile_to_ir("(define x (+ 1 2))")


class TestCompileSourceShape:
    def test_returns_beam_bytes(self) -> None:
        result = compile_source("(+ 1 2)")
        assert result.beam_bytes[:4] == b"FOR1"
        assert b"BEAM" in result.beam_bytes[:16]

    def test_custom_module_name(self) -> None:
        result = compile_source("(+ 1 2)", module_name="adder")
        assert result.module_name == "adder"

    def test_compile_without_optimizer(self) -> None:
        result = compile_source("(+ 1 2)", optimize=False)
        assert result.beam_bytes[:4] == b"FOR1"

    def test_empty_program_returns_zero(self) -> None:
        result = compile_source("")
        assert result.beam_bytes[:4] == b"FOR1"


class TestModuleNameSecurity:
    """Defends against Erlang code-injection via ``module_name``."""

    def test_legal_name_accepted(self) -> None:
        result = compile_source("(+ 1 2)", module_name="answer42")
        assert result.module_name == "answer42"

    @pytest.mark.parametrize(
        "bad",
        [
            "ok, os:cmd(\"id\"), m",
            "../etc_passwd",
            "Module",
            "1main",
            "foo bar",
            "",
            "a" * 65,
        ],
    )
    def test_unsafe_names_rejected(self, bad: str) -> None:
        with pytest.raises(BeamPackageError, match="module_name must match"):
            compile_source("(+ 1 2)", module_name=bad)
