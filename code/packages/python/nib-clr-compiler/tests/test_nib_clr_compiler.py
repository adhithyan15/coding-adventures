from __future__ import annotations

from pathlib import Path

import pytest
from cli_runtime_model import CliValue

from nib_clr_compiler import (
    NibClrCompiler,
    PackageError,
    compile_source,
    pack_source,
    run_source,
    write_assembly_file,
)


def test_compile_source_returns_pipeline_artifacts() -> None:
    result = compile_source("fn main() -> u4 { return 7; }")

    assert result.raw_ir is not None
    assert result.optimized_ir is not None
    assert result.cil_artifact.entry_label == "_start"
    assert result.assembly_bytes
    assert result.decoded_assembly.get_entry_point_method().name == "_start"
    assert result.type_name == "NibProgram"


def test_pack_source_is_alias_for_compile_source() -> None:
    source = "fn main() -> u4 { return 7; }"
    compiled = compile_source(source)
    packed = pack_source(source)

    assert packed.assembly_bytes == compiled.assembly_bytes
    assert packed.type_name == compiled.type_name


def test_write_assembly_file_writes_output(tmp_path: Path) -> None:
    output_path = tmp_path / "NibProgram.dll"

    result = write_assembly_file("fn main() -> u4 { return 7; }", output_path)

    assert result.assembly_path == output_path
    assert output_path.read_bytes() == result.assembly_bytes


def test_run_source_executes_main_return() -> None:
    result = run_source("fn main() -> u4 { return 7; }")

    assert result.vm_result.return_value == CliValue.int32(7)
    assert result.vm_result.output == ""


def test_run_source_executes_function_call_with_arguments() -> None:
    source = """
fn add(a: u4, b: u4) -> u4 {
    return a +% b;
}

fn main() -> u4 {
    return add(3, 4);
}
"""

    result = run_source(source)

    assert result.vm_result.return_value == CliValue.int32(7)


def test_compiler_instance_honors_names() -> None:
    result = NibClrCompiler(
        assembly_name="CustomNib",
        type_name="demo.CustomNib",
    ).compile_source("fn main() -> u4 { return 7; }")

    assert result.assembly_name == "CustomNib"
    assert result.type_name == "demo.CustomNib"
    assert result.decoded_assembly.get_entry_point_method().declaring_type == (
        "demo.CustomNib"
    )


def test_type_error_raises_package_error() -> None:
    with pytest.raises(PackageError) as exc_info:
        compile_source("fn main() { let x: bool = 1 +% 2; }")

    assert exc_info.value.stage == "type-check"


def test_parse_error_raises_package_error() -> None:
    with pytest.raises(PackageError) as exc_info:
        compile_source("fn main(")

    assert exc_info.value.stage == "parse"
