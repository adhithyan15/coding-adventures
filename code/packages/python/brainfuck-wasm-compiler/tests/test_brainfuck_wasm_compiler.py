from __future__ import annotations

from pathlib import Path

from wasm_runtime import WasiConfig, WasiHost, WasmRuntime

from brainfuck_wasm_compiler import BrainfuckWasmCompiler, compile_source, pack_source, write_wasm_file


class _ByteReader:
    def __init__(self, data: str) -> None:
        self._buffer = data.encode("latin-1")
        self._offset = 0

    def __call__(self, count: int) -> bytes:
        chunk = self._buffer[self._offset:self._offset + count]
        self._offset += len(chunk)
        return chunk


def _run(binary: bytes, *, stdin: str = "") -> tuple[list[int | float], list[str]]:
    output: list[str] = []
    host = WasiHost(config=WasiConfig(stdin=_ByteReader(stdin), stdout=output.append))
    runtime = WasmRuntime(host=host)
    return runtime.load_and_run(binary, "_start", []), output


def test_compile_source_returns_pipeline_artifacts() -> None:
    result = compile_source("+.")
    assert result.raw_ir is not None
    assert result.optimized_ir is not None
    assert ".func" in result.wasm_assembly
    assert result.binary
    assert result.filename == "program.bf"


def test_pack_source_is_alias_for_compile_source() -> None:
    compiled = compile_source("+.")
    packed = pack_source("+.")
    assert packed.binary == compiled.binary
    assert packed.wasm_assembly == compiled.wasm_assembly


def test_write_wasm_file_writes_output(tmp_path: Path) -> None:
    output = tmp_path / "program.wasm"
    result = write_wasm_file("+.", output)
    assert output.exists()
    assert output.read_bytes() == result.binary


def test_compiled_output_program_runs_in_wasm_runtime() -> None:
    result = compile_source("+" * 65 + ".")
    runtime_result, output = _run(result.binary)
    assert runtime_result == [0]
    assert output == ["A"]


def test_compiled_input_program_runs_in_wasm_runtime() -> None:
    result = compile_source(",.")
    runtime_result, output = _run(result.binary, stdin="Z")
    assert runtime_result == [0]
    assert output == ["Z"]


def test_compiled_cat_program_runs_in_wasm_runtime() -> None:
    result = compile_source(",[.,]")
    runtime_result, output = _run(result.binary, stdin="Hi")
    assert runtime_result == [0]
    assert output == ["H", "i"]


def test_compiler_instance_honors_custom_filename() -> None:
    result = BrainfuckWasmCompiler(filename="hello.bf").compile_source("+")
    assert result.filename == "hello.bf"
