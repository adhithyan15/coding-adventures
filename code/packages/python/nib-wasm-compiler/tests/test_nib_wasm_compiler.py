from __future__ import annotations

import pytest
from wasm_runtime import WasmRuntime

from nib_wasm_compiler import NibWasmCompiler, PackageError, compile_source, pack_source, write_wasm_file


def test_compile_source_returns_pipeline_artifacts() -> None:
    source = """
fn answer() -> u4 {
    return 7;
}
"""
    result = compile_source(source)
    assert result.raw_ir is not None
    assert result.optimized_ir is not None
    assert ".func" in result.wasm_assembly
    assert result.binary


def test_pack_source_is_alias_for_compile_source() -> None:
    source = "fn answer() -> u4 { return 7; }"
    compiled = compile_source(source)
    packed = pack_source(source)
    assert packed.binary == compiled.binary
    assert packed.wasm_assembly == compiled.wasm_assembly


def test_write_wasm_file_writes_output(tmp_path) -> None:
    source = "fn answer() -> u4 { return 7; }"
    output = tmp_path / "program.wasm"
    result = write_wasm_file(source, output)
    assert output.exists()
    assert output.read_bytes() == result.binary


def test_compiled_program_runs_in_wasm_runtime() -> None:
    source = """
fn add(a: u4, b: u4) -> u4 {
    return a +% b;
}

fn main() -> u4 {
    return add(3, 4);
}
"""
    result = compile_source(source)
    runtime_result = WasmRuntime().load_and_run(result.binary, "_start", [])
    assert runtime_result == [7]


def test_compiled_loop_runs_in_wasm_runtime() -> None:
    source = """
fn count_to(n: u4) -> u4 {
    let acc: u4 = 0;
    for i: u4 in 0..n {
        acc = acc +% 1;
    }
    return acc;
}
"""
    result = compile_source(source)
    runtime_result = WasmRuntime().load_and_run(result.binary, "count_to", [5])
    assert runtime_result == [5]


def test_compiled_loop_and_call_runs_in_wasm_runtime() -> None:
    source = """
fn add(a: u4, b: u4) -> u4 {
    return a +% b;
}

fn main() -> u4 {
    let acc: u4 = 0;
    for i: u4 in 0..3 {
        acc = add(acc, 1);
    }
    return acc;
}
"""
    result = compile_source(source)
    runtime_result = WasmRuntime().load_and_run(result.binary, "_start", [])
    assert runtime_result == [3]


def test_type_error_raises_package_error() -> None:
    source = "fn main() { let x: bool = 1 +% 2; }"
    with pytest.raises(PackageError) as exc_info:
        compile_source(source)
    assert exc_info.value.stage == "type-check"


def test_parse_error_raises_package_error() -> None:
    source = "fn main("
    with pytest.raises(PackageError) as exc_info:
        NibWasmCompiler().compile_source(source)
    assert exc_info.value.stage == "parse"
