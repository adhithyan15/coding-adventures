"""Tests for oct-wasm-compiler — end-to-end Oct → WASM pipeline.

Test structure
--------------

1. compile_source — basic artifact checks
2. pack_source — alias equivalence
3. write_wasm_file — writes bytes to disk
4. OctWasmCompiler class API — instance method
5. Error handling — parse / type-check failures
"""

from __future__ import annotations

from pathlib import Path

import pytest

from oct_wasm_compiler import (
    OctWasmCompiler,
    PackageError,
    compile_source,
    pack_source,
    write_wasm_file,
)

# WASM magic number: every valid .wasm file starts with these 4 bytes.
_WASM_MAGIC = b"\x00asm"

# ---------------------------------------------------------------------------
# Sample Oct programs used across tests
# ---------------------------------------------------------------------------

_EMPTY_MAIN = "fn main() { }"

_OUTPUT_7 = "fn main() { out(0, 7); }"

_ARITHMETIC = (
    "fn main() { let a: u8 = 3; let b: u8 = 4; let c: u8 = a + b; out(0, c); }"
)

_WITH_IF = """
fn main() {
    let x: u8 = 5;
    if x > 3 {
        out(0, x);
    }
}
"""

_WITH_STATIC = """
static LIMIT: u8 = 10;

fn main() {
    let x: u8 = LIMIT;
    out(0, x);
}
"""

# ---------------------------------------------------------------------------
# 1. compile_source — basic artifact checks
# ---------------------------------------------------------------------------


def test_compile_source_returns_wasm_magic() -> None:
    """Every compiled WASM binary starts with the 4-byte WASM magic number."""
    result = compile_source(_EMPTY_MAIN)

    assert result.binary[:4] == _WASM_MAGIC


def test_compile_source_binary_is_non_empty() -> None:
    """The compiled binary is non-empty for all programs."""
    result = compile_source(_EMPTY_MAIN)

    assert len(result.binary) > 0


def test_compile_source_has_wasm_assembly_text() -> None:
    """PackageResult.wasm_assembly is a non-empty string."""
    result = compile_source(_EMPTY_MAIN)

    assert isinstance(result.wasm_assembly, str)
    assert len(result.wasm_assembly) > 0


def test_compile_source_validated_module_is_present() -> None:
    """PackageResult.validated_module is populated."""
    result = compile_source(_EMPTY_MAIN)

    assert result.validated_module is not None


def test_compile_source_stores_source_text() -> None:
    """PackageResult.source holds the original source string."""
    result = compile_source(_EMPTY_MAIN)

    assert result.source == _EMPTY_MAIN


def test_compile_source_ir_is_not_none() -> None:
    """PackageResult.ir is populated with the compiled IrProgram."""
    result = compile_source(_EMPTY_MAIN)

    assert result.ir is not None


def test_compile_source_wasm_path_is_none_by_default() -> None:
    """wasm_path is None when compile_source is used (no file written)."""
    result = compile_source(_EMPTY_MAIN)

    assert result.wasm_path is None


def test_compile_source_output_program() -> None:
    """A program that calls out() compiles to a valid WASM binary."""
    result = compile_source(_OUTPUT_7)

    assert result.binary[:4] == _WASM_MAGIC


def test_compile_source_arithmetic_program() -> None:
    """A program using arithmetic compiles to a valid WASM binary."""
    result = compile_source(_ARITHMETIC)

    assert result.binary[:4] == _WASM_MAGIC


def test_compile_source_if_statement() -> None:
    """A program with an if statement compiles to a valid WASM binary."""
    result = compile_source(_WITH_IF)

    assert result.binary[:4] == _WASM_MAGIC


def test_compile_source_static_variable() -> None:
    """A program with a static variable compiles to a valid WASM binary."""
    result = compile_source(_WITH_STATIC)

    assert result.binary[:4] == _WASM_MAGIC


def test_compile_source_wasm_assembly_contains_start() -> None:
    """The WASM assembly text references the _start entry point."""
    result = compile_source(_EMPTY_MAIN)

    assert "_start" in result.wasm_assembly


# ---------------------------------------------------------------------------
# 2. pack_source — alias equivalence
# ---------------------------------------------------------------------------


def test_pack_source_produces_same_binary_as_compile_source() -> None:
    """pack_source is an alias for compile_source; binaries are identical."""
    compiled = compile_source(_EMPTY_MAIN)
    packed = pack_source(_EMPTY_MAIN)

    assert packed.binary == compiled.binary


# ---------------------------------------------------------------------------
# 3. write_wasm_file — writes bytes to disk
# ---------------------------------------------------------------------------


def test_write_wasm_file_creates_file(tmp_path: Path) -> None:
    """write_wasm_file writes the WASM binary to the given path."""
    output = tmp_path / "program.wasm"

    result = write_wasm_file(_EMPTY_MAIN, output)

    assert output.exists()
    assert output.read_bytes() == result.binary


def test_write_wasm_file_sets_wasm_path(tmp_path: Path) -> None:
    """write_wasm_file populates PackageResult.wasm_path."""
    output = tmp_path / "program.wasm"

    result = write_wasm_file(_EMPTY_MAIN, output)

    assert result.wasm_path == output


def test_write_wasm_file_creates_parent_dirs(tmp_path: Path) -> None:
    """write_wasm_file creates intermediate parent directories."""
    output = tmp_path / "nested" / "program.wasm"

    write_wasm_file(_EMPTY_MAIN, output)

    assert output.exists()


# ---------------------------------------------------------------------------
# 4. OctWasmCompiler class API
# ---------------------------------------------------------------------------


def test_compiler_instance_compile_source() -> None:
    """OctWasmCompiler().compile_source works the same as the module function."""
    compiler = OctWasmCompiler()
    result = compiler.compile_source(_EMPTY_MAIN)

    assert result.binary[:4] == _WASM_MAGIC


# ---------------------------------------------------------------------------
# 5. Error handling
# ---------------------------------------------------------------------------


def test_parse_error_raises_package_error() -> None:
    """A syntax error in the Oct source raises PackageError at the parse stage."""
    with pytest.raises(PackageError) as exc_info:
        compile_source("fn main() {")  # missing closing brace

    assert exc_info.value.stage == "parse"


def test_type_check_error_raises_package_error() -> None:
    """A type error in the Oct source raises PackageError at the type-check stage.

    Assigns an integer literal to a bool variable, which the Oct type checker
    rejects.  (Boolean literals are coercible to u8 in Oct's type system, so
    the opposite assignment is used to guarantee a real type mismatch.)
    """
    with pytest.raises(PackageError) as exc_info:
        compile_source("fn main() { let x: bool = 5; }")

    assert exc_info.value.stage == "type-check"
