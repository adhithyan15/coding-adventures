"""Tests for oct-jvm-compiler — end-to-end Oct → JVM class-file pipeline.

Test structure
--------------

1. compile_source — basic artifact checks
2. pack_source — alias equivalence
3. write_class_file — writes bytes to disk
4. OctJvmCompiler class API — constructor overrides
5. Error handling — parse / type-check failures
"""

from __future__ import annotations

from pathlib import Path

import pytest

from oct_jvm_compiler import (
    OctJvmCompiler,
    PackageError,
    compile_source,
    pack_source,
    write_class_file,
)

# JVM class-file magic number: every valid .class file starts with 0xCAFEBABE.
_JVM_MAGIC = b"\xca\xfe\xba\xbe"

# ---------------------------------------------------------------------------
# Sample Oct programs used across tests
# ---------------------------------------------------------------------------

_EMPTY_MAIN = "fn main() { }"

_OUTPUT_7 = "fn main() { out(0, 7); }"

_ARITHMETIC = (
    "fn main() { let a: u8 = 3; let b: u8 = 4; let c: u8 = a + b; out(0, c); }"
)

_WITH_WHILE = """
fn main() {
    let i: u8 = 0;
    while i < 3 {
        out(0, i);
        i = i + 1;
    }
}
"""

_WITH_FN_CALL = """
fn double(x: u8) -> u8 {
    return x + x;
}

fn main() {
    let d: u8 = double(3);
    out(0, d);
}
"""

# ---------------------------------------------------------------------------
# 1. compile_source — basic artifact checks
# ---------------------------------------------------------------------------


def test_compile_source_returns_jvm_magic() -> None:
    """Every compiled .class file starts with 0xCAFEBABE."""
    result = compile_source(_EMPTY_MAIN)

    assert result.class_bytes[:4] == _JVM_MAGIC


def test_compile_source_class_bytes_non_empty() -> None:
    """The compiled class bytes are non-empty for all programs."""
    result = compile_source(_EMPTY_MAIN)

    assert len(result.class_bytes) > 0


def test_compile_source_parsed_class_is_present() -> None:
    """PackageResult.parsed_class is populated (structural validation passed)."""
    result = compile_source(_EMPTY_MAIN)

    assert result.parsed_class is not None


def test_compile_source_default_class_name() -> None:
    """Default class_name is 'OctProgram'."""
    result = compile_source(_EMPTY_MAIN)

    assert result.class_name == "OctProgram"


def test_compile_source_stores_source_text() -> None:
    """PackageResult.source holds the original source string."""
    result = compile_source(_EMPTY_MAIN)

    assert result.source == _EMPTY_MAIN


def test_compile_source_ir_is_not_none() -> None:
    """PackageResult.ir is populated with the compiled IrProgram."""
    result = compile_source(_EMPTY_MAIN)

    assert result.ir is not None


def test_compile_source_class_file_path_none_by_default() -> None:
    """class_file_path is None when compile_source is used (no file written)."""
    result = compile_source(_EMPTY_MAIN)

    assert result.class_file_path is None


def test_compile_source_output_program() -> None:
    """A program that calls out() compiles to a valid JVM class file."""
    result = compile_source(_OUTPUT_7)

    assert result.class_bytes[:4] == _JVM_MAGIC


def test_compile_source_arithmetic_program() -> None:
    """A program using arithmetic compiles to a valid JVM class file."""
    result = compile_source(_ARITHMETIC)

    assert result.class_bytes[:4] == _JVM_MAGIC


def test_compile_source_while_loop() -> None:
    """A program with a while loop compiles to a valid JVM class file."""
    result = compile_source(_WITH_WHILE)

    assert result.class_bytes[:4] == _JVM_MAGIC


def test_compile_source_function_call() -> None:
    """A program with a user-defined function call compiles successfully."""
    result = compile_source(_WITH_FN_CALL)

    assert result.class_bytes[:4] == _JVM_MAGIC


# ---------------------------------------------------------------------------
# 2. pack_source — alias equivalence
# ---------------------------------------------------------------------------


def test_pack_source_produces_same_bytes_as_compile_source() -> None:
    """pack_source is an alias for compile_source; bytes are identical."""
    compiled = compile_source(_EMPTY_MAIN)
    packed = pack_source(_EMPTY_MAIN)

    assert packed.class_bytes == compiled.class_bytes


# ---------------------------------------------------------------------------
# 3. write_class_file — writes bytes to disk
# ---------------------------------------------------------------------------


def test_write_class_file_creates_file(tmp_path: Path) -> None:
    """write_class_file writes the .class bytes to the output directory."""
    result = write_class_file(_EMPTY_MAIN, tmp_path)

    expected = tmp_path / "OctProgram.class"
    assert expected.exists()
    assert expected.read_bytes() == result.class_bytes


def test_write_class_file_sets_class_file_path(tmp_path: Path) -> None:
    """write_class_file populates PackageResult.class_file_path."""
    result = write_class_file(_EMPTY_MAIN, tmp_path)

    assert result.class_file_path is not None
    assert result.class_file_path.suffix == ".class"


def test_write_class_file_custom_class_name(tmp_path: Path) -> None:
    """write_class_file uses the custom class_name in the output filename."""
    result = write_class_file(_EMPTY_MAIN, tmp_path, class_name="MyOct")

    expected = tmp_path / "MyOct.class"
    assert expected.exists()
    assert result.class_name == "MyOct"


# ---------------------------------------------------------------------------
# 4. OctJvmCompiler class API
# ---------------------------------------------------------------------------


def test_compiler_instance_honors_class_name() -> None:
    """OctJvmCompiler(class_name=...) overrides the default class name."""
    compiler = OctJvmCompiler(class_name="SpecialOct")
    result = compiler.compile_source(_EMPTY_MAIN)

    assert result.class_name == "SpecialOct"


def test_compiler_instance_compile_source() -> None:
    """OctJvmCompiler().compile_source works the same as the module function."""
    compiler = OctJvmCompiler()
    result = compiler.compile_source(_EMPTY_MAIN)

    assert result.class_bytes[:4] == _JVM_MAGIC


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
