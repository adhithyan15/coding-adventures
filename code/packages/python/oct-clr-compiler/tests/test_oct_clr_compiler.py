"""Tests for oct-clr-compiler — end-to-end Oct → CLR pipeline.

Test structure
--------------

1. compile_source — basic artifact checks
2. pack_source — alias equivalence
3. write_assembly_file — writes bytes to disk
4. run_source — full execution via CLR VM simulator
5. OctClrCompiler class API — constructor overrides
6. Error handling — parse / type-check / CIL validation failures
"""

from __future__ import annotations

from pathlib import Path

import pytest

from oct_clr_compiler import (
    OctClrCompiler,
    PackageError,
    compile_source,
    pack_source,
    run_source,
    write_assembly_file,
)

# ---------------------------------------------------------------------------
# Sample Oct programs used across tests
# ---------------------------------------------------------------------------

_EMPTY_MAIN = "fn main() { }"

# Output a single byte.  Under CLR_IO, out(0, 7) emits SYSCALL 1 with value
# 7 in register v2.  The CLR VM's SYSCALL 1 writes the byte using the GE-225
# character table; byte 7 → the character '7'.
_OUTPUT_7 = "fn main() { out(0, 7); }"

# Read one byte and echo it back.
_ECHO = "fn main() { let b: u8 = in(0); out(0, b); }"

# Arithmetic: compute 3 + 4 = 7, output it.
_ARITHMETIC = (
    "fn main() { let a: u8 = 3; let b: u8 = 4; let c: u8 = a + b; out(0, c); }"
)

# ---------------------------------------------------------------------------
# 1. compile_source — basic artifact checks
# ---------------------------------------------------------------------------


def test_compile_source_returns_assembly_bytes() -> None:
    """compile_source returns non-empty assembly bytes for a minimal program."""
    result = compile_source(_EMPTY_MAIN)

    assert result.assembly_bytes
    assert len(result.assembly_bytes) > 0


def test_compile_source_sets_cil_entry_label() -> None:
    """The CIL artifact's entry label is ``_start``."""
    result = compile_source(_EMPTY_MAIN)

    assert result.cil_artifact.entry_label == "_start"


def test_compile_source_sets_decoded_assembly_entry_method() -> None:
    """The decoded PE file exposes a ``_start`` entry-point method."""
    result = compile_source(_EMPTY_MAIN)

    assert result.decoded_assembly.get_entry_point_method().name == "_start"


def test_compile_source_default_names() -> None:
    """Default assembly_name and type_name are 'OctProgram'."""
    result = compile_source(_EMPTY_MAIN)

    assert result.assembly_name == "OctProgram"
    assert result.type_name == "OctProgram"


def test_compile_source_stores_source_text() -> None:
    """PackageResult.source holds the original source string."""
    result = compile_source(_EMPTY_MAIN)

    assert result.source == _EMPTY_MAIN


def test_compile_source_ir_is_not_none() -> None:
    """PackageResult.ir is populated with the compiled IrProgram."""
    result = compile_source(_EMPTY_MAIN)

    assert result.ir is not None


def test_compile_source_assembly_path_is_none_by_default() -> None:
    """assembly_path is None when compile_source is used (no file written)."""
    result = compile_source(_EMPTY_MAIN)

    assert result.assembly_path is None


def test_compile_source_arithmetic_program() -> None:
    """A program using arithmetic compiles successfully."""
    result = compile_source(_ARITHMETIC)

    assert result.assembly_bytes


# ---------------------------------------------------------------------------
# 2. pack_source — alias equivalence
# ---------------------------------------------------------------------------


def test_pack_source_produces_same_bytes_as_compile_source() -> None:
    """pack_source is an alias for compile_source; bytes are identical."""
    compiled = compile_source(_EMPTY_MAIN)
    packed = pack_source(_EMPTY_MAIN)

    assert packed.assembly_bytes == compiled.assembly_bytes
    assert packed.type_name == compiled.type_name


# ---------------------------------------------------------------------------
# 3. write_assembly_file — writes bytes to disk
# ---------------------------------------------------------------------------


def test_write_assembly_file_creates_file(tmp_path: Path) -> None:
    """write_assembly_file writes the PE bytes to the given path."""
    output = tmp_path / "OctProgram.dll"

    result = write_assembly_file(_EMPTY_MAIN, output)

    assert output.exists()
    assert output.read_bytes() == result.assembly_bytes


def test_write_assembly_file_sets_assembly_path(tmp_path: Path) -> None:
    """write_assembly_file populates PackageResult.assembly_path."""
    output = tmp_path / "OctProgram.dll"

    result = write_assembly_file(_EMPTY_MAIN, output)

    assert result.assembly_path == output


