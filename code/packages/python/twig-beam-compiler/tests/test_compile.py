"""Compile-time tests — no ``erl`` required.

Verifies the Twig → IR lowering produces sensible IR and rejects
out-of-scope forms with clear error messages.
"""

from __future__ import annotations

import pytest
from compiler_ir import IrOp
from twig.errors import TwigCompileError

from twig_beam_compiler import compile_source, compile_to_ir


class TestSupportedSurface:
    def test_int_literal_compiles(self) -> None:
        ir = compile_to_ir("42")
        ops = [ins.opcode for ins in ir.instructions]
        # Expect: LABEL, LOAD_IMM (the literal), maybe an ADD for
        # the move, RET.
        assert IrOp.LABEL in ops
        assert IrOp.LOAD_IMM in ops
        assert IrOp.RET in ops

    def test_addition_emits_add(self) -> None:
        ir = compile_to_ir("(+ 1 2)")
        ops = [ins.opcode for ins in ir.instructions]
        assert IrOp.ADD in ops

    def test_subtraction_emits_sub(self) -> None:
        ir = compile_to_ir("(- 10 3)")
        ops = [ins.opcode for ins in ir.instructions]
        assert IrOp.SUB in ops

    def test_multiplication_emits_mul(self) -> None:
        ir = compile_to_ir("(* 6 7)")
        ops = [ins.opcode for ins in ir.instructions]
        assert IrOp.MUL in ops

    def test_let_compiles(self) -> None:
        ir = compile_to_ir("(let ((x 5)) (* x x))")
        ops = [ins.opcode for ins in ir.instructions]
        assert IrOp.MUL in ops


class TestRejectedSurface:
    def test_define_rejected(self) -> None:
        with pytest.raises(TwigCompileError, match="not yet supported"):
            compile_to_ir("(define x 1)")

    def test_lambda_rejected(self) -> None:
        with pytest.raises(TwigCompileError, match="not yet supported"):
            compile_to_ir("(lambda (x) x)")

    def test_comparison_rejected(self) -> None:
        with pytest.raises(TwigCompileError, match="not yet supported"):
            compile_to_ir("(= 1 1)")

    def test_unbound_name_rejected(self) -> None:
        with pytest.raises(TwigCompileError, match="unbound name"):
            compile_to_ir("foo")


class TestCompileSourceShape:
    def test_returns_beam_bytes(self) -> None:
        result = compile_source("(+ 1 2)")
        assert result.beam_bytes[:4] == b"FOR1"
        assert b"BEAM" in result.beam_bytes[:16]
        assert result.module_name == "twig_main"

    def test_custom_module_name(self) -> None:
        result = compile_source("(+ 1 2)", module_name="adder")
        assert result.module_name == "adder"

    def test_compile_without_optimizer(self) -> None:
        result = compile_source("(+ 1 2)", optimize=False)
        assert result.beam_bytes[:4] == b"FOR1"

    def test_empty_program_returns_zero(self) -> None:
        result = compile_source("")
        # Should still produce a valid .beam (main/0 returns 0).
        assert result.beam_bytes[:4] == b"FOR1"

    def test_begin_compiles(self) -> None:
        result = compile_source("(begin 1 2 3)")
        assert result.beam_bytes[:4] == b"FOR1"


class TestRejectedSurfaceMore:
    def test_unknown_builtin_rejected(self) -> None:
        with pytest.raises(TwigCompileError, match="unknown builtin"):
            compile_to_ir("(weirdop 1 2)")

    def test_arity_mismatch_rejected(self) -> None:
        with pytest.raises(TwigCompileError, match="expects exactly 2"):
            compile_to_ir("(+ 1 2 3)")

    def test_multi_binding_let_rejected(self) -> None:
        with pytest.raises(TwigCompileError, match="multiple bindings"):
            compile_to_ir("(let ((x 1) (y 2)) (+ x y))")

    def test_quoted_symbol_rejected(self) -> None:
        with pytest.raises(TwigCompileError, match="quoted symbols"):
            compile_to_ir("'foo")

    def test_call_of_non_var_rejected(self) -> None:
        # An apply where the function position is itself a call —
        # not a VarRef, so v1 rejects.
        with pytest.raises(TwigCompileError, match="named builtins"):
            compile_to_ir("((+ 1 1) 2 3)")


class TestModuleNameSecurity:
    """Defends against Erlang code-injection via ``module_name``.

    ``run_source`` interpolates ``module_name`` into an ``erl
    -eval`` Erlang source string.  Without an allowlist a caller
    could pass e.g. ``module_name="ok, os:cmd(...), m"`` and get
    arbitrary command execution.  ``compile_source`` validates
    upfront and rejects anything that isn't a strict atom-like
    identifier.
    """

    def test_legal_name_accepted(self) -> None:
        result = compile_source("(+ 1 2)", module_name="answer42")
        assert result.module_name == "answer42"

    @pytest.mark.parametrize(
        "bad",
        [
            "ok, os:cmd(\"id\"), m",   # full Erlang injection
            "../etc_passwd",            # path-traversal style
            "Module",                   # uppercase start (Erlang variable)
            "1main",                    # digit start
            "foo bar",                  # space
            "",                          # empty
            "a" * 65,                   # too long
        ],
    )
    def test_unsafe_names_rejected(self, bad: str) -> None:
        # BeamPackageError is the wrapper around module-name violations.
        from twig_beam_compiler import BeamPackageError

        with pytest.raises(BeamPackageError, match="module_name must match"):
            compile_source("(+ 1 2)", module_name=bad)
