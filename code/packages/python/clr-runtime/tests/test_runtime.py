from __future__ import annotations

from clr_pe_file.testing import hello_world_dll_bytes

from clr_runtime import CLRRuntime


def test_run_compiled_hello_world_entry_point() -> None:
    result = CLRRuntime().run_entry_point(hello_world_dll_bytes())

    assert result.output == "Hello, world!\n"
    assert [trace.opcode for trace in result.traces] == ["ldstr", "call", "ret"]
    assert result.assembly.metadata_version == "v4.0.30319"
