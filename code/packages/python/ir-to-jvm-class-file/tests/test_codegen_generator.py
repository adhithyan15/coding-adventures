"""Tests for JVMCodeGenerator — CodeGenerator[IrProgram, JVMClassArtifact] (LANG20).

Covers: name, validate (valid + invalid IR), generate, protocol check, round-trip.
"""

from __future__ import annotations

import pytest

from codegen_core import CodeGenerator
from compiler_ir import IrImmediate, IrInstruction, IrLabel, IrOp, IrProgram, IrRegister
from ir_to_jvm_class_file import JVMCodeGenerator
from ir_to_jvm_class_file.backend import JVMClassArtifact, JvmBackendError


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _minimal_ir() -> IrProgram:
    """Minimal valid IrProgram: LABEL _start; LOAD_IMM r0, 1; HALT.

    The JVM backend requires an explicit LABEL instruction for each callable
    entry point — the entry_label= parameter alone is not enough; the backend
    scans the instruction list for LABEL nodes to determine method boundaries.
    """
    prog = IrProgram(entry_label="_start")
    prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    prog.add_instruction(IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(1)], id=0))
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=1))
    return prog


def _bad_syscall_ir() -> IrProgram:
    """IrProgram with an unsupported SYSCALL number (99 is not in {1, 4})."""
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
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(2**32)], id=0)
    )
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=1))
    return prog


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestJVMCodeGenerator:

    def test_name(self) -> None:
        assert JVMCodeGenerator().name == "jvm"

    def test_satisfies_codegenerator_protocol(self) -> None:
        assert isinstance(JVMCodeGenerator(), CodeGenerator)

    def test_validate_valid_ir_returns_empty(self) -> None:
        gen = JVMCodeGenerator()
        assert gen.validate(_minimal_ir()) == []

    def test_validate_bad_syscall_returns_errors(self) -> None:
        gen = JVMCodeGenerator()
        errors = gen.validate(_bad_syscall_ir())
        assert len(errors) >= 1
        assert any("syscall" in e.lower() or "99" in e for e in errors)

    def test_validate_overflow_constant_returns_errors(self) -> None:
        gen = JVMCodeGenerator()
        errors = gen.validate(_overflow_constant_ir())
        assert len(errors) >= 1

    def test_validate_does_not_raise(self) -> None:
        gen = JVMCodeGenerator()
        result = gen.validate(_bad_syscall_ir())
        assert isinstance(result, list)

    def test_generate_valid_ir_returns_artifact(self) -> None:
        gen = JVMCodeGenerator()
        result = gen.generate(_minimal_ir())
        assert isinstance(result, JVMClassArtifact)

    def test_generate_class_bytes_is_bytes(self) -> None:
        gen = JVMCodeGenerator()
        result = gen.generate(_minimal_ir())
        assert isinstance(result.class_bytes, bytes)

    def test_generate_class_bytes_nonempty(self) -> None:
        gen = JVMCodeGenerator()
        result = gen.generate(_minimal_ir())
        assert len(result.class_bytes) > 0

    def test_generate_class_bytes_magic_header(self) -> None:
        """JVM class files must start with 0xCAFEBABE."""
        gen = JVMCodeGenerator()
        result = gen.generate(_minimal_ir())
        assert result.class_bytes[:4] == b"\xca\xfe\xba\xbe"

    def test_generate_has_class_name(self) -> None:
        gen = JVMCodeGenerator()
        result = gen.generate(_minimal_ir())
        assert isinstance(result.class_name, str)
        assert len(result.class_name) > 0

    def test_generate_bad_ir_raises(self) -> None:
        gen = JVMCodeGenerator()
        with pytest.raises(JvmBackendError):
            gen.generate(_bad_syscall_ir())

    def test_round_trip_validate_then_generate(self) -> None:
        gen = JVMCodeGenerator()
        ir = _minimal_ir()
        errors = gen.validate(ir)
        assert errors == []
        result = gen.generate(ir)
        assert result.class_bytes is not None

    def test_custom_config_accepted(self) -> None:
        """JVMCodeGenerator accepts an optional JvmBackendConfig."""
        from ir_to_jvm_class_file.backend import JvmBackendConfig
        config = JvmBackendConfig(class_name="TestClass")
        gen = JVMCodeGenerator(config=config)
        result = gen.generate(_minimal_ir())
        assert "TestClass" in result.class_name

    def test_exported_from_package(self) -> None:
        import ir_to_jvm_class_file
        assert hasattr(ir_to_jvm_class_file, "JVMCodeGenerator")
