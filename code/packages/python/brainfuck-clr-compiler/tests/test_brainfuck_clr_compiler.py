from __future__ import annotations

from pathlib import Path

import pytest
from cli_runtime_model import CliValue

from brainfuck_clr_compiler import (
    BrainfuckClrCompiler,
    PackageError,
    compile_source,
    pack_source,
    run_source,
    write_assembly_file,
)


def test_compile_source_returns_pipeline_artifacts() -> None:
    result = compile_source("+.")

    assert result.raw_ir is not None
    assert result.optimized_ir is not None
    assert result.cil_artifact.entry_label == "_start"
    assert result.assembly_bytes
    assert result.decoded_assembly.get_entry_point_method().name == "_start"
    assert result.type_name == "BrainfuckProgram"


def test_pack_source_is_alias_for_compile_source() -> None:
    compiled = compile_source("+.")
    packed = pack_source("+.")

    assert packed.assembly_bytes == compiled.assembly_bytes
    assert packed.type_name == compiled.type_name


def test_write_assembly_file_writes_output(tmp_path: Path) -> None:
    output_path = tmp_path / "BrainfuckProgram.dll"

    result = write_assembly_file("+.", output_path)

    assert result.assembly_path == output_path
    assert output_path.read_bytes() == result.assembly_bytes


def test_run_source_executes_output_program() -> None:
    result = run_source("+" * 65 + ".")

    assert result.vm_result.output == "A"
    assert result.vm_result.return_value == CliValue.int32(0)


def test_run_source_executes_input_program() -> None:
    result = run_source(",.", input_bytes=b"Z")

    assert result.vm_result.output == "Z"
    assert result.vm_result.return_value == CliValue.int32(0)


def test_compiler_instance_honors_names() -> None:
    result = BrainfuckClrCompiler(
        filename="hello.bf",
        assembly_name="HelloBrainfuck",
        type_name="demo.HelloBrainfuck",
    ).compile_source("+.")

    assert result.filename == "hello.bf"
    assert result.assembly_name == "HelloBrainfuck"
    assert result.type_name == "demo.HelloBrainfuck"
    assert result.decoded_assembly.get_entry_point_method().declaring_type == (
        "demo.HelloBrainfuck"
    )


def test_parse_error_raises_package_error() -> None:
    with pytest.raises(PackageError) as exc_info:
        compile_source("[")

    assert exc_info.value.stage == "parse"
