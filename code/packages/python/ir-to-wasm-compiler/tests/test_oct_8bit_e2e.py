"""End-to-end tests: Oct-style 8-bit IR arithmetic through the WASM backend.

These tests verify that the WASM backend correctly compiles and executes
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
numbers are *not* supported by the WASM backend (which speaks WASI: 1=fd_write,
2=fd_read, 10=proc_exit).  These tests therefore use SYSCALL 1 for output,
which *is* supported.  ``test_rejects_oct_out_syscall`` confirms that the
validator correctly refuses Oct I/O SYSCALLs at compile time.

**What is being tested?**

- ``LOAD_IMM``  loads an 8-bit constant into a virtual register.
- ``ADD``       adds two registers, producing an 8-bit result.
- ``SUB``       subtracts two registers.
- ``AND``       bitwise-ANDs two registers.
- ``SYSCALL 1`` (WASI fd_write) writes the value in the arg register to stdout.
- The WASM validator rejects unsupported SYSCALL numbers.
"""
from __future__ import annotations

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
from wasm_module_encoder import encode_module
from wasm_runtime import WasiConfig, WasiHost, WasmRuntime

from ir_to_wasm_compiler import FunctionSignature, IrToWasmCompiler, validate_for_wasm


# ── Helpers ───────────────────────────────────────────────────────────────────

def _reg(i: int) -> IrRegister:
    return IrRegister(index=i)


def _imm(v: int) -> IrImmediate:
    return IrImmediate(value=v)


def _compile_and_run(program: IrProgram) -> list[int]:
    """Compile IR → WASM, run in WASI runtime, return received byte values.

    WasiHost delivers each byte as a single-character string; we convert
    back to integers with ord() so callers can compare plain ints.
    """
    module = IrToWasmCompiler().compile(
        program,
        function_signatures=[
            FunctionSignature(label="_start", param_count=0, export_name="_start"),
        ],
    )
    output: list[str] = []
    host = WasiHost(config=WasiConfig(stdout=output.append))
    WasmRuntime(host=host).load_and_run(encode_module(module), "_start", [])
    return [ord(ch) if isinstance(ch, str) else ch for ch in output]


def _simple_program(*body_instructions: IrInstruction) -> IrProgram:
    """Build an IrProgram with a ``_start`` entry and the given body."""
    gen = IDGenerator()
    program = IrProgram(entry_label="_start")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    for instr in body_instructions:
        program.add_instruction(instr)
    return program


def _instr(op: IrOp, *operands: IrRegister | IrImmediate) -> IrInstruction:
    gen = IDGenerator()
    return IrInstruction(op, list(operands), id=gen.next())


# ── Validation ────────────────────────────────────────────────────────────────

class TestValidation:
    def test_simple_8bit_program_passes_validation(self) -> None:
        """A pure-arithmetic Oct-style program passes WASM validation."""
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL,    [IrLabel("_start")], id=-1))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(0), _imm(3)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(7)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.ADD,       [_reg(4), _reg(0), _reg(1)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.SYSCALL,   [_imm(1), _reg(4)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.HALT,      [], id=gen.next()))

        assert validate_for_wasm(program) == []

    def test_rejects_oct_out_syscall(self) -> None:
        """Oct's out(17, val) → SYSCALL 57 is rejected by the WASM validator.

        Oct is an Intel 8008-specific language. Its I/O intrinsics use
        SYSCALL numbers 40+port (output) and 20+port (input). These numbers
        are hardware port numbers for the 8008 and are completely foreign to
        the WASM backend (which speaks WASI: 1/2/10).

        This test confirms that the validator catches the mismatch at compile
        time, giving a clear error rather than silent misbehavior at runtime.
        """
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL,   [IrLabel("_start")],  id=-1))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(4), _imm(10)], id=gen.next()))
        # out(17, val) in Oct → SYSCALL 57; not a WASI syscall
        program.add_instruction(IrInstruction(IrOp.SYSCALL,  [_imm(57), _reg(4)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.HALT,     [], id=gen.next()))

        errors = validate_for_wasm(program)
        assert errors, "Expected validation to reject SYSCALL 57"
        assert any("57" in e or "unsupported" in e.lower() for e in errors)


# ── Execution ─────────────────────────────────────────────────────────────────

class TestOct8BitArithmetic:
    """Full pipeline: IR → WASM binary → WASI runtime → correct byte output."""

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

        assert _compile_and_run(program) == [10]

    def test_addition_near_u8_max(self) -> None:
        """127 + 127 = 254 — near the u8 maximum, no overflow expected."""
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL,    [IrLabel("_start")],         id=-1))
        program.add_instruction(IrInstruction(IrOp.LOAD_IMM, [_reg(0), _imm(127)],        id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.ADD,       [_reg(4), _reg(0), _reg(0)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.SYSCALL,   [_imm(1), _reg(4)],          id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.HALT,      [],                          id=gen.next()))

        assert _compile_and_run(program) == [254]

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

        assert _compile_and_run(program) == [7]

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

        assert _compile_and_run(program) == [0x0F]

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

        assert _compile_and_run(program) == [5, 5]
