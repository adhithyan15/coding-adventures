"""Tests for WASMCodeGenerator — CodeGenerator[IrProgram, WasmModule] (LANG20).

Covers: name, validate (valid + invalid IR), generate, protocol check, round-trip.
"""

from __future__ import annotations

import pytest

from codegen_core import CodeGenerator
from compiler_ir import IrImmediate, IrInstruction, IrLabel, IrOp, IrProgram, IrRegister
from ir_to_wasm_compiler import WASMCodeGenerator, WasmLoweringError
from wasm_types import WasmModule


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _minimal_ir() -> IrProgram:
    """Minimal valid IrProgram: LOAD_IMM r0, 1; HALT."""
    prog = IrProgram(entry_label="_start")
    prog.add_instruction(IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(1)], id=0))
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=1))
    return prog


def _bad_ir() -> IrProgram:
    """IrProgram that should fail WASM validation (unsupported syscall)."""
    prog = IrProgram(entry_label="_start")
    prog.add_instruction(IrInstruction(IrOp.SYSCALL, [IrImmediate(999)], id=0))
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=1))
    return prog


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestWASMCodeGenerator:

    def test_name(self) -> None:
        assert WASMCodeGenerator().name == "wasm"

    def test_satisfies_codegenerator_protocol(self) -> None:
        assert isinstance(WASMCodeGenerator(), CodeGenerator)

    def test_validate_valid_ir_returns_empty(self) -> None:
        gen = WASMCodeGenerator()
        assert gen.validate(_minimal_ir()) == []

    def test_validate_returns_list(self) -> None:
        gen = WASMCodeGenerator()
        result = gen.validate(_minimal_ir())
        assert isinstance(result, list)

    def test_validate_does_not_raise(self) -> None:
        gen = WASMCodeGenerator()
        result = gen.validate(_bad_ir())
        assert isinstance(result, list)

    def test_generate_valid_ir_returns_wasm_module(self) -> None:
        gen = WASMCodeGenerator()
        result = gen.generate(_minimal_ir())
        assert isinstance(result, WasmModule)

    def test_generate_module_is_not_none(self) -> None:
        gen = WASMCodeGenerator()
        result = gen.generate(_minimal_ir())
        assert result is not None

    def test_generate_bad_ir_raises(self) -> None:
        gen = WASMCodeGenerator()
        with pytest.raises(Exception):
            gen.generate(_bad_ir())

    def test_round_trip_validate_then_generate(self) -> None:
        gen = WASMCodeGenerator()
        ir = _minimal_ir()
        errors = gen.validate(ir)
        assert errors == []
        result = gen.generate(ir)
        assert isinstance(result, WasmModule)

    def test_multiple_instances_independent(self) -> None:
        """Each WASMCodeGenerator instance should work independently."""
        gen1 = WASMCodeGenerator()
        gen2 = WASMCodeGenerator()
        r1 = gen1.generate(_minimal_ir())
        r2 = gen2.generate(_minimal_ir())
        assert isinstance(r1, WasmModule)
        assert isinstance(r2, WasmModule)

    def test_exported_from_package(self) -> None:
        import ir_to_wasm_compiler
        assert hasattr(ir_to_wasm_compiler, "WASMCodeGenerator")
