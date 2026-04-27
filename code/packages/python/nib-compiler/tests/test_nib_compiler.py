"""End-to-end tests for the Nib compiler package."""

from __future__ import annotations

import pytest
from intel_4004_packager import decode_hex
from intel4004_simulator import Intel4004Simulator

from nib_compiler import NibCompiler, PackageError, compile_source, pack_source, write_hex_file


def test_compile_source_returns_pipeline_artifacts() -> None:
    source = """
fn main() {
    let x: u4 = 5;
}
"""
    result = compile_source(source)
    assert result.raw_ir is not None
    assert result.optimized_ir is not None
    assert "HLT" in result.assembly
    assert result.binary
    assert result.hex_text.endswith(":00000001FF\n")


def test_pack_source_is_alias_for_compile_source() -> None:
    source = "fn main() { let x: u4 = 5; }"
    compiled = compile_source(source)
    packed = pack_source(source)
    assert packed.hex_text == compiled.hex_text
    assert packed.binary == compiled.binary


def test_write_hex_file_writes_hex_output(tmp_path) -> None:
    source = "fn main() { let x: u4 = 5; }"
    output = tmp_path / "program.hex"
    result = write_hex_file(source, output)
    assert output.exists()
    assert output.read_text(encoding="utf-8") == result.hex_text
    origin, binary = decode_hex(result.hex_text)
    assert origin == 0
    assert binary == result.binary


def test_compiled_program_runs_in_intel4004_simulator() -> None:
    source = "fn main() { let x: u4 = 5; }"
    result = compile_source(source)
    _, binary = decode_hex(result.hex_text)
    sim_result = Intel4004Simulator().execute(binary)
    assert sim_result.halted
    assert sim_result.final_state.registers[2] == 5


def test_type_error_raises_package_error() -> None:
    source = "fn main() { let x: bool = 1 +% 2; }"
    with pytest.raises(PackageError) as exc_info:
        compile_source(source)
    assert exc_info.value.stage == "type-check"


def test_validation_error_raises_package_error() -> None:
    source = """
fn d() { }
fn c() { d(); }
fn b() { c(); }
fn a() { b(); }
fn main() { a(); }
"""
    with pytest.raises(PackageError) as exc_info:
        NibCompiler().compile_source(source)
    assert exc_info.value.stage == "validate"
