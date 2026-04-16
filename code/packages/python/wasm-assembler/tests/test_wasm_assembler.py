from __future__ import annotations

import pytest
from wasm_runtime import WasmRuntime

from wasm_assembler import WasmAssemblerError, assemble, parse_assembly


def test_assemble_simple_function_and_run_it() -> None:
    assembly = """
.type 0 params=none results=i32
.export function answer 0
.func 0 type=0 locals=i32,i32
  i32.const 42
  local.set 1
  local.get 1
  return
  end
.endfunc
"""

    wasm_bytes = assemble(assembly)
    result = WasmRuntime().load_and_run(wasm_bytes, "answer", [])
    assert result == [42]


def test_parse_assembly_with_memory_and_data() -> None:
    assembly = """
.type 0 params=none results=i32
.memory 0 min=1 max=none
.export memory memory 0
.func 0 type=0 locals=none
  i32.const 0
  return
  end
.endfunc
.data 0 offset=0 bytes=4E,69,62
"""

    module = parse_assembly(assembly)
    assert len(module.memories) == 1
    assert module.data[0].data == b"Nib"


def test_assemble_memory_load_with_block_syntax() -> None:
    assembly = """
.type 0 params=none results=i32
.memory 0 min=1 max=none
.export function read_answer 0
.export memory memory 0
.func 0 type=0 locals=none
  block void
  end
  i32.const 0
  i32.load align=2 offset=4
  return
  end
.endfunc
.data 0 offset=4 bytes=2A,00,00,00
"""

    wasm_bytes = assemble(assembly)
    result = WasmRuntime().load_and_run(wasm_bytes, "read_answer", [])
    assert result == [42]


def test_unterminated_function_raises() -> None:
    assembly = """
.type 0 params=none results=i32
.func 0 type=0 locals=none
  i32.const 1
"""

    with pytest.raises(WasmAssemblerError, match="unterminated"):
        parse_assembly(assembly)


def test_unknown_instruction_raises() -> None:
    assembly = """
.type 0 params=none results=i32
.func 0 type=0 locals=none
  totally.fake
.endfunc
"""

    with pytest.raises(WasmAssemblerError, match="unknown instruction"):
        parse_assembly(assembly)