def test_write_assembly_file_creates_parent_dirs(tmp_path: Path) -> None:
    """write_assembly_file creates intermediate parent directories."""
    output = tmp_path / "nested" / "deep" / "OctProgram.dll"

    write_assembly_file(_EMPTY_MAIN, output)

    assert output.exists()


# ---------------------------------------------------------------------------
# 4. run_source — full execution via CLR VM simulator
# ---------------------------------------------------------------------------


def test_run_source_empty_program_produces_no_output() -> None:
    """An empty main() function produces no output when executed."""
    result = run_source(_EMPTY_MAIN)

    assert result.vm_result.output == ""


def test_run_source_output_program_produces_character() -> None:
    """out(0, 7) → SYSCALL 1 with value 7 → CLR VM outputs the character '7'."""
    result = run_source(_OUTPUT_7)

    assert result.vm_result.output == "7"


def test_run_source_arithmetic_outputs_correct_character() -> None:
    """3 + 4 = 7; out(0, 7) → '7'."""
    result = run_source(_ARITHMETIC)

    assert result.vm_result.output == "7"


def test_run_source_echo_program_with_input() -> None:
    """in(0) reads a byte; out(0, b) echoes it back."""
    result = run_source(_ECHO, input_bytes=b"\x07")

    assert result.vm_result.output == "7"


def test_run_source_compilation_is_in_execution_result() -> None:
    """ExecutionResult.compilation holds the full PackageResult."""
    result = run_source(_EMPTY_MAIN)

    assert result.compilation.assembly_bytes
    assert result.compilation.ir is not None


# ---------------------------------------------------------------------------
# 5. OctClrCompiler class API — constructor overrides
# ---------------------------------------------------------------------------


def test_compiler_instance_honors_assembly_name() -> None:
    """Constructor assembly_name overrides the default."""
    compiler = OctClrCompiler(assembly_name="MyOct", type_name="my.OctProgram")
    result = compiler.compile_source(_EMPTY_MAIN)

    assert result.assembly_name == "MyOct"
    assert result.type_name == "my.OctProgram"
    assert result.decoded_assembly.get_entry_point_method().declaring_type == (
        "my.OctProgram"
    )


def test_compiler_instance_run_source() -> None:
    """OctClrCompiler.run_source works the same as the module-level function."""
    compiler = OctClrCompiler()
    result = compiler.run_source(_EMPTY_MAIN)

    assert result.vm_result.output == ""


# ---------------------------------------------------------------------------
# 6. Error handling
# ---------------------------------------------------------------------------


def test_parse_error_raises_package_error() -> None:
    """A syntax error in the Oct source raises PackageError at the parse stage."""
    with pytest.raises(PackageError) as exc_info:
        compile_source("fn main() {")  # missing closing brace → parse error

    assert exc_info.value.stage == "parse"


def test_type_check_error_raises_package_error() -> None:
    """A type error in the Oct source raises PackageError at the type-check stage.

    The Oct type checker rejects assigning an integer literal to a bool variable
    because integer and bool are distinct types.  (Boolean literals like ``true``
    are coercible to u8 in Oct's type system, so the opposite direction is used
    here to guarantee a real type mismatch.)
    """
    with pytest.raises(PackageError) as exc_info:
        # 5 is an integer literal; x is declared bool — type mismatch.
        compile_source("fn main() { let x: bool = 5; }")

    assert exc_info.value.stage == "type-check"


def test_unsupported_syscall_raises_package_error() -> None:
    """8008-specific SYSCALLs (port encoding) are rejected by validate_for_clr.

    This test compiles an Oct program using INTEL_8008_IO config directly
    to produce an IR program with SYSCALL 40 (out port 0), then verifies
    that lower_ir_to_cil_bytecode raises a CILBackendError — which the
    pipeline wraps as PackageError("lower-cil", ...).

    This cannot happen through the normal compile_source path (which always
    uses CLR_IO), so the test uses the ir-to-cil-bytecode package directly.
    """
    from compiler_ir import (
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

    # Build a minimal IR program that uses SYSCALL 40 (8008 port encoding).
    program = IrProgram(entry_label="_start")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")]))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(7)])
    )
    program.add_instruction(
        IrInstruction(IrOp.SYSCALL, [IrImmediate(40), IrRegister(2)])
    )
    program.add_instruction(IrInstruction(IrOp.HALT, []))

    with pytest.raises(CILBackendError, match=r"40|pre-flight"):
        lower_ir_to_cil_bytecode(program, CILBackendConfig(syscall_arg_reg=2))
