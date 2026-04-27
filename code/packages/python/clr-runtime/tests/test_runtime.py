from __future__ import annotations

from clr_bytecode_disassembler import CLRMethodBody
from clr_pe_file import CLRMethodDef, CLRPEFile
from clr_pe_file.testing import hello_world_dll_bytes
from clr_simulator import CLRTrace

from clr_runtime import (
    CLRDecodeStage,
    CLRDisassemblyStage,
    CLRExecutionStage,
    CLRHost,
    CLRMethodSelectionStage,
    CLRRuntime,
    CLRRuntimePipeline,
    CLRStdlibHost,
)


def test_run_compiled_hello_world_entry_point() -> None:
    result = CLRRuntime().run_entry_point(hello_world_dll_bytes())

    assert result.output == "Hello, world!\n"
    assert [trace.opcode for trace in result.traces] == ["ldstr", "call", "ret"]
    assert result.assembly.metadata_version == "v4.0.30319"


def test_runtime_result_exposes_composable_stages() -> None:
    result = CLRRuntime().run_entry_point(hello_world_dll_bytes())

    assert isinstance(result.decode_stage, CLRDecodeStage)
    assert isinstance(result.method_selection_stage, CLRMethodSelectionStage)
    assert isinstance(result.disassembly_stage, CLRDisassemblyStage)
    assert isinstance(result.execution_stage, CLRExecutionStage)
    assert result.decode_stage.assembly is result.assembly
    assert result.method_selection_stage.method.name == "Main"
    assert result.disassembly_stage.method_body is result.method_body
    assert result.execution_stage.traces == result.traces


def test_runtime_pipeline_can_run_each_stage_independently() -> None:
    pipeline = CLRRuntimePipeline()
    decoded = pipeline.decode_assembly(hello_world_dll_bytes())
    selected = pipeline.select_method(decoded)
    disassembled = pipeline.disassemble_selected_method(selected)
    executed = pipeline.execute_disassembled_method(disassembled)

    assert decoded.assembly.metadata_version == "v4.0.30319"
    assert selected.method.name == "Main"
    opcodes = [
        instruction.opcode
        for instruction in disassembled.method_body.instructions
    ]
    assert opcodes == [
        "ldstr",
        "call",
        "ret",
    ]
    assert executed.output == "Hello, world!\n"


def test_runtime_pipeline_accepts_replacement_stages() -> None:
    calls: list[str] = []
    default_pipeline = CLRRuntimePipeline()

    def decode_stage(data: bytes) -> CLRPEFile:
        calls.append("decode")
        return default_pipeline.decode(data)

    def select_stage(assembly: CLRPEFile) -> CLRMethodDef:
        calls.append("select")
        return assembly.get_entry_point_method()

    def disassemble_stage(
        assembly: CLRPEFile,
        method: CLRMethodDef,
    ) -> CLRMethodBody:
        calls.append("disassemble")
        return default_pipeline.disassemble_method(assembly, method)

    def execute_stage(
        method_body: CLRMethodBody,
        host: CLRHost,
        max_steps: int,
    ) -> tuple[CLRTrace, ...]:
        calls.append(f"execute:{max_steps}")
        return default_pipeline._default_execute_method(method_body, host, max_steps)

    pipeline = CLRRuntimePipeline(
        host=CLRStdlibHost(),
        decode_stage=decode_stage,
        select_method_stage=select_stage,
        disassemble_stage=disassemble_stage,
        execute_stage=execute_stage,
    )

    result = pipeline.run_entry_point(hello_world_dll_bytes(), max_steps=8)

    assert result.output == "Hello, world!\n"
    assert calls == ["decode", "select", "disassemble", "execute:8"]
