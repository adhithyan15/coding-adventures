"""End-to-end tests: Oct-style 8-bit IR arithmetic through the JVM backend.

These tests verify that the JVM backend correctly compiles and executes
8-bit integer arithmetic expressed in compiler IR — the same IR that the
Oct compiler generates.

**Why direct IR (not Oct source)?**

Oct's frontend requires a grammar file at a hard-coded relative path that
is only correct when running from the oct-lexer package itself.  When the
oct-lexer package is installed into another package's venv (as a site-package)
the relative path breaks.  To avoid that coupling, we build IR directly using
the compiler_ir primitives, exactly as the Oct IR compiler would produce.

**SYSCALL numbering**

Oct is an Intel 8008-specific language; its ``out(PORT, val)`` intrinsic
maps to ``SYSCALL 40+PORT`` (e.g., ``out(17, v)`` → ``SYSCALL 57``).  Those
numbers are *not* supported by the JVM backend (which uses SYSCALL 1 for
``System.out.write(byte)`` and SYSCALL 4 for ``System.in.read()``).

``test_rejects_oct_out_syscall`` confirms that the validator correctly refuses
Oct I/O SYSCALLs at compile time, giving a clear error rather than silent
misbehavior.

**Execution**

The JVM backend produces standard JVM ``.class`` bytecode; we run it using
the system ``java`` command.  The tests require ``java`` to be on ``PATH``
and skip gracefully if it is not.

**What is being tested?**

- ``LOAD_IMM``  loads an 8-bit constant into a virtual register.
- ``ADD``       adds two registers, producing an 8-bit result.
- ``SUB``       subtracts two registers.
- ``AND``       bitwise-ANDs two registers.
- ``SYSCALL 1`` writes the byte in the arg register to ``System.out``.
- The JVM validator rejects unsupported SYSCALL numbers.
"""
from __future__ import annotations

import subprocess
import tempfile
from pathlib import Path

import pytest
from compiler_ir import (
    IDGenerator,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)

from ir_to_jvm_class_file import (
    JvmBackendConfig,
    lower_ir_to_jvm_class_file,
    validate_for_jvm,
    write_class_file,
)


# ── Helpers ───────────────────────────────────────────────────────────────────

def _reg(i: int) -> IrRegister:
    return IrRegister(index=i)


def _imm(v: int) -> IrImmediate:
    return IrImmediate(value=v)


def _java_available() -> bool:
    try:
        subprocess.run(["java", "-version"], capture_output=True, timeout=5)
        return True
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return False


requires_java = pytest.mark.skipif(
    not _java_available(),
    reason="'java' binary not found on PATH",
)


def _compile_and_run(program: IrProgram, class_name: str) -> bytes:
    """Compile IR → JVM class bytes, run via ``java``, return raw stdout."""
    artifact = lower_ir_to_jvm_class_file(
        program,
        JvmBackendConfig(class_name=class_name),
    )
    with tempfile.TemporaryDirectory() as tmpdir:
        write_class_file(artifact, tmpdir)
        result = subprocess.run(
            ["java", "-cp", tmpdir, class_name],
            capture_output=True,
            timeout=10,
        )
    return result.stdout


# ── Validation ────────────────────────────────────────────────────────────────

class TestValidation:
    def test_simple_8bit_program_passes_validation(self) -> None:
        """A pure-arithmetic Oct-style program passes JVM validation."""
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL,    [IrLabel("_start")], id=-1))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(0), _imm(3)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(7)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.ADD,       [_reg(4), _reg(0), _reg(1)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.SYSCALL,   [_imm(1), _reg(4)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.HALT,      [], id=gen.next()))

        assert validate_for_jvm(program) == []

    def test_rejects_oct_out_syscall(self) -> None:
        """Oct's out(17, val) → SYSCALL 57 is rejected by the JVM validator.

        Oct is an Intel 8008-specific language. Its I/O intrinsics use
        SYSCALL numbers 40+port (output) and 20+port (input). These numbers
        are hardware port numbers for the 8008 and are completely foreign to
        the JVM backend (which supports only SYSCALL 1 and 4).

        This test confirms that the validator catches the mismatch at compile
        time, giving a clear error rather than silent misbehavior at runtime.
        """
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL,    [IrLabel("_start")],  id=-1))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(4), _imm(10)], id=gen.next()))
        # out(17, val) in Oct → SYSCALL 57; not a JVM syscall
        program.add_instruction(IrInstruction(IrOp.SYSCALL,   [_imm(57), _reg(4)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.HALT,      [], id=gen.next()))

        errors = validate_for_jvm(program)
        assert errors, "Expected validation to reject SYSCALL 57"
        assert any("57" in e or "unsupported" in e.lower() for e in errors)


# ── Execution ─────────────────────────────────────────────────────────────────

@requires_java
class TestOct8BitArithmetic:
    """Full pipeline: IR → JVM .class file → java subprocess → correct byte output."""

    def test_addition_3_plus_7_equals_10(self) -> None:
        """3 + 7 = 10 computes correctly and writes byte 10 to stdout."""
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL,    [IrLabel("_start")],         id=-1))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(0), _imm(3)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(7)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.ADD,       [_reg(4), _reg(0), _reg(1)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.SYSCALL,   [_imm(1), _reg(4)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.HALT,      [],                          id=gen.next()))

        assert _compile_and_run(program, "OctAdd") == bytes([10])

    def test_addition_near_u8_max(self) -> None:
        """127 + 127 = 254 — near the u8 maximum, no overflow expected."""
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL,    [IrLabel("_start")],         id=-1))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(0), _imm(127)],        id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.ADD,       [_reg(4), _reg(0), _reg(0)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.SYSCALL,   [_imm(1), _reg(4)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.HALT,      [],                          id=gen.next()))

        assert _compile_and_run(program, "OctLarge") == bytes([254])

    def test_subtraction_10_minus_3_equals_7(self) -> None:
        """10 - 3 = 7."""
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL,    [IrLabel("_start")],         id=-1))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(0), _imm(10)],         id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(3)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.SUB,       [_reg(4), _reg(0), _reg(1)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.SYSCALL,   [_imm(1), _reg(4)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.HALT,      [],                          id=gen.next()))

        assert _compile_and_run(program, "OctSub") == bytes([7])

    def test_bitwise_and_masks_low_nibble(self) -> None:
        """0xFF AND 0x0F = 0x0F — bitwise AND masks out the high nibble."""
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL,    [IrLabel("_start")],         id=-1))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(0), _imm(0xFF)],       id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(0x0F)],       id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.AND,       [_reg(4), _reg(0), _reg(1)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.SYSCALL,   [_imm(1), _reg(4)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.HALT,      [],                          id=gen.next()))

        assert _compile_and_run(program, "OctAnd") == bytes([0x0F])

    def test_multiple_outputs(self) -> None:
        """Compute two values and write both; verifies multi-instruction programs."""
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL,    [IrLabel("_start")],         id=-1))
        # First output: 2 + 3 = 5  (result in v4, the SYSCALL arg register)
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(0), _imm(2)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(3)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.ADD,       [_reg(4), _reg(0), _reg(1)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.SYSCALL,   [_imm(1), _reg(4)],          id=gen.next()))
        # Second output: 10 - 5 = 5  (use explicit v2/v3 to avoid read-write aliasing on v4)
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(2), _imm(10)],         id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(3), _imm(5)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.SUB,       [_reg(4), _reg(2), _reg(3)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.SYSCALL,   [_imm(1), _reg(4)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.HALT,      [],                          id=gen.next()))

        assert _compile_and_run(program, "OctMulti") == bytes([5, 5])
