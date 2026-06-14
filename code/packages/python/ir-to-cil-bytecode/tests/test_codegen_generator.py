"""Tests for CILCodeGenerator — CodeGenerator[IrProgram, CILProgramArtifact] (LANG20).

Covers: name, validate (valid + invalid IR), generate, protocol check, round-trip.
"""

from __future__ import annotations

import pytest

from codegen_core import CodeGenerator
from compiler_ir import IrImmediate, IrInstruction, IrLabel, IrOp, IrProgram, IrRegister
from ir_to_cil_bytecode import CILCodeGenerator, CILProgramArtifact
from ir_to_cil_bytecode.backend import CILBackendError


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _minimal_ir() -> IrProgram:
    """Minimal valid IrProgram: LABEL _start; LOAD_IMM r0, 1; HALT.

    The CIL backend requires an explicit LABEL instruction for the entry point
    — the entry_label= parameter alone is not enough.  The backend scans the
    instruction list for LABEL nodes to determine method boundaries, then
    checks that the entry label appears in that set.
    """
    prog = IrProgram(entry_label="_start")
    prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    prog.add_instruction(IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(1)], id=0))
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=1))
    return prog


def _bad_syscall_ir() -> IrProgram:
    """IrProgram with a SYSCALL number not in {1, 2, 10}."""
    prog = IrProgram(entry_label="_start")
    prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    prog.add_instruction(IrInstruction(IrOp.SYSCALL, [IrImmediate(99)], id=0))
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=1))
    return prog


def _overflow_constant_ir() -> IrProgram:
    """IrProgram with a constant exceeding 32-bit signed int range."""
    prog = IrProgram(entry_label="_start")
    prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    prog.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(2**40)], id=0)
    )
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=1))
    return prog


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestCILCodeGenerator:

    def test_name(self) -> None:
        assert CILCodeGenerator().name == "cil"

    def test_satisfies_codegenerator_protocol(self) -> None:
        assert isinstance(CILCodeGenerator(), CodeGenerator)

    def test_validate_valid_ir_returns_empty(self) -> None:
        gen = CILCodeGenerator()
        assert gen.validate(_minimal_ir()) == []

    def test_validate_bad_syscall_returns_errors(self) -> None:
        gen = CILCodeGenerator()
        errors = gen.validate(_bad_syscall_ir())
        assert len(errors) >= 1
        assert any("syscall" in e.lower() or "99" in e for e in errors)

    def test_validate_overflow_constant_returns_errors(self) -> None:
        gen = CILCodeGenerator()
        errors = gen.validate(_overflow_constant_ir())
        assert len(errors) >= 1

    def test_validate_does_not_raise(self) -> None:
        gen = CILCodeGenerator()
        result = gen.validate(_bad_syscall_ir())
        assert isinstance(result, list)

    def test_generate_valid_ir_returns_artifact(self) -> None:
        gen = CILCodeGenerator()
        result = gen.generate(_minimal_ir())
        assert isinstance(result, CILProgramArtifact)

    def test_generate_artifact_has_entry_label(self) -> None:
        gen = CILCodeGenerator()
        result = gen.generate(_minimal_ir())
        assert result.entry_label == "_start"

    def test_generate_artifact_has_methods(self) -> None:
        gen = CILCodeGenerator()
        result = gen.generate(_minimal_ir())
        assert len(result.methods) >= 1

    def test_generate_method_body_is_bytes(self) -> None:
        gen = CILCodeGenerator()
        result = gen.generate(_minimal_ir())
        for method in result.methods:
            assert isinstance(method.body, bytes)

    def test_generate_bad_ir_raises(self) -> None:
        gen = CILCodeGenerator()
        with pytest.raises(CILBackendError):
            gen.generate(_bad_syscall_ir())

    def test_round_trip_validate_then_generate(self) -> None:
        gen = CILCodeGenerator()
        ir = _minimal_ir()
        errors = gen.validate(ir)
        assert errors == []
        result = gen.generate(ir)
        assert isinstance(result, CILProgramArtifact)

    def test_custom_config_accepted(self) -> None:
        from ir_to_cil_bytecode.backend import CILBackendConfig
        config = CILBackendConfig(method_max_stack=32)
        gen = CILCodeGenerator(config=config)
        result = gen.generate(_minimal_ir())
        assert isinstance(result, CILProgramArtifact)

    def test_exported_from_package(self) -> None:
        import ir_to_cil_bytecode
        assert hasattr(ir_to_cil_bytecode, "CILCodeGenerator")
