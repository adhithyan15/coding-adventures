"""End-to-end tests: Oct-style 8-bit IR arithmetic through the CLR backend.

These tests verify that the CLR backend correctly compiles and executes
8-bit integer arithmetic expressed in compiler IR — the same IR that the
Oct compiler generates.

**Why this file lives in brainfuck-clr-compiler**

The full CLR execution pipeline requires three packages beyond
``ir-to-cil-bytecode`` itself: ``cli-assembly-writer`` (to package CIL into
a PE binary), ``clr-vm-simulator`` (to execute the binary), and
``clr-pe-file`` (to decode/validate it).  Adding those as dev-dependencies
of ``ir-to-cil-bytecode`` would create a heavy circular dependency.
``brainfuck-clr-compiler`` already depends on all four packages as production
dependencies, so this test file lives here as a natural home.

**Why direct IR (not Oct source)?**

Oct's frontend requires a grammar file at a hard-coded relative path that
is only correct when running from the oct-lexer package itself.  When
oct-lexer is installed as a site-package in another venv the path breaks.
We build IR directly using the compiler_ir primitives — exactly what the Oct
IR compiler would produce — to avoid that coupling.

**SYSCALL numbering**

Oct is an Intel 8008-specific language; its ``out(PORT, val)`` intrinsic maps
to ``SYSCALL 40+PORT`` (e.g., ``out(17, v)`` → ``SYSCALL 57``).  Those
numbers are not supported by the CLR host (which implements SYSCALL 1=write
byte, 2=read byte, 10=exit).

The CLR backend now has a compile-time ``validate_for_clr()`` pre-flight
check (the same pattern as ``validate_for_wasm`` and ``validate_for_jvm``).
Unsupported SYSCALL numbers are rejected by ``lower_ir_to_cil_bytecode()``
with a ``CILBackendError`` *before* any CIL bytes are produced.
``test_rejects_oct_out_syscall_at_compile_time`` documents this behaviour.

**Output encoding**

The CLR host's SYSCALL 1 maps byte values through the GE-225 character
encoding used by the original Dartmouth BASIC / GE-225 mainframe.  In
GE-225, code points 0–9 (octal 00–11) map to the ASCII digit characters
'0'–'9', and the fallback for unmapped values is ``chr(value & 0xFF)``.
For example:

    value 7  → GE-225 → '7' (digit character)     ord = 55
    value 10 → fallback → chr(10) = '\\n'           ord = 10
    value 15 → fallback → chr(15) = '\\x0f'         ord = 15

Tests use ``CLRVMResult.output`` (a string) and compare against the
expected GE-225 character.

**What is being tested?**

- ``LOAD_IMM``  loads an 8-bit constant into a virtual register.
- ``ADD``       adds two registers, producing an 8-bit result.
- ``SUB``       subtracts two registers.
- ``AND``       bitwise-ANDs two registers.
- ``SYSCALL 1`` writes the value in the arg register to the CLR host output.
- Unsupported SYSCALL numbers raise ``CILBackendError`` at compile time.
"""
from __future__ import annotations

import pytest
from cli_assembly_writer import CLIAssemblyConfig, write_cli_assembly
from clr_vm_simulator import CLRVMStdlibHost, run_clr_entry_point
from compiler_ir import (
    IDGenerator,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)
from ir_to_cil_bytecode import (
    CILBackendConfig,
    CILBackendError,
    lower_ir_to_cil_bytecode,
)

# GE-225 character table (subset): maps integer values used in these tests
# to the expected output character.  The CLR host uses this encoding for
# SYSCALL 1.  Values not in the GE-225 table fall through to chr(value).
_GE225_EXPECTED: dict[int, str] = {
    0: "0", 1: "1", 2: "2", 3: "3", 4: "4",
    5: "5", 6: "6", 7: "7", 8: "8", 9: "9",
    # 10 is NOT in the GE-225 table → chr(10) = '\n'
}


def _ge225_char(value: int) -> str:
    """Return the expected CLR output character for a given integer value."""
    return _GE225_EXPECTED.get(value, chr(value & 0xFF))


# ── Helpers ───────────────────────────────────────────────────────────────────

def _reg(i: int) -> IrRegister:
    return IrRegister(index=i)


def _imm(v: int) -> IrImmediate:
    return IrImmediate(value=v)


def _compile_and_run(program: IrProgram, input_bytes: bytes = b"") -> str:
    """Compile IR → CIL → CLI assembly, run in CLR VM, return output string."""
    cil_artifact = lower_ir_to_cil_bytecode(
        program,
        CILBackendConfig(syscall_arg_reg=4),
    )
    assembly_artifact = write_cli_assembly(
        cil_artifact,
        CLIAssemblyConfig(
            assembly_name="OctTest",
            module_name="OctTest.dll",
            type_name="OctTest",
        ),
    )
    host = CLRVMStdlibHost(input_bytes=input_bytes)
    result = run_clr_entry_point(assembly_artifact.assembly_bytes, host=host)
    return result.output


# ── Validation ────────────────────────────────────────────────────────────────

