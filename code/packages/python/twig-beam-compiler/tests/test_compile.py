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


class TestRejectedSurface:
    def test_lambda_rejected(self) -> None:
        with pytest.raises(TwigCompileError, match="not yet supported"):
            compile_to_ir("(lambda (x) x)")

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
