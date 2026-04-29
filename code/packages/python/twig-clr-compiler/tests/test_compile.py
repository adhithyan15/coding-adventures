"""Compile-time tests — no ``dotnet`` required.

Verifies the Twig → IR lowering produces sensible IR and rejects
out-of-scope forms with clear error messages.  Real-``dotnet``
end-to-end tests live in ``test_real_dotnet.py``.
"""

from __future__ import annotations

import pytest
from compiler_ir import IrOp
from twig.errors import TwigCompileError

from twig_clr_compiler import (
    ClrPackageError,
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

    def test_subtraction_emits_sub(self) -> None:
        ir = compile_to_ir("(- 10 3)")
        ops = [ins.opcode for ins in ir.instructions]
        assert IrOp.SUB in ops

    def test_multiplication_emits_mul(self) -> None:
        ir = compile_to_ir("(* 6 7)")
        ops = [ins.opcode for ins in ir.instructions]
        assert IrOp.MUL in ops

    def test_division_emits_div(self) -> None:
        ir = compile_to_ir("(/ 10 2)")
        ops = [ins.opcode for ins in ir.instructions]
        assert IrOp.DIV in ops

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


class TestCompileSourceShape:
    def test_returns_pe_bytes(self) -> None:
        result = compile_source("(+ 1 2)")
        # PE/CLI files start with the MS-DOS ``MZ`` signature.
        assert result.assembly_bytes[:2] == b"MZ"
        assert result.assembly_name == "TwigProgram"

    def test_custom_assembly_name(self) -> None:
        result = compile_source("(+ 1 2)", assembly_name="Adder")
        assert result.assembly_name == "Adder"

    def test_compile_without_optimizer(self) -> None:
        result = compile_source("(+ 1 2)", optimize=False)
        assert result.assembly_bytes[:2] == b"MZ"

    def test_empty_program_returns_zero(self) -> None:
        result = compile_source("")
        assert result.assembly_bytes[:2] == b"MZ"


class TestAssemblyNameSecurity:
    """Defends against path-traversal and CLR-name violations via
    ``assembly_name``.  Same allowlist pattern as
    ``twig-beam-compiler``'s ``module_name`` validation."""

    def test_legal_name_accepted(self) -> None:
        result = compile_source("(+ 1 2)", assembly_name="Answer42")
        assert result.assembly_name == "Answer42"

    @pytest.mark.parametrize(
        "bad",
        [
            "../etc_passwd",      # path-traversal style
            "1Bad",                # digit start
            "Has Space",           # space
            "with;semicolon",      # punctuation
            "",                     # empty
            "a" * 65,              # too long
        ],
    )
    def test_unsafe_names_rejected(self, bad: str) -> None:
        with pytest.raises(ClrPackageError, match="assembly_name must match"):
            compile_source("(+ 1 2)", assembly_name=bad)