class TestOctSyscallBehaviour:
    def test_rejects_oct_out_syscall_at_compile_time(self) -> None:
        """Oct's out(17, val) → SYSCALL 57 is rejected at compile time.

        The CLR backend now includes a ``validate_for_clr()`` pre-flight check
        (mirroring the pattern from ``validate_for_wasm`` and
        ``validate_for_jvm``).  Unsupported SYSCALL numbers are caught by
        ``lower_ir_to_cil_bytecode()`` *before* any CIL bytes are produced.

        SYSCALL 57 is Oct's ``out(17, val)`` — an Intel 8008 hardware port
        write (SYSCALL 40+PORT = 40+17 = 57) that is completely foreign to the
        CLR host which only understands SYSCALLs 1, 2, and 10.
        """
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL,    [IrLabel("_start")],  id=-1))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(4), _imm(10)], id=gen.next()))
        # out(17, val) in Oct → SYSCALL 57; not a CLR host syscall
        program.add_instruction(IrInstruction(IrOp.SYSCALL,   [_imm(57), _reg(4)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.HALT,      [], id=gen.next()))

        # The validator fires inside lower_ir_to_cil_bytecode() — no CIL bytes
        # are produced, no assembly is written, and no CLR VM is invoked.
        with pytest.raises(CILBackendError, match=r"57|pre-flight"):
            lower_ir_to_cil_bytecode(program, CILBackendConfig(syscall_arg_reg=4))


# ── Execution ─────────────────────────────────────────────────────────────────

class TestOct8BitArithmetic:
    """Full pipeline: IR → CIL bytecode → CLI PE → CLR VM → correct output.

    The CLR host uses GE-225 encoding for SYSCALL 1 output.  Each test
    checks both the raw output string and the expected arithmetic result.
    See the module docstring for the GE-225 mapping details.
    """

    def test_addition_3_plus_7_equals_10(self) -> None:
        """3 + 7 = 10; CLR host outputs chr(10) = '\\n' (not in GE-225 table)."""
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL,    [IrLabel("_start")],         id=-1))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(0), _imm(3)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(7)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.ADD,       [_reg(4), _reg(0), _reg(1)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.SYSCALL,   [_imm(1), _reg(4)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.HALT,      [],                          id=gen.next()))

        output = _compile_and_run(program)
        assert output == _ge225_char(10)   # chr(10) = '\n'
        assert ord(output) == 10

    def test_addition_near_u8_max(self) -> None:
        """127 + 127 = 254; CLR host outputs chr(254) = 'þ' (not in GE-225 table)."""
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL,    [IrLabel("_start")],         id=-1))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(0), _imm(127)],        id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.ADD,       [_reg(4), _reg(0), _reg(0)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.SYSCALL,   [_imm(1), _reg(4)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.HALT,      [],                          id=gen.next()))

        output = _compile_and_run(program)
        assert output == _ge225_char(254)   # chr(254) = 'þ'
        assert ord(output) == 254

    def test_subtraction_10_minus_3_equals_7(self) -> None:
        """10 - 3 = 7; CLR host outputs '7' (GE-225 code 7 = digit '7')."""
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL,    [IrLabel("_start")],         id=-1))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(0), _imm(10)],         id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(3)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.SUB,       [_reg(4), _reg(0), _reg(1)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.SYSCALL,   [_imm(1), _reg(4)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.HALT,      [],                          id=gen.next()))

        output = _compile_and_run(program)
        # In GE-225, code 7 maps to the digit character '7', NOT chr(7).
        assert output == _ge225_char(7)   # '7'

    def test_bitwise_and_masks_low_nibble(self) -> None:
        """0xFF AND 0x0F = 0x0F = 15; CLR host outputs chr(15) = '\\x0f'."""
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL,    [IrLabel("_start")],         id=-1))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(0), _imm(0xFF)],       id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(0x0F)],       id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.AND,       [_reg(4), _reg(0), _reg(1)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.SYSCALL,   [_imm(1), _reg(4)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.HALT,      [],                          id=gen.next()))

        output = _compile_and_run(program)
        assert output == _ge225_char(0x0F)   # chr(15) = '\x0f'
        assert ord(output) == 15

    def test_multiple_outputs(self) -> None:
        """Two separate SYSCALL 1 writes: 2+3=5, then 10-5=5."""
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL,    [IrLabel("_start")],         id=-1))
        # First: 2 + 3 = 5  (result in v4, the SYSCALL arg register)
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(0), _imm(2)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(3)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.ADD,       [_reg(4), _reg(0), _reg(1)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.SYSCALL,   [_imm(1), _reg(4)],          id=gen.next()))
        # Second: 10 - 5 = 5  (use v2 for intermediate to avoid read-write aliasing on v4)
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(2), _imm(10)],         id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(3), _imm(5)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.SUB,       [_reg(4), _reg(2), _reg(3)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.SYSCALL,   [_imm(1), _reg(4)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.HALT,      [],                          id=gen.next()))

        output = _compile_and_run(program)
        # Both outputs are 5; GE-225 code 5 = digit character '5'
        assert output == _ge225_char(5) + _ge225_char(5)  # '55'
